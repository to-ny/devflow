use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ConfigChangedPayload {
    pub project_path: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub models: Vec<String>,
    pub default_api_key_env: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub state: AppState,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppState {
    pub last_project: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectConfig {
    pub agent: AgentConfig,
    pub execution: ExecutionConfig,
    #[serde(default)]
    pub search: SearchConfig,
    #[serde(default)]
    pub notifications: NotificationsConfig,
    #[serde(default)]
    pub prompts: PromptsConfig,
    /// Custom system prompt (None = use default)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// Custom tool descriptions (None = use defaults)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_descriptions: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SearchConfig {
    #[serde(default = "default_search_provider")]
    pub provider: String,
    #[serde(default = "default_max_results")]
    pub max_results: u32,
}

fn default_search_provider() -> String {
    "duckduckgo".to_string()
}

fn default_max_results() -> u32 {
    10
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            provider: default_search_provider(),
            max_results: default_max_results(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AgentConfig {
    pub provider: String,
    pub model: String,
    pub api_key_env: String,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PromptsConfig {
    #[serde(default)]
    pub pre: String,
    #[serde(default)]
    pub post: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExecutionConfig {
    #[ts(type = "number")]
    pub timeout_secs: u64,
    pub max_tool_iterations: u32,
    #[serde(default = "default_max_agent_depth")]
    pub max_agent_depth: u32,
}

fn default_max_agent_depth() -> u32 {
    3
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NotificationsConfig {
    #[serde(default)]
    pub on_complete: Vec<NotificationAction>,
    #[serde(default)]
    pub on_error: Vec<NotificationAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "lowercase")]
pub enum NotificationAction {
    Sound,
    Window,
}
