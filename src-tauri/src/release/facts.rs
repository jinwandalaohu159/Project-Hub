//! What a project's own files say about releasing it: version numbers, whether it has a cloud
//! build, and version arithmetic.

use serde_json::Value;
use std::path::Path;

/// "1.2.0" or "v1.2.0" -> "v1.2.0"; refuses what git would not take as a tag.
pub fn tag_for(version: &str) -> Result<String, String> {
    let version = version.trim();
    let valid = !version.is_empty()
        && version.chars().all(|ch| ch.is_ascii_alphanumeric() || ".-+_".contains(ch))
        && version.trim_start_matches(['v', 'V']).starts_with(|ch: char| ch.is_ascii_digit());
    if !valid {
        return Err(format!("\"{version}\" is not a version number; use something like 1.2.0."));
    }
    Ok(if version.starts_with(['v', 'V']) { format!("v{}", &version[1..]) } else { format!("v{version}") })
}

fn version_parts(version: &str) -> Vec<u64> {
    version
        .trim_start_matches(['v', 'V'])
        .split(['-', '+'])
        .next()
        .unwrap_or_default()
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}

/// The next patch version after `latest`, or the project's own version when that is already newer.
pub fn suggest_version(latest: Option<&str>, in_files: Option<&str>) -> String {
    let bumped = latest.map(|tag| {
        let mut parts = version_parts(tag);
        parts.resize(3, 0);
        parts[2] += 1;
        parts.iter().map(u64::to_string).collect::<Vec<_>>().join(".")
    });
    match (bumped, in_files) {
        (Some(bumped), Some(own)) if version_parts(own) > version_parts(&bumped) => own.to_string(),
        (Some(bumped), _) => bumped,
        (None, Some(own)) => own.to_string(),
        (None, None) => "0.1.0".to_string(),
    }
}

/// Version numbers written in the project's usual manifest files, as (file, version).
pub fn versions_in_files(root: &Path) -> Vec<(String, String)> {
    let json_version = |file: &str| {
        let text = std::fs::read_to_string(root.join(file)).ok()?;
        serde_json::from_str::<Value>(&text).ok()?.get("version")?.as_str().map(str::to_string)
    };
    // `version = "…"` in one of the given [sections] of a TOML file.
    let toml_version = |file: &str, sections: &[&str]| {
        let text = std::fs::read_to_string(root.join(file)).ok()?;
        let mut in_section = false;
        for line in text.lines().map(str::trim) {
            if line.starts_with('[') {
                in_section = sections.contains(&line);
            } else if in_section {
                if let Some(value) = line.strip_prefix("version").map(str::trim).and_then(|rest| rest.strip_prefix('=')) {
                    return Some(value.trim().trim_matches('"').to_string());
                }
            }
        }
        None
    };
    [
        ("package.json", json_version("package.json")),
        ("Cargo.toml", toml_version("Cargo.toml", &["[package]", "[workspace.package]"])),
        ("src-tauri/Cargo.toml", toml_version("src-tauri/Cargo.toml", &["[package]"])),
        ("src-tauri/tauri.conf.json", json_version("src-tauri/tauri.conf.json")),
        ("pyproject.toml", toml_version("pyproject.toml", &["[project]", "[tool.poetry]"])),
    ]
    .into_iter()
    .filter_map(|(file, version)| Some((file.to_string(), version.filter(|v| !v.is_empty())?)))
    .collect()
}

/// GitHub Actions workflows started by pushing a tag: the project's cloud build.
pub fn tag_workflows(root: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(root.join(".github").join("workflows")) else {
        return Vec::new();
    };
    let mut found: Vec<String> = entries
        .flatten()
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            (name.ends_with(".yml") || name.ends_with(".yaml"))
                && std::fs::read_to_string(entry.path()).is_ok_and(|text| text.contains("push") && text.contains("tags"))
        })
        .map(|entry| format!(".github/workflows/{}", entry.file_name().to_string_lossy()))
        .collect();
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_become_tags() {
        assert_eq!(tag_for("1.2.0").unwrap(), "v1.2.0");
        assert_eq!(tag_for(" V2.0.0-beta.1 ").unwrap(), "v2.0.0-beta.1");
        assert!(tag_for("latest").is_err());
        assert!(tag_for("1.0 final").is_err());
    }

    #[test]
    fn the_next_version_follows_the_latest_release() {
        assert_eq!(suggest_version(Some("v0.3.14"), None), "0.3.15");
        assert_eq!(suggest_version(Some("v1.2"), Some("1.0.0")), "1.2.1");
        // Already bumped in the project's files.
        assert_eq!(suggest_version(Some("v0.3.14"), Some("0.4.0")), "0.4.0");
        assert_eq!(suggest_version(None, Some("2.1.0")), "2.1.0");
        assert_eq!(suggest_version(None, None), "0.1.0");
    }

    #[test]
    fn finds_versions_and_tag_workflows_in_the_project() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("package.json"), r#"{ "name": "x", "version": "1.4.0" }"#).unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[dependencies]\nversion = \"9\"\n[package]\nname = \"x\"\nversion = \"1.4.1\"\n").unwrap();
        let workflows = dir.path().join(".github").join("workflows");
        std::fs::create_dir_all(&workflows).unwrap();
        std::fs::write(workflows.join("release.yml"), "on:\n  push:\n    tags: ['v*']\n").unwrap();
        std::fs::write(workflows.join("ci.yml"), "on:\n  pull_request:\n").unwrap();

        assert_eq!(
            versions_in_files(dir.path()),
            vec![("package.json".to_string(), "1.4.0".to_string()), ("Cargo.toml".to_string(), "1.4.1".to_string())]
        );
        assert_eq!(tag_workflows(dir.path()), vec![".github/workflows/release.yml"]);
    }
}
