//! Project list commands.

use super::git::{identity_of, run_git_as};
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
    let versioning = ensure_versioned(&app, path.clone()).await?;
    let project = with_projects(app, move |service| service.add_project(&path)).await?;
    Ok(AddedProject { project, versioning })
}

/// Sets up a listed project that has no repository or no record yet (copied without `.git`,
/// interrupted setup, `git init` elsewhere).
#[tauri::command]
pub(crate) async fn ensure_project_versioned(app: tauri::AppHandle, id: String) -> Result<VersioningOutcome, String> {
    let lookup = id.clone();
    let project = with_projects(app.clone(), move |service| {
        service.get_project(&lookup).ok_or_else(|| format!("Project not found: {}", lookup))
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
    with_projects(app, move |service| service.remove_project(&id)).await
}

#[tauri::command]
pub(crate) async fn list_projects(app: tauri::AppHandle) -> Result<Vec<project_service::Project>, String> {
    with_projects(app, |service| Ok(service.list_projects())).await
}

#[tauri::command]
pub(crate) async fn auto_relocate_projects(
    app: tauri::AppHandle,
) -> Result<Vec<project_service::RelocatedProject>, String> {
    with_projects(app, |service| Ok(service.auto_relocate_missing())).await
}

#[tauri::command]
pub(crate) async fn relocate_project(
    app: tauri::AppHandle,
    id: String,
    path: String,
) -> Result<project_service::Project, String> {
    ensure_versioned(&app, path.clone()).await?;
    with_projects(app, move |service| service.relocate_project(&id, &path)).await
}
