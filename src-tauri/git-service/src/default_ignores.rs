//! The hiding rules Project Hub gives every project: caches and build output of common languages
//! and tools. Plain gitignore rules, kept in the project's `.git/info/exclude` (this computer
//! only), in a block of their own so the user's rules there are never touched.
//!
//! Names only tools use are hidden at any depth; generic names (`build`, `dist`…) only at the
//! project root, so a folder of code that happens to share the name is still saved.

use crate::GitService;
use std::fs;
use std::path::{Component, Path};

pub(crate) const DEFAULT_IGNORE_MARKER: &str = "# Project Hub default ignores";

pub(crate) const DEFAULT_IGNORE_RULES: &[&str] = &[
    // Anywhere
    "*.log",
    ".DS_Store",
    "Thumbs.db",
    ".env.local",
    // Build output, at the project root only
    "/build/",
    "/dist/",
    "/target/",
    "/coverage/",
    // JavaScript / TypeScript
    "node_modules/",
    ".cache/",
    ".vite/",
    ".next/",
    ".nuxt/",
    ".svelte-kit/",
    ".angular/",
    ".parcel-cache/",
    ".turbo/",
    ".eslintcache",
    "*.tsbuildinfo",
    // Python
    "__pycache__/",
    "*.pyc",
    ".venv/",
    ".pytest_cache/",
    ".mypy_cache/",
    ".ruff_cache/",
    ".tox/",
    ".ipynb_checkpoints/",
    "*.egg-info/",
    // Java / Kotlin / Android
    ".gradle/",
    ".kotlin/",
    // C / C++
    "*.o",
    "*.obj",
    "cmake-build-*/",
    // C# / .NET
    ".vs/",
    // Swift / Xcode
    ".build/",
    "DerivedData/",
    "xcuserdata/",
    // Dart / Flutter
    ".dart_tool/",
    // Ruby
    ".bundle/",
    // Terraform
    ".terraform/",
];

/// `existing` (the exclude file) with the current default block in it, or `None` when it already
/// has exactly that. An older block is replaced in place; everything else stays as it was.
fn with_current_defaults(existing: &str) -> Option<String> {
    let block: String = std::iter::once(DEFAULT_IGNORE_MARKER)
        .chain(DEFAULT_IGNORE_RULES.iter().copied())
        .map(|line| format!("{line}\n"))
        .collect();
    let lines: Vec<&str> = existing.lines().collect();
    let Some(start) = lines.iter().position(|line| line.trim() == DEFAULT_IGNORE_MARKER) else {
        let mut content = existing.to_string();
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push('\n');
        content.push_str(&block);
        return Some(content);
    };
    // The block runs from the marker to the first blank line.
    let end = lines[start..].iter().position(|line| line.trim().is_empty()).map_or(lines.len(), |n| start + n);
    if lines[start..end].iter().map(|line| format!("{line}\n")).collect::<String>() == block {
        return None;
    }
    let mut content: String = lines[..start].iter().map(|line| format!("{line}\n")).collect();
    content.push_str(&block);
    content.extend(lines[end..].iter().map(|line| format!("{line}\n")));
    Some(content)
}

/// Whether a default rule hides a folder called `name` (directly in the project when `at_root`).
pub(crate) fn hides_folder(name: &str, at_root: bool) -> bool {
    DEFAULT_IGNORE_RULES.iter().any(|rule| {
        let Some(pattern) = rule.strip_suffix('/') else {
            return false;
        };
        let pattern = match pattern.strip_prefix('/') {
            Some(root_only) if at_root => root_only,
            Some(_) => return false,
            None => pattern,
        };
        name_matches(pattern, name)
    })
}

/// Whether a default rule for files (`*.log`, `.DS_Store`) hides one called `name`.
fn hides_file(name: &str) -> bool {
    DEFAULT_IGNORE_RULES
        .iter()
        .filter(|rule| !rule.ends_with('/'))
        .any(|pattern| name_matches(pattern, name))
}

/// `pattern` is a name with at most one `*`.
fn name_matches(pattern: &str, name: &str) -> bool {
    match pattern.split_once('*') {
        Some((prefix, suffix)) => name.len() >= prefix.len() + suffix.len() && name.starts_with(prefix) && name.ends_with(suffix),
        None => name == pattern,
    }
}

/// Whether the default rules hide `relative`, a path inside a project: it lies in a hidden folder
/// or is a hidden file. A hidden folder itself is not (its appearance is worth showing).
pub fn hidden_by_default(relative: &Path) -> bool {
    let names: Vec<&str> = relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect();
    let Some((last, folders)) = names.split_last() else {
        return false;
    };
    folders.iter().enumerate().any(|(depth, name)| hides_folder(name, depth == 0)) || hides_file(last)
}

impl GitService {
    /// Writes the default rules into the project's exclude file, or brings an older set up to date.
    pub fn ensure_default_ignores(&self, repo_path: &str) -> Result<(), String> {
        let git_dir = Path::new(repo_path).join(".git");
        if !git_dir.is_dir() {
            return Ok(());
        }
        let info_dir = git_dir.join("info");
        let exclude_path = info_dir.join("exclude");
        let existing = fs::read_to_string(&exclude_path).unwrap_or_default();
        let Some(content) = with_current_defaults(&existing) else {
            return Ok(());
        };
        fs::create_dir_all(&info_dir).map_err(|e| format!("FILE_ERROR:{e}"))?;
        fs::write(&exclude_path, content).map_err(|e| format!("FILE_ERROR:{e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_the_block_once_and_keeps_the_users_rules() {
        let first = with_current_defaults("# git's own comment\nmy-secret.txt").unwrap();
        assert!(first.starts_with("# git's own comment\nmy-secret.txt\n\n# Project Hub default ignores\n"));
        assert!(first.contains("\n/build/\n") && first.contains("\n__pycache__/\n"));
        assert_eq!(with_current_defaults(&first), None);
    }

    #[test]
    fn replaces_an_older_block_in_place() {
        let old = "mine.txt\n\n# Project Hub default ignores\nnode_modules/\nbuild/\ndist/\n\nalso-mine/\n";
        let updated = with_current_defaults(old).unwrap();
        assert!(updated.starts_with("mine.txt\n\n# Project Hub default ignores\n*.log\n"));
        assert!(updated.ends_with("\n\nalso-mine/\n"), "{updated}");
        assert!(!updated.lines().any(|line| line == "build/"));
        assert_eq!(with_current_defaults(&updated), None);
    }

    #[test]
    fn generic_names_are_hidden_only_at_the_root() {
        assert!(hides_folder("build", true));
        assert!(!hides_folder("build", false));
        assert!(hides_folder("node_modules", false));
        assert!(hides_folder("cmake-build-debug", false));
        assert!(hides_folder("my_pkg.egg-info", false));
        assert!(!hides_folder("vendor", true));
        assert!(!hides_folder(".vscode", false));
    }

    #[test]
    fn paths_inside_hidden_folders_and_hidden_files_are_hidden() {
        let hidden = |path: &str| hidden_by_default(Path::new(path));
        assert!(hidden("node_modules/vue/index.js"));
        assert!(hidden("web/node_modules/vue/index.js"));
        assert!(hidden("target/debug/app"));
        assert!(hidden("logs/server.log"));
        assert!(hidden("src/.DS_Store"));
        assert!(!hidden("src/build/tool.rs"));
        assert!(!hidden("node_modules"));
        assert!(!hidden("src/main.rs"));
        assert!(!hidden(""));
    }
}
