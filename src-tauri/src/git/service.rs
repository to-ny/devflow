use std::cell::RefCell;
use std::path::Path;

use git2::{Delta, DiffOptions, Repository, StatusOptions};

use super::error::GitError;
use super::types::{ChangedFile, DiffHunk, DiffLine, FileDiff, FileStatus, LineKind};

pub struct GitService {
    repo: Repository,
}

impl GitService {
    pub fn open(path: &Path) -> Result<Self, GitError> {
        let repo = Repository::open(path).map_err(|e| {
            if e.code() == git2::ErrorCode::NotFound {
                GitError::NotARepository(path.to_path_buf())
            } else {
                GitError::Git2Error(e)
            }
        })?;

        Ok(Self { repo })
    }

    /// Check if the repository has at least one commit
    fn has_head(&self) -> bool {
        self.repo.head().is_ok()
    }

    /// Get list of all files with unstaged changes
    pub fn get_changed_files(&self) -> Result<Vec<ChangedFile>, GitError> {
        let mut options = StatusOptions::new();
        options.include_untracked(true);
        options.recurse_untracked_dirs(true);

        let statuses = self.repo.statuses(Some(&mut options))?;

        let mut files = Vec::new();

        for entry in statuses.iter() {
            let status = entry.status();
            let path = entry.path().unwrap_or("").to_string();

            // We're interested in workdir changes (unstaged)
            let file_status = if status.is_wt_new() {
                Some(FileStatus::Untracked)
            } else if status.is_wt_modified() {
                Some(FileStatus::Modified)
            } else if status.is_wt_deleted() {
                Some(FileStatus::Deleted)
            } else if status.is_wt_renamed() {
                Some(FileStatus::Renamed)
            } else if status.is_wt_typechange() {
                Some(FileStatus::Modified)
            } else {
                None
            };

            if let Some(file_status) = file_status {
                files.push(ChangedFile {
                    path,
                    status: file_status,
                });
            }
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

    /// Get unified diff for all unstaged changes
    pub fn get_all_diffs(&self) -> Result<Vec<FileDiff>, GitError> {
        let mut diff_options = DiffOptions::new();
        diff_options.include_untracked(true);
        diff_options.recurse_untracked_dirs(true);
        diff_options.show_untracked_content(true);

        let diff = if self.has_head() {
            // Compare HEAD tree to workdir
            let head = self.repo.head()?;
            let head_tree = head.peel_to_tree()?;
            self.repo
                .diff_tree_to_workdir_with_index(Some(&head_tree), Some(&mut diff_options))?
        } else {
            // Empty repository: show all files as new
            self.repo
                .diff_tree_to_workdir_with_index(None, Some(&mut diff_options))?
        };

        // Use RefCell for interior mutability to satisfy borrow checker
        let file_diffs: RefCell<Vec<FileDiff>> = RefCell::new(Vec::new());
        let current_file_idx: RefCell<Option<usize>> = RefCell::new(None);

        diff.foreach(
            &mut |delta, _| {
                let path = delta
                    .new_file()
                    .path()
                    .or_else(|| delta.old_file().path())
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();

                let status = match delta.status() {
                    Delta::Added => FileStatus::Added,
                    Delta::Deleted => FileStatus::Deleted,
                    Delta::Modified => FileStatus::Modified,
                    Delta::Renamed => FileStatus::Renamed,
                    Delta::Copied => FileStatus::Copied,
                    Delta::Untracked => FileStatus::Untracked,
                    _ => FileStatus::Modified,
                };

                let mut diffs = file_diffs.borrow_mut();
                *current_file_idx.borrow_mut() = Some(diffs.len());
                diffs.push(FileDiff {
                    path,
                    status,
                    hunks: Vec::new(),
                });

                true
            },
            None,
            Some(&mut |_delta, hunk| {
                if let Some(idx) = *current_file_idx.borrow() {
                    if let Some(file_diff) = file_diffs.borrow_mut().get_mut(idx) {
                        file_diff.hunks.push(DiffHunk {
                            old_start: hunk.old_start(),
                            old_lines: hunk.old_lines(),
                            new_start: hunk.new_start(),
                            new_lines: hunk.new_lines(),
                            lines: Vec::new(),
                        });
                    }
                }
                true
            }),
            Some(&mut |_delta, _hunk, line| {
                if let Some(idx) = *current_file_idx.borrow() {
                    if let Some(file_diff) = file_diffs.borrow_mut().get_mut(idx) {
                        if let Some(hunk) = file_diff.hunks.last_mut() {
                            let kind = match line.origin() {
                                '+' => LineKind::Addition,
                                '-' => LineKind::Deletion,
                                ' ' => LineKind::Context,
                                _ => return true, // Skip header lines etc
                            };

                            let content = String::from_utf8_lossy(line.content())
                                .trim_end()
                                .to_string();

                            hunk.lines.push(DiffLine {
                                kind,
                                old_line_no: line.old_lineno(),
                                new_line_no: line.new_lineno(),
                                content,
                            });
                        }
                    }
                }
                true
            }),
        )?;

        Ok(file_diffs.into_inner())
    }

    /// Stage all changes (git add --all)
    pub fn stage_all(&self) -> Result<(), GitError> {
        let mut index = self.repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
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
    fn test_get_changed_files_multiple() {
        let (temp_dir, service) = create_test_repo();
        create_initial_commit(&temp_dir);

        // Create new file
        fs::write(temp_dir.path().join("new.txt"), "new file").unwrap();
        // Modify existing
        fs::write(temp_dir.path().join("initial.txt"), "modified").unwrap();

        let files = service.get_changed_files().unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_get_all_diffs_new_file() {
        let (temp_dir, service) = create_test_repo();
        create_initial_commit(&temp_dir);

        fs::write(temp_dir.path().join("new.txt"), "line 1\nline 2\n").unwrap();

        let diffs = service.get_all_diffs().unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "new.txt");
        assert_eq!(diffs[0].status, FileStatus::Untracked);
        assert!(!diffs[0].hunks.is_empty());

        let hunk = &diffs[0].hunks[0];
        assert_eq!(hunk.lines.len(), 2);
        assert_eq!(hunk.lines[0].kind, LineKind::Addition);
        assert_eq!(hunk.lines[0].content, "line 1");
    }

    #[test]
    fn test_get_all_diffs_modified_file() {
        let (temp_dir, service) = create_test_repo();
        create_initial_commit(&temp_dir);

        fs::write(temp_dir.path().join("initial.txt"), "modified content").unwrap();

        let diffs = service.get_all_diffs().unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].status, FileStatus::Modified);

        let hunk = &diffs[0].hunks[0];
        // Should have deletion and addition
        let has_deletion = hunk.lines.iter().any(|l| l.kind == LineKind::Deletion);
        let has_addition = hunk.lines.iter().any(|l| l.kind == LineKind::Addition);
        assert!(has_deletion);
        assert!(has_addition);
    }

    #[test]
    fn test_get_file_diff() {
        let (temp_dir, service) = create_test_repo();
        create_initial_commit(&temp_dir);

        fs::write(temp_dir.path().join("new.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("initial.txt"), "modified").unwrap();

        let diff = service.get_file_diff("new.txt").unwrap();
        assert_eq!(diff.path, "new.txt");

        let result = service.get_file_diff("nonexistent.txt");
        assert!(matches!(result, Err(GitError::FileNotFound(_))));
    }

    #[test]
    fn test_get_file_diff_not_found() {
        let (temp_dir, service) = create_test_repo();
        create_initial_commit(&temp_dir);

        let result = service.get_file_diff("nonexistent.txt");
        assert!(matches!(result, Err(GitError::FileNotFound(_))));
    }

    #[test]
    fn test_stage_all() {
        let (temp_dir, service) = create_test_repo();
        create_initial_commit(&temp_dir);

        fs::write(temp_dir.path().join("new.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("initial.txt"), "modified").unwrap();

        service.stage_all().unwrap();

        // Verify files are staged by checking index
        let repo = Repository::open(temp_dir.path()).unwrap();
        let index = repo.index().unwrap();
        assert!(index.get_path(Path::new("new.txt"), 0).is_some());
    }

    #[test]
    fn test_diff_line_numbers() {
        let (temp_dir, service) = create_test_repo();

        // Create file with multiple lines
        fs::write(temp_dir.path().join("test.txt"), "line 1\nline 2\nline 3\n").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Add test.txt"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        // Modify line 2
        fs::write(
            temp_dir.path().join("test.txt"),
            "line 1\nmodified line 2\nline 3\n",
        )
        .unwrap();

        let diffs = service.get_all_diffs().unwrap();
        assert_eq!(diffs.len(), 1);

        let hunk = &diffs[0].hunks[0];
        // Check that we have proper line numbers
        for line in &hunk.lines {
            match line.kind {
                LineKind::Context => {
                    assert!(line.old_line_no.is_some());
                    assert!(line.new_line_no.is_some());
                }
                LineKind::Addition => {
                    assert!(line.new_line_no.is_some());
                }
                LineKind::Deletion => {
                    assert!(line.old_line_no.is_some());
                }
            }
        }
    }
}
