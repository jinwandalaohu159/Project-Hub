//! Files the version history does not carry but a moved project still needs: for a versioned
//! project the ones its ignore rules keep out (`.env`, local settings, keys), for a plain folder all
//! of them. Folders the project's tools can rebuild (dependencies, build output, caches) stay
//! behind; on the new computer `npm install` or a build brings them back.

use git_service::GitService;
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{self, Write},
    path::{Component, Path, PathBuf},
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

/// Rebuilt by the project's tools, so not worth carrying (and often tied to the old system).
const REGENERABLE_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "bower_components",
    ".pnpm-store",
    "dist",
    "build",
    "out",
    "target",
    ".cache",
    ".parcel-cache",
    ".turbo",
    ".vite",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".angular",
    "coverage",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".venv",
    "venv",
    ".gradle",
    ".dart_tool",
    "Pods",
    "DerivedData",
];

/// Leftovers nobody misses on the new computer.
fn is_junk(name: &str) -> bool {
    matches!(name, ".DS_Store" | "Thumbs.db" | "desktop.ini") || name.ends_with(".log")
}

/// Where a project's files sit inside the package.
fn entry_name(project_id: &str, path: &str) -> String {
    format!("files/{project_id}/{path}")
}

/// The files to carry for the project at `root`, relative to it with '/' separators, sorted.
pub fn collect(root: &Path, is_git_repo: bool, git: &GitService) -> Result<Vec<String>, String> {
    // A set: git can list an ignored folder and a file inside it, which would otherwise come twice.
    let mut files = BTreeSet::new();
    if !is_git_repo {
        walk(root, "", &mut files)?;
        return Ok(files.into_iter().collect());
    }
    for path in git.ignored_paths(&root.to_string_lossy())? {
        let relative = path.trim_end_matches('/');
        if relative.split('/').any(|part| REGENERABLE_DIRS.contains(&part)) {
            continue;
        }
        if path.ends_with('/') {
            walk(&root.join(relative), relative, &mut files)?;
        } else if !relative.rsplit('/').next().is_some_and(is_junk) && root.join(relative).is_file() {
            files.insert(relative.to_string());
        }
    }
    Ok(files.into_iter().collect())
}

/// Adds the files under `dir` (which is `prefix` inside the project), leaving out links,
/// rebuildable folders and junk.
fn walk(dir: &Path, prefix: &str, files: &mut BTreeSet<String>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())?.flatten() {
        let Ok(kind) = entry.file_type() else { continue };
        let name = entry.file_name().to_string_lossy().to_string();
        let relative = if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
        if kind.is_dir() && !REGENERABLE_DIRS.contains(&name.as_str()) {
            walk(&entry.path(), &relative, files)?;
        } else if kind.is_file() && !is_junk(&name) {
            files.insert(relative);
        }
    }
    Ok(())
}

/// Writes the files into the package, keeping their permissions (key files are often private).
pub fn write(
    zip: &mut ZipWriter<File>,
    project_id: &str,
    root: &Path,
    files: &[String],
) -> Result<(), String> {
    for path in files {
        let source = root.join(path);
        // Only Unix records permissions.
        #[cfg_attr(not(unix), allow(unused_mut))]
        let mut options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .large_file(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = fs::metadata(&source) {
                options = options.unix_permissions(meta.permissions().mode());
            }
        }
        zip.start_file(entry_name(project_id, path), options)
            .map_err(|e| e.to_string())?;
        let mut file = File::open(&source).map_err(|e| format!("LOCAL_FILE_FAILED:{path} ({e})"))?;
        io::copy(&mut file, zip).map_err(|e| format!("LOCAL_FILE_FAILED:{path} ({e})"))?;
    }
    Ok(())
}

/// `path` as a location inside the project, or None when it could point anywhere else.
fn inside(root: &Path, path: &str) -> Option<PathBuf> {
    let relative = Path::new(path);
    let plain = relative.components().all(|part| matches!(part, Component::Normal(_)));
    (plain && !path.is_empty() && !path.contains('\\')).then(|| root.join(relative))
}

/// Puts the files back under `root`. A file that already exists there is kept as it is, so an
/// import never overwrites what is on the new computer.
pub fn restore(archive: &mut ZipArchive<File>, project_id: &str, root: &Path, files: &[String]) -> Result<(), String> {
    for path in files {
        let dest = inside(root, path).ok_or_else(|| format!("EXPORT_DAMAGED:{path}"))?;
        if dest.exists() {
            continue;
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut entry = archive
            .by_name(&entry_name(project_id, path))
            .map_err(|e| format!("EXPORT_DAMAGED:{e}"))?;
        let mut out = File::create(&dest).map_err(|e| format!("LOCAL_FILE_FAILED:{path} ({e})"))?;
        io::copy(&mut entry, &mut out).map_err(|e| format!("LOCAL_FILE_FAILED:{path} ({e})"))?;
        out.flush().map_err(|e| e.to_string())?;
        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&dest, fs::Permissions::from_mode(mode & 0o777));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_package_paths_inside_the_project() {
        let root = Path::new("/projects/demo");
        assert_eq!(inside(root, "config/.env"), Some(root.join("config/.env")));
        assert_eq!(inside(root, "../escape"), None);
        assert_eq!(inside(root, "/etc/passwd"), None);
        assert_eq!(inside(root, "a\\..\\..\\b"), None);
        assert_eq!(inside(root, ""), None);
    }

    #[test]
    fn plain_folders_skip_rebuildable_folders_and_junk() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        for path in ["src/main.ts", ".env", "node_modules/pkg/index.js", "dist/app.js", "src/.DS_Store", "debug.log"] {
            fs::create_dir_all(root.join(path).parent().unwrap()).unwrap();
            fs::write(root.join(path), "x").unwrap();
        }
        assert_eq!(collect(root, false, &GitService::new()).unwrap(), vec![".env", "src/main.ts"]);
    }
}
