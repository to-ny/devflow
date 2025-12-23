use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use log::debug;
use tokio::fs;
use tokio::process::Command;
use tokio::time::timeout;

use super::executor::ToolExecutor;
use super::types::{
    BashInput, EditFileInput, ListDirectoryInput, ReadFileInput, ToolName, WriteFileInput,
};
use crate::agent::error::AgentError;

const MAX_OUTPUT_SIZE: usize = 1024 * 1024; // 1MB

pub struct LocalExecutor {
    working_dir: PathBuf,
    timeout: Duration,
}

impl LocalExecutor {
    pub fn new(working_dir: PathBuf, timeout_secs: u64) -> Self {
        Self {
            working_dir,
            timeout: Duration::from_secs(timeout_secs),
        }
    }

    fn resolve_path(&self, path: &str) -> Result<PathBuf, AgentError> {
        let path = Path::new(path);

        // Reject absolute paths - all paths must be relative to working_dir
        if path.is_absolute() {
            return Err(AgentError::InvalidToolInput(
                "Absolute paths are not allowed".to_string(),
            ));
        }

        // Check for path traversal attempts in components
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

        // Additional check: if the path exists, verify it resolves within working_dir
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

    fn truncate_output(mut output: String) -> String {
        if output.len() > MAX_OUTPUT_SIZE {
            output.truncate(MAX_OUTPUT_SIZE);
            output.push_str("\n... (output truncated)");
        }
        output
    }

    async fn execute_bash(&self, input: serde_json::Value) -> Result<String, AgentError> {
        let input: BashInput = serde_json::from_value(input)
            .map_err(|e| AgentError::InvalidToolInput(format!("Invalid input: {}", e)))?;

        debug!("Executing bash command: {}", input.command);

        let result = timeout(
            self.timeout,
            Command::new("sh")
                .arg("-c")
                .arg(&input.command)
                .current_dir(&self.working_dir)
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                let combined = if stderr.is_empty() {
                    stdout.to_string()
                } else if stdout.is_empty() {
                    stderr.to_string()
                } else {
                    format!("{}\n{}", stdout, stderr)
                };

                let combined = Self::truncate_output(combined);

                if output.status.success() {
                    Ok(combined)
                } else {
                    Err(AgentError::ToolExecutionError(format!(
                        "Command failed with exit code {}: {}",
                        output.status.code().unwrap_or(-1),
                        combined
                    )))
                }
            }
            Ok(Err(e)) => Err(AgentError::ToolExecutionError(format!(
                "Failed to execute command: {}",
                e
            ))),
            Err(_) => Err(AgentError::ToolTimeout),
        }
    }

    async fn execute_read_file(&self, input: serde_json::Value) -> Result<String, AgentError> {
        let input: ReadFileInput = serde_json::from_value(input)
            .map_err(|e| AgentError::InvalidToolInput(format!("Invalid input: {}", e)))?;

        let path = self.resolve_path(&input.path)?;

        debug!("Reading file: {}", path.display());

        match timeout(self.timeout, fs::read_to_string(&path)).await {
            Ok(Ok(content)) => Ok(content),
            Ok(Err(e)) => Err(AgentError::ToolExecutionError(format!(
                "Failed to read file: {}",
                e
            ))),
            Err(_) => Err(AgentError::ToolTimeout),
        }
    }

    async fn execute_write_file(&self, input: serde_json::Value) -> Result<String, AgentError> {
        let input: WriteFileInput = serde_json::from_value(input)
            .map_err(|e| AgentError::InvalidToolInput(format!("Invalid input: {}", e)))?;

        let path = self.resolve_path(&input.path)?;

        debug!("Writing file: {}", path.display());

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await?;
            }
        }

        match timeout(self.timeout, fs::write(&path, &input.content)).await {
            Ok(Ok(())) => Ok(format!("Successfully wrote to {}", path.display())),
            Ok(Err(e)) => Err(AgentError::ToolExecutionError(format!(
                "Failed to write file: {}",
                e
            ))),
            Err(_) => Err(AgentError::ToolTimeout),
        }
    }

    async fn execute_edit_file(&self, input: serde_json::Value) -> Result<String, AgentError> {
        let input: EditFileInput = serde_json::from_value(input)
            .map_err(|e| AgentError::InvalidToolInput(format!("Invalid input: {}", e)))?;

        let path = self.resolve_path(&input.path)?;

        debug!(
            "Editing file: {} (replacing {} bytes)",
            path.display(),
            input.old_text.len()
        );

        // Read current content
        let content = match timeout(self.timeout, fs::read_to_string(&path)).await {
            Ok(Ok(c)) => c,
            Ok(Err(e)) => {
                return Err(AgentError::ToolExecutionError(format!(
                    "Failed to read file: {}",
                    e
                )))
            }
            Err(_) => return Err(AgentError::ToolTimeout),
        };

        // Check if old_text exists
        if !content.contains(&input.old_text) {
            return Err(AgentError::ToolExecutionError(
                "old_text not found in file".to_string(),
            ));
        }

        // Replace first occurrence
        let new_content = content.replacen(&input.old_text, &input.new_text, 1);

        // Write back
        match timeout(self.timeout, fs::write(&path, &new_content)).await {
            Ok(Ok(())) => Ok(format!("Successfully edited {}", path.display())),
            Ok(Err(e)) => Err(AgentError::ToolExecutionError(format!(
                "Failed to write file: {}",
                e
            ))),
            Err(_) => Err(AgentError::ToolTimeout),
        }
    }

    async fn execute_list_directory(&self, input: serde_json::Value) -> Result<String, AgentError> {
        let input: ListDirectoryInput = serde_json::from_value(input)
            .map_err(|e| AgentError::InvalidToolInput(format!("Invalid input: {}", e)))?;

        let path = self.resolve_path(&input.path)?;

        debug!("Listing directory: {}", path.display());

        let read_dir = match timeout(self.timeout, fs::read_dir(&path)).await {
            Ok(Ok(rd)) => rd,
            Ok(Err(e)) => {
                return Err(AgentError::ToolExecutionError(format!(
                    "Failed to read directory: {}",
                    e
                )))
            }
            Err(_) => return Err(AgentError::ToolTimeout),
        };

        let mut entries = Vec::new();
        let mut read_dir = read_dir;

        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            let file_type = match entry.file_type().await {
                Ok(ft) => {
                    if ft.is_dir() {
                        "dir"
                    } else if ft.is_file() {
                        "file"
                    } else {
                        "other"
                    }
                }
                Err(_) => "unknown",
            };

            entries.push(format!("{} ({})", name, file_type));
        }

        entries.sort();
        Ok(entries.join("\n"))
    }
}

#[async_trait]
impl ToolExecutor for LocalExecutor {
    async fn execute(
        &self,
        tool: ToolName,
        input: serde_json::Value,
    ) -> Result<String, AgentError> {
        match tool {
            ToolName::Bash => self.execute_bash(input).await,
            ToolName::ReadFile => self.execute_read_file(input).await,
            ToolName::WriteFile => self.execute_write_file(input).await,
            ToolName::EditFile => self.execute_edit_file(input).await,
            ToolName::ListDirectory => self.execute_list_directory(input).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_executor() -> (LocalExecutor, TempDir) {
        let temp_dir = tempfile::tempdir().unwrap();
        let executor = LocalExecutor::new(temp_dir.path().to_path_buf(), 30);
        (executor, temp_dir)
    }

    #[tokio::test]
    async fn test_bash_echo() {
        let (executor, _dir) = create_executor();
        let result = executor
            .execute(
                ToolName::Bash,
                serde_json::json!({ "command": "echo hello" }),
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "hello");
    }

    #[tokio::test]
    async fn test_bash_failure() {
        let (executor, _dir) = create_executor();
        let result = executor
            .execute(ToolName::Bash, serde_json::json!({ "command": "exit 1" }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_write_and_read_file() {
        let (executor, _dir) = create_executor();

        // Write
        let result = executor
            .execute(
                ToolName::WriteFile,
                serde_json::json!({ "path": "test.txt", "content": "hello world" }),
            )
            .await;
        assert!(result.is_ok());

        // Read
        let result = executor
            .execute(
                ToolName::ReadFile,
                serde_json::json!({ "path": "test.txt" }),
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello world");
    }

    #[tokio::test]
    async fn test_edit_file() {
        let (executor, _dir) = create_executor();

        // Write initial content
        executor
            .execute(
                ToolName::WriteFile,
                serde_json::json!({ "path": "test.txt", "content": "hello world" }),
            )
            .await
            .unwrap();

        // Edit
        let result = executor
            .execute(
                ToolName::EditFile,
                serde_json::json!({
                    "path": "test.txt",
                    "old_text": "world",
                    "new_text": "rust"
                }),
            )
            .await;
        assert!(result.is_ok());

        // Verify
        let result = executor
            .execute(
                ToolName::ReadFile,
                serde_json::json!({ "path": "test.txt" }),
            )
            .await;
        assert_eq!(result.unwrap(), "hello rust");
    }

    #[tokio::test]
    async fn test_edit_file_not_found() {
        let (executor, _dir) = create_executor();

        // Write initial content
        executor
            .execute(
                ToolName::WriteFile,
                serde_json::json!({ "path": "test.txt", "content": "hello world" }),
            )
            .await
            .unwrap();

        // Edit with non-existent text
        let result = executor
            .execute(
                ToolName::EditFile,
                serde_json::json!({
                    "path": "test.txt",
                    "old_text": "not found",
                    "new_text": "replacement"
                }),
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_list_directory() {
        let (executor, _dir) = create_executor();

        // Create some files
        executor
            .execute(
                ToolName::WriteFile,
                serde_json::json!({ "path": "file1.txt", "content": "a" }),
            )
            .await
            .unwrap();
        executor
            .execute(
                ToolName::WriteFile,
                serde_json::json!({ "path": "file2.txt", "content": "b" }),
            )
            .await
            .unwrap();

        // List
        let result = executor
            .execute(ToolName::ListDirectory, serde_json::json!({ "path": "." }))
            .await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("file1.txt"));
        assert!(output.contains("file2.txt"));
    }

    #[tokio::test]
    async fn test_path_traversal_blocked() {
        let (executor, _dir) = create_executor();

        let result = executor
            .execute(
                ToolName::ReadFile,
                serde_json::json!({ "path": "../../../etc/passwd" }),
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not allowed"));
    }

    #[tokio::test]
    async fn test_absolute_path_blocked() {
        let (executor, _dir) = create_executor();

        let result = executor
            .execute(
                ToolName::ReadFile,
                serde_json::json!({ "path": "/etc/passwd" }),
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Absolute paths"));
    }
}
