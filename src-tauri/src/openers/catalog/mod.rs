//! The terminals, editors and file managers Project Hub knows how to open a folder with, and how
//! to find each one on this computer: one file per platform, sharing the lookups here. `{path}` in
//! arguments is the folder.

use super::{Launch, OpenerKind};
use std::path::{Path, PathBuf};

/// A known program: how it is listed and where it may be installed.
pub(super) struct Known {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: OpenerKind,
    pub find: fn() -> Option<Launch>,
}

macro_rules! known {
    ($id:literal, $name:literal, $kind:ident, $find:expr) => {
        Known { id: $id, name: $name, kind: OpenerKind::$kind, find: $find }
    };
}

// Each platform uses some of these helpers.

/// Whether something is installed at `path`. Windows "app execution aliases" (such as wt.exe from
/// the Store) are links whose target cannot be read, so the link itself counts.
fn installed(path: &Path) -> bool {
    path.exists() || std::fs::symlink_metadata(path).is_ok()
}

/// The first of `candidates` that exists.
#[allow(dead_code)]
fn first_existing(candidates: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    candidates.into_iter().find(|path| installed(path))
}

/// A program on the PATH, by any of its names.
#[allow(dead_code)]
fn on_path(names: &[&str]) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths).find_map(|dir| {
        names.iter().find_map(|name| {
            let candidate = dir.join(name);
            (installed(&candidate) && !candidate.is_dir()).then_some(candidate)
        })
    })
}

/// `<dir>/<prefix>*/<relative>` for every listed `dir`, newest-looking folder first (JetBrains
/// installs carry their version in the folder name).
#[allow(dead_code)]
fn in_versioned_dirs(dirs: &[PathBuf], prefix: &str, relative: &str) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = dirs
        .iter()
        .filter_map(|dir| std::fs::read_dir(dir).ok())
        .flatten()
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(prefix))
        .map(|entry| entry.path().join(relative))
        .filter(|path| path.is_file())
        .collect();
    found.sort();
    found.pop()
}

fn launch(program: PathBuf, args: &[&str]) -> Launch {
    Launch { program, args: args.iter().map(|arg| arg.to_string()).collect(), console: false }
}

#[allow(dead_code)]
fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_default()
}

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub(super) use windows::known;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub(super) use macos::known;

#[cfg(all(unix, not(target_os = "macos")))]
mod linux;
#[cfg(all(unix, not(target_os = "macos")))]
pub(super) use linux::known;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_a_file_manager_is_known() {
        let known = known();
        let mut ids: Vec<_> = known.iter().map(|k| k.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), known.len());
        assert!(known.iter().any(|k| matches!(k.kind, OpenerKind::Files)));
    }

    #[test]
    fn versioned_folders_pick_the_newest() {
        let dir = tempfile::tempdir().unwrap();
        for version in ["PyCharm 2023.3", "PyCharm 2024.2", "WebStorm 2024.1"] {
            std::fs::create_dir_all(dir.path().join(version).join("bin")).unwrap();
            std::fs::write(dir.path().join(version).join("bin").join("run"), "").unwrap();
        }
        let found = in_versioned_dirs(&[dir.path().to_path_buf()], "PyCharm", "bin/run").unwrap();
        assert!(found.to_string_lossy().contains("2024.2"));
        assert!(found.is_file());
    }
}
