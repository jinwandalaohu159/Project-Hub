//! Operation logs. Each project has its own, inside its `.git` folder so it moves with the project
//! (projects without one fall back to the app data folder). Project Hub itself keeps an app log
//! (startup, updates, account, errors) under the reserved id `APP_LOG_ID` in the app data folder.
//!
//! `location` decides where a log lives, `store` reads and writes it, `app` holds the helpers for
//! the app log; the commands the UI calls are below.

mod app;
mod location;
mod store;

pub(crate) use app::{app_log, app_log_on_change, install_panic_logger, log_outcome};
pub(crate) use location::project_log_dir;
pub(crate) use store::append_project_log;
use store::{load_log_settings, read_log_entries, update_log_settings, write_log_entry, LogEntry, LogSettings};

#[tauri::command]
pub(crate) async fn get_app_log_settings(project_id: String) -> Result<LogSettings, String> {
    tauri::async_runtime::spawn_blocking(move || load_log_settings(&project_id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn set_app_log_settings(
    project_id: String,
    max_mb: u64,
    retention_days: u64,
) -> Result<LogSettings, String> {
    tauri::async_runtime::spawn_blocking(move || update_log_settings(&project_id, max_mb, retention_days))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn write_app_log(
    project_id: String,
    project_name: String,
    action: String,
    message: String,
    level: Option<String>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let level = level.unwrap_or_default();
        write_log_entry(&project_id, &project_name, &level, &action, &message)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn read_app_logs(project_id: String, limit: usize) -> Result<Vec<LogEntry>, String> {
    tauri::async_runtime::spawn_blocking(move || read_log_entries(&project_id, limit))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn open_app_log_dir(project_id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let log_dir = project_log_dir(&project_id)?;
        std::fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;
        tauri_plugin_opener::open_path(log_dir, None::<&str>).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
