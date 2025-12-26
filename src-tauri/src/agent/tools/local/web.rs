use log::debug;

use super::context::{ExecutionContext, MAX_OUTPUT_SIZE};
use crate::agent::error::AgentError;
use crate::agent::tools::types::WebFetchInput;

pub async fn fetch(ctx: &ExecutionContext, input: serde_json::Value) -> Result<String, AgentError> {
    let input: WebFetchInput = serde_json::from_value(input)
        .map_err(|e| AgentError::InvalidToolInput(format!("Invalid input: {}", e)))?;

    debug!("Fetching URL: {}", input.url);

    let client = reqwest::Client::builder()
        .timeout(ctx.timeout)
        .build()
        .map_err(|e| AgentError::ToolExecutionError(format!("Failed to create client: {}", e)))?;

    let response = client
        .get(&input.url)
        .send()
        .await
        .map_err(|e| AgentError::ToolExecutionError(format!("Request failed: {}", e)))?;

    let status = response.status();
    if !status.is_success() {
        return Err(AgentError::ToolExecutionError(format!(
            "HTTP error: {}",
            status
        )));
    }

    let content = response
        .text()
        .await
        .map_err(|e| AgentError::ToolExecutionError(format!("Failed to read response: {}", e)))?;

    let content = if content.len() > MAX_OUTPUT_SIZE {
        format!(
            "{}...\n(truncated, {} bytes total)",
            &content[..MAX_OUTPUT_SIZE],
            content.len()
        )
    } else {
        content
    };

    Ok(content)
}
