//! Working-tree status: unsaved changes and empty-folder markers.
//!
//! Git cannot record an empty folder, so Project Hub keeps one by saving a hidden marker file in it.
//! An empty folder without that marker is therefore an unsaved change like any new file, and every
//! view of "unsaved" (the change list, upload, switching) goes through [`GitService::status_entries`]
//! so they always agree. Folders git ignores are never looked into nor given a marker.

#[cfg(windows)]
use crate::command::hidden_command;
use crate::default_ignores::hides_folder;
use crate::{FileDiff, FileStatus, GitService, StatusSummary};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) const KEEP_FILE_NAME: &str = ".project-hub-keep";

/// What the empty-folder rules see inside one folder.
#[derive(Default)]
struct FolderListing {
    /// Subfolders to look into: absolute path and path relative to the project ('/'-separated).
    subdirs: Vec<(PathBuf, String)>,
    /// Anything that keeps the folder from being empty.
    has_content: bool,
    marker: Option<PathBuf>,
}

impl GitService {
    /// Every unsaved change, i.e. everything a save would record: what git reports plus empty
    /// folders still missing their marker.
    pub(crate) fn status_entries(&self, repo_path: &str) -> Result<Vec<FileDiff>, String> {
        let (mut diffs, ignored) = self.git_status(repo_path)?;
        let mut empty_dirs = Vec::new();
        self.collect_unmarked_empty_dirs(Path::new(repo_path), "", &ignored, &mut empty_dirs);

        // A folder git already lists as new ("dir/") covers the empty folders inside it.
        let new_dirs: Vec<String> = diffs
            .iter()
            .filter(|diff| diff.status == FileStatus::Untracked && diff.path.ends_with('/'))
            .map(|diff| diff.path.clone())
            .collect();
        diffs.extend(
            empty_dirs
                .into_iter()
                .filter(|dir| !new_dirs.iter().any(|new_dir| dir.starts_with(new_dir.as_str())))
                .map(|path| FileDiff {
                    path,
                    status: FileStatus::Untracked,
                    old_path: None,
                }),
        );
        Ok(diffs)
    }

    /// One `git status`: the changes it reports and the folders it ignores (relative to the
    /// project, '/'-separated, no trailing slash).
    fn git_status(&self, repo_path: &str) -> Result<(Vec<FileDiff>, HashSet<String>), String> {
        self.ensure_default_ignores(repo_path)?;
        let output = self.run_git(
            repo_path,
            &[
                "status",
                "--porcelain=v1",
                "-z",
                "--ignore-submodules=dirty",
                // A user's status.showUntrackedFiles=no must not hide new files.
                "--untracked-files=normal",
                // Names ignored folders (even empty ones) without listing what is inside them.
                "--ignored=matching",
            ],
        )?;
        let diffs = parse_porcelain_z(&output)
            .into_iter()
            .map(|mut diff| {
                diff.path = self.display_path_for_status(diff.path);
                diff
            })
            .collect();
        Ok((diffs, parse_ignored_dirs(&output)))
    }

    pub fn get_status(&self, repo_path: &str) -> Result<Vec<FileDiff>, String> {
        self.status_entries(repo_path)
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
        Ok(!self.status_entries(repo_path)?.is_empty())
    }

    /// Lists `dir` for the empty-folder rules; `rel` is `dir` relative to the project ("" for the
    /// project itself). Folders git ignores and nested repositories are neither looked into nor
    /// counted; the fixed tool folders (dependencies, caches) never count as content.
    fn list_folder(&self, dir: &Path, rel: &str, ignored: &HashSet<String>) -> Option<FolderListing> {
        let entries = fs::read_dir(dir).ok()?;
        let mut listing = FolderListing::default();
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name == KEEP_FILE_NAME {
                listing.marker = Some(path);
                continue;
            }
            // Links (including Windows junctions) are recorded as links, never followed.
            let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
            if !is_dir {
                listing.has_content = true;
                continue;
            }
            if name == ".git" || hides_folder(&name, rel.is_empty()) || self.is_nested_git_worktree(&path) {
                continue;
            }
            listing.has_content = true;
            let child_rel = if rel.is_empty() { name.into_owned() } else { format!("{rel}/{name}") };
            if !ignored.contains(&child_rel) {
                listing.subdirs.push((path, child_rel));
            }
        }
        Some(listing)
    }

    /// Collects the folders below `dir` that hold nothing and have no marker yet.
    fn collect_unmarked_empty_dirs(
        &self,
        dir: &Path,
        rel: &str,
        ignored: &HashSet<String>,
        out: &mut Vec<String>,
    ) {
        let Some(listing) = self.list_folder(dir, rel, ignored) else {
            return;
        };
        for (child, child_rel) in &listing.subdirs {
            self.collect_unmarked_empty_dirs(child, child_rel, ignored, out);
        }
        if !rel.is_empty() && !listing.has_content && listing.marker.is_none() {
            out.push(rel.to_string());
        }
    }

    /// Gives every empty folder its marker and removes markers from folders that are no longer
    /// empty, so the next record keeps the folder layout as it is. Best effort: a folder that cannot
    /// be read or written is left alone (and stays listed as unsaved) rather than blocking the save.
    pub(crate) fn sync_empty_dir_markers(&self, repo_path: &str) -> Result<(), String> {
        let (_, ignored) = self.git_status(repo_path)?;
        self.sync_empty_dir_markers_in(Path::new(repo_path), "", &ignored);
        Ok(())
    }

    fn sync_empty_dir_markers_in(&self, dir: &Path, rel: &str, ignored: &HashSet<String>) {
        let Some(listing) = self.list_folder(dir, rel, ignored) else {
            return;
        };
        for (child, child_rel) in &listing.subdirs {
            self.sync_empty_dir_markers_in(child, child_rel, ignored);
        }
        if rel.is_empty() {
            return;
        }

        match (listing.has_content, listing.marker) {
            (true, Some(marker)) => {
                let _ = fs::remove_file(marker);
            }
            (false, None) => {
                let marker = dir.join(KEEP_FILE_NAME);
                if fs::write(&marker, "Project Hub keeps this empty folder in versions.\n").is_ok() {
                    self.hide_marker_file(&marker);
                }
            }
            (false, Some(marker)) => self.hide_marker_file(&marker),
            (true, None) => {}
        }
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

    pub(crate) fn is_nested_git_worktree(&self, path: &Path) -> bool {
        path.join(".git").exists()
    }

    #[cfg(windows)]
    pub(crate) fn hide_marker_file(&self, path: &Path) {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        // Every save visits every marker; only start `attrib` for one that is not hidden yet.
        if fs::metadata(path).is_ok_and(|meta| meta.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0) {
            return;
        }
        let _ = hidden_command("attrib").arg("+H").arg(path).status();
    }

    #[cfg(not(windows))]
    pub(crate) fn hide_marker_file(&self, _path: &Path) {}
}

/// Parses `git status --porcelain=v1 -z`; rename entries carry the old path as a separate field.
/// Ignored entries ("!!") are not changes and are skipped.
pub(crate) fn parse_porcelain_z(output: &str) -> Vec<FileDiff> {
    let mut diffs = Vec::new();
    let mut fields = output.split('\0');
    while let Some(entry) = fields.next() {
        if entry.len() < 4 {
            continue;
        }
        let code = &entry[..2];
        if code == "!!" {
            continue;
        }
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

/// The ignored folders ("!! dir/") in `git status --porcelain=v1 -z --ignored` output, without the
/// trailing slash. A rename's second field is a file path, which never ends in '/'.
pub(crate) fn parse_ignored_dirs(output: &str) -> HashSet<String> {
    output
        .split('\0')
        .filter_map(|entry| entry.strip_prefix("!! "))
        .filter_map(|path| path.strip_suffix('/'))
        .map(|path| path.replace('\\', "/"))
        .collect()
}
