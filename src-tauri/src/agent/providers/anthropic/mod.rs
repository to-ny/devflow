mod types;

use std::env;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::agent::error::AgentError;
use crate::agent::provider::{HeadlessResult, ProviderAdapter};
use crate::agent::tools::{get_tool_definitions, SessionState};
use crate::agent::types::{
    AgentCancelledPayload, AgentChunkPayload, AgentCompletePayload, AgentErrorPayload, AgentStatus,
    ChatMessage, ContentBlockStartPayload, ContentBlockType, ToolDefinition,
};
use crate::agent::usage::{SessionUsageTracker, UsageSource};
use crate::config::{AgentConfig, ExecutionConfig, PromptsConfig};

use super::{
    build_system_prompt, check_iteration_limit, create_executor, emit_status, emit_usage,
    execute_tool_calls,
    headless::{
        HeadlessResponse, HeadlessStreamer, ToolCall as HeadlessToolCall,
        ToolResult as HeadlessToolResult,
    },
    run_headless_loop, HeadlessContext, StreamContext, StreamingState, ToolCall,
};
use types::{
    AnthropicErrorResponse, AnthropicEvent, AnthropicMessage, AnthropicRequest, ContentBlock,
    ContentDelta, MessageContent, StreamedResponse,
};

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";

pub struct AnthropicAdapter {
    client: Client,
    config: AgentConfig,
    prompts: PromptsConfig,
    execution: ExecutionConfig,
    api_key: String,
    project_path: PathBuf,
    app_system_prompt: &'static str,
}

impl AnthropicAdapter {
    pub fn new(
        config: AgentConfig,
        prompts: PromptsConfig,
        execution: ExecutionConfig,
        project_path: PathBuf,
        app_system_prompt: &'static str,
    ) -> Result<Self, AgentError> {
        let api_key = env::var(&config.api_key_env)
            .map_err(|_| AgentError::MissingApiKey(config.api_key_env.clone()))?;

        Ok(Self {
            client: Client::new(),
            config,
            prompts,
            execution,
            api_key,
            project_path,
            app_system_prompt,
        })
    }

    async fn stream_response(
        &self,
        messages: &[AnthropicMessage],
        system: Option<String>,
        ctx: &StreamContext<'_>,
    ) -> Result<StreamedResponse, AgentError> {
        if ctx.cancel_token.is_cancelled() {
            return Err(AgentError::Cancelled);
        }

        let tools = get_tool_definitions();

        let request = AnthropicRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            messages: messages.to_vec(),
            stream: true,
            system,
            tools: Some(tools),
        };

        emit_status(ctx.app_handle, AgentStatus::Thinking, None);

        let response = self
            .client
            .post(API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            if let Ok(error_response) = serde_json::from_str::<AnthropicErrorResponse>(&body) {
                return Err(AgentError::ApiError(error_response.error.message));
            }
            return Err(AgentError::ApiError(format!("{}: {}", status, body)));
        }

        let mut streamed = StreamedResponse::new();
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut first_text_chunk = true;

        loop {
            if ctx.cancel_token.is_cancelled() {
                return Err(AgentError::Cancelled);
            }

            tokio::select! {
                _ = ctx.cancel_token.cancelled() => {
                    return Err(AgentError::Cancelled);
                }
                chunk_result = stream.next() => {
                    match chunk_result {
                        Some(Ok(chunk)) => {
                            buffer.push_str(&String::from_utf8_lossy(&chunk));

                            while let Some(event_end) = buffer.find("\n\n") {
                                let event_data = buffer[..event_end].to_string();
                                buffer = buffer[event_end + 2..].to_string();

                                let is_text = self.process_sse_event(&event_data, ctx, &mut streamed);

                                if is_text && first_text_chunk {
                                    emit_status(ctx.app_handle, AgentStatus::Streaming, None);
                                    first_text_chunk = false;
                                }
                            }
                        }
                        Some(Err(e)) => {
                            return Err(AgentError::Http(e));
                        }
                        None => break,
                    }
                }
            }
        }

        Ok(streamed)
    }

    fn process_sse_event(
        &self,
        event_data: &str,
        ctx: &StreamContext<'_>,
        streamed: &mut StreamedResponse,
    ) -> bool {
        let mut data = None;
        let mut is_text_delta = false;

        for line in event_data.lines() {
            if let Some(suffix) = line.strip_prefix("data: ") {
                data = Some(suffix.to_string());
            }
        }

        let Some(data) = data else { return false };

        if data == "[DONE]" {
            return false;
        }

        let event: AnthropicEvent = match serde_json::from_str(&data) {
            Ok(e) => e,
            Err(_) => return false,
        };

        match event {
            AnthropicEvent::ContentBlockStart {
                index,
                content_block,
            } => {
                streamed.on_content_block_start(index, content_block.clone());

                let global_index = ctx.block_offset + index;
                let block_type = match &content_block {
                    ContentBlock::Text { .. } => ContentBlockType::Text,
                    ContentBlock::ToolUse { id, name, .. } => ContentBlockType::ToolUse {
                        tool_use_id: id.clone(),
                        tool_name: name.clone(),
                    },
                    ContentBlock::ToolResult { .. } => return false,
                };

                let _ = ctx.app_handle.emit(
                    "agent-content-block-start",
                    ContentBlockStartPayload {
                        block_index: global_index,
                        block_type,
                    },
                );
            }
            AnthropicEvent::ContentBlockDelta { index, delta } => {
                if let ContentDelta::TextDelta { ref text } = delta {
                    let global_index = ctx.block_offset + index;
                    let _ = ctx.app_handle.emit(
                        "agent-chunk",
                        AgentChunkPayload {
                            delta: text.clone(),
                            block_index: global_index,
                        },
                    );
                    is_text_delta = true;
                }
                streamed.on_content_delta(index, delta);
            }
            AnthropicEvent::ContentBlockStop { index } => {
                streamed.on_content_block_stop(index);
            }
            AnthropicEvent::MessageStart { message } => {
                streamed.on_message_start(&message);
            }
            AnthropicEvent::MessageDelta { delta, usage } => {
                streamed.on_message_delta(delta, usage);
            }
            AnthropicEvent::Error { error } => {
                let _ = ctx.app_handle.emit(
                    "agent-error",
                    AgentErrorPayload {
                        error: error.message,
                    },
                );
            }
            _ => {}
        }

        is_text_delta
    }

    async fn execute_tool_loop(
        &self,
        initial_messages: Vec<AnthropicMessage>,
        system_prompt: Option<String>,
        session: SessionState,
        app_handle: &AppHandle,
        cancel_token: &CancellationToken,
        usage_tracker: &Arc<SessionUsageTracker>,
    ) -> Result<Option<String>, AgentError> {
        let executor = create_executor(
            &self.project_path,
            &self.execution,
            session.clone(),
            cancel_token.clone(),
            Arc::clone(usage_tracker),
        );
        let mut conversation = initial_messages;
        let max_iterations = self.execution.max_tool_iterations;
        let mut iteration = 0u32;
        let mut streaming = StreamingState::new();

        loop {
            if cancel_token.is_cancelled() {
                return Err(AgentError::Cancelled);
            }

            let ctx = streaming.create_context(app_handle, cancel_token);
            let response = self
                .stream_response(&conversation, system_prompt.clone(), &ctx)
                .await?;

            emit_usage(app_handle, usage_tracker, response.usage, UsageSource::Main);
            streaming.advance(response.block_count());

            if !response.has_tool_use() {
                return Ok(response.stop_reason);
            }

            iteration += 1;
            check_iteration_limit(iteration, max_iterations)?;

            conversation.push(AnthropicMessage {
                role: "assistant".to_string(),
                content: MessageContent::Blocks(response.content_blocks.clone()),
            });

            let tool_calls: Vec<ToolCall> = response
                .content_blocks
                .iter()
                .enumerate()
                .filter_map(|(index, block)| {
                    if let ContentBlock::ToolUse { id, name, input } = block {
                        Some(ToolCall {
                            id: id.clone(),
                            name: name.clone(),
                            input: input.clone(),
                            block_index: ctx.block_offset + index as u32,
                        })
                    } else {
                        None
                    }
                })
                .collect();

            let results =
                execute_tool_calls(tool_calls, &executor, &session, app_handle, cancel_token)
                    .await?;

            let tool_results: Vec<ContentBlock> = results
                .into_iter()
                .map(|r| ContentBlock::ToolResult {
                    tool_use_id: r.id,
                    content: r.output,
                    is_error: if r.is_error { Some(true) } else { None },
                })
                .collect();

            conversation.push(AnthropicMessage {
                role: "user".to_string(),
                content: MessageContent::Blocks(tool_results),
            });

            emit_status(app_handle, AgentStatus::ToolWaiting, None);
        }
    }

    async fn stream_response_headless(
        &self,
        messages: &[AnthropicMessage],
        system: Option<String>,
        tools: &[ToolDefinition],
        cancel_token: &CancellationToken,
    ) -> Result<StreamedResponse, AgentError> {
        if cancel_token.is_cancelled() {
            return Err(AgentError::Cancelled);
        }

        let request = AnthropicRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            messages: messages.to_vec(),
            stream: true,
            system,
            tools: Some(tools.to_vec()),
        };

        let response = self
            .client
            .post(API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            if let Ok(error_response) = serde_json::from_str::<AnthropicErrorResponse>(&body) {
                return Err(AgentError::ApiError(error_response.error.message));
            }
            return Err(AgentError::ApiError(format!("{}: {}", status, body)));
        }

        let mut streamed = StreamedResponse::new();
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        loop {
            if cancel_token.is_cancelled() {
                return Err(AgentError::Cancelled);
            }

            tokio::select! {
                _ = cancel_token.cancelled() => {
                    return Err(AgentError::Cancelled);
                }
                chunk_result = stream.next() => {
                    match chunk_result {
                        Some(Ok(chunk)) => {
                            buffer.push_str(&String::from_utf8_lossy(&chunk));

                            while let Some(event_end) = buffer.find("\n\n") {
                                let event_data = buffer[..event_end].to_string();
                                buffer = buffer[event_end + 2..].to_string();
                                self.process_headless_sse_event(&event_data, &mut streamed);
                            }
                        }
                        Some(Err(e)) => {
                            return Err(AgentError::Http(e));
                        }
                        None => break,
                    }
                }
            }
        }

        Ok(streamed)
    }

    fn process_headless_sse_event(&self, event_data: &str, streamed: &mut StreamedResponse) {
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
            Err(_) => return,
        };

        match event {
            AnthropicEvent::ContentBlockStart {
                index,
                content_block,
            } => {
                streamed.on_content_block_start(index, content_block);
            }
            AnthropicEvent::ContentBlockDelta { index, delta } => {
                streamed.on_content_delta(index, delta);
            }
            AnthropicEvent::ContentBlockStop { index } => {
                streamed.on_content_block_stop(index);
            }
            AnthropicEvent::MessageStart { message } => {
                streamed.on_message_start(&message);
            }
            AnthropicEvent::MessageDelta { delta, usage } => {
                streamed.on_message_delta(delta, usage);
            }
            _ => {}
        }
    }

    fn to_headless_response(&self, response: &StreamedResponse) -> HeadlessResponse {
        let text = response
            .content_blocks
            .iter()
            .filter_map(|block| {
                if let ContentBlock::Text { text } = block {
                    Some(text.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        let tool_calls = response
            .content_blocks
            .iter()
            .filter_map(|block| {
                if let ContentBlock::ToolUse { id, name, input } = block {
                    Some(HeadlessToolCall {
                        id: id.clone(),
                        name: name.clone(),
                        input: input.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();

        HeadlessResponse {
            text,
            tool_calls,
            usage: response.usage,
        }
    }
}

#[async_trait]
impl HeadlessStreamer for AnthropicAdapter {
    type Conversation = Vec<AnthropicMessage>;

    fn initial_conversation(&self, messages: Vec<ChatMessage>) -> Self::Conversation {
        messages.iter().map(AnthropicMessage::from).collect()
    }

    async fn stream_response(
        &self,
        conversation: &Self::Conversation,
        system_prompt: Option<String>,
        tools: &[ToolDefinition],
        cancel_token: &CancellationToken,
    ) -> Result<HeadlessResponse, AgentError> {
        let response = self
            .stream_response_headless(conversation, system_prompt, tools, cancel_token)
            .await?;

        Ok(self.to_headless_response(&response))
    }

    fn append_assistant_response(
        &self,
        conversation: &mut Self::Conversation,
        response: &HeadlessResponse,
    ) {
        // Build content blocks from the response
        let mut blocks = Vec::new();

        if !response.text.is_empty() {
            blocks.push(ContentBlock::Text {
                text: response.text.clone(),
            });
        }

        for tc in &response.tool_calls {
            blocks.push(ContentBlock::ToolUse {
                id: tc.id.clone(),
                name: tc.name.clone(),
                input: tc.input.clone(),
            });
        }

        conversation.push(AnthropicMessage {
            role: "assistant".to_string(),
            content: MessageContent::Blocks(blocks),
        });
    }

    fn append_tool_results(
        &self,
        conversation: &mut Self::Conversation,
        results: Vec<HeadlessToolResult>,
    ) {
        let blocks: Vec<ContentBlock> = results
            .into_iter()
            .map(|r| ContentBlock::ToolResult {
                tool_use_id: r.id,
                content: r.output,
                is_error: if r.is_error { Some(true) } else { None },
            })
            .collect();

        conversation.push(AnthropicMessage {
            role: "user".to_string(),
            content: MessageContent::Blocks(blocks),
        });
    }
}

#[async_trait]
impl ProviderAdapter for AnthropicAdapter {
    async fn send_message(
        &self,
        messages: Vec<ChatMessage>,
        system_prompt: Option<String>,
        session: SessionState,
        app_handle: AppHandle,
        cancel_token: CancellationToken,
        usage_tracker: Arc<SessionUsageTracker>,
    ) -> Result<(), AgentError> {
        let anthropic_messages: Vec<AnthropicMessage> =
            messages.iter().map(AnthropicMessage::from).collect();

        let system = build_system_prompt(self.app_system_prompt, &self.prompts, system_prompt);

        let message_id = Uuid::new_v4().to_string();

        emit_status(&app_handle, AgentStatus::Sending, None);

        let result = self
            .execute_tool_loop(
                anthropic_messages,
                Some(system),
                session,
                &app_handle,
                &cancel_token,
                &usage_tracker,
            )
            .await;

        match result {
            Ok(stop_reason) => {
                emit_status(&app_handle, AgentStatus::Idle, None);
                let _ = app_handle.emit(
                    "agent-complete",
                    AgentCompletePayload {
                        message_id,
                        stop_reason,
                    },
                );
                Ok(())
            }
            Err(AgentError::Cancelled) => {
                emit_status(&app_handle, AgentStatus::Cancelled, None);
                let _ = app_handle.emit(
                    "agent-cancelled",
                    AgentCancelledPayload {
                        reason: "Cancelled by user".to_string(),
                    },
                );
                Err(AgentError::Cancelled)
            }
            Err(e) => {
                emit_status(&app_handle, AgentStatus::Error, Some(e.to_string()));
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

    async fn run_headless(
        &self,
        messages: Vec<ChatMessage>,
        system_prompt: Option<String>,
        tools: Vec<ToolDefinition>,
        session: SessionState,
        cancel_token: CancellationToken,
        usage_tracker: Arc<SessionUsageTracker>,
    ) -> Result<HeadlessResult, AgentError> {
        let system = build_system_prompt(self.app_system_prompt, &self.prompts, system_prompt);
        let executor = create_executor(
            &self.project_path,
            &self.execution,
            session,
            cancel_token.clone(),
            Arc::clone(&usage_tracker),
        );

        run_headless_loop(
            self,
            messages,
            HeadlessContext {
                system_prompt: Some(system),
                tools,
                executor: &executor,
                max_iterations: self.execution.max_tool_iterations,
                cancel_token: &cancel_token,
                usage_tracker,
            },
        )
        .await
    }

    fn model(&self) -> &str {
        &self.config.model
    }
}
