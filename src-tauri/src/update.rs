//! Self-update from the publisher's GitHub releases: check, download the installer, run it, restart.
//! The release source is fixed in the program and has nothing to do with the user's own GitHub login.

use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

/// Where new versions are published. Change only together with `scripts/release.mjs`.
pub const UPDATE_REPO: &str = "jinwandalaohu159/Project-Hub";
const USER_AGENT: &str = "Project-Hub-Updater";

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub available: bool,
    pub notes: String,
    pub published_at: Option<String>,
    pub release_url: String,
    pub asset_name: Option<String>,
    pub asset_url: Option<String>,
    pub asset_size: u64,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    body: Option<String>,
    html_url: String,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Deserialize, Clone)]
struct Asset {
    name: String,
    browser_download_url: String,
    size: u64,
}

pub fn releases_page() -> String {
    format!("https://github.com/{UPDATE_REPO}/releases")
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout(timeout)
        .build()
}

/// "v1.2.3" / "1.2.3-beta" -> [1, 2, 3]; missing parts count as 0.
fn version_parts(version: &str) -> Vec<u64> {
    version
        .trim()
        .trim_start_matches(['v', 'V'])
        .split(['-', '+'])
        .next()
        .unwrap_or_default()
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect()
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    let (a, b) = (version_parts(latest), version_parts(current));
    let len = a.len().max(b.len());
    for i in 0..len {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    false
}

/// The installer for this computer among the release files.
fn pick_asset(assets: &[Asset]) -> Option<Asset> {
    let arch = if cfg!(target_arch = "aarch64") { "aarch64" } else { "x64" };
    let matches = |asset: &&Asset| {
        let name = asset.name.to_lowercase();
        if cfg!(target_os = "windows") {
            name.ends_with("-setup.exe")
        } else if cfg!(target_os = "macos") {
            name.ends_with(".dmg")
        } else {
            name.ends_with(".appimage")
        }
    };
    let candidates: Vec<&Asset> = assets.iter().filter(matches).collect();
    candidates
        .iter()
        .find(|asset| asset.name.to_lowercase().contains(arch))
        .or_else(|| candidates.first())
        .map(|asset| (*asset).clone())
}

pub fn check(current: &str) -> Result<UpdateInfo, String> {
    let url = format!("https://api.github.com/repos/{UPDATE_REPO}/releases/latest");
    let release: Release = match agent(Duration::from_secs(30))
        .get(&url)
        .set("Accept", "application/vnd.github+json")
        .set("User-Agent", USER_AGENT)
        .call()
    {
        Ok(response) => response.into_json().map_err(|e| e.to_string())?,
        // No release published yet.
        Err(ureq::Error::Status(404, _)) => {
            return Ok(UpdateInfo {
                current: current.to_string(),
                latest: current.to_string(),
                available: false,
                notes: String::new(),
                published_at: None,
                release_url: releases_page(),
                asset_name: None,
                asset_url: None,
                asset_size: 0,
            })
        }
        Err(ureq::Error::Status(403, _)) => return Err("UPDATE_RATE_LIMITED".to_string()),
        Err(error) => return Err(format!("Cannot reach GitHub: {error}")),
    };

    let latest = release.tag_name.trim_start_matches(['v', 'V']).to_string();
    let asset = pick_asset(&release.assets);
    Ok(UpdateInfo {
        current: current.to_string(),
        available: !release.draft && !release.prerelease && is_newer(&latest, current) && asset.is_some(),
        latest,
        notes: release.body.unwrap_or_default(),
        published_at: release.published_at,
        release_url: release.html_url,
        asset_name: asset.as_ref().map(|a| a.name.clone()),
        asset_url: asset.as_ref().map(|a| a.browser_download_url.clone()),
        asset_size: asset.map(|a| a.size).unwrap_or(0),
    })
}

/// Only files published in our own repository may be downloaded and run.
fn is_trusted_url(url: &str) -> bool {
    url.starts_with(&format!("https://github.com/{UPDATE_REPO}/releases/download/"))
}

fn safe_file_name(name: &str) -> String {
    name.chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || "._-".contains(ch) { ch } else { '_' })
        .collect()
}

/// Downloads the installer into a temporary folder, reporting `(downloaded, total)` bytes.
pub fn download(
    url: &str,
    name: &str,
    expected_size: u64,
    mut progress: impl FnMut(u64, u64),
) -> Result<PathBuf, String> {
    if !is_trusted_url(url) {
        return Err("The update file does not come from the official release page.".to_string());
    }
    let dir = std::env::temp_dir().join("project-hub-update");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dest = dir.join(safe_file_name(name));
    let partial = dest.with_extension("part");

    let response = agent(Duration::from_secs(60 * 30))
        .get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| format!("Download failed: {e}"))?;
    let total = response
        .header("Content-Length")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(expected_size);

    let mut reader = response.into_reader();
    let mut file = File::create(&partial).map_err(|e| e.to_string())?;
    let mut buffer = vec![0u8; 64 * 1024];
    let mut downloaded = 0u64;
    let mut last_report = 0u64;
    loop {
        let read = reader.read(&mut buffer).map_err(|e| format!("Download failed: {e}"))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read]).map_err(|e| e.to_string())?;
        downloaded += read as u64;
        if downloaded - last_report >= 256 * 1024 {
            last_report = downloaded;
            progress(downloaded, total);
        }
    }
    file.flush().map_err(|e| e.to_string())?;
    drop(file);
    progress(downloaded, total);

    if expected_size > 0 && downloaded != expected_size {
        let _ = fs::remove_file(&partial);
        return Err("The downloaded file is incomplete. Please try again.".to_string());
    }
    let _ = fs::remove_file(&dest);
    fs::rename(&partial, &dest).map_err(|e| e.to_string())?;
    Ok(dest)
}

/// The app bundle this copy runs from, such as /Applications/Project Hub.app.
#[cfg(target_os = "macos")]
fn running_app_bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    // <name>.app/Contents/MacOS/<binary>
    let bundle = exe.parent()?.parent()?.parent()?;
    (bundle.extension()? == "app").then(|| bundle.to_path_buf())
}

/// Replaces the app bundle with the one in the downloaded disk image and opens it again.
/// Arguments: the app's process id, the disk image, the bundle to replace. Runs once the app has
/// quit; when the bundle cannot be replaced (no write access, for example) it keeps the old one and
/// opens the disk image instead. Output goes to $TMPDIR/project-hub-update.log.
#[cfg(target_os = "macos")]
const MACOS_REPLACE_SCRIPT: &str = r#"
pid="$1"; dmg="$2"; app="$3"
exec >>"${TMPDIR:-/tmp}/project-hub-update.log" 2>&1
echo "$(date) updating $app from $dmg"
while kill -0 "$pid" 2>/dev/null; do sleep 0.2; done

staged="$app.update"; old="$app.old"; mnt="$(mktemp -d)"
replace() {
  hdiutil attach -nobrowse -noautoopen -readonly -mountpoint "$mnt" "$dmg" || return 1
  new="$(ls -d "$mnt"/*.app 2>/dev/null | head -n 1)"
  rm -rf "$staged" "$old"
  [ -n "$new" ] && ditto "$new" "$staged" || return 1
  mv "$app" "$old" || return 1
  if ! mv "$staged" "$app"; then mv "$old" "$app"; return 1; fi
  rm -rf "$old"
}

if replace; then
  hdiutil detach -quiet "$mnt"
  open "$app"
else
  echo "could not replace the app; opening the disk image"
  rm -rf "$staged"
  hdiutil detach -quiet "$mnt" 2>/dev/null
  open "$dmg"
fi
rmdir "$mnt" 2>/dev/null
"#;

/// Starts the installer and arranges for the updated app to open again. The caller exits right after.
pub fn launch_installer(installer: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
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
            .map_err(|e| format!("Cannot start the installer: {e}"))?;
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::process::CommandExt;
        // Run from the build folder (or anywhere but an app bundle): leave the disk image to the user.
        let Some(bundle) = running_app_bundle() else {
            std::process::Command::new("open")
                .arg(installer)
                .spawn()
                .map_err(|e| format!("Cannot open the update: {e}"))?;
            return Ok(());
        };
        std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(MACOS_REPLACE_SCRIPT)
            .arg("project-hub-update")
            .arg(std::process::id().to_string())
            .arg(installer)
            .arg(&bundle)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            // Its own process group, so it outlives the app it is about to replace.
            .process_group(0)
            .spawn()
            .map_err(|e| format!("Cannot start the update: {e}"))?;
        Ok(())
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        use std::os::unix::fs::PermissionsExt;
        let target = PathBuf::from(
            std::env::var("APPIMAGE")
                .map_err(|_| "Automatic update only works for the AppImage build.".to_string())?,
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
            .map_err(|e| format!("Cannot restart: {e}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions() {
        assert!(is_newer("v0.2.0", "0.1.9"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(is_newer("0.1.10", "0.1.9"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        assert!(!is_newer("0.2.0-beta", "0.2.0"));
    }

    #[test]
    fn picks_the_installer_for_this_platform() {
        let assets = vec![
            Asset { name: "Project-Hub_0.2.0_x64-setup.exe".into(), browser_download_url: "u1".into(), size: 1 },
            Asset { name: "Project-Hub_0.2.0_aarch64.dmg".into(), browser_download_url: "u2".into(), size: 1 },
            Asset { name: "Project-Hub_0.2.0_amd64.AppImage".into(), browser_download_url: "u3".into(), size: 1 },
        ];
        let picked = pick_asset(&assets).unwrap();
        if cfg!(target_os = "windows") {
            assert_eq!(picked.browser_download_url, "u1");
        }
    }

    #[test]
    fn only_trusts_our_release_downloads() {
        assert!(is_trusted_url(&format!("https://github.com/{UPDATE_REPO}/releases/download/v0.2.0/a.exe")));
        assert!(!is_trusted_url("https://github.com/someone/else/releases/download/v1/a.exe"));
        assert!(!is_trusted_url("http://github.com/x"));
    }
}
