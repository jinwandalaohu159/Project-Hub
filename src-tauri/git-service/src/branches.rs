//! Versions (git branches): listing, switching, creating, deleting and merging.

use crate::refs::draft_ref;
use crate::{Branch, GitService};
use chrono::{DateTime, NaiveDateTime, Utc};

impl GitService {
    pub fn get_branches(&self, repo_path: &str) -> Result<Vec<Branch>, String> {
        let output = self.run_git(
            repo_path,
            &[
                "branch",
                "--format=%(refname:short)|%(HEAD)|%(committerdate:iso8601)|%(subject)",
            ],
        )?;
        let mut branches = Vec::new();
        for line in output.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.splitn(4, '|').collect();
            if parts.len() < 2 {
                continue;
            }
            let name = parts[0].trim().to_string();
            let is_current = parts[1].trim() == "*";
            let last_commit_time = parts
                .get(2)
                .and_then(|s| {
                    DateTime::parse_from_str(s.trim(), "%Y-%m-%d %H:%M:%S %z")
                        .map(|dt| dt.with_timezone(&Utc))
                        .ok()
                })
                .or_else(|| {
                    parts.get(2).and_then(|s| {
                        NaiveDateTime::parse_from_str(s.trim(), "%Y-%m-%d %H:%M:%S")
                            .ok()
                            .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
                    })
                });
            let last_commit_message = parts.get(3).map(|s| s.trim().to_string());

            branches.push(Branch {
                name,
                is_current,
                last_commit_time,
                last_commit_message,
            });
        }
        Ok(branches)
    }

    pub fn get_current_branch(&self, repo_path: &str) -> Result<String, String> {
        let output = self.run_git(repo_path, &["branch", "--show-current"])?;
        Ok(output.trim().to_string())
    }

    pub(crate) fn require_current_branch(&self, repo_path: &str) -> Result<String, String> {
        let branch = self.get_current_branch(repo_path)?;
        if branch.is_empty() {
            return Err("VERSION_DETACHED".to_string());
        }
        Ok(branch)
    }

    pub fn switch_branch(&self, repo_path: &str, branch_name: &str) -> Result<(), String> {
        lock_repo!(repo_path);
        self.switch_branch_unlocked(repo_path, branch_name)
    }

    pub(crate) fn switch_branch_unlocked(&self, repo_path: &str, branch_name: &str) -> Result<(), String> {
        self.run_git(repo_path, &["switch", branch_name])?;
        self.run_git(repo_path, &["clean", "-fd"])?;
        Ok(())
    }

    pub fn create_branch(&self, repo_path: &str, name: &str) -> Result<(), String> {
        self.create_initial_branch_ref(repo_path, name)?;
        Ok(())
    }

    pub fn create_and_switch_branch(&self, repo_path: &str, name: &str) -> Result<(), String> {
        if self.has_unsaved_changes(repo_path)? {
            return Err("UNSAVED_BEFORE_BLANK_VERSION".to_string());
        }

        lock_repo!(repo_path);
        self.create_initial_branch_ref(repo_path, name)?;
        self.switch_branch_unlocked(repo_path, name)?;
        Ok(())
    }

    pub(crate) fn create_initial_branch_ref(&self, repo_path: &str, name: &str) -> Result<(), String> {
        self.run_git(repo_path, &["check-ref-format", "--branch", name])?;
        if self.branch_exists(repo_path, name) {
            return Err(format!("VERSION_EXISTS:{name}"));
        }

        self.ensure_commit_identity(repo_path)?;
        let tree = self.run_git(repo_path, &["rev-parse", "HEAD^{tree}"])?;
        let tree = tree.trim();
        let commit = self.run_git(repo_path, &["commit-tree", tree, "-m", "Initial commit"])?;
        let commit = commit.trim();
        let branch_ref = format!("refs/heads/{}", name);
        self.run_git(repo_path, &["update-ref", &branch_ref, commit])?;
        Ok(())
    }

    pub(crate) fn branch_exists(&self, repo_path: &str, name: &str) -> bool {
        let branch_ref = format!("refs/heads/{}", name);
        self.git_succeeds(repo_path, &["show-ref", "--verify", "--quiet", &branch_ref])
    }

    pub fn delete_branch(&self, repo_path: &str, name: &str, force: bool) -> Result<(), String> {
        if force {
            self.run_git(repo_path, &["branch", "-D", name])?;
        } else {
            self.run_git(repo_path, &["branch", "-d", name])?;
        }
        let draft_ref = draft_ref(name);
        if let Some(draft) = self.resolve_commit(repo_path, &draft_ref) {
            // Kept as a hidden safety copy rather than lost with the version.
            let _ = self.archive_commit(repo_path, name, &draft);
            let _ = self.run_git(repo_path, &["update-ref", "-d", &draft_ref]);
        }
        // Otherwise a new version with the same name would inherit these records.
        for checkpoint in self.checkpoint_refs_for_branch(repo_path, name).unwrap_or_default() {
            let _ = self.run_git(repo_path, &["update-ref", "-d", &checkpoint]);
        }
        Ok(())
    }

    pub fn merge_branch(&self, repo_path: &str, source_branch: &str) -> Result<(), String> {
        self.ensure_commit_identity(repo_path)?;
        self.run_git(
            repo_path,
            &["merge", "--allow-unrelated-histories", source_branch],
        )?;
        Ok(())
    }

    pub fn has_merge_conflicts(&self, repo_path: &str) -> bool {
        self.run_git(repo_path, &["ls-files", "--unmerged"])
            .map(|output| !output.trim().is_empty())
            .unwrap_or(false)
    }
}
