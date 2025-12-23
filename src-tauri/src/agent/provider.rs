use async_trait::async_trait;
use tauri::AppHandle;

use super::error::AgentError;
use super::types::ChatMessage;

#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    async fn send_message(
        &self,
        messages: Vec<ChatMessage>,
        system_prompt: Option<String>,
        app_handle: AppHandle,
    ) -> Result<String, AgentError>;

    fn model(&self) -> &str;
}
