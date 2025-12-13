use std::path::Path;

use super::service::GitService;
use super::types::{ChangedFile, FileDiff};

#[tauri::command]
pub fn git_get_changed_files(project_path: String) -> Result<Vec<ChangedFile>, String> {
    let service = GitService::open(Path::new(&project_path)).map_err(|e| e.to_string())?;
    service.get_changed_files().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn git_get_file_diff(project_path: String, file_path: String) -> Result<FileDiff, String> {
    let service = GitService::open(Path::new(&project_path)).map_err(|e| e.to_string())?;
    service.get_file_diff(&file_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn git_get_all_diffs(project_path: String) -> Result<Vec<FileDiff>, String> {
    let service = GitService::open(Path::new(&project_path)).map_err(|e| e.to_string())?;
    service.get_all_diffs().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn git_stage_all(project_path: String) -> Result<(), String> {
    let service = GitService::open(Path::new(&project_path)).map_err(|e| e.to_string())?;
    service.stage_all().map_err(|e| e.to_string())
}
