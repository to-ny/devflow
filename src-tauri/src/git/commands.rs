use std::path::Path;
use std::sync::OnceLock;

use log::info;

use super::highlighter::Highlighter;
use super::service::GitService;
use super::types::{ChangedFile, FileDiff, FileStatus, LineKind, RepositoryCheckResult};

static HIGHLIGHTER: OnceLock<Highlighter> = OnceLock::new();

fn get_highlighter() -> &'static Highlighter {
    HIGHLIGHTER.get_or_init(Highlighter::new)
}

fn apply_syntax_highlighting(mut diff: FileDiff) -> FileDiff {
    let highlighter = get_highlighter();

    // Build full content for context-aware highlighting
    let full_content: String = diff
        .hunks
        .iter()
        .flat_map(|h| h.lines.iter())
        .filter(|l| l.kind != LineKind::Deletion)
        .map(|l| l.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let highlighted_lines = highlighter.highlight_lines(&full_content, &diff.path);
    let mut highlight_iter = highlighted_lines.into_iter();

    for hunk in &mut diff.hunks {
        for line in &mut hunk.lines {
            if line.kind != LineKind::Deletion {
                line.highlighted = highlight_iter.next();
            } else {
                // For deletions, highlight individually
                let single = highlighter.highlight_lines(&line.content, &diff.path);
                line.highlighted = single.into_iter().next();
            }
        }
    }

    diff
}

#[tauri::command]
pub fn git_is_repository(path: String) -> RepositoryCheckResult {
    GitService::check_repository(Path::new(&path))
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
    let diff = service
        .get_file_diff_with_status(&file_path, index_status, worktree_status)
        .map_err(|e| e.to_string())?;
    Ok(apply_syntax_highlighting(diff))
}

#[tauri::command]
pub fn git_stage_all(project_path: String) -> Result<(), String> {
    let service = GitService::open(Path::new(&project_path)).map_err(|e| e.to_string())?;
    service.stage_all().map_err(|e| e.to_string())
}
