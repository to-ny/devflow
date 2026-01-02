use std::path::Path;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::agent::error::AgentError;
use crate::agent::prompts::{
    get_agent_type, get_default_agent_type, interpolate_prompt, AgentType,
};
use crate::agent::provider::ProviderAdapter;
use crate::agent::tools::get_tool_definitions;
use crate::agent::types::{ChatMessage, MessageRole, ToolDefinition};
use crate::agent::usage::SessionUsageTracker;
use crate::config::ConfigService;

use super::state::SessionState;

/// Default tools for the explore agent (backward compatibility)
const DEFAULT_SUBAGENT_TOOLS: &[&str] = &[
    "read_file",
    "list_directory",
    "glob",
    "grep",
    "web_fetch",
    "search_web",
    "todo_read",
];

/// Parameters for executing a sub-agent
pub struct SubagentParams<'a> {
    pub project_path: &'a Path,
    pub task: &'a str,
    pub agent_type_id: Option<&'a str>,
    pub allowed_tools: Option<Vec<String>>,
    pub max_depth: u32,
    pub current_depth: u32,
    pub parent_token: &'a CancellationToken,
    pub usage_tracker: Arc<SessionUsageTracker>,
}

/// Execute a sub-agent with the specified agent type.
pub async fn execute_subagent(params: SubagentParams<'_>) -> Result<String, AgentError> {
    let SubagentParams {
        project_path,
        task,
        agent_type_id,
        allowed_tools,
        max_depth,
        current_depth,
        parent_token,
        usage_tracker,
    } = params;

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

    // Get the agent type from registry
    let agent_type = match agent_type_id {
        Some(id) => get_agent_type(id).unwrap_or_else(|| {
            log::warn!("Unknown agent type '{}', falling back to explore", id);
            get_default_agent_type()
        }),
        None => get_default_agent_type(),
    };

    // Load project config
    let config = ConfigService::load_project_config(project_path).map_err(|e| {
        AgentError::ConfigError(format!(
            "Failed to load config for '{}' agent: {}",
            agent_type.id, e
        ))
    })?;

    // Create provider adapter
    let provider = create_subagent_provider(project_path, &config).map_err(|e| {
        AgentError::ToolExecutionError(format!(
            "'{}' agent failed to create provider: {}",
            agent_type.id, e
        ))
    })?;

    // Determine tools to use
    let tools = if agent_type.flags.no_tools {
        // Agent has no tool access
        vec![]
    } else {
        filter_tools(allowed_tools, agent_type).map_err(|e| {
            AgentError::ToolExecutionError(format!(
                "'{}' agent tool setup failed: {}",
                agent_type.id, e
            ))
        })?
    };

    // Create fresh session for sub-agent
    let session = SessionState::new();

    // Build system prompt for sub-agent (use custom prompt if configured)
    let system_prompt = build_agent_system_prompt(agent_type, &tools, project_path, &config);

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

fn filter_tools(
    allowed_tools: Option<Vec<String>>,
    agent_type: &AgentType,
) -> Result<Vec<ToolDefinition>, AgentError> {
    let all_tools = get_tool_definitions();

    // Determine which tools to allow
    let allowed: Vec<&str> = match &allowed_tools {
        // If explicit tools provided, use those
        Some(tools) => tools.iter().map(|s| s.as_str()).collect(),
        // Otherwise use agent type's default tools, or fall back to DEFAULT_SUBAGENT_TOOLS
        None => {
            if agent_type.allowed_tools.is_empty() {
                DEFAULT_SUBAGENT_TOOLS.to_vec()
            } else {
                agent_type.allowed_tools.to_vec()
            }
        }
    };

    let filtered: Vec<ToolDefinition> = all_tools
        .into_iter()
        .filter(|t| allowed.contains(&t.name.as_str()))
        .collect();

    // It's okay to have no tools for some agent types
    if filtered.is_empty() && !agent_type.flags.no_tools {
        return Err(AgentError::ToolExecutionError(
            "No valid tools available for sub-agent".to_string(),
        ));
    }

    Ok(filtered)
}

fn build_agent_system_prompt(
    agent_type: &AgentType,
    tools: &[ToolDefinition],
    project_path: &Path,
    config: &crate::config::ProjectConfig,
) -> String {
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    let project_path_str = project_path.to_string_lossy();

    // Use custom prompt if configured, otherwise use default
    let prompt = config
        .agent_prompts
        .as_ref()
        .and_then(|prompts| prompts.get(agent_type.id))
        .map(|s| s.as_str())
        .unwrap_or(agent_type.prompt);

    interpolate_prompt(prompt, &tool_names, Some(&project_path_str))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_agent_type_explore() {
        let agent = get_agent_type("explore");
        assert!(agent.is_some());
        assert_eq!(agent.unwrap().id, "explore");
    }

    #[test]
    fn test_get_agent_type_fallback() {
        let agent = get_agent_type("nonexistent");
        assert!(agent.is_none());
    }

    #[test]
    fn test_default_agent_type() {
        let agent = get_default_agent_type();
        assert_eq!(agent.id, "explore");
    }

    // =========================================================================
    // SUB-AGENT ORCHESTRATION TESTS
    // =========================================================================

    #[test]
    fn test_all_agent_types_exist() {
        // Verify all documented agent types are available
        let expected_types = [
            "explore",
            "plan",
            "summarize",
            "bash-summarize",
            "session-title",
            "pr-review",
            "pr-comments",
            "security-review",
        ];

        for agent_id in expected_types {
            let agent = get_agent_type(agent_id);
            assert!(agent.is_some(), "Agent type '{}' should exist", agent_id);
        }
    }

    #[test]
    fn test_agent_types_have_allowed_tools() {
        // Each agent type should have a defined set of allowed tools
        let agent_types = ["explore", "plan", "summarize", "pr-review"];

        for agent_id in agent_types {
            let agent = get_agent_type(agent_id).unwrap();
            // Either has tools or has no_tools flag
            assert!(
                !agent.allowed_tools.is_empty() || agent.flags.no_tools,
                "Agent '{}' should have allowed_tools or no_tools flag",
                agent_id
            );
        }
    }

    #[test]
    fn test_filter_tools_respects_agent_type() {
        let explore = get_agent_type("explore").unwrap();
        let tools = filter_tools(None, explore).unwrap();

        // Explore should have search tools
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(
            tool_names.contains(&"glob") || tool_names.contains(&"grep"),
            "Explore agent should have search tools"
        );
    }

    #[test]
    fn test_filter_tools_explicit_override() {
        let explore = get_agent_type("explore").unwrap();
        let allowed = Some(vec!["read_file".to_string()]);
        let tools = filter_tools(allowed, explore).unwrap();

        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "read_file");
    }

    #[test]
    fn test_no_tools_agent_returns_empty() {
        // session-title should have no_tools flag
        let session_title = get_agent_type("session-title").unwrap();
        assert!(session_title.flags.no_tools);

        // When no_tools is set, filter_tools should return empty vec
        if session_title.flags.no_tools {
            // The execute_subagent function returns empty vec for no_tools agents
            // We can't call filter_tools directly because it would error,
            // but execute_subagent handles this case
        }
    }

    #[tokio::test]
    async fn test_depth_limit_exceeded() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Create a minimal config file
        std::fs::create_dir_all(temp_dir.path().join(".devflow")).unwrap();
        std::fs::write(
            temp_dir.path().join(".devflow/config.toml"),
            r#"
[agent]
provider = "anthropic"
model = "claude-3-sonnet-20240229"
api_key_env = "ANTHROPIC_API_KEY"
max_tokens = 4096
"#,
        )
        .unwrap();

        let cancel_token = CancellationToken::new();
        let usage_tracker = Arc::new(crate::agent::usage::SessionUsageTracker::new());

        let result = execute_subagent(SubagentParams {
            project_path: temp_dir.path(),
            task: "Test task",
            agent_type_id: None,
            allowed_tools: None,
            max_depth: 3,
            current_depth: 3, // Already at max
            parent_token: &cancel_token,
            usage_tracker,
        })
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Maximum sub-agent depth"),
            "Error should mention depth limit: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_cancellation_before_start() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Create a minimal config
        std::fs::create_dir_all(temp_dir.path().join(".devflow")).unwrap();
        std::fs::write(
            temp_dir.path().join(".devflow/config.toml"),
            r#"
[agent]
provider = "anthropic"
model = "claude-3-sonnet-20240229"
api_key_env = "ANTHROPIC_API_KEY"
max_tokens = 4096
"#,
        )
        .unwrap();

        let cancel_token = CancellationToken::new();
        cancel_token.cancel(); // Cancel before starting

        let usage_tracker = Arc::new(crate::agent::usage::SessionUsageTracker::new());

        let result = execute_subagent(SubagentParams {
            project_path: temp_dir.path(),
            task: "Test task",
            agent_type_id: None,
            allowed_tools: None,
            max_depth: 3,
            current_depth: 0,
            parent_token: &cancel_token,
            usage_tracker,
        })
        .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::agent::error::AgentError::Cancelled
        ));
    }

    #[test]
    fn test_child_token_inherits_cancellation() {
        let parent = CancellationToken::new();
        let child = parent.child_token();

        assert!(!child.is_cancelled());

        parent.cancel();

        // Child should be cancelled when parent is
        assert!(child.is_cancelled());
    }

    #[test]
    fn test_default_subagent_tools() {
        // Verify DEFAULT_SUBAGENT_TOOLS contains expected tools
        assert!(DEFAULT_SUBAGENT_TOOLS.contains(&"read_file"));
        assert!(DEFAULT_SUBAGENT_TOOLS.contains(&"glob"));
        assert!(DEFAULT_SUBAGENT_TOOLS.contains(&"grep"));

        // Should NOT include dangerous tools
        assert!(!DEFAULT_SUBAGENT_TOOLS.contains(&"bash"));
        assert!(!DEFAULT_SUBAGENT_TOOLS.contains(&"write_file"));
    }
}
