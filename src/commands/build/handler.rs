//! Cargo build command handler
//!
//! This module provides the handler function for the cargo build command.

use crate::commands::utils::{create_array_property, create_bool_property, create_string_property};
use crate::error::CommandResult;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::process::Command;

/// Parameters for the cargo build tool
#[derive(Debug, Deserialize, Clone)]
pub struct BuildParams {
    /// Specific package to build
    pub package: Option<String>,
    /// Build entire workspace
    #[serde(default)]
    pub workspace: bool,
    /// Build in release mode
    #[serde(default)]
    pub release: bool,
    /// Build with debug information (default)
    #[serde(default)]
    pub debug: bool,
    /// Build all targets
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
    /// Require Cargo.lock to be up to date
    #[serde(default)]
    pub locked: bool,
    /// Require Cargo.lock to exist and be up to date
    #[serde(default)]
    pub frozen: bool,
    /// Build for a specific target triple
    pub target: Option<String>,
    /// Directory for generated artifacts
    pub target_dir: Option<String>,
}

/// A single diagnostic message from cargo build
#[derive(Debug, Serialize, Clone)]
pub struct BuildDiagnostic {
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

/// Result of the cargo build tool
#[derive(Debug, Serialize, Clone)]
pub struct BuildResult {
    /// Whether the command executed successfully
    pub success: bool,
    /// Whether this was a release build
    pub release_mode: bool,
    /// Target triple if specified
    pub target: Option<String>,
    /// Number of warnings found
    pub warnings: u32,
    /// Number of errors found
    pub errors: u32,
    /// Structured diagnostics (errors and warnings)
    pub diagnostics: Vec<BuildDiagnostic>,
    /// Error message if the command failed
    pub error: Option<String>,
}

/// Handler for cargo build command
pub async fn handle_build(params: BuildParams) -> CommandResult<BuildResult> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build");

    if params.workspace {
        cmd.arg("--workspace");
    }

    if let Some(pkg) = params.package {
        cmd.args(["--package", &pkg]);
    }

    if params.release {
        cmd.arg("--release");
    }

    if params.debug {
        cmd.arg("--debug");
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

    if params.locked {
        cmd.arg("--locked");
    }

    if params.frozen {
        cmd.arg("--frozen");
    }

    if let Some(ref target) = params.target {
        cmd.args(["--target", target]);
    }

    if let Some(ref target_dir) = params.target_dir {
        cmd.args(["--target-dir", target_dir]);
    }

    cmd.arg("--message-format=json");

    let output = cmd.output().map_err(|e| {
        crate::error::CommandError::ExecutionFailed(format!("Failed to execute cargo build: {}", e))
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let _stderr = String::from_utf8_lossy(&output.stderr);

    // Parse JSON output from stdout only
    let diagnostics = parse_build_output(&stdout);

    // Count errors and warnings
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

    let success = output.status.success();

    if !success {
        return Ok(BuildResult {
            success: false,
            release_mode: params.release,
            target: params.target,
            warnings,
            errors,
            diagnostics,
            error: Some(format!(
                "cargo build failed with {} errors and {} warnings",
                errors, warnings
            )),
        });
    }

    Ok(BuildResult {
        success: true,
        release_mode: params.release,
        target: params.target,
        warnings,
        errors,
        diagnostics,
        error: None,
    })
}

/// Parse cargo build JSON output and extract diagnostics
fn parse_build_output(stdout: &str) -> Vec<BuildDiagnostic> {
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

/// Parse a compiler-message JSON object into a BuildDiagnostic
fn parse_compiler_message(message: &serde_json::Value) -> Option<BuildDiagnostic> {
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

    Some(BuildDiagnostic {
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

/// Create JSON schema for cargo build tool
pub fn create_schema() -> serde_json::Map<String, serde_json::Value> {
    let mut properties = serde_json::Map::new();

    properties.insert(
        "package".to_string(),
        serde_json::Value::Object(create_string_property("Specific package to build")),
    );

    properties.insert(
        "workspace".to_string(),
        serde_json::Value::Object(create_bool_property("Build entire workspace")),
    );

    properties.insert(
        "release".to_string(),
        serde_json::Value::Object(create_bool_property("Build in release mode")),
    );

    properties.insert(
        "debug".to_string(),
        serde_json::Value::Object(create_bool_property("Build with debug information")),
    );

    properties.insert(
        "all_targets".to_string(),
        serde_json::Value::Object(create_bool_property("Build all targets")),
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
        "locked".to_string(),
        serde_json::Value::Object(create_bool_property("Require Cargo.lock to be up to date")),
    );

    properties.insert(
        "frozen".to_string(),
        serde_json::Value::Object(create_bool_property(
            "Require Cargo.lock to exist and be up to date",
        )),
    );

    properties.insert(
        "target".to_string(),
        serde_json::Value::Object(create_string_property("Build for a specific target triple")),
    );

    properties.insert(
        "target_dir".to_string(),
        serde_json::Value::Object(create_string_property("Directory for generated artifacts")),
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
            "Run cargo build to compile the Rust project. Returns build information including warnings, errors, and target details.".to_string(),
        ),
    );

    schema
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_build_params_deserialization() {
        let json = json!({
            "package": "my-package",
            "workspace": true,
            "release": true,
            "debug": false,
            "all_targets": true,
            "all_features": false,
            "no_default_features": true,
            "features": ["feat1", "feat2"],
            "locked": true,
            "frozen": false,
            "target": "x86_64-unknown-linux-gnu",
            "target_dir": "./target"
        });

        let params: BuildParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.package, Some("my-package".to_string()));
        assert!(params.workspace);
        assert!(params.release);
        assert!(!params.debug);
        assert!(params.all_targets);
        assert!(!params.all_features);
        assert!(params.no_default_features);
        assert_eq!(
            params.features,
            Some(vec!["feat1".to_string(), "feat2".to_string()])
        );
        assert!(params.locked);
        assert!(!params.frozen);
        assert_eq!(params.target, Some("x86_64-unknown-linux-gnu".to_string()));
        assert_eq!(params.target_dir, Some("./target".to_string()));
    }

    #[test]
    fn test_build_params_default() {
        let json = json!({});

        let params: BuildParams = serde_json::from_value(json).unwrap();
        assert!(params.package.is_none());
        assert!(!params.workspace);
        assert!(!params.release);
        assert!(!params.debug);
        assert!(!params.all_targets);
        assert!(!params.all_features);
        assert!(!params.no_default_features);
        assert!(params.features.is_none());
        assert!(!params.locked);
        assert!(!params.frozen);
        assert!(params.target.is_none());
        assert!(params.target_dir.is_none());
    }

    #[test]
    fn test_build_result_serialization() {
        let result = BuildResult {
            success: true,
            release_mode: true,
            target: Some("x86_64-unknown-linux-gnu".to_string()),
            diagnostics: vec![],
            error: None,
            warnings: 2,
            errors: 0,
        };

        let json = serde_json::to_value(&result).unwrap();
        assert!(json["success"].as_bool().unwrap());
        assert!(json["release_mode"].as_bool().unwrap());
        assert_eq!(json["target"], "x86_64-unknown-linux-gnu");
        assert!(json["error"].is_null());
        assert!(json["diagnostics"].as_array().unwrap().is_empty());
        assert_eq!(json["warnings"], 2);
        assert_eq!(json["errors"], 0);
    }

    #[test]
    fn test_build_result_error_serialization() {
        let result = BuildResult {
            success: false,
            release_mode: false,
            target: None,
            diagnostics: vec![],
            error: Some("cargo build failed with 2 errors and 5 warnings".to_string()),
            warnings: 5,
            errors: 2,
        };

        let json = serde_json::to_value(&result).unwrap();
        assert!(!json["success"].as_bool().unwrap());
        assert!(!json["release_mode"].as_bool().unwrap());
        assert_eq!(
            json["error"],
            "cargo build failed with 2 errors and 5 warnings"
        );
        assert_eq!(json["warnings"], 5);
        assert_eq!(json["errors"], 2);
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
        assert!(properties.contains_key("release"));
        assert!(properties.contains_key("debug"));
        assert!(properties.contains_key("all_targets"));
        assert!(properties.contains_key("all_features"));
        assert!(properties.contains_key("no_default_features"));
        assert!(properties.contains_key("features"));
        assert!(properties.contains_key("locked"));
        assert!(properties.contains_key("frozen"));
        assert!(properties.contains_key("target"));
        assert!(properties.contains_key("target_dir"));
    }

    #[test]
    fn test_schema_description() {
        let schema = create_schema();
        let description = schema["description"].as_str().unwrap();

        assert!(!description.is_empty());
        assert!(description.contains("cargo build"));
        assert!(description.contains("compile"));
    }

    #[test]
    fn test_schema_property_types() {
        let schema = create_schema();
        let properties = schema["properties"].as_object().unwrap();

        // Check that boolean properties are boolean
        let workspace_prop = properties["workspace"].as_object().unwrap();
        assert_eq!(workspace_prop["type"], "boolean");

        let release_prop = properties["release"].as_object().unwrap();
        assert_eq!(release_prop["type"], "boolean");

        // Check that string properties are string
        let package_prop = properties["package"].as_object().unwrap();
        assert_eq!(package_prop["type"], "string");

        let target_prop = properties["target"].as_object().unwrap();
        assert_eq!(target_prop["type"], "string");

        // Check that array property is array
        let features_prop = properties["features"].as_object().unwrap();
        assert_eq!(features_prop["type"], "array");
    }
}
