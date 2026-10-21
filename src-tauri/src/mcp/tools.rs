//! The tools agents can call: listing projects, status, diff, saving, history and the rules that
//! hide files from versions. A project is
//! named by id, by any path inside it, or by its name. Each tool says whether it only reads
//! (`annotations.readOnlyHint`); the settings page lists the same definitions and may try out the
//! read-only ones.

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
            "inputSchema": { "type": "object", "properties": {} },
            "annotations": read_only("List projects")
        },
        {
            "name": "get_status",
            "description": "Show the current version, unsaved changes and recent save records of a project.",
            "inputSchema": {
                "type": "object",
                "properties": { "project": project },
                "required": ["project"]
            },
            "annotations": read_only("Project status")
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
            },
            "annotations": read_only("Unsaved changes")
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
            },
            // Adds a record; nothing existing is changed or lost.
            "annotations": {
                "title": "Save version",
                "readOnlyHint": false,
                "destructiveHint": false,
                "idempotentHint": false,
                "openWorldHint": false
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
            },
            "annotations": read_only("Save history")
        },
        {
            "name": "get_ignore_rules",
            "description": "Show which files are hidden from versions: the project's .gitignore rules, Project Hub's default rules and what is hidden right now.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": project,
                    "limit": { "type": "integer", "description": "Hidden paths to list (default 100, max 1000)." }
                },
                "required": ["project"]
            },
            "annotations": read_only("Hidden files")
        },
        {
            "name": "update_ignore_rules",
            "description": "Add or remove .gitignore rules (gitignore syntax, e.g. \"node_modules/\", \"*.log\", \"/build/\"). Files that earlier records contain and the new rules hide stop being saved but stay on disk. Call save_version afterwards to record the change.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": project,
                    "add": { "type": "array", "items": { "type": "string" }, "description": "Rules to add." },
                    "remove": { "type": "array", "items": { "type": "string" }, "description": "Rules to remove, written exactly as they appear." }
                },
                "required": ["project"]
            },
            // Changes .gitignore; files stay on disk and the records keep them.
            "annotations": {
                "title": "Change hidden files",
                "readOnlyHint": false,
                "destructiveHint": false,
                "idempotentHint": true,
                "openWorldHint": false
            }
        }
    ])
}

fn read_only(title: &str) -> Value {
    json!({ "title": title, "readOnlyHint": true, "openWorldHint": false })
}

fn is_read_only(name: &str) -> bool {
    tools()
        .as_array()
        .into_iter()
        .flatten()
        .find(|tool| tool["name"] == name)
        .is_some_and(|tool| tool["annotations"]["readOnlyHint"] == true)
}

/// Runs a tool for the settings page's "try it". Only read-only tools: trying must never change
/// a project.
pub(super) fn preview_tool(host: &dyn McpHost, name: &str, args: &Value) -> Result<String, String> {
    if !is_read_only(name) {
        return Err(format!("MCP_TOOL_NOT_PREVIEWABLE:{name}"));
    }
    call_tool(host, name, args)
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

fn str_list(args: &Value, key: &str) -> Vec<String> {
    args.get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default()
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
        "get_ignore_rules" => {
            let project = resolve_project(host, str_arg(args, "project"))?;
            let limit = args
                .get("limit")
                .and_then(Value::as_u64)
                .map(|n| n.clamp(1, 1000) as usize)
                .unwrap_or(100);
            let rules = git.ignore_rules(&project.path, limit)?;
            serde_json::to_string_pretty(&json!({
                "project": project.name,
                "gitignore_rules": rules.rules,
                "project_hub_defaults": rules.defaults,
                "hidden_now": rules.hidden,
                "hidden_total": rules.hidden_total,
            }))
            .map_err(|e| e.to_string())
        }
        "update_ignore_rules" => {
            let project = resolve_project(host, str_arg(args, "project"))?;
            let (add, remove) = (str_list(args, "add"), str_list(args, "remove"));
            if add.is_empty() && remove.is_empty() {
                return Err("Give \"add\" and/or \"remove\".".to_string());
            }
            let update = git.update_ignore_rules(&project.path, &add, &remove)?;
            if !update.added.is_empty() || !update.removed.is_empty() {
                let summary = format!("hide {:?}, unhide {:?}", update.added, update.removed);
                host.on_hiding_changed(&project, &summary);
            }
            let next = if update.added.is_empty() && update.removed.is_empty() {
                "Nothing changed."
            } else {
                "Call save_version to record the new rules."
            };
            serde_json::to_string_pretty(&json!({
                "added": update.added,
                "removed": update.removed,
                "unchanged": update.unchanged,
                "stopped_saving": update.stopped_saving,
                "next": next,
            }))
            .map_err(|e| e.to_string())
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
    fn only_read_only_tools_can_be_tried() {
        let (host, _dir) = host_with_repo();
        let listed = super::preview_tool(&host, "list_projects", &json!({})).unwrap();
        assert!(listed.contains("Demo"));
        let refused = super::preview_tool(&host, "save_version", &json!({ "project": "Demo", "title": "x" }));
        assert!(refused.unwrap_err().starts_with("MCP_TOOL_NOT_PREVIEWABLE"));
        assert!(host.saved.lock().unwrap().is_empty());
    }

    #[test]
    fn every_tool_says_whether_it_writes() {
        for tool in super::tools().as_array().unwrap() {
            assert!(tool["annotations"]["readOnlyHint"].is_boolean(), "{}", tool["name"]);
        }
        assert!(!super::is_read_only("save_version"));
        assert!(super::is_read_only("get_diff"));
    }

    #[test]
    fn agents_can_hide_files_and_save_the_rules() {
        let (host, dir) = host_with_repo();
        std::fs::create_dir_all(dir.path().join("dist")).unwrap();
        std::fs::write(dir.path().join("dist").join("app.js"), "built").unwrap();

        let updated = call(&host, "update_ignore_rules", json!({ "project": "Demo", "add": ["dist/"] }));
        let text = updated["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"added\"") && text.contains("dist/"), "{text}");

        let rules = call(&host, "get_ignore_rules", json!({ "project": "Demo" }));
        let text = rules["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"gitignore_rules\"") && text.contains("dist/"), "{text}");

        let saved = call(&host, "save_version", json!({ "project": "Demo", "title": "Hide build output" }));
        assert_eq!(saved["result"]["isError"], false, "{}", saved);

        let refused = call(&host, "update_ignore_rules", json!({ "project": "Demo", "add": ["*"] }));
        assert_eq!(refused["result"]["isError"], true);
    }

    #[test]
    fn unknown_project_is_tool_error() {
        let (host, _dir) = host_with_repo();
        let response = call(&host, "get_status", json!({ "project": "C:/nowhere" }));
        assert_eq!(response["result"]["isError"], true);
    }
}
