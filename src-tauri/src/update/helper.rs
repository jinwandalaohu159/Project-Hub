//! The update helper's work: wait for the app to quit, get the installer, install it, open the
//! updated app. On failure the old version is opened again. `helper_window` shows the progress.

use super::{download, handoff::HelperJob, handoff::UpdateSource, install, outcome};
use serde::Serialize;
use std::time::Duration;

/// Where the helper is in its work.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum HelperProgress {
    Downloading { downloaded: u64, total: u64 },
    /// Installing and opening the new version; there is no byte count, the window estimates.
    Restarting,
    Done,
    /// The old version is opened again.
    Failed { error: String },
}

impl HelperProgress {
    /// What the job starts with.
    pub fn first(job: &HelperJob) -> HelperProgress {
        match &job.source {
            UpdateSource::Download { size, .. } => HelperProgress::Downloading { downloaded: 0, total: *size },
            UpdateSource::Installer { .. } => HelperProgress::Restarting,
        }
    }
}

/// How long the app may take to quit after starting the helper.
const PARENT_EXIT_LIMIT: Duration = Duration::from_secs(20);
/// Lets the window say what went wrong before the old version takes its place.
const FAILURE_LINGER: Duration = Duration::from_millis(2500);

/// Does the job, reporting each step. Returns once an app is open again; the caller then quits.
pub fn run_job(job: &HelperJob, mut report: impl FnMut(HelperProgress)) {
    wait_for_exit(job.parent_pid, PARENT_EXIT_LIMIT);
    let result = (|| {
        let installer = match &job.source {
            UpdateSource::Download { url, name, size } => download(url, name, *size, |downloaded, total| {
                report(HelperProgress::Downloading { downloaded, total })
            })?,
            UpdateSource::Installer { path } => path.clone(),
        };
        report(HelperProgress::Restarting);
        install::install(&installer, &job.app)
    })();

    match result {
        Ok(()) => {
            outcome::record(&job.version, None);
            match install::relaunch(&job.app, job.silent) {
                Ok(()) => report(HelperProgress::Done),
                Err(error) => report(HelperProgress::Failed { error }),
            }
        }
        Err(error) => {
            outcome::record(&job.version, Some(&error));
            report(HelperProgress::Failed { error });
            if !job.silent {
                std::thread::sleep(FAILURE_LINGER);
            }
            let _ = install::relaunch(&job.app, job.silent);
        }
    }
}

/// Waits until process `pid` has exited, or `limit` has passed.
fn wait_for_exit(pid: u32, limit: Duration) {
    let started = std::time::Instant::now();
    while is_running(pid) && started.elapsed() < limit {
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(target_os = "windows")]
fn is_running(pid: u32) -> bool {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, WAIT_TIMEOUT},
        System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE},
    };
    // SAFETY: plain Win32 calls on a handle this function opens and closes itself.
    unsafe {
        let handle = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if handle.is_null() {
            return false;
        }
        let running = WaitForSingleObject(handle, 0) == WAIT_TIMEOUT;
        CloseHandle(handle);
        running
    }
}

#[cfg(unix)]
fn is_running(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_finished_process_is_not_running() {
        let mut child = std::process::Command::new(if cfg!(windows) { "cmd" } else { "true" })
            .args(if cfg!(windows) { vec!["/C", "exit"] } else { vec![] })
            .spawn()
            .unwrap();
        let pid = child.id();
        child.wait().unwrap();
        assert!(!is_running(pid));
        assert!(is_running(std::process::id()));
    }
}
