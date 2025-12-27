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

#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    /// Send a message with full UI integration (events, streaming)
    async fn send_message(
        &self,
        messages: Vec<ChatMessage>,
        system_prompt: Option<String>,
        session: SessionState,
        app_handle: AppHandle,
        cancel_token: CancellationToken,
        usage_tracker: Arc<SessionUsageTracker>,
    ) -> Result<(), AgentError>;

    /// Run headless without UI - for sub-agents
    /// Returns the final text response after tool execution loop
    async fn run_headless(
        &self,
        messages: Vec<ChatMessage>,
        system_prompt: Option<String>,
        tools: Vec<ToolDefinition>,
        session: SessionState,
        cancel_token: CancellationToken,
        usage_tracker: Arc<SessionUsageTracker>,
    ) -> Result<HeadlessResult, AgentError>;

    fn model(&self) -> &str;
}
