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
    WebSearch,
    // Task Management Tools
    TodoRead,
    TodoWrite,
    Agent,
    ExitPlanMode,
}

impl ToolName {
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
            ToolName::WebSearch => "web_search",
            ToolName::TodoRead => "todo_read",
            ToolName::TodoWrite => "todo_write",
            ToolName::Agent => "agent",
            ToolName::ExitPlanMode => "exit_plan_mode",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
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
            "web_search" => Some(ToolName::WebSearch),
            "todo_read" => Some(ToolName::TodoRead),
            "todo_write" => Some(ToolName::TodoWrite),
            "agent" => Some(ToolName::Agent),
            "exit_plan_mode" => Some(ToolName::ExitPlanMode),
            _ => None,
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
