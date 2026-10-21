//! Project Hub's own log: startup, updates, account, import/export, errors and crashes. Every helper
//! is best effort, so logging never gets in the way of the operation it records.

use super::location::APP_LOG_ID;
use super::store::{write_log_entry, write_log_entry_unlocked, LOG_WRITE};

const APP_LOG_NAME: &str = "Project Hub";

/// Best-effort entry in Project Hub's own log.
pub(crate) fn app_log(level: &str, action: &str, message: &str) {
    let _ = write_log_entry(APP_LOG_ID, APP_LOG_NAME, level, action, message);
}

/// Records how an app-level operation ended and passes its result through. `detail` names what it
/// worked on; a failure adds the error.
pub(crate) fn log_outcome<T>(action: &str, detail: &str, result: Result<T, String>) -> Result<T, String> {
    match &result {
        Ok(_) => app_log("info", action, detail),
        Err(error) if detail.is_empty() => app_log("error", action, error),
        Err(error) => app_log("error", action, &format!("{detail}: {error}")),
    }
    result
}

/// Like [`app_log`], but only when the message differs from the last one under the same `key`, so
/// a periodic check that keeps failing (or keeps finding the same version) is recorded once.
pub(crate) fn app_log_on_change(key: &str, level: &str, action: &str, message: &str) {
    static LAST: std::sync::Mutex<Vec<(String, String)>> = std::sync::Mutex::new(Vec::new());
    let Ok(mut last) = LAST.lock() else {
        return;
    };
    let current = format!("{level}|{message}");
    match last.iter_mut().find(|(seen, _)| seen == key) {
        Some((_, previous)) if *previous == current => return,
        Some((_, previous)) => *previous = current,
        None => last.push((key.to_string(), current)),
    }
    drop(last);
    app_log(level, action, message);
}

/// Crashes end up in the app log before the default handler reports them.
pub(crate) fn install_panic_logger() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Never wait here: the panic may have happened while this thread was writing a log.
        let guard = match LOG_WRITE.try_lock() {
            Ok(guard) => Some(guard),
            Err(std::sync::TryLockError::Poisoned(poisoned)) => Some(poisoned.into_inner()),
            Err(std::sync::TryLockError::WouldBlock) => None,
        };
        if guard.is_some() {
            let _ = write_log_entry_unlocked(APP_LOG_ID, APP_LOG_NAME, "error", "app.crash", &info.to_string());
        }
        drop(guard);
        default_hook(info);
    }));
}
