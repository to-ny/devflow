pub mod commands;
mod diff_parser;
mod error;
mod highlighter;
mod service;
mod types;
pub mod wsl;

pub use error::GitError;
pub use service::GitService;
pub use types::*;
