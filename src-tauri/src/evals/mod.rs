//! Behavioral evaluation framework for testing agent quality.
//!
//! This module provides tools for comparing DevFlow agent behavior against
//! expected patterns (based on competitor analysis like Claude CLI).
//!
//! # Usage
//!
//! 1. Define test cases with expected behaviors
//! 2. Run the agent and collect tool usage + output
//! 3. Evaluate against expectations
//! 4. Generate reports to identify gaps
//!
//! # Example
//!
//! ```rust,ignore
//! use devflow::evals::{cases, runner, integration};
//!
//! // Get a test case
//! let case = cases::senior_architect_review();
//!
//! // Run your agent using the integration
//! let run_data = integration::run_eval_case(
//!     &streamer,
//!     executor,
//!     &case.prompt,
//!     integration::EvalRunConfig::default(),
//! ).await?;
//!
//! // Evaluate
//! let result = runner::evaluate_case(&case, &run_data);
//!
//! // Check results
//! if !result.passed {
//!     println!("{}", runner::format_result(&result));
//! }
//! ```

pub mod cases;
pub mod integration;
pub mod real_eval;
pub mod runner;
pub mod types;

pub use cases::{get_all_cases, get_cases_by_category};
pub use integration::{
    run_eval_case, run_eval_case_with_timeout, CollectingExecutor, EvalRunConfig,
};
pub use runner::{evaluate_case, format_result, format_summary, run_eval_suite, AgentRunData};
pub use types::{EvalCase, EvalResult, EvalSummary, ExpectedBehavior, Severity, ToolUsage};
