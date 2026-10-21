//! Runs the downloaded installer the way each platform needs, so the updated app opens again by
//! itself. The caller quits right after `launch_installer` returns.

use std::path::Path;

/// Runs the NSIS installer in its own update mode once this process has exited.
#[cfg(target_os = "windows")]
pub fn launch_installer(installer: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    // Give this process a moment to exit, then run the Tauri installer in its own update mode:
    // /P progress bar only, /UPDATE keep shortcuts and settings, /R reopen the app when done.
    let script = format!(
        "ping 127.0.0.1 -n 3 >nul & start \"\" \"{}\" /P /UPDATE /R",
        installer.display()
    );
    std::process::Command::new("cmd")
        .arg("/C")
        .raw_arg(script)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("UPDATE_INSTALL_FAILED:{e}"))?;
    Ok(())
}

/// Leaves `replace_app.sh` to swap the app bundle for the one in the disk image once this process
/// has exited, and to open it again.
#[cfg(target_os = "macos")]
pub fn launch_installer(installer: &Path) -> Result<(), String> {
    use std::{os::unix::process::CommandExt, process::Stdio};
    // Run from the build folder (or anywhere but an app bundle): leave the disk image to the user.
    let Some(bundle) = running_app_bundle() else {
        std::process::Command::new("open")
            .arg(installer)
            .spawn()
            .map_err(|e| format!("UPDATE_INSTALL_FAILED:{e}"))?;
        return Ok(());
    };
    std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(include_str!("replace_app.sh"))
        .arg("project-hub-update")
        .arg(std::process::id().to_string())
        .arg(installer)
        .arg(&bundle)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        // Its own process group, so it outlives the app it is about to replace.
        .process_group(0)
        .spawn()
        .map_err(|e| format!("UPDATE_INSTALL_FAILED:{e}"))?;
    Ok(())
}

/// The app bundle this copy runs from, such as /Applications/Project Hub.app.
#[cfg(target_os = "macos")]
fn running_app_bundle() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    // <name>.app/Contents/MacOS/<binary>
    let bundle = exe.parent()?.parent()?.parent()?;
    (bundle.extension()? == "app").then(|| bundle.to_path_buf())
}

/// Replaces the running AppImage with the downloaded one and starts it.
#[cfg(all(unix, not(target_os = "macos")))]
pub fn launch_installer(installer: &Path) -> Result<(), String> {
    use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
    let target = PathBuf::from(
        std::env::var("APPIMAGE")
            .map_err(|_| "UPDATE_NEEDS_APPIMAGE".to_string())?,
    );
    // The running AppImage cannot be written to ("Text file busy"), but it can be replaced:
    // copy next to it, then rename over it.
    let staged = target.with_extension("update");
    fs::copy(installer, &staged).map_err(|e| e.to_string())?;
    fs::set_permissions(&staged, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    fs::rename(&staged, &target).map_err(|e| {
        let _ = fs::remove_file(&staged);
        e.to_string()
    })?;
    std::process::Command::new(&target)
        .spawn()
        .map_err(|e| format!("UPDATE_INSTALL_FAILED:{e}"))?;
    Ok(())
}
