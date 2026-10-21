//! A local project against its cloud copy, and bringing the cloud's records down.
//!
//! The two are the same project when the project's `origin` points at the cloud repository, and the
//! same version when they share the branch name: local `main` only ever syncs with the cloud's
//! `main`. Their save records then say which side is newer: the counts of records only one side
//! has (`ahead` here, `behind` in the cloud).

use crate::refs::{draft_ref, unique_suffix};
use crate::remote::cloud_ref;
use crate::{GitService, PullOutcome, SyncState};

impl GitService {
    /// Records only this computer has, and records only the cloud has, for the current version.
    fn ahead_behind(&self, repo_path: &str, branch: &str) -> Result<(usize, usize), String> {
        let range = format!("{}...HEAD", cloud_ref(branch));
        let output = self.run_git(repo_path, &["rev-list", "--left-right", "--count", &range])?;
        let mut counts = output.split_whitespace().map(|n| n.parse::<usize>().unwrap_or(0));
        let behind = counts.next().unwrap_or(0);
        let ahead = counts.next().unwrap_or(0);
        Ok((ahead, behind))
    }

    /// Compares with the cloud as this computer last saw it; `fetch` brings that up to date.
    pub fn sync_state(&self, repo_path: &str) -> Result<SyncState, String> {
        let branch = self.get_current_branch(repo_path)?;
        let remote_url = self.get_remote_url(repo_path, "origin");
        let has_upstream = !branch.is_empty() && self.resolve_commit(repo_path, &cloud_ref(&branch)).is_some();
        let (ahead, behind) = if has_upstream {
            self.ahead_behind(repo_path, &branch)?
        } else {
            let ahead = self
                .run_git(repo_path, &["rev-list", "--count", "HEAD"])
                .ok()
                .and_then(|n| n.trim().parse::<usize>().ok())
                .unwrap_or(0);
            (ahead, 0)
        };
        Ok(SyncState {
            remote_url,
            branch,
            has_upstream,
            ahead,
            behind,
        })
    }

    /// Downloads the cloud's newer records of the current version. Only moves forward: when this
    /// computer also has records the cloud lacks, nothing changes and `SYNC_DIVERGED` asks the user
    /// to pick a side. Unsaved changes stay, unless the cloud changed the same files; then nothing
    /// changes and `SYNC_LOCAL_CHANGES:<files>` names them.
    pub fn pull(&self, repo_path: &str, token: Option<&str>) -> Result<PullOutcome, String> {
        if !self.fetch(repo_path, token)? {
            return Err("SYNC_NOT_IN_CLOUD".to_string());
        }
        lock_repo!(repo_path);
        let branch = self.require_current_branch(repo_path)?;
        let (ahead, behind) = self.ahead_behind(repo_path, &branch)?;
        if behind == 0 {
            return Ok(PullOutcome { received: 0 });
        }
        if ahead > 0 {
            return Err("SYNC_DIVERGED".to_string());
        }
        self.run_git(repo_path, &["merge", "--ff-only", "--no-edit", &cloud_ref(&branch)])
            .map_err(|error| match conflicting_files(&error) {
                Some(files) => format!("SYNC_LOCAL_CHANGES:{files}"),
                None => error,
            })?;
        Ok(PullOutcome { received: behind })
    }

    /// Switches the current version to the cloud's, keeping this computer's own records as a new
    /// version (with any unsaved changes as its unsaved version) so nothing is lost. Returns the
    /// name of that version.
    pub fn use_cloud_version(&self, repo_path: &str, token: Option<&str>) -> Result<String, String> {
        if !self.fetch(repo_path, token)? {
            return Err("SYNC_NOT_IN_CLOUD".to_string());
        }
        lock_repo!(repo_path);
        let branch = self.require_current_branch(repo_path)?;
        let cloud = cloud_ref(&branch);
        let backup = format!("{branch}-local-{}", chrono::Local::now().format("%Y%m%d-%H%M%S"));
        let backup = if self.branch_exists(repo_path, &backup) { format!("{backup}-{}", unique_suffix()) } else { backup };

        // Keep everything of this computer's before anything on disk changes.
        self.run_git(repo_path, &["branch", &backup, "HEAD"])?;
        let message = format!("Project Hub unsaved changes on {backup}");
        let unsaved = self.snapshot_worktree(repo_path, &message)?;
        let stored = self.resolve_commit(repo_path, &draft_ref(&branch));
        if let Some(draft) = unsaved.as_ref().or(stored.as_ref()) {
            self.run_git(repo_path, &["update-ref", &draft_ref(&backup), draft])?;
        }
        if stored.is_some() {
            // Its base is one of this computer's records, which the cloud version does not have.
            if unsaved.is_some() {
                self.archive_existing_draft(repo_path, &branch)?;
            }
            self.run_git(repo_path, &["update-ref", "-d", &draft_ref(&branch)])?;
        }

        let target = self.resolve_commit(repo_path, &cloud).ok_or("SYNC_NOT_IN_CLOUD")?;
        self.preserve_head_and(repo_path, &branch, &target)?;
        self.run_git(repo_path, &["reset", "--hard", &target])?;
        self.run_git(repo_path, &["clean", "-fd"])?;
        Ok(backup)
    }
}

/// The files git names when it refuses to update over unsaved changes, comma separated.
fn conflicting_files(error: &str) -> Option<String> {
    if !error.contains("would be overwritten") {
        return None;
    }
    let files: Vec<&str> = error
        .lines()
        .filter(|line| line.starts_with('\t') || line.starts_with("        "))
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    Some(files.join(", "))
}
