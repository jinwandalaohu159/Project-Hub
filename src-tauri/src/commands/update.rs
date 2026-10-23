//! Checking for and installing new versions of Project Hub itself.

use crate::commands::preferences::update_settings;
use crate::logs::{app_log, app_log_on_change};
use crate::update::{
    self,
    handoff::{self, HelperJob, UpdateSource},
    outcome::{self, UpdateOutcome},
};
use crate::AppState;
use serde::Serialize;
use tauri::Emitter;

pub(crate) const UPDATE_PROGRESS_EVENT: &str = "update-progress";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AppVersionInfo {
    version: String,
    releases_url: String,
}

#[tauri::command]
pub(crate) fn get_app_version(app: tauri::AppHandle) -> AppVersionInfo {
    AppVersionInfo {
        version: app.package_info().version.to_string(),
        releases_url: update::releases_page(),
    }
}

#[tauri::command]
pub(crate) async fn check_for_update(app: tauri::AppHandle) -> Result<update::UpdateInfo, String> {
    let current = app.package_info().version.to_string();
    let result = tauri::async_runtime::spawn_blocking(move || update::check(&current))
        .await
        .map_err(|e| e.to_string())?;
    // Checks run every few minutes; only a change of outcome is worth a line.
    match &result {
        Ok(info) if info.available => app_log_on_change(
            "update.check",
            "info",
            "update.available",
            &format!("v{} -> v{}", info.current, info.latest),
        ),
        Ok(info) => app_log_on_change("update.check", "info", "update.up_to_date", &format!("v{}", info.current)),
        Err(error) => app_log_on_change("update.check", "warn", "update.check_failed", error),
    }
    result
}

/// Downloads the new installer, hands it to the update helper and quits; the helper's window shows
/// the restart and opens the updated app. Download progress goes out as `update-progress` events:
/// `downloading` with byte counts, then `restarting`.
#[tauri::command]
pub(crate) async fn install_update(
    app: tauri::AppHandle,
    version: String,
    url: String,
    name: String,
    size: u64,
) -> Result<String, String> {
    let progress_app = app.clone();
    app_log("info", "update.install", &name);
    let result = tauri::async_runtime::spawn_blocking(move || {
        let app_path = update::app_location().ok_or_else(self_update_unavailable)?;
        let emit = |phase: &str, downloaded: u64, total: u64| {
            let _ = progress_app.emit(
                UPDATE_PROGRESS_EVENT,
                serde_json::json!({ "phase": phase, "downloaded": downloaded, "total": total }),
            );
        };
        let installer = update::download(&url, &name, size, |downloaded, total| {
            emit("downloading", downloaded, total)
        })?;
        emit("restarting", size, size);
        handoff::spawn_helper(&HelperJob {
            version,
            source: UpdateSource::Installer { path: installer },
            app: app_path,
            parent_pid: std::process::id(),
            silent: false,
        })
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|inner| inner);
    if let Err(error) = &result {
        app_log("error", "update.install_failed", error);
    }
    result?;
    app_log("info", "update.restart", "quitting for the update helper");
    app.exit(0);
    Ok("restarting".to_string())
}

fn self_update_unavailable() -> String {
    if cfg!(all(unix, not(target_os = "macos"))) {
        "UPDATE_NEEDS_APPIMAGE".to_string()
    } else {
        "UPDATE_INSTALL_FAILED:this copy of Project Hub cannot update itself".to_string()
    }
}

/// How the last update went, once: the app tells the user after it reopens.
#[tauri::command]
pub(crate) fn take_update_outcome() -> Option<UpdateOutcome> {
    let outcome = outcome::take()?;
    match &outcome.error {
        None => app_log("info", "update.installed", &format!("v{}", outcome.version)),
        Some(error) => app_log("error", "update.install_failed", &format!("v{}: {error}", outcome.version)),
    }
    Some(outcome)
}

#[tauri::command]
pub(crate) fn get_auto_update(state: tauri::State<AppState>) -> Result<bool, String> {
    Ok(state.settings.lock().map_err(|e| e.to_string())?.auto_update)
}

#[tauri::command]
pub(crate) fn set_auto_update(enabled: bool, state: tauri::State<AppState>) -> Result<bool, String> {
    update_settings(&state, |settings| settings.auto_update = enabled).map(|_| enabled)
}
