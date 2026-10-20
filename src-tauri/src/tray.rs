//! Main window and tray icon.

use crate::trace_startup;
#[cfg(target_os = "windows")]
use std::time::Duration;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";

pub(crate) const TRAY_MENU_SHOW: &str = "tray-show";

pub(crate) const TRAY_MENU_SETTINGS: &str = "tray-settings";

pub(crate) const TRAY_MENU_EXIT: &str = "tray-exit";

pub(crate) const TRAY_ID: &str = "main";

pub(crate) const OPEN_SETTINGS_EVENT: &str = "open-settings";

pub(crate) fn ensure_main_window(app: &tauri::AppHandle, visible: bool) -> tauri::Result<WebviewWindow> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        return Ok(window);
    }

    let builder = WebviewWindowBuilder::new(app, MAIN_WINDOW_LABEL, WebviewUrl::default())
        .title("Project Hub")
        .inner_size(1200.0, 800.0)
        .min_inner_size(720.0, 480.0)
        .visible(visible);

    // The app draws its own title bar (with settings and debug next to the window buttons).
    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true);
    #[cfg(not(target_os = "macos"))]
    let builder = builder.decorations(false);

    builder.build()
}

pub(crate) fn show_main_window(app: &tauri::AppHandle) {
    if let Ok(window) = ensure_main_window(app, true) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub(crate) fn open_settings_window(app: &tauri::AppHandle) {
    show_main_window(app);
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let window_for_later = window.clone();
        let _ = window.emit(OPEN_SETTINGS_EVENT, ());
        tauri::async_runtime::spawn(async move {
            std::thread::sleep(std::time::Duration::from_millis(400));
            let _ = window_for_later.emit(OPEN_SETTINGS_EVENT, ());
        });
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn refresh_tray_registration(app: &tauri::AppHandle) -> bool {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        trace_startup("tray_retry:no_tray");
        return false;
    };

    let _ = tray.set_visible(true);
    let _ = tray.set_tooltip(Some("Project Hub"));
    if let Some(icon) = app.default_window_icon() {
        let _ = tray.set_icon(Some(icon.clone()));
    }

    let registered = tray.rect().ok().flatten().is_some();
    trace_startup(format!("tray_retry:registered={registered}"));
    registered
}

#[cfg(target_os = "windows")]
pub(crate) fn schedule_tray_registration_retries(app: &tauri::AppHandle) {
    let app = app.clone();

    std::thread::spawn(move || {
        let retry_seconds = [1_u64, 2, 5, 10, 20, 30, 45, 60, 90, 120, 180];
        let mut elapsed_seconds = 0_u64;

        for seconds in retry_seconds {
            std::thread::sleep(Duration::from_secs(seconds - elapsed_seconds));
            elapsed_seconds = seconds;

            if refresh_tray_registration(&app) {
                break;
            }
        }
    });
}

pub(crate) fn setup_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, TRAY_MENU_SHOW, "\u{6253}\u{5f00}", true, None::<&str>)?;
    let settings = MenuItem::with_id(
        app,
        TRAY_MENU_SETTINGS,
        "\u{8bbe}\u{7f6e}",
        true,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let exit = MenuItem::with_id(app, TRAY_MENU_EXIT, "\u{9000}\u{51fa}", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &settings, &separator, &exit])?;

    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Project Hub")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if event.id() == TRAY_MENU_SHOW {
                show_main_window(app);
            } else if event.id() == TRAY_MENU_SETTINGS {
                open_settings_window(app);
            } else if event.id() == TRAY_MENU_EXIT {
                app.exit(0);
            }
        })
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            }
            | TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } => show_main_window(tray.app_handle()),
            _ => {}
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    tray.build(app)?;
    Ok(())
}

pub(crate) fn should_start_hidden() -> bool {
    std::env::args().any(|arg| arg == "--hidden")
}
