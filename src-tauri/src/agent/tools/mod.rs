mod executor;
mod local;
mod types;

pub use executor::ToolExecutor;
pub use local::LocalExecutor;
pub use local::SessionState;
pub use types::ToolName;

use crate::agent::types::ToolDefinition;
use once_cell::sync::Lazy;
use serde_json::json;

mod descriptions {
    pub const BASH: &str = include_str!("descriptions/bash.md");
    pub const READ_FILE: &str = include_str!("descriptions/read_file.md");
    pub const WRITE_FILE: &str = include_str!("descriptions/write_file.md");
    pub const EDIT_FILE: &str = include_str!("descriptions/edit_file.md");
    pub const MULTI_EDIT: &str = include_str!("descriptions/multi_edit.md");
    pub const LIST_DIRECTORY: &str = include_str!("descriptions/list_directory.md");
    pub const GLOB: &str = include_str!("descriptions/glob.md");
    pub const GREP: &str = include_str!("descriptions/grep.md");
    pub const NOTEBOOK_READ: &str = include_str!("descriptions/notebook_read.md");
    pub const NOTEBOOK_EDIT: &str = include_str!("descriptions/notebook_edit.md");
    pub const WEB_FETCH: &str = include_str!("descriptions/web_fetch.md");
    pub const WEB_SEARCH: &str = include_str!("descriptions/web_search.md");
    pub const TODO_READ: &str = include_str!("descriptions/todo_read.md");
    pub const TODO_WRITE: &str = include_str!("descriptions/todo_write.md");
    pub const AGENT: &str = include_str!("descriptions/agent.md");
    pub const EXIT_PLAN_MODE: &str = include_str!("descriptions/exit_plan_mode.md");
}

static TOOL_DEFINITIONS: Lazy<Vec<ToolDefinition>> = Lazy::new(|| {
    vec![
        // File & Shell Tools
        ToolDefinition {
            name: "bash".to_string(),
            description: descriptions::BASH.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute"
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Timeout in seconds (optional)"
                    }
                },
                "required": ["command"]
            }),
        },
        ToolDefinition {
            name: "read_file".to_string(),
            description: descriptions::READ_FILE.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Starting line number (optional)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Number of lines to read (optional)"
                    }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "write_file".to_string(),
            description: descriptions::WRITE_FILE.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write"
                    }
                },
                "required": ["path", "content"]
            }),
        },
        ToolDefinition {
            name: "edit_file".to_string(),
            description: descriptions::EDIT_FILE.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file"
                    },
                    "old_text": {
                        "type": "string",
                        "description": "Text to find"
                    },
                    "new_text": {
                        "type": "string",
                        "description": "Text to replace with"
                    },
                    "replace_all": {
                        "type": "boolean",
                        "description": "Replace all occurrences (default: false)"
                    }
                },
                "required": ["path", "old_text", "new_text"]
            }),
        },
        ToolDefinition {
            name: "multi_edit".to_string(),
            description: descriptions::MULTI_EDIT.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file"
                    },
                    "edits": {
                        "type": "array",
                        "description": "List of edits to apply",
                        "items": {
                            "type": "object",
                            "properties": {
                                "old_text": { "type": "string" },
                                "new_text": { "type": "string" }
                            },
                            "required": ["old_text", "new_text"]
                        }
                    }
                },
                "required": ["path", "edits"]
            }),
        },
        ToolDefinition {
            name: "list_directory".to_string(),
            description: descriptions::LIST_DIRECTORY.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to directory"
                    }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "glob".to_string(),
            description: descriptions::GLOB.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern (e.g., **/*.rs)"
                    },
                    "path": {
                        "type": "string",
                        "description": "Base directory (optional, defaults to project root)"
                    }
                },
                "required": ["pattern"]
            }),
        },
        ToolDefinition {
            name: "grep".to_string(),
            description: descriptions::GREP.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regex pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory to search (optional)"
                    },
                    "include": {
                        "type": "string",
                        "description": "File pattern filter (e.g., *.rs)"
                    }
                },
                "required": ["pattern"]
            }),
        },
        // Notebook Tools
        ToolDefinition {
            name: "notebook_read".to_string(),
            description: descriptions::NOTEBOOK_READ.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to .ipynb file"
                    }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "notebook_edit".to_string(),
            description: descriptions::NOTEBOOK_EDIT.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to .ipynb file"
                    },
                    "cell_number": {
                        "type": "integer",
                        "description": "Zero-indexed cell position"
                    },
                    "new_source": {
                        "type": "string",
                        "description": "New cell content"
                    },
                    "cell_type": {
                        "type": "string",
                        "enum": ["code", "markdown"],
                        "description": "Cell type (required for insert)"
                    },
                    "edit_mode": {
                        "type": "string",
                        "enum": ["replace", "insert", "delete"],
                        "description": "Edit mode (default: replace)"
                    }
                },
                "required": ["path", "cell_number", "new_source"]
            }),
        },
        // Web Tools
        ToolDefinition {
            name: "web_fetch".to_string(),
            description: descriptions::WEB_FETCH.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "URL to fetch"
                    },
                    "prompt": {
                        "type": "string",
                        "description": "What to extract from the page"
                    }
                },
                "required": ["url", "prompt"]
            }),
        },
        ToolDefinition {
            name: "web_search".to_string(),
            description: descriptions::WEB_SEARCH.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    },
                    "allowed_domains": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Only include these domains"
                    },
                    "blocked_domains": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Exclude these domains"
                    }
                },
                "required": ["query"]
            }),
        },
        // Task Management Tools
        ToolDefinition {
            name: "todo_read".to_string(),
            description: descriptions::TODO_READ.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        ToolDefinition {
            name: "todo_write".to_string(),
            description: descriptions::TODO_WRITE.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "todos": {
                        "type": "array",
                        "description": "Task list",
                        "items": {
                            "type": "object",
                            "properties": {
                                "id": { "type": "string" },
                                "content": { "type": "string" },
                                "status": {
                                    "type": "string",
                                    "enum": ["pending", "in_progress", "completed"]
                                },
                                "priority": {
                                    "type": "string",
                                    "enum": ["high", "medium", "low"]
                                }
                            },
                            "required": ["id", "content", "status", "priority"]
                        }
                    }
                },
                "required": ["todos"]
            }),
        },
        ToolDefinition {
            name: "agent".to_string(),
            description: descriptions::AGENT.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "description": {
                        "type": "string",
                        "description": "Short task summary (3-5 words)"
                    },
                    "prompt": {
                        "type": "string",
                        "description": "Detailed task instructions"
                    }
                },
                "required": ["description", "prompt"]
            }),
        },
        ToolDefinition {
            name: "exit_plan_mode".to_string(),
            description: descriptions::EXIT_PLAN_MODE.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "plan": {
                        "type": "string",
                        "description": "Markdown-formatted plan"
                    }
                },
                "required": ["plan"]
            }),
        },
    ]
});

pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    TOOL_DEFINITIONS.clone()
}
