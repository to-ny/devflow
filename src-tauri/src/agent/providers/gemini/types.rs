use serde::{Deserialize, Serialize};

use crate::agent::types::{ChatMessage, MessageRole, ToolDefinition};

// === Request Types ===

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiRequest {
    pub contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<GeminiSystemInstruction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<GeminiTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Serialize)]
pub struct GeminiSystemInstruction {
    pub parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Clone)]
pub struct GeminiContent {
    pub role: String,
    pub parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(untagged)]
pub enum GeminiPart {
    Text {
        text: String,
    },
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: FunctionCall,
    },
    FunctionResponse {
        #[serde(rename = "functionResponse")]
        function_response: FunctionResponse,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCall {
    pub name: String,
    #[serde(default)]
    pub args: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionResponse {
    pub name: String,
    pub response: FunctionResponseContent,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionResponseContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiTool {
    pub function_declarations: Vec<FunctionDeclaration>,
}

#[derive(Debug, Serialize)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
}

impl From<&ToolDefinition> for FunctionDeclaration {
    fn from(tool: &ToolDefinition) -> Self {
        Self {
            name: tool.name.clone(),
            description: tool.description.clone(),
            parameters: tool.input_schema.clone(),
        }
    }
}

impl From<&ChatMessage> for GeminiContent {
    fn from(msg: &ChatMessage) -> Self {
        Self {
            role: match msg.role {
                MessageRole::User => "user".to_string(),
                MessageRole::Assistant => "model".to_string(),
            },
            parts: vec![GeminiPart::Text {
                text: msg.content.clone(),
            }],
        }
    }
}

// === Response Types ===

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiResponse {
    pub candidates: Option<Vec<Candidate>>,
    #[serde(default)]
    pub error: Option<GeminiError>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub content: Option<CandidateContent>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct CandidateContent {
    pub role: Option<String>,
    pub parts: Option<Vec<ResponsePart>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum ResponsePart {
    Text {
        text: String,
    },
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: FunctionCall,
    },
    #[serde(rename_all = "camelCase")]
    Unknown(serde_json::Value),
}

// Handle Gemini's actual response format
impl<'de> Deserialize<'de> for GeminiPart {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct PartHelper {
            text: Option<String>,
            function_call: Option<FunctionCall>,
            function_response: Option<FunctionResponse>,
        }

        let helper = PartHelper::deserialize(deserializer)?;

        if let Some(text) = helper.text {
            Ok(GeminiPart::Text { text })
        } else if let Some(function_call) = helper.function_call {
            Ok(GeminiPart::FunctionCall { function_call })
        } else if let Some(function_response) = helper.function_response {
            Ok(GeminiPart::FunctionResponse { function_response })
        } else {
            Err(serde::de::Error::custom("Unknown part type"))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GeminiError {
    pub message: String,
    #[allow(dead_code)]
    pub code: Option<i32>,
}

// === Streaming State ===

#[derive(Debug, Default)]
pub struct StreamedResponse {
    pub text_content: String,
    pub function_calls: Vec<FunctionCall>,
    pub finish_reason: Option<String>,
}

impl StreamedResponse {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append_text(&mut self, text: &str) {
        self.text_content.push_str(text);
    }

    pub fn add_function_call(&mut self, call: FunctionCall) {
        self.function_calls.push(call);
    }

    pub fn set_finish_reason(&mut self, reason: String) {
        self.finish_reason = Some(reason);
    }

    pub fn has_function_calls(&self) -> bool {
        !self.function_calls.is_empty()
    }
}
