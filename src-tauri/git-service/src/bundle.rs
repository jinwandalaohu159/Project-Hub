//! Export / import of a whole repository as a git bundle.

use crate::refs::draft_ref;
use crate::{BundleMeta, GitService};
use std::fs;
use std::path::Path;

pub(crate) const EXPORT_DRAFT_REF: &str = "refs/project-hub/export-draft";

impl GitService {
    /// Writes every version, save record and the current unsaved changes into a git bundle.
    pub fn create_bundle(&self, repo_path: &str, out_path: &Path) -> Result<BundleMeta, String> {
        let head_branch = self.require_current_branch(repo_path)?;
        lock_repo!(repo_path);
        let draft = self.snapshot_worktree(repo_path, "Project Hub export snapshot")?;
        if let Some(draft) = &draft {
            self.run_git(repo_path, &["update-ref", EXPORT_DRAFT_REF, draft])?;
        }
        let out = out_path.to_string_lossy().to_string();
        let result = self.run_git(repo_path, &["bundle", "create", &out, "--all"]);
        if draft.is_some() {
            let _ = self.run_git(repo_path, &["update-ref", "-d", EXPORT_DRAFT_REF]);
        }
        result?;
        Ok(BundleMeta {
            head_branch,
            has_draft: draft.is_some(),
        })
    }

    /// Rebuilds a project from a bundle. Existing files in `target` are kept and show up as unsaved
    /// changes; an empty or missing folder gets the exported files back.
    pub fn restore_bundle(
        &self,
        target: &Path,
        bundle: &Path,
        head_branch: &str,
        remote_url: Option<&str>,
    ) -> Result<(), String> {
        fs::create_dir_all(target).map_err(|e| format!("FILE_ERROR:{e}"))?;
        let repo = target.to_string_lossy().to_string();
        // An existing repository without any record holds nothing worth keeping; import into it.
        let had_git = target.join(".git").exists();
        if had_git && self.has_commits(&repo) {
            return Err("TARGET_HAS_HISTORY".to_string());
        }
        let was_empty = fs::read_dir(target)
            .map_err(|e| format!("FILE_ERROR:{e}"))?
            .flatten()
            .all(|entry| entry.file_name() == ".git");
        let bundle = bundle.to_string_lossy().to_string();

        let result = self.restore_bundle_into(&repo, &bundle, head_branch, remote_url, was_empty);
        if result.is_err() && !had_git {
            // A half-initialised repository would look like a finished import on the next attempt.
            let _ = fs::remove_dir_all(target.join(".git"));
        }
        result
    }

    pub(crate) fn restore_bundle_into(
        &self,
        repo: &str,
        bundle: &str,
        head_branch: &str,
        remote_url: Option<&str>,
        was_empty: bool,
    ) -> Result<(), String> {
        let repo = repo.to_string();
        let bundle = bundle.to_string();
        self.run_git(&repo, &["init"])?;
        self.ensure_default_ignores(&repo)?;
        self.run_git(&repo, &["fetch", "--update-head-ok", &bundle, "+refs/*:refs/*"])?;
        let head_ref = format!("refs/heads/{}", head_branch);
        self.run_git(&repo, &["symbolic-ref", "HEAD", &head_ref])?;
        if was_empty {
            self.run_git(&repo, &["reset", "--hard", "HEAD"])?;
        } else {
            self.run_git(&repo, &["reset", "-q", "HEAD"])?;
        }

        if let Some(draft) = self.resolve_commit(&repo, EXPORT_DRAFT_REF) {
            if was_empty {
                self.run_git(&repo, &["restore", "--source", &draft, "--worktree", "--", "."])?;
            } else {
                self.run_git(&repo, &["update-ref", &draft_ref(head_branch), &draft])?;
            }
            self.run_git(&repo, &["update-ref", "-d", EXPORT_DRAFT_REF])?;
        }

        if let Some(url) = remote_url.filter(|url| !url.trim().is_empty()) {
            self.set_remote_url(&repo, "origin", url)?;
        }
        Ok(())
    }
}
