//! Handing an update over to a helper process. The app cannot replace itself while it runs, so it
//! starts this same program as the helper (see `helper`) with a job, and quits.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Command-line flag that starts this program as the update helper.
pub const HELPER_FLAG: &str = "--update-helper";

/// Where the new version comes from.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UpdateSource {
    /// Still to be downloaded (updates started when the app opens).
    Download { url: String, name: String, size: u64 },
    /// Already downloaded by the app (updates the user started).
    Installer { path: PathBuf },
}

/// Everything the helper needs, passed to it as one JSON argument.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelperJob {
    pub version: String,
    pub source: UpdateSource,
    /// The app to replace and open again (see `install::app_location`).
    pub app: PathBuf,
    /// The app's process, which must be gone before its files are replaced.
    pub parent_pid: u32,
    /// Started in the tray: no window, and the updated app opens in the tray again.
    pub silent: bool,
}

/// The job when this process was started as the update helper.
pub fn job_from_args() -> Option<HelperJob> {
    let mut args = std::env::args().skip_while(|arg| arg != HELPER_FLAG).skip(1);
    serde_json::from_str(&args.next()?).ok()
}

/// Starts the helper for `job`. The caller quits right after.
pub fn spawn_helper(job: &HelperJob) -> Result<(), String> {
    let program = helper_program(job)?;
    let json = serde_json::to_string(job).map_err(|e| e.to_string())?;
    let mut command = std::process::Command::new(program);
    command
        .arg(HELPER_FLAG)
        .arg(json)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command.spawn().map(|_| ()).map_err(|e| format!("UPDATE_INSTALL_FAILED:{e}"))
}

/// What runs the helper. Windows locks a running program's file, so the helper runs from a copy
/// under another name (the installer also closes every process named like the app). On Linux the
/// AppImage itself is started again: the app's own files vanish when the AppImage it came from
/// exits. On macOS the bundle may be moved away under the running helper, which Unix allows.
fn helper_program(job: &HelperJob) -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        let _ = job;
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let dir = std::env::temp_dir().join("project-hub-update");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let copy = dir.join("project-hub-updater.exe");
        std::fs::copy(&exe, &copy).map_err(|e| format!("UPDATE_INSTALL_FAILED:{e}"))?;
        Ok(copy)
    }
    #[cfg(target_os = "macos")]
    {
        let _ = job;
        std::env::current_exe().map_err(|e| e.to_string())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    Ok(job.app.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_job_survives_the_command_line() {
        let job = HelperJob {
            version: "1.2.3".into(),
            source: UpdateSource::Download { url: "https://github.com/x".into(), name: "a b.exe".into(), size: 7 },
            app: PathBuf::from(r"C:\Program Files\Project Hub\project-hub.exe"),
            parent_pid: 42,
            silent: true,
        };
        let parsed: HelperJob = serde_json::from_str(&serde_json::to_string(&job).unwrap()).unwrap();
        assert_eq!(parsed.version, "1.2.3");
        assert!(parsed.silent);
        assert!(matches!(parsed.source, UpdateSource::Download { size: 7, .. }));
    }
}
