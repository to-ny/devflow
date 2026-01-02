use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolName {
    // File & Shell Tools
    Bash,
    ReadFile,
    WriteFile,
    EditFile,
    MultiEdit,
    ListDirectory,
    Glob,
    Grep,
    // Notebook Tools
    NotebookRead,
    NotebookEdit,
    // Web Tools
    WebFetch,
    SearchWeb,
    // Task Management Tools
    TodoRead,
    TodoWrite,
    DispatchAgent,
    SubmitPlan,
}

impl ToolName {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "bash" => Some(ToolName::Bash),
            "read_file" => Some(ToolName::ReadFile),
            "write_file" => Some(ToolName::WriteFile),
            "edit_file" => Some(ToolName::EditFile),
            "multi_edit" => Some(ToolName::MultiEdit),
            "list_directory" => Some(ToolName::ListDirectory),
            "glob" => Some(ToolName::Glob),
            "grep" => Some(ToolName::Grep),
            "notebook_read" => Some(ToolName::NotebookRead),
            "notebook_edit" => Some(ToolName::NotebookEdit),
            "web_fetch" => Some(ToolName::WebFetch),
            "search_web" => Some(ToolName::SearchWeb),
            "todo_read" => Some(ToolName::TodoRead),
            "todo_write" => Some(ToolName::TodoWrite),
            "dispatch_agent" => Some(ToolName::DispatchAgent),
            "submit_plan" => Some(ToolName::SubmitPlan),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ToolName::Bash => "bash",
            ToolName::ReadFile => "read_file",
            ToolName::WriteFile => "write_file",
            ToolName::EditFile => "edit_file",
            ToolName::MultiEdit => "multi_edit",
            ToolName::ListDirectory => "list_directory",
            ToolName::Glob => "glob",
            ToolName::Grep => "grep",
            ToolName::NotebookRead => "notebook_read",
            ToolName::NotebookEdit => "notebook_edit",
            ToolName::WebFetch => "web_fetch",
            ToolName::SearchWeb => "search_web",
            ToolName::TodoRead => "todo_read",
            ToolName::TodoWrite => "todo_write",
            ToolName::DispatchAgent => "dispatch_agent",
            ToolName::SubmitPlan => "submit_plan",
        }
    }
}

// File & Shell Tool Inputs

#[derive(Debug, Clone, Deserialize)]
pub struct BashInput {
    pub command: String,
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReadFileInput {
    pub path: String,
    pub offset: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WriteFileInput {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EditFileInput {
    pub path: String,
    pub old_text: String,
    pub new_text: String,
    pub replace_all: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EditOperation {
    pub old_text: String,
    pub new_text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MultiEditInput {
    pub path: String,
    pub edits: Vec<EditOperation>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListDirectoryInput {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GlobInput {
    pub pattern: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GrepInput {
    pub pattern: String,
    pub path: Option<String>,
    pub include: Option<String>,
}

// Notebook Tool Inputs

#[derive(Debug, Clone, Deserialize)]
pub struct NotebookReadInput {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NotebookEditInput {
    pub path: String,
    pub cell_number: u32,
    pub new_source: String,
    pub cell_type: Option<String>,
    pub edit_mode: Option<String>,
}

// Web Tool Inputs

#[derive(Debug, Clone, Deserialize)]
pub struct WebFetchInput {
    pub url: String,
    #[serde(rename = "prompt")]
    pub _prompt: Option<String>, // Not yet implemented
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebSearchInput {
    pub query: String,
    pub allowed_domains: Option<Vec<String>>,
    pub blocked_domains: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubmitPlanInput {
    pub plan: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DispatchAgentInput {
    pub task: String,
    /// Agent type to use (e.g., "explore", "plan", "pr-review")
    #[serde(default)]
    pub agent_type: Option<String>,
    /// Override allowed tools for the agent
    #[serde(default)]
    pub tools: Option<Vec<String>>,
}

// Task Management Tool Inputs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub content: String,
    pub status: String,
    pub priority: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TodoWriteInput {
    pub todos: Vec<TodoItem>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_bash_input_deserializes() {
        let input: BashInput = serde_json::from_value(json!({
            "command": "ls -la"
        }))
        .unwrap();
        assert_eq!(input.command, "ls -la");
        assert!(input.timeout.is_none());

        let input_with_timeout: BashInput = serde_json::from_value(json!({
            "command": "sleep 5",
            "timeout": 10
        }))
        .unwrap();
        assert_eq!(input_with_timeout.timeout, Some(10));
    }

    #[test]
    fn test_read_file_input_deserializes() {
        let input: ReadFileInput = serde_json::from_value(json!({
            "path": "src/main.rs"
        }))
        .unwrap();
        assert_eq!(input.path, "src/main.rs");
        assert!(input.offset.is_none());
        assert!(input.limit.is_none());

        let input_with_range: ReadFileInput = serde_json::from_value(json!({
            "path": "src/lib.rs",
            "offset": 10,
            "limit": 50
        }))
        .unwrap();
        assert_eq!(input_with_range.offset, Some(10));
        assert_eq!(input_with_range.limit, Some(50));
    }

    #[test]
    fn test_write_file_input_deserializes() {
        let input: WriteFileInput = serde_json::from_value(json!({
            "path": "output.txt",
            "content": "Hello, World!"
        }))
        .unwrap();
        assert_eq!(input.path, "output.txt");
        assert_eq!(input.content, "Hello, World!");
    }

    #[test]
    fn test_edit_file_input_deserializes() {
        let input: EditFileInput = serde_json::from_value(json!({
            "path": "file.rs",
            "old_text": "foo",
            "new_text": "bar"
        }))
        .unwrap();
        assert_eq!(input.old_text, "foo");
        assert_eq!(input.new_text, "bar");
        assert!(input.replace_all.is_none());
    }

    #[test]
    fn test_multi_edit_input_deserializes() {
        let input: MultiEditInput = serde_json::from_value(json!({
            "path": "file.rs",
            "edits": [
                { "old_text": "a", "new_text": "b" },
                { "old_text": "c", "new_text": "d" }
            ]
        }))
        .unwrap();
        assert_eq!(input.edits.len(), 2);
        assert_eq!(input.edits[0].old_text, "a");
    }

    #[test]
    fn test_glob_input_deserializes() {
        let input: GlobInput = serde_json::from_value(json!({
            "pattern": "**/*.rs"
        }))
        .unwrap();
        assert_eq!(input.pattern, "**/*.rs");
        assert!(input.path.is_none());
    }

    #[test]
    fn test_grep_input_deserializes() {
        let input: GrepInput = serde_json::from_value(json!({
            "pattern": "fn main",
            "path": "src",
            "include": "*.rs"
        }))
        .unwrap();
        assert_eq!(input.pattern, "fn main");
        assert_eq!(input.path, Some("src".to_string()));
        assert_eq!(input.include, Some("*.rs".to_string()));
    }

    #[test]
    fn test_dispatch_agent_input_deserializes() {
        // Minimal input
        let input: DispatchAgentInput = serde_json::from_value(json!({
            "task": "Find auth files"
        }))
        .unwrap();
        assert_eq!(input.task, "Find auth files");
        assert!(input.agent_type.is_none());
        assert!(input.tools.is_none());

        // Full input
        let input_full: DispatchAgentInput = serde_json::from_value(json!({
            "task": "Review PR",
            "agent_type": "pr-review",
            "tools": ["read_file", "grep"]
        }))
        .unwrap();
        assert_eq!(input_full.agent_type, Some("pr-review".to_string()));
        assert_eq!(
            input_full.tools,
            Some(vec!["read_file".to_string(), "grep".to_string()])
        );
    }

    #[test]
    fn test_web_search_input_deserializes() {
        let input: WebSearchInput = serde_json::from_value(json!({
            "query": "rust async tutorial",
            "allowed_domains": ["docs.rs", "rust-lang.org"],
            "blocked_domains": ["spam.com"]
        }))
        .unwrap();
        assert_eq!(input.query, "rust async tutorial");
        assert_eq!(input.allowed_domains.unwrap().len(), 2);
        assert_eq!(input.blocked_domains.unwrap().len(), 1);
    }

    #[test]
    fn test_todo_write_input_deserializes() {
        let input: TodoWriteInput = serde_json::from_value(json!({
            "todos": [
                {
                    "id": "1",
                    "content": "Fix bug",
                    "status": "pending",
                    "priority": "high"
                }
            ]
        }))
        .unwrap();
        assert_eq!(input.todos.len(), 1);
        assert_eq!(input.todos[0].content, "Fix bug");
    }

    #[test]
    fn test_toolname_from_str() {
        assert_eq!(ToolName::parse("bash"), Some(ToolName::Bash));
        assert_eq!(ToolName::parse("read_file"), Some(ToolName::ReadFile));
        assert_eq!(
            ToolName::parse("dispatch_agent"),
            Some(ToolName::DispatchAgent)
        );
        assert_eq!(ToolName::parse("unknown"), None);
    }

    #[test]
    fn test_toolname_serde_roundtrip() {
        let name = ToolName::ReadFile;
        let serialized = serde_json::to_string(&name).unwrap();
        assert_eq!(serialized, "\"read_file\"");

        let deserialized: ToolName = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, ToolName::ReadFile);
    }
}
