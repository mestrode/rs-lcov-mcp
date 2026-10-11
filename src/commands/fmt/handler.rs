//! Cargo fmt command handler
//!
//! This module provides the handler function for the cargo fmt command.

use crate::commands::utils::{create_bool_property, create_string_property};
use crate::error::CommandResult;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::process::Command;

/// Parameters for the cargo fmt tool
#[derive(Debug, Deserialize, Clone)]
pub struct FmtParams {
    /// Run in check mode (no changes, just check if formatting is needed)
    #[serde(default)]
    pub check: bool,
    /// Show verbose output
    #[serde(default)]
    pub verbose: bool,
    /// Specific package to format
    pub package: Option<String>,
    /// Working directory for cargo command
    pub working_dir: Option<String>,
}

/// A single diagnostic message from cargo fmt
#[derive(Debug, Serialize, Clone)]
pub struct FmtDiagnostic {
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

/// Result of the cargo fmt tool
#[derive(Debug, Serialize, Clone)]
pub struct FmtResult {
    /// Whether the command executed successfully
    pub success: bool,
    /// Whether any files were formatted (or would be formatted in check mode)
    pub formatted: bool,
    /// Structured diagnostics (errors and warnings)
    pub diagnostics: Vec<FmtDiagnostic>,
    /// Error message if the command failed
    pub error: Option<String>,
}

/// Handler for cargo fmt command
pub async fn handle_fmt(params: FmtParams) -> CommandResult<FmtResult> {
    let mut cmd = Command::new("cargo");

    if params.check {
        cmd.arg("fmt").arg("--check");
    } else {
        cmd.arg("fmt");
    }

    if params.verbose {
        cmd.arg("--verbose");
    }

    if let Some(pkg) = params.package {
        cmd.args(["--package", &pkg]);
    }

    if let Some(ref dir) = params.working_dir {
        cmd.current_dir(dir);
    }

    cmd.arg("--message-format=json");

    let output = cmd.output().map_err(|e| {
        crate::error::CommandError::ExecutionFailed(format!("Failed to execute cargo fmt: {}", e))
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let _stderr = String::from_utf8_lossy(&output.stderr);

    // Parse JSON output for diagnostics
    let diagnostics = parse_fmt_output_json(&stdout);

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

    // Check if any files were formatted (non-check mode)
    // With --message-format=json, fmt outputs JSON for each file changed
    let formatted = if !params.check && !diagnostics.is_empty() {
        true
    } else if params.check && diagnostics.is_empty() {
        // In check mode, no diagnostics means no changes needed
        false
    } else {
        // In check mode, diagnostics mean files need formatting
        true
    };

    if !output.status.success() {
        return Ok(FmtResult {
            success: false,
            formatted: false,
            diagnostics,
            error: Some(format!(
                "cargo fmt failed with {} errors and {} warnings",
                errors, warnings
            )),
        });
    }

    Ok(FmtResult {
        success: true,
        formatted,
        diagnostics,
        error: None,
    })
}

/// Parse cargo fmt JSON output and extract diagnostics
fn parse_fmt_output_json(stdout: &str) -> Vec<FmtDiagnostic> {
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

/// Parse a compiler-message JSON object into a FmtDiagnostic
fn parse_compiler_message(message: &serde_json::Value) -> Option<FmtDiagnostic> {
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

    Some(FmtDiagnostic {
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

/// Create JSON schema for cargo fmt tool
pub fn create_schema() -> serde_json::Map<String, serde_json::Value> {
    let mut properties = serde_json::Map::new();

    properties.insert(
        "check".to_string(),
        serde_json::Value::Object(create_bool_property(
            "Run in check mode (no changes, just verify formatting)",
        )),
    );

    properties.insert(
        "verbose".to_string(),
        serde_json::Value::Object(create_bool_property("Show verbose output")),
    );

    properties.insert(
        "package".to_string(),
        serde_json::Value::Object(create_string_property("Specific package to format")),
    );

    properties.insert(
        "working_dir".to_string(),
        serde_json::Value::Object(create_string_property(
            "Working directory for cargo command",
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
            "Run cargo fmt to format Rust code according to rustfmt rules. Returns whether files were formatted and any output.".to_string(),
        ),
    );

    schema
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_fmt_params_deserialization() {
        let json = json!({
            "check": true,
            "verbose": false,
            "package": "my-package",
            "working_dir": "/path/to/dir"
        });

        let params: FmtParams = serde_json::from_value(json).unwrap();
        assert!(params.check);
        assert!(!params.verbose);
        assert_eq!(params.package, Some("my-package".to_string()));
        assert_eq!(params.working_dir, Some("/path/to/dir".to_string()));
    }

    #[test]
    fn test_fmt_params_default() {
        let json = json!({});

        let params: FmtParams = serde_json::from_value(json).unwrap();
        assert!(!params.check);
        assert!(!params.verbose);
        assert!(params.package.is_none());
        assert!(params.working_dir.is_none());
    }

    #[test]
    fn test_fmt_result_serialization() {
        let result = FmtResult {
            success: true,
            formatted: true,
            diagnostics: vec![],
            error: None,
        };

        let json = serde_json::to_value(&result).unwrap();
        assert!(json["success"].as_bool().unwrap());
        assert!(json["formatted"].as_bool().unwrap());
        assert!(json["diagnostics"].as_array().unwrap().is_empty());
        assert!(json["error"].is_null());
    }

    #[test]
    fn test_fmt_result_error_serialization() {
        let result = FmtResult {
            success: false,
            formatted: false,
            diagnostics: vec![],
            error: Some("cargo fmt failed".to_string()),
        };

        let json = serde_json::to_value(&result).unwrap();
        assert!(!json["success"].as_bool().unwrap());
        assert!(!json["formatted"].as_bool().unwrap());
        assert_eq!(json["error"], "cargo fmt failed");
    }

    #[test]
    fn test_create_schema() {
        let schema = create_schema();

        assert!(schema.contains_key("type"));
        assert!(schema.contains_key("properties"));
        assert!(schema.contains_key("description"));

        let properties = schema["properties"].as_object().unwrap();
        assert!(properties.contains_key("check"));
        assert!(properties.contains_key("verbose"));
        assert!(properties.contains_key("package"));
    }

    #[test]
    fn test_schema_description() {
        let schema = create_schema();
        let description = schema["description"].as_str().unwrap();

        assert!(!description.is_empty());
        assert!(description.contains("cargo fmt"));
        assert!(description.contains("rustfmt"));
    }

    #[test]
    fn test_schema_property_types() {
        let schema = create_schema();
        let properties = schema["properties"].as_object().unwrap();

        // Check that check and verbose are boolean
        let check_prop = properties["check"].as_object().unwrap();
        assert_eq!(check_prop["type"], "boolean");

        let verbose_prop = properties["verbose"].as_object().unwrap();
        assert_eq!(verbose_prop["type"], "boolean");

        // Check that package is string
        let package_prop = properties["package"].as_object().unwrap();
        assert_eq!(package_prop["type"], "string");
    }
}
