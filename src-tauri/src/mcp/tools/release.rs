//! Tools that publish versions on GitHub Releases through the account signed in to Project Hub
//! (see `crate::release`), so agents never need their own GitHub access. The agent builds; these
//! tools hand its files to GitHub, or start the project's cloud build, and report progress.

use super::{agent_error, resolve_project, str_arg, str_list};
use crate::github::{self, releases::RepoRef};
use crate::mcp::McpHost;
use crate::release::{facts, ReleaseTarget, NOTES_RECORDS};
use git_service::GitService;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub(super) fn definitions(project: &Value) -> Vec<Value> {
    vec![
        json!({
            "name": "get_release_info",
            "description": "Everything needed before publishing a release: the GitHub repository, the latest release, a suggested next version, the save records since the latest release (to write the notes), the version numbers in the project's files, and whether the project has a cloud build (a GitHub Actions workflow started by version tags).",
            "inputSchema": { "type": "object", "properties": { "project": project }, "required": ["project"] },
            "annotations": { "title": "Release info", "readOnlyHint": true, "openWorldHint": true }
        }),
        json!({
            "name": "publish_release",
            "description": "Publish a version on GitHub Releases with the account signed in to Project Hub. Save and build first. Give files (installers or archives you built on this computer) and/or cloud_build: true (uploads the version tag, which starts the project's release workflow on GitHub to build the other platforms). Files go to a draft that becomes public only once all are uploaded. Calling again with the same version and files attaches them to that release (e.g. after a cloud build created it).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": project,
                    "version": { "type": "string", "description": "Version number, e.g. 1.2.0 (the tag becomes v1.2.0)." },
                    "title": { "type": "string", "description": "Release title (default: the tag)." },
                    "notes": { "type": "string", "description": "Release notes in Markdown (default: the titles of the save records since the last release)." },
                    "files": { "type": "array", "items": { "type": "string" }, "description": "Absolute paths of files to attach." },
                    "cloud_build": { "type": "boolean", "description": "Upload the version tag to start the project's release workflow on GitHub." },
                    "prerelease": { "type": "boolean", "description": "Mark as a pre-release (not offered as the latest version)." }
                },
                "required": ["project", "version"]
            },
            // Public and visible to everyone who can see the repository.
            "annotations": { "title": "Publish release", "readOnlyHint": false, "destructiveHint": false, "idempotentHint": true, "openWorldHint": true }
        }),
        json!({
            "name": "get_release_status",
            "description": "How a release is doing: draft or published, the files attached, and the cloud build runs its version tag started (each platform's job and result).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": project,
                    "version": { "type": "string", "description": "Version to look at (default: the newest release)." }
                },
                "required": ["project"]
            },
            "annotations": { "title": "Release status", "readOnlyHint": true, "openWorldHint": true }
        }),
    ]
}

pub(super) fn call(host: &dyn McpHost, name: &str, args: &Value) -> Option<Result<String, String>> {
    let result = match name {
        "get_release_info" => release_info(host, args),
        "publish_release" => publish(host, args),
        "get_release_status" => release_status(host, args),
        _ => return None,
    };
    Some(result.map_err(agent_error))
}

fn pretty(value: Value) -> Result<String, String> {
    serde_json::to_string_pretty(&value).map_err(|e| e.to_string())
}

/// Works without a repository or a sign-in too: then it says what is missing.
fn release_info(host: &dyn McpHost, args: &Value) -> Result<String, String> {
    let project = resolve_project(host, str_arg(args, "project"))?;
    let git = GitService::with_identity(host.identity());
    let path = project.path.as_str();
    let root = Path::new(path);
    let url = git.get_remote_url(path, "origin");
    let target = ReleaseTarget::open(&git, path).ok();
    let account = crate::settings::load_settings().github.map(|account| account.login);

    let (latest, drafts, records) = match &target {
        Some(target) => {
            let all = github::releases::list_releases(&target.token, &target.repo)?;
            let latest = all.iter().find(|release| !release.draft).cloned();
            let drafts: Vec<String> = all.iter().filter(|release| release.draft).map(|release| release.tag_name.clone()).collect();
            let records = target.records_since(latest.as_ref().map(|release| release.tag_name.as_str()))?;
            (latest, drafts, records)
        }
        None => (None, Vec::new(), git.records_since(path, None, NOTES_RECORDS)?),
    };
    let in_files = facts::versions_in_files(root);
    let unsaved = git.get_status_summary(path, 1).map(|summary| summary.total).unwrap_or(0);

    let mut blockers = Vec::new();
    if account.is_none() || github::load_token().is_none() {
        blockers.push("Project Hub is not signed in to GitHub: ask the user to sign in under Settings → GitHub account.".to_string());
    }
    match &url {
        None => blockers.push("The project has no GitHub repository: upload_to_cloud with new_repository creates one.".to_string()),
        Some(url) if RepoRef::from_url(url).is_none() => blockers.push(format!("The project's repository ({url}) is not on GitHub.")),
        Some(_) => {}
    }
    if unsaved > 0 {
        blockers.push(format!("{unsaved} unsaved change(s): save_version before publishing (a release is made from the last saved record)."));
    }

    pretty(json!({
        "project": project.name,
        "repository": url.as_deref().and_then(RepoRef::from_url).map(|repo| repo.full_name()),
        "signed_in_as": account,
        "latest_release": latest.as_ref().map(|release| json!({
            "version": release.tag_name,
            "url": release.html_url,
            "published_at": release.published_at,
            "files": release.assets.iter().map(|asset| asset.name.clone()).collect::<Vec<_>>(),
        })),
        "drafts": drafts,
        "suggested_version": facts::suggest_version(
            latest.as_ref().map(|release| release.tag_name.as_str()),
            in_files.first().map(|(_, version)| version.as_str()),
        ),
        "versions_in_files": in_files.iter().map(|(file, version)| json!({ "file": file, "version": version })).collect::<Vec<_>>(),
        "records_since_latest_release": records,
        "cloud_build_workflows": facts::tag_workflows(root),
        "records_not_uploaded": git.sync_state(path).map(|state| state.ahead).unwrap_or(0),
        "blockers": blockers,
        "how_to_publish": "Update the version in the project's files if it has them and save_version. Then either build here and publish_release with files (this platform only), or publish_release with cloud_build: true when cloud_build_workflows is not empty (to add one, write a GitHub Actions workflow started by pushing tags like v1.2.0 that builds each platform and creates the release, then save and upload it).",
    }))
}

fn publish(host: &dyn McpHost, args: &Value) -> Result<String, String> {
    let project = resolve_project(host, str_arg(args, "project"))?;
    let git = GitService::with_identity(host.identity());
    let target = ReleaseTarget::open(&git, &project.path)?;
    let tag = facts::tag_for(str_arg(args, "version"))?;
    let files: Vec<PathBuf> = str_list(args, "files").into_iter().map(PathBuf::from).collect();
    let cloud_build = args.get("cloud_build").and_then(Value::as_bool).unwrap_or(false);
    if files.is_empty() && !cloud_build {
        return Err("Nothing to publish: give files built on this computer and/or cloud_build: true.".to_string());
    }
    if let Some(missing) = files.iter().find(|file| !file.is_file()) {
        return Err(format!("Not a file: {}", missing.display()));
    }
    if git.has_unsaved_changes(&project.path)? {
        return Err("The project has unsaved changes. A release is made from the last saved record: save_version first.".to_string());
    }

    if cloud_build {
        let workflows = target.start_cloud_build(&tag).map_err(|error| {
            if error.starts_with("The project has no cloud build") {
                format!("{error} Write one (it builds each platform and creates the release for the tag), save_version and upload_to_cloud, then try again.")
            } else {
                error
            }
        })?;
        host.on_changed(&project, "release.cloud_build", &tag);
        let attach_later = if files.is_empty() {
            String::new()
        } else {
            format!(" The {} file(s) were not attached yet: the workflow creates the release. Once get_release_status shows it, call publish_release again with version {tag} and the files.", files.len())
        };
        return Ok(format!(
            "Uploaded tag {tag}, which starts {} on GitHub. Follow it with get_release_status (version {tag}).{attach_later}",
            workflows.join(", ")
        ));
    }

    let title = match str_arg(args, "title").trim() {
        "" => tag.clone(),
        title => title.to_string(),
    };
    let notes = match str_arg(args, "notes").trim() {
        "" => target.default_notes()?,
        notes => notes.to_string(),
    };
    let prerelease = args.get("prerelease").and_then(Value::as_bool).unwrap_or(false);
    let release = target.publish_files(&tag, &title, &notes, prerelease, &files)?;
    host.on_changed(&project, "release.publish", &format!("{tag} ({} file(s))", files.len()));
    let uploaded: Vec<&str> = files.iter().filter_map(|file| file.file_name()?.to_str()).collect();
    pretty(json!({
        "published": tag,
        "url": release.html_url,
        "uploaded": release.assets.iter().filter(|asset| uploaded.contains(&asset.name.as_str())).map(|asset| json!({ "name": asset.name, "size": asset.size })).collect::<Vec<_>>(),
        "all_files": release.assets.iter().map(|asset| asset.name.clone()).collect::<Vec<_>>(),
    }))
}

fn release_status(host: &dyn McpHost, args: &Value) -> Result<String, String> {
    let project = resolve_project(host, str_arg(args, "project"))?;
    let git = GitService::with_identity(host.identity());
    let target = ReleaseTarget::open(&git, &project.path)?;
    let tag = match str_arg(args, "version").trim() {
        "" => None,
        version => Some(facts::tag_for(version)?),
    };
    let status = target.status(tag.as_deref())?;
    pretty(json!({
        "version": status.tag,
        "release": status.release.as_ref().map(|release| json!({
            "state": status.state(),
            "url": release.html_url,
            "files": release.assets.iter().map(|asset| json!({ "name": asset.name, "size": asset.size })).collect::<Vec<_>>(),
        })),
        "cloud_build": status.run.as_ref().map(|run| json!({
            "workflow": run.name,
            "status": run.status,
            "conclusion": run.conclusion,
            "url": run.html_url,
            "jobs": status.jobs.iter().map(|job| json!({
                "name": job.name,
                "status": job.status,
                "conclusion": job.conclusion,
            })).collect::<Vec<_>>(),
        })),
        "summary": status.summary(),
    }))
}
