//! Naming of Project Hub's private refs and the per-repository operation lock.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const DRAFT_REF_PREFIX: &str = "refs/project-hub/drafts/";
pub(crate) const DRAFT_ARCHIVE_PREFIX: &str = "refs/project-hub/draft-archive/";
pub(crate) const DRAFT_ARCHIVE_KEEP: usize = 20;

pub(crate) fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    format!("{:024}", nanos)
}

/// Branch names may contain '/', which would nest refs of different branches inside each other.
/// Plain names stay unchanged so refs written by earlier versions keep working.
pub(crate) fn ref_segment(branch: &str) -> String {
    branch.trim().replace('%', "%25").replace('/', "%2F")
}

pub(crate) fn draft_ref(branch: &str) -> String {
    format!("{}{}", DRAFT_REF_PREFIX, ref_segment(branch))
}

pub(crate) fn archive_prefix(branch: &str) -> String {
    format!("{}{}/", DRAFT_ARCHIVE_PREFIX, ref_segment(branch))
}

/// Serialises operations that rewrite the working tree of one repository (UI, MCP and polling run
/// on different threads).
pub(crate) fn repo_lock(repo_path: &str) -> Arc<Mutex<()>> {
    static LOCKS: OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> = OnceLock::new();
    let key = repo_path.replace('\\', "/").trim_end_matches('/').to_lowercase();
    let mut locks = LOCKS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    locks.entry(key).or_default().clone()
}
