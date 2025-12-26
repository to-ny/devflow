use async_trait::async_trait;
use tauri::AppHandle;
use tokio_util::sync::CancellationToken;

use super::error::AgentError;
use super::tools::SessionState;
use super::types::ChatMessage;

#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    async fn send_message(
        &self,
        messages: Vec<ChatMessage>,
        system_prompt: Option<String>,
        session: SessionState,
        app_handle: AppHandle,
        cancel_token: CancellationToken,
    ) -> Result<(), AgentError>;

    fn model(&self) -> &str;
}
