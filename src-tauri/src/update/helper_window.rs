//! The update helper as a program: a small progress window (none when the app was in the tray)
//! while `helper::run_job` works, then it quits.

use super::{
    handoff::HelperJob,
    helper::{self, HelperProgress},
};
use serde::Serialize;
use std::{sync::Mutex, time::Duration};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

pub const PROGRESS_EVENT: &str = "update-helper-progress";
const WINDOW_LABEL: &str = "updater";
/// Lets the progress bar fill before the window goes away.
const DONE_LINGER: Duration = Duration::from_millis(900);

struct HelperState {
    version: String,
    progress: Mutex<HelperProgress>,
}

#[derive(Serialize)]
struct HelperView {
    version: String,
    progress: HelperProgress,
}

/// What the progress window shows when it opens; later changes arrive as events.
#[tauri::command]
fn update_helper_state(state: tauri::State<HelperState>) -> Result<HelperView, String> {
    let progress = state.progress.lock().map_err(|e| e.to_string())?.clone();
    Ok(HelperView { version: state.version.clone(), progress })
}

/// Runs this process as the update helper.
pub fn run(job: HelperJob, context: tauri::Context<tauri::Wry>) {
    if job.silent {
        helper::run_job(&job, |_| {});
        return;
    }
    let state = HelperState { version: job.version.clone(), progress: Mutex::new(HelperProgress::first(&job)) };
    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![update_helper_state])
        .setup(move |app| {
            WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::default())
                .title("Project Hub")
                .inner_size(400.0, 156.0)
                .resizable(false)
                .maximizable(false)
                .center()
                // Shown by the page once it has drawn itself, so it never flashes white.
                .visible(false)
                // A progress card: nothing to close or resize, on any platform.
                .decorations(false)
                .initialization_script("window.__PROJECT_HUB_UPDATER__ = true;")
                .build()?;

            let handle = app.handle().clone();
            std::thread::spawn(move || {
                helper::run_job(&job, |progress| {
                    if let Ok(mut current) = handle.state::<HelperState>().progress.lock() {
                        *current = progress.clone();
                    }
                    let _ = handle.emit(PROGRESS_EVENT, &progress);
                });
                std::thread::sleep(DONE_LINGER);
                handle.exit(0);
            });
            Ok(())
        })
        .run(context)
        .expect("error while running the update helper");
}
