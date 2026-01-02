//! Integration between the eval framework and the actual agent.
//!
//! This module provides the machinery to run evaluation cases against
//! the real DevFlow agent and collect behavioral data for analysis.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::agent::error::AgentError;
use crate::agent::providers::{run_headless_loop, HeadlessContext, HeadlessStreamer};
use crate::agent::tools::{get_tool_definitions, ToolExecutor, ToolName};
use crate::agent::types::{ChatMessage, MessageRole, ToolDefinition};
use crate::agent::usage::SessionUsageTracker;

use super::runner::AgentRunData;
use super::types::ToolUsage;

/// A wrapper executor that collects all tool calls for eval analysis.
pub struct CollectingExecutor<E: ToolExecutor> {
    inner: E,
    collected: Arc<Mutex<Vec<ToolUsage>>>,
}

impl<E: ToolExecutor> CollectingExecutor<E> {
    pub fn new(inner: E) -> Self {
        Self {
            inner,
            collected: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn get_collected(&self) -> Vec<ToolUsage> {
        self.collected.lock().unwrap().clone()
    }
}

#[async_trait]
impl<E: ToolExecutor + Send + Sync> ToolExecutor for CollectingExecutor<E> {
    async fn execute(
        &self,
        tool: ToolName,
        input: serde_json::Value,
    ) -> Result<String, AgentError> {
        let result = self.inner.execute(tool, input.clone()).await;

        let (output, is_error) = match &result {
            Ok(output) => (output.clone(), false),
            Err(e) => (e.to_string(), true),
        };

        self.collected.lock().unwrap().push(ToolUsage {
            name: tool.as_str().to_string(),
            input,
            output,
            is_error,
        });

        result
    }
}

/// Configuration for running an eval case
pub struct EvalRunConfig {
    pub system_prompt: Option<String>,
    pub tools: Vec<ToolDefinition>,
    pub max_iterations: u32,
}

impl Default for EvalRunConfig {
    fn default() -> Self {
        Self {
            system_prompt: None,
            tools: get_tool_definitions(),
            max_iterations: 20,
        }
    }
}

/// Run an eval case and collect behavioral data.
///
/// This is the main integration point - it runs the agent with a prompt
/// and returns structured data that can be evaluated against expectations.
pub async fn run_eval_case<S, E>(
    streamer: &S,
    executor: E,
    prompt: &str,
    config: EvalRunConfig,
) -> Result<AgentRunData, AgentError>
where
    S: HeadlessStreamer,
    E: ToolExecutor + Send + Sync,
{
    let start = Instant::now();
    let cancel_token = CancellationToken::new();
    let usage_tracker = Arc::new(SessionUsageTracker::new());

    // Wrap executor to collect tool usage
    let collecting_executor = CollectingExecutor::new(executor);

    // Build initial messages
    let messages = vec![ChatMessage::new(MessageRole::User, prompt.to_string())];

    let ctx = HeadlessContext {
        system_prompt: config.system_prompt,
        tools: config.tools,
        executor: &collecting_executor,
        max_iterations: config.max_iterations,
        cancel_token: &cancel_token,
        usage_tracker: usage_tracker.clone(),
    };

    let result = run_headless_loop(streamer, messages, ctx).await?;

    let duration_ms = start.elapsed().as_millis() as u64;
    let totals = usage_tracker.get_totals();

    Ok(AgentRunData {
        tools_used: collecting_executor.get_collected(),
        output: result.text,
        iterations: result.tool_calls_made,
        tokens_used: totals.input_tokens + totals.output_tokens,
        duration_ms,
    })
}

/// Run eval with timeout protection
pub async fn run_eval_case_with_timeout<S, E>(
    streamer: &S,
    executor: E,
    prompt: &str,
    config: EvalRunConfig,
    timeout_ms: u64,
) -> Result<AgentRunData, AgentError>
where
    S: HeadlessStreamer,
    E: ToolExecutor + Send + Sync,
{
    let timeout = std::time::Duration::from_millis(timeout_ms);

    match tokio::time::timeout(timeout, run_eval_case(streamer, executor, prompt, config)).await {
        Ok(result) => result,
        Err(_) => Err(AgentError::ToolExecutionError(format!(
            "Eval timed out after {}ms",
            timeout_ms
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::providers::mock::{MockExecutor, MockStreamer, ScriptedResponse};

    #[tokio::test]
    async fn test_collecting_executor_records_tools() {
        let mock = MockExecutor::default();
        let collector = CollectingExecutor::new(mock);

        // Execute a few tools
        let _ = collector
            .execute(ToolName::Glob, serde_json::json!({"pattern": "*.rs"}))
            .await;

        let _ = collector
            .execute(
                ToolName::ReadFile,
                serde_json::json!({"path": "src/main.rs"}),
            )
            .await;

        let collected = collector.get_collected();
        assert_eq!(collected.len(), 2);
        assert_eq!(collected[0].name, "glob");
        assert_eq!(collected[1].name, "read_file");
    }

    #[tokio::test]
    async fn test_run_eval_case_collects_data() {
        let streamer = MockStreamer::new(vec![
            // First response: call glob
            ScriptedResponse::text_with_tool(
                "Let me search for files.",
                "glob",
                "glob-1",
                serde_json::json!({"pattern": "**/*.rs"}),
            ),
            // Second response: read a file
            ScriptedResponse::text_with_tool(
                "Found files, reading main.",
                "read_file",
                "read-1",
                serde_json::json!({"path": "src/main.rs"}),
            ),
            // Final response: provide analysis
            ScriptedResponse::text("## Analysis\n\nThe codebase has good structure."),
        ]);

        let executor = MockExecutor::default();
        let config = EvalRunConfig::default();

        let result = run_eval_case(&streamer, executor, "Analyze the codebase", config)
            .await
            .expect("Should complete successfully");

        // Verify collected data
        assert_eq!(result.tools_used.len(), 2);
        assert_eq!(result.tools_used[0].name, "glob");
        assert_eq!(result.tools_used[1].name, "read_file");
        assert!(result.output.contains("Analysis"));
        assert_eq!(result.iterations, 2); // Two tool iterations
    }

    #[tokio::test]
    async fn test_eval_case_with_read_only_behavior() {
        // Simulates good behavior: only reads, doesn't write
        let streamer = MockStreamer::new(vec![
            ScriptedResponse::text_with_tool(
                "I'll review the changes.",
                "bash",
                "bash-1",
                serde_json::json!({"command": "git diff"}),
            ),
            ScriptedResponse::with_tool_call(
                "read_file",
                "read-1",
                serde_json::json!({"path": "SPEC.md"}),
            ),
            ScriptedResponse::text(
                "## Review Summary\n\n### Critical Issues\nNone.\n\n### Recommendation\nReady to commit.",
            ),
        ]);

        let executor = MockExecutor::default();
        let config = EvalRunConfig::default();

        let result = run_eval_case(
            &streamer,
            executor,
            "Review the changes as a senior architect",
            config,
        )
        .await
        .expect("Should complete");

        // Check that no write tools were used
        let write_tools = ["write_file", "edit_file", "multi_edit"];
        let used_write = result
            .tools_used
            .iter()
            .any(|t| write_tools.contains(&t.name.as_str()));

        assert!(!used_write, "Review should not use write tools");
        assert!(result.output.contains("Review") || result.output.contains("Summary"));
    }

    #[tokio::test]
    async fn test_eval_case_detects_bad_behavior() {
        // Simulates bad behavior: edits files during review
        let streamer = MockStreamer::new(vec![
            ScriptedResponse::text_with_tool(
                "I'll fix the issues.",
                "edit_file",
                "edit-1",
                serde_json::json!({
                    "path": "src/main.rs",
                    "old_text": "foo",
                    "new_text": "bar"
                }),
            ),
            ScriptedResponse::text("Fixed the code."),
        ]);

        let executor = MockExecutor::default();
        let config = EvalRunConfig::default();

        let result = run_eval_case(&streamer, executor, "Review the changes", config)
            .await
            .expect("Should complete");

        // The eval framework should detect this used edit_file
        let used_edit = result.tools_used.iter().any(|t| t.name == "edit_file");
        assert!(used_edit, "Should detect that edit_file was used");
    }
}
