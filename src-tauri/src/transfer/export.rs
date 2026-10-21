//! Writing a package: every project's history as a bundle, its local files and its logs, then the
//! manifest. The package only replaces the target file once it is complete.

use super::local_files;
use super::package::{scratch_dir, zip_error};
use super::{ExportManifest, ExportReport, ExportedProject, ProjectIssue, FORMAT, FORMAT_VERSION, MANIFEST};
use chrono::Utc;
use git_service::GitService;
use project_service::Project;
use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

pub fn export(
    out_path: &Path,
    projects: &[Project],
    author_name: &str,
    author_email: &str,
    git: &GitService,
    log_dir_for: impl Fn(&str) -> Option<PathBuf>,
) -> Result<ExportReport, String> {
    let scratch = scratch_dir("export")?;
    let partial_path = out_path.with_extension("partial");
    let result = (|| {
        let file = File::create(&partial_path).map_err(|e| format!("EXPORT_CREATE_FAILED:{e}"))?;
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
                local_files: Vec::new(),
                logs: Vec::new(),
            };

            if project.missing {
                warnings.push(ProjectIssue::new(&project.name, "EXPORT_FOLDER_MISSING"));
            } else if project.is_git_repo {
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
                    Err(error) => warnings.push(ProjectIssue::new(&project.name, error)),
                }
                entry.remote_url = git.get_remote_url(&project.path, "origin");
            }

            if !project.missing {
                let root = Path::new(&project.path);
                let carried = local_files::collect(root, project.is_git_repo, git)
                    .and_then(|files| local_files::write(&mut zip, &project.id, root, &files).map(|()| files));
                match carried {
                    Ok(files) => entry.local_files = files,
                    Err(error) => warnings.push(ProjectIssue::new(&project.name, error)),
                }
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
