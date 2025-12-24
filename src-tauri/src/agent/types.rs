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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ChatMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
}

impl ChatMessage {
    pub fn new(role: MessageRole, content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role,
            content,
        }
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
    /// JSON string of the tool input
    pub tool_input: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ToolEndPayload {
    pub tool_use_id: String,
    pub output: String,
    pub is_error: bool,
}
