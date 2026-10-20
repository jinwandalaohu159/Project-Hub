//! Checking for and installing new versions of Project Hub itself.

use crate::update;
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
    tauri::async_runtime::spawn_blocking(move || update::check(&current))
        .await
        .map_err(|e| e.to_string())?
}

/// Downloads the new installer, starts it and quits; the installer reopens the updated app.
#[tauri::command]
pub(crate) async fn install_update(
    app: tauri::AppHandle,
    url: String,
    name: String,
    size: u64,
) -> Result<String, String> {
    let progress_app = app.clone();
    let installer = tauri::async_runtime::spawn_blocking(move || {
        update::download(&url, &name, size, |downloaded, total| {
            let _ = progress_app.emit(
                UPDATE_PROGRESS_EVENT,
                serde_json::json!({ "downloaded": downloaded, "total": total }),
            );
        })
    })
    .await
    .map_err(|e| e.to_string())??;
    update::launch_installer(&installer)?;
    app.exit(0);
    Ok("restarting".to_string())
}
