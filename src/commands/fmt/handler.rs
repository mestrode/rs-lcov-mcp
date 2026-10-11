//! Cargo fmt command handler
//!
//! This module provides the handler function for the cargo fmt command.

use crate::commands::utils::{create_bool_property, create_string_property};
use crate::error::CommandResult;
use serde::{Deserialize, Serialize};
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

/// Result of the cargo fmt tool
#[derive(Debug, Serialize, Clone)]
pub struct FmtResult {
    /// Whether the command executed successfully
    pub success: bool,
    /// Whether any files were formatted (or would be formatted in check mode)
    pub formatted: bool,
    /// Command output
    pub output: Option<String>,
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

    let output = cmd.output().map_err(|e| {
        crate::error::CommandError::ExecutionFailed(format!("Failed to execute cargo fmt: {}", e))
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        let stderr_str = stderr.to_string();
        return Ok(FmtResult {
            success: false,
            formatted: false,
            output: Some(stderr_str.clone()),
            error: Some(format!("cargo fmt failed: {}", stderr_str)),
        });
    }

    // Check if any files were formatted (non-check mode)
    let formatted = if !params.check && !stdout.is_empty() {
        true
    } else if params.check && output.status.success() {
        // In check mode, success means no changes needed
        false
    } else {
        // In check mode, failure means files need formatting
        true
    };

    Ok(FmtResult {
        success: true,
        formatted,
        output: Some(stdout.into_owned()),
        error: None,
    })
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
            output: Some("Files formatted".to_string()),
            error: None,
        };

        let json = serde_json::to_value(&result).unwrap();
        assert!(json["success"].as_bool().unwrap());
        assert!(json["formatted"].as_bool().unwrap());
        assert_eq!(json["output"], "Files formatted");
        assert!(json["error"].is_null());
    }

    #[test]
    fn test_fmt_result_error_serialization() {
        let result = FmtResult {
            success: false,
            formatted: false,
            output: Some("Error output".to_string()),
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
