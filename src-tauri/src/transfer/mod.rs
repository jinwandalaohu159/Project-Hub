//! Export / import of everything Project Hub knows, for moving to another computer: each project as
//! it is, ready to use on the new one after its dependencies are installed again.
//! The package is a zip: `manifest.json`, one git bundle per project (all versions, save records and
//! unsaved changes), the local files the history leaves out (see `local_files`) and the per-project
//! operation logs.
//!
//! `export` writes a package, `import` previews and restores one, `package` holds what both share.

mod export;
mod import;
mod local_files;
mod package;

use chrono::{DateTime, Utc};
use git_service::GitService;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

pub use export::export;
pub use import::{apply, preview};

const FORMAT: &str = "project-hub-export";
const FORMAT_VERSION: u32 = 1;
const MANIFEST: &str = "manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportManifest {
    pub format: String,
    pub version: u32,
    pub exported_at: DateTime<Utc>,
    #[serde(default)]
    pub author_name: String,
    #[serde(default)]
    pub author_email: String,
    pub projects: Vec<ExportedProject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedProject {
    pub id: String,
    pub name: String,
    pub original_path: String,
    pub head_branch: Option<String>,
    pub remote_url: Option<String>,
    pub bundle: Option<String>,
    pub has_draft: bool,
    /// Files outside the version history, under `files/<id>/` (missing in older packages).
    #[serde(default)]
    pub local_files: Vec<String>,
    #[serde(default)]
    pub logs: Vec<String>,
}

/// Something that went wrong with one project; `message` is an error code the UI translates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectIssue {
    pub project: String,
    pub message: String,
}

impl ProjectIssue {
    fn new(project: &str, message: impl Into<String>) -> Self {
        Self { project: project.to_string(), message: message.into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportReport {
    pub exported: usize,
    pub warnings: Vec<ProjectIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreviewItem {
    pub id: String,
    pub name: String,
    pub original_path: String,
    pub suggested_path: String,
    pub has_history: bool,
    pub has_draft: bool,
    pub local_files: usize,
    pub registered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreview {
    pub exported_at: DateTime<Utc>,
    pub author_name: String,
    pub author_email: String,
    pub projects: Vec<ImportPreviewItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportItem {
    pub id: String,
    pub target_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportReport {
    pub imported: Vec<String>,
    pub errors: Vec<ProjectIssue>,
}

/// What a target folder looks like before importing into it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetState {
    Missing,
    Empty,
    Files,
    Repo,
}

pub fn target_state(path: &str) -> TargetState {
    let dir = Path::new(path);
    if !dir.is_dir() {
        return TargetState::Missing;
    }
    if dir.join(".git").exists() && GitService::new().has_commits(path) {
        return TargetState::Repo;
    }
    // A repository without records is restored into like a plain folder.
    let is_empty = fs::read_dir(dir).is_ok_and(|entries| entries.flatten().all(|entry| entry.file_name() == ".git"));
    if is_empty {
        TargetState::Empty
    } else {
        TargetState::Files
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use project_service::ProjectService;
    use std::process::Command;

    fn git(path: &Path, args: &[&str]) {
        Command::new("git").args(args).current_dir(path).output().unwrap();
    }

    #[test]
    fn export_then_import_on_a_fresh_machine() {
        let source_root = tempfile::TempDir::new().unwrap();
        let source = source_root.path().join("demo");
        fs::create_dir_all(&source).unwrap();
        git(&source, &["init"]);
        git(&source, &["config", "user.email", "t@t.com"]);
        git(&source, &["config", "user.name", "T"]);
        fs::write(source.join("a.txt"), "saved").unwrap();
        fs::write(source.join(".gitignore"), ".env\nconfig/secret.json\nnode_modules/\n").unwrap();
        git(&source, &["add", "."]);
        git(&source, &["commit", "-m", "first"]);
        fs::write(source.join("wip.txt"), "unsaved").unwrap();
        // Local files the history leaves out: carried along, except what `npm install` rebuilds.
        fs::write(source.join(".env"), "KEY=secret").unwrap();
        fs::create_dir_all(source.join("config")).unwrap();
        fs::write(source.join("config/secret.json"), "{}").unwrap();
        fs::create_dir_all(source.join("node_modules/pkg")).unwrap();
        fs::write(source.join("node_modules/pkg/index.js"), "x").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(source.join(".env"), fs::Permissions::from_mode(0o600)).unwrap();
        }

        let logs_root = tempfile::TempDir::new().unwrap();
        let storage_root = tempfile::TempDir::new().unwrap();
        let storage = storage_root.path().join("projects.json");
        let mut service = ProjectService::new(storage.to_str().unwrap());
        let project = service.add_project(source.to_str().unwrap()).unwrap();
        let log_dir = logs_root.path().join(&project.id);
        fs::create_dir_all(&log_dir).unwrap();
        fs::write(log_dir.join("project-hub.log"), "{\"line\":1}\n").unwrap();

        let package = storage_root.path().join("backup.phub");
        let git_service = GitService::new();
        let logs_for = |id: &str| Some(logs_root.path().join(id));
        let report = export(&package, &service.list_projects(), "Me", "me@x.com", &git_service, logs_for).unwrap();
        assert_eq!(report.exported, 1);
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);

        // "New machine": empty project list, different folders, empty log store.
        let new_storage_root = tempfile::TempDir::new().unwrap();
        let new_storage = new_storage_root.path().join("projects.json");
        let mut fresh = ProjectService::new(new_storage.to_str().unwrap());
        let preview_result = preview(&package, &fresh).unwrap();
        assert_eq!(preview_result.author_name, "Me");
        assert!(preview_result.projects[0].has_history && preview_result.projects[0].has_draft);
        assert!(!preview_result.projects[0].registered);
        assert_eq!(preview_result.projects[0].local_files, 2);

        let new_logs = tempfile::TempDir::new().unwrap();
        let target = new_storage_root.path().join("restored-demo");
        let items = vec![ImportItem {
            id: project.id.clone(),
            target_path: target.to_string_lossy().to_string(),
        }];
        let (import_report, _) =
            apply(&package, &items, &mut fresh, &git_service, |id| Some(new_logs.path().join(id))).unwrap();
        assert!(import_report.errors.is_empty(), "{:?}", import_report.errors);
        assert_eq!(fs::read_to_string(target.join("a.txt")).unwrap(), "saved");
        assert_eq!(fs::read_to_string(target.join("wip.txt")).unwrap(), "unsaved");
        assert_eq!(fs::read_to_string(target.join(".env")).unwrap(), "KEY=secret");
        assert!(target.join("config/secret.json").exists());
        assert!(!target.join("node_modules").exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(target.join(".env")).unwrap().permissions().mode() & 0o777, 0o600);
        }
        assert!(new_logs.path().join(&project.id).join("project-hub.log").exists());
        let listed = fresh.list_projects();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, project.id);
        assert!(listed[0].is_git_repo);
    }

    #[test]
    fn moves_a_folder_without_history_and_keeps_existing_files() {
        let source_root = tempfile::TempDir::new().unwrap();
        let source = source_root.path().join("plain");
        fs::create_dir_all(source.join("src")).unwrap();
        fs::create_dir_all(source.join("node_modules/pkg")).unwrap();
        fs::write(source.join("src/main.ts"), "code").unwrap();
        fs::write(source.join(".env"), "KEY=old-computer").unwrap();
        fs::write(source.join("node_modules/pkg/index.js"), "x").unwrap();

        let storage_root = tempfile::TempDir::new().unwrap();
        let mut service = ProjectService::new(storage_root.path().join("projects.json").to_str().unwrap());
        let project = service.add_project(source.to_str().unwrap()).unwrap();
        assert!(!project.is_git_repo);
        let package = storage_root.path().join("backup.phub");
        let git_service = GitService::new();
        let report = export(&package, &service.list_projects(), "", "", &git_service, |_| None).unwrap();
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);

        // The new computer already has its own .env in the chosen folder.
        let new_root = tempfile::TempDir::new().unwrap();
        let target = new_root.path().join("plain");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join(".env"), "KEY=new-computer").unwrap();
        let mut fresh = ProjectService::new(new_root.path().join("projects.json").to_str().unwrap());
        let items = vec![ImportItem { id: project.id.clone(), target_path: target.to_string_lossy().to_string() }];
        let (import_report, _) = apply(&package, &items, &mut fresh, &git_service, |_| None).unwrap();
        assert!(import_report.errors.is_empty(), "{:?}", import_report.errors);
        assert_eq!(fs::read_to_string(target.join("src/main.ts")).unwrap(), "code");
        assert_eq!(fs::read_to_string(target.join(".env")).unwrap(), "KEY=new-computer");
        assert!(!target.join("node_modules").exists());
    }

    #[test]
    fn rejects_unrelated_files() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("random.phub");
        fs::write(&path, "not a zip").unwrap();
        let storage = dir.path().join("projects.json");
        let service = ProjectService::new(storage.to_str().unwrap());
        assert!(preview(&path, &service).is_err());
    }
}
