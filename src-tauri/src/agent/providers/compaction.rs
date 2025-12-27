//! Context compaction for long conversations.

use std::future::Future;

use serde::Deserialize;
use tauri::{AppHandle, Emitter};

use crate::agent::error::AgentError;
use crate::agent::tools::{CompactedContext, SessionState};
use crate::agent::types::{
    AgentCompactionPayload, AgentCompactionWarningPayload, AgentStatus, ChatContentBlock,
    ChatMessage, CompactedFact, FactCategory,
};

use super::{emit_status, DEFAULT_EXTRACTION_PROMPT};

const DEFAULT_CONTEXT_LIMIT: u32 = 200_000;
const COMPACTION_THRESHOLD: f64 = 0.8;
const PRESERVED_EXCHANGES: usize = 6;
const AGGRESSIVE_PRESERVED_EXCHANGES: usize = 4;

pub fn get_context_limit(config_limit: Option<u32>) -> u32 {
    config_limit.unwrap_or(DEFAULT_CONTEXT_LIMIT)
}

/// Estimate tokens using chars/4 heuristic.
pub fn estimate_tokens(text: &str) -> u32 {
    (text.chars().count() / 4) as u32
}

fn estimate_block_tokens(block: &ChatContentBlock) -> u32 {
    match block {
        ChatContentBlock::Text { text } => estimate_tokens(text),
        ChatContentBlock::ToolUse {
            tool_name,
            tool_input,
            output,
            ..
        } => {
            let mut tokens = estimate_tokens(tool_name);
            tokens += estimate_tokens(&tool_input.to_string());
            if let Some(out) = output {
                tokens += estimate_tokens(out);
            }
            tokens
        }
    }
}

pub fn estimate_message_tokens(message: &ChatMessage) -> u32 {
    message
        .content_blocks
        .iter()
        .map(estimate_block_tokens)
        .sum()
}

pub fn estimate_context_size(
    system_prompt: Option<&str>,
    messages: &[ChatMessage],
    compacted: Option<&CompactedContext>,
) -> u32 {
    let mut total = 0u32;

    if let Some(system) = system_prompt {
        total += estimate_tokens(system);
    }

    if let Some(ctx) = compacted {
        total += estimate_tokens(&format_compacted_context(ctx));
    }

    for msg in messages {
        total += estimate_message_tokens(msg);
    }

    total
}

pub fn should_compact(estimated_tokens: u32, context_limit: u32) -> bool {
    estimated_tokens > (context_limit as f64 * COMPACTION_THRESHOLD) as u32
}

/// Returns (to_compact, to_preserve).
pub fn split_messages_for_compaction(
    messages: &[ChatMessage],
    aggressive: bool,
) -> (Vec<&ChatMessage>, Vec<&ChatMessage>) {
    let preserve_count = if aggressive {
        AGGRESSIVE_PRESERVED_EXCHANGES * 2
    } else {
        PRESERVED_EXCHANGES * 2
    };

    if messages.len() <= preserve_count {
        return (vec![], messages.iter().collect());
    }

    let split_point = messages.len() - preserve_count;
    let to_compact: Vec<&ChatMessage> = messages[..split_point].iter().collect();
    let to_preserve: Vec<&ChatMessage> = messages[split_point..].iter().collect();

    (to_compact, to_preserve)
}

pub fn format_messages_for_extraction(messages: &[&ChatMessage]) -> String {
    let mut output = String::new();

    for msg in messages {
        let role = match msg.role {
            crate::agent::types::MessageRole::User => "User",
            crate::agent::types::MessageRole::Assistant => "Assistant",
        };

        output.push_str(&format!("## {}\n", role));

        for block in &msg.content_blocks {
            match block {
                ChatContentBlock::Text { text } => {
                    output.push_str(text);
                    output.push('\n');
                }
                ChatContentBlock::ToolUse {
                    tool_name,
                    tool_input,
                    output: tool_output,
                    is_error,
                    ..
                } => {
                    output.push_str(&format!("[Tool: {}]\n", tool_name));
                    output.push_str(&format!("Input: {}\n", tool_input));
                    if let Some(out) = tool_output {
                        let truncated = if out.len() > 500 {
                            format!("{}... (truncated)", &out[..500])
                        } else {
                            out.clone()
                        };
                        let error_marker = if is_error.unwrap_or(false) {
                            " [ERROR]"
                        } else {
                            ""
                        };
                        output.push_str(&format!("Output{}: {}\n", error_marker, truncated));
                    }
                }
            }
        }
        output.push('\n');
    }

    output
}

pub fn build_extraction_prompt(formatted_messages: &str, custom_prompt: Option<&str>) -> String {
    let template = custom_prompt.unwrap_or(DEFAULT_EXTRACTION_PROMPT);
    template.replace("{conversation}", formatted_messages)
}

#[derive(Debug, Deserialize)]
pub struct ExtractionResponse {
    pub summary: String,
    pub facts: Vec<ExtractionFact>,
}

#[derive(Debug, Deserialize)]
pub struct ExtractionFact {
    pub category: String,
    pub content: String,
}

pub fn parse_extraction_response(response: &str) -> Result<ExtractionResponse, String> {
    let json_str = extract_json_from_response(response);
    serde_json::from_str(&json_str).map_err(|e| format!("Failed to parse extraction JSON: {}", e))
}

fn extract_json_from_response(response: &str) -> String {
    if let Some(start) = response.find("```json") {
        let json_start = start + 7;
        if let Some(end) = response[json_start..].find("```") {
            return response[json_start..json_start + end].trim().to_string();
        }
    }

    if let Some(start) = response.find("```") {
        let json_start = start + 3;
        if let Some(end) = response[json_start..].find("```") {
            let content = response[json_start..json_start + end].trim();
            if let Some(newline) = content.find('\n') {
                if !content[..newline].contains('{') {
                    return content[newline..].trim().to_string();
                }
            }
            return content.to_string();
        }
    }

    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            return response[start..=end].to_string();
        }
    }

    response.to_string()
}

pub fn extraction_to_compacted(response: ExtractionResponse) -> CompactedContext {
    let facts: Vec<CompactedFact> = response
        .facts
        .into_iter()
        .filter_map(|f| {
            let category = match f.category.to_lowercase().as_str() {
                "decision" => Some(FactCategory::Decision),
                "preference" => Some(FactCategory::Preference),
                "context" => Some(FactCategory::Context),
                "blocker" => Some(FactCategory::Blocker),
                _ => None,
            };
            category.map(|cat| CompactedFact {
                category: cat,
                content: f.content,
            })
        })
        .collect();

    CompactedContext {
        summary: Some(response.summary),
        facts,
    }
}

pub fn merge_compacted_contexts(
    existing: &CompactedContext,
    new: CompactedContext,
) -> CompactedContext {
    if existing.summary.is_none() && existing.facts.is_empty() {
        return new;
    }

    let summary = match (&existing.summary, &new.summary) {
        (Some(old), Some(new_sum)) => Some(format!("{} {}", old, new_sum)),
        (Some(old), None) => Some(old.clone()),
        (None, Some(new_sum)) => Some(new_sum.clone()),
        (None, None) => None,
    };

    let mut facts = existing.facts.clone();
    for fact in new.facts {
        if !facts.iter().any(|f| f.content == fact.content) {
            facts.push(fact);
        }
    }

    const MAX_FACTS: usize = 20;
    if facts.len() > MAX_FACTS {
        facts.truncate(MAX_FACTS);
    }

    CompactedContext { summary, facts }
}

pub fn format_compacted_context(context: &CompactedContext) -> String {
    let mut output = String::from("[Session Context]\n");

    if let Some(summary) = &context.summary {
        output.push_str(&format!("Summary: {}\n\n", summary));
    }

    if !context.facts.is_empty() {
        output.push_str("Key Facts:\n");
        for fact in &context.facts {
            let prefix = match fact.category {
                FactCategory::Decision => "[DECISION]",
                FactCategory::Preference => "[PREFERENCE]",
                FactCategory::Context => "[CONTEXT]",
                FactCategory::Blocker => "[BLOCKER]",
            };
            output.push_str(&format!("- {} {}\n", prefix, fact.content));
        }
        output.push('\n');
    }

    output.push_str("[Recent Conversation Follows]");

    output
}

pub struct CompactionResult {
    pub preserved_messages: Vec<ChatMessage>,
    pub compacted_text: String,
}

/// Context for compaction operations, reducing function argument count.
pub struct CompactionContext<'a> {
    pub context_limit: u32,
    pub extraction_prompt: Option<&'a str>,
    pub session: &'a SessionState,
    pub app_handle: &'a AppHandle,
}

/// Shared compaction logic for all providers.
/// The `call_extraction` callback is provider-specific.
pub async fn maybe_compact<F, Fut>(
    messages: &[ChatMessage],
    system_prompt: Option<&str>,
    ctx: &CompactionContext<'_>,
    call_extraction: F,
) -> Result<Option<CompactionResult>, AgentError>
where
    F: FnOnce(String) -> Fut,
    Fut: Future<Output = Result<String, AgentError>>,
{
    let existing_compacted = ctx.session.get_compacted().await;

    let compacted_ref =
        if existing_compacted.summary.is_some() || !existing_compacted.facts.is_empty() {
            Some(&existing_compacted)
        } else {
            None
        };

    let estimated_tokens = estimate_context_size(system_prompt, messages, compacted_ref);

    if !should_compact(estimated_tokens, ctx.context_limit) {
        return Ok(None);
    }

    emit_status(ctx.app_handle, AgentStatus::Compacting, None);

    let (to_compact, to_preserve) = split_messages_for_compaction(messages, false);

    if to_compact.is_empty() {
        let (to_compact_aggressive, to_preserve_aggressive) =
            split_messages_for_compaction(messages, true);

        if to_compact_aggressive.is_empty() {
            let _ = ctx.app_handle.emit(
                "agent-compaction-warning",
                AgentCompactionWarningPayload {
                    message: "Unable to compact: not enough messages".to_string(),
                },
            );
            return Ok(None);
        }

        return perform_compaction(
            &to_compact_aggressive,
            &to_preserve_aggressive,
            &existing_compacted,
            ctx,
            call_extraction,
            estimated_tokens,
        )
        .await;
    }

    perform_compaction(
        &to_compact,
        &to_preserve,
        &existing_compacted,
        ctx,
        call_extraction,
        estimated_tokens,
    )
    .await
}

async fn perform_compaction<F, Fut>(
    to_compact: &[&ChatMessage],
    to_preserve: &[&ChatMessage],
    existing_compacted: &CompactedContext,
    ctx: &CompactionContext<'_>,
    call_extraction: F,
    original_tokens: u32,
) -> Result<Option<CompactionResult>, AgentError>
where
    F: FnOnce(String) -> Fut,
    Fut: Future<Output = Result<String, AgentError>>,
{
    let formatted = format_messages_for_extraction(to_compact);

    let extraction_input = if existing_compacted.summary.is_some() {
        format!(
            "Previous session context:\n{}\n\nNew conversation to analyze:\n{}",
            format_compacted_context(existing_compacted),
            formatted
        )
    } else {
        formatted
    };

    let extraction_prompt = build_extraction_prompt(&extraction_input, ctx.extraction_prompt);

    let extraction_result = call_extraction(extraction_prompt).await;

    match extraction_result {
        Ok(response_text) => match parse_extraction_response(&response_text) {
            Ok(extraction) => {
                let new_compacted = extraction_to_compacted(extraction);
                let merged = merge_compacted_contexts(existing_compacted, new_compacted);

                ctx.session.set_compacted(merged.clone()).await;

                let preserved: Vec<ChatMessage> =
                    to_preserve.iter().map(|m| (*m).clone()).collect();

                let compacted_text = format_compacted_context(&merged);
                let compacted_tokens = estimate_context_size(None, &preserved, Some(&merged));
                let facts_count = merged.facts.len() as u32;

                let _ = ctx.app_handle.emit(
                    "agent-compaction",
                    AgentCompactionPayload {
                        original_tokens,
                        compacted_tokens,
                        facts_count,
                    },
                );

                Ok(Some(CompactionResult {
                    preserved_messages: preserved,
                    compacted_text,
                }))
            }
            Err(e) => {
                let _ = ctx.app_handle.emit(
                    "agent-compaction-warning",
                    AgentCompactionWarningPayload {
                        message: format!("Failed to parse extraction: {}", e),
                    },
                );
                Ok(None)
            }
        },
        Err(e) => {
            let _ = ctx.app_handle.emit(
                "agent-compaction-warning",
                AgentCompactionWarningPayload {
                    message: format!("Extraction API failed: {}", e),
                },
            );
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::types::MessageRole;

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("hello"), 1);
        assert_eq!(estimate_tokens("hello world"), 2);
        // 100 chars should be ~25 tokens
        let long_text = "a".repeat(100);
        assert_eq!(estimate_tokens(&long_text), 25);
    }

    #[test]
    fn test_get_context_limit() {
        // With config override
        assert_eq!(get_context_limit(Some(100_000)), 100_000);

        // Default when not configured
        assert_eq!(get_context_limit(None), 200_000);
    }

    #[test]
    fn test_should_compact() {
        // 80% of 200k = 160k
        assert!(!should_compact(100_000, 200_000));
        assert!(!should_compact(160_000, 200_000));
        assert!(should_compact(160_001, 200_000));
        assert!(should_compact(200_000, 200_000));
    }

    #[test]
    fn test_split_messages_for_compaction() {
        let messages: Vec<ChatMessage> = (0..20)
            .map(|i| ChatMessage {
                id: format!("msg-{}", i),
                role: if i % 2 == 0 {
                    MessageRole::User
                } else {
                    MessageRole::Assistant
                },
                content_blocks: vec![ChatContentBlock::Text {
                    text: format!("Message {}", i),
                }],
            })
            .collect();

        // Normal mode: preserve 6 exchanges = 12 messages
        let (compact, preserve) = split_messages_for_compaction(&messages, false);
        assert_eq!(compact.len(), 8);
        assert_eq!(preserve.len(), 12);

        // Aggressive mode: preserve 4 exchanges = 8 messages
        let (compact, preserve) = split_messages_for_compaction(&messages, true);
        assert_eq!(compact.len(), 12);
        assert_eq!(preserve.len(), 8);

        // Not enough messages to compact
        let small_messages: Vec<ChatMessage> = (0..10)
            .map(|i| ChatMessage {
                id: format!("msg-{}", i),
                role: if i % 2 == 0 {
                    MessageRole::User
                } else {
                    MessageRole::Assistant
                },
                content_blocks: vec![ChatContentBlock::Text {
                    text: format!("Message {}", i),
                }],
            })
            .collect();

        let (compact, preserve) = split_messages_for_compaction(&small_messages, false);
        assert_eq!(compact.len(), 0);
        assert_eq!(preserve.len(), 10);
    }

    #[test]
    fn test_extract_json_from_response() {
        // Raw JSON
        let raw = r#"{"summary": "test", "facts": []}"#;
        assert!(extract_json_from_response(raw).contains("summary"));

        // JSON in code block
        let with_block = r#"Here's the analysis:
```json
{"summary": "test", "facts": []}
```"#;
        assert!(extract_json_from_response(with_block).contains("summary"));

        // JSON in plain code block
        let plain_block = r#"```
{"summary": "test", "facts": []}
```"#;
        assert!(extract_json_from_response(plain_block).contains("summary"));
    }

    #[test]
    fn test_parse_extraction_response() {
        let json = r#"{"summary": "User implemented auth", "facts": [{"category": "decision", "content": "Using JWT"}]}"#;
        let result = parse_extraction_response(json).unwrap();
        assert_eq!(result.summary, "User implemented auth");
        assert_eq!(result.facts.len(), 1);
        assert_eq!(result.facts[0].category, "decision");
    }

    #[test]
    fn test_extraction_to_compacted() {
        let response = ExtractionResponse {
            summary: "Test summary".to_string(),
            facts: vec![
                ExtractionFact {
                    category: "decision".to_string(),
                    content: "Use Rust".to_string(),
                },
                ExtractionFact {
                    category: "invalid".to_string(),
                    content: "Should be filtered".to_string(),
                },
            ],
        };

        let compacted = extraction_to_compacted(response);
        assert_eq!(compacted.summary, Some("Test summary".to_string()));
        assert_eq!(compacted.facts.len(), 1);
        assert_eq!(compacted.facts[0].category, FactCategory::Decision);
    }

    #[test]
    fn test_format_compacted_context() {
        let context = CompactedContext {
            summary: Some("User is building a CLI tool".to_string()),
            facts: vec![
                CompactedFact {
                    category: FactCategory::Decision,
                    content: "Using Rust for the backend".to_string(),
                },
                CompactedFact {
                    category: FactCategory::Preference,
                    content: "Prefers minimal dependencies".to_string(),
                },
            ],
        };

        let formatted = format_compacted_context(&context);
        assert!(formatted.contains("[Session Context]"));
        assert!(formatted.contains("User is building a CLI tool"));
        assert!(formatted.contains("[DECISION]"));
        assert!(formatted.contains("[PREFERENCE]"));
        assert!(formatted.contains("[Recent Conversation Follows]"));
    }

    #[test]
    fn test_merge_compacted_contexts() {
        let existing = CompactedContext {
            summary: Some("First part".to_string()),
            facts: vec![CompactedFact {
                category: FactCategory::Decision,
                content: "Fact 1".to_string(),
            }],
        };

        let new = CompactedContext {
            summary: Some("Second part".to_string()),
            facts: vec![
                CompactedFact {
                    category: FactCategory::Decision,
                    content: "Fact 1".to_string(), // Duplicate
                },
                CompactedFact {
                    category: FactCategory::Context,
                    content: "Fact 2".to_string(),
                },
            ],
        };

        let merged = merge_compacted_contexts(&existing, new);
        assert!(merged.summary.unwrap().contains("First part"));
        assert_eq!(merged.facts.len(), 2); // Duplicate removed
    }
}
