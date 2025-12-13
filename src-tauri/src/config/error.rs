use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file not found: {0}")]
    NotFound(PathBuf),

    #[error("Failed to read config file {path}: {source}")]
    ReadError {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to parse config file {path}: {source}")]
    ParseError {
        path: PathBuf,
        source: toml::de::Error,
    },

    #[error("Failed to serialize config: {0}")]
    SerializeError(#[from] toml::ser::Error),

    #[error("Failed to write config file {path}: {source}")]
    WriteError {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to create config directory {path}: {source}")]
    CreateDirError {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Could not determine app data directory")]
    NoAppDataDir,

    #[error("Invalid execution mode: {0}. Expected 'local' or 'container'")]
    InvalidExecutionMode(String),
}
