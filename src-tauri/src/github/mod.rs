//! GitHub: the REST calls Project Hub makes for the signed-in account. The token is kept in
//! `token`, repository addresses and names are tidied in `repo_url`, and releases (with the cloud
//! builds that make them) are in `releases`.

pub mod releases;
mod repo_url;
mod token;

use crate::settings::GithubAccount;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub use repo_url::{is_github_https, normalize_repo_url, sanitize_repo_name};
pub use token::{delete_token, load_token, store_token};

const API: &str = "https://api.github.com";
const USER_AGENT: &str = "Project-Hub";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoInfo {
    pub full_name: String,
    pub clone_url: String,
    pub html_url: String,
    pub private: bool,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout(Duration::from_secs(60))
        .build()
}

fn request(method: &str, path: &str, token: &str) -> ureq::Request {
    agent()
        .request(method, &format!("{API}{path}"))
        .set("Authorization", &format!("Bearer {token}"))
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .set("User-Agent", USER_AGENT)
}

fn api_error(error: ureq::Error) -> String {
    match error {
        ureq::Error::Status(401, _) => "GITHUB_UNAUTHORIZED".to_string(),
        ureq::Error::Status(code, response) => {
            let body = response.into_string().unwrap_or_default();
            let message = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|value| {
                    let base = value.get("message")?.as_str()?.to_string();
                    let detail = value
                        .get("errors")
                        .and_then(|errors| errors.get(0))
                        .and_then(|error| error.get("message"))
                        .and_then(|m| m.as_str())
                        .map(|m| format!(" ({m})"))
                        .unwrap_or_default();
                    Some(format!("{base}{detail}"))
                })
                .unwrap_or(body);
            format!("GITHUB_API_ERROR:{code} {message}")
        }
        ureq::Error::Transport(transport) => format!("Cannot reach GitHub: {transport}"),
    }
}

/// The sign-in for a project's cloud repository: a GitHub repository gets the signed-in account
/// (required to upload; to download only used when present, public repositories need none), any
/// other host none.
pub fn token_for_remote(remote_url: Option<&str>, required: bool) -> Result<Option<String>, String> {
    if !remote_url.is_some_and(is_github_https) {
        return Ok(None);
    }
    match load_token() {
        Some(token) => Ok(Some(token)),
        None if required => Err("GITHUB_NOT_SIGNED_IN".to_string()),
        None => Ok(None),
    }
}

pub fn fetch_user(token: &str) -> Result<GithubAccount, String> {
    request("GET", "/user", token)
        .call()
        .map_err(api_error)?
        .into_json::<GithubAccount>()
        .map_err(|e| e.to_string())
}

pub fn create_repo(token: &str, name: &str, description: &str, private: bool) -> Result<RepoInfo, String> {
    request("POST", "/user/repos", token)
        .send_json(serde_json::json!({
            "name": name,
            "description": description,
            "private": private,
            "auto_init": false,
        }))
        .map_err(api_error)?
        .into_json::<RepoInfo>()
        .map_err(|e| e.to_string())
}

pub fn list_repos(token: &str) -> Result<Vec<RepoInfo>, String> {
    request("GET", "/user/repos?per_page=100&sort=updated&affiliation=owner,collaborator", token)
        .call()
        .map_err(api_error)?
        .into_json::<Vec<RepoInfo>>()
        .map_err(|e| e.to_string())
}
