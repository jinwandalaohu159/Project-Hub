//! A project's cloud repository: creating or linking it, uploading and downloading.

use super::account::require_token;
use super::git::{identity_of, run_git_task};
use crate::{github, AppState};
use git_service::GitService;

#[tauri::command]
pub(crate) async fn get_sync_state(repo_path: String) -> Result<git_service::SyncState, String> {
    run_git_task(move |svc| svc.sync_state(&repo_path)).await
}

#[tauri::command]
pub(crate) fn suggest_repo_name(name: String) -> String {
    github::sanitize_repo_name(&name)
}

/// The GitHub sign-in for the project's cloud repository. Uploading always needs one; downloading
/// only from a private repository, so there it is used when present.
fn github_token(svc: &GitService, repo_path: &str, required: bool) -> Result<Option<String>, String> {
    let url = svc.get_remote_url(repo_path, "origin").unwrap_or_default();
    if !github::is_github_https(&url) {
        return Ok(None);
    }
    if required {
        require_token().map(Some)
    } else {
        Ok(github::load_token())
    }
}

#[tauri::command]
pub(crate) async fn publish_to_github(
    state: tauri::State<'_, AppState>,
    repo_path: String,
    name: String,
    description: String,
    private: bool,
) -> Result<github::RepoInfo, String> {
    let identity = identity_of(&state);
    tauri::async_runtime::spawn_blocking(move || {
        let token = require_token()?;
        let repo = github::create_repo(&token, &github::sanitize_repo_name(&name), description.trim(), private)?;
        let svc = GitService::with_identity(identity);
        svc.set_remote_url(&repo_path, "origin", &repo.clone_url)?;
        svc.push(&repo_path, Some(&token), false)?;
        Ok(repo)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn link_remote(repo_path: String, url: String) -> Result<String, String> {
    let url = github::normalize_repo_url(&url)?;
    run_git_task(move |svc| {
        svc.set_remote_url(&repo_path, "origin", &url)?;
        Ok(url)
    })
    .await
}

#[tauri::command]
pub(crate) async fn push_to_remote(repo_path: String, force: bool) -> Result<(), String> {
    run_git_task(move |svc| svc.push(&repo_path, github_token(&svc, &repo_path, true)?.as_deref(), force)).await
}

/// Asks the cloud what it has now and compares with this computer.
#[tauri::command]
pub(crate) async fn fetch_remote(repo_path: String) -> Result<git_service::SyncState, String> {
    run_git_task(move |svc| {
        svc.fetch(&repo_path, github_token(&svc, &repo_path, false)?.as_deref())?;
        svc.sync_state(&repo_path)
    })
    .await
}

#[tauri::command]
pub(crate) async fn pull_from_remote(repo_path: String) -> Result<git_service::PullOutcome, String> {
    run_git_task(move |svc| svc.pull(&repo_path, github_token(&svc, &repo_path, false)?.as_deref())).await
}

/// Takes the cloud's version, keeping this computer's own records as a new version; returns its name.
#[tauri::command]
pub(crate) async fn use_cloud_version(state: tauri::State<'_, AppState>, repo_path: String) -> Result<String, String> {
    let identity = identity_of(&state);
    tauri::async_runtime::spawn_blocking(move || {
        let svc = GitService::with_identity(identity);
        svc.use_cloud_version(&repo_path, github_token(&svc, &repo_path, false)?.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}
