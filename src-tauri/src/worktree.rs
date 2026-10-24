//! Tells the window when files of the open project change on disk, so the file tree, the unsaved
//! changes and the open file follow edits made in other programs.
//!
//! The operating system reports every write; most come from places the window never shows as
//! changes (git's own data, and what the default rules hide: dependencies, caches, logs). Those are
//! dropped here, and a burst of writes (a save, a checkout, an install) reaches the window once.

use notify::event::{AccessKind, EventKind, MetadataKind, ModifyKind};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};
use tauri::Emitter;

/// Sent with the watched folder's path.
pub(crate) const WORKTREE_CHANGED_EVENT: &str = "worktree-changed";

/// A burst counts as over after this long without writes…
const QUIET: Duration = Duration::from_millis(300);
/// …or after this long in any case, so a long install still shows progress.
const MAX_WAIT: Duration = Duration::from_secs(2);

/// Watches one project folder until dropped.
pub struct WorktreeWatch {
    root: PathBuf,
    _watcher: RecommendedWatcher,
}

impl WorktreeWatch {
    pub(crate) fn start(app: tauri::AppHandle, root: &str) -> Result<Self, String> {
        let root_path = PathBuf::from(root);
        // Events may name the folder by its real path (macOS resolves links); either way it is ours.
        let roots = [root_path.clone(), std::fs::canonicalize(&root_path).unwrap_or_else(|_| root_path.clone())];
        // Holds at most one pending signal: further writes in a burst only need to keep it there.
        let (tx, rx) = mpsc::sync_channel::<()>(1);
        let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            if event.is_ok_and(|event| shows_in_window(&event, &roots)) {
                // Full means a signal is already waiting, which covers this write too.
                let _ = tx.try_send(());
            }
        })
        .map_err(|error| error.to_string())?;
        watcher.watch(&root_path, RecursiveMode::Recursive).map_err(|error| error.to_string())?;

        let payload = root.to_string();
        // Ends when the watcher is dropped, which drops the sending side.
        std::thread::spawn(move || {
            forward_bursts(&rx, QUIET, MAX_WAIT, || {
                let _ = app.emit(WORKTREE_CHANGED_EVENT, &payload);
            })
        });

        Ok(Self { root: root_path, _watcher: watcher })
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }
}

/// Whether `event` changed something the window shows. Reads (and the access times they touch)
/// do not; on some systems the window's own reads would otherwise report back to it.
fn shows_in_window(event: &notify::Event, roots: &[PathBuf]) -> bool {
    let is_read = matches!(event.kind, EventKind::Access(AccessKind::Read | AccessKind::Open(_) | AccessKind::Any))
        || matches!(event.kind, EventKind::Modify(ModifyKind::Metadata(MetadataKind::AccessTime)));
    !is_read && event.paths.iter().any(|path| !skipped(roots, path))
}

/// Paths in git's folder or hidden by the default rules; anything outside the project too.
fn skipped(roots: &[PathBuf], path: &Path) -> bool {
    let Some(relative) = roots.iter().find_map(|root| path.strip_prefix(root).ok()) else {
        return true;
    };
    relative.components().next() == Some(Component::Normal(".git".as_ref())) || git_service::hidden_by_default(relative)
}

/// Calls `notify` once per burst of signals on `rx`: when `quiet` passes without one, or `max_wait`
/// after the burst began. Returns when the sending side is gone (the watch ended), without
/// reporting a burst it cut short.
fn forward_bursts(rx: &Receiver<()>, quiet: Duration, max_wait: Duration, notify: impl Fn()) {
    while rx.recv().is_ok() {
        let started = Instant::now();
        loop {
            let left = max_wait.saturating_sub(started.elapsed());
            if left.is_zero() {
                break;
            }
            match rx.recv_timeout(quiet.min(left)) {
                Ok(()) => continue,
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
        notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn git_data_hidden_paths_and_other_folders_are_skipped() {
        let roots = [PathBuf::from("/p")];
        let skipped = |path: &str| skipped(&roots, Path::new(path));
        assert!(skipped("/p/.git/index"));
        assert!(skipped("/p/node_modules/vue/index.js"));
        assert!(skipped("/elsewhere/a.txt"));
        assert!(!skipped("/p/src/main.rs"));
        assert!(!skipped("/p/.gitignore"));
    }

    #[test]
    fn a_burst_is_reported_once() {
        let (tx, rx) = mpsc::sync_channel::<()>(1);
        let sender = std::thread::spawn(move || {
            for _ in 0..5 {
                let _ = tx.try_send(());
                std::thread::sleep(Duration::from_millis(5));
            }
            // Still watching while the burst settles.
            std::thread::sleep(Duration::from_millis(300));
        });
        let calls = Cell::new(0);
        forward_bursts(&rx, Duration::from_millis(100), Duration::from_secs(5), || calls.set(calls.get() + 1));
        sender.join().unwrap();
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn a_long_burst_is_still_reported_along_the_way() {
        let (tx, rx) = mpsc::sync_channel::<()>(1);
        let sender = std::thread::spawn(move || {
            let started = Instant::now();
            while started.elapsed() < Duration::from_millis(250) {
                let _ = tx.try_send(());
                std::thread::sleep(Duration::from_millis(5));
            }
            std::thread::sleep(Duration::from_millis(300));
        });
        let calls = Cell::new(0);
        forward_bursts(&rx, Duration::from_millis(100), Duration::from_millis(60), || calls.set(calls.get() + 1));
        sender.join().unwrap();
        assert!(calls.get() >= 2, "{}", calls.get());
    }
}
