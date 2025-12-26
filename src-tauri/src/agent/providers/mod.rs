pub mod anthropic;
pub mod gemini;
pub mod headless;

pub use headless::run_headless_loop;

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

pub const DEFAULT_SYSTEM_PROMPT: &str = include_str!("../default_system_prompt.md");

pub(crate) fn emit_status(app_handle: &AppHandle, status: AgentStatus, detail: Option<String>) {
    let _ = app_handle.emit("agent-status", AgentStatusPayload::new(status, detail));
}

pub(crate) fn build_system_prompt(
    app_system_prompt: &str,
    prompts: &PromptsConfig,
    custom: Option<String>,
) -> String {
    let mut parts = Vec::new();

    parts.push(app_system_prompt.to_string());

    if !prompts.pre.is_empty() {
        parts.push(prompts.pre.clone());
    }

    if let Some(custom) = custom {
        parts.push(custom);
    }

    if !prompts.post.is_empty() {
        parts.push(prompts.post.clone());
    }

    parts.join("\n\n")
}

use tokio_util::sync::CancellationToken;

use super::tools::SessionState;

pub(crate) fn create_executor(
    project_path: &Path,
    execution: &ExecutionConfig,
    session: SessionState,
    cancel_token: CancellationToken,
) -> LocalExecutor {
    LocalExecutor::with_session(
        project_path.to_path_buf(),
        execution.timeout_secs,
        session,
        cancel_token,
    )
}

use super::types::{PlanReadyPayload, ToolEndPayload, ToolStartPayload};

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
    session: &SessionState,
    app_handle: &AppHandle,
    cancel_token: &CancellationToken,
) -> Result<Vec<ToolResult>, AgentError> {
    use super::tools::{ToolExecutor, ToolName};

    let mut results = Vec::new();

    for call in tool_calls {
        if cancel_token.is_cancelled() {
            return Err(AgentError::Cancelled);
        }

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

        let _ = app_handle.emit(
            "agent-tool-end",
            ToolEndPayload {
                tool_use_id: call.id.clone(),
                output: output.clone(),
                is_error,
            },
        );

        if tool_name == ToolName::SubmitPlan && !is_error {
            if let Some(plan) = session.get_plan().await {
                let _ =
                    app_handle.emit("agent-plan-ready", PlanReadyPayload { plan: plan.clone() });

                // Wait for user approval
                emit_status(
                    app_handle,
                    AgentStatus::ToolWaiting,
                    Some("Awaiting plan approval".to_string()),
                );

                // Wait for approval/rejection from user
                if let Some(approval) = session.wait_for_plan_approval().await {
                    use super::tools::PlanApproval;

                    match approval {
                        PlanApproval::Approved => {
                            results.push(ToolResult {
                                id: call.id,
                                name: call.name,
                                output: "Plan approved by user. Proceed with implementation."
                                    .to_string(),
                                is_error: false,
                            });
                        }
                        PlanApproval::Rejected(reason) => {
                            let rejection_msg = match reason {
                                Some(r) => format!("Plan rejected by user: {}", r),
                                None => "Plan rejected by user.".to_string(),
                            };
                            results.push(ToolResult {
                                id: call.id,
                                name: call.name,
                                output: rejection_msg,
                                is_error: true,
                            });
                        }
                    }
                } else {
                    // No pending plan (shouldn't happen)
                    results.push(ToolResult {
                        id: call.id,
                        name: call.name,
                        output,
                        is_error,
                    });
                }
                continue;
            }
        }

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
                DEFAULT_SYSTEM_PROMPT,
            )?;
            Ok(Arc::new(adapter))
        }
        "gemini" => {
            let adapter = GeminiAdapter::new(
                project_config.agent,
                project_config.prompts,
                project_config.execution,
                project_path.to_path_buf(),
                DEFAULT_SYSTEM_PROMPT,
            )?;
            Ok(Arc::new(adapter))
        }
        _ => Err(AgentError::UnsupportedProvider(provider)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_system_prompt_includes_app_prompt() {
        let prompts = PromptsConfig::default();
        let result = build_system_prompt("App prompt", &prompts, None);
        assert_eq!(result, "App prompt");
    }

    #[test]
    fn test_build_system_prompt_combination_order() {
        let prompts = PromptsConfig {
            pre: "Pre prompt".to_string(),
            post: "Post prompt".to_string(),
        };
        let result = build_system_prompt("App prompt", &prompts, Some("Custom prompt".to_string()));
        assert_eq!(
            result,
            "App prompt\n\nPre prompt\n\nCustom prompt\n\nPost prompt"
        );
    }

    #[test]
    fn test_build_system_prompt_skips_empty_parts() {
        let prompts = PromptsConfig {
            pre: "".to_string(),
            post: "Post prompt".to_string(),
        };
        let result = build_system_prompt("App prompt", &prompts, None);
        assert_eq!(result, "App prompt\n\nPost prompt");
    }

    #[test]
    fn test_build_system_prompt_joins_with_double_newline() {
        let prompts = PromptsConfig {
            pre: "Pre".to_string(),
            post: "".to_string(),
        };
        let result = build_system_prompt("App", &prompts, None);
        assert!(result.contains("\n\n"));
        assert_eq!(result, "App\n\nPre");
    }

    #[test]
    fn test_default_system_prompt_is_not_empty() {
        assert!(!DEFAULT_SYSTEM_PROMPT.is_empty());
        assert!(DEFAULT_SYSTEM_PROMPT.len() > 50); // Should have meaningful content
    }
}
