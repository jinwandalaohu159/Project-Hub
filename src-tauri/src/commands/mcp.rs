//! Running the in-app MCP server and reporting its status.

use super::git::identity_of;
use super::preferences::update_settings;
use crate::logs::append_project_log;
use crate::{mcp, settings, AppState};
use serde::Serialize;
use std::sync::Arc;
use tauri::{Emitter, Manager};

pub(crate) const REPO_CHANGED_EVENT: &str = "repo-changed";

#[derive(Default)]
pub struct McpRuntime {
    server: Option<mcp::McpServer>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct McpStatus {
    enabled: bool,
    port: u16,
    running: bool,
    error: Option<String>,
    url: String,
    exe_path: String,
}

pub(crate) struct AppHost {
    app: tauri::AppHandle,
}

impl mcp::McpHost for AppHost {
    fn projects(&self) -> Vec<project_service::Project> {
        self.app
            .state::<AppState>()
            .project_service
            .lock()
            .map(|service| service.list_projects())
            .unwrap_or_default()
    }

    fn identity(&self) -> Option<git_service::Identity> {
        identity_of(&self.app.state::<AppState>())
    }

    fn on_saved(&self, project: &project_service::Project, message: &str) {
        append_project_log(
            &project.id,
            &project.name,
            "save.create",
            &format!("{}: {} (MCP)", project.name, message),
        );
        let _ = self
            .app
            .emit(REPO_CHANGED_EVENT, serde_json::json!({ "project_id": project.id }));
    }

    fn on_hiding_changed(&self, project: &project_service::Project, summary: &str) {
        append_project_log(
            &project.id,
            &project.name,
            "ignore.update",
            &format!("{}: {} (MCP)", project.name, summary),
        );
        let _ = self
            .app
            .emit(REPO_CHANGED_EVENT, serde_json::json!({ "project_id": project.id }));
    }
}

pub(crate) fn restart_mcp(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let (enabled, port) = state
        .settings
        .lock()
        .map(|s| (s.mcp_enabled, s.mcp_port))
        .unwrap_or((false, settings::DEFAULT_MCP_PORT));
    let Ok(mut runtime) = state.mcp.lock() else {
        return;
    };
    let was_running = runtime.server.is_some();
    if let Some(server) = runtime.server.take() {
        server.stop();
    }
    runtime.error = None;
    if !enabled {
        if was_running {
            crate::logs::app_log("info", "mcp.stop", "");
        }
        return;
    }
    let host: Arc<dyn mcp::McpHost> = Arc::new(AppHost { app: app.clone() });
    match mcp::McpServer::start(port, host) {
        Ok(server) => {
            runtime.server = Some(server);
            crate::logs::app_log("info", "mcp.start", &mcp::endpoint(port));
        }
        Err(error) => {
            crate::logs::app_log("error", "mcp.start", &format!("port {port}: {error}"));
            runtime.error = Some(error);
        }
    }
}

pub(crate) fn mcp_status(state: &AppState) -> McpStatus {
    let (enabled, port) = state
        .settings
        .lock()
        .map(|s| (s.mcp_enabled, s.mcp_port))
        .unwrap_or((false, settings::DEFAULT_MCP_PORT));
    let (running, error) = state
        .mcp
        .lock()
        .map(|runtime| (runtime.server.is_some(), runtime.error.clone()))
        .unwrap_or((false, None));
    McpStatus {
        enabled,
        port,
        running,
        error,
        url: mcp::endpoint(port),
        exe_path: mcp::exe_path(),
    }
}

#[tauri::command]
pub(crate) fn get_mcp_status(state: tauri::State<AppState>) -> McpStatus {
    mcp_status(&state)
}

#[tauri::command]
pub(crate) async fn set_mcp_config(app: tauri::AppHandle, enabled: bool, port: u16) -> Result<McpStatus, String> {
    if port < 1024 {
        return Err("INVALID_PORT".to_string());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        update_settings(&state, |settings| {
            settings.mcp_enabled = enabled;
            settings.mcp_port = port;
        })?;
        restart_mcp(&app);
        Ok(mcp_status(&state))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The tools agents see, as `tools/list` returns them (names, parameters, read-only or not).
#[tauri::command]
pub(crate) fn get_mcp_tools() -> serde_json::Value {
    mcp::tool_catalog()
}

/// "Try it" on the settings page: runs a read-only tool exactly as an agent would and returns its
/// answer. Works whether or not the server is switched on.
#[tauri::command]
pub(crate) async fn try_mcp_tool(app: tauri::AppHandle, name: String, args: serde_json::Value) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || mcp::try_read_only_tool(&AppHost { app }, &name, &args))
        .await
        .map_err(|e| e.to_string())?
}
