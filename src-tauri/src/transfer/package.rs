//! The package file itself: scratch space while building or reading one, zip errors, and opening a
//! package with its manifest checked.

use super::{ExportManifest, FORMAT, FORMAT_VERSION, MANIFEST};
use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use zip::ZipArchive;

pub(super) fn scratch_dir(kind: &str) -> Result<PathBuf, String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let dir = std::env::temp_dir().join(format!("project-hub-{kind}-{nanos}"));
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub(super) fn zip_error(error: zip::result::ZipError) -> String {
    format!("PACKAGE_ERROR:{error}")
}

/// Ids become file and folder names; reject anything that could escape the scratch folder.
fn is_safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

pub(super) fn open_package(path: &Path) -> Result<(ZipArchive<File>, ExportManifest), String> {
    let file = File::open(path).map_err(|e| format!("PACKAGE_OPEN_FAILED:{e}"))?;
    let mut archive = ZipArchive::new(file).map_err(|_| "EXPORT_NOT_PROJECT_HUB".to_string())?;
    let mut content = String::new();
    archive
        .by_name(MANIFEST)
        .map_err(|_| "EXPORT_NOT_PROJECT_HUB".to_string())?
        .read_to_string(&mut content)
        .map_err(|e| e.to_string())?;
    let manifest: ExportManifest =
        serde_json::from_str(&content).map_err(|_| "EXPORT_DAMAGED".to_string())?;
    if manifest.format != FORMAT {
        return Err("EXPORT_NOT_PROJECT_HUB".to_string());
    }
    if manifest.version > FORMAT_VERSION {
        return Err("EXPORT_TOO_NEW".to_string());
    }
    if !manifest.projects.iter().all(|project| is_safe_id(&project.id)) {
        return Err("EXPORT_DAMAGED".to_string());
    }
    Ok((archive, manifest))
}

pub(super) fn extract_entry(archive: &mut ZipArchive<File>, name: &str, dest: &Path) -> Result<(), String> {
    let mut entry = archive.by_name(name).map_err(zip_error)?;
    let mut out = File::create(dest).map_err(|e| e.to_string())?;
    io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
    Ok(())
}
