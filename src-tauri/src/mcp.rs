//! Local MCP server so coding agents can inspect changes and save versions through Project Hub.
//! Two transports share the same handlers: streamable HTTP inside the running app, and stdio
//! (`project-hub.exe --mcp`) for agents that only launch commands.

use git_service::{GitService, Identity};
use project_service::{same_path, Project};
use serde_json::{json, Value};
use std::{
    io::{BufRead, Read, Write},
    sync::Arc,
    thread::JoinHandle,
};

const SERVER_NAME: &str = "project-hub";
const SUPPORTED_PROTOCOLS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];
const MAX_BODY_BYTES: u64 = 4 * 1024 * 1024;
const DEFAULT_DIFF_BYTES: usize = 60_000;
const INSTRUCTIONS: &str = "Project Hub keeps local versions of the user's projects. \
Use get_status or get_diff to see unsaved work, then save_version with a short title and an optional \
description to record it. Pass the project's folder path (your working directory works) or its name.";

pub trait McpHost: Send + Sync {
    fn projects(&self) -> Vec<Project>;
    fn identity(&self) -> Option<Identity>;
    fn on_saved(&self, project: &Project, message: &str);
}

fn tools() -> Value {
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

fn call_tool(host: &dyn McpHost, name: &str, args: &Value) -> Result<String, String> {
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
            git.save_version(&project.path, &message)?;
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

fn rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// Handles one JSON-RPC message. Notifications (no id) produce no response.
pub fn handle_message(host: &dyn McpHost, message: &Value) -> Option<Value> {
    let id = message.get("id").cloned();
    let method = message.get("method").and_then(Value::as_str).unwrap_or_default();
    let params = message.get("params").cloned().unwrap_or(Value::Null);

    let id = id.filter(|id| !id.is_null())?;

    let result = match method {
        "initialize" => {
            let requested = params
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let version = if SUPPORTED_PROTOCOLS.contains(&requested) {
                requested
            } else {
                SUPPORTED_PROTOCOLS[0]
            };
            json!({
                "protocolVersion": version,
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": SERVER_NAME, "version": env!("CARGO_PKG_VERSION") },
                "instructions": INSTRUCTIONS,
            })
        }
        "ping" => json!({}),
        "tools/list" => json!({ "tools": tools() }),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or_default();
            let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
            match call_tool(host, name, &args) {
                Ok(text) => json!({ "content": [{ "type": "text", "text": text }], "isError": false }),
                Err(error) if error.starts_with("UNKNOWN_TOOL:") => {
                    return Some(rpc_error(id, -32602, &format!("Unknown tool: {name}")));
                }
                Err(error) => json!({ "content": [{ "type": "text", "text": error }], "isError": true }),
            }
        }
        "resources/list" => json!({ "resources": [] }),
        "prompts/list" => json!({ "prompts": [] }),
        _ => return Some(rpc_error(id, -32601, &format!("Method not found: {method}"))),
    };
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

fn handle_payload(host: &dyn McpHost, payload: &Value) -> Option<Value> {
    match payload {
        Value::Array(batch) => {
            let responses: Vec<Value> = batch.iter().filter_map(|m| handle_message(host, m)).collect();
            (!responses.is_empty()).then_some(Value::Array(responses))
        }
        single => handle_message(host, single),
    }
}

// ----- stdio transport -----

pub fn run_stdio(host: &dyn McpHost) {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(payload) => handle_payload(host, &payload),
            Err(_) => Some(rpc_error(Value::Null, -32700, "Parse error")),
        };
        if let Some(response) = response {
            let _ = writeln!(stdout, "{}", response);
            let _ = stdout.flush();
        }
    }
}

// ----- HTTP transport -----

pub struct McpServer {
    server: Arc<tiny_http::Server>,
    thread: Option<JoinHandle<()>>,
}

impl McpServer {
    pub fn start(port: u16, host: Arc<dyn McpHost>) -> Result<Self, String> {
        let server = tiny_http::Server::http(("127.0.0.1", port))
            .map_err(|e| format!("Cannot listen on 127.0.0.1:{port}: {e}"))?;
        let server = Arc::new(server);
        let worker = server.clone();
        let thread = std::thread::spawn(move || {
            for request in worker.incoming_requests() {
                let host = host.clone();
                // Tool calls may run git for a while; keep accepting other requests meanwhile.
                std::thread::spawn(move || serve(host.as_ref(), request));
            }
        });
        Ok(Self {
            server,
            thread: Some(thread),
        })
    }

    pub fn stop(mut self) {
        self.server.unblock();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub fn endpoint(port: u16) -> String {
    format!("http://127.0.0.1:{port}/mcp")
}

fn header(request: &tiny_http::Request, name: &str) -> Option<String> {
    request
        .headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str().to_string())
}

const LOCAL_HOSTS: &[&str] = &["127.0.0.1", "localhost", "[::1]"];

/// Host part of `host[:port]`, lowercased.
fn host_only(authority: &str) -> String {
    let authority = authority.trim().to_lowercase();
    if authority.starts_with('[') {
        return authority.split(']').next().map(|h| format!("{h}]")).unwrap_or_default();
    }
    authority.split(':').next().unwrap_or_default().to_string()
}

/// Blocks drive-by requests from web pages: browsers always send Origin, so it must be a local page.
fn origin_allowed(origin: Option<&str>) -> bool {
    let Some(origin) = origin else { return true };
    let origin = origin.trim().to_lowercase();
    let Some((scheme, rest)) = origin.split_once("://") else { return false };
    let host = host_only(rest.split('/').next().unwrap_or_default());
    match scheme {
        "http" | "https" => LOCAL_HOSTS.contains(&host.as_str()) || host == "tauri.localhost",
        "tauri" => host == "localhost",
        _ => false,
    }
}

/// DNS rebinding makes a hostile page "same origin" but its Host header still names the attacker.
fn host_allowed(host: Option<&str>) -> bool {
    host.is_some_and(|host| LOCAL_HOSTS.contains(&host_only(host).as_str()))
}

fn is_json_content_type(value: Option<&str>) -> bool {
    value.is_some_and(|value| {
        value.split(';').next().unwrap_or_default().trim().eq_ignore_ascii_case("application/json")
    })
}

fn respond(request: tiny_http::Request, status: u16, body: Option<String>) {
    let mut response = tiny_http::Response::from_string(body.clone().unwrap_or_default()).with_status_code(status);
    if body.is_some() {
        if let Ok(h) = tiny_http::Header::from_bytes("Content-Type", "application/json") {
            response = response.with_header(h);
        }
    }
    let _ = request.respond(response);
}

fn serve(host: &dyn McpHost, mut request: tiny_http::Request) {
    let path = request.url().split('?').next().unwrap_or_default().to_string();
    if path != "/mcp" && path != "/mcp/" && path != "/" {
        return respond(request, 404, None);
    }
    if !origin_allowed(header(&request, "Origin").as_deref())
        || !host_allowed(header(&request, "Host").as_deref())
    {
        return respond(request, 403, None);
    }
    match request.method() {
        tiny_http::Method::Post => {}
        tiny_http::Method::Get | tiny_http::Method::Delete => {
            let mut response = tiny_http::Response::empty(405);
            if let Ok(h) = tiny_http::Header::from_bytes("Allow", "POST") {
                response = response.with_header(h);
            }
            let _ = request.respond(response);
            return;
        }
        _ => return respond(request, 405, None),
    }
    // Exact type: anything a plain HTML form could send skips CORS preflight.
    if !is_json_content_type(header(&request, "Content-Type").as_deref()) {
        return respond(request, 415, None);
    }

    let mut body = String::new();
    if request
        .as_reader()
        .take(MAX_BODY_BYTES)
        .read_to_string(&mut body)
        .is_err()
    {
        return respond(request, 400, None);
    }
    let payload = match serde_json::from_str::<Value>(&body) {
        Ok(payload) => payload,
        Err(_) => {
            let error = rpc_error(Value::Null, -32700, "Parse error");
            return respond(request, 400, Some(error.to_string()));
        }
    };
    match handle_payload(host, &payload) {
        Some(response) => respond(request, 200, Some(response.to_string())),
        None => respond(request, 202, None),
    }
}

/// `project-hub.exe --mcp` host: reads the same project list and settings the app uses.
pub struct FileHost;

impl McpHost for FileHost {
    fn projects(&self) -> Vec<Project> {
        let storage = crate::settings::projects_storage_path();
        project_service::ProjectService::new(&storage.to_string_lossy()).list_projects()
    }

    fn identity(&self) -> Option<Identity> {
        crate::settings::load_settings().effective_identity()
    }

    fn on_saved(&self, project: &Project, message: &str) {
        crate::logs::append_app_log(&project.id, &project.name, "save.create", &format!("{}: {} (MCP)", project.name, message));
    }
}

pub fn exe_path() -> String {
    std::env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::Mutex;

    struct TestHost {
        projects: Vec<Project>,
        saved: Mutex<Vec<String>>,
    }

    impl McpHost for TestHost {
        fn projects(&self) -> Vec<Project> {
            self.projects.clone()
        }
        fn identity(&self) -> Option<Identity> {
            Some(Identity { name: "Agent".into(), email: "agent@example.com".into() })
        }
        fn on_saved(&self, _project: &Project, message: &str) {
            self.saved.lock().unwrap().push(message.to_string());
        }
    }

    fn git(path: &Path, args: &[&str]) {
        std::process::Command::new("git").args(args).current_dir(path).output().unwrap();
    }

    fn host_with_repo() -> (TestHost, tempfile::TempDir) {
        let dir = tempfile::TempDir::new().unwrap();
        git(dir.path(), &["init"]);
        git(dir.path(), &["config", "user.email", "t@t.com"]);
        git(dir.path(), &["config", "user.name", "T"]);
        std::fs::write(dir.path().join("a.txt"), "one").unwrap();
        git(dir.path(), &["add", "."]);
        git(dir.path(), &["commit", "-m", "init"]);
        let project = Project {
            id: "p1".into(),
            name: "Demo".into(),
            path: dir.path().to_string_lossy().to_string(),
            is_git_repo: true,
            added_at: chrono::Utc::now(),
            missing: false,
        };
        (TestHost { projects: vec![project], saved: Mutex::new(vec![]) }, dir)
    }

    fn call(host: &TestHost, name: &str, args: Value) -> Value {
        handle_message(
            host,
            &json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": args } }),
        )
        .unwrap()
    }

    #[test]
    fn initialize_negotiates_protocol() {
        let (host, _dir) = host_with_repo();
        let response = handle_message(
            &host,
            &json!({ "jsonrpc": "2.0", "id": 0, "method": "initialize", "params": { "protocolVersion": "2025-03-26" } }),
        )
        .unwrap();
        assert_eq!(response["result"]["protocolVersion"], "2025-03-26");
        assert!(handle_message(&host, &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })).is_none());
        let tools = handle_message(&host, &json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" })).unwrap();
        assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 5);
    }

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

    #[test]
    fn rejects_foreign_origins() {
        assert!(origin_allowed(None));
        assert!(origin_allowed(Some("http://localhost:3000")));
        assert!(origin_allowed(Some("http://127.0.0.1:47821")));
        assert!(!origin_allowed(Some("https://evil.example")));
        assert!(!origin_allowed(Some("http://localhost.evil.com")));
        assert!(!origin_allowed(Some("http://127.0.0.1.evil.com:47821")));
        assert!(!origin_allowed(Some("null")));
    }

    #[test]
    fn rejects_rebound_hosts_and_form_content_types() {
        assert!(host_allowed(Some("127.0.0.1:47821")));
        assert!(host_allowed(Some("localhost:47821")));
        assert!(!host_allowed(Some("evil.example:47821")));
        assert!(!host_allowed(None));
        assert!(is_json_content_type(Some("application/json; charset=utf-8")));
        assert!(!is_json_content_type(Some("text/plain;application/json")));
    }
}
