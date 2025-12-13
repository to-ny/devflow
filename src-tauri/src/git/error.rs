use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitError {
    #[error("Not a git repository: {0}")]
    NotARepository(PathBuf),

    #[error("Repository has no commits yet")]
    EmptyRepository,

    #[error("File not found in diff: {0}")]
    FileNotFound(String),

    #[error("Git operation failed: {0}")]
    Git2Error(#[from] git2::Error),
}
