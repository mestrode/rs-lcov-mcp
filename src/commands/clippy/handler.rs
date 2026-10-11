//! Cargo clippy command handler
//!
//! This module provides the handler function for the cargo clippy command.

use crate::commands::utils::{create_array_property, create_bool_property, create_string_property};
use crate::error::CommandResult;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::process::Command;

/// Parameters for the cargo clippy tool
#[derive(Debug, Deserialize, Clone)]
pub struct ClippyParams {
    /// Specific package to lint
    pub package: Option<String>,
    /// Lint entire workspace
    #[serde(default)]
    pub workspace: bool,
    /// Lint all targets (including tests, examples, etc.)
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
    /// Treat warnings as errors
    #[serde(default)]
    pub warnings_as_errors: bool,
    /// List of lints to enable or configure
    pub lints: Option<Vec<String>>,
    /// Working directory for cargo command
    pub working_dir: Option<String>,
}

/// A single diagnostic message from cargo clippy
#[derive(Debug, Serialize, Clone)]
pub struct ClippyDiagnostic {
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

/// Result of the cargo clippy tool
#[derive(Debug, Serialize, Clone)]
pub struct ClippyResult {
    /// Whether the command executed successfully
    pub success: bool,
    /// Number of warnings found
    pub warnings: u32,
    /// Number of errors found
    pub errors: u32,
    /// List of lint names that were found
    pub lints_found: Vec<String>,
    /// Structured diagnostics (errors and warnings)
    pub diagnostics: Vec<ClippyDiagnostic>,
    /// Error message if the command failed
    pub error: Option<String>,
}

/// Handler for cargo clippy command
pub async fn handle_clippy(params: ClippyParams) -> CommandResult<ClippyResult> {
    let mut cmd = Command::new("cargo");
    cmd.arg("clippy");

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

    if params.warnings_as_errors {
        cmd.arg("--");
        cmd.arg("-D");
        cmd.arg("warnings");
    }

    if let Some(lints) = params.lints
        && !lints.is_empty()
    {
        cmd.arg("--");
        cmd.arg("-W");
        cmd.arg(lints.join(","));
    }

    if let Some(ref dir) = params.working_dir {
        cmd.current_dir(dir);
    }

    cmd.arg("--message-format=json");

    let output = cmd.output().map_err(|e| {
        crate::error::CommandError::ExecutionFailed(format!(
            "Failed to execute cargo clippy: {}",
            e
        ))
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Parse JSON output for diagnostics
    let diagnostics = parse_clippy_output_json(&stdout);

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

    // Parse lints from stderr (clippy-specific output)
    let (_, _, lints_found) = parse_clippy_output(&stderr);

    if !output.status.success() {
        return Ok(ClippyResult {
            success: false,
            warnings,
            errors,
            lints_found,
            diagnostics,
            error: Some(format!(
                "cargo clippy failed with {} errors and {} warnings",
                errors, warnings
            )),
        });
    }

    Ok(ClippyResult {
        success: true,
        warnings,
        errors,
        lints_found,
        diagnostics,
        error: None,
    })
}

/// Parse clippy output to extract warnings, errors, and lint names
fn parse_clippy_output(output: &str) -> (u32, u32, Vec<String>) {
    let mut warnings = 0;
    let mut errors = 0;
    let mut lints_found = Vec::new();

    for line in output.lines() {
        if line.contains("warning:") || line.contains("warning[") {
            warnings += 1;
        } else if line.contains("error:") || line.contains("error[") {
            errors += 1;
        }

        // Extract lint name from any line that contains lint information
        if let Some(start) = line.find("lint: `") {
            let rest = &line[start + 7..]; // Skip "lint: `"
            if let Some(end) = rest.find('`') {
                let lint = rest[..end].to_string();
                if !lints_found.contains(&lint) {
                    lints_found.push(lint);
                }
            }
        }
    }

    (warnings, errors, lints_found)
}

/// Parse cargo clippy JSON output and extract diagnostics
fn parse_clippy_output_json(stdout: &str) -> Vec<ClippyDiagnostic> {
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

/// Parse a compiler-message JSON object into a ClippyDiagnostic
fn parse_compiler_message(message: &serde_json::Value) -> Option<ClippyDiagnostic> {
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

    Some(ClippyDiagnostic {
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

/// Create JSON schema for cargo clippy tool
pub fn create_schema() -> serde_json::Map<String, serde_json::Value> {
    let mut properties = serde_json::Map::new();

    properties.insert(
        "package".to_string(),
        serde_json::Value::Object(create_string_property("Specific package to lint")),
    );

    properties.insert(
        "workspace".to_string(),
        serde_json::Value::Object(create_bool_property("Lint entire workspace")),
    );

    properties.insert(
        "all_targets".to_string(),
        serde_json::Value::Object(create_bool_property(
            "Lint all targets (including tests, examples, etc.)",
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

    properties.insert(
        "warnings_as_errors".to_string(),
        serde_json::Value::Object(create_bool_property("Treat warnings as errors")),
    );

    properties.insert(
        "lints".to_string(),
        serde_json::Value::Object(create_array_property(
            "string",
            "List of lints to enable or configure",
        )),
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
            "Run cargo clippy to catch common mistakes and improve Rust code. Returns warnings, errors, and lint information.".to_string(),
        ),
    );

    schema
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_clippy_params_deserialization() {
        let json = json!({
            "package": "my-package",
            "workspace": true,
            "all_targets": true,
            "all_features": false,
            "no_default_features": true,
            "features": ["feat1", "feat2"],
            "warnings_as_errors": true,
            "lints": ["clippy::pedantic", "clippy::nursery"],
            "working_dir": "/path/to/dir"
        });

        let params: ClippyParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.package, Some("my-package".to_string()));
        assert!(params.workspace);
        assert!(params.all_targets);
        assert!(!params.all_features);
        assert!(params.no_default_features);
        assert_eq!(
            params.features,
            Some(vec!["feat1".to_string(), "feat2".to_string()])
        );
        assert!(params.warnings_as_errors);
        assert_eq!(
            params.lints,
            Some(vec![
                "clippy::pedantic".to_string(),
                "clippy::nursery".to_string()
            ])
        );
        assert_eq!(params.working_dir, Some("/path/to/dir".to_string()));
    }

    #[test]
    fn test_clippy_params_default() {
        let json = json!({});

        let params: ClippyParams = serde_json::from_value(json).unwrap();
        assert!(params.package.is_none());
        assert!(!params.workspace);
        assert!(!params.all_targets);
        assert!(!params.all_features);
        assert!(!params.no_default_features);
        assert!(params.features.is_none());
        assert!(!params.warnings_as_errors);
        assert!(params.lints.is_none());
        assert!(params.working_dir.is_none());
    }

    #[test]
    fn test_clippy_result_serialization() {
        let result = ClippyResult {
            success: true,
            warnings: 3,
            errors: 0,
            lints_found: vec![
                "clippy::pedantic".to_string(),
                "clippy::nursery".to_string(),
            ],
            diagnostics: vec![],
            error: None,
        };

        let json = serde_json::to_value(&result).unwrap();
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["warnings"], 3);
        assert_eq!(json["errors"], 0);
        assert_eq!(json["lints_found"].as_array().unwrap().len(), 2);
        assert!(json["diagnostics"].as_array().unwrap().is_empty());
        assert!(json["error"].is_null());
    }

    #[test]
    fn test_clippy_result_error_serialization() {
        let result = ClippyResult {
            success: false,
            warnings: 5,
            errors: 2,
            lints_found: vec!["clippy::pedantic".to_string()],
            diagnostics: vec![],
            error: Some("cargo clippy failed with 2 errors and 5 warnings".to_string()),
        };

        let json = serde_json::to_value(&result).unwrap();
        assert!(!json["success"].as_bool().unwrap());
        assert_eq!(json["warnings"], 5);
        assert_eq!(json["errors"], 2);
        assert_eq!(
            json["error"],
            "cargo clippy failed with 2 errors and 5 warnings"
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
        assert!(properties.contains_key("warnings_as_errors"));
        assert!(properties.contains_key("lints"));
    }

    #[test]
    fn test_schema_description() {
        let schema = create_schema();
        let description = schema["description"].as_str().unwrap();

        assert!(!description.is_empty());
        assert!(description.contains("cargo clippy"));
        assert!(description.contains("lint"));
    }

    #[test]
    fn test_parse_clippy_output() {
        let output = "warning: unused variable\n  --> src/main.rs:10:5\n   |\n   = note: lint: `clippy::unused_variables`\nerror: expected identifier\n  --> src/main.rs:15:10\n   |\n   = note: lint: `clippy::expect_used`";
        let (warnings, errors, lints) = parse_clippy_output(output);

        assert_eq!(warnings, 1);
        assert_eq!(errors, 1);
        assert!(lints.contains(&"clippy::unused_variables".to_string()));
        assert!(lints.contains(&"clippy::expect_used".to_string()));
    }

    #[test]
    fn test_parse_clippy_output_empty() {
        let output = "";
        let (warnings, errors, lints) = parse_clippy_output(output);

        assert_eq!(warnings, 0);
        assert_eq!(errors, 0);
        assert!(lints.is_empty());
    }

    #[test]
    fn test_parse_clippy_output_no_lints() {
        let output = "warning: this is a warning\nerror: this is an error";
        let (warnings, errors, lints) = parse_clippy_output(output);

        assert_eq!(warnings, 1);
        assert_eq!(errors, 1);
        assert!(lints.is_empty());
    }
}
