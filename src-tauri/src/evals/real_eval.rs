//! Real eval runner for testing against actual LLM providers.
//!
//! These tests make real API calls and should be run manually with:
//! ```
//! cargo test evals::real_eval --ignored -- --nocapture
//! ```

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::agent::error::AgentError;
use crate::agent::providers::{
    run_headless_loop, AnthropicAdapter, GeminiAdapter, HeadlessContext, HeadlessStreamer,
};
use crate::agent::tools::{get_tool_definitions, LocalExecutor, SessionState};
use crate::agent::usage::SessionUsageTracker;
use crate::config::ConfigService;

use super::cases;
use super::integration::CollectingExecutor;
use super::runner::{evaluate_case, AgentRunData};

/// Run a single eval case against the real provider (Anthropic or Gemini based on config)
pub async fn run_real_eval(
    project_path: &std::path::Path,
    case_id: &str,
) -> Result<(AgentRunData, super::types::EvalResult), AgentError> {
    let case = cases::get_all_cases()
        .into_iter()
        .find(|c| c.id == case_id)
        .ok_or_else(|| AgentError::ToolExecutionError(format!("Unknown case: {}", case_id)))?;

    // Load project config
    let project_config = ConfigService::load_project_config(project_path)
        .map_err(|e| AgentError::ConfigError(e.to_string()))?;

    let provider = project_config.agent.provider.to_lowercase();

    // Create executor with collecting wrapper
    let cancel_token = CancellationToken::new();
    let usage_tracker = Arc::new(SessionUsageTracker::new());
    let session = SessionState::new();

    let executor = LocalExecutor::with_session(
        project_path.to_path_buf(),
        project_config.execution.timeout_secs,
        session.clone(),
        cancel_token.clone(),
        usage_tracker.clone(),
    );

    let collecting_executor = CollectingExecutor::new(executor);

    // Build message
    let messages = vec![crate::agent::types::ChatMessage::new(
        crate::agent::types::MessageRole::User,
        case.prompt.clone(),
    )];

    let start = std::time::Instant::now();

    // Run with appropriate provider - capture partial results even on error
    let run_result = match provider.as_str() {
        "anthropic" => {
            let adapter = AnthropicAdapter::new(
                project_config.agent.clone(),
                project_config.prompts,
                project_config.execution.clone(),
                project_path.to_path_buf(),
                crate::agent::providers::DEFAULT_SYSTEM_PROMPT,
                project_config.extraction_prompt,
            )?;
            run_with_streamer(
                &adapter,
                messages,
                &collecting_executor,
                &cancel_token,
                usage_tracker.clone(),
            )
            .await
        }
        "gemini" => {
            let adapter = GeminiAdapter::new(
                project_config.agent.clone(),
                project_config.prompts,
                project_config.execution.clone(),
                project_path.to_path_buf(),
                crate::agent::providers::DEFAULT_SYSTEM_PROMPT,
                project_config.extraction_prompt,
            )?;
            run_with_streamer(
                &adapter,
                messages,
                &collecting_executor,
                &cancel_token,
                usage_tracker.clone(),
            )
            .await
        }
        _ => return Err(AgentError::UnsupportedProvider(provider)),
    };

    let duration_ms = start.elapsed().as_millis() as u64;
    let totals = usage_tracker.get_totals();

    // Even on error, we have partial tool usage data
    let tools_used = collecting_executor.get_collected();

    let (output, iterations, error_note) = match run_result {
        Ok(result) => (result.text, result.tool_calls_made, None),
        Err(ref e) => {
            let note = format!("Run ended with error: {:?}", e);
            (note.clone(), tools_used.len() as u32, Some(note))
        }
    };

    let run_data = AgentRunData {
        tools_used,
        output,
        iterations,
        tokens_used: totals.input_tokens + totals.output_tokens,
        duration_ms,
    };

    let mut eval_result = evaluate_case(&case, &run_data);

    // If we hit an error like max iterations, that itself is a failure indicator
    if let Some(note) = error_note {
        eval_result.passed = false;
        eval_result.output = format!("{}\n\n[ERROR: {}]", eval_result.output, note);
    }

    Ok((run_data, eval_result))
}

/// Helper to run with any HeadlessStreamer
async fn run_with_streamer<S: HeadlessStreamer>(
    streamer: &S,
    messages: Vec<crate::agent::types::ChatMessage>,
    collecting_executor: &CollectingExecutor<LocalExecutor>,
    cancel_token: &CancellationToken,
    usage_tracker: Arc<SessionUsageTracker>,
) -> Result<crate::agent::provider::HeadlessResult, AgentError> {
    let ctx = HeadlessContext {
        system_prompt: Some(crate::agent::providers::DEFAULT_SYSTEM_PROMPT.to_string()),
        tools: get_tool_definitions(),
        executor: collecting_executor,
        max_iterations: 20,
        cancel_token,
        usage_tracker,
    };

    run_headless_loop(streamer, messages, ctx).await
}

/// Print a detailed eval report
pub fn print_eval_report(run_data: &AgentRunData, eval_result: &super::types::EvalResult) {
    println!("\n{}", "=".repeat(60));
    println!("EVAL REPORT: {}", eval_result.case_id);
    println!("{}", "=".repeat(60));

    println!(
        "\n## Status: {}",
        if eval_result.passed {
            "PASS ✓"
        } else {
            "FAIL ✗"
        }
    );

    println!("\n## Metrics");
    println!("  - Duration: {}ms", run_data.duration_ms);
    println!("  - Tokens: {}", run_data.tokens_used);
    println!("  - Tool iterations: {}", run_data.iterations);
    println!("  - Tools used: {}", run_data.tools_used.len());

    println!("\n## Tool Sequence");
    for (i, tool) in run_data.tools_used.iter().enumerate() {
        let status = if tool.is_error { "ERROR" } else { "OK" };
        println!("  {}. {} [{}]", i + 1, tool.name, status);
    }

    println!("\n## Assertions");
    for assertion in &eval_result.assertions {
        let status = if assertion.passed { "✓" } else { "✗" };
        println!(
            "  [{}] {:?} {} - {}",
            status, assertion.severity, assertion.name, assertion.expected
        );
        if !assertion.passed {
            println!("       Actual: {}", assertion.actual);
        }
    }

    println!("\n## Agent Output (first 500 chars)");
    let output_preview = if run_data.output.len() > 500 {
        format!("{}...", &run_data.output[..500])
    } else {
        run_data.output.clone()
    };
    println!("{}", output_preview);

    println!("\n{}", "=".repeat(60));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Run the senior_architect_review eval against real LLM API.
    ///
    /// Run with: GEMINI_API_KEY=your_key cargo test evals::real_eval::tests::test_senior_architect_review_real -- --ignored --nocapture
    /// Or with Anthropic: Change config.toml to use anthropic and run with ANTHROPIC_API_KEY
    #[tokio::test]
    #[ignore = "Requires API key and makes real API calls"]
    async fn test_senior_architect_review_real() {
        // Use the current project as the test project
        let project_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();

        println!("\nRunning eval against project: {:?}", project_path);

        let result = run_real_eval(&project_path, "senior_architect_review").await;

        match result {
            Ok((run_data, eval_result)) => {
                print_eval_report(&run_data, &eval_result);

                // Don't assert pass - we want to see the result either way
                if !eval_result.passed {
                    println!(
                        "\n⚠️  EVAL FAILED - This indicates DevFlow behavior differs from expected"
                    );
                }
            }
            Err(AgentError::MissingApiKey(key)) => {
                println!("\n⚠️  Skipping eval - {} not set", key);
                println!("   Set the environment variable and re-run:");
                println!("   {}=your_key cargo test evals::real_eval::tests::test_senior_architect_review_real -- --ignored --nocapture", key);
            }
            Err(e) => {
                println!("\n❌ Error running eval: {:?}", e);
                panic!("Eval failed with error: {:?}", e);
            }
        }
    }

    /// Run code_review_no_changes eval
    #[tokio::test]
    #[ignore = "Requires ANTHROPIC_API_KEY and makes real API calls"]
    async fn test_code_review_no_changes_real() {
        let project_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();

        let result = run_real_eval(&project_path, "code_review_no_changes").await;

        match result {
            Ok((run_data, eval_result)) => {
                print_eval_report(&run_data, &eval_result);
            }
            Err(e) => {
                println!("\n❌ Error running eval: {:?}", e);
            }
        }
    }

    /// Run all review evals
    #[tokio::test]
    #[ignore = "Requires ANTHROPIC_API_KEY and makes real API calls"]
    async fn test_all_review_evals() {
        let project_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();

        let review_cases = cases::get_cases_by_category("review");

        println!("\n🧪 Running {} review evals...\n", review_cases.len());

        let mut passed = 0;
        let mut failed = 0;

        for case in review_cases {
            println!("\n--- Running: {} ---", case.id);

            match run_real_eval(&project_path, &case.id).await {
                Ok((run_data, eval_result)) => {
                    print_eval_report(&run_data, &eval_result);
                    if eval_result.passed {
                        passed += 1;
                    } else {
                        failed += 1;
                    }
                }
                Err(e) => {
                    println!("❌ Error: {:?}", e);
                    failed += 1;
                }
            }
        }

        println!("\n{}", "=".repeat(60));
        println!("SUMMARY: {} passed, {} failed", passed, failed);
        println!("{}", "=".repeat(60));
    }
}
