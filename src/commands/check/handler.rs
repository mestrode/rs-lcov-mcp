//! Cargo check command handler
//!
//! This module provides the handler function for the cargo check command.

use crate::commands::utils::{create_array_property, create_bool_property, create_string_property};
use crate::error::CommandResult;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::process::Command;

/// Parameters for the cargo check tool
#[derive(Debug, Deserialize, Clone)]
pub struct CheckParams {
    /// Specific package to check
    pub package: Option<String>,
    /// Check entire workspace
    #[serde(default)]
    pub workspace: bool,
    /// Check all targets (including tests, examples, etc.)
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
}

/// A single diagnostic message from cargo check
#[derive(Debug, Serialize, Clone)]
pub struct CheckDiagnostic {
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

/// Result of the cargo check tool
#[derive(Debug, Serialize, Clone)]
pub struct CheckResult {
    /// Whether the compilation succeeded
    pub success: bool,
    /// Number of warnings found
    pub warnings: u32,
    /// Number of errors found
    pub errors: u32,
    /// Structured diagnostics (errors and warnings)
    pub diagnostics: Vec<CheckDiagnostic>,
    /// Raw command output (for backwards compatibility)
    pub output: Option<String>,
    /// Error message if the command failed
    pub error: Option<String>,
}

/// Handler for cargo check command
pub async fn handle_check(params: CheckParams) -> CommandResult<CheckResult> {
    let mut cmd = Command::new("cargo");
    cmd.arg("check");
    cmd.arg("--message-format=json");

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

    let output = cmd.output().map_err(|e| {
        crate::error::CommandError::ExecutionFailed(format!("Failed to execute cargo check: {}", e))
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Parse JSON output from stdout only
    let diagnostics = parse_check_output(&stdout);

    // Count errors and warnings
    let errors = diagnostics.iter().filter(|d| d.level == "error").count() as u32;
    let warnings = diagnostics.iter().filter(|d| d.level == "warning").count() as u32;

    // Sort diagnostics: errors first, then warnings, then others
    let mut diagnostics = diagnostics;
    diagnostics.sort_by(|a, b| {
        // Order: error > warning > note > help > other
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

    let success = output.status.success();

    if !success {
        return Ok(CheckResult {
            success: false,
            warnings,
            errors,
            diagnostics,
            output: Some(stderr.into_owned()),
            error: Some(format!(
                "cargo check failed with {} errors and {} warnings",
                errors, warnings
            )),
        });
    }

    Ok(CheckResult {
        success: true,
        warnings,
        errors,
        diagnostics,
        output: Some(stdout.into_owned()),
        error: None,
    })
}

/// Parse cargo check JSON output and extract diagnostics
fn parse_check_output(stdout: &str) -> Vec<CheckDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = HashSet::new();

    for line in stdout.lines() {
        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }

        // Try to parse as JSON
        if let Ok(message) = serde_json::from_str::<serde_json::Value>(line)
            && let Some(reason) = message.get("reason").and_then(|v| v.as_str())
        {
            match reason {
                "compiler-message" => {
                    if let Some(diag) = parse_compiler_message(&message) {
                        // Create a unique key for deduplication
                        let key = format!(
                            "{:?}-{:?}-{:?}",
                            diag.file.as_deref(),
                            diag.line,
                            diag.code.as_deref()
                        );
                        if seen.insert(key) {
                            diagnostics.push(diag);
                        }
                    }
                }
                "build-finished" => {
                    // We can extract success status from build-finished if needed
                    // For now, we rely on the exit code
                }
                _ => {
                    // Ignore other message types
                }
            }
        }
        // Silently ignore lines that aren't valid JSON (JSONC comments, etc.)
    }

    diagnostics
}

/// Parse a compiler-message JSON object into a CheckDiagnostic
fn parse_compiler_message(message: &serde_json::Value) -> Option<CheckDiagnostic> {
    let message_obj = message.get("message")?.as_object()?;

    let level = message_obj.get("level")?.as_str()?.to_string();
    let code = message_obj.get("code")?.as_str().map(String::from);
    let message_text = message_obj.get("message")?.as_str().map(String::from);
    let rendered = message_obj
        .get("rendered")
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| message_text.clone());

    // Extract file and line from spans
    let (file, line, column, suggested_replacement) = extract_spans(message);

    Some(CheckDiagnostic {
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
        // Get the first primary span
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

                    // Try to get suggested replacement
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

/// Create JSON schema for cargo check tool
pub fn create_schema() -> serde_json::Map<String, serde_json::Value> {
    let mut properties = serde_json::Map::new();

    properties.insert(
        "package".to_string(),
        serde_json::Value::Object(create_string_property("Specific package to check")),
    );

    properties.insert(
        "workspace".to_string(),
        serde_json::Value::Object(create_bool_property("Check entire workspace")),
    );

    properties.insert(
        "all_targets".to_string(),
        serde_json::Value::Object(create_bool_property(
            "Check all targets (including tests, examples, etc.)",
        )),
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
            "Run cargo check to verify code compiles without running tests. Returns structured diagnostics with errors, warnings, and compilation details.".to_string(),
        ),
    );

    schema
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_check_params_deserialization() {
        let json = json!({
            "package": "my-package",
            "workspace": true,
            "all_targets": true,
            "all_features": false,
            "no_default_features": true,
            "features": ["feat1", "feat2"]
        });

        let params: CheckParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.package, Some("my-package".to_string()));
        assert!(params.workspace);
        assert!(params.all_targets);
        assert!(!params.all_features);
        assert!(params.no_default_features);
        assert_eq!(
            params.features,
            Some(vec!["feat1".to_string(), "feat2".to_string()])
        );
    }

    #[test]
    fn test_check_params_default() {
        let json = json!({});

        let params: CheckParams = serde_json::from_value(json).unwrap();
        assert!(params.package.is_none());
        assert!(!params.workspace);
        assert!(!params.all_targets);
        assert!(!params.all_features);
        assert!(!params.no_default_features);
        assert!(params.features.is_none());
    }

    #[test]
    fn test_check_result_serialization() {
        let result = CheckResult {
            success: true,
            warnings: 2,
            errors: 0,
            diagnostics: vec![],
            output: Some("Compilation successful".to_string()),
            error: None,
        };

        let json = serde_json::to_value(&result).unwrap();
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["warnings"], 2);
        assert_eq!(json["errors"], 0);
        assert_eq!(json["output"], "Compilation successful");
        assert!(json["error"].is_null());
        assert!(json["diagnostics"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_check_result_with_diagnostics() {
        let diagnostics = vec![
            CheckDiagnostic {
                level: "error".to_string(),
                code: Some("E0583".to_string()),
                message: "file not found for module".to_string(),
                file: Some("src/error/mod.rs".to_string()),
                line: Some(70),
                column: Some(1),
                rendered: "error[E0583]: file not found for module `respons`".to_string(),
                suggested_replacement: None,
            },
            CheckDiagnostic {
                level: "warning".to_string(),
                code: Some("E0432".to_string()),
                message: "unresolved import".to_string(),
                file: Some("src/error/mod.rs".to_string()),
                line: Some(72),
                column: Some(9),
                rendered: "warning[E0432]: unresolved import".to_string(),
                suggested_replacement: Some("respons".to_string()),
            },
        ];

        let result = CheckResult {
            success: false,
            warnings: 1,
            errors: 1,
            diagnostics,
            output: Some("Compilation failed".to_string()),
            error: Some("cargo check failed with 1 errors and 1 warnings".to_string()),
        };

        let json = serde_json::to_value(&result).unwrap();
        assert!(!json["success"].as_bool().unwrap());
        assert_eq!(json["warnings"], 1);
        assert_eq!(json["errors"], 1);

        let diags = json["diagnostics"].as_array().unwrap();
        assert_eq!(diags.len(), 2);

        // Check first diagnostic (should be error due to sorting)
        assert_eq!(diags[0]["level"], "error");
        assert_eq!(diags[0]["code"], "E0583");

        // Check second diagnostic (should be warning)
        assert_eq!(diags[1]["level"], "warning");
        assert_eq!(diags[1]["code"], "E0432");
    }

    #[test]
    fn test_check_result_error_serialization() {
        let result = CheckResult {
            success: false,
            warnings: 5,
            errors: 2,
            diagnostics: vec![],
            output: Some("Compilation failed".to_string()),
            error: Some("cargo check failed with 2 errors and 5 warnings".to_string()),
        };

        let json = serde_json::to_value(&result).unwrap();
        assert!(!json["success"].as_bool().unwrap());
        assert_eq!(json["warnings"], 5);
        assert_eq!(json["errors"], 2);
        assert_eq!(
            json["error"],
            "cargo check failed with 2 errors and 5 warnings"
        );
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
    }

    #[test]
    fn test_schema_description() {
        let schema = create_schema();
        let description = schema["description"].as_str().unwrap();

        assert!(!description.is_empty());
        assert!(description.contains("cargo check"));
        assert!(description.contains("compiles"));
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

        // Check that package is string
        let package_prop = properties["package"].as_object().unwrap();
        assert_eq!(package_prop["type"], "string");

        // Check that features is array
        let features_prop = properties["features"].as_object().unwrap();
        assert_eq!(features_prop["type"], "array");
    }

    #[test]
    fn test_parse_compiler_message() {
        let message = json!({
            "reason": "compiler-message",
            "package_id": "path+file:///Users/test/project#0.1.0",
            "manifest_path": "/Users/test/project/Cargo.toml",
            "target": {
                "kind": ["lib"],
                "crate_types": ["lib"],
                "name": "test",
                "src_path": "/Users/test/project/src/lib.rs",
                "edition": "2021"
            },
            "message": {
                "rendered": "error[E0583]: file not found for module `respons`\n  --> src/error/mod.rs:70:1\n   |\n70 | mod respons;\n   | ^^^^^^^^^^^^\n",
                "level": "error",
                "code": "E0583",
                "message": "file not found for module `respons`",
                "spans": [{
                    "is_primary": true,
                    "file_name": "src/error/mod.rs",
                    "line_start": 70,
                    "line_end": 70,
                    "column_start": 1,
                    "column_end": 13,
                    "suggested_replacement": null
                }]
            }
        });

        let diagnostic = parse_compiler_message(&message).unwrap();

        assert_eq!(diagnostic.level, "error");
        assert_eq!(diagnostic.code, Some("E0583".to_string()));
        assert_eq!(diagnostic.message, "file not found for module `respons`");
        assert_eq!(diagnostic.file, Some("src/error/mod.rs".to_string()));
        assert_eq!(diagnostic.line, Some(70));
        assert_eq!(diagnostic.column, Some(1));
        assert!(diagnostic.rendered.contains("error[E0583]"));
    }

    #[test]
    fn test_parse_check_output() {
        // Create valid JSON lines
        let line1 = json!({
            "reason": "compiler-message",
            "message": {
                "level": "error",
                "code": "E0583",
                "message": "file not found",
                "spans": [{
                    "is_primary": true,
                    "file_name": "src/lib.rs",
                    "line_start": 10,
                    "column_start": 5
                }]
            }
        })
        .to_string();

        let line2 = json!({
            "reason": "compiler-message",
            "message": {
                "level": "warning",
                "code": "E0432",
                "message": "unresolved import",
                "spans": [{
                    "is_primary": true,
                    "file_name": "src/lib.rs",
                    "line_start": 15,
                    "column_start": 10
                }]
            }
        })
        .to_string();

        let line3 = json!({
            "reason": "build-finished",
            "success": false
        })
        .to_string();

        let output = format!("{}\n{}\n{}", line1, line2, line3);

        let diagnostics = parse_check_output(&output);

        assert_eq!(diagnostics.len(), 2);

        // Check that we have one error and one warning
        let error_count = diagnostics.iter().filter(|d| d.level == "error").count();
        let warning_count = diagnostics.iter().filter(|d| d.level == "warning").count();

        assert_eq!(error_count, 1);
        assert_eq!(warning_count, 1);
    }

    #[test]
    fn test_deduplication() {
        // Same diagnostic twice
        let line = json!({
            "reason": "compiler-message",
            "message": {
                "level": "error",
                "code": "E0583",
                "message": "file not found",
                "spans": [{
                    "is_primary": true,
                    "file_name": "src/lib.rs",
                    "line_start": 10,
                    "column_start": 5
                }]
            }
        })
        .to_string();

        let output = format!("{}\n{}", line, line);

        let diagnostics = parse_check_output(&output);

        // Should only have one diagnostic due to deduplication
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_diagnostic_sorting() {
        let diagnostics = vec![
            CheckDiagnostic {
                level: "warning".to_string(),
                code: Some("W0001".to_string()),
                message: "warning message".to_string(),
                file: None,
                line: None,
                column: None,
                rendered: "warning".to_string(),
                suggested_replacement: None,
            },
            CheckDiagnostic {
                level: "error".to_string(),
                code: Some("E0001".to_string()),
                message: "error message".to_string(),
                file: None,
                line: None,
                column: None,
                rendered: "error".to_string(),
                suggested_replacement: None,
            },
            CheckDiagnostic {
                level: "note".to_string(),
                code: None,
                message: "note message".to_string(),
                file: None,
                line: None,
                column: None,
                rendered: "note".to_string(),
                suggested_replacement: None,
            },
        ];

        let mut diagnostics = diagnostics;
        // Apply the same sorting logic as in handle_check
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

        // Check order: error, warning, note
        assert_eq!(diagnostics[0].level, "error");
        assert_eq!(diagnostics[1].level, "warning");
        assert_eq!(diagnostics[2].level, "note");
    }

    #[test]
    fn test_extract_spans() {
        let message = json!({
            "reason": "compiler-message",
            "message": {
                "level": "error",
                "code": "E0583",
                "message": "test",
                "spans": [{
                    "is_primary": true,
                    "file_name": "src/lib.rs",
                    "line_start": 10,
                    "column_start": 5,
                    "suggested_replacement": "corrected"
                }]
            }
        });

        let (file, line, column, suggested) = extract_spans(&message);

        assert_eq!(file, Some("src/lib.rs".to_string()));
        assert_eq!(line, Some(10));
        assert_eq!(column, Some(5));
        assert_eq!(suggested, Some("corrected".to_string()));
    }

    #[test]
    fn test_extract_spans_fallback_to_target() {
        let message = json!({
            "reason": "compiler-message",
            "target": {
                "src_path": "/path/to/src/lib.rs"
            },
            "message": {
                "level": "error",
                "code": "E0583",
                "message": "test"
            }
        });

        let (file, line, column, suggested) = extract_spans(&message);

        assert_eq!(file, Some("/path/to/src/lib.rs".to_string()));
        assert_eq!(line, None);
        assert_eq!(column, None);
        assert_eq!(suggested, None);
    }
}
