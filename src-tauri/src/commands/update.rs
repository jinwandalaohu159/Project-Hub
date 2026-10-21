//! Checking for and installing new versions of Project Hub itself.

use crate::logs::{app_log, app_log_on_change};
use crate::update;
use serde::Serialize;
use std::time::Duration;
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

/// Downloads the new installer, starts it and quits; the installer reopens the updated app.
/// Progress goes out as `update-progress` events: `downloading` with byte counts, then `restarting`.
#[tauri::command]
pub(crate) async fn install_update(
    app: tauri::AppHandle,
    url: String,
    name: String,
    size: u64,
) -> Result<String, String> {
    let progress_app = app.clone();
    app_log("info", "update.install", &name);
    let result = tauri::async_runtime::spawn_blocking(move || {
        let emit = |phase: &str, downloaded: u64, total: u64| {
            let _ = progress_app.emit(
                UPDATE_PROGRESS_EVENT,
                serde_json::json!({ "phase": phase, "downloaded": downloaded, "total": total }),
            );
        };
        let installer = update::download(&url, &name, size, |downloaded, total| {
            emit("downloading", downloaded, total)
        })?;
        update::launch_installer(&installer)?;
        emit("restarting", size, size);
        // Let the window show that it is restarting before it disappears.
        std::thread::sleep(Duration::from_millis(1200));
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|inner| inner);
    if let Err(error) = &result {
        app_log("error", "update.install_failed", error);
    }
    result?;
    app_log("info", "update.restart", "quitting for the installer");
    app.exit(0);
    Ok("restarting".to_string())
}
