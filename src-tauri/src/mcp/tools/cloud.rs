//! Tools that upload a project to its GitHub repository and download from it, with the app's rules:
//! nothing is overwritten unless the agent says so explicitly (after asking the user), and when
//! both sides have new records the agent gets the choice instead of a guess.

use super::{agent_error, resolve_project, str_arg};
use crate::{github, mcp::McpHost};
use git_service::{GitService, PUSH_REJECTED};
use serde_json::{json, Value};

pub(super) fn definitions(project: &Value) -> Vec<Value> {
    vec![
        json!({
            "name": "upload_to_cloud",
            "description": "Upload the project's saved records (current version) to its GitHub repository, using the GitHub account signed in to Project Hub. Unsaved changes are not uploaded; call save_version first. A project without a repository needs new_repository (ask the user for the name and whether it should be private).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": project,
                    "new_repository": { "type": "string", "description": "Name for a new GitHub repository, only when the project has none yet." },
                    "private": { "type": "boolean", "description": "Whether the new repository is private (default true)." },
                    "description": { "type": "string", "description": "Description of the new repository." },
                    "overwrite_cloud": { "type": "boolean", "description": "Replace records the cloud has and this computer does not. Only after the user agreed." }
                },
                "required": ["project"]
            },
            // May replace the cloud's records, but only with overwrite_cloud.
            "annotations": { "title": "Upload to GitHub", "readOnlyHint": false, "destructiveHint": true, "idempotentHint": true, "openWorldHint": true }
        }),
        json!({
            "name": "download_from_cloud",
            "description": "Download new save records of the current version from the project's GitHub repository. When this computer and the cloud both have new records nothing is downloaded: ask the user which side to keep and call again with keep.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": project,
                    "keep": {
                        "type": "string",
                        "enum": ["cloud", "local"],
                        "description": "Only when both sides have new records. \"cloud\": switch to the cloud's state; this computer's own records and unsaved changes are kept as a new version, nothing is lost. \"local\": keep this computer's records (then upload_to_cloud with overwrite_cloud to make the cloud match)."
                    }
                },
                "required": ["project"]
            },
            // Changes the files on disk, but keeps everything it replaces as a version.
            "annotations": { "title": "Download from GitHub", "readOnlyHint": false, "destructiveHint": false, "idempotentHint": true, "openWorldHint": true }
        }),
    ]
}

pub(super) fn call(host: &dyn McpHost, name: &str, args: &Value) -> Option<Result<String, String>> {
    let result = match name {
        "upload_to_cloud" => upload(host, args),
        "download_from_cloud" => download(host, args),
        _ => return None,
    };
    Some(result.map_err(agent_error))
}

fn upload(host: &dyn McpHost, args: &Value) -> Result<String, String> {
    let project = resolve_project(host, str_arg(args, "project"))?;
    let git = GitService::with_identity(host.identity());
    let path = project.path.as_str();
    let branch = git.get_current_branch(path)?;
    let unsaved_note = if git.has_unsaved_changes(path)? {
        " Unsaved changes stay on this computer; call save_version first if they belong in the upload."
    } else {
        ""
    };

    if git.get_remote_url(path, "origin").is_none() {
        let name = str_arg(args, "new_repository").trim();
        if name.is_empty() {
            return Err("This project has no GitHub repository yet. Ask the user for a repository name (and whether it should be private), then call again with new_repository.".to_string());
        }
        let token = github::load_token().ok_or_else(|| "GITHUB_NOT_SIGNED_IN".to_string())?;
        let private = args.get("private").and_then(Value::as_bool).unwrap_or(true);
        let repo = github::create_repo(&token, &github::sanitize_repo_name(name), str_arg(args, "description").trim(), private)?;
        git.set_remote_url(path, "origin", &repo.clone_url)?;
        git.push(path, Some(&token), false)?;
        host.on_changed(&project, "cloud.publish", &repo.full_name);
        return Ok(format!(
            "Created the {} repository {} and uploaded version {branch}.{unsaved_note}",
            if repo.private { "private" } else { "public" },
            repo.html_url
        ));
    }

    let overwrite = args.get("overwrite_cloud").and_then(Value::as_bool).unwrap_or(false);
    let token = github::token_for_remote(git.get_remote_url(path, "origin").as_deref(), true)?;
    let pending = git.sync_state(path).map(|state| state.ahead).unwrap_or(0);
    match git.push(path, token.as_deref(), overwrite) {
        Ok(()) => {
            host.on_changed(&project, "cloud.upload", &format!("{branch}{}", if overwrite { " (overwrote the cloud)" } else { "" }));
            Ok(if pending == 0 && !overwrite {
                format!("The cloud already has every saved record of version {branch}.{unsaved_note}")
            } else {
                format!("Uploaded version {branch}{}.{unsaved_note}", if overwrite { ", replacing the cloud's records" } else { "" })
            })
        }
        Err(error) if error.starts_with(PUSH_REJECTED) => Err("The cloud has records this computer does not (uploaded from another computer, or this computer went back to an older record). To keep both, call download_from_cloud first. To replace the cloud's records with this computer's, ask the user, then call again with overwrite_cloud: true.".to_string()),
        Err(error) => Err(error),
    }
}

fn download(host: &dyn McpHost, args: &Value) -> Result<String, String> {
    let project = resolve_project(host, str_arg(args, "project"))?;
    let git = GitService::with_identity(host.identity());
    let path = project.path.as_str();
    let branch = git.get_current_branch(path)?;
    let token = github::token_for_remote(git.get_remote_url(path, "origin").as_deref(), false)?;
    if !git.fetch(path, token.as_deref())? {
        return Ok(format!("The cloud does not have version {branch} yet; upload_to_cloud first."));
    }
    let state = git.sync_state(path)?;
    if state.behind == 0 {
        return Ok(match state.ahead {
            0 => "Already up to date with the cloud.".to_string(),
            ahead => format!("Nothing new in the cloud. This computer has {ahead} record(s) not uploaded yet."),
        });
    }
    if state.ahead == 0 {
        let outcome = git.pull(path, token.as_deref())?;
        host.on_changed(&project, "cloud.download", &format!("{} record(s)", outcome.received));
        return Ok(format!("Downloaded {} save record(s) of version {branch}.", outcome.received));
    }
    match str_arg(args, "keep") {
        "cloud" => {
            let kept_as = git.use_cloud_version(path, token.as_deref())?;
            host.on_changed(&project, "cloud.use_cloud", &kept_as);
            Ok(format!("Switched to the cloud's latest state of version {branch}. This computer's own records and unsaved changes were kept as version \"{kept_as}\"."))
        }
        "local" => Ok("Kept this computer's records; nothing was downloaded. To make the cloud match, call upload_to_cloud with overwrite_cloud: true once the user agreed.".to_string()),
        _ => Ok(json!({
            "downloaded": false,
            "situation": format!(
                "Version {branch}: this computer has {} record(s) the cloud does not, and the cloud has {} this computer does not.",
                state.ahead, state.behind
            ),
            "next": "Ask the user which to keep, then call download_from_cloud again with keep: \"cloud\" (take the cloud's state; this computer's records are kept as a new version) or keep: \"local\" (keep this computer's; then upload_to_cloud with overwrite_cloud: true).",
        })
        .to_string()),
    }
}

#[cfg(test)]
mod tests {
    use crate::mcp::test_support::{call, host_with_repo};
    use serde_json::json;
    use std::process::Command;

    fn git(dir: &std::path::Path, args: &[&str]) {
        let output = Command::new("git").args(args).current_dir(dir).output().unwrap();
        assert!(output.status.success(), "git {:?}: {}", args, String::from_utf8_lossy(&output.stderr));
    }

    fn text(response: &serde_json::Value) -> String {
        response["result"]["content"][0]["text"].as_str().unwrap_or_default().to_string()
    }

    #[test]
    fn a_project_without_a_repository_asks_for_a_name() {
        let (host, _dir) = host_with_repo();
        let response = call(&host, "upload_to_cloud", json!({ "project": "Demo" }));
        assert_eq!(response["result"]["isError"], true);
        assert!(text(&response).contains("new_repository"));
    }

    #[test]
    fn uploads_and_downloads_through_a_linked_repository() {
        let (host, dir) = host_with_repo();
        let cloud = tempfile::TempDir::new().unwrap();
        git(cloud.path(), &["init", "-q", "--bare"]);
        git(dir.path(), &["remote", "add", "origin", cloud.path().to_str().unwrap()]);

        let uploaded = call(&host, "upload_to_cloud", json!({ "project": "Demo" }));
        assert_eq!(uploaded["result"]["isError"], false, "{uploaded}");
        assert!(text(&uploaded).starts_with("Uploaded version"));

        let downloaded = call(&host, "download_from_cloud", json!({ "project": "Demo" }));
        assert_eq!(text(&downloaded), "Already up to date with the cloud.");
        let actions: Vec<String> = host.changes.lock().unwrap().iter().map(|(action, _)| action.clone()).collect();
        assert_eq!(actions, ["cloud.upload"]);
    }
}
