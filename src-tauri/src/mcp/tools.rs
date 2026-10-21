//! The tools agents can call: listing projects, status, diff, saving and history. A project is
//! named by id, by any path inside it, or by its name.

use super::McpHost;
use git_service::GitService;
use project_service::{same_path, Project};
use serde_json::{json, Value};

const DEFAULT_DIFF_BYTES: usize = 60_000;

pub(super) fn tools() -> Value {
    let project = json!({
        "type": "string",
        "description": "Project folder path (any path inside the project works), project name or id."
    });
    json!([
        {
            "name": "list_projects",
            "description": "List the projects managed by Project Hub.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "get_status",
            "description": "Show the current version, unsaved changes and recent save records of a project.",
            "inputSchema": {
                "type": "object",
                "properties": { "project": project },
                "required": ["project"]
            }
        },
        {
            "name": "get_diff",
            "description": "Return the patch of all unsaved changes (including new files) so you can describe them.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": project,
                    "max_bytes": { "type": "integer", "description": "Truncate the patch after this many bytes (default 60000)." }
                },
                "required": ["project"]
            }
        },
        {
            "name": "save_version",
            "description": "Save all current changes of the project as a new save record in the current version.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": project,
                    "title": { "type": "string", "description": "One-line summary of the change." },
                    "description": { "type": "string", "description": "Optional details: what changed and why." }
                },
                "required": ["project", "title"]
            }
        },
        {
            "name": "get_history",
            "description": "List recent save records of the current version.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": project,
                    "limit": { "type": "integer", "description": "Number of records (default 20, max 100)." }
                },
                "required": ["project"]
            }
        }
    ])
}

fn is_inside(path: &str, root: &str) -> bool {
    let normalize = |value: &str| {
        let value = value.replace('\\', "/").trim_end_matches('/').to_string();
        if cfg!(windows) {
            value.to_lowercase()
        } else {
            value
        }
    };
    let path = normalize(path);
    let root = normalize(root);
    path == root || path.starts_with(&format!("{root}/"))
}

fn resolve_project(host: &dyn McpHost, query: &str) -> Result<Project, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("Missing \"project\".".to_string());
    }
    let projects: Vec<Project> = host.projects().into_iter().filter(|p| !p.missing).collect();
    if let Some(project) = projects.iter().find(|p| p.id == query) {
        return Ok(project.clone());
    }
    if let Some(project) = projects.iter().find(|p| same_path(&p.path, query)) {
        return Ok(project.clone());
    }
    // Deepest project containing the path wins (handles nested projects).
    if let Some(project) = projects
        .iter()
        .filter(|p| is_inside(query, &p.path))
        .max_by_key(|p| p.path.len())
    {
        return Ok(project.clone());
    }
    let by_name: Vec<&Project> = projects
        .iter()
        .filter(|p| p.name.eq_ignore_ascii_case(query))
        .collect();
    if by_name.len() == 1 {
        return Ok(by_name[0].clone());
    }
    let names: Vec<String> = projects.iter().map(|p| format!("{} ({})", p.name, p.path)).collect();
    Err(format!(
        "No Project Hub project matches \"{}\". Add the folder in Project Hub first. Known projects: {}",
        query,
        if names.is_empty() { "none".to_string() } else { names.join("; ") }
    ))
}

fn str_arg<'a>(args: &'a Value, key: &str) -> &'a str {
    args.get(key).and_then(Value::as_str).unwrap_or_default()
}

pub(super) fn call_tool(host: &dyn McpHost, name: &str, args: &Value) -> Result<String, String> {
    let git = GitService::with_identity(host.identity());
    match name {
        "list_projects" => {
            let list: Vec<Value> = host
                .projects()
                .into_iter()
                .map(|p| json!({ "id": p.id, "name": p.name, "path": p.path, "missing": p.missing }))
                .collect();
            serde_json::to_string_pretty(&list).map_err(|e| e.to_string())
        }
        "get_status" => {
            let project = resolve_project(host, str_arg(args, "project"))?;
            let branch = git.get_current_branch(&project.path)?;
            let status = git.settled(&project.path, |git| git.get_status_summary(&project.path, 200))?;
            let draft = git.get_draft(&project.path)?;
            let history = git.get_log(&project.path, 5)?;
            serde_json::to_string_pretty(&json!({
                "project": project.name,
                "path": project.path,
                "version": branch,
                "unsaved_changes": status.files,
                "unsaved_total": status.total,
                "kept_unsaved_version": draft,
                "recent_records": history,
            }))
            .map_err(|e| e.to_string())
        }
        "get_diff" => {
            let project = resolve_project(host, str_arg(args, "project"))?;
            let max_bytes = args
                .get("max_bytes")
                .and_then(Value::as_u64)
                .map(|n| n.clamp(1_000, 2_000_000) as usize)
                .unwrap_or(DEFAULT_DIFF_BYTES);
            let diff = git.worktree_diff(&project.path, max_bytes)?;
            Ok(if diff.trim().is_empty() {
                "No unsaved changes.".to_string()
            } else {
                diff
            })
        }
        "save_version" => {
            let project = resolve_project(host, str_arg(args, "project"))?;
            let title = str_arg(args, "title").trim();
            if title.is_empty() {
                return Err("\"title\" is required.".to_string());
            }
            git.ensure_versioned(&project.path)?;
            if !git.has_unsaved_changes(&project.path)? {
                return Ok("Nothing to save: the project has no unsaved changes.".to_string());
            }
            let description = str_arg(args, "description").trim();
            let message = if description.is_empty() {
                title.to_string()
            } else {
                format!("{title}\n\n{description}")
            };
            let before = git.get_repo_state(&project.path)?.head;
            git.save_version(&project.path, &message)?;
            // E.g. an empty folder that could not be given its marker: nothing was recorded.
            if git.get_repo_state(&project.path)?.head == before {
                return Ok("Nothing to save: the unsaved changes could not be recorded.".to_string());
            }
            host.on_saved(&project, title);
            let head = git.get_log(&project.path, 1)?.into_iter().next();
            Ok(match head {
                Some(record) => format!(
                    "Saved \"{}\" to version {} ({}).",
                    record.message,
                    git.get_current_branch(&project.path).unwrap_or_default(),
                    &record.hash[..record.hash.len().min(7)]
                ),
                None => "Saved.".to_string(),
            })
        }
        "get_history" => {
            let project = resolve_project(host, str_arg(args, "project"))?;
            let limit = args
                .get("limit")
                .and_then(Value::as_u64)
                .map(|n| n.clamp(1, 100) as usize)
                .unwrap_or(20);
            let history = git.get_log(&project.path, limit)?;
            serde_json::to_string_pretty(&history).map_err(|e| e.to_string())
        }
        other => Err(format!("UNKNOWN_TOOL:{other}")),
    }
}

#[cfg(test)]
mod tests {
    use crate::mcp::test_support::{call, host_with_repo};
    use serde_json::json;

    #[test]
    fn save_version_through_subfolder_path() {
        let (host, dir) = host_with_repo();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src").join("b.txt"), "two").unwrap();
        let sub = dir.path().join("src").to_string_lossy().to_string();

        let diff = call(&host, "get_diff", json!({ "project": sub }));
        assert!(diff["result"]["content"][0]["text"].as_str().unwrap().contains("b.txt"));

        let saved = call(&host, "save_version", json!({ "project": sub, "title": "Add b", "description": "Because." }));
        assert_eq!(saved["result"]["isError"], false, "{}", saved);
        assert_eq!(host.saved.lock().unwrap().as_slice(), ["Add b"]);

        let history = call(&host, "get_history", json!({ "project": "Demo", "limit": 1 }));
        let text = history["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Add b") && text.contains("Because.") && text.contains("Agent"));

        let again = call(&host, "save_version", json!({ "project": "Demo", "title": "noop" }));
        assert!(again["result"]["content"][0]["text"].as_str().unwrap().starts_with("Nothing to save"));
    }

    #[test]
    fn unknown_project_is_tool_error() {
        let (host, _dir) = host_with_repo();
        let response = call(&host, "get_status", json!({ "project": "C:/nowhere" }));
        assert_eq!(response["result"]["isError"], true);
    }
}
