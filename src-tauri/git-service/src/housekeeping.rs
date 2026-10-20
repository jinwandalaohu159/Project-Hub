//! Git's routine clean-up (packing loose objects), kept off the user's path.
//!
//! Every foreground git call disables the automatic clean-up (see `command.rs`): on a large history
//! it can repack for many seconds right after a save. It runs here instead, in the background.

use crate::command::hidden_command;
#[cfg(windows)]
use crate::command::CREATE_NO_WINDOW;
use crate::GitService;
use std::collections::HashSet;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::process::Stdio;
use std::sync::{Mutex, OnceLock};

#[cfg(windows)]
const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x00004000;

impl GitService {
    /// Runs `maintenance run --auto` (or `gc --auto` on older git) on a background thread at low
    /// priority, at most once at a time per repository. Git decides whether anything needs doing.
    pub(crate) fn schedule_housekeeping(&self, repo_path: &str) {
        // Test repositories are deleted right away; a clean-up still running there would only get in the way.
        if cfg!(test) {
            return;
        }
        static RUNNING: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
        let key = repo_path.replace('\\', "/").trim_end_matches('/').to_lowercase();
        let running = RUNNING.get_or_init(|| Mutex::new(HashSet::new()));
        if !running
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(key.clone())
        {
            return;
        }
        let repo = repo_path.to_string();
        std::thread::spawn(move || {
            if !run_background_git(&repo, &["maintenance", "run", "--auto", "--quiet"]) {
                run_background_git(&repo, &["gc", "--auto", "--quiet"]);
            }
            running
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(&key);
        });
    }
}

fn run_background_git(repo: &str, args: &[&str]) -> bool {
    let mut command = hidden_command("git");
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS);
    command
        .args(["-c", "safe.directory=*", "-c", "gc.autoDetach=false", "-c", "maintenance.autoDetach=false"])
        .args(args)
        .current_dir(repo)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
