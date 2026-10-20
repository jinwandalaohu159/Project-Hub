//! App preferences: who saves are recorded as.

use crate::settings::{self, AppSettings};
use crate::AppState;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SettingsView {
    author_name: String,
    author_email: String,
    effective_name: String,
    effective_email: String,
    github: Option<settings::GithubAccount>,
}

pub(crate) fn settings_view(settings: &AppSettings) -> SettingsView {
    let identity = settings.effective_identity().unwrap_or_default();
    SettingsView {
        author_name: settings.author_name.clone(),
        author_email: settings.author_email.clone(),
        effective_name: identity.name,
        effective_email: identity.email,
        github: settings.github.clone(),
    }
}

pub(crate) fn update_settings<T>(
    state: &AppState,
    change: impl FnOnce(&mut AppSettings) -> T,
) -> Result<(T, AppSettings), String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    let value = change(&mut settings);
    settings::save_settings(&settings)?;
    Ok((value, settings.clone()))
}

#[tauri::command]
pub(crate) fn get_app_settings(state: tauri::State<AppState>) -> Result<SettingsView, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(settings_view(&settings))
}

#[tauri::command]
pub(crate) fn update_author(
    name: String,
    email: String,
    state: tauri::State<AppState>,
) -> Result<SettingsView, String> {
    let email = email.trim().to_string();
    if !email.is_empty() && (!email.contains('@') || email.contains(char::is_whitespace)) {
        return Err("INVALID_EMAIL".to_string());
    }
    let (_, settings) = update_settings(&state, |settings| {
        settings.author_name = name.trim().to_string();
        settings.author_email = email;
    })?;
    Ok(settings_view(&settings))
}
