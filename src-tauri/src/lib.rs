pub mod config;
pub mod git;
mod menu;

#[cfg(feature = "devtools")]
use tauri::Manager;

use config::commands::{config_get_last_project, config_set_last_project};
use git::commands::{
    git_get_all_diffs, git_get_changed_files, git_get_file_diff, git_is_repository, git_stage_all,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            menu::setup(app)?;
            // Open devtools when built with --features devtools
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
            git_get_file_diff,
            git_get_all_diffs,
            git_stage_all,
            config_get_last_project,
            config_set_last_project,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
