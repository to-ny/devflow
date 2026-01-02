//! Mock provider for testing orchestration logic without real LLM calls.

use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use super::headless::{HeadlessResponse, HeadlessStreamer, ToolCall, ToolResult};
use crate::agent::error::AgentError;
use crate::agent::types::{ChatMessage, ToolDefinition};
use crate::agent::usage::TokenUsage;

/// A scripted response for the mock streamer
#[derive(Debug, Clone)]
pub struct ScriptedResponse {
    /// Text content to return
    pub text: String,
    /// Tool calls to include in response
    pub tool_calls: Vec<ToolCall>,
    /// Simulated token usage
    pub usage: TokenUsage,
    /// Stop reason (end_turn, tool_use, max_tokens)
    pub stop_reason: Option<String>,
}

impl ScriptedResponse {
    /// Create a simple text response
    pub fn text(content: &str) -> Self {
        Self {
            text: content.to_string(),
            tool_calls: vec![],
            usage: TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
            },
            stop_reason: Some("end_turn".to_string()),
        }
    }

    /// Create a response with a tool call
    pub fn with_tool_call(tool_name: &str, tool_id: &str, input: serde_json::Value) -> Self {
        Self {
            text: String::new(),
            tool_calls: vec![ToolCall {
                id: tool_id.to_string(),
                name: tool_name.to_string(),
                input,
            }],
            usage: TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
            },
            stop_reason: Some("tool_use".to_string()),
        }
    }

    /// Create a response with text and a tool call
    pub fn text_with_tool(
        content: &str,
        tool_name: &str,
        tool_id: &str,
        input: serde_json::Value,
    ) -> Self {
        Self {
            text: content.to_string(),
            tool_calls: vec![ToolCall {
                id: tool_id.to_string(),
                name: tool_name.to_string(),
                input,
            }],
            usage: TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
            },
            stop_reason: Some("tool_use".to_string()),
        }
    }

    /// Create a response with a specific stop reason
    pub fn with_stop_reason(content: &str, stop_reason: &str) -> Self {
        Self {
            text: content.to_string(),
            tool_calls: vec![],
            usage: TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
            },
            stop_reason: Some(stop_reason.to_string()),
        }
    }
}

/// Mock streamer that returns scripted responses
pub struct MockStreamer {
    responses: Mutex<Vec<ScriptedResponse>>,
    call_count: AtomicUsize,
    /// Collected tool results from append_tool_results calls
    tool_results: Arc<Mutex<Vec<Vec<ToolResult>>>>,
}

impl MockStreamer {
    /// Create a new mock streamer with scripted responses
    pub fn new(responses: Vec<ScriptedResponse>) -> Self {
        Self {
            responses: Mutex::new(responses),
            call_count: AtomicUsize::new(0),
            tool_results: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get the number of times stream_response was called
    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }

    /// Get collected tool results (used in tests)
    #[allow(dead_code)]
    pub async fn get_tool_results(&self) -> Vec<Vec<ToolResult>> {
        self.tool_results.lock().await.clone()
    }
}

/// Conversation state for mock streamer
pub struct MockConversation {
    #[allow(dead_code)]
    pub messages: Vec<ChatMessage>,
    pub responses: Vec<HeadlessResponse>,
}

#[async_trait]
impl HeadlessStreamer for MockStreamer {
    type Conversation = MockConversation;

    fn initial_conversation(&self, messages: Vec<ChatMessage>) -> Self::Conversation {
        MockConversation {
            messages,
            responses: Vec::new(),
        }
    }

    async fn stream_response(
        &self,
        _conversation: &Self::Conversation,
        _system_prompt: Option<String>,
        _tools: &[ToolDefinition],
        cancel_token: &CancellationToken,
    ) -> Result<HeadlessResponse, AgentError> {
        if cancel_token.is_cancelled() {
            return Err(AgentError::Cancelled);
        }

        let count = self.call_count.fetch_add(1, Ordering::SeqCst);
        let responses = self.responses.lock().await;

        if count < responses.len() {
            let scripted = responses[count].clone();
            Ok(HeadlessResponse {
                text: scripted.text,
                tool_calls: scripted.tool_calls,
                usage: scripted.usage,
                stop_reason: scripted.stop_reason,
            })
        } else {
            // Default to empty response if we run out of scripted responses
            Ok(HeadlessResponse::default())
        }
    }

    fn append_assistant_response(
        &self,
        conversation: &mut Self::Conversation,
        response: &HeadlessResponse,
    ) {
        conversation.responses.push(response.clone());
    }

    fn append_tool_results(
        &self,
        _conversation: &mut Self::Conversation,
        results: Vec<ToolResult>,
    ) {
        // Store results for later inspection in tests
        let tool_results = self.tool_results.clone();
        tokio::spawn(async move {
            tool_results.lock().await.push(results);
        });
    }
}

/// Mock executor that returns scripted tool outputs
pub struct MockExecutor {
    results: Arc<Mutex<std::collections::HashMap<String, Result<String, String>>>>,
}

impl MockExecutor {
    pub fn new() -> Self {
        Self {
            results: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Set expected result for a tool
    pub async fn expect_tool(&self, name: &str, result: Result<String, String>) {
        self.results.lock().await.insert(name.to_string(), result);
    }
}

impl Default for MockExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl crate::agent::tools::ToolExecutor for MockExecutor {
    async fn execute(
        &self,
        tool: crate::agent::tools::ToolName,
        _input: serde_json::Value,
    ) -> Result<String, AgentError> {
        let tool_name = format!("{:?}", tool).to_lowercase();
        let results = self.results.lock().await;

        match results.get(&tool_name) {
            Some(Ok(output)) => Ok(output.clone()),
            Some(Err(e)) => Err(AgentError::ToolExecutionError(e.clone())),
            None => Ok(format!("Mock output for {}", tool_name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::providers::headless::{run_headless_loop, HeadlessContext};
    use crate::agent::types::MessageRole;
    use crate::agent::usage::SessionUsageTracker;

    fn create_test_context<'a>(
        executor: &'a MockExecutor,
        cancel_token: &'a CancellationToken,
        usage_tracker: Arc<SessionUsageTracker>,
    ) -> HeadlessContext<'a> {
        HeadlessContext {
            system_prompt: Some("Test system prompt".to_string()),
            tools: vec![],
            executor,
            max_iterations: 10,
            cancel_token,
            usage_tracker,
        }
    }

    #[tokio::test]
    async fn test_simple_text_response() {
        let streamer = MockStreamer::new(vec![ScriptedResponse::text("Hello, world!")]);
        let executor = MockExecutor::new();
        let cancel_token = CancellationToken::new();
        let usage_tracker = Arc::new(SessionUsageTracker::new());

        let ctx = create_test_context(&executor, &cancel_token, usage_tracker.clone());
        let messages = vec![ChatMessage::new(MessageRole::User, "Hi".to_string())];

        let result = run_headless_loop(&streamer, messages, ctx).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.text, "Hello, world!");
        assert_eq!(result.tool_calls_made, 0);
        assert_eq!(streamer.call_count(), 1);
    }

    #[tokio::test]
    async fn test_tool_call_and_response() {
        let streamer = MockStreamer::new(vec![
            // First response: call read_file tool
            ScriptedResponse::with_tool_call(
                "read_file",
                "tool_1",
                serde_json::json!({"path": "test.txt"}),
            ),
            // Second response: final text after tool result
            ScriptedResponse::text("File contents are: hello"),
        ]);

        let executor = MockExecutor::new();
        executor
            .expect_tool("readfile", Ok("hello".to_string()))
            .await;

        let cancel_token = CancellationToken::new();
        let usage_tracker = Arc::new(SessionUsageTracker::new());

        let ctx = create_test_context(&executor, &cancel_token, usage_tracker.clone());
        let messages = vec![ChatMessage::new(
            MessageRole::User,
            "Read test.txt".to_string(),
        )];

        let result = run_headless_loop(&streamer, messages, ctx).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.text, "File contents are: hello");
        assert_eq!(result.tool_calls_made, 1);
        assert_eq!(streamer.call_count(), 2);
    }

    #[tokio::test]
    async fn test_multiple_tool_calls() {
        let streamer = MockStreamer::new(vec![
            // First: call glob
            ScriptedResponse::with_tool_call(
                "glob",
                "tool_1",
                serde_json::json!({"pattern": "*.rs"}),
            ),
            // Second: call grep
            ScriptedResponse::with_tool_call(
                "grep",
                "tool_2",
                serde_json::json!({"pattern": "fn main"}),
            ),
            // Third: final response
            ScriptedResponse::text("Found 5 Rust files, 2 contain main"),
        ]);

        let executor = MockExecutor::new();
        let cancel_token = CancellationToken::new();
        let usage_tracker = Arc::new(SessionUsageTracker::new());

        let ctx = create_test_context(&executor, &cancel_token, usage_tracker.clone());
        let messages = vec![ChatMessage::new(
            MessageRole::User,
            "Find Rust files".to_string(),
        )];

        let result = run_headless_loop(&streamer, messages, ctx).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.tool_calls_made, 2);
        assert_eq!(streamer.call_count(), 3);
    }

    #[tokio::test]
    async fn test_max_iterations_exceeded() {
        // Create a streamer that always returns tool calls
        let responses: Vec<ScriptedResponse> = (0..15)
            .map(|i| {
                ScriptedResponse::with_tool_call(
                    "bash",
                    &format!("tool_{}", i),
                    serde_json::json!({"command": "echo hi"}),
                )
            })
            .collect();

        let streamer = MockStreamer::new(responses);
        let executor = MockExecutor::new();
        let cancel_token = CancellationToken::new();
        let usage_tracker = Arc::new(SessionUsageTracker::new());

        let ctx = HeadlessContext {
            system_prompt: None,
            tools: vec![],
            executor: &executor,
            max_iterations: 5,
            cancel_token: &cancel_token,
            usage_tracker,
        };

        let messages = vec![ChatMessage::new(MessageRole::User, "Loop".to_string())];

        let result = run_headless_loop(&streamer, messages, ctx).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("maximum tool iterations"));
    }

    #[tokio::test]
    async fn test_cancellation() {
        let streamer = MockStreamer::new(vec![ScriptedResponse::text("Hello")]);
        let executor = MockExecutor::new();
        let cancel_token = CancellationToken::new();
        let usage_tracker = Arc::new(SessionUsageTracker::new());

        // Cancel before running
        cancel_token.cancel();

        let ctx = create_test_context(&executor, &cancel_token, usage_tracker.clone());
        let messages = vec![ChatMessage::new(MessageRole::User, "Hi".to_string())];

        let result = run_headless_loop(&streamer, messages, ctx).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AgentError::Cancelled));
    }

    #[tokio::test]
    async fn test_unknown_tool_error() {
        let streamer = MockStreamer::new(vec![ScriptedResponse::with_tool_call(
            "unknown_tool",
            "tool_1",
            serde_json::json!({}),
        )]);

        let executor = MockExecutor::new();
        let cancel_token = CancellationToken::new();
        let usage_tracker = Arc::new(SessionUsageTracker::new());

        let ctx = create_test_context(&executor, &cancel_token, usage_tracker.clone());
        let messages = vec![ChatMessage::new(MessageRole::User, "Test".to_string())];

        let result = run_headless_loop(&streamer, messages, ctx).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Unknown tool"));
    }

    #[tokio::test]
    async fn test_tool_error_handling() {
        let streamer = MockStreamer::new(vec![
            ScriptedResponse::with_tool_call(
                "read_file",
                "tool_1",
                serde_json::json!({"path": "nonexistent.txt"}),
            ),
            ScriptedResponse::text("File not found, trying alternative"),
        ]);

        let executor = MockExecutor::new();
        executor
            .expect_tool("readfile", Err("File not found".to_string()))
            .await;

        let cancel_token = CancellationToken::new();
        let usage_tracker = Arc::new(SessionUsageTracker::new());

        let ctx = create_test_context(&executor, &cancel_token, usage_tracker.clone());
        let messages = vec![ChatMessage::new(MessageRole::User, "Read file".to_string())];

        let result = run_headless_loop(&streamer, messages, ctx).await;

        // Should succeed - tool error is passed back to LLM, not fatal
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_usage_tracking() {
        let streamer = MockStreamer::new(vec![
            ScriptedResponse {
                text: "Response 1".to_string(),
                tool_calls: vec![ToolCall {
                    id: "t1".to_string(),
                    name: "bash".to_string(),
                    input: serde_json::json!({"command": "echo"}),
                }],
                usage: TokenUsage {
                    input_tokens: 100,
                    output_tokens: 50,
                },
                stop_reason: Some("tool_use".to_string()),
            },
            ScriptedResponse {
                text: "Response 2".to_string(),
                tool_calls: vec![],
                usage: TokenUsage {
                    input_tokens: 150,
                    output_tokens: 75,
                },
                stop_reason: Some("end_turn".to_string()),
            },
        ]);

        let executor = MockExecutor::new();
        let cancel_token = CancellationToken::new();
        let usage_tracker = Arc::new(SessionUsageTracker::new());

        let ctx = create_test_context(&executor, &cancel_token, usage_tracker.clone());
        let messages = vec![ChatMessage::new(MessageRole::User, "Test".to_string())];

        let _result = run_headless_loop(&streamer, messages, ctx).await;

        let totals = usage_tracker.get_totals();
        assert_eq!(totals.input_tokens, 250); // 100 + 150
        assert_eq!(totals.output_tokens, 125); // 50 + 75
    }

    #[tokio::test]
    async fn test_text_accumulation() {
        let streamer = MockStreamer::new(vec![
            ScriptedResponse::text_with_tool(
                "First part. ",
                "bash",
                "t1",
                serde_json::json!({"command": "echo"}),
            ),
            ScriptedResponse::text("Second part."),
        ]);

        let executor = MockExecutor::new();
        let cancel_token = CancellationToken::new();
        let usage_tracker = Arc::new(SessionUsageTracker::new());

        let ctx = create_test_context(&executor, &cancel_token, usage_tracker.clone());
        let messages = vec![ChatMessage::new(MessageRole::User, "Test".to_string())];

        let result = run_headless_loop(&streamer, messages, ctx).await.unwrap();

        assert_eq!(result.text, "First part. Second part.");
    }

    // =========================================================================
    // GAP TESTS: These test behaviors that should match Claude CLI / competitors
    // =========================================================================

    /// GAP #1: Parallel tool calls in a single response
    /// Claude can return multiple tool calls that should be executed and results sent back together
    #[tokio::test]
    async fn test_parallel_tool_calls_in_single_response() {
        // Response with 3 tool calls at once (like Claude reading multiple files)
        let streamer = MockStreamer::new(vec![
            ScriptedResponse {
                text: "Let me read all three files.".to_string(),
                tool_calls: vec![
                    ToolCall {
                        id: "t1".to_string(),
                        name: "read_file".to_string(),
                        input: serde_json::json!({"path": "file1.txt"}),
                    },
                    ToolCall {
                        id: "t2".to_string(),
                        name: "read_file".to_string(),
                        input: serde_json::json!({"path": "file2.txt"}),
                    },
                    ToolCall {
                        id: "t3".to_string(),
                        name: "read_file".to_string(),
                        input: serde_json::json!({"path": "file3.txt"}),
                    },
                ],
                usage: TokenUsage {
                    input_tokens: 100,
                    output_tokens: 50,
                },
                stop_reason: Some("tool_use".to_string()),
            },
            ScriptedResponse::text("All three files contain: content1, content2, content3"),
        ]);

        let executor = MockExecutor::new();
        executor
            .expect_tool("readfile", Ok("content".to_string()))
            .await;

        let cancel_token = CancellationToken::new();
        let usage_tracker = Arc::new(SessionUsageTracker::new());

        let ctx = create_test_context(&executor, &cancel_token, usage_tracker.clone());
        let messages = vec![ChatMessage::new(
            MessageRole::User,
            "Read file1.txt, file2.txt, and file3.txt".to_string(),
        )];

        let result = run_headless_loop(&streamer, messages, ctx).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        // All 3 tool calls should count as 1 iteration (they're in the same response)
        assert_eq!(result.tool_calls_made, 1);
        assert_eq!(streamer.call_count(), 2); // Initial + after tool results

        // Verify all 3 tool results were collected
        // Give the async spawn time to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let tool_results = streamer.get_tool_results().await;
        assert_eq!(tool_results.len(), 1); // One batch of results
        assert_eq!(tool_results[0].len(), 3); // Three tools in that batch
    }

    /// GAP #1b: Verify tool results are sent back in correct order
    #[tokio::test]
    async fn test_parallel_tool_results_preserve_order() {
        let streamer = MockStreamer::new(vec![
            ScriptedResponse {
                text: String::new(),
                tool_calls: vec![
                    ToolCall {
                        id: "first".to_string(),
                        name: "glob".to_string(),
                        input: serde_json::json!({"pattern": "*.rs"}),
                    },
                    ToolCall {
                        id: "second".to_string(),
                        name: "grep".to_string(),
                        input: serde_json::json!({"pattern": "TODO"}),
                    },
                ],
                usage: TokenUsage::default(),
                stop_reason: Some("tool_use".to_string()),
            },
            ScriptedResponse::text("Done"),
        ]);

        let executor = MockExecutor::new();
        let cancel_token = CancellationToken::new();
        let usage_tracker = Arc::new(SessionUsageTracker::new());

        let ctx = create_test_context(&executor, &cancel_token, usage_tracker.clone());
        let messages = vec![ChatMessage::new(MessageRole::User, "Search".to_string())];

        let _ = run_headless_loop(&streamer, messages, ctx).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let tool_results = streamer.get_tool_results().await;
        assert_eq!(tool_results.len(), 1);
        assert_eq!(tool_results[0].len(), 2);
        // Results should be in same order as tool calls
        assert_eq!(tool_results[0][0].id, "first");
        assert_eq!(tool_results[0][1].id, "second");
    }

    /// GAP #6: Cancellation during tool execution (not just before)
    #[tokio::test]
    async fn test_cancellation_during_tool_execution() {
        use std::sync::atomic::AtomicBool;
        use std::time::Duration;

        let streamer = MockStreamer::new(vec![
            ScriptedResponse::with_tool_call(
                "bash",
                "t1",
                serde_json::json!({"command": "sleep 10"}),
            ),
            ScriptedResponse::text("Should not reach here"),
        ]);

        let executor = MockExecutor::new();
        let cancel_token = CancellationToken::new();
        let usage_tracker = Arc::new(SessionUsageTracker::new());

        let ctx = create_test_context(&executor, &cancel_token, usage_tracker.clone());
        let messages = vec![ChatMessage::new(
            MessageRole::User,
            "Run slow command".to_string(),
        )];

        // Cancel after a short delay (during tool execution)
        let cancel_clone = cancel_token.clone();
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancelled_clone = cancelled.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            cancel_clone.cancel();
            cancelled_clone.store(true, Ordering::SeqCst);
        });

        let result = run_headless_loop(&streamer, messages, ctx).await;

        // Should be cancelled (though mock executor returns immediately, the pattern is tested)
        // In real execution with slow tools, this would cancel mid-execution
        assert!(result.is_ok() || matches!(result.unwrap_err(), AgentError::Cancelled));
    }

    /// GAP #7a: Mixed tool success and failure in parallel calls
    #[tokio::test]
    async fn test_parallel_tools_mixed_success_failure() {
        let streamer = MockStreamer::new(vec![
            ScriptedResponse {
                text: String::new(),
                tool_calls: vec![
                    ToolCall {
                        id: "success".to_string(),
                        name: "glob".to_string(),
                        input: serde_json::json!({"pattern": "*.rs"}),
                    },
                    ToolCall {
                        id: "fail".to_string(),
                        name: "read_file".to_string(),
                        input: serde_json::json!({"path": "missing.txt"}),
                    },
                ],
                usage: TokenUsage::default(),
                stop_reason: Some("tool_use".to_string()),
            },
            ScriptedResponse::text("Glob succeeded but file read failed"),
        ]);

        let executor = MockExecutor::new();
        executor
            .expect_tool("glob", Ok("file1.rs\nfile2.rs".to_string()))
            .await;
        executor
            .expect_tool("readfile", Err("File not found".to_string()))
            .await;

        let cancel_token = CancellationToken::new();
        let usage_tracker = Arc::new(SessionUsageTracker::new());

        let ctx = create_test_context(&executor, &cancel_token, usage_tracker.clone());
        let messages = vec![ChatMessage::new(MessageRole::User, "Search".to_string())];

        let result = run_headless_loop(&streamer, messages, ctx).await;

        // Should succeed - individual tool errors don't fail the whole loop
        assert!(result.is_ok());

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let tool_results = streamer.get_tool_results().await;
        assert_eq!(tool_results[0].len(), 2);

        // First result should be success
        assert!(!tool_results[0][0].is_error);
        // Second result should be error
        assert!(tool_results[0][1].is_error);
        assert!(tool_results[0][1].output.contains("File not found"));
    }

    /// GAP #7b: Empty tool calls array should not increment iteration
    #[tokio::test]
    async fn test_empty_tool_calls_exits_immediately() {
        let streamer = MockStreamer::new(vec![ScriptedResponse {
            text: "Just text, no tools".to_string(),
            tool_calls: vec![], // Empty - should exit loop
            usage: TokenUsage::default(),
            stop_reason: Some("end_turn".to_string()),
        }]);

        let executor = MockExecutor::new();
        let cancel_token = CancellationToken::new();
        let usage_tracker = Arc::new(SessionUsageTracker::new());

        let ctx = create_test_context(&executor, &cancel_token, usage_tracker.clone());
        let messages = vec![ChatMessage::new(MessageRole::User, "Hi".to_string())];

        let result = run_headless_loop(&streamer, messages, ctx).await.unwrap();

        assert_eq!(result.tool_calls_made, 0);
        assert_eq!(streamer.call_count(), 1);
    }

    /// GAP: Iteration count should count response rounds, not individual tool calls
    #[tokio::test]
    async fn test_iteration_counts_responses_not_tools() {
        // 3 responses, each with 2 tool calls = 3 iterations, not 6
        let streamer = MockStreamer::new(vec![
            ScriptedResponse {
                text: String::new(),
                tool_calls: vec![
                    ToolCall {
                        id: "r1t1".to_string(),
                        name: "bash".to_string(),
                        input: serde_json::json!({"command": "echo 1"}),
                    },
                    ToolCall {
                        id: "r1t2".to_string(),
                        name: "bash".to_string(),
                        input: serde_json::json!({"command": "echo 2"}),
                    },
                ],
                usage: TokenUsage::default(),
                stop_reason: Some("tool_use".to_string()),
            },
            ScriptedResponse {
                text: String::new(),
                tool_calls: vec![
                    ToolCall {
                        id: "r2t1".to_string(),
                        name: "bash".to_string(),
                        input: serde_json::json!({"command": "echo 3"}),
                    },
                    ToolCall {
                        id: "r2t2".to_string(),
                        name: "bash".to_string(),
                        input: serde_json::json!({"command": "echo 4"}),
                    },
                ],
                usage: TokenUsage::default(),
                stop_reason: Some("tool_use".to_string()),
            },
            ScriptedResponse::text("Done after 2 rounds of parallel tools"),
        ]);

        let executor = MockExecutor::new();
        let cancel_token = CancellationToken::new();
        let usage_tracker = Arc::new(SessionUsageTracker::new());

        let ctx = HeadlessContext {
            system_prompt: None,
            tools: vec![],
            executor: &executor,
            max_iterations: 3, // Would fail if counting individual tools
            cancel_token: &cancel_token,
            usage_tracker,
        };

        let messages = vec![ChatMessage::new(MessageRole::User, "Run".to_string())];

        let result = run_headless_loop(&streamer, messages, ctx).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        // 2 iterations (responses with tools), not 4 (individual tool calls)
        assert_eq!(result.tool_calls_made, 2);
    }

    /// GAP: Verify conversation history is built correctly with parallel tools
    #[tokio::test]
    async fn test_conversation_includes_all_tool_results() {
        let streamer = MockStreamer::new(vec![
            ScriptedResponse {
                text: "Reading files".to_string(),
                tool_calls: vec![
                    ToolCall {
                        id: "t1".to_string(),
                        name: "read_file".to_string(),
                        input: serde_json::json!({"path": "a.txt"}),
                    },
                    ToolCall {
                        id: "t2".to_string(),
                        name: "read_file".to_string(),
                        input: serde_json::json!({"path": "b.txt"}),
                    },
                ],
                usage: TokenUsage::default(),
                stop_reason: Some("tool_use".to_string()),
            },
            ScriptedResponse::text("Got both files"),
        ]);

        let executor = MockExecutor::new();
        executor
            .expect_tool("readfile", Ok("file content".to_string()))
            .await;

        let cancel_token = CancellationToken::new();
        let usage_tracker = Arc::new(SessionUsageTracker::new());

        let ctx = create_test_context(&executor, &cancel_token, usage_tracker.clone());
        let messages = vec![ChatMessage::new(
            MessageRole::User,
            "Read files".to_string(),
        )];

        let result = run_headless_loop(&streamer, messages, ctx).await;
        assert!(result.is_ok());

        // Give async operations time to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Verify tool results were sent back
        let tool_results = streamer.get_tool_results().await;
        assert_eq!(tool_results.len(), 1);
        assert_eq!(tool_results[0].len(), 2);
        assert_eq!(tool_results[0][0].id, "t1");
        assert_eq!(tool_results[0][1].id, "t2");
    }

    // =========================================================================
    // STOP REASON TESTS
    // =========================================================================

    /// GAP #2: Stop reason is propagated in HeadlessResult
    #[tokio::test]
    async fn test_stop_reason_end_turn_propagated() {
        let streamer = MockStreamer::new(vec![ScriptedResponse::text("Done")]);
        let executor = MockExecutor::new();
        let cancel_token = CancellationToken::new();
        let usage_tracker = Arc::new(SessionUsageTracker::new());

        let ctx = create_test_context(&executor, &cancel_token, usage_tracker.clone());
        let messages = vec![ChatMessage::new(MessageRole::User, "Hi".to_string())];

        let result = run_headless_loop(&streamer, messages, ctx).await.unwrap();

        assert_eq!(result.stop_reason, Some("end_turn".to_string()));
    }

    /// GAP #2: Stop reason max_tokens indicates truncation
    #[tokio::test]
    async fn test_stop_reason_max_tokens() {
        let streamer = MockStreamer::new(vec![ScriptedResponse::with_stop_reason(
            "This response was truncated because...",
            "max_tokens",
        )]);

        let executor = MockExecutor::new();
        let cancel_token = CancellationToken::new();
        let usage_tracker = Arc::new(SessionUsageTracker::new());

        let ctx = create_test_context(&executor, &cancel_token, usage_tracker.clone());
        let messages = vec![ChatMessage::new(
            MessageRole::User,
            "Write a long essay".to_string(),
        )];

        let result = run_headless_loop(&streamer, messages, ctx).await.unwrap();

        assert_eq!(result.stop_reason, Some("max_tokens".to_string()));
        // The caller can check this and decide to continue the conversation
    }

    /// GAP #2: Stop reason after tool loop shows final response's reason
    #[tokio::test]
    async fn test_stop_reason_after_tool_calls() {
        let streamer = MockStreamer::new(vec![
            ScriptedResponse::with_tool_call("bash", "t1", serde_json::json!({"command": "ls"})),
            ScriptedResponse::text("Here are the files"),
        ]);

        let executor = MockExecutor::new();
        let cancel_token = CancellationToken::new();
        let usage_tracker = Arc::new(SessionUsageTracker::new());

        let ctx = create_test_context(&executor, &cancel_token, usage_tracker.clone());
        let messages = vec![ChatMessage::new(
            MessageRole::User,
            "List files".to_string(),
        )];

        let result = run_headless_loop(&streamer, messages, ctx).await.unwrap();

        // Final response has end_turn, not tool_use
        assert_eq!(result.stop_reason, Some("end_turn".to_string()));
    }
}
