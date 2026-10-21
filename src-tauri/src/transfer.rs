//! Export / import of everything Project Hub knows, for moving to another computer.
//! The package is a zip: `manifest.json`, one git bundle per project (all versions, save records and
//! unsaved changes) and the per-project operation logs.

use chrono::{DateTime, Utc};
use git_service::GitService;
use project_service::{same_path, Project, ProjectService};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

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
    #[serde(default)]
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportReport {
    pub exported: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreviewItem {
    pub id: String,
    pub name: String,
    pub original_path: String,
    pub suggested_path: String,
    pub has_history: bool,
    pub has_draft: bool,
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
    pub errors: Vec<String>,
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

fn scratch_dir(kind: &str) -> Result<PathBuf, String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let dir = std::env::temp_dir().join(format!("project-hub-{kind}-{nanos}"));
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn zip_error(error: zip::result::ZipError) -> String {
    format!("Package error: {error}")
}

pub fn export(
    out_path: &Path,
    projects: &[Project],
    include_history: bool,
    author_name: &str,
    author_email: &str,
    git: &GitService,
    log_dir_for: impl Fn(&str) -> Option<PathBuf>,
) -> Result<ExportReport, String> {
    let scratch = scratch_dir("export")?;
    let partial_path = out_path.with_extension("partial");
    let result = (|| {
        let file = File::create(&partial_path).map_err(|e| format!("Cannot create the export file: {e}"))?;
        let mut zip = ZipWriter::new(file);
        let stored = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .large_file(true);
        let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let mut warnings = Vec::new();
        let mut exported = Vec::new();

        for project in projects {
            let mut entry = ExportedProject {
                id: project.id.clone(),
                name: project.name.clone(),
                original_path: project.path.clone(),
                head_branch: None,
                remote_url: None,
                bundle: None,
                has_draft: false,
                logs: Vec::new(),
            };

            if project.missing {
                warnings.push(format!("{}: folder not found, only the project entry was exported.", project.name));
            } else if include_history && project.is_git_repo {
                let bundle_path = scratch.join(format!("{}.bundle", project.id));
                match git.create_bundle(&project.path, &bundle_path) {
                    Ok(meta) => {
                        let name = format!("bundles/{}.bundle", project.id);
                        zip.start_file(name.as_str(), stored).map_err(zip_error)?;
                        let mut bundle = File::open(&bundle_path).map_err(|e| e.to_string())?;
                        io::copy(&mut bundle, &mut zip).map_err(|e| e.to_string())?;
                        drop(bundle);
                        let _ = fs::remove_file(&bundle_path);
                        entry.head_branch = Some(meta.head_branch);
                        entry.has_draft = meta.has_draft;
                        entry.bundle = Some(name);
                    }
                    Err(error) => warnings.push(format!("{}: {}", project.name, error)),
                }
                entry.remote_url = git.get_remote_url(&project.path, "origin");
            }

            if let Some(log_dir) = log_dir_for(&project.id).filter(|dir| dir.is_dir()) {
                for log in fs::read_dir(&log_dir).map_err(|e| e.to_string())?.flatten() {
                    let path = log.path();
                    if !path.is_file() {
                        continue;
                    }
                    let file_name = log.file_name().to_string_lossy().to_string();
                    let name = format!("logs/{}/{}", project.id, file_name);
                    zip.start_file(name.as_str(), deflated).map_err(zip_error)?;
                    let mut source = File::open(&path).map_err(|e| e.to_string())?;
                    io::copy(&mut source, &mut zip).map_err(|e| e.to_string())?;
                    entry.logs.push(name);
                }
            }
            exported.push(entry);
        }

        let manifest = ExportManifest {
            format: FORMAT.to_string(),
            version: FORMAT_VERSION,
            exported_at: Utc::now(),
            author_name: author_name.to_string(),
            author_email: author_email.to_string(),
            projects: exported,
        };
        zip.start_file(MANIFEST, deflated).map_err(zip_error)?;
        serde_json::to_writer_pretty(&mut zip, &manifest).map_err(|e| e.to_string())?;
        zip.finish().map_err(zip_error)?;
        Ok(ExportReport {
            exported: manifest.projects.len(),
            warnings,
        })
    })();

    let _ = fs::remove_dir_all(&scratch);
    match result {
        Ok(report) => {
            let _ = fs::remove_file(out_path);
            fs::rename(&partial_path, out_path).map_err(|e| e.to_string())?;
            Ok(report)
        }
        Err(error) => {
            let _ = fs::remove_file(&partial_path);
            Err(error)
        }
    }
}

/// Ids become file and folder names; reject anything that could escape the scratch folder.
fn is_safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

fn open_package(path: &Path) -> Result<(ZipArchive<File>, ExportManifest), String> {
    let file = File::open(path).map_err(|e| format!("Cannot open the package: {e}"))?;
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

pub fn preview(path: &Path, service: &ProjectService) -> Result<ImportPreview, String> {
    let (_, manifest) = open_package(path)?;
    let known = service.list_projects();
    let projects = manifest
        .projects
        .iter()
        .map(|project| {
            let original = Path::new(&project.original_path);
            let usable = original.is_dir() || original.parent().is_some_and(Path::is_dir);
            ImportPreviewItem {
                id: project.id.clone(),
                name: project.name.clone(),
                original_path: project.original_path.clone(),
                suggested_path: if usable { project.original_path.clone() } else { String::new() },
                has_history: project.bundle.is_some(),
                has_draft: project.has_draft,
                registered: known
                    .iter()
                    .any(|p| (p.id == project.id || same_path(&p.path, &project.original_path)) && !p.missing),
            }
        })
        .collect();
    Ok(ImportPreview {
        exported_at: manifest.exported_at,
        author_name: manifest.author_name,
        author_email: manifest.author_email,
        projects,
    })
}

fn extract_entry(archive: &mut ZipArchive<File>, name: &str, dest: &Path) -> Result<(), String> {
    let mut entry = archive.by_name(name).map_err(zip_error)?;
    let mut out = File::create(dest).map_err(|e| e.to_string())?;
    io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
    Ok(())
}

fn register(service: &mut ProjectService, id: &str, target: &str) -> Result<(), String> {
    if service.list_projects().iter().any(|p| p.id == id) {
        service.relocate_project(id, target)?;
    } else if !service.list_projects().iter().any(|p| same_path(&p.path, target)) {
        service.add_project_with_id(target, Some(id))?;
    }
    Ok(())
}

fn import_one(
    archive: &mut ZipArchive<File>,
    project: &ExportedProject,
    target: &str,
    scratch: &Path,
    service: &mut ProjectService,
    git: &GitService,
    log_dir_for: &dyn Fn(&str) -> Option<PathBuf>,
) -> Result<(), String> {
    let state = target_state(target);
    match (&project.bundle, state) {
        (_, TargetState::Repo) => {}
        (Some(bundle_name), _) => {
            let bundle_path = scratch.join(format!("{}.bundle", project.id));
            extract_entry(archive, bundle_name, &bundle_path)?;
            let head = project.head_branch.as_deref().unwrap_or("master");
            git.restore_bundle(Path::new(target), &bundle_path, head, project.remote_url.as_deref())?;
            let _ = fs::remove_file(&bundle_path);
        }
        (None, TargetState::Missing) => {
            return Err("EXPORT_NO_FILES".to_string());
        }
        (None, _) => {}
    }

    // Exported without history: start recording from the files already in the folder.
    git.ensure_versioned(target)?;
    register(service, &project.id, target)?;

    if let Some(log_dir) = log_dir_for(&project.id) {
        for name in &project.logs {
            let Some(file_name) = Path::new(name).file_name() else { continue };
            let dest = log_dir.join(file_name);
            if dest.exists() {
                continue;
            }
            fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;
            extract_entry(archive, name, &dest)?;
        }
    }
    Ok(())
}

pub fn apply(
    path: &Path,
    items: &[ImportItem],
    service: &mut ProjectService,
    git: &GitService,
    log_dir_for: impl Fn(&str) -> Option<PathBuf>,
) -> Result<(ImportReport, ExportManifest), String> {
    let (mut archive, manifest) = open_package(path)?;
    let scratch = scratch_dir("import")?;
    let mut report = ImportReport {
        imported: Vec::new(),
        errors: Vec::new(),
    };

    for item in items {
        let Some(project) = manifest.projects.iter().find(|p| p.id == item.id) else {
            continue;
        };
        let target = item.target_path.trim();
        if target.is_empty() {
            report.errors.push(format!("{}: no target folder chosen.", project.name));
            continue;
        }
        match import_one(&mut archive, project, target, &scratch, service, git, &log_dir_for) {
            Ok(()) => report.imported.push(project.name.clone()),
            Err(error) => report.errors.push(format!("{}: {}", project.name, error)),
        }
    }

    let _ = fs::remove_dir_all(&scratch);
    Ok((report, manifest))
}

#[cfg(test)]
mod tests {
    use super::*;
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
        git(&source, &["add", "."]);
        git(&source, &["commit", "-m", "first"]);
        fs::write(source.join("wip.txt"), "unsaved").unwrap();

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
        let report = export(&package, &service.list_projects(), true, "Me", "me@x.com", &git_service, logs_for).unwrap();
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
        assert!(new_logs.path().join(&project.id).join("project-hub.log").exists());
        let listed = fresh.list_projects();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, project.id);
        assert!(listed[0].is_git_repo);
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
