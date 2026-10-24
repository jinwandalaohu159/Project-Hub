//! Watching the open project's folder for changes made outside the app (see crate::worktree).

use crate::worktree::WorktreeWatch;
use crate::AppState;
use std::path::Path;

/// Watches `path` instead of whatever was watched before.
#[tauri::command]
pub(crate) async fn watch_worktree(app: tauri::AppHandle, state: tauri::State<'_, AppState>, path: String) -> Result<(), String> {
    let mut current = state.worktree.lock().map_err(|e| e.to_string())?;
    if current.as_ref().is_some_and(|watch| watch.root() == Path::new(&path)) {
        return Ok(());
    }
    // Stop the old watch first: two at once would only double the work.
    *current = None;
    *current = Some(WorktreeWatch::start(app, &path)?);
    Ok(())
}

/// Stops watching `path`; a watch of another folder, started since, stays.
#[tauri::command]
pub(crate) fn unwatch_worktree(state: tauri::State<'_, AppState>, path: String) -> Result<(), String> {
    let mut current = state.worktree.lock().map_err(|e| e.to_string())?;
    if current.as_ref().is_some_and(|watch| watch.root() == Path::new(&path)) {
        *current = None;
    }
    Ok(())
}
