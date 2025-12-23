use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use log::{debug, error, info};
use reqwest::Client;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::config::{AgentConfig, ConfigService, ExecutionConfig, PromptsConfig};

use super::anthropic::{
    AnthropicErrorResponse, AnthropicEvent, AnthropicMessage, AnthropicRequest, ContentBlock,
    ContentDelta, MessageContent, StreamedResponse,
};
use super::error::AgentError;
use super::provider::ProviderAdapter;
use super::tools::{get_tool_definitions, LocalExecutor, ToolExecutor, ToolName};
use super::types::{
    AgentChunkPayload, AgentCompletePayload, AgentErrorPayload, ChatMessage, ToolEndPayload,
    ToolStartPayload,
};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

pub struct AnthropicAdapter {
    client: Client,
    config: AgentConfig,
    prompts: PromptsConfig,
    execution: ExecutionConfig,
    api_key: String,
    project_path: PathBuf,
}

impl AnthropicAdapter {
    fn new(
        config: AgentConfig,
        prompts: PromptsConfig,
        execution: ExecutionConfig,
        api_key: String,
        project_path: PathBuf,
    ) -> Self {
        Self {
            client: Client::new(),
            config,
            prompts,
            execution,
            api_key,
            project_path,
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

    fn create_executor(&self) -> LocalExecutor {
        LocalExecutor::new(self.project_path.clone(), self.execution.timeout_secs)
    }

    async fn stream_response(
        &self,
        messages: &[AnthropicMessage],
        system: Option<String>,
        app_handle: &AppHandle,
    ) -> Result<StreamedResponse, AgentError> {
        let tools = get_tool_definitions();

        let request = AnthropicRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            messages: messages.to_vec(),
            stream: true,
            system,
            tools: Some(tools),
        };

        info!(
            "Sending message to Anthropic API (model: {}, {} messages)",
            self.config.model,
            messages.len()
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

        let mut streamed = StreamedResponse::new();
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(event_end) = buffer.find("\n\n") {
                let event_data = buffer[..event_end].to_string();
                buffer = buffer[event_end + 2..].to_string();

                self.process_sse_event(&event_data, app_handle, &mut streamed);
            }
        }

        Ok(streamed)
    }

    fn process_sse_event(
        &self,
        event_data: &str,
        app_handle: &AppHandle,
        streamed: &mut StreamedResponse,
    ) {
        let mut data = None;

        for line in event_data.lines() {
            if let Some(suffix) = line.strip_prefix("data: ") {
                data = Some(suffix.to_string());
            }
        }

        let Some(data) = data else { return };

        if data == "[DONE]" {
            return;
        }

        let event: AnthropicEvent = match serde_json::from_str(&data) {
            Ok(e) => e,
            Err(e) => {
                debug!("Failed to parse SSE event: {} - data: {}", e, data);
                return;
            }
        };

        match event {
            AnthropicEvent::ContentBlockStart {
                index,
                content_block,
            } => {
                streamed.on_content_block_start(index, content_block);
            }
            AnthropicEvent::ContentBlockDelta { index, delta } => {
                if let ContentDelta::TextDelta { ref text } = delta {
                    let _ = app_handle.emit(
                        "agent-chunk",
                        AgentChunkPayload {
                            delta: text.clone(),
                        },
                    );
                }
                streamed.on_content_delta(index, delta);
            }
            AnthropicEvent::ContentBlockStop { index } => {
                streamed.on_content_block_stop(index);
            }
            AnthropicEvent::MessageDelta { delta, .. } => {
                streamed.on_message_delta(delta);
            }
            AnthropicEvent::Error { error } => {
                let _ = app_handle.emit(
                    "agent-error",
                    AgentErrorPayload {
                        error: error.message,
                    },
                );
            }
            _ => {}
        }
    }

    async fn execute_tool_loop(
        &self,
        initial_messages: Vec<AnthropicMessage>,
        system_prompt: Option<String>,
        app_handle: &AppHandle,
    ) -> Result<Option<String>, AgentError> {
        let executor = self.create_executor();
        let mut conversation = initial_messages;
        let max_iterations = self.execution.max_tool_iterations;
        let mut iteration = 0u32;

        loop {
            let response = self
                .stream_response(&conversation, system_prompt.clone(), app_handle)
                .await?;

            if !response.has_tool_use() {
                info!(
                    "Agent response complete (stop_reason: {:?})",
                    response.stop_reason
                );
                return Ok(response.stop_reason);
            }

            iteration += 1;
            if iteration >= max_iterations {
                return Err(AgentError::ToolExecutionError(format!(
                    "Exceeded maximum tool iterations ({})",
                    max_iterations
                )));
            }

            conversation.push(AnthropicMessage {
                role: "assistant".to_string(),
                content: MessageContent::Blocks(response.content_blocks.clone()),
            });

            let mut tool_results = Vec::new();

            for block in &response.content_blocks {
                if let ContentBlock::ToolUse { id, name, input } = block {
                    info!("Executing tool: {} (id: {})", name, id);

                    let _ = app_handle.emit(
                        "agent-tool-start",
                        ToolStartPayload {
                            tool_use_id: id.clone(),
                            tool_name: name.clone(),
                            tool_input: serde_json::to_string(input).unwrap_or_default(),
                        },
                    );

                    let tool_name = ToolName::from_str(name)
                        .ok_or_else(|| AgentError::UnknownTool(name.clone()))?;

                    let (output, is_error) = match executor.execute(tool_name, input.clone()).await
                    {
                        Ok(result) => (result, false),
                        Err(e) => (e.to_string(), true),
                    };

                    info!(
                        "Tool {} completed (error: {}, output: {} chars)",
                        name,
                        is_error,
                        output.len()
                    );

                    let _ = app_handle.emit(
                        "agent-tool-end",
                        ToolEndPayload {
                            tool_use_id: id.clone(),
                            output: output.clone(),
                            is_error,
                        },
                    );

                    tool_results.push(ContentBlock::ToolResult {
                        tool_use_id: id.clone(),
                        content: output,
                        is_error: if is_error { Some(true) } else { None },
                    });
                }
            }

            conversation.push(AnthropicMessage {
                role: "user".to_string(),
                content: MessageContent::Blocks(tool_results),
            });

            info!(
                "Continuing conversation with tool results (iteration {}/{})",
                iteration, max_iterations
            );
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
    ) -> Result<(), AgentError> {
        let anthropic_messages: Vec<AnthropicMessage> =
            messages.iter().map(AnthropicMessage::from).collect();

        let system = self.build_system_prompt(system_prompt);

        let message_id = Uuid::new_v4().to_string();

        let result = self
            .execute_tool_loop(anthropic_messages, system, &app_handle)
            .await;

        match result {
            Ok(stop_reason) => {
                let _ = app_handle.emit(
                    "agent-complete",
                    AgentCompletePayload {
                        message_id,
                        stop_reason,
                    },
                );
                Ok(())
            }
            Err(e) => {
                let _ = app_handle.emit(
                    "agent-error",
                    AgentErrorPayload {
                        error: e.to_string(),
                    },
                );
                Err(e)
            }
        }
    }

    fn model(&self) -> &str {
        &self.config.model
    }
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
                project_config.execution,
                api_key,
                project_path.to_path_buf(),
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
