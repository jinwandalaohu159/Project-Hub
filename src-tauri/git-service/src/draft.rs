//! The per-version "unsaved changes" entry: snapshots of the working tree kept as private refs.

use crate::history::LOG_FIELD_SEP;
use crate::refs::{archive_prefix, draft_ref, unique_suffix, DRAFT_ARCHIVE_KEEP};
use crate::{DraftInfo, GitService, RepoState, RestoreOutcome};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::Path;

impl GitService {
    /// Records the current working tree as a commit object without touching HEAD, the index or files.
    /// Returns `None` when the working tree matches HEAD.
    pub fn snapshot_worktree(&self, repo_path: &str, message: &str) -> Result<Option<String>, String> {
        self.snapshot_worktree_with(repo_path, message, &[])
    }

    /// Like [`Self::snapshot_worktree`], additionally recording `forced` paths even if they are ignored.
    pub(crate) fn snapshot_worktree_with(
        &self,
        repo_path: &str,
        message: &str,
        forced: &[String],
    ) -> Result<Option<String>, String> {
        self.ensure_default_ignores(repo_path)?;
        self.sync_empty_dir_markers(repo_path)?;
        let head_tree = self.run_git(repo_path, &["rev-parse", "HEAD^{tree}"])?;
        let head_tree = head_tree.trim().to_string();

        let tree = self.with_worktree_index_forced(repo_path, forced, |envs| {
            let tree = self.run_git_with_env(repo_path, &["write-tree"], envs)?;
            Ok(tree.trim().to_string())
        })?;
        if tree == head_tree {
            return Ok(None);
        }

        self.ensure_commit_identity(repo_path)?;
        let commit = self.run_git(repo_path, &["commit-tree", &tree, "-p", "HEAD", "-m", message])?;
        Ok(Some(commit.trim().to_string()))
    }

    /// Runs `task` with a throwaway index that mirrors the full working tree (including new files).
    pub(crate) fn with_worktree_index<T>(
        &self,
        repo_path: &str,
        task: impl FnOnce(&[(&str, &str)]) -> Result<T, String>,
    ) -> Result<T, String> {
        self.with_worktree_index_forced(repo_path, &[], task)
    }

    pub(crate) fn with_worktree_index_forced<T>(
        &self,
        repo_path: &str,
        forced: &[String],
        task: impl FnOnce(&[(&str, &str)]) -> Result<T, String>,
    ) -> Result<T, String> {
        let git_dir = self.git_dir(repo_path)?;
        let temp_index = git_dir.join(format!("project-hub-{}.index", unique_suffix()));
        let real_index = git_dir.join("index");
        let temp_index_str = temp_index.to_string_lossy().to_string();
        let envs = [("GIT_INDEX_FILE", temp_index_str.as_str())];

        let result = (|| {
            if real_index.exists() {
                // Copying keeps git's stat cache, so unchanged files are not re-hashed.
                fs::copy(&real_index, &temp_index)
                    .map_err(|e| format!("Failed to prepare snapshot index: {}", e))?;
            } else {
                self.run_git_with_env(repo_path, &["read-tree", "HEAD"], &envs)?;
            }
            self.run_git_with_env(repo_path, &["add", "--all"], &envs)?;
            for chunk in forced.chunks(100) {
                let mut args = vec!["add", "-f", "--"];
                args.extend(chunk.iter().map(String::as_str));
                self.run_git_with_env(repo_path, &args, &envs)?;
            }
            task(&envs)
        })();

        let _ = fs::remove_file(&temp_index);
        result
    }

    /// Ignored files on disk that `commit` tracks; checking it out would silently replace them.
    pub(crate) fn ignored_paths_tracked_in(&self, repo_path: &str, commit: &str) -> Result<Vec<String>, String> {
        let ignored = self.run_git(
            repo_path,
            &["ls-files", "-o", "-i", "--exclude-standard", "--directory", "-z"],
        )?;
        let ignored: Vec<&str> = ignored.split('\0').filter(|entry| !entry.is_empty()).collect();
        if ignored.is_empty() {
            return Ok(Vec::new());
        }
        let tracked = self.run_git(repo_path, &["ls-tree", "-r", "--name-only", "-z", commit])?;
        Ok(tracked
            .split('\0')
            .filter(|path| !path.is_empty())
            .filter(|path| {
                ignored.iter().any(|entry| {
                    *path == *entry || (entry.ends_with('/') && path.starts_with(entry))
                })
            })
            .filter(|path| Path::new(repo_path).join(path).is_file())
            .map(ToString::to_string)
            .collect())
    }

    pub(crate) fn archive_commit(&self, repo_path: &str, branch: &str, commit: &str) -> Result<(), String> {
        let archive_ref = format!("{}{}", archive_prefix(branch), unique_suffix());
        self.run_git(repo_path, &["update-ref", &archive_ref, commit])?;
        self.prune_draft_archive(repo_path, branch);
        Ok(())
    }

    pub(crate) fn prune_draft_archive(&self, repo_path: &str, branch: &str) {
        let prefix = archive_prefix(branch);
        let Ok(output) = self.run_git(
            repo_path,
            &["for-each-ref", "--sort=-refname", "--format=%(refname)", &prefix],
        ) else {
            return;
        };
        for stale in output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .skip(DRAFT_ARCHIVE_KEEP)
        {
            let _ = self.run_git(repo_path, &["update-ref", "-d", stale]);
        }
    }

    pub(crate) fn archive_existing_draft(&self, repo_path: &str, branch: &str) -> Result<(), String> {
        if let Some(existing) = self.resolve_commit(repo_path, &draft_ref(branch)) {
            self.archive_commit(repo_path, branch, &existing)?;
        }
        Ok(())
    }

    /// Stores the current unsaved changes as the branch's unsaved version. Returns false when clean.
    pub fn save_draft(&self, repo_path: &str) -> Result<bool, String> {
        lock_repo!(repo_path);
        self.save_draft_unlocked(repo_path, &[])
    }

    pub(crate) fn save_draft_unlocked(&self, repo_path: &str, forced: &[String]) -> Result<bool, String> {
        let branch = self.require_current_branch(repo_path)?;
        let message = format!("Project Hub unsaved changes on {}", branch);
        let Some(commit) = self.snapshot_worktree_with(repo_path, &message, forced)? else {
            return Ok(false);
        };
        self.archive_existing_draft(repo_path, &branch)?;
        self.run_git(repo_path, &["update-ref", &draft_ref(&branch), &commit])?;
        Ok(true)
    }

    /// Moves unsaved changes into the unsaved version and leaves a clean working tree.
    pub fn stash_to_draft(&self, repo_path: &str) -> Result<bool, String> {
        lock_repo!(repo_path);
        let saved = self.save_draft_unlocked(repo_path, &[])?;
        self.run_git(repo_path, &["reset", "--hard", "HEAD"])?;
        self.run_git(repo_path, &["clean", "-fd"])?;
        Ok(saved)
    }

    pub fn get_draft(&self, repo_path: &str) -> Result<Option<DraftInfo>, String> {
        let branch = self.get_current_branch(repo_path)?;
        if branch.is_empty() {
            return Ok(None);
        }
        let Some(hash) = self.resolve_commit(repo_path, &draft_ref(&branch)) else {
            return Ok(None);
        };
        let meta = self.run_git(repo_path, &["log", "-1", "--format=%P%x1f%aI", &hash])?;
        let mut parts = meta.trim().splitn(2, LOG_FIELD_SEP);
        let base = parts
            .next()
            .and_then(|parents| parents.split_whitespace().next())
            .unwrap_or_default()
            .to_string();
        let time = parts
            .next()
            .and_then(|time| DateTime::parse_from_rfc3339(time.trim()).ok())
            .map(|dt| dt.with_timezone(&Utc));
        let files = if base.is_empty() {
            0
        } else {
            self.run_git(repo_path, &["diff", "--name-only", &base, &hash])?
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count()
        };
        Ok(Some(DraftInfo {
            hash,
            base,
            branch,
            time,
            files,
        }))
    }

    /// Brings back the unsaved version. Whatever is unsaved right now becomes the new unsaved version,
    /// so switching back and forth never loses work.
    pub fn restore_draft(&self, repo_path: &str) -> Result<RestoreOutcome, String> {
        let branch = self.require_current_branch(repo_path)?;
        let draft_ref = draft_ref(&branch);
        let draft = self
            .resolve_commit(repo_path, &draft_ref)
            .ok_or_else(|| "There is no unsaved version to restore.".to_string())?;
        let base = self
            .resolve_commit(repo_path, &format!("{}^", draft))
            .ok_or_else(|| "The unsaved version has no base record.".to_string())?;
        let draft_tree = self.tree_of(repo_path, &draft)?;
        lock_repo!(repo_path);

        let mut at_risk = self.ignored_paths_tracked_in(repo_path, &base)?;
        for path in self.ignored_paths_tracked_in(repo_path, &draft)? {
            if !at_risk.contains(&path) {
                at_risk.push(path);
            }
        }
        let current_snapshot = if self.has_unsaved_changes(repo_path)? || !at_risk.is_empty() {
            let message = format!("Project Hub unsaved changes on {}", branch);
            match self.snapshot_worktree_with(repo_path, &message, &at_risk)? {
                Some(snapshot) if self.tree_of(repo_path, &snapshot)? != draft_tree => Some(snapshot),
                _ => None,
            }
        } else {
            None
        };
        // Reference the current work before anything on disk changes, so a failure below loses nothing.
        if let Some(snapshot) = &current_snapshot {
            self.archive_commit(repo_path, &branch, snapshot)?;
        }

        self.preserve_head_and(repo_path, &branch, &base)?;
        self.run_git(repo_path, &["reset", "--hard", &base])?;
        self.run_git(repo_path, &["clean", "-fd"])?;
        self.run_git(repo_path, &["restore", "--source", &draft, "--worktree", "--", "."])?;

        self.archive_commit(repo_path, &branch, &draft)?;
        match &current_snapshot {
            Some(snapshot) => {
                self.run_git(repo_path, &["update-ref", &draft_ref, snapshot])?;
            }
            None => {
                self.run_git(repo_path, &["update-ref", "-d", &draft_ref])?;
            }
        }
        Ok(RestoreOutcome {
            draft_saved: current_snapshot.is_some(),
        })
    }

    pub fn discard_draft(&self, repo_path: &str) -> Result<(), String> {
        lock_repo!(repo_path);
        let branch = self.require_current_branch(repo_path)?;
        let draft_ref = draft_ref(&branch);
        if let Some(draft) = self.resolve_commit(repo_path, &draft_ref) {
            self.archive_commit(repo_path, &branch, &draft)?;
            self.run_git(repo_path, &["update-ref", "-d", &draft_ref])?;
        }
        Ok(())
    }

    /// Cheap fingerprint used by the UI to notice changes made outside the app (agents, terminals).
    pub fn get_repo_state(&self, repo_path: &str) -> Result<RepoState, String> {
        let branch = self.get_current_branch(repo_path)?;
        let head = self
            .run_git(repo_path, &["rev-parse", "-q", "--verify", "HEAD"])
            .unwrap_or_default()
            .trim()
            .to_string();
        let draft = if branch.is_empty() {
            None
        } else {
            self.resolve_commit(repo_path, &draft_ref(&branch))
        };
        Ok(RepoState { branch, head, draft })
    }

    /// Patch of every unsaved change, including new files, truncated to `max_bytes`.
    pub fn worktree_diff(&self, repo_path: &str, max_bytes: usize) -> Result<String, String> {
        self.ensure_default_ignores(repo_path)?;
        self.with_worktree_index(repo_path, |envs| {
            let stat = self.run_git_with_env(repo_path, &["diff", "--cached", "--stat", "HEAD"], envs)?;
            let patch = self.run_git_with_env(repo_path, &["diff", "--cached", "HEAD"], envs)?;
            let mut text = stat;
            if !patch.trim().is_empty() {
                text.push('\n');
                text.push_str(&patch);
            }
            if text.len() > max_bytes {
                let mut cut = max_bytes;
                while cut > 0 && !text.is_char_boundary(cut) {
                    cut -= 1;
                }
                text.truncate(cut);
                text.push_str("\n... (diff truncated)\n");
            }
            Ok(text)
        })
    }
}
