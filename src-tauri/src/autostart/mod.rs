//! Launch at login on every desktop platform.

#[cfg(target_os = "windows")]
mod windows;
// Always compiled so its entry builders stay tested on every platform.
mod unix;

pub(crate) fn is_enabled() -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    return windows::get_autostart_enabled_platform();
    #[cfg(not(target_os = "windows"))]
    return unix::is_enabled();
}

pub(crate) fn set_enabled(enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    return windows::set_autostart_enabled_platform(enabled);
    #[cfg(not(target_os = "windows"))]
    return unix::set_enabled(enabled);
}

#[tauri::command]
pub(crate) async fn get_autostart_enabled() -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(is_enabled)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn set_autostart_enabled(enabled: bool) -> Result<(), String> {
    let result = tauri::async_runtime::spawn_blocking(move || set_enabled(enabled))
        .await
        .map_err(|e| e.to_string())?;
    let action = if enabled { "autostart.enable" } else { "autostart.disable" };
    crate::logs::log_outcome(action, "", result)
}
