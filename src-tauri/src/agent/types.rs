use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

/// Tool definition for Anthropic API requests
#[derive(Debug, Serialize, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Ordered content block within a message (text or tool use).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        tool_use_id: String,
        tool_name: String,
        #[ts(type = "unknown")]
        tool_input: serde_json::Value,
        output: Option<String>,
        is_error: Option<bool>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ChatMessage {
    pub id: String,
    pub role: MessageRole,
    pub content_blocks: Vec<ChatContentBlock>,
}

impl ChatMessage {
    pub fn new(role: MessageRole, content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role,
            content_blocks: vec![ChatContentBlock::Text { text: content }],
        }
    }

    pub fn with_blocks(role: MessageRole, blocks: Vec<ChatContentBlock>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role,
            content_blocks: blocks,
        }
    }

    /// Get concatenated text content from all text blocks
    pub fn get_text(&self) -> String {
        self.content_blocks
            .iter()
            .filter_map(|block| match block {
                ChatContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AgentChunkPayload {
    pub delta: String,
    pub block_index: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AgentCompletePayload {
    pub message_id: String,
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AgentErrorPayload {
    pub error: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ToolStartPayload {
    pub tool_use_id: String,
    pub tool_name: String,
    #[ts(type = "unknown")]
    pub tool_input: serde_json::Value,
    pub block_index: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ToolEndPayload {
    pub tool_use_id: String,
    pub output: String,
    pub is_error: bool,
    pub block_index: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ContentBlockStartPayload {
    pub block_index: u32,
    pub block_type: ContentBlockType,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlockType {
    Text,
    ToolUse {
        tool_use_id: String,
        tool_name: String,
    },
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AgentStatusPayload {
    pub status: AgentStatus,
    pub status_text: String,
    pub detail: Option<String>,
}

impl AgentStatusPayload {
    pub fn new(status: AgentStatus, detail: Option<String>) -> Self {
        let status_text = status.display_text(&detail);
        Self {
            status,
            status_text,
            detail,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Idle,
    Sending,
    Thinking,
    Streaming,
    ToolRunning,
    ToolWaiting,
    Cancelled,
    Error,
}

impl AgentStatus {
    pub fn display_text(&self, detail: &Option<String>) -> String {
        match self {
            AgentStatus::Idle => String::new(),
            AgentStatus::Sending => "Sending...".to_string(),
            AgentStatus::Thinking => "Thinking...".to_string(),
            AgentStatus::Streaming => "Generating...".to_string(),
            AgentStatus::ToolRunning => {
                if let Some(tool_name) = detail {
                    format!("Running {}...", tool_name)
                } else {
                    "Running tool...".to_string()
                }
            }
            AgentStatus::ToolWaiting => "Waiting for response...".to_string(),
            AgentStatus::Cancelled => "Cancelled".to_string(),
            AgentStatus::Error => "Error".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AgentCancelledPayload {
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct PlanReadyPayload {
    pub plan: String,
}
