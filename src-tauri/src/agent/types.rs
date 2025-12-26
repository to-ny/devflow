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

/// Tool execution record for message history
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ToolExecution {
    pub tool_use_id: String,
    pub tool_name: String,
    #[ts(type = "unknown")]
    pub tool_input: serde_json::Value,
    pub output: Option<String>,
    pub is_error: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ChatMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    #[ts(optional)]
    pub tool_executions: Option<Vec<ToolExecution>>,
}

impl ChatMessage {
    pub fn new(role: MessageRole, content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role,
            content,
            tool_executions: None,
        }
    }

    pub fn with_tools(role: MessageRole, content: String, tools: Vec<ToolExecution>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role,
            content,
            tool_executions: if tools.is_empty() { None } else { Some(tools) },
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
    #[ts(type = "unknown")]
    pub tool_input: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ToolEndPayload {
    pub tool_use_id: String,
    pub output: String,
    pub is_error: bool,
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
