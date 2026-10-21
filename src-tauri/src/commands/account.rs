//! Signing in to GitHub: in the browser, with a pasted token, and signing out.

use super::preferences::update_settings;
use crate::logs::{app_log, log_outcome};
use crate::{github, github_login, settings, AppState};
use tauri::Manager;

pub(crate) fn require_token() -> Result<String, String> {
    github::load_token().ok_or_else(|| "GITHUB_NOT_SIGNED_IN".to_string())
}

pub(crate) fn finish_login(app: &tauri::AppHandle, token: &str) -> Result<settings::GithubAccount, String> {
    let result = (|| -> Result<settings::GithubAccount, String> {
        let account = github::fetch_user(token)?;
        github::store_token(token)?;
        let state = app.state::<AppState>();
        update_settings(&state, |settings| settings.github = Some(account.clone()))?;
        Ok(account)
    })();
    match &result {
        Ok(account) => app_log("info", "account.login", &account.login),
        Err(error) => app_log("error", "account.login", error),
    }
    result
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
    let result = tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        update_settings(&state, |settings| settings.github = None)?;
        github::delete_token();
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?;
    log_outcome("account.logout", "", result)
}

#[tauri::command]
pub(crate) async fn github_list_repos() -> Result<Vec<github::RepoInfo>, String> {
    tauri::async_runtime::spawn_blocking(|| github::list_repos(&require_token()?))
        .await
        .map_err(|e| e.to_string())?
}
