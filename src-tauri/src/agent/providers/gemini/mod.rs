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
use crate::agent::provider::{ExecutionContext, HeadlessResult, ProviderAdapter};
use crate::agent::tools::{get_tool_definitions, SessionState};
use crate::agent::types::{
    AgentCancelledPayload, AgentChunkPayload, AgentCompletePayload, AgentErrorPayload, AgentStatus,
    ChatMessage, ContentBlockStartPayload, ContentBlockType, ToolDefinition,
};
use crate::agent::usage::{SessionUsageTracker, UsageSource};
use crate::config::{AgentConfig, ExecutionConfig, PromptsConfig};

use super::{
    build_system_prompt, check_iteration_limit,
    compaction::{format_compacted_context, get_context_limit, maybe_compact, CompactionContext},
    create_executor, emit_status, emit_usage, execute_tool_calls,
    headless::{
        HeadlessResponse, HeadlessStreamer, ToolCall as HeadlessToolCall,
        ToolResult as HeadlessToolResult,
    },
    run_headless_loop, HeadlessContext, StreamContext, StreamingState, ToolCall,
};
use types::{
    FunctionDeclaration, FunctionResponse, FunctionResponseContent, GeminiContent, GeminiPart,
    GeminiRequest, GeminiResponse, GeminiSystemInstruction, GeminiTool, GenerationConfig,
    StreamedResponse,
};

const API_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

pub struct GeminiAdapter {
    client: Client,
    config: AgentConfig,
    prompts: PromptsConfig,
    execution: ExecutionConfig,
    api_key: String,
    project_path: PathBuf,
    app_system_prompt: &'static str,
    context_limit: u32,
    extraction_prompt: Option<String>,
}

impl GeminiAdapter {
    pub fn new(
        config: AgentConfig,
        prompts: PromptsConfig,
        execution: ExecutionConfig,
        project_path: PathBuf,
        app_system_prompt: &'static str,
        extraction_prompt: Option<String>,
    ) -> Result<Self, AgentError> {
        let api_key = env::var(&config.api_key_env)
            .map_err(|_| AgentError::MissingApiKey(config.api_key_env.clone()))?;

        let context_limit = get_context_limit(config.context_limit);

        Ok(Self {
            client: Client::new(),
            config,
            prompts,
            execution,
            api_key,
            project_path,
            app_system_prompt,
            context_limit,
            extraction_prompt,
        })
    }

    fn build_tools(&self) -> Vec<GeminiTool> {
        let tool_defs = get_tool_definitions();
        let declarations: Vec<FunctionDeclaration> =
            tool_defs.iter().map(FunctionDeclaration::from).collect();

        vec![GeminiTool {
            function_declarations: declarations,
        }]
    }

    fn api_url(&self) -> String {
        format!(
            "{}/{}:streamGenerateContent?alt=sse&key={}",
            API_BASE_URL, self.config.model, self.api_key
        )
    }

    async fn stream_response(
        &self,
        contents: &[GeminiContent],
        system: Option<String>,
        ctx: &StreamContext<'_>,
    ) -> Result<StreamedResponse, AgentError> {
        if ctx.cancel_token.is_cancelled() {
            return Err(AgentError::Cancelled);
        }

        let system_instruction = system.map(|text| GeminiSystemInstruction {
            parts: vec![GeminiPart::Text { text }],
        });

        let request = GeminiRequest {
            contents: contents.to_vec(),
            system_instruction,
            tools: Some(self.build_tools()),
            generation_config: Some(GenerationConfig {
                max_output_tokens: Some(self.config.max_tokens),
            }),
        };

        emit_status(ctx.app_handle, AgentStatus::Thinking, None);

        let response = self
            .client
            .post(self.api_url())
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
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

                            // Handle both \r\n\r\n (Gemini) and \n\n delimiters
                            while let Some((event_end, delim_len)) = buffer
                                .find("\r\n\r\n")
                                .map(|i| (i, 4))
                                .or_else(|| buffer.find("\n\n").map(|i| (i, 2)))
                            {
                                let event_data = buffer[..event_end].to_string();
                                buffer = buffer[event_end + delim_len..].to_string();

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
        let mut is_text = false;

        for line in event_data.lines() {
            if let Some(suffix) = line.strip_prefix("data: ") {
                data = Some(suffix.to_string());
            }
        }

        let Some(data) = data else { return false };

        let response: GeminiResponse = match serde_json::from_str(&data) {
            Ok(r) => r,
            Err(_) => return false,
        };

        if let Some(error) = response.error {
            let _ = ctx.app_handle.emit(
                "agent-error",
                AgentErrorPayload {
                    error: error.message,
                },
            );
            return false;
        }

        if let Some(usage_metadata) = &response.usage_metadata {
            streamed.update_usage(usage_metadata);
        }

        if let Some(candidates) = response.candidates {
            for candidate in candidates {
                if let Some(reason) = candidate.finish_reason {
                    streamed.set_finish_reason(reason);
                }

                if let Some(content) = candidate.content {
                    if let Some(parts) = content.parts {
                        for part in parts {
                            match part {
                                types::ResponsePart::Text { text } => {
                                    if !streamed.has_text_block {
                                        let _ = ctx.app_handle.emit(
                                            "agent-content-block-start",
                                            ContentBlockStartPayload {
                                                block_index: ctx.block_offset,
                                                block_type: ContentBlockType::Text,
                                            },
                                        );
                                    }

                                    let local_index = streamed.append_text(&text);
                                    let _ = ctx.app_handle.emit(
                                        "agent-chunk",
                                        AgentChunkPayload {
                                            delta: text.clone(),
                                            block_index: ctx.block_offset + local_index,
                                        },
                                    );
                                    is_text = true;
                                }
                                types::ResponsePart::FunctionCall { function_call } => {
                                    let local_index =
                                        streamed.add_function_call(function_call.clone());
                                    let _ = ctx.app_handle.emit(
                                        "agent-content-block-start",
                                        ContentBlockStartPayload {
                                            block_index: ctx.block_offset + local_index,
                                            block_type: ContentBlockType::ToolUse {
                                                tool_use_id: Uuid::new_v4().to_string(),
                                                tool_name: function_call.name,
                                            },
                                        },
                                    );
                                }
                                types::ResponsePart::Unknown => {}
                            }
                        }
                    }
                }
            }
        }

        is_text
    }

    async fn execute_tool_loop(
        &self,
        initial_contents: Vec<GeminiContent>,
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
        let mut conversation = initial_contents;
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

            if !response.has_function_calls() {
                return Ok(response.finish_reason);
            }

            iteration += 1;
            check_iteration_limit(iteration, max_iterations)?;

            // Add model's response with function calls to conversation
            let model_parts: Vec<GeminiPart> = response
                .function_calls
                .iter()
                .map(|fc| GeminiPart::FunctionCall {
                    function_call: fc.clone(),
                })
                .collect();

            conversation.push(GeminiContent {
                role: "model".to_string(),
                parts: model_parts,
            });

            // Execute tools with global block indices
            let text_offset = if response.has_text_block { 1 } else { 0 };
            let tool_calls: Vec<ToolCall> = response
                .function_calls
                .iter()
                .enumerate()
                .map(|(i, fc)| ToolCall {
                    id: Uuid::new_v4().to_string(),
                    name: fc.name.clone(),
                    input: fc.args.clone(),
                    block_index: ctx.block_offset + text_offset + i as u32,
                })
                .collect();

            let results =
                execute_tool_calls(tool_calls, &executor, &session, app_handle, cancel_token)
                    .await?;

            let function_responses: Vec<GeminiPart> = results
                .into_iter()
                .map(|r| {
                    let response_content = if r.is_error {
                        FunctionResponseContent {
                            result: None,
                            error: Some(r.output),
                        }
                    } else {
                        FunctionResponseContent {
                            result: Some(r.output),
                            error: None,
                        }
                    };
                    GeminiPart::FunctionResponse {
                        function_response: FunctionResponse {
                            name: r.name,
                            response: response_content,
                        },
                    }
                })
                .collect();

            // Add function responses as user message
            conversation.push(GeminiContent {
                role: "user".to_string(),
                parts: function_responses,
            });

            emit_status(app_handle, AgentStatus::ToolWaiting, None);
        }
    }

    async fn call_extraction_api(
        &self,
        prompt: String,
        cancel_token: &CancellationToken,
    ) -> Result<String, AgentError> {
        if cancel_token.is_cancelled() {
            return Err(AgentError::Cancelled);
        }

        let system_instruction = GeminiSystemInstruction {
            parts: vec![GeminiPart::Text {
                text: "You are a precise assistant that extracts and summarizes information. Always respond with valid JSON.".to_string(),
            }],
        };

        let request = GeminiRequest {
            contents: vec![GeminiContent {
                role: "user".to_string(),
                parts: vec![GeminiPart::Text { text: prompt }],
            }],
            system_instruction: Some(system_instruction),
            tools: None,
            generation_config: Some(GenerationConfig {
                max_output_tokens: Some(2048),
            }),
        };

        // Use non-streaming endpoint
        let url = format!(
            "{}/{}:generateContent?key={}",
            API_BASE_URL, self.config.model, self.api_key
        );

        let response = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AgentError::ApiError(format!(
                "Extraction failed: {}: {}",
                status, body
            )));
        }

        // Parse the non-streaming response
        let body = response.text().await?;
        let json: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| AgentError::ApiError(format!("Failed to parse response: {}", e)))?;

        // Extract text from the Gemini response
        let text = json["candidates"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|candidate| candidate["content"]["parts"].as_array())
            .and_then(|parts| parts.first())
            .and_then(|part| part["text"].as_str())
            .unwrap_or("")
            .to_string();

        Ok(text)
    }

    fn build_tools_from_definitions(&self, tools: &[ToolDefinition]) -> Vec<GeminiTool> {
        let declarations: Vec<FunctionDeclaration> =
            tools.iter().map(FunctionDeclaration::from).collect();

        vec![GeminiTool {
            function_declarations: declarations,
        }]
    }

    async fn stream_response_headless(
        &self,
        contents: &[GeminiContent],
        system: Option<String>,
        tools: &[ToolDefinition],
        cancel_token: &CancellationToken,
    ) -> Result<StreamedResponse, AgentError> {
        if cancel_token.is_cancelled() {
            return Err(AgentError::Cancelled);
        }

        let system_instruction = system.map(|text| GeminiSystemInstruction {
            parts: vec![GeminiPart::Text { text }],
        });

        let request = GeminiRequest {
            contents: contents.to_vec(),
            system_instruction,
            tools: Some(self.build_tools_from_definitions(tools)),
            generation_config: Some(GenerationConfig {
                max_output_tokens: Some(self.config.max_tokens),
            }),
        };

        let response = self
            .client
            .post(self.api_url())
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
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

                            while let Some((event_end, delim_len)) = buffer
                                .find("\r\n\r\n")
                                .map(|i| (i, 4))
                                .or_else(|| buffer.find("\n\n").map(|i| (i, 2)))
                            {
                                let event_data = buffer[..event_end].to_string();
                                buffer = buffer[event_end + delim_len..].to_string();
                                self.process_sse_event_headless(&event_data, &mut streamed);
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

    fn process_sse_event_headless(&self, event_data: &str, streamed: &mut StreamedResponse) {
        let mut data = None;

        for line in event_data.lines() {
            if let Some(suffix) = line.strip_prefix("data: ") {
                data = Some(suffix.to_string());
            }
        }

        let Some(data) = data else { return };

        let response: GeminiResponse = match serde_json::from_str(&data) {
            Ok(r) => r,
            Err(_) => return,
        };

        if let Some(usage_metadata) = &response.usage_metadata {
            streamed.update_usage(usage_metadata);
        }

        if let Some(candidates) = response.candidates {
            for candidate in candidates {
                if let Some(reason) = candidate.finish_reason {
                    streamed.set_finish_reason(reason);
                }

                if let Some(content) = candidate.content {
                    if let Some(parts) = content.parts {
                        for part in parts {
                            match part {
                                types::ResponsePart::Text { text } => {
                                    streamed.append_text(&text);
                                }
                                types::ResponsePart::FunctionCall { function_call } => {
                                    streamed.add_function_call(function_call);
                                }
                                types::ResponsePart::Unknown => {}
                            }
                        }
                    }
                }
            }
        }
    }

    fn to_headless_response(&self, response: &StreamedResponse) -> HeadlessResponse {
        let tool_calls = response
            .function_calls
            .iter()
            .map(|fc| HeadlessToolCall {
                id: fc.name.clone(), // Gemini uses name as ID
                name: fc.name.clone(),
                input: fc.args.clone(),
            })
            .collect();

        HeadlessResponse {
            text: response.text_content.clone(),
            tool_calls,
            usage: response.usage,
        }
    }
}

#[async_trait]
impl HeadlessStreamer for GeminiAdapter {
    type Conversation = Vec<GeminiContent>;

    fn initial_conversation(&self, messages: Vec<ChatMessage>) -> Self::Conversation {
        messages.iter().map(GeminiContent::from).collect()
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
        let mut parts = Vec::new();

        if !response.text.is_empty() {
            parts.push(GeminiPart::Text {
                text: response.text.clone(),
            });
        }

        for tc in &response.tool_calls {
            parts.push(GeminiPart::FunctionCall {
                function_call: types::FunctionCall {
                    name: tc.name.clone(),
                    args: tc.input.clone(),
                },
            });
        }

        conversation.push(GeminiContent {
            role: "model".to_string(),
            parts,
        });
    }

    fn append_tool_results(
        &self,
        conversation: &mut Self::Conversation,
        results: Vec<HeadlessToolResult>,
    ) {
        let parts: Vec<GeminiPart> = results
            .into_iter()
            .map(|r| {
                let response_content = if r.is_error {
                    FunctionResponseContent {
                        result: None,
                        error: Some(r.output),
                    }
                } else {
                    FunctionResponseContent {
                        result: Some(r.output),
                        error: None,
                    }
                };

                GeminiPart::FunctionResponse {
                    function_response: FunctionResponse {
                        name: r.name,
                        response: response_content,
                    },
                }
            })
            .collect();

        conversation.push(GeminiContent {
            role: "user".to_string(),
            parts,
        });
    }
}

#[async_trait]
impl ProviderAdapter for GeminiAdapter {
    async fn send_message(
        &self,
        messages: Vec<ChatMessage>,
        system_prompt: Option<String>,
        memory: Option<String>,
        ctx: ExecutionContext,
        app_handle: AppHandle,
    ) -> Result<(), AgentError> {
        // Build system prompt first
        let base_system = build_system_prompt(
            self.app_system_prompt,
            &self.prompts,
            system_prompt,
            memory.as_deref(),
        );

        let message_id = Uuid::new_v4().to_string();

        emit_status(&app_handle, AgentStatus::Sending, None);

        // Check if compaction is needed
        let compaction_ctx = CompactionContext {
            context_limit: self.context_limit,
            extraction_prompt: self.extraction_prompt.as_deref(),
            session: &ctx.session,
            app_handle: &app_handle,
        };

        let compaction_result =
            maybe_compact(&messages, Some(&base_system), &compaction_ctx, |prompt| {
                self.call_extraction_api(prompt, &ctx.cancel_token)
            })
            .await?;

        // Prepare messages and system prompt based on compaction result
        let (final_messages, final_system) = match compaction_result {
            Some(result) => {
                let system_with_context = format!("{}\n\n{}", base_system, result.compacted_text);
                (result.preserved_messages, system_with_context)
            }
            None => {
                let existing = ctx.session.get_compacted().await;
                if existing.summary.is_some() || !existing.facts.is_empty() {
                    let compacted_text = format_compacted_context(&existing);
                    let system_with_context = format!("{}\n\n{}", base_system, compacted_text);
                    (messages.clone(), system_with_context)
                } else {
                    (messages.clone(), base_system)
                }
            }
        };

        let gemini_contents: Vec<GeminiContent> =
            final_messages.iter().map(GeminiContent::from).collect();

        let result = self
            .execute_tool_loop(
                gemini_contents,
                Some(final_system),
                ctx.session,
                &app_handle,
                &ctx.cancel_token,
                &ctx.usage_tracker,
            )
            .await;

        match result {
            Ok(finish_reason) => {
                emit_status(&app_handle, AgentStatus::Idle, None);
                let _ = app_handle.emit(
                    "agent-complete",
                    AgentCompletePayload {
                        message_id,
                        stop_reason: finish_reason,
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
        memory: Option<String>,
        tools: Vec<ToolDefinition>,
        ctx: ExecutionContext,
    ) -> Result<HeadlessResult, AgentError> {
        let system = build_system_prompt(
            self.app_system_prompt,
            &self.prompts,
            system_prompt,
            memory.as_deref(),
        );
        let executor = create_executor(
            &self.project_path,
            &self.execution,
            ctx.session,
            ctx.cancel_token.clone(),
            Arc::clone(&ctx.usage_tracker),
        );

        run_headless_loop(
            self,
            messages,
            HeadlessContext {
                system_prompt: Some(system),
                tools,
                executor: &executor,
                max_iterations: self.execution.max_tool_iterations,
                cancel_token: &ctx.cancel_token,
                usage_tracker: ctx.usage_tracker,
            },
        )
        .await
    }

    fn model(&self) -> &str {
        &self.config.model
    }
}
