use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("Failed to create templates directory: {path}")]
    CreateDirError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to read template: {path}")]
    ReadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to write template: {path}")]
    WriteError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to render template: {0}")]
    RenderError(#[from] handlebars::RenderError),

    #[error("Failed to register template: {0}")]
    TemplateError(#[from] handlebars::TemplateError),

    #[error("Could not determine config directory")]
    NoConfigDir,
}

impl serde::Serialize for TemplateError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
