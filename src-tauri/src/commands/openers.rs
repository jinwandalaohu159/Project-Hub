//! Opening project folders in terminals, editors and the file manager.

use crate::commands::preferences::update_settings;
use crate::openers::{self, OpenerView};
use crate::AppState;
use std::path::Path;

#[tauri::command]
pub(crate) fn list_openers(state: tauri::State<AppState>) -> Result<Vec<OpenerView>, String> {
    let prefs = state.settings.lock().map_err(|e| e.to_string())?.openers.clone();
    Ok(openers::list(&prefs))
}

#[tauri::command]
pub(crate) fn open_folder_with(id: String, path: String, state: tauri::State<AppState>) -> Result<(), String> {
    let prefs = state.settings.lock().map_err(|e| e.to_string())?.openers.clone();
    openers::open(&prefs, &id, Path::new(&path))
}

#[tauri::command]
pub(crate) fn set_default_opener(id: String, state: tauri::State<AppState>) -> Result<Vec<OpenerView>, String> {
    change(&state, |prefs| {
        openers::set_hidden(prefs, &id, false);
        prefs.default = Some(id);
        Ok(())
    })
}

#[tauri::command]
pub(crate) fn set_opener_hidden(id: String, hidden: bool, state: tauri::State<AppState>) -> Result<Vec<OpenerView>, String> {
    change(&state, |prefs| {
        openers::set_hidden(prefs, &id, hidden);
        Ok(())
    })
}

#[tauri::command]
pub(crate) fn add_custom_opener(
    name: String,
    program: String,
    args: String,
    state: tauri::State<AppState>,
) -> Result<Vec<OpenerView>, String> {
    change(&state, |prefs| openers::add_custom(prefs, &name, &program, &args).map(|_| ()))
}

#[tauri::command]
pub(crate) fn remove_custom_opener(id: String, state: tauri::State<AppState>) -> Result<Vec<OpenerView>, String> {
    change(&state, |prefs| {
        openers::remove_custom(prefs, &id);
        Ok(())
    })
}

/// Applies a change to the opener choices, saves it, and returns the updated list.
fn change(
    state: &AppState,
    apply: impl FnOnce(&mut openers::OpenerPrefs) -> Result<(), String>,
) -> Result<Vec<OpenerView>, String> {
    let (result, settings) = update_settings(state, |settings| apply(&mut settings.openers))?;
    result?;
    Ok(openers::list(&settings.openers))
}
