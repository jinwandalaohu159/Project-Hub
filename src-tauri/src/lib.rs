mod autostart;
mod commands;
mod credential_manager;
mod github;
mod logs;
mod mcp;
mod settings;
mod transfer;
mod tray;
mod update;

use project_service::ProjectService;
use settings::AppSettings;
use std::{
    fs::OpenOptions,
    io::Write,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::WindowEvent;

pub struct AppState {
    pub project_service: Mutex<ProjectService>,
    pub settings: Mutex<AppSettings>,
    pub mcp: Mutex<commands::mcp::McpRuntime>,
}

pub(crate) fn trace_startup(event: impl AsRef<str>) {
    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let path = std::env::temp_dir().join("project-hub-startup.log");

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(
            file,
            "{timestamp_ms} pid={} {}",
            std::process::id(),
            event.as_ref()
        );
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if std::env::args().any(|arg| arg == "--mcp") {
        // Agents launch this as a stdio MCP server; no window, tray or single-app state involved.
        mcp::run_stdio(&mcp::FileHost);
        return;
    }

    let start_hidden = tray::should_start_hidden();

    trace_startup(format!(
        "run:enter args={:?}",
        std::env::args().collect::<Vec<_>>()
    ));

    let storage_path = settings::projects_storage_path();
    let storage_path = storage_path.to_str().expect("Invalid storage path");
    trace_startup(format!("run:storage_path={storage_path}"));

    trace_startup("project_service:start");
    let project_service = ProjectService::new(storage_path);
    trace_startup("project_service:done");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(move |app| {
            trace_startup("setup:enter");
            tray::setup_tray(app)?;
            trace_startup("setup:tray_created");

            #[cfg(target_os = "windows")]
            tray::schedule_tray_registration_retries(app.handle());

            #[cfg(target_os = "windows")]
            tauri::async_runtime::spawn_blocking(move || autostart::migrate_autostart_to_task(!start_hidden));

            let mcp_handle = app.handle().clone();
            tauri::async_runtime::spawn_blocking(move || commands::mcp::restart_mcp(&mcp_handle));

            trace_startup(format!(
                "setup:create_main_window visible={}",
                !start_hidden
            ));
            let window = tray::ensure_main_window(app.handle(), !start_hidden)?;

            if !start_hidden {
                let _ = window.show();
                let _ = window.set_focus();
            } else {
                trace_startup("setup:hidden_window_ready");
            }

            trace_startup("setup:done");
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != tray::MAIN_WINDOW_LABEL {
                return;
            }

            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .manage(AppState {
            project_service: Mutex::new(project_service),
            settings: Mutex::new(settings::load_settings()),
            mcp: Mutex::new(commands::mcp::McpRuntime::default()),
        })
        .invoke_handler(tauri::generate_handler![
            commands::projects::add_project,
            commands::projects::remove_project,
            commands::projects::list_projects,
            commands::projects::auto_relocate_projects,
            commands::projects::relocate_project,
            commands::preferences::get_app_settings,
            commands::preferences::update_author,
            commands::mcp::get_mcp_status,
            commands::mcp::set_mcp_config,
            autostart::get_autostart_enabled,
            logs::get_app_log_settings,
            logs::set_app_log_settings,
            logs::write_app_log,
            logs::read_app_logs,
            logs::open_app_log_dir,
            autostart::set_autostart_enabled,
            commands::git::get_branches,
            commands::git::get_current_branch,
            commands::git::switch_branch,
            commands::git::get_git_status,
            commands::git::get_git_status_summary,
            commands::git::has_unsaved_changes,
            commands::git::git_stash,
            commands::git::git_stash_pop,
            commands::git::discard_changes,
            commands::git::git_add,
            commands::git::git_add_all,
            commands::git::git_unstage,
            commands::git::git_commit,
            commands::git::git_save_version,
            commands::git::create_branch,
            commands::git::delete_branch,
            commands::git::merge_branch,
            commands::git::has_merge_conflicts,
            commands::git::get_log,
            commands::git::restore_commit,
            commands::git::get_draft,
            commands::git::restore_draft,
            commands::git::stash_to_draft,
            commands::git::discard_draft,
            commands::git::get_repo_state,
            commands::git::edit_record,
            commands::git::delete_record,
            commands::git::git_init,
            commands::git::is_git_repo,
            commands::projects::ensure_project_versioned,
            commands::cloud::github_login_browser,
            commands::cloud::github_login_token,
            commands::cloud::github_logout,
            commands::cloud::github_list_repos,
            commands::cloud::get_sync_state,
            commands::cloud::suggest_repo_name,
            commands::cloud::publish_to_github,
            commands::cloud::link_remote,
            commands::cloud::push_to_remote,
            commands::transfer::export_data,
            commands::transfer::preview_import,
            commands::transfer::get_import_target_state,
            commands::transfer::apply_import,
            commands::update::get_app_version,
            commands::update::check_for_update,
            commands::update::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
