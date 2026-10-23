//! The tools agents can call, by area: `local` works on the project itself (status, diff, saving,
//! history, hidden files), `cloud` uploads and downloads with GitHub, `release` publishes versions
//! on GitHub Releases. A project is named by id, by any path inside it, or by its name. Each tool
//! says whether it only reads (`annotations.readOnlyHint`); the settings page lists the same
//! definitions and may try out the read-only ones.

mod cloud;
mod local;
mod release;

use super::McpHost;
use project_service::{same_path, Project};
use serde_json::{json, Value};

pub(super) fn tools() -> Value {
    let project = json!({
        "type": "string",
        "description": "Project folder path (any path inside the project works), project name or id."
    });
    let mut all = local::definitions(&project);
    all.extend(cloud::definitions(&project));
    all.extend(release::definitions(&project));
    Value::Array(all)
}

pub(super) fn call_tool(host: &dyn McpHost, name: &str, args: &Value) -> Result<String, String> {
    local::call(host, name, args)
        .or_else(|| cloud::call(host, name, args))
        .or_else(|| release::call(host, name, args))
        .unwrap_or_else(|| Err(format!("UNKNOWN_TOOL:{name}")))
}

/// What an error means for an agent, and what to do about it; codes the app's own screens translate
/// are spelled out here.
fn agent_error(error: String) -> String {
    let (code, detail) = error.split_once(':').unwrap_or((error.as_str(), ""));
    match code {
        "GITHUB_NOT_SIGNED_IN" => "Project Hub is not signed in to GitHub. Ask the user to sign in under Settings → GitHub account, then try again.".to_string(),
        "GITHUB_UNAUTHORIZED" => "GitHub rejected Project Hub's sign-in. Ask the user to sign in again under Settings → GitHub account.".to_string(),
        "GITHUB_API_ERROR" => format!("GitHub returned an error: {}", detail.trim()),
        "REMOTE_NOT_LINKED" => "This project has no GitHub repository yet; upload_to_cloud with new_repository creates one.".to_string(),
        "SYNC_LOCAL_CHANGES" => format!("The cloud changed files that have unsaved changes here ({}). Save them with save_version (or have the user discard them), then download again.", detail.trim()),
        "SYNC_NOT_IN_CLOUD" => "The cloud does not have this version yet; upload_to_cloud first.".to_string(),
        "TAG_EXISTS" => format!("The tag {} already marks a different save record; pick a new version number.", detail.trim()),
        _ => error,
    }
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
