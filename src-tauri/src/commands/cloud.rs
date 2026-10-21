//! GitHub sign-in and uploading projects.

use super::git::{identity_of, run_git_task};
use super::preferences::update_settings;
use crate::{github, github_login, settings, AppState};
use git_service::GitService;
use tauri::Manager;

pub(crate) fn require_token() -> Result<String, String> {
    github::load_token().ok_or_else(|| "GITHUB_NOT_SIGNED_IN".to_string())
}

pub(crate) fn finish_login(app: &tauri::AppHandle, token: &str) -> Result<settings::GithubAccount, String> {
    let account = github::fetch_user(token)?;
    github::store_token(token)?;
    let state = app.state::<AppState>();
    update_settings(&state, |settings| settings.github = Some(account.clone()))?;
    Ok(account)
}

/// Starts a browser sign-in and returns the code the user enters on github.com.
#[tauri::command]
pub(crate) async fn github_browser_login_start() -> Result<github_login::DeviceCode, String> {
    tauri::async_runtime::spawn_blocking(github_login::start)
        .await
        .map_err(|e| e.to_string())?
}

/// Waits until the user has approved the code from `github_browser_login_start`, then signs in.
#[tauri::command]
pub(crate) async fn github_browser_login_finish(app: tauri::AppHandle) -> Result<settings::GithubAccount, String> {
    tauri::async_runtime::spawn_blocking(move || finish_login(&app, &github_login::wait_for_token()?))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) fn github_browser_login_cancel() {
    github_login::cancel();
}

#[tauri::command]
pub(crate) async fn github_login_token(app: tauri::AppHandle, token: String) -> Result<settings::GithubAccount, String> {
    let token = token.trim().to_string();
    if token.is_empty() {
        return Err("GITHUB_TOKEN_EMPTY".to_string());
    }
    tauri::async_runtime::spawn_blocking(move || finish_login(&app, &token))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn github_logout(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        update_settings(&state, |settings| settings.github = None)?;
        github::delete_token();
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn github_list_repos() -> Result<Vec<github::RepoInfo>, String> {
    tauri::async_runtime::spawn_blocking(|| github::list_repos(&require_token()?))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn get_sync_state(repo_path: String) -> Result<git_service::SyncState, String> {
    run_git_task(move |svc| svc.sync_state(&repo_path)).await
}

#[tauri::command]
pub(crate) fn suggest_repo_name(name: String) -> String {
    github::sanitize_repo_name(&name)
}

pub(crate) fn push_with_account(svc: &GitService, repo_path: &str, force: bool) -> Result<(), String> {
    let url = svc.get_remote_url(repo_path, "origin").unwrap_or_default();
    let token = if github::is_github_https(&url) {
        Some(require_token()?)
    } else {
        None
    };
    svc.push(repo_path, token.as_deref(), force)
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
    run_git_task(move |svc| push_with_account(&svc, &repo_path, force)).await
}
