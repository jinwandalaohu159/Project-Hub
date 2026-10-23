//! Where each log lives: inside the project's `.git` folder so it moves with the project, and the app
//! data folder for projects without one and for Project Hub's own log.

use crate::settings;
use std::{fs, path::PathBuf};

pub(crate) const LOG_DIR_NAME: &str = "logs";

pub(crate) const LOG_FILE_NAME: &str = "project-hub.log";

pub(crate) const LOG_SETTINGS_FILE_NAME: &str = "log-settings.json";

/// Log id of Project Hub itself; project ids are UUIDs, so it never clashes with one.
pub(crate) const APP_LOG_ID: &str = "project-hub-app";

const APP_LOG_DIR_NAME: &str = "app";

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
/// and never part of a save. Logs written to the central folder while the project was missing join
/// it again once it is found.
pub(crate) fn resolve_log_dir(project_path: Option<&std::path::Path>, central: PathBuf) -> PathBuf {
    let dir = match project_path {
        Some(path) if path.join(".git").is_dir() => path.join(".git").join("project-hub").join("logs"),
        _ => central.clone(),
    };
    if central != dir {
        migrate_log_files(&central, &dir);
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
    if project_id == APP_LOG_ID {
        return Ok(settings::app_data_dir().join(LOG_DIR_NAME).join(APP_LOG_DIR_NAME));
    }
    Ok(resolve_log_dir(
        registered_project_path(project_id).as_deref(),
        central_log_dir(project_id),
    ))
}

pub(crate) fn project_log_path(project_id: &str) -> Result<PathBuf, String> {
    Ok(project_log_dir(project_id)?.join(LOG_FILE_NAME))
}

pub(crate) fn project_log_settings_path(project_id: &str) -> Result<PathBuf, String> {
    Ok(project_log_dir(project_id)?.join(LOG_SETTINGS_FILE_NAME))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_log_has_its_own_folder() {
        let dir = project_log_dir(APP_LOG_ID).unwrap();
        assert_eq!(dir, settings::app_data_dir().join(LOG_DIR_NAME).join(APP_LOG_DIR_NAME));
        assert_ne!(dir, central_log_dir("92611742-2cef-494c-aa7b-409556a3b823"));
    }

    #[test]
    fn logs_live_in_the_project_and_follow_it() {
        let root = tempfile::TempDir::new().unwrap();
        let project = root.path().join("demo");
        fs::create_dir_all(project.join(".git")).unwrap();
        let central = root.path().join("appdata").join("logs").join("p1");

        let dir = resolve_log_dir(Some(&project), central.clone());
        assert_eq!(dir, project.join(".git").join("project-hub").join("logs"));

        // Missing folder: logs wait in the central store, then rejoin the project once it is found.
        let missing = root.path().join("gone");
        let fallback = resolve_log_dir(Some(&missing), central.clone());
        assert_eq!(fallback, central);
        fs::create_dir_all(&central).unwrap();
        fs::write(central.join("extra.log"), "while missing\n").unwrap();
        let back = resolve_log_dir(Some(&project), central.clone());
        assert!(back.join("extra.log").exists());
        assert!(!central.exists());
    }
}
