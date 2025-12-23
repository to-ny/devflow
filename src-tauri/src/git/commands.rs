use std::path::Path;

use log::info;

use super::service::GitService;
use super::types::{ChangedFile, FileDiff, FileStatus};

#[derive(serde::Serialize)]
pub struct RepoCheckResult {
    pub is_repo: bool,
    pub path: String,
    pub exists: bool,
    pub is_dir: bool,
    pub error: Option<String>,
}

#[tauri::command]
pub fn git_is_repository(path: String) -> RepoCheckResult {
    let result = GitService::check_repository(Path::new(&path));
    RepoCheckResult {
        is_repo: result.is_repo,
        path,
        exists: result.exists,
        is_dir: result.is_dir,
        error: result.error,
    }
}

#[tauri::command]
pub fn git_get_changed_files(project_path: String) -> Result<Vec<ChangedFile>, String> {
    info!("git_get_changed_files: path={}", project_path);
    let service = GitService::open(Path::new(&project_path)).map_err(|e| {
        info!("git_get_changed_files: failed to open: {}", e);
        e.to_string()
    })?;
    let files = service.get_changed_files().map_err(|e| {
        info!("git_get_changed_files: failed to get files: {}", e);
        e.to_string()
    })?;
    info!("git_get_changed_files: returning {} files", files.len());
    Ok(files)
}

/// Get file diff with status passed from frontend (avoids redundant git status call)
#[tauri::command]
pub fn git_get_file_diff_with_status(
    project_path: String,
    file_path: String,
    index_status: Option<FileStatus>,
    worktree_status: Option<FileStatus>,
) -> Result<FileDiff, String> {
    info!(
        "git_get_file_diff_with_status: path={}, index={:?}, worktree={:?}",
        file_path, index_status, worktree_status
    );
    let service = GitService::open(Path::new(&project_path)).map_err(|e| e.to_string())?;
    service
        .get_file_diff_with_status(&file_path, index_status, worktree_status)
        .map_err(|e| e.to_string())
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
