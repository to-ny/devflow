pub mod config;
pub mod git;
mod menu;
pub mod watcher;

use std::sync::Mutex;

#[cfg(feature = "devtools")]
use tauri::Manager;

use config::commands::{
    config_get_last_project, config_load_permissions, config_load_project, config_project_exists,
    config_save_permissions, config_save_project, config_set_last_project,
};
use git::commands::{
    git_get_changed_files, git_get_file_diff_with_status, git_is_repository, git_stage_all,
};
use watcher::commands::{watcher_start, watcher_stop};
use watcher::WatcherState;

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
        .manage(WatcherState(Mutex::new(None)))
        .setup(|app| {
            menu::setup(app)?;
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
            git_is_repository,
            git_get_changed_files,
            git_get_file_diff_with_status,
            git_stage_all,
            config_get_last_project,
            config_set_last_project,
            config_project_exists,
            config_load_project,
            config_save_project,
            config_load_permissions,
            config_save_permissions,
            watcher_start,
            watcher_stop,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
