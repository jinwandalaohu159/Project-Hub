use super::{agent, UPDATE_REPO, USER_AGENT};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
    time::Duration,
};

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
        return Err("UPDATE_UNTRUSTED".to_string());
    }
    let dir = std::env::temp_dir().join("project-hub-update");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dest = dir.join(safe_file_name(name));
    let partial = dest.with_extension("part");

    let response = agent(Duration::from_secs(60 * 30))
        .get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| format!("UPDATE_DOWNLOAD_FAILED:{e}"))?;
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
        let read = reader.read(&mut buffer).map_err(|e| format!("UPDATE_DOWNLOAD_FAILED:{e}"))?;
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
        return Err("UPDATE_INCOMPLETE".to_string());
    }
    let _ = fs::remove_file(&dest);
    fs::rename(&partial, &dest).map_err(|e| e.to_string())?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_trusts_our_release_downloads() {
        assert!(is_trusted_url(&format!("https://github.com/{UPDATE_REPO}/releases/download/v0.2.0/a.exe")));
        assert!(!is_trusted_url("https://github.com/someone/else/releases/download/v1/a.exe"));
        assert!(!is_trusted_url("http://github.com/x"));
    }
}
