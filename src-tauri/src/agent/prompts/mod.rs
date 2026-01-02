//! Agent type registry and prompt loading.

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

/// Pre-compiled regex for {TOOL:name} patterns
static TOOL_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"\{TOOL:(\w+)\}").unwrap());

/// Pre-compiled regex for {AGENT:id} patterns
static AGENT_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"\{AGENT:(\w+)\}").unwrap());

/// Agent prompt files compiled into the binary
mod agent_prompts {
    pub const EXPLORE: &str = include_str!("agents/explore.md");
    pub const PLAN: &str = include_str!("agents/plan.md");
    pub const SUMMARIZE: &str = include_str!("agents/summarize.md");
    pub const BASH_SUMMARIZE: &str = include_str!("agents/bash_summarize.md");
    pub const SESSION_TITLE: &str = include_str!("agents/session_title.md");
    pub const PR_REVIEW: &str = include_str!("agents/pr_review.md");
    pub const PR_COMMENTS: &str = include_str!("agents/pr_comments.md");
    pub const SECURITY_REVIEW: &str = include_str!("agents/security_review.md");
}

/// System reminder files compiled into the binary.
/// TODO: Integrate these into the conversation flow:
/// - PLAN_MODE_*: Inject when plan mode is active/entered
/// - EMPTY_TODO_LIST: Inject when user hasn't used todo_write recently
/// - STALE_TODO_LIST: Inject when todo list appears outdated
#[allow(dead_code)]
pub mod system_reminders {
    pub const PLAN_MODE_ACTIVE: &str = include_str!("system_reminders/plan_mode_active.md");
    pub const PLAN_MODE_SUBAGENT: &str = include_str!("system_reminders/plan_mode_subagent.md");
    pub const PLAN_MODE_REENTRY: &str = include_str!("system_reminders/plan_mode_reentry.md");
    pub const EMPTY_TODO_LIST: &str = include_str!("system_reminders/empty_todo_list.md");
    pub const STALE_TODO_LIST: &str = include_str!("system_reminders/stale_todo_list.md");
}

/// Agent type metadata for frontend display
#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentTypeInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// Flags that control agent behavior
#[derive(Debug, Clone)]
pub struct AgentFlags {
    /// Agent cannot modify files
    pub read_only: bool,
    /// Agent can run in background mode
    pub can_background: bool,
    /// Agent has no tool access (text processing only)
    pub no_tools: bool,
}

impl Default for AgentFlags {
    fn default() -> Self {
        Self {
            read_only: true,
            can_background: false,
            no_tools: false,
        }
    }
}

/// Definition of an agent type
#[derive(Debug, Clone)]
pub struct AgentType {
    /// Unique identifier
    pub id: &'static str,
    /// Human-readable name
    pub name: &'static str,
    /// Brief description
    pub description: &'static str,
    /// System prompt content
    pub prompt: &'static str,
    /// Allowed tool names
    pub allowed_tools: &'static [&'static str],
    /// Behavioral flags
    pub flags: AgentFlags,
}

/// Tools for read-only exploration
const EXPLORE_TOOLS: &[&str] = &[
    "read_file",
    "list_directory",
    "glob",
    "grep",
    "bash",
    "web_fetch",
    "search_web",
];

/// Tools for planning (includes dispatch_agent for sub-exploration)
const PLAN_TOOLS: &[&str] = &[
    "read_file",
    "list_directory",
    "glob",
    "grep",
    "bash",
    "dispatch_agent",
];

/// Tools for PR review
const PR_REVIEW_TOOLS: &[&str] = &["read_file", "glob", "grep", "bash"];

/// Tools for PR comments
const PR_COMMENTS_TOOLS: &[&str] = &["bash", "web_fetch"];

/// Tools for security review
const SECURITY_REVIEW_TOOLS: &[&str] = &["read_file", "glob", "grep"];

/// No tools (text processing only)
const NO_TOOLS: &[&str] = &[];

/// Static registry of all agent types
static AGENT_REGISTRY: Lazy<HashMap<&'static str, AgentType>> = Lazy::new(|| {
    let agents = vec![
        AgentType {
            id: "explore",
            name: "Explore",
            description: "Fast codebase exploration, file search, read-only analysis",
            prompt: agent_prompts::EXPLORE,
            allowed_tools: EXPLORE_TOOLS,
            flags: AgentFlags {
                read_only: true,
                can_background: true,
                no_tools: false,
            },
        },
        AgentType {
            id: "plan",
            name: "Plan",
            description: "Software architect for designing implementation plans",
            prompt: agent_prompts::PLAN,
            allowed_tools: PLAN_TOOLS,
            flags: AgentFlags {
                read_only: true,
                can_background: false,
                no_tools: false,
            },
        },
        AgentType {
            id: "summarize",
            name: "Summarize",
            description: "Conversation summarization for context compaction",
            prompt: agent_prompts::SUMMARIZE,
            allowed_tools: NO_TOOLS,
            flags: AgentFlags {
                read_only: true,
                can_background: false,
                no_tools: true,
            },
        },
        AgentType {
            id: "bash-summarize",
            name: "Bash Summarize",
            description: "Summarize long bash command outputs",
            prompt: agent_prompts::BASH_SUMMARIZE,
            allowed_tools: NO_TOOLS,
            flags: AgentFlags {
                read_only: true,
                can_background: false,
                no_tools: true,
            },
        },
        AgentType {
            id: "session-title",
            name: "Session Title",
            description: "Generate session titles and branch names",
            prompt: agent_prompts::SESSION_TITLE,
            allowed_tools: NO_TOOLS,
            flags: AgentFlags {
                read_only: true,
                can_background: false,
                no_tools: true,
            },
        },
        AgentType {
            id: "pr-review",
            name: "PR Review",
            description: "Review pull requests",
            prompt: agent_prompts::PR_REVIEW,
            allowed_tools: PR_REVIEW_TOOLS,
            flags: AgentFlags {
                read_only: true,
                can_background: false,
                no_tools: false,
            },
        },
        AgentType {
            id: "pr-comments",
            name: "PR Comments",
            description: "Fetch and analyze PR comments",
            prompt: agent_prompts::PR_COMMENTS,
            allowed_tools: PR_COMMENTS_TOOLS,
            flags: AgentFlags {
                read_only: true,
                can_background: false,
                no_tools: false,
            },
        },
        AgentType {
            id: "security-review",
            name: "Security Review",
            description: "Security-focused code review",
            prompt: agent_prompts::SECURITY_REVIEW,
            allowed_tools: SECURITY_REVIEW_TOOLS,
            flags: AgentFlags {
                read_only: true,
                can_background: false,
                no_tools: false,
            },
        },
    ];

    agents.into_iter().map(|a| (a.id, a)).collect()
});

/// Get an agent type by ID
pub fn get_agent_type(id: &str) -> Option<&'static AgentType> {
    AGENT_REGISTRY.get(id)
}

/// Get the default agent type (explore)
pub fn get_default_agent_type() -> &'static AgentType {
    AGENT_REGISTRY
        .get("explore")
        .expect("explore agent must exist")
}

/// Get all registered agent types
pub fn get_all_agent_types() -> Vec<&'static AgentType> {
    AGENT_REGISTRY.values().collect()
}

/// Get a formatted list of agent types for use in prompts
pub fn get_agent_types_description() -> String {
    let mut desc = String::new();
    for agent in AGENT_REGISTRY.values() {
        desc.push_str(&format!("- `{}`: {}\n", agent.id, agent.description));
    }
    desc
}

/// Get all agent prompts as a HashMap for settings UI
pub fn get_agent_prompts() -> std::collections::HashMap<String, String> {
    AGENT_REGISTRY
        .iter()
        .map(|(id, agent)| (id.to_string(), agent.prompt.to_string()))
        .collect()
}

/// Get all agent type metadata for frontend display
pub fn get_agent_type_infos() -> Vec<AgentTypeInfo> {
    AGENT_REGISTRY
        .values()
        .map(|agent| AgentTypeInfo {
            id: agent.id.to_string(),
            name: agent.name.to_string(),
            description: agent.description.to_string(),
        })
        .collect()
}

/// Replace {ALLOWED_TOOLS}, {PROJECT_PATH}, {CURRENT_DATE}, {TOOL:name}, {AGENT:id} in prompt.
pub fn interpolate_prompt(
    prompt: &str,
    allowed_tools: &[&str],
    project_path: Option<&str>,
) -> String {
    let mut result = prompt.to_string();

    // Replace {ALLOWED_TOOLS}
    result = result.replace("{ALLOWED_TOOLS}", &allowed_tools.join(", "));

    // Replace {PROJECT_PATH}
    if let Some(path) = project_path {
        result = result.replace("{PROJECT_PATH}", path);
    }

    // Replace {CURRENT_DATE}
    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    result = result.replace("{CURRENT_DATE}", &date);

    // Replace {TOOL:name} patterns
    result = TOOL_PATTERN
        .replace_all(&result, |caps: &regex::Captures| {
            caps.get(1).map_or("", |m| m.as_str()).to_string()
        })
        .to_string();

    // Replace {AGENT:id} patterns
    result = AGENT_PATTERN
        .replace_all(&result, |caps: &regex::Captures| {
            caps.get(1).map_or("", |m| m.as_str()).to_string()
        })
        .to_string();

    result
}

/// Format a system reminder for injection
pub fn format_system_reminder(content: &str) -> String {
    format!("<system-reminder>\n{}\n</system-reminder>", content.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_agent_type() {
        assert!(get_agent_type("explore").is_some());
        assert!(get_agent_type("plan").is_some());
        assert!(get_agent_type("summarize").is_some());
        assert!(get_agent_type("nonexistent").is_none());
    }

    #[test]
    fn test_all_agent_types_exist() {
        let expected = [
            "explore",
            "plan",
            "summarize",
            "bash-summarize",
            "session-title",
            "pr-review",
            "pr-comments",
            "security-review",
        ];
        for id in expected {
            assert!(get_agent_type(id).is_some(), "Agent '{}' should exist", id);
        }
    }

    #[test]
    fn test_default_agent() {
        let agent = get_default_agent_type();
        assert_eq!(agent.id, "explore");
    }

    #[test]
    fn test_get_all_agent_types() {
        let agents = get_all_agent_types();
        assert!(!agents.is_empty());
        assert!(agents.len() >= 8);
    }

    #[test]
    fn test_get_agent_prompts() {
        let prompts = get_agent_prompts();
        assert!(prompts.contains_key("explore"));
        assert!(prompts.contains_key("plan"));
        assert!(!prompts.get("explore").unwrap().is_empty());
    }

    #[test]
    fn test_get_agent_type_infos() {
        let infos = get_agent_type_infos();
        assert!(!infos.is_empty());
        let explore_info = infos.iter().find(|i| i.id == "explore");
        assert!(explore_info.is_some());
        assert_eq!(explore_info.unwrap().name, "Explore");
    }

    #[test]
    fn test_interpolate_prompt() {
        let prompt = "Tools: {ALLOWED_TOOLS}, Date: {CURRENT_DATE}";
        let result = interpolate_prompt(prompt, &["read_file", "glob"], None);
        assert!(result.contains("read_file, glob"));
        assert!(result.contains("202")); // Year prefix
    }

    #[test]
    fn test_interpolate_prompt_with_project_path() {
        let prompt = "Path: {PROJECT_PATH}";
        let result = interpolate_prompt(prompt, &[], Some("/test/project"));
        assert!(result.contains("/test/project"));
    }

    #[test]
    fn test_interpolate_prompt_tool_pattern() {
        let prompt = "Use {TOOL:read_file} to read files";
        let result = interpolate_prompt(prompt, &[], None);
        assert!(result.contains("read_file"));
        assert!(!result.contains("{TOOL:"));
    }

    #[test]
    fn test_interpolate_prompt_agent_pattern() {
        let prompt = "Dispatch {AGENT:explore} agent";
        let result = interpolate_prompt(prompt, &[], None);
        assert!(result.contains("explore"));
        assert!(!result.contains("{AGENT:"));
    }

    #[test]
    fn test_format_system_reminder() {
        let reminder = format_system_reminder("Test reminder");
        assert!(reminder.starts_with("<system-reminder>"));
        assert!(reminder.ends_with("</system-reminder>"));
        assert!(reminder.contains("Test reminder"));
    }

    #[test]
    fn test_agent_flags() {
        let explore = get_agent_type("explore").unwrap();
        assert!(explore.flags.read_only);
        assert!(explore.flags.can_background);
        assert!(!explore.flags.no_tools);

        let summarize = get_agent_type("summarize").unwrap();
        assert!(summarize.flags.no_tools);
    }

    #[test]
    fn test_get_agent_types_description() {
        let desc = get_agent_types_description();
        assert!(desc.contains("explore"));
        assert!(desc.contains("plan"));
    }
}
