use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolName {
    Bash,
    ReadFile,
    WriteFile,
    EditFile,
    ListDirectory,
}

impl ToolName {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            ToolName::Bash => "bash",
            ToolName::ReadFile => "read_file",
            ToolName::WriteFile => "write_file",
            ToolName::EditFile => "edit_file",
            ToolName::ListDirectory => "list_directory",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "bash" => Some(ToolName::Bash),
            "read_file" => Some(ToolName::ReadFile),
            "write_file" => Some(ToolName::WriteFile),
            "edit_file" => Some(ToolName::EditFile),
            "list_directory" => Some(ToolName::ListDirectory),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BashInput {
    pub command: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReadFileInput {
    pub path: String,
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
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListDirectoryInput {
    pub path: String,
}
