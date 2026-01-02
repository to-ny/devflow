pub mod anthropic;
pub mod compaction;
pub mod gemini;
pub mod headless;

#[cfg(test)]
pub mod mock;

// Compaction utilities are used internally by anthropic and gemini adapters via `super::compaction::`
pub use headless::{
    run_headless_loop, HeadlessContext, HeadlessResponse, HeadlessStreamer,
    ToolResult as HeadlessToolResult,
};

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

use tokio_util::sync::CancellationToken;

pub const DEFAULT_SYSTEM_PROMPT: &str = include_str!("../default_system_prompt.md");
pub const DEFAULT_EXTRACTION_PROMPT: &str = include_str!("../extraction_prompt.md");

/// Context for streaming responses, reducing parameter passing.
pub(crate) struct StreamContext<'a> {
    pub app_handle: &'a AppHandle,
    pub cancel_token: &'a CancellationToken,
    pub block_offset: u32,
}

/// Tracks global block indices across multiple streaming responses in a tool loop.
pub(crate) struct StreamingState {
    global_block_counter: u32,
}

impl StreamingState {
    pub fn new() -> Self {
        Self {
            global_block_counter: 0,
        }
    }

    pub fn create_context<'a>(
        &self,
        app_handle: &'a AppHandle,
        cancel_token: &'a CancellationToken,
    ) -> StreamContext<'a> {
        StreamContext {
            app_handle,
            cancel_token,
            block_offset: self.global_block_counter,
        }
    }

    pub fn advance(&mut self, block_count: u32) {
        self.global_block_counter += block_count;
    }

    #[cfg(test)]
    pub fn current_offset(&self) -> u32 {
        self.global_block_counter
    }
}

pub(crate) fn emit_status(app_handle: &AppHandle, status: AgentStatus, detail: Option<String>) {
    let _ = app_handle.emit("agent-status", AgentStatusPayload::new(status, detail));
}

pub(crate) fn build_system_prompt(
    app_system_prompt: &str,
    prompts: &PromptsConfig,
    custom: Option<String>,
    memory: Option<&str>,
) -> String {
    let mut parts = Vec::new();

    parts.push(app_system_prompt.to_string());

    // Memory content (AGENTS.md) comes right after base prompt
    if let Some(memory_content) = memory {
        parts.push(memory_content.to_string());
    }

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

use super::tools::SessionState;
use super::usage::{AgentUsagePayload, SessionUsageTracker, TokenUsage, UsageSource};

pub(crate) fn emit_usage(
    app_handle: &AppHandle,
    tracker: &SessionUsageTracker,
    usage: TokenUsage,
    source: UsageSource,
) {
    if usage.input_tokens > 0 || usage.output_tokens > 0 {
        let totals = tracker.add_tokens(usage.input_tokens, usage.output_tokens);
        let _ = app_handle.emit(
            "agent-usage",
            AgentUsagePayload {
                input_tokens: totals.input_tokens,
                output_tokens: totals.output_tokens,
                source,
            },
        );
    }
}

pub(crate) fn create_executor(
    project_path: &Path,
    execution: &ExecutionConfig,
    session: SessionState,
    cancel_token: CancellationToken,
    usage_tracker: Arc<SessionUsageTracker>,
) -> LocalExecutor {
    LocalExecutor::with_session(
        project_path.to_path_buf(),
        execution.timeout_secs,
        session,
        cancel_token,
        usage_tracker,
    )
}

use super::types::{PlanReadyPayload, ToolEndPayload, ToolStartPayload};

pub(crate) struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
    pub block_index: u32,
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
                block_index: call.block_index,
            },
        );

        let tool_name = ToolName::parse(&call.name)
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
                    block_index: call.block_index,
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
                block_index: call.block_index,
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
                project_config.extraction_prompt,
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
                project_config.extraction_prompt,
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
        let result = build_system_prompt("App prompt", &prompts, None, None);
        assert_eq!(result, "App prompt");
    }

    #[test]
    fn test_build_system_prompt_combination_order() {
        let prompts = PromptsConfig {
            pre: "Pre prompt".to_string(),
            post: "Post prompt".to_string(),
        };
        let result = build_system_prompt(
            "App prompt",
            &prompts,
            Some("Custom prompt".to_string()),
            None,
        );
        assert_eq!(
            result,
            "App prompt\n\nPre prompt\n\nCustom prompt\n\nPost prompt"
        );
    }

    #[test]
    fn test_build_system_prompt_with_memory() {
        let prompts = PromptsConfig {
            pre: "Pre prompt".to_string(),
            post: "Post prompt".to_string(),
        };
        let memory = "<project-memory source=\"AGENTS.md\">\nTest memory\n</project-memory>";
        let result = build_system_prompt("App prompt", &prompts, None, Some(memory));
        assert_eq!(
            result,
            "App prompt\n\n<project-memory source=\"AGENTS.md\">\nTest memory\n</project-memory>\n\nPre prompt\n\nPost prompt"
        );
    }

    #[test]
    fn test_build_system_prompt_skips_empty_parts() {
        let prompts = PromptsConfig {
            pre: "".to_string(),
            post: "Post prompt".to_string(),
        };
        let result = build_system_prompt("App prompt", &prompts, None, None);
        assert_eq!(result, "App prompt\n\nPost prompt");
    }

    #[test]
    fn test_build_system_prompt_joins_with_double_newline() {
        let prompts = PromptsConfig {
            pre: "Pre".to_string(),
            post: "".to_string(),
        };
        let result = build_system_prompt("App", &prompts, None, None);
        assert!(result.contains("\n\n"));
        assert_eq!(result, "App\n\nPre");
    }

    #[test]
    fn test_default_system_prompt_is_not_empty() {
        assert!(!DEFAULT_SYSTEM_PROMPT.is_empty());
        assert!(DEFAULT_SYSTEM_PROMPT.len() > 50); // Should have meaningful content
    }

    #[test]
    fn test_streaming_state_starts_at_zero() {
        let state = StreamingState::new();
        assert_eq!(state.current_offset(), 0);
    }

    #[test]
    fn test_streaming_state_advances_correctly() {
        let mut state = StreamingState::new();

        // First response: 3 blocks
        assert_eq!(state.current_offset(), 0);
        state.advance(3);

        // Second response: 2 blocks
        assert_eq!(state.current_offset(), 3);
        state.advance(2);

        // Third response: 1 block
        assert_eq!(state.current_offset(), 5);
        state.advance(1);
        assert_eq!(state.current_offset(), 6);
    }

    #[test]
    fn test_streaming_state_handles_empty_responses() {
        let mut state = StreamingState::new();

        state.advance(0);
        assert_eq!(state.current_offset(), 0);

        state.advance(2);
        state.advance(0);
        assert_eq!(state.current_offset(), 2);
    }

    #[test]
    fn test_streaming_state_simulates_tool_loop() {
        let mut state = StreamingState::new();

        // Turn 1: text (0), tool (1)
        assert_eq!(state.current_offset(), 0);
        state.advance(2);

        // Turn 2: text (2), tool (3)
        assert_eq!(state.current_offset(), 2);
        state.advance(2);

        // Turn 3: final text (4)
        assert_eq!(state.current_offset(), 4);
        state.advance(1);

        assert_eq!(state.current_offset(), 5);
    }
}
