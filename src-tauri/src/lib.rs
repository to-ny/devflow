pub mod agent;
pub mod config;
pub mod git;
mod menu;

use std::sync::{Arc, RwLock};

use tauri::{Listener, Manager};

use agent::commands::{
    agent_approve_plan, agent_cancel, agent_clear_state, agent_has_pending_plan, agent_is_running,
    agent_reject_plan, agent_send_message, get_session_usage, reset_session_usage,
};
use agent::{AgentState, SessionUsageTracker};
use config::commands::{
    config_get_last_project, config_get_providers, config_load_project, config_project_exists,
    config_save_project, config_set_last_project,
};
use git::commands::{
    git_get_changed_files, git_get_file_diff_with_status, git_is_repository, git_stage_all,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Webview,
                ))
                .build(),
        )
        .manage(RwLock::new(AgentState::new()))
        .manage(Arc::new(SessionUsageTracker::new()))
        .setup(|app| {
            menu::setup(app)?;

            // Listen for config changes and mark agent state stale
            let handle = app.handle().clone();
            app.listen("config-changed", move |_| {
                if let Some(state) = handle.try_state::<RwLock<AgentState>>() {
                    if let Ok(mut guard) = state.write() {
                        guard.mark_config_stale();
                    }
                }
            });

            #[cfg(feature = "devtools")]
            if let Some(window) = app.get_webview_window("main") {
                window.open_devtools();
            }
            Ok(())
        })
        .on_menu_event(|app, event| {
            menu::handle_event(app, event.id().as_ref());
        })
        .invoke_handler(tauri::generate_handler![
            agent_send_message,
            agent_cancel,
            agent_is_running,
            agent_clear_state,
            agent_approve_plan,
            agent_reject_plan,
            agent_has_pending_plan,
            get_session_usage,
            reset_session_usage,
            git_is_repository,
            git_get_changed_files,
            git_get_file_diff_with_status,
            git_stage_all,
            config_get_last_project,
            config_set_last_project,
            config_project_exists,
            config_load_project,
            config_get_providers,
            config_save_project,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
