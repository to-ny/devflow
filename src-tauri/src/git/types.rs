use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct RepositoryCheckResult {
    pub is_repo: bool,
    pub exists: bool,
    pub is_dir: bool,
    pub error: Option<String>,
}

/// Status of a file in git (staged or unstaged)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Untracked,
}

/// A file with changes, tracking both staged (index) and unstaged (worktree) status
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct ChangedFile {
    pub path: String,
    /// Status of staged changes (in the index)
    pub index_status: Option<FileStatus>,
    /// Status of unstaged changes (in the working tree)
    pub worktree_status: Option<FileStatus>,
}

impl ChangedFile {
    /// Returns the primary status for display purposes
    /// Prefers worktree status over index status
    pub fn display_status(&self) -> FileStatus {
        self.worktree_status
            .or(self.index_status)
            .unwrap_or(FileStatus::Modified)
    }

    /// Returns true if the file has staged changes
    pub fn is_staged(&self) -> bool {
        self.index_status.is_some()
    }

    /// Returns true if the file has unstaged changes
    pub fn is_unstaged(&self) -> bool {
        self.worktree_status.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct FileDiff {
    pub path: String,
    pub status: FileStatus,
    pub hunks: Vec<DiffHunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct DiffLine {
    pub kind: LineKind,
    pub old_line_no: Option<u32>,
    pub new_line_no: Option<u32>,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub enum LineKind {
    Context,
    Addition,
    Deletion,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This test generates TypeScript type definitions.
    /// Run with `cargo test export_typescript_types` to regenerate.
    #[test]
    fn export_typescript_types() {
        // ts-rs automatically exports types when tests run if they have #[ts(export)]
        // This test just ensures the types are valid
        let _ = FileStatus::Modified;
        let _ = LineKind::Context;
    }
}
