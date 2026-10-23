//! GitHub Releases and Actions for the signed-in account: creating a release, attaching
//! installers, publishing it, and following the cloud build a version tag starts.

use super::{api_error, request, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::{fs::File, path::Path, time::Duration};

/// `owner/name` of a repository on GitHub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoRef {
    pub owner: String,
    pub name: String,
}

impl RepoRef {
    /// From a clone address such as https://github.com/owner/name.git.
    pub fn from_url(url: &str) -> Option<RepoRef> {
        let rest = url.trim().strip_prefix("https://github.com/")?;
        let rest = rest.trim_end_matches('/').trim_end_matches(".git");
        let (owner, name) = rest.split_once('/')?;
        (!owner.is_empty() && !name.is_empty() && !name.contains('/'))
            .then(|| RepoRef { owner: owner.to_string(), name: name.to_string() })
    }

    fn path(&self, rest: &str) -> String {
        format!("/repos/{}/{}{rest}", self.owner, self.name)
    }

    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub id: u64,
    pub name: String,
    pub size: u64,
    #[serde(default)]
    pub browser_download_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub id: u64,
    pub tag_name: String,
    #[serde(default)]
    pub name: Option<String>,
    pub draft: bool,
    pub prerelease: bool,
    pub html_url: String,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub assets: Vec<ReleaseAsset>,
}

/// A run of a GitHub Actions workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: u64,
    #[serde(default)]
    pub name: Option<String>,
    pub status: Option<String>,
    pub conclusion: Option<String>,
    pub html_url: String,
    #[serde(default)]
    pub head_sha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowJob {
    pub name: String,
    pub status: Option<String>,
    pub conclusion: Option<String>,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub completed_at: Option<String>,
}

/// The repository's releases, drafts included, newest first.
pub fn list_releases(token: &str, repo: &RepoRef) -> Result<Vec<Release>, String> {
    request("GET", &repo.path("/releases?per_page=30"), token)
        .call()
        .map_err(api_error)?
        .into_json()
        .map_err(|e| e.to_string())
}

/// The release for `tag`, draft or published. (GitHub's lookup by tag leaves drafts out.)
pub fn release_for_tag(token: &str, repo: &RepoRef, tag: &str) -> Result<Option<Release>, String> {
    Ok(list_releases(token, repo)?.into_iter().find(|release| release.tag_name == tag))
}

/// A draft release; the tag is created at `target` when it is published.
pub fn create_draft(token: &str, repo: &RepoRef, tag: &str, target: &str, title: &str, notes: &str, prerelease: bool) -> Result<Release, String> {
    request("POST", &repo.path("/releases"), token)
        .send_json(serde_json::json!({
            "tag_name": tag,
            "target_commitish": target,
            "name": title,
            "body": notes,
            "draft": true,
            "prerelease": prerelease,
        }))
        .map_err(api_error)?
        .into_json()
        .map_err(|e| e.to_string())
}

/// Attaches `file` to the release, replacing an earlier file of the same name.
pub fn upload_asset(token: &str, repo: &RepoRef, release: &Release, file: &Path) -> Result<ReleaseAsset, String> {
    let name = file
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| format!("Not a file: {}", file.display()))?;
    if let Some(old) = release.assets.iter().find(|asset| asset.name == name) {
        request("DELETE", &repo.path(&format!("/releases/assets/{}", old.id)), token)
            .call()
            .map_err(api_error)?;
    }
    let size = std::fs::metadata(file).map_err(|e| format!("{}: {e}", file.display()))?.len();
    let reader = File::open(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let url = format!(
        "https://uploads.github.com/repos/{}/{}/releases/{}/assets?name={}",
        repo.owner,
        repo.name,
        release.id,
        percent_encode(&name)
    );
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout(Duration::from_secs(60 * 60))
        .build()
        .post(&url)
        .set("Authorization", &format!("Bearer {token}"))
        .set("Accept", "application/vnd.github+json")
        .set("User-Agent", USER_AGENT)
        .set("Content-Type", "application/octet-stream")
        .set("Content-Length", &size.to_string())
        .send(reader)
        .map_err(api_error)?
        .into_json()
        .map_err(|e| e.to_string())
}

/// Makes a draft public (and the latest release, unless it is a pre-release).
pub fn publish(token: &str, repo: &RepoRef, release: &Release) -> Result<Release, String> {
    request("PATCH", &repo.path(&format!("/releases/{}", release.id)), token)
        .send_json(serde_json::json!({
            "draft": false,
            "make_latest": if release.prerelease { "false" } else { "true" },
        }))
        .map_err(api_error)?
        .into_json()
        .map_err(|e| e.to_string())
}

/// Workflow runs started by pushing `tag`, newest first.
pub fn runs_for_tag(token: &str, repo: &RepoRef, tag: &str) -> Result<Vec<WorkflowRun>, String> {
    #[derive(Deserialize)]
    struct Runs {
        workflow_runs: Vec<WorkflowRun>,
    }
    let path = repo.path(&format!("/actions/runs?event=push&per_page=10&branch={}", percent_encode(tag)));
    Ok(request("GET", &path, token)
        .call()
        .map_err(api_error)?
        .into_json::<Runs>()
        .map_err(|e| e.to_string())?
        .workflow_runs)
}

pub fn run_jobs(token: &str, repo: &RepoRef, run_id: u64) -> Result<Vec<WorkflowJob>, String> {
    #[derive(Deserialize)]
    struct Jobs {
        jobs: Vec<WorkflowJob>,
    }
    Ok(request("GET", &repo.path(&format!("/actions/runs/{run_id}/jobs?per_page=50")), token)
        .call()
        .map_err(api_error)?
        .into_json::<Jobs>()
        .map_err(|e| e.to_string())?
        .jobs)
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (byte as char).to_string(),
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repositories_come_from_clone_addresses() {
        assert_eq!(
            RepoRef::from_url("https://github.com/me/app.git"),
            Some(RepoRef { owner: "me".into(), name: "app".into() })
        );
        assert_eq!(RepoRef::from_url("https://github.com/me/app/").map(|r| r.full_name()), Some("me/app".into()));
        assert!(RepoRef::from_url("https://gitlab.com/me/app.git").is_none());
        assert!(RepoRef::from_url("https://github.com/me").is_none());
    }

    #[test]
    fn asset_names_are_encoded_for_the_upload_address() {
        assert_eq!(percent_encode("Project Hub_1.0.0_x64-setup.exe"), "Project%20Hub_1.0.0_x64-setup.exe");
        assert_eq!(percent_encode("v1.0.0"), "v1.0.0");
    }
}
