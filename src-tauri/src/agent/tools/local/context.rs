use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::Client;
use tokio::time::timeout;

use crate::agent::error::AgentError;

pub const MAX_OUTPUT_SIZE: usize = 1024 * 1024; // 1MB

#[derive(Clone)]
pub struct ExecutionContext {
    pub working_dir: PathBuf,
    pub timeout: Duration,
    pub http_client: Client,
}

impl ExecutionContext {
    pub fn new(working_dir: PathBuf, timeout_secs: u64) -> Self {
        let timeout = Duration::from_secs(timeout_secs);
        let http_client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            working_dir,
            timeout,
            http_client,
        }
    }

    pub async fn with_timeout<T, E, F>(&self, operation: &str, fut: F) -> Result<T, AgentError>
    where
        E: std::fmt::Display,
        F: Future<Output = Result<T, E>>,
    {
        match timeout(self.timeout, fut).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(e)) => Err(AgentError::ToolExecutionError(format!(
                "Failed to {}: {}",
                operation, e
            ))),
            Err(_) => Err(AgentError::ToolTimeout),
        }
    }

    /// Rejects absolute paths and path traversal attempts.
    pub fn resolve_path(&self, path: &str) -> Result<PathBuf, AgentError> {
        let path = Path::new(path);

        if path.is_absolute() {
            return Err(AgentError::InvalidToolInput(
                "Absolute paths are not allowed".to_string(),
            ));
        }

        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    return Err(AgentError::InvalidToolInput(
                        "Path traversal ('..') is not allowed".to_string(),
                    ));
                }
                std::path::Component::Prefix(_) => {
                    return Err(AgentError::InvalidToolInput(
                        "Invalid path component".to_string(),
                    ));
                }
                _ => {}
            }
        }

        let resolved = self.working_dir.join(path);

        if resolved.exists() {
            let canonical = resolved.canonicalize()?;
            let canonical_working = self.working_dir.canonicalize()?;

            if !canonical.starts_with(&canonical_working) {
                return Err(AgentError::InvalidToolInput(format!(
                    "Path '{}' resolves outside working directory",
                    path.display()
                )));
            }
        }

        Ok(resolved)
    }

    pub fn truncate_output(mut output: String) -> String {
        if output.len() > MAX_OUTPUT_SIZE {
            output.truncate(MAX_OUTPUT_SIZE);
            output.push_str("\n... (output truncated)");
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_resolve_path_rejects_absolute() {
        let temp = tempdir().unwrap();
        let ctx = ExecutionContext::new(temp.path().to_path_buf(), 30);
        assert!(ctx.resolve_path("/etc/passwd").is_err());
    }

    #[test]
    fn test_resolve_path_rejects_traversal() {
        let temp = tempdir().unwrap();
        let ctx = ExecutionContext::new(temp.path().to_path_buf(), 30);
        assert!(ctx.resolve_path("../../../etc/passwd").is_err());
    }

    #[test]
    fn test_resolve_path_accepts_relative() {
        let temp = tempdir().unwrap();
        let ctx = ExecutionContext::new(temp.path().to_path_buf(), 30);
        let result = ctx.resolve_path("foo/bar.txt");
        assert!(result.is_ok());
        assert!(result.unwrap().starts_with(temp.path()));
    }

    #[test]
    fn test_truncate_output() {
        let short = "hello".to_string();
        assert_eq!(ExecutionContext::truncate_output(short.clone()), short);

        let long = "x".repeat(MAX_OUTPUT_SIZE + 100);
        let truncated = ExecutionContext::truncate_output(long);
        assert!(truncated.len() <= MAX_OUTPUT_SIZE + 50);
        assert!(truncated.ends_with("(output truncated)"));
    }
}
