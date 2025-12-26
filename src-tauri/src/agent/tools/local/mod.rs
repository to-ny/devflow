mod context;
mod file;
mod notebook;
mod search;
mod shell;
mod state;
mod subagent;
mod web;

use std::path::PathBuf;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

pub use context::ExecutionContext;
pub use state::{PlanApproval, SessionState};

use super::executor::ToolExecutor;
use super::types::ToolName;
use crate::agent::error::AgentError;

#[cfg(windows)]
use crate::git::wsl::{is_wsl_path, parse_wsl_path};

pub struct LocalExecutor {
    ctx: ExecutionContext,
    session: SessionState,
    shell: shell::ShellExecutor,
    cancel_token: CancellationToken,
}

impl LocalExecutor {
    #[cfg(test)]
    pub fn new(working_dir: PathBuf, timeout_secs: u64) -> Self {
        Self::with_session(
            working_dir,
            timeout_secs,
            SessionState::new(),
            CancellationToken::new(),
        )
    }

    pub fn with_session(
        working_dir: PathBuf,
        timeout_secs: u64,
        session: SessionState,
        cancel_token: CancellationToken,
    ) -> Self {
        let ctx = ExecutionContext::new(working_dir.clone(), timeout_secs);

        #[cfg(windows)]
        let shell = {
            let wsl_path = if is_wsl_path(&working_dir) {
                parse_wsl_path(&working_dir)
            } else {
                None
            };
            shell::ShellExecutor::new(working_dir, ctx.timeout, wsl_path)
        };

        #[cfg(not(windows))]
        let shell = shell::ShellExecutor::new(working_dir, ctx.timeout);

        Self {
            ctx,
            session,
            shell,
            cancel_token,
        }
    }

    async fn execute_todo_read(&self) -> Result<String, AgentError> {
        let todos = self.session.get_todos().await;

        if todos.is_empty() {
            return Ok("No todos".to_string());
        }

        let output: Vec<String> = todos
            .iter()
            .map(|t| format!("[{}] {} ({})", t.status, t.content, t.priority))
            .collect();

        Ok(output.join("\n"))
    }

    async fn execute_todo_write(&self, input: serde_json::Value) -> Result<String, AgentError> {
        use super::types::TodoWriteInput;

        let input: TodoWriteInput = serde_json::from_value(input)
            .map_err(|e| AgentError::InvalidToolInput(format!("Invalid input: {}", e)))?;

        let count = input.todos.len();
        self.session.set_todos(input.todos).await;

        Ok(format!("Updated {} todos", count))
    }

    async fn execute_submit_plan(&self, input: serde_json::Value) -> Result<String, AgentError> {
        use super::types::SubmitPlanInput;

        let input: SubmitPlanInput = serde_json::from_value(input)
            .map_err(|e| AgentError::InvalidToolInput(format!("Invalid input: {}", e)))?;

        self.session.set_plan(input.plan.clone()).await;

        Ok("Plan submitted for review. Awaiting user approval.".to_string())
    }

    async fn execute_dispatch_agent(&self, input: serde_json::Value) -> Result<String, AgentError> {
        use super::types::DispatchAgentInput;
        use crate::config::ConfigService;

        let input: DispatchAgentInput = serde_json::from_value(input)
            .map_err(|e| AgentError::InvalidToolInput(format!("Invalid input: {}", e)))?;

        // Load config to get max_agent_depth
        let config = ConfigService::load_project_config(&self.ctx.working_dir)
            .map_err(|e| AgentError::ConfigError(e.to_string()))?;

        let max_depth = config.execution.max_agent_depth;

        // Execute the sub-agent with a child cancellation token
        subagent::execute_subagent(
            &self.ctx.working_dir,
            &input.task,
            input.tools,
            max_depth,
            0, // current_depth starts at 0
            &self.cancel_token,
        )
        .await
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
            ToolName::Bash => self.shell.execute(input).await,
            ToolName::ReadFile => file::read_file(&self.ctx, input).await,
            ToolName::WriteFile => file::write_file(&self.ctx, input).await,
            ToolName::EditFile => file::edit_file(&self.ctx, input).await,
            ToolName::MultiEdit => file::multi_edit(&self.ctx, input).await,
            ToolName::ListDirectory => file::list_directory(&self.ctx, input).await,
            ToolName::Glob => file::glob(&self.ctx, input).await,
            ToolName::Grep => file::grep(&self.ctx, input).await,
            ToolName::NotebookRead => notebook::read(&self.ctx, input).await,
            ToolName::NotebookEdit => notebook::edit(&self.ctx, input).await,
            ToolName::TodoRead => self.execute_todo_read().await,
            ToolName::TodoWrite => self.execute_todo_write(input).await,
            ToolName::WebFetch => web::fetch(&self.ctx, input).await,
            ToolName::SearchWeb => search::search(&self.ctx, input).await,
            ToolName::SubmitPlan => self.execute_submit_plan(input).await,
            ToolName::DispatchAgent => self.execute_dispatch_agent(input).await,
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

        let result = executor
            .execute(
                ToolName::WriteFile,
                serde_json::json!({ "path": "test.txt", "content": "hello world" }),
            )
            .await;
        assert!(result.is_ok());

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

        executor
            .execute(
                ToolName::WriteFile,
                serde_json::json!({ "path": "test.txt", "content": "hello world" }),
            )
            .await
            .unwrap();

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

        executor
            .execute(
                ToolName::WriteFile,
                serde_json::json!({ "path": "test.txt", "content": "hello world" }),
            )
            .await
            .unwrap();

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

    #[tokio::test]
    async fn test_session_state_shared() {
        let session = SessionState::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let executor = LocalExecutor::with_session(
            temp_dir.path().to_path_buf(),
            30,
            session.clone(),
            CancellationToken::new(),
        );

        executor
            .execute(
                ToolName::TodoWrite,
                serde_json::json!({
                    "todos": [{
                        "id": "1",
                        "content": "Test task",
                        "status": "pending",
                        "priority": "high"
                    }]
                }),
            )
            .await
            .unwrap();

        assert_eq!(session.todos_count().await, 1);
    }
}
