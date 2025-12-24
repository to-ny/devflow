pub mod anthropic;
pub mod gemini;

use std::path::Path;
use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use crate::config::{ConfigService, ExecutionConfig, PromptsConfig};

use super::error::AgentError;
use super::provider::ProviderAdapter;
use super::tools::LocalExecutor;
use super::types::{AgentStatus, AgentStatusPayload};

pub use anthropic::AnthropicAdapter;
pub use gemini::GeminiAdapter;

pub(crate) fn emit_status(app_handle: &AppHandle, status: AgentStatus, detail: Option<String>) {
    let _ = app_handle.emit("agent-status", AgentStatusPayload::new(status, detail));
}

pub(crate) fn build_system_prompt(
    prompts: &PromptsConfig,
    custom: Option<String>,
) -> Option<String> {
    let mut parts = Vec::new();

    if !prompts.pre.is_empty() {
        parts.push(prompts.pre.clone());
    }

    if let Some(custom) = custom {
        parts.push(custom);
    }

    if !prompts.post.is_empty() {
        parts.push(prompts.post.clone());
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

pub(crate) fn create_executor(project_path: &Path, execution: &ExecutionConfig) -> LocalExecutor {
    LocalExecutor::new(project_path.to_path_buf(), execution.timeout_secs)
}

use log::{info, warn};
use tokio_util::sync::CancellationToken;

use super::types::{ToolEndPayload, ToolStartPayload};

pub(crate) struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

pub(crate) struct ToolResult {
    pub id: String,
    pub name: String,
    pub output: String,
    pub is_error: bool,
}

pub(crate) async fn execute_tool_calls(
    tool_calls: Vec<ToolCall>,
    executor: &LocalExecutor,
    app_handle: &AppHandle,
    cancel_token: &CancellationToken,
) -> Result<Vec<ToolResult>, AgentError> {
    use super::tools::{ToolExecutor, ToolName};

    let mut results = Vec::new();

    for call in tool_calls {
        if cancel_token.is_cancelled() {
            warn!("Tool execution cancelled by user");
            return Err(AgentError::Cancelled);
        }

        info!("Executing tool: {} (id: {})", call.name, call.id);

        emit_status(
            app_handle,
            AgentStatus::ToolRunning,
            Some(call.name.clone()),
        );

        let _ = app_handle.emit(
            "agent-tool-start",
            ToolStartPayload {
                tool_use_id: call.id.clone(),
                tool_name: call.name.clone(),
                tool_input: call.input.clone(),
            },
        );

        let tool_name = ToolName::from_str(&call.name)
            .ok_or_else(|| AgentError::UnknownTool(call.name.clone()))?;

        let (output, is_error) = tokio::select! {
            _ = cancel_token.cancelled() => {
                warn!("Tool {} cancelled by user", call.name);
                ("Cancelled by user".to_string(), true)
            }
            result = executor.execute(tool_name, call.input.clone()) => {
                match result {
                    Ok(result) => (result, false),
                    Err(e) => (e.to_string(), true),
                }
            }
        };

        if cancel_token.is_cancelled() {
            let _ = app_handle.emit(
                "agent-tool-end",
                ToolEndPayload {
                    tool_use_id: call.id.clone(),
                    output: "Cancelled by user".to_string(),
                    is_error: true,
                },
            );
            return Err(AgentError::Cancelled);
        }

        info!(
            "Tool {} completed (error: {}, output: {} chars)",
            call.name,
            is_error,
            output.len()
        );

        let _ = app_handle.emit(
            "agent-tool-end",
            ToolEndPayload {
                tool_use_id: call.id.clone(),
                output: output.clone(),
                is_error,
            },
        );

        results.push(ToolResult {
            id: call.id,
            name: call.name,
            output,
            is_error,
        });
    }

    Ok(results)
}

pub(crate) fn check_iteration_limit(iteration: u32, max_iterations: u32) -> Result<(), AgentError> {
    if iteration >= max_iterations {
        return Err(AgentError::ToolExecutionError(format!(
            "Exceeded maximum tool iterations ({})",
            max_iterations
        )));
    }
    Ok(())
}

pub fn create_provider_adapter(
    project_path: &Path,
) -> Result<Arc<dyn ProviderAdapter>, AgentError> {
    let project_config = ConfigService::load_project_config(project_path)
        .map_err(|e| AgentError::ConfigError(e.to_string()))?;

    let provider = project_config.agent.provider.to_lowercase();

    match provider.as_str() {
        "anthropic" => {
            let adapter = AnthropicAdapter::new(
                project_config.agent,
                project_config.prompts,
                project_config.execution,
                project_path.to_path_buf(),
            )?;
            Ok(Arc::new(adapter))
        }
        "gemini" => {
            let adapter = GeminiAdapter::new(
                project_config.agent,
                project_config.prompts,
                project_config.execution,
                project_path.to_path_buf(),
            )?;
            Ok(Arc::new(adapter))
        }
        _ => Err(AgentError::UnsupportedProvider(provider)),
    }
}
