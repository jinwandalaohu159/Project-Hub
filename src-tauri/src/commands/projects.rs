//! Project list commands.

use super::git::{identity_of, run_git_as};
use crate::logs::log_outcome;
use crate::AppState;
use git_service::VersioningOutcome;
use project_service::{Project, ProjectService};
use serde::Serialize;
use tauri::Manager;

/// Project list access off the UI thread: imports and relocation scans hold the lock for a while.
pub(crate) async fn with_projects<T, F>(app: tauri::AppHandle, task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&mut ProjectService) -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let mut service = state.project_service.lock().map_err(|e| e.to_string())?;
        task(&mut service)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Puts `path` under version management (repository and first record) before it is used.
async fn ensure_versioned(app: &tauri::AppHandle, path: String) -> Result<VersioningOutcome, String> {
    let identity = identity_of(&app.state::<AppState>());
    run_git_as(identity, move |svc| svc.ensure_versioned(&path)).await
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AddedProject {
    project: Project,
    versioning: VersioningOutcome,
}

#[tauri::command]
pub(crate) async fn add_project(app: tauri::AppHandle, path: String) -> Result<AddedProject, String> {
    let detail = path.clone();
    let result = async {
        let versioning = ensure_versioned(&app, path.clone()).await?;
        let project = with_projects(app, move |service| service.add_project(&path)).await?;
        Ok(AddedProject { project, versioning })
    }
    .await;
    log_outcome("project.add", &detail, result)
}

/// Sets up a listed project that has no repository or no record yet (copied without `.git`,
/// interrupted setup, `git init` elsewhere).
#[tauri::command]
pub(crate) async fn ensure_project_versioned(app: tauri::AppHandle, id: String) -> Result<VersioningOutcome, String> {
    let lookup = id.clone();
    let project = with_projects(app.clone(), move |service| {
        service.get_project(&lookup).ok_or_else(|| format!("PROJECT_NOT_FOUND:{lookup}"))
    })
    .await?;
    let outcome = ensure_versioned(&app, project.path).await?;
    if outcome.initialized {
        with_projects(app, move |service| {
            service.stamp_marker(&id);
            Ok(())
        })
        .await?;
    }
    Ok(outcome)
}

#[tauri::command]
pub(crate) async fn remove_project(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let result = with_projects(app, move |service| {
        let label = service
            .get_project(&id)
            .map(|project| format!("{} ({})", project.name, project.path))
            .unwrap_or_else(|| id.clone());
        service.remove_project(&id).map(|_| label)
    })
    .await;
    match result {
        Ok(label) => log_outcome("project.remove", &label, Ok(())),
        Err(error) => log_outcome("project.remove", "", Err(error)),
    }
}

#[tauri::command]
pub(crate) async fn list_projects(app: tauri::AppHandle) -> Result<Vec<project_service::Project>, String> {
    with_projects(app, |service| Ok(service.list_projects())).await
}

#[tauri::command]
pub(crate) async fn auto_relocate_projects(
    app: tauri::AppHandle,
) -> Result<Vec<project_service::RelocatedProject>, String> {
    let relocated = with_projects(app, |service| Ok(service.auto_relocate_missing())).await?;
    for project in &relocated {
        crate::logs::app_log(
            "info",
            "project.relocate.auto",
            &format!("{}: {} -> {}", project.name, project.old_path, project.new_path),
        );
    }
    Ok(relocated)
}

#[tauri::command]
pub(crate) async fn relocate_project(
    app: tauri::AppHandle,
    id: String,
    path: String,
) -> Result<project_service::Project, String> {
    let detail = path.clone();
    let result = async {
        ensure_versioned(&app, path.clone()).await?;
        with_projects(app, move |service| service.relocate_project(&id, &path)).await
    }
    .await;
    log_outcome("project.relocate", &detail, result)
}
