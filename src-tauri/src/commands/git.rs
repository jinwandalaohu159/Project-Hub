//! Version management commands. Git work runs off the UI thread.

use crate::AppState;
use git_service::GitService;

pub(crate) async fn run_git_task<T, F>(task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(GitService) -> Result<T, String> + Send + 'static,
{
    run_git_as(None, task).await
}

/// Runs git work off the UI thread, committing as `identity` when one is configured.
pub(crate) async fn run_git_as<T, F>(identity: Option<git_service::Identity>, task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(GitService) -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || task(GitService::with_identity(identity)))
        .await
        .map_err(|e| e.to_string())?
}

pub(crate) fn identity_of(state: &AppState) -> Option<git_service::Identity> {
    state.settings.lock().ok().and_then(|settings| settings.effective_identity())
}

#[tauri::command]
pub(crate) async fn get_branches(repo_path: String) -> Result<Vec<git_service::Branch>, String> {
    run_git_task(move |svc| svc.get_branches(&repo_path)).await
}

#[tauri::command]
pub(crate) async fn get_current_branch(repo_path: String) -> Result<String, String> {
    run_git_task(move |svc| svc.get_current_branch(&repo_path)).await
}

#[tauri::command]
pub(crate) async fn switch_branch(repo_path: String, branch_name: String) -> Result<(), String> {
    run_git_task(move |svc| svc.switch_branch(&repo_path, &branch_name)).await
}

#[tauri::command]
pub(crate) async fn get_git_status(repo_path: String) -> Result<Vec<git_service::FileDiff>, String> {
    run_git_task(move |svc| svc.get_status(&repo_path)).await
}

#[tauri::command]
pub(crate) async fn get_git_status_summary(
    repo_path: String,
    limit: usize,
) -> Result<git_service::StatusSummary, String> {
    run_git_task(move |svc| svc.get_status_summary(&repo_path, limit)).await
}

#[tauri::command]
pub(crate) async fn has_unsaved_changes(repo_path: String) -> Result<bool, String> {
    run_git_task(move |svc| svc.has_unsaved_changes(&repo_path)).await
}

#[tauri::command]
pub(crate) async fn git_stash(repo_path: String) -> Result<(), String> {
    run_git_task(move |svc| svc.stash(&repo_path)).await
}

#[tauri::command]
pub(crate) async fn git_stash_pop(repo_path: String) -> Result<(), String> {
    run_git_task(move |svc| svc.stash_pop(&repo_path)).await
}

#[tauri::command]
pub(crate) async fn discard_changes(state: tauri::State<'_, AppState>, repo_path: String) -> Result<(), String> {
    run_git_as(identity_of(&state), move |svc| svc.discard_changes(&repo_path)).await
}

#[tauri::command]
pub(crate) async fn git_add(repo_path: String, paths: Vec<String>) -> Result<(), String> {
    run_git_task(move |svc| {
        let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
        svc.add_files(&repo_path, &path_refs)
    })
    .await
}

#[tauri::command]
pub(crate) async fn git_add_all(repo_path: String) -> Result<(), String> {
    run_git_task(move |svc| svc.add_all(&repo_path)).await
}

#[tauri::command]
pub(crate) async fn git_unstage(repo_path: String, paths: Vec<String>) -> Result<(), String> {
    run_git_task(move |svc| {
        let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
        svc.unstage_files(&repo_path, &path_refs)
    })
    .await
}

#[tauri::command]
pub(crate) async fn git_commit(state: tauri::State<'_, AppState>, repo_path: String, message: String) -> Result<(), String> {
    run_git_as(identity_of(&state), move |svc| svc.commit(&repo_path, &message)).await
}

#[tauri::command]
pub(crate) async fn git_save_version(
    state: tauri::State<'_, AppState>,
    repo_path: String,
    message: String,
) -> Result<(), String> {
    run_git_as(identity_of(&state), move |svc| svc.save_version(&repo_path, &message)).await
}

#[tauri::command]
pub(crate) async fn create_branch(
    state: tauri::State<'_, AppState>,
    repo_path: String,
    name: String,
    switch: bool,
) -> Result<(), String> {
    run_git_as(identity_of(&state), move |svc| {
        if switch {
            svc.create_and_switch_branch(&repo_path, &name)
        } else {
            svc.create_branch(&repo_path, &name)
        }
    })
    .await
}

#[tauri::command]
pub(crate) async fn delete_branch(repo_path: String, name: String, force: bool) -> Result<(), String> {
    run_git_task(move |svc| svc.delete_branch(&repo_path, &name, force)).await
}

#[tauri::command]
pub(crate) async fn merge_branch(
    state: tauri::State<'_, AppState>,
    repo_path: String,
    source_branch: String,
) -> Result<(), String> {
    run_git_as(identity_of(&state), move |svc| svc.merge_branch(&repo_path, &source_branch)).await
}

#[tauri::command]
pub(crate) async fn has_merge_conflicts(repo_path: String) -> Result<bool, String> {
    run_git_task(move |svc| Ok(svc.has_merge_conflicts(&repo_path))).await
}

#[tauri::command]
pub(crate) async fn get_log(repo_path: String, count: usize) -> Result<Vec<git_service::CommitLog>, String> {
    run_git_task(move |svc| svc.get_log(&repo_path, count)).await
}

#[tauri::command]
pub(crate) async fn restore_commit(
    state: tauri::State<'_, AppState>,
    repo_path: String,
    commit_hash: String,
) -> Result<git_service::RestoreOutcome, String> {
    run_git_as(identity_of(&state), move |svc| svc.restore_commit(&repo_path, &commit_hash)).await
}

#[tauri::command]
pub(crate) async fn get_draft(repo_path: String) -> Result<Option<git_service::DraftInfo>, String> {
    run_git_task(move |svc| svc.get_draft(&repo_path)).await
}

#[tauri::command]
pub(crate) async fn restore_draft(
    state: tauri::State<'_, AppState>,
    repo_path: String,
) -> Result<git_service::RestoreOutcome, String> {
    run_git_as(identity_of(&state), move |svc| svc.restore_draft(&repo_path)).await
}

#[tauri::command]
pub(crate) async fn stash_to_draft(state: tauri::State<'_, AppState>, repo_path: String) -> Result<bool, String> {
    run_git_as(identity_of(&state), move |svc| svc.stash_to_draft(&repo_path)).await
}

#[tauri::command]
pub(crate) async fn discard_draft(repo_path: String) -> Result<(), String> {
    run_git_task(move |svc| svc.discard_draft(&repo_path)).await
}

#[tauri::command]
pub(crate) async fn get_repo_state(repo_path: String) -> Result<git_service::RepoState, String> {
    run_git_task(move |svc| svc.get_repo_state(&repo_path)).await
}

#[tauri::command]
pub(crate) async fn git_init(state: tauri::State<'_, AppState>, repo_path: String) -> Result<(), String> {
    run_git_as(identity_of(&state), move |svc| svc.init_repo(&repo_path)).await
}

#[tauri::command]
pub(crate) async fn is_git_repo(path: String) -> Result<bool, String> {
    run_git_task(move |svc| Ok(svc.is_git_repo(&path))).await
}
