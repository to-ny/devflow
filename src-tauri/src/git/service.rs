use std::path::{Path, PathBuf};
use std::process::Command;

use gix::status::index_worktree::iter::Summary;
use gix::Repository;

use super::error::GitError;
use super::types::{
    ChangedFile, DiffHunk, DiffLine, FileDiff, FileStatus, LineKind, RepositoryCheckResult,
};

/// GitService supports two modes:
/// - Native mode using gix library (faster, but doesn't work on WSL UNC paths from Windows)
/// - CLI mode using git commands (slower, but works everywhere)
pub struct GitService {
    mode: GitMode,
}

enum GitMode {
    Native(Box<Repository>),
    Cli(PathBuf),
}

impl GitService {
    pub fn open(path: &Path) -> Result<Self, GitError> {
        // Try gix first
        match gix::open(path) {
            Ok(repo) => Ok(Self {
                mode: GitMode::Native(Box::new(repo)),
            }),
            Err(e) => {
                // Fallback to CLI mode if .git exists (handles WSL paths on Windows)
                let git_dir = path.join(".git");
                if git_dir.exists() {
                    return Ok(Self {
                        mode: GitMode::Cli(path.to_path_buf()),
                    });
                }

                let msg = e.to_string().to_lowercase();
                if msg.contains("not a git repository")
                    || msg.contains("does not appear to be a git repository")
                {
                    Err(GitError::NotARepository(path.to_path_buf()))
                } else {
                    Err(GitError::GixError(Box::new(e)))
                }
            }
        }
    }

    /// Check if a path is a git repository
    pub fn is_repository(path: &Path) -> bool {
        gix::open(path).is_ok() || path.join(".git").exists()
    }

    /// Check if a path is a git repository with detailed result
    pub fn check_repository(path: &Path) -> RepositoryCheckResult {
        let exists = path.exists();
        let is_dir = path.is_dir();

        match gix::open(path) {
            Ok(_) => RepositoryCheckResult {
                is_repo: true,
                exists,
                is_dir,
                error: None,
            },
            Err(e) => {
                // Fallback: check if .git directory exists (handles WSL paths on Windows)
                let git_dir = path.join(".git");
                if git_dir.exists() {
                    return RepositoryCheckResult {
                        is_repo: true,
                        exists,
                        is_dir,
                        error: None,
                    };
                }

                RepositoryCheckResult {
                    is_repo: false,
                    exists,
                    is_dir,
                    error: Some(e.to_string()),
                }
            }
        }
    }

    /// Get list of all files with unstaged changes
    pub fn get_changed_files(&self) -> Result<Vec<ChangedFile>, GitError> {
        match &self.mode {
            GitMode::Native(repo) => self.get_changed_files_native(repo),
            GitMode::Cli(path) => self.get_changed_files_cli(path),
        }
    }

    fn get_changed_files_native(&self, repo: &Repository) -> Result<Vec<ChangedFile>, GitError> {
        let mut files = Vec::new();

        // Get status of working directory
        let status = repo
            .status(gix::progress::Discard)
            .map_err(|e| GitError::GixError(Box::new(e)))?
            .index_worktree_options_mut(|opts| {
                opts.sorting =
                    Some(gix::status::plumbing::index_as_worktree_with_renames::Sorting::ByPathCaseSensitive);
            })
            .into_index_worktree_iter(Vec::new())
            .map_err(|e| GitError::GixError(Box::new(e)))?;

        for item in status {
            let item = item.map_err(|e| GitError::GixError(Box::new(e)))?;
            let path = String::from_utf8_lossy(item.rela_path()).to_string();

            // Use the summary() method for cleaner status detection
            let file_status = match item.summary() {
                Some(Summary::Added) => FileStatus::Untracked,
                Some(Summary::Removed) => FileStatus::Deleted,
                Some(Summary::Modified) => FileStatus::Modified,
                Some(Summary::TypeChange) => FileStatus::Modified,
                Some(Summary::Renamed) => FileStatus::Renamed,
                Some(Summary::Copied) => FileStatus::Copied,
                Some(Summary::IntentToAdd) => FileStatus::Added,
                Some(Summary::Conflict) => FileStatus::Modified,
                None => continue, // Skip items that don't represent actual changes
            };

            files.push(ChangedFile {
                path,
                status: file_status,
            });
        }

        Ok(files)
    }

    fn get_changed_files_cli(&self, workdir: &Path) -> Result<Vec<ChangedFile>, GitError> {
        let output = Command::new("git")
            .args(["status", "--porcelain", "-uall"])
            .current_dir(workdir)
            .output()?;

        if !output.status.success() {
            return Err(GitError::GixError(Box::new(std::io::Error::other(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))));
        }

        let mut files = Vec::new();
        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            if line.len() < 4 {
                continue;
            }

            let status_code = &line[0..2];
            let file_path = line[3..].to_string();

            let status = match status_code {
                "??" => FileStatus::Untracked,
                " M" | "M " | "MM" => FileStatus::Modified,
                " D" | "D " => FileStatus::Deleted,
                "A " | " A" => FileStatus::Added,
                "R " | " R" => FileStatus::Renamed,
                "C " | " C" => FileStatus::Copied,
                _ => FileStatus::Modified,
            };

            files.push(ChangedFile {
                path: file_path,
                status,
            });
        }

        Ok(files)
    }

    /// Get unified diff for a specific file
    pub fn get_file_diff(&self, file_path: &str) -> Result<FileDiff, GitError> {
        let diffs = self.get_all_diffs()?;

        diffs
            .into_iter()
            .find(|d| d.path == file_path)
            .ok_or_else(|| GitError::FileNotFound(file_path.to_string()))
    }

    /// Get the working directory path
    fn workdir(&self) -> Result<PathBuf, GitError> {
        match &self.mode {
            GitMode::Native(repo) => repo.workdir().map(|p| p.to_path_buf()).ok_or_else(|| {
                GitError::GixError(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "No working directory",
                )))
            }),
            GitMode::Cli(path) => Ok(path.clone()),
        }
    }

    /// Get unified diff for all unstaged changes
    pub fn get_all_diffs(&self) -> Result<Vec<FileDiff>, GitError> {
        let changed_files = self.get_changed_files()?;
        let mut file_diffs = Vec::new();
        let workdir = self.workdir()?;

        for file in changed_files {
            let file_path = workdir.join(&file.path);
            let mut hunks = Vec::new();

            match file.status {
                FileStatus::Untracked | FileStatus::Added => {
                    // New file - show all lines as additions
                    if file_path.is_file() {
                        if let Ok(content) = std::fs::read_to_string(&file_path) {
                            let lines: Vec<DiffLine> = content
                                .lines()
                                .enumerate()
                                .map(|(i, line)| DiffLine {
                                    kind: LineKind::Addition,
                                    old_line_no: None,
                                    new_line_no: Some((i + 1) as u32),
                                    content: line.to_string(),
                                })
                                .collect();

                            if !lines.is_empty() {
                                hunks.push(DiffHunk {
                                    old_start: 0,
                                    old_lines: 0,
                                    new_start: 1,
                                    new_lines: lines.len() as u32,
                                    lines,
                                });
                            }
                        }
                    }
                }
                FileStatus::Deleted => {
                    // Deleted file - get content from HEAD
                    if let Ok(old_content) = self.get_file_content_from_head(&file.path) {
                        let lines: Vec<DiffLine> = old_content
                            .lines()
                            .enumerate()
                            .map(|(i, line)| DiffLine {
                                kind: LineKind::Deletion,
                                old_line_no: Some((i + 1) as u32),
                                new_line_no: None,
                                content: line.to_string(),
                            })
                            .collect();

                        if !lines.is_empty() {
                            hunks.push(DiffHunk {
                                old_start: 1,
                                old_lines: lines.len() as u32,
                                new_start: 0,
                                new_lines: 0,
                                lines,
                            });
                        }
                    }
                }
                FileStatus::Modified | FileStatus::Renamed | FileStatus::Copied => {
                    // Modified file - compute diff between HEAD and working dir
                    let old_content = self
                        .get_file_content_from_head(&file.path)
                        .unwrap_or_default();
                    let new_content = std::fs::read_to_string(&file_path).unwrap_or_default();

                    hunks = self.compute_diff(&old_content, &new_content);
                }
            }

            file_diffs.push(FileDiff {
                path: file.path,
                status: file.status,
                hunks,
            });
        }

        Ok(file_diffs)
    }

    /// Get file content from HEAD commit
    fn get_file_content_from_head(&self, file_path: &str) -> Result<String, GitError> {
        match &self.mode {
            GitMode::Native(repo) => self.get_file_content_from_head_native(repo, file_path),
            GitMode::Cli(workdir) => self.get_file_content_from_head_cli(workdir, file_path),
        }
    }

    fn get_file_content_from_head_native(
        &self,
        repo: &Repository,
        file_path: &str,
    ) -> Result<String, GitError> {
        let head_commit = repo
            .head_commit()
            .map_err(|e| GitError::GixError(Box::new(e)))?;

        let tree = head_commit
            .tree()
            .map_err(|e| GitError::GixError(Box::new(e)))?;

        let entry = tree
            .lookup_entry_by_path(file_path)
            .map_err(|e| GitError::GixError(Box::new(e)))?
            .ok_or_else(|| GitError::FileNotFound(file_path.to_string()))?;

        let object = entry
            .object()
            .map_err(|e| GitError::GixError(Box::new(e)))?;

        let blob = object.into_blob();
        let content = String::from_utf8_lossy(blob.data.as_ref()).to_string();

        Ok(content)
    }

    fn get_file_content_from_head_cli(
        &self,
        workdir: &Path,
        file_path: &str,
    ) -> Result<String, GitError> {
        let output = Command::new("git")
            .args(["show", &format!("HEAD:{}", file_path)])
            .current_dir(workdir)
            .output()?;

        if !output.status.success() {
            return Err(GitError::FileNotFound(file_path.to_string()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Compute diff between two strings using a simple line-based diff
    fn compute_diff(&self, old: &str, new: &str) -> Vec<DiffHunk> {
        let old_lines: Vec<&str> = old.lines().collect();
        let new_lines: Vec<&str> = new.lines().collect();

        // Simple diff algorithm - for production, consider using the `similar` crate
        let mut hunks = Vec::new();
        let mut diff_lines = Vec::new();
        let mut old_idx = 0;
        let mut new_idx = 0;
        let hunk_old_start = 1;
        let hunk_new_start = 1;

        // Use longest common subsequence approach
        let lcs = self.compute_lcs(&old_lines, &new_lines);
        let mut lcs_idx = 0;

        while old_idx < old_lines.len() || new_idx < new_lines.len() {
            if lcs_idx < lcs.len() {
                let (lcs_old, lcs_new) = lcs[lcs_idx];

                // Add deletions (lines in old but not at LCS position)
                while old_idx < lcs_old {
                    diff_lines.push(DiffLine {
                        kind: LineKind::Deletion,
                        old_line_no: Some((old_idx + 1) as u32),
                        new_line_no: None,
                        content: old_lines[old_idx].to_string(),
                    });
                    old_idx += 1;
                }

                // Add additions (lines in new but not at LCS position)
                while new_idx < lcs_new {
                    diff_lines.push(DiffLine {
                        kind: LineKind::Addition,
                        old_line_no: None,
                        new_line_no: Some((new_idx + 1) as u32),
                        content: new_lines[new_idx].to_string(),
                    });
                    new_idx += 1;
                }

                // Add context line (the matching line)
                diff_lines.push(DiffLine {
                    kind: LineKind::Context,
                    old_line_no: Some((old_idx + 1) as u32),
                    new_line_no: Some((new_idx + 1) as u32),
                    content: old_lines[old_idx].to_string(),
                });
                old_idx += 1;
                new_idx += 1;
                lcs_idx += 1;
            } else {
                // No more LCS matches - remaining lines are changes
                while old_idx < old_lines.len() {
                    diff_lines.push(DiffLine {
                        kind: LineKind::Deletion,
                        old_line_no: Some((old_idx + 1) as u32),
                        new_line_no: None,
                        content: old_lines[old_idx].to_string(),
                    });
                    old_idx += 1;
                }
                while new_idx < new_lines.len() {
                    diff_lines.push(DiffLine {
                        kind: LineKind::Addition,
                        old_line_no: None,
                        new_line_no: Some((new_idx + 1) as u32),
                        content: new_lines[new_idx].to_string(),
                    });
                    new_idx += 1;
                }
            }
        }

        // Create hunks from diff lines (simplified - one hunk for all changes)
        if !diff_lines.is_empty() {
            let old_count = diff_lines
                .iter()
                .filter(|l| matches!(l.kind, LineKind::Deletion | LineKind::Context))
                .count();
            let new_count = diff_lines
                .iter()
                .filter(|l| matches!(l.kind, LineKind::Addition | LineKind::Context))
                .count();

            hunks.push(DiffHunk {
                old_start: hunk_old_start,
                old_lines: old_count as u32,
                new_start: hunk_new_start,
                new_lines: new_count as u32,
                lines: diff_lines,
            });
        }

        hunks
    }

    /// Compute longest common subsequence indices
    fn compute_lcs(&self, old: &[&str], new: &[&str]) -> Vec<(usize, usize)> {
        let m = old.len();
        let n = new.len();

        if m == 0 || n == 0 {
            return Vec::new();
        }

        // Build LCS table
        let mut dp = vec![vec![0; n + 1]; m + 1];
        for i in 1..=m {
            for j in 1..=n {
                if old[i - 1] == new[j - 1] {
                    dp[i][j] = dp[i - 1][j - 1] + 1;
                } else {
                    dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
                }
            }
        }

        // Backtrack to find LCS indices
        let mut result = Vec::new();
        let mut i = m;
        let mut j = n;
        while i > 0 && j > 0 {
            if old[i - 1] == new[j - 1] {
                result.push((i - 1, j - 1));
                i -= 1;
                j -= 1;
            } else if dp[i - 1][j] > dp[i][j - 1] {
                i -= 1;
            } else {
                j -= 1;
            }
        }

        result.reverse();
        result
    }

    /// Stage all changes (git add --all)
    pub fn stage_all(&self) -> Result<(), GitError> {
        let workdir = self.workdir()?;

        // Use git command for staging as gix index manipulation is complex
        let output = Command::new("git")
            .args(["add", "--all"])
            .current_dir(&workdir)
            .output()?;

        if !output.status.success() {
            return Err(GitError::GixError(Box::new(std::io::Error::other(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn create_test_repo() -> (TempDir, GitService) {
        let temp_dir = tempfile::tempdir().unwrap();

        // Initialize git repo
        Command::new("git")
            .args(["init"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        // Configure git user for commits
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        let service = GitService::open(temp_dir.path()).unwrap();
        (temp_dir, service)
    }

    fn create_initial_commit(temp_dir: &TempDir) {
        fs::write(temp_dir.path().join("initial.txt"), "initial content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();
    }

    #[test]
    fn test_open_non_repo() {
        let temp_dir = tempfile::tempdir().unwrap();
        let result = GitService::open(temp_dir.path());
        assert!(matches!(result, Err(GitError::NotARepository(_))));
    }

    #[test]
    fn test_is_repository() {
        let temp_dir = tempfile::tempdir().unwrap();
        assert!(!GitService::is_repository(temp_dir.path()));

        Command::new("git")
            .args(["init"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        assert!(GitService::is_repository(temp_dir.path()));
    }

    #[test]
    fn test_get_changed_files_empty_repo() {
        let (temp_dir, service) = create_test_repo();

        // Create a new file
        fs::write(temp_dir.path().join("test.txt"), "hello").unwrap();

        let files = service.get_changed_files().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "test.txt");
        assert_eq!(files[0].status, FileStatus::Untracked);
    }

    #[test]
    fn test_get_changed_files_modified() {
        let (temp_dir, service) = create_test_repo();
        create_initial_commit(&temp_dir);

        // Modify the file
        fs::write(temp_dir.path().join("initial.txt"), "modified content").unwrap();

        let files = service.get_changed_files().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "initial.txt");
        assert_eq!(files[0].status, FileStatus::Modified);
    }

    #[test]
    fn test_get_changed_files_deleted() {
        let (temp_dir, service) = create_test_repo();
        create_initial_commit(&temp_dir);

        // Delete the file
        fs::remove_file(temp_dir.path().join("initial.txt")).unwrap();

        let files = service.get_changed_files().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "initial.txt");
        assert_eq!(files[0].status, FileStatus::Deleted);
    }

    #[test]
    fn test_stage_all() {
        let (temp_dir, service) = create_test_repo();
        create_initial_commit(&temp_dir);

        fs::write(temp_dir.path().join("new.txt"), "content").unwrap();

        service.stage_all().unwrap();

        // Verify by checking git status
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        let status = String::from_utf8_lossy(&output.stdout);
        assert!(status.contains("A  new.txt"));
    }
}
