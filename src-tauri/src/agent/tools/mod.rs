mod executor;
mod local;
mod types;

pub use executor::ToolExecutor;
pub use local::LocalExecutor;
pub use types::ToolName;

use crate::agent::types::ToolDefinition;
use once_cell::sync::Lazy;
use serde_json::json;

static TOOL_DEFINITIONS: Lazy<Vec<ToolDefinition>> = Lazy::new(|| {
    vec![
        ToolDefinition {
            name: "bash".to_string(),
            description: "Execute a shell command in the project directory. Use this to run build commands, tests, git operations, or any other shell commands.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute"
                    }
                },
                "required": ["command"]
            }),
        },
        ToolDefinition {
            name: "read_file".to_string(),
            description: "Read the contents of a file. The path must be relative to the project directory.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file to read"
                    }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "write_file".to_string(),
            description: "Create or overwrite a file with the given content. The path must be relative to the project directory. Parent directories will be created if needed.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file"
                    }
                },
                "required": ["path", "content"]
            }),
        },
        ToolDefinition {
            name: "edit_file".to_string(),
            description: "Replace the first occurrence of old_text with new_text in a file. Use this for targeted edits. The old_text must exist exactly in the file.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file to edit"
                    },
                    "old_text": {
                        "type": "string",
                        "description": "The exact text to find and replace"
                    },
                    "new_text": {
                        "type": "string",
                        "description": "The text to replace old_text with"
                    }
                },
                "required": ["path", "old_text", "new_text"]
            }),
        },
        ToolDefinition {
            name: "list_directory".to_string(),
            description: "List the contents of a directory. Returns files and subdirectories with their types.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the directory to list (use '.' for project root)"
                    }
                },
                "required": ["path"]
            }),
        },
    ]
});

pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    TOOL_DEFINITIONS.clone()
}
