//! The automatic update when the app opens: with automatic updates on, a newer version found while
//! the app starts is installed right away by the helper, which then opens the new version.

use super::{
    app_location, check,
    handoff::{self, HelperJob, UpdateSource},
    outcome,
};
use crate::{logs::app_log, settings::AppSettings};
use std::{sync::mpsc, time::Duration};

/// How long opening the app waits for the update check before carrying on without it.
const CHECK_LIMIT: Duration = Duration::from_secs(3);

/// Hands over to the update helper when a newer version is out. Returns true when this process
/// should quit. A slow or failed check never holds the app up for more than `CHECK_LIMIT`.
pub fn hand_over(settings: &AppSettings, hidden: bool) -> bool {
    if cfg!(debug_assertions) || !settings.auto_update {
        return false;
    }
    let Some(app) = app_location() else {
        return false;
    };
    let current = env!("CARGO_PKG_VERSION").to_string();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(check(&current));
    });
    let Ok(Ok(info)) = receiver.recv_timeout(CHECK_LIMIT) else {
        return false;
    };
    if !info.available || outcome::skipped_version().as_deref() == Some(info.latest.as_str()) {
        return false;
    }
    let (Some(url), Some(name)) = (info.asset_url, info.asset_name) else {
        return false;
    };
    let job = HelperJob {
        version: info.latest.clone(),
        source: UpdateSource::Download { url, name, size: info.asset_size },
        app,
        parent_pid: std::process::id(),
        silent: hidden,
    };
    match handoff::spawn_helper(&job) {
        Ok(()) => {
            app_log("info", "update.auto", &format!("v{} -> v{}", info.current, info.latest));
            true
        }
        Err(error) => {
            app_log("warn", "update.auto_failed", &error);
            false
        }
    }
}
