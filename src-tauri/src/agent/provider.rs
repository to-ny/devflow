use std::sync::Arc;

use async_trait::async_trait;
use tauri::AppHandle;
use tokio_util::sync::CancellationToken;

use super::error::AgentError;
use super::tools::SessionState;
use super::types::{ChatMessage, ToolDefinition};
use super::usage::SessionUsageTracker;

/// Result from headless execution
pub struct HeadlessResult {
    pub text: String,
    pub tool_calls_made: u32,
}

/// Runtime context for agent execution
pub struct ExecutionContext {
    pub session: SessionState,
    pub cancel_token: CancellationToken,
    pub usage_tracker: Arc<SessionUsageTracker>,
}

#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    /// Send a message with full UI integration (events, streaming)
    async fn send_message(
        &self,
        messages: Vec<ChatMessage>,
        system_prompt: Option<String>,
        memory: Option<String>,
        ctx: ExecutionContext,
        app_handle: AppHandle,
    ) -> Result<(), AgentError>;

    /// Run headless without UI - for sub-agents
    async fn run_headless(
        &self,
        messages: Vec<ChatMessage>,
        system_prompt: Option<String>,
        memory: Option<String>,
        tools: Vec<ToolDefinition>,
        ctx: ExecutionContext,
    ) -> Result<HeadlessResult, AgentError>;

    fn model(&self) -> &str;
}
