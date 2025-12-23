use async_trait::async_trait;

use super::types::ToolName;
use crate::agent::error::AgentError;

#[async_trait]
pub trait ToolExecutor: Send + Sync {
    async fn execute(&self, tool: ToolName, input: serde_json::Value)
        -> Result<String, AgentError>;
}
