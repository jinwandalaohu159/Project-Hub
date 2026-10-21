//! Save records: saving, the checkpoint refs that keep later records listed, and restoring.

use crate::refs::ref_segment;
use crate::{CommitLog, GitService, RestoreOutcome, VersioningOutcome};
use std::path::Path;
use chrono::{DateTime, Utc};

pub(crate) const DEFAULT_AUTHOR_NAME: &str = "Project Hub";
pub(crate) const DEFAULT_AUTHOR_EMAIL: &str = "project-hub@local";
pub(crate) const LOG_FIELD_SEP: char = '\u{1f}';
pub(crate) const LOG_RECORD_SEP: char = '\u{1e}';

impl GitService {
    pub fn stash(&self, repo_path: &str) -> Result<(), String> {
        self.run_git(repo_path, &["stash"])?;
        Ok(())
    }

    pub fn stash_pop(&self, repo_path: &str) -> Result<(), String> {
        self.run_git(repo_path, &["stash", "pop"])?;
        Ok(())
    }

    pub fn discard_changes(&self, repo_path: &str) -> Result<(), String> {
        lock_repo!(repo_path);
        // Keep an invisible safety copy so discarded work can still be recovered by hand.
        if let Ok(branch) = self.get_current_branch(repo_path) {
            if !branch.is_empty() {
                if let Ok(Some(snapshot)) =
                    self.snapshot_worktree(repo_path, "Project Hub discarded changes")
                {
                    let _ = self.archive_commit(repo_path, &branch, &snapshot);
                }
            }
        }
        self.run_git(repo_path, &["reset", "--hard", "HEAD"])?;
        self.run_git(repo_path, &["clean", "-fd"])?;
        Ok(())
    }

    pub fn add_files(&self, repo_path: &str, paths: &[&str]) -> Result<(), String> {
        let mut args = vec!["add"];
        args.extend(paths);
        self.run_git(repo_path, &args)?;
        Ok(())
    }

    pub fn add_all(&self, repo_path: &str) -> Result<(), String> {
        self.ensure_default_ignores(repo_path)?;
        self.run_git(repo_path, &["add", "--all"])?;
        Ok(())
    }

    pub(crate) fn add_all_with_empty_dir_markers(&self, repo_path: &str) -> Result<(), String> {
        self.ensure_default_ignores(repo_path)?;
        self.sync_empty_dir_markers(repo_path)?;
        self.run_git(repo_path, &["add", "--all"])?;
        Ok(())
    }

    pub fn unstage_files(&self, repo_path: &str, paths: &[&str]) -> Result<(), String> {
        let mut args = vec!["reset", "HEAD"];
        args.extend(paths);
        self.run_git(repo_path, &args)?;
        Ok(())
    }

    pub fn commit(&self, repo_path: &str, message: &str) -> Result<(), String> {
        self.ensure_commit_identity(repo_path)?;
        self.run_git(repo_path, &["commit", "-m", message])?;
        Ok(())
    }

    /// Falls back to a neutral identity only when neither the app settings nor git config provide one.
    pub(crate) fn ensure_commit_identity(&self, repo_path: &str) -> Result<(), String> {
        let has_name = self
            .identity
            .as_ref()
            .is_some_and(|id| !id.name.trim().is_empty());
        let has_email = self
            .identity
            .as_ref()
            .is_some_and(|id| !id.email.trim().is_empty());

        if !has_email {
            let email_missing = self
                .run_git(repo_path, &["config", "--get", "user.email"])
                .map(|email| email.trim().is_empty())
                .unwrap_or(true);
            if email_missing {
                self.run_git(repo_path, &["config", "user.email", DEFAULT_AUTHOR_EMAIL])?;
            }
        }

        if !has_name {
            let name_missing = self
                .run_git(repo_path, &["config", "--get", "user.name"])
                .map(|name| name.trim().is_empty())
                .unwrap_or(true);
            if name_missing {
                self.run_git(repo_path, &["config", "user.name", DEFAULT_AUTHOR_NAME])?;
            }
        }

        Ok(())
    }

    pub fn save_version(&self, repo_path: &str, message: &str) -> Result<(), String> {
        let result = self.save_version_locked(repo_path, message);
        if result.is_ok() {
            self.schedule_housekeeping(repo_path);
        }
        result
    }

    fn save_version_locked(&self, repo_path: &str, message: &str) -> Result<(), String> {
        lock_repo!(repo_path);
        // Empty folders are part of every save, not only of saves without other changes.
        self.add_all_with_empty_dir_markers(repo_path)?;
        if !self.has_staged_changes(repo_path)? {
            return Ok(());
        }

        self.commit(repo_path, message)
    }

    pub(crate) fn has_staged_changes(&self, repo_path: &str) -> Result<bool, String> {
        let output = self.run_git(repo_path, &["diff", "--cached", "--name-only"])?;
        Ok(!output.trim().is_empty())
    }

    pub(crate) fn checkpoint_ref_prefix(&self, branch_name: &str) -> String {
        format!("refs/project-hub/checkpoints/{}/", ref_segment(branch_name))
    }

    pub(crate) fn checkpoint_ref(&self, branch_name: &str, commit_hash: &str) -> String {
        format!("{}{}", self.checkpoint_ref_prefix(branch_name), commit_hash)
    }

    /// Keeps both the current record and `target` listed, in one git call.
    pub(crate) fn preserve_head_and(&self, repo_path: &str, branch_name: &str, target: &str) -> Result<(), String> {
        let head = self.run_git(repo_path, &["rev-parse", "HEAD"])?;
        let head = head.trim();
        let mut input = format!("update {} {}\n", self.checkpoint_ref(branch_name, head), head);
        if target != head {
            input.push_str(&format!("update {} {}\n", self.checkpoint_ref(branch_name, target), target));
        }
        self.run_git_with_input(repo_path, &["update-ref", "--stdin"], &[], &input)?;
        Ok(())
    }

    pub(crate) fn checkpoint_refs_for_branch(
        &self,
        repo_path: &str,
        branch_name: &str,
    ) -> Result<Vec<String>, String> {
        let prefix = self.checkpoint_ref_prefix(branch_name);
        let output = self.run_git(repo_path, &["for-each-ref", "--format=%(refname)", &prefix])?;
        Ok(output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToString::to_string)
            .collect())
    }

    pub fn get_log(&self, repo_path: &str, count: usize) -> Result<Vec<CommitLog>, String> {
        let current_branch = self.get_current_branch(repo_path).unwrap_or_default();
        let current_head = self
            .run_git(repo_path, &["rev-parse", "HEAD"])
            .unwrap_or_default()
            .trim()
            .to_string();
        // A glob keeps the command line short however many records were restored.
        let checkpoints = if current_branch.trim().is_empty() {
            None
        } else {
            Some(format!("--glob={}*", self.checkpoint_ref_prefix(&current_branch)))
        };

        let limit = format!("-{}", count);
        let mut args = vec![
            "log".to_string(),
            limit,
            "--date-order".to_string(),
            "--format=%H%x1f%an%x1f%aI%x1f%s%x1f%b%x1e".to_string(),
            "HEAD".to_string(),
        ];
        args.extend(checkpoints);
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let output = self.run_git(repo_path, &arg_refs)?;
        let mut logs = Vec::new();
        for record in output.split(LOG_RECORD_SEP) {
            let record = record.trim_start_matches(['\n', '\r']);
            if record.trim().is_empty() {
                continue;
            }
            let parts: Vec<&str> = record.splitn(5, LOG_FIELD_SEP).collect();
            if parts.len() < 4 {
                continue;
            }
            let time = DateTime::parse_from_rfc3339(parts[2].trim())
                .ok()
                .map(|dt| dt.with_timezone(&Utc));
            logs.push(CommitLog {
                hash: parts[0].trim().to_string(),
                author: parts[1].trim().to_string(),
                time,
                message: parts[3].trim().to_string(),
                body: parts.get(4).map(|s| s.trim().to_string()).unwrap_or_default(),
                is_current: parts[0].trim() == current_head,
            });
        }
        Ok(logs)
    }

    pub fn restore_commit(&self, repo_path: &str, commit_hash: &str) -> Result<RestoreOutcome, String> {
        let commit_spec = format!("{}^{{commit}}", commit_hash);
        let target_commit = self.run_git(repo_path, &["rev-parse", "--verify", &commit_spec])?;
        let target_commit = target_commit.trim().to_string();
        let current_branch = self.require_current_branch(repo_path)?;
        lock_repo!(repo_path);

        // Ignored local files (e.g. .env.local) that the target record tracks would be overwritten.
        let at_risk = self.ignored_paths_tracked_in(repo_path, &target_commit)?;
        let dirty = self.has_unsaved_changes(repo_path)?;
        let draft_saved = if dirty || !at_risk.is_empty() {
            self.save_draft_unlocked(repo_path, &at_risk)?
        } else {
            false
        };

        self.preserve_head_and(repo_path, &current_branch, &target_commit)?;
        self.run_git(repo_path, &["reset", "--hard", &target_commit])?;
        // A clean working tree has no untracked files for `clean` to remove.
        if dirty {
            self.run_git(repo_path, &["clean", "-fd"])?;
        }
        Ok(RestoreOutcome { draft_saved })
    }

    pub fn init_repo(&self, repo_path: &str) -> Result<(), String> {
        self.ensure_versioned(repo_path).map(|_| ())
    }

    /// Every folder handed to Project Hub is version-managed: create the repository and/or the
    /// first record when missing. Does nothing for a folder that is already set up.
    pub fn ensure_versioned(&self, repo_path: &str) -> Result<VersioningOutcome, String> {
        if !Path::new(repo_path).is_dir() {
            return Err(format!("PROJECT_FOLDER_MISSING:{repo_path}"));
        }
        let initialized = !self.is_git_repo(repo_path);
        if initialized {
            self.run_git(repo_path, &["init"])?;
        }
        let initial_version = self.ensure_initial_version(repo_path)?;
        Ok(VersioningOutcome {
            initialized,
            initial_version,
        })
    }

    pub fn has_commits(&self, repo_path: &str) -> bool {
        self.resolve_commit(repo_path, "HEAD").is_some()
    }

    /// A repository without any record (an interrupted setup, `git init` by another tool) has no
    /// version to show or branch from; give it the same empty first record a new project gets.
    /// Returns true when the record was created.
    pub fn ensure_initial_version(&self, repo_path: &str) -> Result<bool, String> {
        if !self.is_git_repo(repo_path) || self.has_commits(repo_path) {
            return Ok(false);
        }
        lock_repo!(repo_path);
        self.ensure_default_ignores(repo_path)?;
        self.ensure_commit_identity(repo_path)?;
        self.run_git(
            repo_path,
            &["commit", "--allow-empty", "-m", "Initial version"],
        )?;
        Ok(true)
    }
}
