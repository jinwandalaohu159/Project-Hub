//! Reading a package back: the preview shown before importing, and restoring the chosen projects
//! into their target folders and the project list.

use super::local_files;
use super::package::{extract_entry, open_package, scratch_dir};
use super::{
    target_state, ExportManifest, ExportedProject, ImportItem, ImportPreview, ImportPreviewItem, ImportReport,
    ProjectIssue, TargetState,
};
use git_service::GitService;
use project_service::{same_path, ProjectService};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};
use zip::ZipArchive;

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
                local_files: project.local_files.len(),
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
        (None, TargetState::Missing) if project.local_files.is_empty() => {
            return Err("EXPORT_NO_FILES".to_string());
        }
        (None, _) => fs::create_dir_all(target).map_err(|e| e.to_string())?,
    }
    local_files::restore(archive, &project.id, Path::new(target), &project.local_files)?;

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
            report.errors.push(ProjectIssue::new(&project.name, "IMPORT_NO_TARGET"));
            continue;
        }
        match import_one(&mut archive, project, target, &scratch, service, git, &log_dir_for) {
            Ok(()) => report.imported.push(project.name.clone()),
            Err(error) => report.errors.push(ProjectIssue::new(&project.name, error)),
        }
    }

    let _ = fs::remove_dir_all(&scratch);
    Ok((report, manifest))
}
