//! Types for behavioral evaluation of agent outputs.

use serde::{Deserialize, Serialize};

/// A test case for evaluating agent behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalCase {
    /// Unique identifier for this test case
    pub id: String,
    /// Human-readable description
    pub description: String,
    /// The prompt to send to the agent
    pub prompt: String,
    /// Expected behaviors to check
    pub expected: ExpectedBehavior,
    /// Optional setup files to create before running
    #[serde(default)]
    pub setup_files: Vec<SetupFile>,
}

/// Files to create before running the eval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupFile {
    pub path: String,
    pub content: String,
}

/// Expected behaviors for comparison
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExpectedBehavior {
    /// Should the agent use todos for this task?
    #[serde(default)]
    pub uses_todos: Option<bool>,

    /// Should this be read-only (no write/edit tools)?
    #[serde(default)]
    pub read_only: Option<bool>,

    /// Should the agent search before reading specific files?
    #[serde(default)]
    pub searches_before_reading: Option<bool>,

    /// Tools that MUST be used
    #[serde(default)]
    pub required_tools: Vec<String>,

    /// Tools that must NOT be used
    #[serde(default)]
    pub forbidden_tools: Vec<String>,

    /// Patterns that must appear in output (regex)
    #[serde(default)]
    pub output_must_contain: Vec<String>,

    /// Patterns that must NOT appear in output
    #[serde(default)]
    pub output_must_not_contain: Vec<String>,

    /// Expected tool call order (partial match)
    #[serde(default)]
    pub tool_sequence: Vec<String>,

    /// Minimum number of files to read
    #[serde(default)]
    pub min_files_read: Option<u32>,

    /// Should output have structured sections (for reviews)?
    #[serde(default)]
    pub structured_output: Option<bool>,
}

/// Result of running an eval case
#[derive(Debug, Clone, Serialize)]
pub struct EvalResult {
    /// The case that was run
    pub case_id: String,
    /// Overall pass/fail
    pub passed: bool,
    /// Individual assertion results
    pub assertions: Vec<AssertionResult>,
    /// Tools that were called
    pub tools_used: Vec<ToolUsage>,
    /// Tool call sequence (names only)
    pub tool_sequence: Vec<String>,
    /// Final output text from agent
    pub output: String,
    /// Number of LLM iterations
    pub iterations: u32,
    /// Total tokens used
    pub tokens_used: u32,
    /// Execution time in milliseconds
    pub duration_ms: u64,
}

/// A single tool usage record
#[derive(Debug, Clone, Serialize)]
pub struct ToolUsage {
    pub name: String,
    pub input: serde_json::Value,
    pub output: String,
    pub is_error: bool,
}

/// Result of a single assertion
#[derive(Debug, Clone, Serialize)]
pub struct AssertionResult {
    pub name: String,
    pub passed: bool,
    pub expected: String,
    pub actual: String,
    pub severity: Severity,
}

/// Severity of assertion failure
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Critical - fundamental behavior wrong
    Critical,
    /// Major - significant issue
    Major,
    /// Minor - suboptimal but acceptable
    Minor,
}

/// Summary of multiple eval runs
#[derive(Debug, Clone, Serialize)]
pub struct EvalSummary {
    pub total_cases: u32,
    pub passed: u32,
    pub failed: u32,
    pub critical_failures: u32,
    pub major_failures: u32,
    pub minor_failures: u32,
    pub results: Vec<EvalResult>,
}

impl EvalSummary {
    pub fn new() -> Self {
        Self {
            total_cases: 0,
            passed: 0,
            failed: 0,
            critical_failures: 0,
            major_failures: 0,
            minor_failures: 0,
            results: Vec::new(),
        }
    }

    pub fn add_result(&mut self, result: EvalResult) {
        self.total_cases += 1;
        if result.passed {
            self.passed += 1;
        } else {
            self.failed += 1;
            for assertion in &result.assertions {
                if !assertion.passed {
                    match assertion.severity {
                        Severity::Critical => self.critical_failures += 1,
                        Severity::Major => self.major_failures += 1,
                        Severity::Minor => self.minor_failures += 1,
                    }
                }
            }
        }
        self.results.push(result);
    }

    pub fn pass_rate(&self) -> f64 {
        if self.total_cases == 0 {
            0.0
        } else {
            (self.passed as f64 / self.total_cases as f64) * 100.0
        }
    }
}

impl Default for EvalSummary {
    fn default() -> Self {
        Self::new()
    }
}
