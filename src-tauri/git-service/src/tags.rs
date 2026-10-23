//! Version tags for releases: which save records came since the last one, and tagging the current
//! record and uploading that tag (which starts a project's cloud build, if it has one).

use crate::remote::{auth_envs, auth_header};
use crate::GitService;
use serde::{Deserialize, Serialize};

const FIELD_SEP: char = '\u{1f}';
const RECORD_SEP: char = '\u{1e}';

/// A save record as release notes use it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordSummary {
    pub title: String,
    pub body: String,
}

impl GitService {
    /// The commit `tag` points at, when this computer has it.
    pub fn tag_commit(&self, repo_path: &str, tag: &str) -> Option<String> {
        self.run_git(repo_path, &["rev-parse", "--verify", "--quiet", &format!("refs/tags/{tag}^{{commit}}")])
            .ok()
            .map(|sha| sha.trim().to_string())
            .filter(|sha| !sha.is_empty())
    }

    /// Save records of the current version after `since` (a tag), newest first; all of them when
    /// there is no such tag. At most `limit`.
    pub fn records_since(&self, repo_path: &str, since: Option<&str>, limit: usize) -> Result<Vec<RecordSummary>, String> {
        let range = match since.and_then(|tag| self.tag_commit(repo_path, tag)) {
            Some(commit) => format!("{commit}..HEAD"),
            None => "HEAD".to_string(),
        };
        let limit = format!("-{limit}");
        let output = self.run_git(repo_path, &["log", &limit, "--format=%s%x1f%b%x1e", &range])?;
        Ok(output
            .split(RECORD_SEP)
            .filter_map(|record| {
                let record = record.trim_start_matches(['\n', '\r']);
                let (title, body) = record.split_once(FIELD_SEP)?;
                Some(RecordSummary { title: title.trim().to_string(), body: body.trim().to_string() })
            })
            .filter(|record| !record.title.is_empty())
            .collect())
    }

    /// Brings the cloud's version tags down, without replacing tags this computer already has.
    pub fn fetch_tags(&self, repo_path: &str, token: Option<&str>) -> Result<(), String> {
        if self.get_remote_url(repo_path, "origin").is_none() {
            return Err("REMOTE_NOT_LINKED".to_string());
        }
        let header = auth_header(token);
        self.run_git_with_env(repo_path, &["fetch", "--no-recurse-submodules", "origin", "refs/tags/*:refs/tags/*"], &auth_envs(header.as_deref()))
            .map(|_| ())
    }

    /// Tags the current save record as `tag` (unless it already is) and uploads the tag. Refuses a
    /// tag that already names another record.
    pub fn push_tag(&self, repo_path: &str, tag: &str, token: Option<&str>) -> Result<(), String> {
        let head = self.run_git(repo_path, &["rev-parse", "HEAD"])?.trim().to_string();
        match self.tag_commit(repo_path, tag) {
            Some(existing) if existing != head => return Err(format!("TAG_EXISTS:{tag}")),
            Some(_) => {}
            None => {
                self.run_git(repo_path, &["tag", tag, &head])?;
            }
        }
        let header = auth_header(token);
        let refspec = format!("refs/tags/{tag}:refs/tags/{tag}");
        self.run_git_with_env(repo_path, &["push", "origin", &refspec], &auth_envs(header.as_deref()))
            .map(|_| ())
    }
}
