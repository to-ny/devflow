pub mod config;
pub mod git;

use git::commands::{git_get_all_diffs, git_get_changed_files, git_get_file_diff, git_stage_all};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            git_get_changed_files,
            git_get_file_diff,
            git_get_all_diffs,
            git_stage_all,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
