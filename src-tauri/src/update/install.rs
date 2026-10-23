//! Installing a downloaded update and opening the app again, the way each platform needs. Both run
//! in the update helper (see `handoff`) after the app itself has quit.

use std::path::{Path, PathBuf};

/// What gets replaced by an update and opened again afterwards: the installed program on Windows,
/// the app bundle on macOS, the AppImage on Linux. `None` when this copy cannot update itself
/// (a development build, or Linux outside an AppImage).
pub fn app_location() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    return std::env::current_exe().ok();
    #[cfg(target_os = "macos")]
    return running_app_bundle();
    #[cfg(all(unix, not(target_os = "macos")))]
    return std::env::var_os("APPIMAGE").map(PathBuf::from);
}

/// Runs the NSIS installer silently in its update mode (keeps shortcuts and settings) and waits
/// for it. The installer's own progress window is not shown; the helper window shows progress.
#[cfg(target_os = "windows")]
pub fn install(installer: &Path, _app: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let status = std::process::Command::new(installer)
        .args(["/S", "/UPDATE"])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|e| format!("UPDATE_INSTALL_FAILED:{e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("UPDATE_INSTALL_FAILED:installer exited with {status}"))
    }
}

/// Swaps the app bundle for the one in the disk image (see `replace_app.sh`); on failure the old
/// bundle stays in place.
#[cfg(target_os = "macos")]
pub fn install(installer: &Path, app: &Path) -> Result<(), String> {
    let status = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(include_str!("replace_app.sh"))
        .arg("project-hub-update")
        .arg(installer)
        .arg(app)
        .stdin(std::process::Stdio::null())
        .status()
        .map_err(|e| format!("UPDATE_INSTALL_FAILED:{e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("UPDATE_INSTALL_FAILED:could not replace the app bundle".to_string())
    }
}

/// Replaces the AppImage with the downloaded one. The running file cannot be written to ("Text file
/// busy"), but it can be replaced: copy next to it, then rename over it.
#[cfg(all(unix, not(target_os = "macos")))]
pub fn install(installer: &Path, app: &Path) -> Result<(), String> {
    use std::{fs, os::unix::fs::PermissionsExt};
    let staged = app.with_extension("update");
    fs::copy(installer, &staged).map_err(|e| format!("UPDATE_INSTALL_FAILED:{e}"))?;
    fs::set_permissions(&staged, fs::Permissions::from_mode(0o755))
        .map_err(|e| format!("UPDATE_INSTALL_FAILED:{e}"))?;
    fs::rename(&staged, app).map_err(|e| {
        let _ = fs::remove_file(&staged);
        format!("UPDATE_INSTALL_FAILED:{e}")
    })
}

/// Opens the app at `app` as its own process, in the tray when `hidden`.
pub fn relaunch(app: &Path, hidden: bool) -> Result<(), String> {
    let mut command = relaunch_command(app);
    if hidden {
        #[cfg(target_os = "macos")]
        command.arg("--args");
        command.arg("--hidden");
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Its own process group, so it is not tied to the helper that is about to exit.
        command.process_group(0);
    }
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("UPDATE_RELAUNCH_FAILED:{e}"))
}

#[cfg(target_os = "macos")]
fn relaunch_command(app: &Path) -> std::process::Command {
    let mut command = std::process::Command::new("open");
    // A new instance even if macOS still remembers the old one as running.
    command.arg("-n").arg(app);
    command
}

#[cfg(not(target_os = "macos"))]
fn relaunch_command(app: &Path) -> std::process::Command {
    std::process::Command::new(app)
}

/// The app bundle this copy runs from, such as /Applications/Project Hub.app.
#[cfg(target_os = "macos")]
fn running_app_bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    // <name>.app/Contents/MacOS/<binary>
    let bundle = exe.parent()?.parent()?.parent()?;
    (bundle.extension()? == "app").then(|| bundle.to_path_buf())
}
