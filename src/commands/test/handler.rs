//! Cargo test command handler
//!
//! This module provides the handler function for the cargo test command.

use crate::commands::utils::{
    create_array_property, create_bool_property, create_number_property, create_string_property,
};
use crate::error::CommandResult;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::process::Command;

/// Parameters for the cargo test tool
#[derive(Debug, Deserialize, Clone)]
pub struct TestParams {
    /// Specific package to test
    pub package: Option<String>,
    /// Test entire workspace
    #[serde(default)]
    pub workspace: bool,
    /// Test all targets
    #[serde(default)]
    pub all_targets: bool,
    /// Build with all features enabled
    #[serde(default)]
    pub all_features: bool,
    /// Build without default features
    #[serde(default)]
    pub no_default_features: bool,
    /// List of features to enable
    pub features: Option<Vec<String>>,
    /// Don't run tests, just compile them
    #[serde(default)]
    pub no_run: bool,
    /// Suppress output
    #[serde(default)]
    pub quiet: bool,
    /// Number of test threads to use
    pub test_threads: Option<usize>,
    /// Name of specific test to run
    pub test_name: Option<String>,
}

/// A single diagnostic message from cargo test
#[derive(Debug, Serialize, Clone)]
pub struct TestDiagnostic {
    /// Severity level: "error", "warning", "note", "help"
    pub level: String,
    /// Error code (e.g., "E0583")
    pub code: Option<String>,
    /// Short message
    pub message: String,
    /// File path where the diagnostic occurred
    pub file: Option<String>,
    /// Line number
    pub line: Option<u32>,
    /// Column number
    pub column: Option<u32>,
    /// Full rendered message
    pub rendered: String,
    /// Suggested replacement text
    pub suggested_replacement: Option<String>,
}

/// Result of the cargo test tool
#[derive(Debug, Serialize, Clone)]
pub struct TestResult {
    /// Whether the command executed successfully
    pub success: bool,
    /// Number of tests run
    pub tests_run: u32,
    /// Number of tests passed
    pub tests_passed: u32,
    /// Number of tests failed
    pub tests_failed: u32,
    /// Number of tests ignored
    pub ignored: u32,
    /// Number of tests measured (benchmarks)
    pub measured: u32,
    /// Number of tests filtered out
    pub filtered_out: u32,
    /// Structured diagnostics (errors and warnings)
    pub diagnostics: Vec<TestDiagnostic>,
    /// Error message if the command failed
    pub error: Option<String>,
}

/// Handler for cargo test command
pub async fn handle_test(params: TestParams) -> CommandResult<TestResult> {
    let mut cmd = Command::new("cargo");
    cmd.arg("test");

    if params.workspace {
        cmd.arg("--workspace");
    }

    if let Some(pkg) = params.package {
        cmd.args(["--package", &pkg]);
    }

    if params.all_targets {
        cmd.arg("--all-targets");
    }

    if params.all_features {
        cmd.arg("--all-features");
    }

    if params.no_default_features {
        cmd.arg("--no-default-features");
    }

    if let Some(features) = params.features
        && !features.is_empty()
    {
        cmd.arg("--features");
        cmd.arg(features.join(","));
    }

    if params.no_run {
        cmd.arg("--no-run");
    }

    if params.quiet {
        cmd.arg("--quiet");
    }

    if let Some(threads) = params.test_threads {
        cmd.args(["--test-threads", &threads.to_string()]);
    }

    if let Some(test_name) = params.test_name {
        cmd.args(["--test", &test_name]);
    }

    cmd.arg("--message-format=json");

    let output = cmd.output().map_err(|e| {
        crate::error::CommandError::ExecutionFailed(format!("Failed to execute cargo test: {}", e))
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let _stderr = String::from_utf8_lossy(&output.stderr);

    // Parse JSON output for diagnostics (compiler messages go to stdout with --message-format=json)
    let diagnostics = parse_test_output_json(&stdout);

    // Count errors and warnings from diagnostics
    let errors = diagnostics.iter().filter(|d| d.level == "error").count() as u32;
    let warnings = diagnostics.iter().filter(|d| d.level == "warning").count() as u32;

    // Sort diagnostics: errors first, then warnings, then others
    let mut diagnostics = diagnostics;
    diagnostics.sort_by(|a, b| {
        let a_order = match a.level.as_str() {
            "error" => 0,
            "warning" => 1,
            "note" => 2,
            "help" => 3,
            _ => 4,
        };
        let b_order = match b.level.as_str() {
            "error" => 0,
            "warning" => 1,
            "note" => 2,
            "help" => 3,
            _ => 4,
        };
        a_order.cmp(&b_order)
    });

    // Parse test summary from stdout (with --message-format=json, test results go to stdout)
    let test_summary = crate::commands::utils::parse_test_output(&stdout);

    if !output.status.success() {
        return Ok(TestResult {
            success: false,
            tests_run: test_summary.tests_run,
            tests_passed: test_summary.tests_passed,
            tests_failed: test_summary.tests_failed,
            ignored: test_summary.ignored,
            measured: test_summary.measured,
            filtered_out: test_summary.filtered_out,
            diagnostics,
            error: Some(format!(
                "cargo test failed with {} errors and {} warnings",
                errors, warnings
            )),
        });
    }

    Ok(TestResult {
        success: true,
        tests_run: test_summary.tests_run,
        tests_passed: test_summary.tests_passed,
        tests_failed: test_summary.tests_failed,
        ignored: test_summary.ignored,
        measured: test_summary.measured,
        filtered_out: test_summary.filtered_out,
        diagnostics,
        error: None,
    })
}

/// Create JSON schema for cargo test tool
pub fn create_schema() -> serde_json::Map<String, serde_json::Value> {
    let mut properties = serde_json::Map::new();

    properties.insert(
        "package".to_string(),
        serde_json::Value::Object(create_string_property("Specific package to test")),
    );

    properties.insert(
        "workspace".to_string(),
        serde_json::Value::Object(create_bool_property("Test entire workspace")),
    );

    properties.insert(
        "all_targets".to_string(),
        serde_json::Value::Object(create_bool_property("Test all targets")),
    );

    properties.insert(
        "all_features".to_string(),
        serde_json::Value::Object(create_bool_property("Build with all features enabled")),
    );

    properties.insert(
        "no_default_features".to_string(),
        serde_json::Value::Object(create_bool_property("Build without default features")),
    );

    properties.insert(
        "features".to_string(),
        serde_json::Value::Object(create_array_property(
            "string",
            "List of features to enable",
        )),
    );

    properties.insert(
        "no_run".to_string(),
        serde_json::Value::Object(create_bool_property("Don't run tests, just compile them")),
    );

    properties.insert(
        "quiet".to_string(),
        serde_json::Value::Object(create_bool_property("Suppress output")),
    );

    properties.insert(
        "test_threads".to_string(),
        serde_json::Value::Object(create_number_property("Number of test threads to use")),
    );

    properties.insert(
        "test_name".to_string(),
        serde_json::Value::Object(create_string_property("Name of specific test to run")),
    );

    let mut schema = serde_json::Map::new();
    schema.insert(
        "type".to_string(),
        serde_json::Value::String("object".to_string()),
    );
    schema.insert(
        "properties".to_string(),
        serde_json::Value::Object(properties),
    );
    schema.insert(
        "description".to_string(),
        serde_json::Value::String(
            "Run cargo test to execute unit and integration tests. Returns detailed test results including passed, failed, ignored, and filtered counts.".to_string(),
        ),
    );

    schema
}

/// Parse cargo test JSON output and extract diagnostics
fn parse_test_output_json(stdout: &str) -> Vec<TestDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen: HashSet<(String, Option<u32>, Option<String>)> = HashSet::new();

    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(message) = serde_json::from_str::<serde_json::Value>(line)
            && let Some(reason) = message.get("reason").and_then(|v| v.as_str())
            && reason == "compiler-message"
            && let Some(diag) = parse_compiler_message(&message)
        {
            let key = (
                diag.file.clone().unwrap_or_default(),
                diag.line,
                diag.code.clone(),
            );
            if seen.insert(key) {
                diagnostics.push(diag);
            }
        }
    }

    diagnostics
}

/// Parse a compiler-message JSON object into a TestDiagnostic
fn parse_compiler_message(message: &serde_json::Value) -> Option<TestDiagnostic> {
    let message_obj = message.get("message")?.as_object()?;

    let level = message_obj.get("level")?.as_str()?.to_string();
    let code = message_obj.get("code")?.as_str().map(String::from);
    let message_text = message_obj.get("message")?.as_str().map(String::from);
    let rendered = message_obj
        .get("rendered")
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| message_text.clone());

    let (file, line, column, suggested_replacement) = extract_spans(message);

    Some(TestDiagnostic {
        level,
        code,
        message: message_text.unwrap_or_default(),
        file,
        line,
        column,
        rendered: rendered.unwrap_or_default(),
        suggested_replacement,
    })
}

/// Extract file, line, column, and suggested replacement from message spans
fn extract_spans(
    message: &serde_json::Value,
) -> (Option<String>, Option<u32>, Option<u32>, Option<String>) {
    let spans = message.get("message").and_then(|m| m.get("spans"));

    if let Some(spans_array) = spans.and_then(|s| s.as_array()) {
        for span in spans_array {
            if let Some(span_obj) = span.as_object() {
                let is_primary = span_obj
                    .get("is_primary")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if is_primary {
                    let file_name = span_obj
                        .get("file_name")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    let line_start = span_obj
                        .get("line_start")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as u32);
                    let column_start = span_obj
                        .get("column_start")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as u32);

                    let suggested_replacement = span_obj
                        .get("suggested_replacement")
                        .and_then(|v| v.as_str())
                        .map(String::from);

                    return (file_name, line_start, column_start, suggested_replacement);
                }
            }
        }
    }

    // Fallback: try to get from target
    let target = message.get("target").and_then(|t| t.as_object());
    if let Some(target_obj) = target {
        let file_path = target_obj
            .get("src_path")
            .and_then(|v| v.as_str())
            .map(String::from);
        return (file_path, None, None, None);
    }

    (None, None, None, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_test_params_deserialization() {
        let json = json!({
            "package": "my-package",
            "workspace": true,
            "all_targets": true,
            "all_features": false,
            "no_default_features": true,
            "features": ["feat1", "feat2"],
            "no_run": false,
            "quiet": true,
            "test_threads": 4,
            "test_name": "my_test"
        });

        let params: TestParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.package, Some("my-package".to_string()));
        assert!(params.workspace);
        assert!(params.all_targets);
        assert!(!params.all_features);
        assert!(params.no_default_features);
        assert_eq!(
            params.features,
            Some(vec!["feat1".to_string(), "feat2".to_string()])
        );
        assert!(!params.no_run);
        assert!(params.quiet);
        assert_eq!(params.test_threads, Some(4));
        assert_eq!(params.test_name, Some("my_test".to_string()));
    }

    #[test]
    fn test_test_params_default() {
        let json = json!({});

        let params: TestParams = serde_json::from_value(json).unwrap();
        assert!(params.package.is_none());
        assert!(!params.workspace);
        assert!(!params.all_targets);
        assert!(!params.all_features);
        assert!(!params.no_default_features);
        assert!(params.features.is_none());
        assert!(!params.no_run);
        assert!(!params.quiet);
        assert!(params.test_threads.is_none());
        assert!(params.test_name.is_none());
    }

    #[test]
    fn test_test_result_serialization() {
        let result = TestResult {
            success: true,
            tests_run: 10,
            tests_passed: 8,
            tests_failed: 2,
            ignored: 1,
            measured: 0,
            filtered_out: 0,
            diagnostics: vec![],
            error: None,
        };

        let json = serde_json::to_value(&result).unwrap();
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["tests_run"], 10);
        assert_eq!(json["tests_passed"], 8);
        assert_eq!(json["tests_failed"], 2);
        assert_eq!(json["ignored"], 1);
        assert!(json["diagnostics"].as_array().unwrap().is_empty());
        assert!(json["error"].is_null());
    }

    #[test]
    fn test_test_result_error_serialization() {
        let result = TestResult {
            success: false,
            tests_run: 5,
            tests_passed: 3,
            tests_failed: 2,
            ignored: 0,
            measured: 0,
            filtered_out: 0,
            diagnostics: vec![],
            error: Some("cargo test failed".to_string()),
        };

        let json = serde_json::to_value(&result).unwrap();
        assert!(!json["success"].as_bool().unwrap());
        assert_eq!(json["tests_run"], 5);
        assert_eq!(json["error"], "cargo test failed");
    }

    #[test]
    fn test_create_schema() {
        let schema = create_schema();

        assert!(schema.contains_key("type"));
        assert!(schema.contains_key("properties"));
        assert!(schema.contains_key("description"));

        let properties = schema["properties"].as_object().unwrap();
        assert!(properties.contains_key("package"));
        assert!(properties.contains_key("workspace"));
        assert!(properties.contains_key("all_targets"));
        assert!(properties.contains_key("all_features"));
        assert!(properties.contains_key("no_default_features"));
        assert!(properties.contains_key("features"));
        assert!(properties.contains_key("no_run"));
        assert!(properties.contains_key("quiet"));
        assert!(properties.contains_key("test_threads"));
        assert!(properties.contains_key("test_name"));
    }

    #[test]
    fn test_schema_description() {
        let schema = create_schema();
        let description = schema["description"].as_str().unwrap();

        assert!(!description.is_empty());
        assert!(description.contains("cargo test"));
        assert!(description.contains("unit and integration"));
    }

    #[test]
    fn test_schema_property_types() {
        let schema = create_schema();
        let properties = schema["properties"].as_object().unwrap();

        // Check that boolean properties are boolean
        let workspace_prop = properties["workspace"].as_object().unwrap();
        assert_eq!(workspace_prop["type"], "boolean");

        let all_targets_prop = properties["all_targets"].as_object().unwrap();
        assert_eq!(all_targets_prop["type"], "boolean");

        // Check that string properties are string
        let package_prop = properties["package"].as_object().unwrap();
        assert_eq!(package_prop["type"], "string");

        let test_name_prop = properties["test_name"].as_object().unwrap();
        assert_eq!(test_name_prop["type"], "string");

        // Check that number property is number
        let test_threads_prop = properties["test_threads"].as_object().unwrap();
        assert_eq!(test_threads_prop["type"], "number");

        // Check that array property is array
        let features_prop = properties["features"].as_object().unwrap();
        assert_eq!(features_prop["type"], "array");
    }
}
