use crate::settings::{app_data_dir, GithubAccount};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const API: &str = "https://api.github.com";
const USER_AGENT: &str = "Project-Hub";
const TOKEN_FILE_NAME: &str = "github-token.bin";
const BROWSER_LOGIN_TIMEOUT: Duration = Duration::from_secs(300);
pub const GCM_NOT_AVAILABLE: &str = "GCM_NOT_AVAILABLE";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoInfo {
    pub full_name: String,
    pub clone_url: String,
    pub html_url: String,
    pub private: bool,
}

fn token_path() -> PathBuf {
    app_data_dir().join(TOKEN_FILE_NAME)
}

pub fn store_token(token: &str) -> Result<(), String> {
    let path = token_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, protect(token.as_bytes())?).map_err(|e| e.to_string())
}

pub fn load_token() -> Option<String> {
    let bytes = fs::read(token_path()).ok()?;
    let token = String::from_utf8(unprotect(&bytes).ok()?).ok()?;
    let token = token.trim().to_string();
    (!token.is_empty()).then_some(token)
}

pub fn delete_token() {
    let _ = fs::remove_file(token_path());
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout(Duration::from_secs(60))
        .build()
}

fn request(method: &str, path: &str, token: &str) -> ureq::Request {
    agent()
        .request(method, &format!("{API}{path}"))
        .set("Authorization", &format!("Bearer {token}"))
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .set("User-Agent", USER_AGENT)
}

fn api_error(error: ureq::Error) -> String {
    match error {
        ureq::Error::Status(401, _) => "GitHub rejected the sign-in (token invalid or expired).".to_string(),
        ureq::Error::Status(code, response) => {
            let body = response.into_string().unwrap_or_default();
            let message = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|value| {
                    let base = value.get("message")?.as_str()?.to_string();
                    let detail = value
                        .get("errors")
                        .and_then(|errors| errors.get(0))
                        .and_then(|error| error.get("message"))
                        .and_then(|m| m.as_str())
                        .map(|m| format!(" ({m})"))
                        .unwrap_or_default();
                    Some(format!("{base}{detail}"))
                })
                .unwrap_or(body);
            format!("GitHub error {code}: {message}")
        }
        ureq::Error::Transport(transport) => format!("Cannot reach GitHub: {transport}"),
    }
}

pub fn fetch_user(token: &str) -> Result<GithubAccount, String> {
    request("GET", "/user", token)
        .call()
        .map_err(api_error)?
        .into_json::<GithubAccount>()
        .map_err(|e| e.to_string())
}

pub fn create_repo(token: &str, name: &str, description: &str, private: bool) -> Result<RepoInfo, String> {
    request("POST", "/user/repos", token)
        .send_json(serde_json::json!({
            "name": name,
            "description": description,
            "private": private,
            "auto_init": false,
        }))
        .map_err(api_error)?
        .into_json::<RepoInfo>()
        .map_err(|e| e.to_string())
}

pub fn list_repos(token: &str) -> Result<Vec<RepoInfo>, String> {
    request("GET", "/user/repos?per_page=100&sort=updated&affiliation=owner,collaborator", token)
        .call()
        .map_err(api_error)?
        .into_json::<Vec<RepoInfo>>()
        .map_err(|e| e.to_string())
}

fn git_command() -> Command {
    let mut command = Command::new("git");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    // Always talk to Git Credential Manager, whatever helper the user configured.
    command
        .args(["-c", "credential.helper=", "-c", "credential.helper=manager"])
        .env("GIT_TERMINAL_PROMPT", "0");
    command
}

pub fn credential_manager_available() -> bool {
    let mut command = Command::new("git");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
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

pub fn is_github_https(url: &str) -> bool {
    url.trim().to_lowercase().starts_with("https://github.com/")
}

/// Turns "owner/repo", an https URL or an SSH URL into an https clone URL GitHub accepts.
pub fn normalize_repo_url(input: &str) -> Result<String, String> {
    let value = input.trim().trim_end_matches('/');
    if value.is_empty() {
        return Err("Repository address is empty.".to_string());
    }
    if let Some(rest) = value.strip_prefix("git@github.com:") {
        return Ok(format!("https://github.com/{}", with_git_suffix(rest)));
    }
    if value.starts_with("https://") || value.starts_with("http://") || value.starts_with("ssh://") {
        return Ok(if is_github_https(value) { with_git_suffix(value) } else { value.to_string() });
    }
    let parts: Vec<&str> = value.split('/').collect();
    if parts.len() == 2 && parts.iter().all(|part| !part.is_empty()) {
        return Ok(format!("https://github.com/{}", with_git_suffix(value)));
    }
    Err("Use owner/repo or a full repository URL.".to_string())
}

fn with_git_suffix(value: &str) -> String {
    if value.ends_with(".git") {
        value.to_string()
    } else {
        format!("{value}.git")
    }
}

/// GitHub repository names allow letters, digits, '-', '_' and '.'.
pub fn sanitize_repo_name(name: &str) -> String {
    let mut out = String::new();
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            out.push(ch);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_matches(['-', '.']).to_string();
    if out.is_empty() {
        "project".to_string()
    } else {
        out
    }
}

#[cfg(windows)]
fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::Foundation::{LocalFree, HLOCAL};
    use windows_sys::Win32::Security::Cryptography::{CryptProtectData, CRYPT_INTEGER_BLOB};
    let input = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    let ok = unsafe {
        CryptProtectData(
            &input,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            &mut output,
        )
    };
    if ok == 0 {
        return Err("Failed to encrypt the GitHub token.".to_string());
    }
    let bytes = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe {
        LocalFree(output.pbData as HLOCAL);
    }
    Ok(bytes)
}

#[cfg(windows)]
fn unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::Foundation::{LocalFree, HLOCAL};
    use windows_sys::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};
    let input = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    let ok = unsafe {
        CryptUnprotectData(
            &input,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            &mut output,
        )
    };
    if ok == 0 {
        return Err("Failed to decrypt the GitHub token.".to_string());
    }
    let bytes = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe {
        LocalFree(output.pbData as HLOCAL);
    }
    Ok(bytes)
}

#[cfg(not(windows))]
fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
    Ok(data.to_vec())
}

#[cfg(not(windows))]
fn unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
    Ok(data.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_repo_addresses() {
        assert_eq!(normalize_repo_url("octo/demo").unwrap(), "https://github.com/octo/demo.git");
        assert_eq!(
            normalize_repo_url("https://github.com/octo/demo/").unwrap(),
            "https://github.com/octo/demo.git"
        );
        assert_eq!(
            normalize_repo_url("git@github.com:octo/demo.git").unwrap(),
            "https://github.com/octo/demo.git"
        );
        assert!(normalize_repo_url("not a repo").is_err());
    }

    #[test]
    fn sanitizes_repo_names() {
        assert_eq!(sanitize_repo_name("Project Hub"), "Project-Hub");
        assert_eq!(sanitize_repo_name("我的项目"), "project");
        assert_eq!(sanitize_repo_name("demo_v2.0"), "demo_v2.0");
    }

    #[test]
    fn protects_round_trip() {
        let secret = b"gho_example";
        let sealed = protect(secret).unwrap();
        assert_eq!(unprotect(&sealed).unwrap(), secret);
    }
}
