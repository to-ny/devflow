mod types;

use std::env;
use std::path::PathBuf;

use async_trait::async_trait;
use futures::StreamExt;
use log::{error, info, warn};
use reqwest::Client;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::agent::error::AgentError;
use crate::agent::provider::ProviderAdapter;
use crate::agent::tools::get_tool_definitions;
use crate::agent::types::{
    AgentCancelledPayload, AgentChunkPayload, AgentCompletePayload, AgentErrorPayload, AgentStatus,
    ChatMessage,
};
use crate::config::{AgentConfig, ExecutionConfig, PromptsConfig};

use super::{
    build_system_prompt, check_iteration_limit, create_executor, emit_status, execute_tool_calls,
    ToolCall,
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
}

impl GeminiAdapter {
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
        app_handle: &AppHandle,
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
            tools: Some(self.build_tools()),
            generation_config: Some(GenerationConfig {
                max_output_tokens: Some(self.config.max_tokens),
            }),
        };

        info!(
            "Sending message to Gemini API (model: {}, {} contents)",
            self.config.model,
            contents.len()
        );

        emit_status(app_handle, AgentStatus::Thinking, None);

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
            error!("Gemini API error: {} - {}", status, body);
            return Err(AgentError::ApiError(format!("{}: {}", status, body)));
        }

        let mut streamed = StreamedResponse::new();
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut first_text_chunk = true;

        loop {
            if cancel_token.is_cancelled() {
                warn!("Stream cancelled by user");
                return Err(AgentError::Cancelled);
            }

            tokio::select! {
                _ = cancel_token.cancelled() => {
                    warn!("Stream cancelled by user");
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

                                let is_text =
                                    self.process_sse_event(&event_data, app_handle, &mut streamed);

                                if is_text && first_text_chunk {
                                    emit_status(app_handle, AgentStatus::Streaming, None);
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
        app_handle: &AppHandle,
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
            Err(e) => {
                warn!("Failed to parse Gemini SSE event: {} - data: {}", e, data);
                return false;
            }
        };

        if let Some(error) = response.error {
            let _ = app_handle.emit(
                "agent-error",
                AgentErrorPayload {
                    error: error.message,
                },
            );
            return false;
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
                                    let _ = app_handle.emit(
                                        "agent-chunk",
                                        AgentChunkPayload {
                                            delta: text.clone(),
                                        },
                                    );
                                    streamed.append_text(&text);
                                    is_text = true;
                                }
                                types::ResponsePart::FunctionCall { function_call } => {
                                    streamed.add_function_call(function_call);
                                }
                                types::ResponsePart::Unknown(value) => {
                                    warn!("Unknown Gemini response part: {:?}", value);
                                }
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
        app_handle: &AppHandle,
        cancel_token: &CancellationToken,
    ) -> Result<Option<String>, AgentError> {
        let executor = create_executor(&self.project_path, &self.execution);
        let mut conversation = initial_contents;
        let max_iterations = self.execution.max_tool_iterations;
        let mut iteration = 0u32;

        loop {
            if cancel_token.is_cancelled() {
                return Err(AgentError::Cancelled);
            }

            let response = self
                .stream_response(
                    &conversation,
                    system_prompt.clone(),
                    app_handle,
                    cancel_token,
                )
                .await?;

            if !response.has_function_calls() {
                info!(
                    "Agent response complete (finish_reason: {:?})",
                    response.finish_reason
                );
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

            // Execute tools and collect results
            let tool_calls: Vec<ToolCall> = response
                .function_calls
                .iter()
                .map(|fc| ToolCall {
                    id: Uuid::new_v4().to_string(),
                    name: fc.name.clone(),
                    input: fc.args.clone(),
                })
                .collect();

            let results =
                execute_tool_calls(tool_calls, &executor, app_handle, cancel_token).await?;

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

            info!(
                "Continuing conversation with tool results (iteration {}/{})",
                iteration, max_iterations
            );
        }
    }
}

#[async_trait]
impl ProviderAdapter for GeminiAdapter {
    async fn send_message(
        &self,
        messages: Vec<ChatMessage>,
        system_prompt: Option<String>,
        app_handle: AppHandle,
        cancel_token: CancellationToken,
    ) -> Result<(), AgentError> {
        let gemini_contents: Vec<GeminiContent> =
            messages.iter().map(GeminiContent::from).collect();

        let system = build_system_prompt(self.app_system_prompt, &self.prompts, system_prompt);

        let message_id = Uuid::new_v4().to_string();

        emit_status(&app_handle, AgentStatus::Sending, None);

        let result = self
            .execute_tool_loop(gemini_contents, Some(system), &app_handle, &cancel_token)
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

    fn model(&self) -> &str {
        &self.config.model
    }
}
