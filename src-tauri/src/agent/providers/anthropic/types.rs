use serde::{Deserialize, Serialize};

use crate::agent::types::{ChatMessage, MessageRole, ToolDefinition};

// === Request Types ===

#[derive(Debug, Serialize)]
pub struct AnthropicRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<AnthropicMessage>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
}

#[derive(Debug, Serialize, Clone)]
pub struct AnthropicMessage {
    pub role: String,
    #[serde(serialize_with = "serialize_content")]
    pub content: MessageContent,
}

#[derive(Debug, Clone)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

fn serialize_content<S>(content: &MessageContent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match content {
        MessageContent::Text(s) => serializer.serialize_str(s),
        MessageContent::Blocks(blocks) => blocks.serialize(serializer),
    }
}

impl From<&ChatMessage> for AnthropicMessage {
    fn from(msg: &ChatMessage) -> Self {
        use crate::agent::types::ChatContentBlock;

        let role = match msg.role {
            MessageRole::User => "user".to_string(),
            MessageRole::Assistant => "assistant".to_string(),
        };

        // Convert ChatContentBlocks to Anthropic ContentBlocks
        let blocks: Vec<ContentBlock> = msg
            .content_blocks
            .iter()
            .map(|block| match block {
                ChatContentBlock::Text { text } => ContentBlock::Text { text: text.clone() },
                ChatContentBlock::ToolUse {
                    tool_use_id,
                    tool_name,
                    tool_input,
                    ..
                } => ContentBlock::ToolUse {
                    id: tool_use_id.clone(),
                    name: tool_name.clone(),
                    input: tool_input.clone(),
                },
            })
            .collect();

        Self {
            role,
            content: if blocks.len() == 1 {
                if let Some(ContentBlock::Text { text }) = blocks.first() {
                    MessageContent::Text(text.clone())
                } else {
                    MessageContent::Blocks(blocks)
                }
            } else {
                MessageContent::Blocks(blocks)
            },
        }
    }
}

// === Content Block Types (used in both requests and responses) ===

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        #[serde(default)]
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

// === Streaming Response Types ===

#[derive(Debug, Deserialize)]
pub struct AnthropicErrorResponse {
    pub error: AnthropicErrorDetail,
}

#[derive(Debug, Deserialize)]
pub struct AnthropicErrorDetail {
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
pub enum AnthropicEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: MessageStartData },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: u32,
        content_block: ContentBlock,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: u32, delta: ContentDelta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: u32 },
    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: MessageDeltaData,
        usage: Option<UsageData>,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "error")]
    Error { error: AnthropicErrorDetail },
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct MessageStartData {
    pub id: String,
    pub model: String,
    pub stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ContentDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
}

#[derive(Debug, Deserialize)]
pub struct MessageDeltaData {
    pub stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct UsageData {
    pub output_tokens: u32,
}

/// Accumulates streamed response chunks into complete content blocks.
#[derive(Debug, Default)]
pub struct StreamedResponse {
    pub content_blocks: Vec<ContentBlock>,
    pub stop_reason: Option<String>,
    current_block_index: Option<u32>,
    current_tool_json: String,
}

impl StreamedResponse {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_content_block_start(&mut self, index: u32, block: ContentBlock) {
        self.current_block_index = Some(index);
        self.current_tool_json.clear();

        match &block {
            ContentBlock::ToolUse { id, name, .. } => {
                self.content_blocks.push(ContentBlock::ToolUse {
                    id: id.clone(),
                    name: name.clone(),
                    input: serde_json::Value::Null,
                });
            }
            _ => {
                self.content_blocks.push(block);
            }
        }
    }

    pub fn on_content_delta(&mut self, index: u32, delta: ContentDelta) {
        if let Some(block) = self.content_blocks.get_mut(index as usize) {
            match (block, delta) {
                (ContentBlock::Text { text }, ContentDelta::TextDelta { text: delta_text }) => {
                    text.push_str(&delta_text);
                }
                (ContentBlock::ToolUse { .. }, ContentDelta::InputJsonDelta { partial_json }) => {
                    self.current_tool_json.push_str(&partial_json);
                }
                _ => {}
            }
        }
    }

    pub fn on_content_block_stop(&mut self, index: u32) {
        if let Some(ContentBlock::ToolUse { input, .. }) =
            self.content_blocks.get_mut(index as usize)
        {
            if !self.current_tool_json.is_empty() {
                if let Ok(parsed) = serde_json::from_str(&self.current_tool_json) {
                    *input = parsed;
                }
            }
        }
        self.current_tool_json.clear();
        self.current_block_index = None;
    }

    pub fn on_message_delta(&mut self, delta: MessageDeltaData) {
        self.stop_reason = delta.stop_reason;
    }

    pub fn has_tool_use(&self) -> bool {
        self.content_blocks
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolUse { .. }))
    }

    pub fn block_count(&self) -> u32 {
        self.content_blocks.len() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streamed_response_text_accumulation() {
        let mut response = StreamedResponse::new();

        response.on_content_block_start(
            0,
            ContentBlock::Text {
                text: String::new(),
            },
        );

        response.on_content_delta(
            0,
            ContentDelta::TextDelta {
                text: "Hello ".to_string(),
            },
        );
        response.on_content_delta(
            0,
            ContentDelta::TextDelta {
                text: "world!".to_string(),
            },
        );

        response.on_content_block_stop(0);

        assert!(!response.has_tool_use());
        assert_eq!(response.content_blocks.len(), 1);

        if let ContentBlock::Text { text } = &response.content_blocks[0] {
            assert_eq!(text, "Hello world!");
        } else {
            panic!("Expected Text block");
        }
    }

    #[test]
    fn test_streamed_response_tool_use_accumulation() {
        let mut response = StreamedResponse::new();

        response.on_content_block_start(
            0,
            ContentBlock::ToolUse {
                id: "tool_123".to_string(),
                name: "bash".to_string(),
                input: serde_json::Value::Null,
            },
        );

        response.on_content_delta(
            0,
            ContentDelta::InputJsonDelta {
                partial_json: r#"{"command":"#.to_string(),
            },
        );
        response.on_content_delta(
            0,
            ContentDelta::InputJsonDelta {
                partial_json: r#""ls -la"}"#.to_string(),
            },
        );

        response.on_content_block_stop(0);

        assert!(response.has_tool_use());
        assert_eq!(response.content_blocks.len(), 1);

        if let ContentBlock::ToolUse { id, name, input } = &response.content_blocks[0] {
            assert_eq!(id, "tool_123");
            assert_eq!(name, "bash");
            assert_eq!(input["command"], "ls -la");
        } else {
            panic!("Expected ToolUse block");
        }
    }

    #[test]
    fn test_streamed_response_mixed_content() {
        let mut response = StreamedResponse::new();

        response.on_content_block_start(
            0,
            ContentBlock::Text {
                text: String::new(),
            },
        );
        response.on_content_delta(
            0,
            ContentDelta::TextDelta {
                text: "Let me check.".to_string(),
            },
        );
        response.on_content_block_stop(0);

        response.on_content_block_start(
            1,
            ContentBlock::ToolUse {
                id: "tool_456".to_string(),
                name: "read_file".to_string(),
                input: serde_json::Value::Null,
            },
        );
        response.on_content_delta(
            1,
            ContentDelta::InputJsonDelta {
                partial_json: r#"{"path":"src/main.rs"}"#.to_string(),
            },
        );
        response.on_content_block_stop(1);

        assert!(response.has_tool_use());
        assert_eq!(response.content_blocks.len(), 2);
    }

    #[test]
    fn test_streamed_response_stop_reason() {
        let mut response = StreamedResponse::new();

        assert!(response.stop_reason.is_none());

        response.on_message_delta(MessageDeltaData {
            stop_reason: Some("end_turn".to_string()),
        });

        assert_eq!(response.stop_reason, Some("end_turn".to_string()));
    }
}
