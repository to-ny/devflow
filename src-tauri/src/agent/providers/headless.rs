use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::agent::error::AgentError;
use crate::agent::provider::HeadlessResult;
use crate::agent::tools::{ToolExecutor, ToolName};
use crate::agent::types::ToolDefinition;
use crate::agent::usage::{SessionUsageTracker, TokenUsage};

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub id: String,
    pub name: String,
    pub output: String,
    pub is_error: bool,
}

#[derive(Debug, Default)]
pub struct HeadlessResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCall>,
    pub usage: TokenUsage,
}

/// Execution context for headless (sub-agent) runs.
pub struct HeadlessContext<'a> {
    pub system_prompt: Option<String>,
    pub tools: Vec<ToolDefinition>,
    pub executor: &'a dyn ToolExecutor,
    pub max_iterations: u32,
    pub cancel_token: &'a CancellationToken,
    pub usage_tracker: Arc<SessionUsageTracker>,
}

impl HeadlessResponse {
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }
}

#[async_trait]
pub trait HeadlessStreamer: Send + Sync {
    type Conversation: Send;

    fn initial_conversation(
        &self,
        messages: Vec<crate::agent::types::ChatMessage>,
    ) -> Self::Conversation;

    async fn stream_response(
        &self,
        conversation: &Self::Conversation,
        system_prompt: Option<String>,
        tools: &[ToolDefinition],
        cancel_token: &CancellationToken,
    ) -> Result<HeadlessResponse, AgentError>;

    fn append_assistant_response(
        &self,
        conversation: &mut Self::Conversation,
        response: &HeadlessResponse,
    );

    fn append_tool_results(&self, conversation: &mut Self::Conversation, results: Vec<ToolResult>);
}

pub async fn run_headless_loop<S: HeadlessStreamer>(
    streamer: &S,
    messages: Vec<crate::agent::types::ChatMessage>,
    ctx: HeadlessContext<'_>,
) -> Result<HeadlessResult, AgentError> {
    let mut conversation = streamer.initial_conversation(messages);
    let mut iteration = 0u32;
    let mut final_text = String::new();

    loop {
        if ctx.cancel_token.is_cancelled() {
            return Err(AgentError::Cancelled);
        }

        let response = streamer
            .stream_response(
                &conversation,
                ctx.system_prompt.clone(),
                &ctx.tools,
                ctx.cancel_token,
            )
            .await?;

        let usage = response.usage;
        if usage.input_tokens > 0 || usage.output_tokens > 0 {
            ctx.usage_tracker
                .add_tokens(usage.input_tokens, usage.output_tokens);
        }

        final_text.push_str(&response.text);

        if !response.has_tool_calls() {
            return Ok(HeadlessResult {
                text: final_text,
                tool_calls_made: iteration,
            });
        }

        iteration += 1;
        if iteration > ctx.max_iterations {
            return Err(AgentError::ToolExecutionError(format!(
                "Exceeded maximum tool iterations ({})",
                ctx.max_iterations
            )));
        }

        streamer.append_assistant_response(&mut conversation, &response);

        let mut results = Vec::new();
        for tc in &response.tool_calls {
            let tool_name = ToolName::from_str(&tc.name)
                .ok_or_else(|| AgentError::UnknownTool(tc.name.clone()))?;

            let (output, is_error) = match ctx.executor.execute(tool_name, tc.input.clone()).await {
                Ok(output) => (output, false),
                Err(e) => (e.to_string(), true),
            };

            results.push(ToolResult {
                id: tc.id.clone(),
                name: tc.name.clone(),
                output,
                is_error,
            });
        }

        streamer.append_tool_results(&mut conversation, results);
    }
}
