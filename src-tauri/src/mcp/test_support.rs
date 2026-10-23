//! A test host backed by a throwaway repository, shared by the MCP tests.

use super::protocol::handle_message;
use super::McpHost;
use git_service::Identity;
use project_service::Project;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Mutex;

pub(crate) struct TestHost {
    projects: Vec<Project>,
    /// Titles of the records tools saved.
    pub(crate) saved: Mutex<Vec<String>>,
    /// Every change tools reported, as (action, summary).
    pub(crate) changes: Mutex<Vec<(String, String)>>,
}

impl McpHost for TestHost {
    fn projects(&self) -> Vec<Project> {
        self.projects.clone()
    }
    fn identity(&self) -> Option<Identity> {
        Some(Identity { name: "Agent".into(), email: "agent@example.com".into() })
    }
    fn on_changed(&self, _project: &Project, action: &str, summary: &str) {
        if action == "save.create" {
            self.saved.lock().unwrap().push(summary.to_string());
        }
        self.changes.lock().unwrap().push((action.to_string(), summary.to_string()));
    }
}

fn git(path: &Path, args: &[&str]) {
    std::process::Command::new("git").args(args).current_dir(path).output().unwrap();
}

pub(crate) fn host_with_repo() -> (TestHost, tempfile::TempDir) {
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
    (TestHost { projects: vec![project], saved: Mutex::new(vec![]), changes: Mutex::new(vec![]) }, dir)
}

pub(crate) fn call(host: &TestHost, name: &str, args: Value) -> Value {
    handle_message(
        host,
        &json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": args } }),
    )
    .unwrap()
}
