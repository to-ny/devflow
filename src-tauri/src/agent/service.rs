use std::env;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use log::{debug, error, info};
use reqwest::Client;
use serde_json;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::config::{AgentConfig, ConfigService, PromptsConfig};

use super::anthropic::{
    AnthropicErrorResponse, AnthropicEvent, AnthropicMessage, AnthropicRequest, ContentDelta,
};
use super::error::AgentError;
use super::provider::ProviderAdapter;
use super::types::{AgentChunkPayload, AgentCompletePayload, AgentErrorPayload, ChatMessage};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

pub struct AnthropicAdapter {
    client: Client,
    config: AgentConfig,
    prompts: PromptsConfig,
    api_key: String,
}

impl AnthropicAdapter {
    fn new(config: AgentConfig, prompts: PromptsConfig, api_key: String) -> Self {
        Self {
            client: Client::new(),
            config,
            prompts,
            api_key,
        }
    }

    fn build_system_prompt(&self, custom: Option<String>) -> Option<String> {
        let mut parts = Vec::new();

        if !self.prompts.pre.is_empty() {
            parts.push(self.prompts.pre.clone());
        }

        if let Some(custom) = custom {
            parts.push(custom);
        }

        if !self.prompts.post.is_empty() {
            parts.push(self.prompts.post.clone());
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n\n"))
        }
    }

    fn process_sse_event(&self, event_data: &str, app_handle: &AppHandle) -> Option<SseResult> {
        let mut event_type = None;
        let mut data = None;

        for line in event_data.lines() {
            if let Some(suffix) = line.strip_prefix("event: ") {
                event_type = Some(suffix.to_string());
            } else if let Some(suffix) = line.strip_prefix("data: ") {
                data = Some(suffix.to_string());
            }
        }

        let data = data?;

        if data == "[DONE]" {
            return Some(SseResult::Done);
        }

        let event: AnthropicEvent = match serde_json::from_str(&data) {
            Ok(e) => e,
            Err(e) => {
                debug!("Failed to parse SSE event: {} - data: {}", e, data);
                return None;
            }
        };

        match event {
            AnthropicEvent::ContentBlockDelta { delta, .. } => {
                let ContentDelta::TextDelta { text } = delta;
                let _ = app_handle.emit(
                    "agent-chunk",
                    AgentChunkPayload {
                        delta: text.clone(),
                    },
                );
                Some(SseResult::Delta(text))
            }
            AnthropicEvent::MessageDelta { delta, .. } => {
                delta.stop_reason.map(SseResult::StopReason)
            }
            AnthropicEvent::Error { error } => Some(SseResult::Error(error.message)),
            AnthropicEvent::MessageStop => Some(SseResult::Done),
            _ => {
                debug!("Ignoring SSE event type: {:?}", event_type);
                None
            }
        }
    }
}

#[async_trait]
impl ProviderAdapter for AnthropicAdapter {
    async fn send_message(
        &self,
        messages: Vec<ChatMessage>,
        system_prompt: Option<String>,
        app_handle: AppHandle,
    ) -> Result<String, AgentError> {
        let anthropic_messages: Vec<AnthropicMessage> =
            messages.iter().map(AnthropicMessage::from).collect();

        let system = self.build_system_prompt(system_prompt);

        let request = AnthropicRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            messages: anthropic_messages,
            stream: true,
            system,
        };

        info!(
            "Sending message to Anthropic API (model: {})",
            self.config.model
        );

        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("Anthropic API error: {} - {}", status, body);

            if let Ok(error_response) = serde_json::from_str::<AnthropicErrorResponse>(&body) {
                return Err(AgentError::ApiError(error_response.error.message));
            }
            return Err(AgentError::ApiError(format!("{}: {}", status, body)));
        }

        let message_id = Uuid::new_v4().to_string();
        let mut full_content = String::new();
        let mut stop_reason: Option<String> = None;
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(event_end) = buffer.find("\n\n") {
                let event_data = buffer[..event_end].to_string();
                buffer = buffer[event_end + 2..].to_string();

                if let Some(result) = self.process_sse_event(&event_data, &app_handle) {
                    match result {
                        SseResult::Delta(delta) => {
                            full_content.push_str(&delta);
                        }
                        SseResult::StopReason(reason) => {
                            stop_reason = Some(reason);
                        }
                        SseResult::Error(err) => {
                            let _ = app_handle
                                .emit("agent-error", AgentErrorPayload { error: err.clone() });
                            return Err(AgentError::ApiError(err));
                        }
                        SseResult::Done => break,
                    }
                }
            }
        }

        info!("Anthropic response complete ({} chars)", full_content.len());

        let _ = app_handle.emit(
            "agent-complete",
            AgentCompletePayload {
                message_id,
                full_content: full_content.clone(),
                stop_reason,
            },
        );

        Ok(full_content)
    }

    fn model(&self) -> &str {
        &self.config.model
    }
}

enum SseResult {
    Delta(String),
    StopReason(String),
    Error(String),
    Done,
}

pub fn create_provider_adapter(
    project_path: &Path,
) -> Result<Arc<dyn ProviderAdapter>, AgentError> {
    let project_config = ConfigService::load_project_config(project_path)
        .map_err(|e| AgentError::ConfigError(e.to_string()))?;

    let provider = project_config.agent.provider.to_lowercase();

    match provider.as_str() {
        "anthropic" => {
            let api_key = env::var(&project_config.agent.api_key_env)
                .map_err(|_| AgentError::MissingApiKey(project_config.agent.api_key_env.clone()))?;

            Ok(Arc::new(AnthropicAdapter::new(
                project_config.agent,
                project_config.prompts,
                api_key,
            )))
        }
        _ => Err(AgentError::UnsupportedProvider(provider)),
    }
}

pub struct AgentState {
    pub adapter: Option<Arc<dyn ProviderAdapter>>,
    pub project_path: Option<String>,
}

impl AgentState {
    pub fn new() -> Self {
        Self {
            adapter: None,
            project_path: None,
        }
    }

    pub fn initialize(&mut self, project_path: &str) -> Result<(), AgentError> {
        let path = Path::new(project_path);
        let adapter = create_provider_adapter(path)?;
        self.adapter = Some(adapter);
        self.project_path = Some(project_path.to_string());
        Ok(())
    }

    pub fn get_adapter(&self) -> Option<Arc<dyn ProviderAdapter>> {
        self.adapter.clone()
    }

    pub fn clear(&mut self) {
        self.adapter = None;
        self.project_path = None;
    }
}

impl Default for AgentState {
    fn default() -> Self {
        Self::new()
    }
}
