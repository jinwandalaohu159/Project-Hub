//! Working-tree status: unsaved changes, empty-folder markers and default ignore rules.

#[cfg(windows)]
use crate::command::hidden_command;
use crate::{FileDiff, FileStatus, GitService, StatusSummary};
use std::fs;
use std::path::Path;

pub(crate) const KEEP_FILE_NAME: &str = ".project-hub-keep";
pub(crate) const DEFAULT_IGNORE_MARKER: &str = "# Project Hub default ignores";
pub(crate) const DEFAULT_IGNORE_RULES: &[&str] = &[
    "node_modules/",
    "dist/",
    "build/",
    "target/",
    ".cache/",
    ".vite/",
    ".next/",
    ".nuxt/",
    "coverage/",
    "*.log",
    ".DS_Store",
    ".env.local",
];
pub(crate) const IGNORED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "__pycache__",
    ".vscode",
    "dist",
    "build",
    "target",
    ".cache",
    ".vite",
    ".next",
    ".nuxt",
    "coverage",
    ".idea",
    "vendor",
    "out",
];

impl GitService {
    pub(crate) fn status_entries(&self, repo_path: &str) -> Result<Vec<FileDiff>, String> {
        self.ensure_default_ignores(repo_path)?;
        let output = self.run_git(
            repo_path,
            &[
                "status",
                "--porcelain=v1",
                "-z",
                "--ignore-submodules=dirty",
            ],
        )?;
        Ok(parse_porcelain_z(&output)
            .into_iter()
            .map(|mut diff| {
                diff.path = self.display_path_for_status(diff.path);
                diff
            })
            .collect())
    }

    pub fn get_status(&self, repo_path: &str) -> Result<Vec<FileDiff>, String> {
        let mut diffs = self.status_entries(repo_path)?;
        self.collect_unmarked_empty_dirs(
            Path::new(repo_path),
            Path::new(repo_path),
            true,
            &mut diffs,
        );
        Ok(diffs)
    }

    pub fn get_status_summary(
        &self,
        repo_path: &str,
        limit: usize,
    ) -> Result<StatusSummary, String> {
        let entries = self.status_entries(repo_path)?;
        let total = entries.len();
        let files = entries.into_iter().take(limit).collect();
        Ok(StatusSummary {
            files,
            total,
            truncated: total > limit,
        })
    }

    pub(crate) fn display_path_for_status(&self, path: String) -> String {
        let normalized = path.replace('\\', "/");
        let marker_suffix = format!("/{}", KEEP_FILE_NAME);
        if let Some(folder) = normalized.strip_suffix(&marker_suffix) {
            if !folder.is_empty() {
                return folder.to_string();
            }
        }
        normalized
    }

    pub fn has_unsaved_changes(&self, repo_path: &str) -> Result<bool, String> {
        for entry in self.status_entries(repo_path)? {
            if entry.status == FileStatus::Untracked {
                if self.path_has_versionable_content(&Path::new(repo_path).join(&entry.path)) {
                    return Ok(true);
                }
                continue;
            }
            return Ok(true);
        }
        Ok(self.has_unmarked_empty_dirs(Path::new(repo_path), true))
    }

    pub(crate) fn path_has_versionable_content(&self, path: &Path) -> bool {
        if path.is_file() {
            return true;
        }

        if path.is_dir() {
            let entries = match fs::read_dir(path) {
                Ok(entries) => entries,
                Err(_) => return true,
            };

            for entry in entries.flatten() {
                let entry_path = entry.path();
                let Some(name) = entry_path.file_name().and_then(|name| name.to_str()) else {
                    continue;
                };
                if self.is_ignored_dir_name(name)
                    || name == KEEP_FILE_NAME
                    || self.is_nested_git_worktree(&entry_path)
                {
                    continue;
                }
                if self.path_has_versionable_content(&entry_path) {
                    return true;
                }
            }
        }

        false
    }

    pub(crate) fn has_unmarked_empty_dirs(&self, dir: &Path, is_root: bool) -> bool {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return false,
        };

        let mut has_visible_entry = false;
        let mut has_keep_file = false;

        for entry in entries.flatten() {
            let entry_path = entry.path();
            let Some(name) = entry_path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };

            if name == KEEP_FILE_NAME {
                has_keep_file = true;
                continue;
            }

            if entry_path.is_dir() {
                if self.is_ignored_dir_name(name) || self.is_nested_git_worktree(&entry_path) {
                    continue;
                }
                has_visible_entry = true;
                if self.has_unmarked_empty_dirs(&entry_path, false) {
                    return true;
                }
            } else {
                has_visible_entry = true;
            }
        }

        !is_root && !has_visible_entry && !has_keep_file
    }

    pub(crate) fn collect_unmarked_empty_dirs(
        &self,
        root: &Path,
        dir: &Path,
        is_root: bool,
        diffs: &mut Vec<FileDiff>,
    ) -> bool {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return false,
        };

        let mut has_visible_entry = false;
        let mut has_keep_file = false;

        for entry in entries.flatten() {
            let entry_path = entry.path();
            let Some(name) = entry_path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };

            if name == KEEP_FILE_NAME {
                has_keep_file = true;
                continue;
            }

            if entry_path.is_dir() {
                if self.is_ignored_dir_name(name) || self.is_nested_git_worktree(&entry_path) {
                    continue;
                }
                has_visible_entry = true;
                self.collect_unmarked_empty_dirs(root, &entry_path, false, diffs);
            } else {
                has_visible_entry = true;
            }
        }

        let is_unmarked_empty_dir = !is_root && !has_visible_entry && !has_keep_file;
        if is_unmarked_empty_dir {
            if let Ok(relative) = dir.strip_prefix(root) {
                diffs.push(FileDiff {
                    path: relative.to_string_lossy().replace('\\', "/"),
                    status: FileStatus::Untracked,
                    old_path: None,
                });
            }
        }
        is_unmarked_empty_dir
    }

    pub(crate) fn sync_empty_dir_markers(&self, repo_path: &str) -> Result<(), String> {
        self.sync_empty_dir_markers_in_dir(Path::new(repo_path), true)
    }

    pub(crate) fn sync_empty_dir_markers_in_dir(&self, dir: &Path, is_root: bool) -> Result<(), String> {
        let entries = fs::read_dir(dir).map_err(|e| format!("FILE_ERROR:{e}"))?;
        let mut has_visible_entry = false;
        let mut keep_file_path = None;

        for entry in entries {
            let entry = entry.map_err(|e| format!("FILE_ERROR:{e}"))?;
            let entry_path = entry.path();
            let Some(name) = entry_path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };

            if name == KEEP_FILE_NAME {
                keep_file_path = Some(entry_path);
                continue;
            }

            if entry_path.is_dir() {
                if self.is_ignored_dir_name(name) || self.is_nested_git_worktree(&entry_path) {
                    continue;
                }
                self.sync_empty_dir_markers_in_dir(&entry_path, false)?;
                has_visible_entry = true;
            } else {
                has_visible_entry = true;
            }
        }

        if is_root {
            return Ok(());
        }

        match (has_visible_entry, keep_file_path) {
            (true, Some(path)) => {
                fs::remove_file(path)
                    .map_err(|e| format!("FILE_ERROR:{e}"))?;
            }
            (false, None) => {
                let marker_path = dir.join(KEEP_FILE_NAME);
                fs::write(
                    &marker_path,
                    "Project Hub keeps this empty folder in versions.\n",
                )
                .map_err(|e| format!("FILE_ERROR:{e}"))?;
                self.hide_marker_file(&marker_path);
            }
            (false, Some(path)) => {
                self.hide_marker_file(&path);
            }
            _ => {}
        }

        Ok(())
    }

    /// Untracked files the ignore rules keep out of the history, relative to the project with '/'
    /// separators. A wholly ignored folder is listed once, as "folder/".
    pub fn ignored_paths(&self, repo_path: &str) -> Result<Vec<String>, String> {
        let output = self.run_git(
            repo_path,
            &["ls-files", "--others", "--ignored", "--exclude-standard", "--directory", "-z"],
        )?;
        Ok(output.split('\0').filter(|path| !path.is_empty()).map(str::to_string).collect())
    }

    pub(crate) fn is_ignored_dir_name(&self, name: &str) -> bool {
        IGNORED_DIRS.contains(&name)
    }

    pub(crate) fn is_nested_git_worktree(&self, path: &Path) -> bool {
        path.join(".git").exists()
    }

    pub fn ensure_default_ignores(&self, repo_path: &str) -> Result<(), String> {
        let info_dir = Path::new(repo_path).join(".git").join("info");
        if !Path::new(repo_path).join(".git").is_dir() {
            return Ok(());
        }

        fs::create_dir_all(&info_dir)
            .map_err(|e| format!("FILE_ERROR:{e}"))?;
        let exclude_path = info_dir.join("exclude");
        let existing = fs::read_to_string(&exclude_path).unwrap_or_default();
        if existing.contains(DEFAULT_IGNORE_MARKER) {
            return Ok(());
        }

        let mut content = existing;
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push('\n');
        content.push_str(DEFAULT_IGNORE_MARKER);
        content.push('\n');
        for rule in DEFAULT_IGNORE_RULES {
            content.push_str(rule);
            content.push('\n');
        }

        fs::write(&exclude_path, content)
            .map_err(|e| format!("FILE_ERROR:{e}"))?;
        Ok(())
    }

    #[cfg(windows)]
    pub(crate) fn hide_marker_file(&self, path: &Path) {
        let _ = hidden_command("attrib").arg("+H").arg(path).status();
    }

    #[cfg(not(windows))]
    pub(crate) fn hide_marker_file(&self, _path: &Path) {}
}

/// Parses `git status --porcelain=v1 -z`; rename entries carry the old path as a separate field.
pub(crate) fn parse_porcelain_z(output: &str) -> Vec<FileDiff> {
    let mut diffs = Vec::new();
    let mut fields = output.split('\0');
    while let Some(entry) = fields.next() {
        if entry.len() < 4 {
            continue;
        }
        let code = &entry[..2];
        let path = entry[3..].to_string();
        let index_code = code.as_bytes()[0] as char;
        let worktree_code = code.as_bytes()[1] as char;

        if index_code == 'R' || index_code == 'C' {
            let old_path = fields.next().map(|old| old.replace('\\', "/"));
            diffs.push(FileDiff {
                path,
                status: FileStatus::Renamed,
                old_path,
            });
            continue;
        }

        let status = if code == "??" {
            FileStatus::Untracked
        } else if index_code == 'A' {
            FileStatus::Added
        } else if index_code == 'D' || worktree_code == 'D' {
            FileStatus::Deleted
        } else {
            FileStatus::Modified
        };
        diffs.push(FileDiff {
            path,
            status,
            old_path: None,
        });
    }
    diffs
}
