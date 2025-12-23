use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitError {
    #[error("Not a git repository: {0}")]
    NotARepository(PathBuf),

    #[error("File not found in diff: {0}")]
    FileNotFound(String),

    #[error("Git operation failed: {0}")]
    GixError(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
