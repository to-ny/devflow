use serde::{Deserialize, Serialize};
use ts_rs::TS;

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
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct ProjectConfig {
    pub agent: AgentConfig,
    #[serde(default)]
    pub prompts: PromptsConfig,
    #[serde(default)]
    pub execution: ExecutionConfig,
    #[serde(default)]
    pub notifications: NotificationsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct AgentConfig {
    pub provider: String,
    pub model: String,
    pub api_key_env: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct PromptsConfig {
    #[serde(default)]
    pub pre: String,
    #[serde(default)]
    pub post: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct ExecutionConfig {
    #[serde(default = "default_execution_mode")]
    pub mode: ExecutionMode,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub patterns: ExecutionPatterns,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            mode: ExecutionMode::Local,
            timeout_secs: default_timeout_secs(),
            patterns: ExecutionPatterns::default(),
        }
    }
}

fn default_execution_mode() -> ExecutionMode {
    ExecutionMode::Local
}

fn default_timeout_secs() -> u64 {
    120
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/types/generated/")]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    #[default]
    Local,
    Container,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct ExecutionPatterns {
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct NotificationsConfig {
    #[serde(default)]
    pub on_complete: Vec<NotificationAction>,
    #[serde(default)]
    pub on_error: Vec<NotificationAction>,
    #[serde(default)]
    pub on_permission_request: Vec<NotificationAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/types/generated/")]
#[serde(rename_all = "lowercase")]
pub enum NotificationAction {
    Sound,
    Window,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct SavedPermissions {
    #[serde(default)]
    pub allowed: PermissionEntries,
    #[serde(default)]
    pub denied: PermissionEntries,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct PermissionEntries {
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub patterns: Vec<String>,
}
