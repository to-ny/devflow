use std::path::Path;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::agent::error::AgentError;
use crate::agent::provider::ProviderAdapter;
use crate::agent::tools::get_tool_definitions;
use crate::agent::types::{ChatMessage, MessageRole, ToolDefinition};
use crate::agent::usage::SessionUsageTracker;
use crate::config::ConfigService;

use super::state::SessionState;

const DEFAULT_SUBAGENT_TOOLS: &[&str] = &[
    "read_file",
    "list_directory",
    "glob",
    "grep",
    "web_fetch",
    "search_web",
    "todo_read",
];

pub async fn execute_subagent(
    project_path: &Path,
    task: &str,
    allowed_tools: Option<Vec<String>>,
    max_depth: u32,
    current_depth: u32,
    parent_token: &CancellationToken,
    usage_tracker: Arc<SessionUsageTracker>,
) -> Result<String, AgentError> {
    // Check for cancellation before starting
    if parent_token.is_cancelled() {
        return Err(AgentError::Cancelled);
    }

    if current_depth >= max_depth {
        return Err(AgentError::ToolExecutionError(format!(
            "Maximum sub-agent depth ({}) exceeded",
            max_depth
        )));
    }

    // Load project config
    let config = ConfigService::load_project_config(project_path)
        .map_err(|e| AgentError::ConfigError(e.to_string()))?;

    // Create provider adapter
    let provider = create_subagent_provider(project_path, &config)?;

    let tools = filter_tools(allowed_tools)?;
    // Create fresh session for sub-agent
    let session = SessionState::new();

    // Build system prompt for sub-agent
    let system_prompt = build_subagent_system_prompt(&tools);

    // Create initial message
    let messages = vec![ChatMessage::new(MessageRole::User, task.to_string())];

    let cancel_token = parent_token.child_token();

    use crate::agent::provider::ExecutionContext;

    let ctx = ExecutionContext {
        session,
        cancel_token,
        usage_tracker,
    };
    let result = provider
        .run_headless(
            messages,
            Some(system_prompt),
            None, // Sub-agents don't use project memory
            tools,
            ctx,
        )
        .await?;

    Ok(result.text)
}

fn create_subagent_provider(
    project_path: &Path,
    config: &crate::config::ProjectConfig,
) -> Result<Box<dyn ProviderAdapter>, AgentError> {
    use crate::agent::providers::{AnthropicAdapter, GeminiAdapter, DEFAULT_SYSTEM_PROMPT};

    let provider = config.agent.provider.to_lowercase();

    match provider.as_str() {
        "anthropic" => {
            let adapter = AnthropicAdapter::new(
                config.agent.clone(),
                config.prompts.clone(),
                config.execution.clone(),
                project_path.to_path_buf(),
                DEFAULT_SYSTEM_PROMPT,
                config.extraction_prompt.clone(),
            )?;
            Ok(Box::new(adapter))
        }
        "gemini" => {
            let adapter = GeminiAdapter::new(
                config.agent.clone(),
                config.prompts.clone(),
                config.execution.clone(),
                project_path.to_path_buf(),
                DEFAULT_SYSTEM_PROMPT,
                config.extraction_prompt.clone(),
            )?;
            Ok(Box::new(adapter))
        }
        _ => Err(AgentError::UnsupportedProvider(provider)),
    }
}

fn filter_tools(allowed_tools: Option<Vec<String>>) -> Result<Vec<ToolDefinition>, AgentError> {
    let all_tools = get_tool_definitions();

    let allowed: Vec<&str> = match &allowed_tools {
        Some(tools) => tools.iter().map(|s| s.as_str()).collect(),
        None => DEFAULT_SUBAGENT_TOOLS.to_vec(),
    };

    let filtered: Vec<ToolDefinition> = all_tools
        .into_iter()
        .filter(|t| allowed.contains(&t.name.as_str()))
        .collect();

    if filtered.is_empty() {
        return Err(AgentError::ToolExecutionError(
            "No valid tools available for sub-agent".to_string(),
        ));
    }

    Ok(filtered)
}

fn build_subagent_system_prompt(tools: &[ToolDefinition]) -> String {
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

    format!(
        r#"You are a focused sub-agent with a specific task to complete.

Your goal is to complete the assigned task efficiently and return a clear result.

## Constraints
- You have access to a limited set of read-only tools: {}
- Focus on gathering information and providing a comprehensive answer
- Do not attempt to modify files or execute destructive commands
- Return your findings in a clear, structured format

When you have completed the task, provide your final answer directly without using any tools."#,
        tool_names.join(", ")
    )
}
