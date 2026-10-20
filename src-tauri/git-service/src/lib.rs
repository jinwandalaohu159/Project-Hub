//! Git operations behind Project Hub. `GitService` is split by responsibility across modules;
//! each adds its own `impl GitService` block.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Holds the per-repository lock for the rest of the enclosing scope.
macro_rules! lock_repo {
    ($repo:expr) => {
        let lock = $crate::refs::repo_lock($repo);
        let _guard = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    };
}

mod branches;
mod bundle;
mod command;
mod draft;
mod history;
mod housekeeping;
mod refs;
mod remote;
mod rewrite;
mod status;

pub use remote::base64_encode;
pub use rewrite::{RECORD_NOT_IN_VERSION, RECORD_UNDELETABLE};

/// Error prefix returned by [`GitService::push`] when the remote refuses a non-fast-forward update.
pub const PUSH_REJECTED: &str = "PUSH_REJECTED";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitLog {
    pub hash: String,
    pub author: String,
    pub time: Option<DateTime<Utc>>,
    pub message: String,
    #[serde(default)]
    pub body: String,
    pub is_current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    pub is_current: bool,
    pub last_commit_time: Option<DateTime<Utc>>,
    pub last_commit_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub status: FileStatus,
    #[serde(default)]
    pub old_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusSummary {
    pub files: Vec<FileDiff>,
    pub total: usize,
    pub truncated: bool,
}

/// Author identity applied to every commit Project Hub creates.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Identity {
    pub name: String,
    pub email: String,
}

/// The "unsaved version" kept per branch when the working tree is replaced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftInfo {
    pub hash: String,
    pub base: String,
    pub branch: String,
    pub time: Option<DateTime<Utc>>,
    pub files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepoState {
    pub branch: String,
    pub head: String,
    pub draft: Option<String>,
}

/// What [`GitService::ensure_versioned`] had to do to put a folder under version management.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct VersioningOutcome {
    /// The folder had no repository and one was created.
    pub initialized: bool,
    /// The repository had no record and the first one was created.
    pub initial_version: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreOutcome {
    /// True when the previous unsaved changes were kept as the branch's unsaved version.
    pub draft_saved: bool,
}

/// Result of deleting a save record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRecordOutcome {
    /// The deleted record was the current one: the files stay as they are and its changes now show
    /// as unsaved changes.
    pub kept_as_unsaved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    pub remote_url: Option<String>,
    pub branch: String,
    pub has_upstream: bool,
    pub ahead: usize,
    pub behind: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleMeta {
    pub head_branch: String,
    pub has_draft: bool,
}

#[derive(Default)]
pub struct GitService {
    identity: Option<Identity>,
}

impl GitService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_identity(identity: Option<Identity>) -> Self {
        let identity = identity.filter(|id| !id.name.trim().is_empty() || !id.email.trim().is_empty());
        Self { identity }
    }

    /// Runs a read once no save, restore or switch is half-way through, so status checks never
    /// report the in-between state of the working tree as unsaved changes.
    pub fn settled<T>(&self, repo_path: &str, read: impl FnOnce(&Self) -> T) -> T {
        lock_repo!(repo_path);
        read(self)
    }

    pub fn is_git_repo(&self, path: &str) -> bool {
        std::path::Path::new(path).join(".git").exists()
    }
}

#[cfg(test)]
mod tests;
