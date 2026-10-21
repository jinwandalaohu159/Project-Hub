//! Local MCP server so coding agents can inspect changes and save versions through Project Hub.
//! Two transports share the same handlers: streamable HTTP inside the running app, and stdio
//! (`project-hub.exe --mcp`) for agents that only launch commands.
//!
//! `protocol` answers JSON-RPC messages (and runs the stdio loop), `tools` implements the tools
//! agents call, `http` serves them to local clients.

mod http;
mod protocol;
mod tools;
#[cfg(test)]
mod test_support;

use git_service::Identity;
use project_service::Project;

pub use http::{endpoint, McpServer};
pub use protocol::run_stdio;

/// The tool definitions agents get from `tools/list`, for the settings page to show.
pub fn tool_catalog() -> serde_json::Value {
    tools::tools()
}

/// Runs a read-only tool on behalf of the settings page; refuses tools that would change anything.
pub fn try_read_only_tool(host: &dyn McpHost, name: &str, args: &serde_json::Value) -> Result<String, String> {
    tools::preview_tool(host, name, args)
}

pub trait McpHost: Send + Sync {
    fn projects(&self) -> Vec<Project>;
    fn identity(&self) -> Option<Identity>;
    fn on_saved(&self, project: &Project, message: &str);
    /// The project's hiding rules changed (nothing is saved yet).
    fn on_hiding_changed(&self, _project: &Project, _summary: &str) {}
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
        crate::logs::append_project_log(&project.id, &project.name, "save.create", &format!("{}: {} (MCP)", project.name, message));
    }

    fn on_hiding_changed(&self, project: &Project, summary: &str) {
        crate::logs::append_project_log(&project.id, &project.name, "ignore.update", &format!("{}: {} (MCP)", project.name, summary));
    }
}

pub fn exe_path() -> String {
    std::env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_default()
}
