//! Tools on the project itself: listing projects, status, diff, saving, history and the rules
//! that hide files from versions.

use super::{read_only, resolve_project, str_arg, str_list};
use crate::mcp::McpHost;
use git_service::GitService;
use serde_json::{json, Value};

const DEFAULT_DIFF_BYTES: usize = 60_000;

pub(super) fn definitions(project: &Value) -> Vec<Value> {
    let list = json!([
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
    ]);
    list.as_array().cloned().unwrap_or_default()
}

pub(super) fn call(host: &dyn McpHost, name: &str, args: &Value) -> Option<Result<String, String>> {
    let git = GitService::with_identity(host.identity());
    Some(match name {
        "list_projects" => list_projects(host, &git, args),
        "get_status" => get_status(host, &git, args),
        "get_diff" => get_diff(host, &git, args),
        "save_version" => save_version(host, &git, args),
        "get_history" => get_history(host, &git, args),
        "get_ignore_rules" => get_ignore_rules(host, &git, args),
        "update_ignore_rules" => update_ignore_rules(host, &git, args),
        _ => return None,
    })
}

fn list_projects(host: &dyn McpHost, _git: &GitService, _args: &Value) -> Result<String, String> {
    let list: Vec<Value> = host
        .projects()
        .into_iter()
        .map(|p| json!({ "id": p.id, "name": p.name, "path": p.path, "missing": p.missing }))
        .collect();
    serde_json::to_string_pretty(&list).map_err(|e| e.to_string())
}

fn get_status(host: &dyn McpHost, git: &GitService, args: &Value) -> Result<String, String> {
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

fn get_diff(host: &dyn McpHost, git: &GitService, args: &Value) -> Result<String, String> {
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

fn save_version(host: &dyn McpHost, git: &GitService, args: &Value) -> Result<String, String> {
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
    host.on_changed(&project, "save.create", title);
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

fn get_history(host: &dyn McpHost, git: &GitService, args: &Value) -> Result<String, String> {
    let project = resolve_project(host, str_arg(args, "project"))?;
    let limit = args
        .get("limit")
        .and_then(Value::as_u64)
        .map(|n| n.clamp(1, 100) as usize)
        .unwrap_or(20);
    let history = git.get_log(&project.path, limit)?;
    serde_json::to_string_pretty(&history).map_err(|e| e.to_string())
}

fn get_ignore_rules(host: &dyn McpHost, git: &GitService, args: &Value) -> Result<String, String> {
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

fn update_ignore_rules(host: &dyn McpHost, git: &GitService, args: &Value) -> Result<String, String> {
    let project = resolve_project(host, str_arg(args, "project"))?;
    let (add, remove) = (str_list(args, "add"), str_list(args, "remove"));
    if add.is_empty() && remove.is_empty() {
        return Err("Give \"add\" and/or \"remove\".".to_string());
    }
    let update = git.update_ignore_rules(&project.path, &add, &remove)?;
    if !update.added.is_empty() || !update.removed.is_empty() {
        let summary = format!("hide {:?}, unhide {:?}", update.added, update.removed);
        host.on_changed(&project, "ignore.update", &summary);
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
