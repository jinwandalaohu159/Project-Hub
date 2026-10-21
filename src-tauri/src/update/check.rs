use super::{agent, releases_page, UPDATE_REPO, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::{
    sync::{Mutex, PoisonError},
    time::Duration,
};

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

#[derive(Deserialize, Clone)]
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

/// The release last fetched from the API. Checks only look up the latest tag through the website,
/// which the API's hourly limit (60 requests without signing in) does not cover, and ask the API
/// just once per new tag, so they can run every few minutes.
static LAST_RELEASE: Mutex<Option<Release>> = Mutex::new(None);

/// The latest release's tag, read from where github.com/<repo>/releases/latest redirects to
/// (".../releases/tag/v1.2.3"). None while nothing is published.
fn latest_tag() -> Result<Option<String>, String> {
    let url = format!("https://github.com/{UPDATE_REPO}/releases/latest");
    let response = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout(Duration::from_secs(30))
        .redirects(0)
        .build()
        .get(&url)
        .set("User-Agent", USER_AGENT)
        .call();
    match response {
        Ok(response) => Ok(response
            .header("Location")
            .and_then(|location| location.split_once("/releases/tag/"))
            .map(|(_, tag)| tag.to_string())),
        Err(ureq::Error::Status(404, _)) => Ok(None),
        Err(error) => Err(format!("Cannot reach GitHub: {error}")),
    }
}

fn fetch_release(tag: &str) -> Result<Release, String> {
    let url = format!("https://api.github.com/repos/{UPDATE_REPO}/releases/tags/{tag}");
    match agent(Duration::from_secs(30))
        .get(&url)
        .set("Accept", "application/vnd.github+json")
        .set("User-Agent", USER_AGENT)
        .call()
    {
        Ok(response) => response.into_json().map_err(|e| e.to_string()),
        Err(ureq::Error::Status(403 | 429, _)) => Err("UPDATE_RATE_LIMITED".to_string()),
        Err(error) => Err(format!("Cannot reach GitHub: {error}")),
    }
}

/// The latest published release, or None when there is none yet.
fn latest_release() -> Result<Option<Release>, String> {
    let Some(tag) = latest_tag()? else {
        return Ok(None);
    };
    let mut last = LAST_RELEASE.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(release) = last.as_ref().filter(|release| release.tag_name == tag) {
        return Ok(Some(release.clone()));
    }
    let release = fetch_release(&tag)?;
    *last = Some(release.clone());
    Ok(Some(release))
}

pub fn check(current: &str) -> Result<UpdateInfo, String> {
    let Some(release) = latest_release()? else {
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
        });
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
}
