//! Eval runner for executing test cases and collecting results.

// Allow regex creation in loops - each pattern needs individual checking
#![allow(clippy::regex_creation_in_loops)]

use regex::Regex;

use super::types::{AssertionResult, EvalCase, EvalResult, EvalSummary, Severity, ToolUsage};

/// Collected data from an agent run (to be filled by the actual runner)
#[derive(Debug, Clone)]
pub struct AgentRunData {
    pub tools_used: Vec<ToolUsage>,
    pub output: String,
    pub iterations: u32,
    pub tokens_used: u32,
    pub duration_ms: u64,
}

/// Check all assertions for a case against the actual run data
pub fn evaluate_case(case: &EvalCase, run_data: &AgentRunData) -> EvalResult {
    let mut assertions = Vec::new();
    let tool_names: Vec<String> = run_data.tools_used.iter().map(|t| t.name.clone()).collect();

    // Check read-only constraint
    if let Some(expected_read_only) = case.expected.read_only {
        if expected_read_only {
            let write_tools = ["write_file", "edit_file", "multi_edit"];
            let used_write = tool_names.iter().any(|t| write_tools.contains(&t.as_str()));
            assertions.push(AssertionResult {
                name: "read_only".to_string(),
                passed: !used_write,
                expected: "No write/edit tools".to_string(),
                actual: if used_write {
                    format!(
                        "Used: {:?}",
                        tool_names
                            .iter()
                            .filter(|t| write_tools.contains(&t.as_str()))
                            .collect::<Vec<_>>()
                    )
                } else {
                    "Read-only tools only".to_string()
                },
                severity: Severity::Critical,
            });
        }
    }

    // Check required tools
    for required in &case.expected.required_tools {
        let found = tool_names.iter().any(|t| t == required);
        assertions.push(AssertionResult {
            name: format!("required_tool_{}", required),
            passed: found,
            expected: format!("Should use '{}'", required),
            actual: if found {
                format!("Used '{}'", required)
            } else {
                format!("Did not use '{}'. Used: {:?}", required, tool_names)
            },
            severity: Severity::Major,
        });
    }

    // Check forbidden tools
    for forbidden in &case.expected.forbidden_tools {
        let found = tool_names.iter().any(|t| t == forbidden);
        assertions.push(AssertionResult {
            name: format!("forbidden_tool_{}", forbidden),
            passed: !found,
            expected: format!("Should NOT use '{}'", forbidden),
            actual: if found {
                format!("Used forbidden tool '{}'", forbidden)
            } else {
                "Did not use forbidden tool".to_string()
            },
            severity: Severity::Critical,
        });
    }

    // Check uses_todos
    if let Some(expected_todos) = case.expected.uses_todos {
        let used_todos = tool_names.iter().any(|t| t == "todo_write");
        assertions.push(AssertionResult {
            name: "uses_todos".to_string(),
            passed: used_todos == expected_todos,
            expected: if expected_todos {
                "Should use todo_write".to_string()
            } else {
                "Should not use todo_write".to_string()
            },
            actual: if used_todos {
                "Used todo_write".to_string()
            } else {
                "Did not use todo_write".to_string()
            },
            severity: Severity::Major,
        });
    }

    // Check searches_before_reading
    if let Some(true) = case.expected.searches_before_reading {
        let search_tools = ["glob", "grep"];
        let first_search = tool_names
            .iter()
            .position(|t| search_tools.contains(&t.as_str()));
        let first_read = tool_names.iter().position(|t| t == "read_file");

        let searched_first = match (first_search, first_read) {
            (Some(s), Some(r)) => s < r,
            (Some(_), None) => true,  // Searched but didn't read - ok
            (None, Some(_)) => false, // Read without searching - bad
            (None, None) => true,     // Neither - ok for this check
        };

        assertions.push(AssertionResult {
            name: "searches_before_reading".to_string(),
            passed: searched_first,
            expected: "Should search (glob/grep) before reading files".to_string(),
            actual: format!(
                "Tool sequence: {:?}",
                tool_names.iter().take(5).collect::<Vec<_>>()
            ),
            severity: Severity::Major,
        });
    }

    // Check min_files_read
    if let Some(min_files) = case.expected.min_files_read {
        let files_read = run_data
            .tools_used
            .iter()
            .filter(|t| t.name == "read_file")
            .count() as u32;
        assertions.push(AssertionResult {
            name: "min_files_read".to_string(),
            passed: files_read >= min_files,
            expected: format!("Should read at least {} files", min_files),
            actual: format!("Read {} files", files_read),
            severity: Severity::Minor,
        });
    }

    // Check output patterns (must contain)
    for pattern in &case.expected.output_must_contain {
        let re = Regex::new(pattern).unwrap_or_else(|_| Regex::new(".").unwrap());
        let found = re.is_match(&run_data.output);
        assertions.push(AssertionResult {
            name: format!("output_contains_{}", truncate(pattern, 20)),
            passed: found,
            expected: format!("Output should match: {}", pattern),
            actual: if found {
                "Pattern found".to_string()
            } else {
                format!(
                    "Pattern not found in output (first 200 chars): {}",
                    truncate(&run_data.output, 200)
                )
            },
            severity: Severity::Major,
        });
    }

    // Check output patterns (must NOT contain)
    for pattern in &case.expected.output_must_not_contain {
        let re = Regex::new(pattern).unwrap_or_else(|_| Regex::new(".").unwrap());
        let found = re.is_match(&run_data.output);
        assertions.push(AssertionResult {
            name: format!("output_not_contains_{}", truncate(pattern, 20)),
            passed: !found,
            expected: format!("Output should NOT match: {}", pattern),
            actual: if found {
                "Forbidden pattern found".to_string()
            } else {
                "Pattern not found (good)".to_string()
            },
            severity: Severity::Critical,
        });
    }

    // Check structured output
    if let Some(true) = case.expected.structured_output {
        // Look for markdown headers, bullet points, or numbered lists
        let has_structure = run_data.output.contains('#')
            || run_data.output.contains("- ")
            || run_data.output.contains("1.")
            || run_data.output.contains("* ");
        assertions.push(AssertionResult {
            name: "structured_output".to_string(),
            passed: has_structure,
            expected: "Output should be structured (headers, lists, etc.)".to_string(),
            actual: if has_structure {
                "Has structured formatting".to_string()
            } else {
                "No clear structure detected".to_string()
            },
            severity: Severity::Minor,
        });
    }

    // Determine overall pass/fail
    let passed = assertions.iter().all(|a| a.passed);

    EvalResult {
        case_id: case.id.clone(),
        passed,
        assertions,
        tools_used: run_data.tools_used.clone(),
        tool_sequence: tool_names,
        output: run_data.output.clone(),
        iterations: run_data.iterations,
        tokens_used: run_data.tokens_used,
        duration_ms: run_data.duration_ms,
    }
}

/// Run multiple cases and produce a summary
pub fn run_eval_suite(cases: &[EvalCase], run_data: &[AgentRunData]) -> EvalSummary {
    let mut summary = EvalSummary::new();

    for (case, data) in cases.iter().zip(run_data.iter()) {
        let result = evaluate_case(case, data);
        summary.add_result(result);
    }

    summary
}

/// Format an eval result for display
pub fn format_result(result: &EvalResult) -> String {
    let status = if result.passed { "PASS" } else { "FAIL" };
    let mut output = format!("\n[{}] {}\n", status, result.case_id);
    output.push_str(&format!("  Tools used: {:?}\n", result.tool_sequence));
    output.push_str(&format!(
        "  Iterations: {}, Tokens: {}, Duration: {}ms\n",
        result.iterations, result.tokens_used, result.duration_ms
    ));

    if !result.passed {
        output.push_str("  Failed assertions:\n");
        for assertion in &result.assertions {
            if !assertion.passed {
                output.push_str(&format!(
                    "    [{:?}] {}: {} (got: {})\n",
                    assertion.severity, assertion.name, assertion.expected, assertion.actual
                ));
            }
        }
    }

    output
}

/// Format a summary for display
pub fn format_summary(summary: &EvalSummary) -> String {
    let mut output = String::new();
    output.push_str("\n========== EVAL SUMMARY ==========\n");
    output.push_str(&format!(
        "Total: {} | Passed: {} | Failed: {}\n",
        summary.total_cases, summary.passed, summary.failed
    ));
    output.push_str(&format!("Pass rate: {:.1}%\n", summary.pass_rate()));

    if summary.failed > 0 {
        output.push_str(&format!(
            "Failures by severity: Critical={}, Major={}, Minor={}\n",
            summary.critical_failures, summary.major_failures, summary.minor_failures
        ));
    }

    output.push_str("\n--- Individual Results ---\n");
    for result in &summary.results {
        output.push_str(&format_result(result));
    }

    output
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evals::cases::senior_architect_review;

    fn mock_read_only_run() -> AgentRunData {
        AgentRunData {
            tools_used: vec![
                ToolUsage {
                    name: "bash".to_string(),
                    input: serde_json::json!({"command": "git diff"}),
                    output: "diff output".to_string(),
                    is_error: false,
                },
                ToolUsage {
                    name: "read_file".to_string(),
                    input: serde_json::json!({"path": "SPEC.md"}),
                    output: "spec content".to_string(),
                    is_error: false,
                },
                ToolUsage {
                    name: "read_file".to_string(),
                    input: serde_json::json!({"path": "src/main.rs"}),
                    output: "code".to_string(),
                    is_error: false,
                },
            ],
            output: r#"
## Code Review Summary

### Critical Issues
None found.

### Major Issues
1. Missing error handling in auth module

### Minor Issues
- Consider renaming variable for clarity

### Recommendation
Ready to commit after addressing the major issue.
            "#
            .to_string(),
            iterations: 1,
            tokens_used: 500,
            duration_ms: 2000,
        }
    }

    fn mock_bad_run_uses_edit() -> AgentRunData {
        AgentRunData {
            tools_used: vec![
                ToolUsage {
                    name: "bash".to_string(),
                    input: serde_json::json!({"command": "git diff"}),
                    output: "diff".to_string(),
                    is_error: false,
                },
                ToolUsage {
                    name: "edit_file".to_string(), // BAD: Should not edit during review
                    input: serde_json::json!({"path": "src/main.rs", "old_text": "foo", "new_text": "bar"}),
                    output: "edited".to_string(),
                    is_error: false,
                },
            ],
            output: "Fixed the issue.".to_string(),
            iterations: 1,
            tokens_used: 300,
            duration_ms: 1500,
        }
    }

    #[test]
    fn test_good_review_passes() {
        let case = senior_architect_review();
        let run_data = mock_read_only_run();

        let result = evaluate_case(&case, &run_data);

        assert!(
            result.passed,
            "Good review run should pass: {:?}",
            result.assertions
        );
    }

    #[test]
    fn test_bad_review_fails_on_edit() {
        let case = senior_architect_review();
        let run_data = mock_bad_run_uses_edit();

        let result = evaluate_case(&case, &run_data);

        assert!(!result.passed, "Review that edits should fail");

        // Check that the right assertions failed
        let read_only_assertion = result
            .assertions
            .iter()
            .find(|a| a.name == "read_only")
            .unwrap();
        assert!(!read_only_assertion.passed);
        assert_eq!(read_only_assertion.severity, Severity::Critical);
    }

    #[test]
    fn test_format_result_shows_failures() {
        let case = senior_architect_review();
        let run_data = mock_bad_run_uses_edit();
        let result = evaluate_case(&case, &run_data);

        let formatted = format_result(&result);

        assert!(formatted.contains("FAIL"));
        assert!(formatted.contains("read_only"));
        assert!(formatted.contains("Critical"));
    }
}
