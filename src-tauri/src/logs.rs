//! Per-project operation logs. They live inside each project's `.git` folder so they move with it;
//! projects without one fall back to the app data folder.

use crate::settings;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) const LOG_DIR_NAME: &str = "logs";

pub(crate) const LOG_PROJECTS_DIR_NAME: &str = "projects";

pub(crate) const LOG_FILE_NAME: &str = "project-hub.log";

pub(crate) const LOG_SETTINGS_FILE_NAME: &str = "log-settings.json";

pub(crate) const DEFAULT_LOG_MAX_MB: u64 = 50;

pub(crate) const DEFAULT_LOG_RETENTION_DAYS: u64 = 7;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AppLogSettings {
    max_mb: u64,
    retention_days: u64,
    log_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AppLogEntry {
    timestamp_ms: u64,
    level: String,
    project_id: String,
    project_name: String,
    action: String,
    message: String,
}

pub(crate) fn app_install_dir() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    exe.parent()
        .map(PathBuf::from)
        .ok_or_else(|| "APP_DIR_UNKNOWN".to_string())
}

/// Where logs lived before they moved into each project; only read for migration.
pub(crate) fn legacy_log_dir(project_id: &str) -> Option<PathBuf> {
    Some(
        app_install_dir()
            .ok()?
            .join(LOG_DIR_NAME)
            .join(LOG_PROJECTS_DIR_NAME)
            .join(safe_log_project_id(project_id)),
    )
}

/// Fallback for projects whose folder is missing or has no version history yet.
pub(crate) fn central_log_dir(project_id: &str) -> PathBuf {
    settings::app_data_dir()
        .join(LOG_DIR_NAME)
        .join(safe_log_project_id(project_id))
}

/// Project folder from the registry on disk. Reading the file (instead of the in-memory list)
/// keeps this usable while an import holds the project lock, and from the `--mcp` process.
pub(crate) fn registered_project_path(project_id: &str) -> Option<PathBuf> {
    let content = fs::read_to_string(settings::projects_storage_path()).ok()?;
    let projects: Vec<project_service::Project> = serde_json::from_str(&content).ok()?;
    projects
        .into_iter()
        .find(|project| project.id == project_id)
        .map(|project| PathBuf::from(project.path))
}

/// Logs travel with the project: they live inside its `.git` folder, invisible to the file tree
/// and never part of a save. Older locations are merged in on first use.
pub(crate) fn resolve_log_dir(project_path: Option<&std::path::Path>, central: PathBuf, legacy: Option<PathBuf>) -> PathBuf {
    let dir = match project_path {
        Some(path) if path.join(".git").is_dir() => path.join(".git").join("project-hub").join("logs"),
        _ => central.clone(),
    };
    let mut sources: Vec<PathBuf> = legacy.into_iter().collect();
    if central != dir {
        sources.push(central);
    }
    for source in sources {
        migrate_log_files(&source, &dir);
    }
    dir
}

pub(crate) fn migrate_log_files(source: &std::path::Path, dest: &std::path::Path) {
    let Ok(entries) = fs::read_dir(source) else {
        return;
    };
    for entry in entries.flatten() {
        let from = entry.path();
        if !from.is_file() {
            continue;
        }
        let to = dest.join(entry.file_name());
        if to.exists() || fs::create_dir_all(dest).is_err() {
            continue;
        }
        if fs::rename(&from, &to).is_err() && fs::copy(&from, &to).is_ok() {
            let _ = fs::remove_file(&from);
        }
    }
    let _ = fs::remove_dir(source);
}

pub(crate) fn safe_log_project_id(project_id: &str) -> String {
    let sanitized = project_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();

    if sanitized.trim_matches('_').is_empty() {
        "default".to_string()
    } else {
        sanitized
    }
}

pub(crate) fn project_log_dir(project_id: &str) -> Result<PathBuf, String> {
    Ok(resolve_log_dir(
        registered_project_path(project_id).as_deref(),
        central_log_dir(project_id),
        legacy_log_dir(project_id),
    ))
}

pub(crate) fn project_log_path(project_id: &str) -> Result<PathBuf, String> {
    Ok(project_log_dir(project_id)?.join(LOG_FILE_NAME))
}

pub(crate) fn project_log_settings_path(project_id: &str) -> Result<PathBuf, String> {
    Ok(project_log_dir(project_id)?.join(LOG_SETTINGS_FILE_NAME))
}

pub(crate) fn default_app_log_settings(project_id: &str) -> Result<AppLogSettings, String> {
    Ok(AppLogSettings {
        max_mb: DEFAULT_LOG_MAX_MB,
        retention_days: DEFAULT_LOG_RETENTION_DAYS,
        log_dir: project_log_dir(project_id)?.display().to_string(),
    })
}

pub(crate) fn load_app_log_settings(project_id: &str) -> Result<AppLogSettings, String> {
    let default = default_app_log_settings(project_id)?;
    let path = project_log_settings_path(project_id)?;
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return Ok(default),
    };

    let mut settings: AppLogSettings = serde_json::from_str(&content).unwrap_or(default);
    settings.max_mb = settings.max_mb.clamp(1, 1024);
    settings.retention_days = settings.retention_days.clamp(1, 3650);
    settings.log_dir = project_log_dir(project_id)?.display().to_string();
    Ok(settings)
}

pub(crate) fn save_app_log_settings(project_id: &str, settings: &AppLogSettings) -> Result<(), String> {
    let log_dir = project_log_dir(project_id)?;
    fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;
    let content = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(project_log_settings_path(project_id)?, content).map_err(|e| e.to_string())
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

pub(crate) fn prune_app_logs(project_id: &str) -> Result<(), String> {
    let settings = load_app_log_settings(project_id)?;
    let path = project_log_path(project_id)?;
    if !path.exists() {
        return Ok(());
    }

    let mut content = String::new();
    OpenOptions::new()
        .read(true)
        .open(&path)
        .map_err(|e| e.to_string())?
        .read_to_string(&mut content)
        .map_err(|e| e.to_string())?;

    let cutoff_ms = now_ms().saturating_sub(settings.retention_days * 24 * 60 * 60 * 1000);
    let mut retained = Vec::new();
    for line in content.lines() {
        let keep = serde_json::from_str::<AppLogEntry>(line)
            .map(|entry| entry.timestamp_ms >= cutoff_ms)
            .unwrap_or(true);
        if keep {
            retained.push(line.to_string());
        }
    }

    let max_bytes = (settings.max_mb * 1024 * 1024) as usize;
    let mut total_bytes = retained.iter().map(|line| line.len() + 1).sum::<usize>();
    while total_bytes > max_bytes && !retained.is_empty() {
        let removed = retained.remove(0);
        total_bytes = total_bytes.saturating_sub(removed.len() + 1);
    }

    let mut output = retained.join("\n");
    if !output.is_empty() {
        output.push('\n');
    }
    fs::write(path, output).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn get_app_log_settings(project_id: String) -> Result<AppLogSettings, String> {
    tauri::async_runtime::spawn_blocking(move || load_app_log_settings(&project_id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn set_app_log_settings(
    project_id: String,
    max_mb: u64,
    retention_days: u64,
) -> Result<AppLogSettings, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut settings = default_app_log_settings(&project_id)?;
        settings.max_mb = max_mb.clamp(1, 1024);
        settings.retention_days = retention_days.clamp(1, 3650);
        save_app_log_settings(&project_id, &settings)?;
        prune_app_logs(&project_id)?;
        load_app_log_settings(&project_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

pub(crate) fn write_log_entry(project_id: &str, project_name: &str, action: &str, message: &str) -> Result<(), String> {
    let log_dir = project_log_dir(project_id)?;
    fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;
    let entry = AppLogEntry {
        timestamp_ms: now_ms(),
        level: "info".to_string(),
        project_id: project_id.to_string(),
        project_name: project_name.to_string(),
        action: action.to_string(),
        message: message.to_string(),
    };
    let line = serde_json::to_string(&entry).map_err(|e| e.to_string())?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(project_log_path(project_id)?)
        .map_err(|e| e.to_string())?;
    writeln!(file, "{line}").map_err(|e| e.to_string())?;
    prune_app_logs(project_id)
}

/// Best-effort logging for background paths (MCP) where errors must not surface.
pub(crate) fn append_app_log(project_id: &str, project_name: &str, action: &str, message: &str) {
    let _ = write_log_entry(project_id, project_name, action, message);
}

#[tauri::command]
pub(crate) async fn write_app_log(
    project_id: String,
    project_name: String,
    action: String,
    message: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        write_log_entry(&project_id, &project_name, &action, &message)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn read_app_logs(project_id: String, limit: usize) -> Result<Vec<AppLogEntry>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        prune_app_logs(&project_id)?;
        let path = project_log_path(&project_id)?;
        let content = fs::read_to_string(path).unwrap_or_default();
        let mut entries = content
            .lines()
            .rev()
            .filter_map(|line| serde_json::from_str::<AppLogEntry>(line).ok())
            .take(limit.clamp(1, 1000))
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| std::cmp::Reverse(entry.timestamp_ms));
        Ok(entries)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn open_app_log_dir(project_id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let log_dir = project_log_dir(&project_id)?;
        fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;
        tauri_plugin_opener::open_path(log_dir, None::<&str>).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod log_location_tests {
    use super::*;

    #[test]
    fn logs_move_into_the_project_and_follow_it() {
        let root = tempfile::TempDir::new().unwrap();
        let project = root.path().join("demo");
        fs::create_dir_all(project.join(".git")).unwrap();
        let legacy = root.path().join("install").join("logs").join("p1");
        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join(LOG_FILE_NAME), "old line\n").unwrap();
        let central = root.path().join("appdata").join("logs").join("p1");

        let dir = resolve_log_dir(Some(&project), central.clone(), Some(legacy.clone()));
        assert_eq!(dir, project.join(".git").join("project-hub").join("logs"));
        assert_eq!(fs::read_to_string(dir.join(LOG_FILE_NAME)).unwrap(), "old line\n");
        assert!(!legacy.exists());

        // Missing folder: logs wait in the central store, then rejoin the project once it is found.
        let missing = root.path().join("gone");
        let fallback = resolve_log_dir(Some(&missing), central.clone(), None);
        assert_eq!(fallback, central);
        fs::create_dir_all(&central).unwrap();
        fs::write(central.join("extra.log"), "while missing\n").unwrap();
        let back = resolve_log_dir(Some(&project), central.clone(), None);
        assert!(back.join("extra.log").exists());
        assert!(!central.exists());
    }
}
