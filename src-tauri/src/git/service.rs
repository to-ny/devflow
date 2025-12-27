use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use super::diff_parser::parse_unified_diff;
use super::error::GitError;
use super::types::{
    ChangedFile, DiffHunk, DiffLine, FileDiff, FileStatus, LineKind, RepositoryCheckResult,
};
use super::wsl::{is_wsl_path, parse_wsl_path, run_git_via_wsl, WslPath};

/// Windows flag to prevent console window from appearing
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Create a git command that won't show a console window on Windows
fn git_command() -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new("git");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// Git CLI wrapper. Routes WSL paths through wsl.exe for proper .gitignore handling.
pub struct GitService {
    workdir: PathBuf,
    wsl_path: Option<WslPath>,
}

impl GitService {
    pub fn open(path: &Path) -> Result<Self, GitError> {
        let git_dir = path.join(".git");
        if git_dir.exists() {
            let wsl_path = if is_wsl_path(path) {
                parse_wsl_path(path)
            } else {
                None
            };

            Ok(Self {
                workdir: path.to_path_buf(),
                wsl_path,
            })
        } else {
            Err(GitError::NotARepository(path.to_path_buf()))
        }
    }

    fn run_git(&self, args: &[&str]) -> std::io::Result<Output> {
        if let Some(ref wsl) = self.wsl_path {
            run_git_via_wsl(wsl, args)
        } else {
            git_command().args(args).current_dir(&self.workdir).output()
        }
    }

    pub fn is_repository(path: &Path) -> bool {
        path.join(".git").exists()
    }

    pub fn check_repository(path: &Path) -> RepositoryCheckResult {
        let exists = path.exists();
        let is_dir = path.is_dir();
        let git_dir = path.join(".git");

        RepositoryCheckResult {
            is_repo: git_dir.exists(),
            exists,
            is_dir,
            error: None,
        }
    }

    pub fn get_changed_files(&self) -> Result<Vec<ChangedFile>, GitError> {
        let output = self.run_git(&["status", "--porcelain", "-uall"])?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitError::CommandFailed(stderr.to_string()));
        }

        let mut files = Vec::new();
        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            if line.len() < 4 {
                continue;
            }

            // git status --porcelain format: XY filename
            // X = index status (staged), Y = worktree status (unstaged)
            let index_char = line.chars().next().unwrap_or(' ');
            let worktree_char = line.chars().nth(1).unwrap_or(' ');
            let file_path = line[3..].to_string();

            let index_status = parse_status_char(index_char);
            let worktree_status = parse_status_char(worktree_char);

            if index_char == '?' && worktree_char == '?' {
                files.push(ChangedFile {
                    path: file_path,
                    index_status: None,
                    worktree_status: Some(FileStatus::Untracked),
                });
                continue;
            }

            files.push(ChangedFile {
                path: file_path,
                index_status,
                worktree_status,
            });
        }

        Ok(files)
    }

    /// Accepts status to avoid redundant git status calls.
    pub fn get_file_diff_with_status(
        &self,
        file_path: &str,
        index_status: Option<FileStatus>,
        worktree_status: Option<FileStatus>,
    ) -> Result<FileDiff, GitError> {
        let display_status = worktree_status
            .or(index_status)
            .unwrap_or(FileStatus::Modified);

        let hunks = if worktree_status == Some(FileStatus::Untracked) {
            self.diff_untracked_file(file_path)?
        } else {
            self.run_git_diff(file_path, index_status, worktree_status)?
        };

        Ok(FileDiff {
            path: file_path.to_string(),
            status: display_status,
            hunks,
        })
    }

    fn diff_untracked_file(&self, file_path: &str) -> Result<Vec<DiffHunk>, GitError> {
        let full_path = self.workdir.join(file_path);
        if !full_path.is_file() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&full_path)?;
        let lines: Vec<DiffLine> = content
            .lines()
            .enumerate()
            .map(|(i, line)| DiffLine {
                kind: LineKind::Addition,
                old_line_no: None,
                new_line_no: Some((i + 1) as u32),
                content: line.to_string(),
                highlighted: None,
            })
            .collect();

        if lines.is_empty() {
            return Ok(Vec::new());
        }

        Ok(vec![DiffHunk {
            old_start: 0,
            old_lines: 0,
            new_start: 1,
            new_lines: lines.len() as u32,
            lines,
        }])
    }

    /// Uses --cached for staged-only, HEAD for unstaged/mixed changes.
    fn run_git_diff(
        &self,
        file_path: &str,
        index_status: Option<FileStatus>,
        worktree_status: Option<FileStatus>,
    ) -> Result<Vec<DiffHunk>, GitError> {
        let output = match (index_status, worktree_status) {
            (Some(_), None) => self.run_git(&["diff", "--cached", "--", file_path])?,
            _ => self.run_git(&["diff", "HEAD", "--", file_path])?,
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitError::CommandFailed(stderr.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_unified_diff(&stdout))
    }

    pub fn stage_all(&self) -> Result<(), GitError> {
        let output = self.run_git(&["add", "--all"])?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitError::CommandFailed(stderr.to_string()));
        }

        Ok(())
    }
}

fn parse_status_char(c: char) -> Option<FileStatus> {
    match c {
        'M' => Some(FileStatus::Modified),
        'A' => Some(FileStatus::Added),
        'D' => Some(FileStatus::Deleted),
        'R' => Some(FileStatus::Renamed),
        'C' => Some(FileStatus::Copied),
        ' ' | '?' => None,
        _ => Some(FileStatus::Modified), // Treat unknown as modified
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
    fn test_get_changed_files_untracked() {
        let (temp_dir, service) = create_test_repo();

        // Create a new file
        fs::write(temp_dir.path().join("test.txt"), "hello").unwrap();

        let files = service.get_changed_files().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "test.txt");
        assert_eq!(files[0].index_status, None);
        assert_eq!(files[0].worktree_status, Some(FileStatus::Untracked));
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
        assert_eq!(files[0].index_status, None);
        assert_eq!(files[0].worktree_status, Some(FileStatus::Modified));
    }

    #[test]
    fn test_get_changed_files_staged() {
        let (temp_dir, service) = create_test_repo();
        create_initial_commit(&temp_dir);

        // Modify and stage the file
        fs::write(temp_dir.path().join("initial.txt"), "modified content").unwrap();
        Command::new("git")
            .args(["add", "initial.txt"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        let files = service.get_changed_files().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "initial.txt");
        assert_eq!(files[0].index_status, Some(FileStatus::Modified));
        assert_eq!(files[0].worktree_status, None);
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
        assert_eq!(files[0].index_status, None);
        assert_eq!(files[0].worktree_status, Some(FileStatus::Deleted));
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
