//! Browser sign-in through Git Credential Manager (GCM), which runs GitHub's OAuth flow and hands
//! back a token. GCM ships with Git for Windows and is installed separately on macOS and Linux.

use std::{
    ffi::OsString,
    io::{Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const BROWSER_LOGIN_TIMEOUT: Duration = Duration::from_secs(300);
/// The frontend shows the install hint for the platform the code names.
#[cfg(windows)]
const GCM_NOT_AVAILABLE: &str = "GCM_NOT_AVAILABLE_WINDOWS";
#[cfg(target_os = "macos")]
const GCM_NOT_AVAILABLE: &str = "GCM_NOT_AVAILABLE_MACOS";
#[cfg(all(unix, not(target_os = "macos")))]
const GCM_NOT_AVAILABLE: &str = "GCM_NOT_AVAILABLE_LINUX";

/// Where git and Git Credential Manager may be installed beyond the app's own PATH:
/// - macOS: apps opened from the Dock or Finder only get /usr/bin:/bin:/usr/sbin:/sbin, while the
///   GCM installer links into /usr/local/bin and Homebrew lives in /opt/homebrew/bin.
/// - Linux: desktop launchers may leave out ~/.local/bin, Homebrew and `dotnet tool` installs.
/// - Windows: the app reads PATH once and then stays in the tray, so Git for Windows installed
///   after it started is only found through the installer's registry entry.
#[cfg(target_os = "macos")]
fn extra_tool_dirs() -> Vec<PathBuf> {
    vec![PathBuf::from("/opt/homebrew/bin"), PathBuf::from("/usr/local/bin")]
}

#[cfg(all(unix, not(target_os = "macos")))]
fn extra_tool_dirs() -> Vec<PathBuf> {
    let mut found = vec![
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/home/linuxbrew/.linuxbrew/bin"),
    ];
    if let Some(home) = dirs::home_dir() {
        found.push(home.join(".local").join("bin"));
        found.push(home.join(".dotnet").join("tools"));
    }
    found
}

#[cfg(windows)]
fn extra_tool_dirs() -> Vec<PathBuf> {
    use winreg::{
        enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE},
        RegKey,
    };
    let mut found = Vec::new();
    // Machine-wide and per-user installs of Git for Windows record where they went.
    for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        let install = RegKey::predef(hive)
            .open_subkey(r"SOFTWARE\GitForWindows")
            .and_then(|key| key.get_value::<String, _>("InstallPath"));
        if let Ok(install) = install {
            found.push(PathBuf::from(install).join("cmd"));
        }
    }
    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        found.push(PathBuf::from(program_files).join("Git").join("cmd"));
    }
    found
}

/// The app's PATH with the usual install locations appended; looked up on every call so a tool
/// installed while the app runs is found without a restart.
fn tool_search_path() -> OsString {
    let current = std::env::var_os("PATH").unwrap_or_default();
    let mut dirs: Vec<PathBuf> = std::env::split_paths(&current).collect();
    for dir in extra_tool_dirs() {
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    std::env::join_paths(dirs).unwrap_or(current)
}

fn plain_git() -> Command {
    let mut command = Command::new("git");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    // Rust looks the program up in the PATH given here, so git itself is found the same way.
    command.env("PATH", tool_search_path());
    command
}

/// GCM on Linux refuses to run until a credential store is chosen. When the user has not picked
/// one, fall back to git's in-memory cache: the app keeps the token it receives in its own
/// settings, so nothing depends on the store keeping it.
#[cfg(all(unix, not(target_os = "macos")))]
fn ensure_credential_store(command: &mut Command) {
    let chosen = std::env::var_os("GCM_CREDENTIAL_STORE").is_some_and(|value| !value.is_empty())
        || plain_git()
            .args(["config", "--get", "credential.credentialStore"])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .is_ok_and(|output| output.status.success());
    if !chosen {
        command.env("GCM_CREDENTIAL_STORE", "cache");
    }
}

fn git_command() -> Command {
    let mut command = plain_git();
    // Always talk to Git Credential Manager, whatever helper the user configured.
    command
        .args(["-c", "credential.helper=", "-c", "credential.helper=manager"])
        .env("GIT_TERMINAL_PROMPT", "0");
    #[cfg(all(unix, not(target_os = "macos")))]
    ensure_credential_store(&mut command);
    command
}

fn credential_manager_available() -> bool {
    plain_git()
        .args(["credential-manager", "--version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn run_credential(action: &str, input: &str, timeout: Duration) -> Result<String, String> {
    let mut child = git_command()
        .args(["credential", action])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start git: {e}"))?;
    child
        .stdin
        .take()
        .ok_or("Failed to talk to git")?
        .write_all(input.as_bytes())
        .map_err(|e| e.to_string())?;

    let started = Instant::now();
    loop {
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            break;
        }
        if started.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err("GitHub sign-in timed out.".to_string());
        }
        std::thread::sleep(Duration::from_millis(150));
    }

    let mut stdout = String::new();
    let mut stderr = String::new();
    if let Some(mut out) = child.stdout.take() {
        let _ = out.read_to_string(&mut stdout);
    }
    if let Some(mut err) = child.stderr.take() {
        let _ = err.read_to_string(&mut stderr);
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        let message = stderr.trim();
        return Err(if message.is_empty() {
            "GitHub sign-in was cancelled.".to_string()
        } else {
            message.to_string()
        });
    }
    Ok(stdout)
}

/// Opens the Git Credential Manager sign-in (browser OAuth) and returns the GitHub token it issues.
/// An already stored credential is returned without any prompt.
pub fn browser_login() -> Result<String, String> {
    if !credential_manager_available() {
        return Err(GCM_NOT_AVAILABLE.to_string());
    }
    let output = run_credential(
        "fill",
        "protocol=https\nhost=github.com\n\n",
        BROWSER_LOGIN_TIMEOUT,
    )?;
    let mut username = String::new();
    let mut password = String::new();
    for line in output.lines() {
        if let Some(value) = line.strip_prefix("username=") {
            username = value.to_string();
        } else if let Some(value) = line.strip_prefix("password=") {
            password = value.to_string();
        }
    }
    if password.is_empty() {
        return Err("GitHub sign-in was cancelled.".to_string());
    }
    let _ = run_credential(
        "approve",
        &format!("protocol=https\nhost=github.com\nusername={username}\npassword={password}\n\n"),
        Duration::from_secs(20),
    );
    Ok(password)
}

/// Removes the credential Git Credential Manager keeps, so the next browser sign-in can pick another account.
pub fn forget_browser_login(login: &str) {
    if !credential_manager_available() {
        return;
    }
    let _ = run_credential(
        "reject",
        &format!("protocol=https\nhost=github.com\nusername={login}\n\n"),
        Duration::from_secs(20),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_search_path_keeps_the_app_path_first() {
        let current: Vec<PathBuf> = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect();
        let searched: Vec<PathBuf> = std::env::split_paths(&tool_search_path()).collect();
        assert_eq!(searched[..current.len()], current[..]);
        for dir in extra_tool_dirs() {
            assert!(searched.contains(&dir), "{} is not searched", dir.display());
        }
    }
}
