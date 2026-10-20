//! Export / import of everything Project Hub knows, for moving to another computer.

use super::git::identity_of;
use super::preferences::update_settings;
use crate::logs::project_log_dir;
use crate::{transfer, AppState};
use git_service::GitService;
use tauri::Manager;

#[tauri::command]
pub(crate) async fn export_data(
    app: tauri::AppHandle,
    path: String,
    include_history: bool,
) -> Result<transfer::ExportReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let projects = state
            .project_service
            .lock()
            .map_err(|e| e.to_string())?
            .list_projects();
        let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();
        let git = GitService::with_identity(settings.effective_identity());
        transfer::export(
            std::path::Path::new(&path),
            &projects,
            include_history,
            &settings.author_name,
            &settings.author_email,
            &git,
            |id| project_log_dir(id).ok(),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn preview_import(app: tauri::AppHandle, path: String) -> Result<transfer::ImportPreview, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let service = state.project_service.lock().map_err(|e| e.to_string())?;
        transfer::preview(std::path::Path::new(&path), &service)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) fn get_import_target_state(path: String) -> transfer::TargetState {
    transfer::target_state(&path)
}

#[tauri::command]
pub(crate) async fn apply_import(
    app: tauri::AppHandle,
    path: String,
    items: Vec<transfer::ImportItem>,
    import_author: bool,
) -> Result<transfer::ImportReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let identity = identity_of(&state);
        let git = GitService::with_identity(identity);
        let (report, manifest) = {
            let mut service = state.project_service.lock().map_err(|e| e.to_string())?;
            transfer::apply(std::path::Path::new(&path), &items, &mut service, &git, |id| {
                project_log_dir(id).ok()
            })?
        };
        if import_author {
            update_settings(&state, |settings| {
                if settings.author_name.trim().is_empty() {
                    settings.author_name = manifest.author_name.clone();
                }
                if settings.author_email.trim().is_empty() {
                    settings.author_email = manifest.author_email.clone();
                }
            })?;
        }
        Ok(report)
    })
    .await
    .map_err(|e| e.to_string())?
}
