//! Shared utilities for cargo command execution
//!
//! This module provides common functionality used across all cargo command handlers.

use crate::error::CommandError;
use serde_json::{Map, Value};
use std::process::Command;

/// Execute a cargo subcommand with the given arguments
pub fn execute_cargo_command(
    subcommand: &str,
    args: &[&str],
    working_dir: Option<&str>,
) -> Result<String, CommandError> {
    let mut cmd = Command::new("cargo");
    cmd.arg(subcommand);

    for arg in args {
        cmd.arg(arg);
    }

    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }

    let output = cmd.output().map_err(|e| {
        CommandError::ExecutionFailed(format!("Failed to execute cargo {}: {}", subcommand, e))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CommandError::ExecutionFailed(format!(
            "cargo {} failed: {}",
            subcommand, stderr
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Execute a cargo subcommand and return both stdout and stderr
pub fn execute_cargo_command_full(
    subcommand: &str,
    args: &[&str],
    working_dir: Option<&str>,
) -> Result<(String, String), CommandError> {
    let mut cmd = Command::new("cargo");
    cmd.arg(subcommand);

    for arg in args {
        cmd.arg(arg);
    }

    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }

    let output = cmd.output().map_err(|e| {
        CommandError::ExecutionFailed(format!("Failed to execute cargo {}: {}", subcommand, e))
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    if !output.status.success() {
        return Err(CommandError::ExecutionFailed(format!(
            "cargo {} failed: {}",
            subcommand, stderr
        )));
    }

    Ok((stdout, stderr))
}

/// Check if a cargo subcommand is available (installed)
pub fn is_cargo_subcommand_available(subcommand: &str) -> bool {
    let mut cmd = Command::new("cargo");
    cmd.arg(subcommand);
    cmd.arg("--help");

    // We don't care about the output, just whether it succeeds
    let output = cmd.output();
    output.is_ok_and(|o| o.status.success())
}

/// Parse cargo command output to count warnings and errors
pub fn parse_cargo_output(output: &str) -> (u32, u32) {
    let mut warnings = 0;
    let mut errors = 0;

    for line in output.lines() {
        if line.contains("warning:") || line.contains("warning[") {
            warnings += 1;
        } else if line.contains("error:") || line.contains("error[") {
            errors += 1;
        }
    }

    (warnings, errors)
}

/// Parse test output to extract test summary
pub fn parse_test_output(output: &str) -> TestSummary {
    let mut summary = TestSummary {
        tests_run: 0,
        tests_passed: 0,
        tests_failed: 0,
        ignored: 0,
        measured: 0,
        filtered_out: 0,
    };

    // Parse the test summary line
    for line in output.lines() {
        if line.contains("test result:") {
            // Example: "test result: ok. 5 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out"
            let parts: Vec<&str> = line.split_whitespace().collect();

            for (i, part) in parts.iter().enumerate() {
                if *part == "passed;" || *part == "passed," {
                    if i > 0 && parts[i - 1].parse::<u32>().is_ok() {
                        summary.tests_passed = parts[i - 1].parse().unwrap_or(0);
                    }
                } else if *part == "failed;" || *part == "failed," {
                    if i > 0 && parts[i - 1].parse::<u32>().is_ok() {
                        summary.tests_failed = parts[i - 1].parse().unwrap_or(0);
                    }
                } else if *part == "ignored;" || *part == "ignored," {
                    if i > 0 && parts[i - 1].parse::<u32>().is_ok() {
                        summary.ignored = parts[i - 1].parse().unwrap_or(0);
                    }
                } else if *part == "measured;" || *part == "measured," {
                    if i > 0 && parts[i - 1].parse::<u32>().is_ok() {
                        summary.measured = parts[i - 1].parse().unwrap_or(0);
                    }
                } else if *part == "filtered"
                    && i + 1 < parts.len()
                    && parts[i + 1] == "out"
                    && i > 0
                    && parts[i - 1].parse::<u32>().is_ok()
                {
                    summary.filtered_out = parts[i - 1].parse().unwrap_or(0);
                }
            }

            // Calculate total run
            summary.tests_run = summary.tests_passed + summary.tests_failed + summary.ignored;
        }
    }

    summary
}

/// Summary of test results from cargo test output
#[derive(Debug, Clone, Default)]
pub struct TestSummary {
    /// Total number of tests run (passed + failed + ignored)
    pub tests_run: u32,
    /// Number of tests that passed
    pub tests_passed: u32,
    /// Number of tests that failed
    pub tests_failed: u32,
    /// Number of tests that were ignored
    pub ignored: u32,
    /// Number of tests that were measured (benchmark tests)
    pub measured: u32,
    /// Number of tests that were filtered out
    pub filtered_out: u32,
}

/// Helper function to create a boolean property for JSON schemas
pub fn create_bool_property(description: &str) -> Map<String, Value> {
    let mut prop = Map::new();
    prop.insert("type".to_string(), Value::String("boolean".to_string()));
    prop.insert(
        "description".to_string(),
        Value::String(description.to_string()),
    );
    prop
}

/// Helper function to create a string property for JSON schemas
pub fn create_string_property(description: &str) -> Map<String, Value> {
    let mut prop = Map::new();
    prop.insert("type".to_string(), Value::String("string".to_string()));
    prop.insert(
        "description".to_string(),
        Value::String(description.to_string()),
    );
    prop
}

/// Helper function to create a number property for JSON schemas
pub fn create_number_property(description: &str) -> Map<String, Value> {
    let mut prop = Map::new();
    prop.insert("type".to_string(), Value::String("number".to_string()));
    prop.insert(
        "description".to_string(),
        Value::String(description.to_string()),
    );
    prop
}

/// Helper function to create an array property for JSON schemas
pub fn create_array_property(item_type: &str, description: &str) -> Map<String, Value> {
    let mut prop = Map::new();
    prop.insert("type".to_string(), Value::String("array".to_string()));

    let mut items = Map::new();
    items.insert("type".to_string(), Value::String(item_type.to_string()));
    prop.insert("items".to_string(), Value::Object(items));

    prop.insert(
        "description".to_string(),
        Value::String(description.to_string()),
    );
    prop
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_bool_property() {
        let prop = create_bool_property("Test description");

        assert!(prop.contains_key("type"));
        assert!(prop.contains_key("description"));

        assert_eq!(prop["type"], Value::String("boolean".to_string()));
        assert_eq!(
            prop["description"],
            Value::String("Test description".to_string())
        );
    }

    #[test]
    fn test_create_string_property() {
        let prop = create_string_property("Test description");

        assert!(prop.contains_key("type"));
        assert!(prop.contains_key("description"));

        assert_eq!(prop["type"], Value::String("string".to_string()));
        assert_eq!(
            prop["description"],
            Value::String("Test description".to_string())
        );
    }

    #[test]
    fn test_create_number_property() {
        let prop = create_number_property("Test description");

        assert!(prop.contains_key("type"));
        assert!(prop.contains_key("description"));

        assert_eq!(prop["type"], Value::String("number".to_string()));
        assert_eq!(
            prop["description"],
            Value::String("Test description".to_string())
        );
    }

    #[test]
    fn test_create_array_property() {
        let prop = create_array_property("string", "Test description");

        assert!(prop.contains_key("type"));
        assert!(prop.contains_key("items"));
        assert!(prop.contains_key("description"));

        assert_eq!(prop["type"], Value::String("array".to_string()));
        assert_eq!(
            prop["description"],
            Value::String("Test description".to_string())
        );
    }

    #[test]
    fn test_parse_cargo_output() {
        let output = "warning: unused variable\nerror: expected identifier\nwarning: unused import";
        let (warnings, errors) = parse_cargo_output(output);

        assert_eq!(warnings, 2);
        assert_eq!(errors, 1);
    }

    #[test]
    fn test_parse_cargo_output_empty() {
        let output = "";
        let (warnings, errors) = parse_cargo_output(output);

        assert_eq!(warnings, 0);
        assert_eq!(errors, 0);
    }

    #[test]
    fn test_parse_test_output() {
        let output = "test result: ok. 5 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out";
        let summary = parse_test_output(output);

        assert_eq!(summary.tests_passed, 5);
        assert_eq!(summary.tests_failed, 0);
        assert_eq!(summary.ignored, 2);
        assert_eq!(summary.measured, 0);
        assert_eq!(summary.filtered_out, 0);
        assert_eq!(summary.tests_run, 7); // 5 + 0 + 2
    }

    #[test]
    fn test_parse_test_output_with_failures() {
        let output =
            "test result: FAILED. 8 passed; 2 failed; 1 ignored; 0 measured; 0 filtered out";
        let summary = parse_test_output(output);

        assert_eq!(summary.tests_passed, 8);
        assert_eq!(summary.tests_failed, 2);
        assert_eq!(summary.ignored, 1);
        assert_eq!(summary.tests_run, 11); // 8 + 2 + 1
    }

    #[test]
    fn test_parse_test_output_no_summary() {
        let output = "running 5 tests\ntest test1 ... ok\ntest test2 ... ok";
        let summary = parse_test_output(output);

        // Should return all zeros if no test result line
        assert_eq!(summary.tests_run, 0);
        assert_eq!(summary.tests_passed, 0);
        assert_eq!(summary.tests_failed, 0);
    }

    #[test]
    fn test_test_summary_default() {
        let summary = TestSummary::default();

        assert_eq!(summary.tests_run, 0);
        assert_eq!(summary.tests_passed, 0);
        assert_eq!(summary.tests_failed, 0);
        assert_eq!(summary.ignored, 0);
        assert_eq!(summary.measured, 0);
        assert_eq!(summary.filtered_out, 0);
    }
}
