//! Cargo clean command handler
//!
//! This module provides the handler function for the cargo clean command.

use crate::commands::utils::{create_bool_property, create_string_property};
use crate::error::CommandResult;
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Parameters for the cargo clean tool
#[derive(Debug, Deserialize, Clone)]
pub struct CleanParams {
    /// Specific package to clean
    pub package: Option<String>,
    /// Clean entire workspace
    #[serde(default)]
    pub workspace: bool,
    /// Clean release artifacts only
    #[serde(default)]
    pub release: bool,
    /// Clean debug artifacts only
    #[serde(default)]
    pub debug: bool,
    /// Clean all target directories
    #[serde(default)]
    pub all_targets: bool,
    /// Clean doc artifacts
    #[serde(default)]
    pub doc: bool,
    /// Target directory to clean
    pub target_dir: Option<String>,
    /// Working directory for cargo command
    pub working_dir: Option<String>,
}

/// Result of the cargo clean tool
#[derive(Debug, Serialize, Clone)]
pub struct CleanResult {
    /// Whether the command executed successfully
    pub success: bool,
    /// What was cleaned (debug, release, doc, etc.)
    pub cleaned: Vec<String>,
    /// Command output
    pub output: Option<String>,
    /// Error message if the command failed
    pub error: Option<String>,
}

/// Handler for cargo clean command
pub async fn handle_clean(params: CleanParams) -> CommandResult<CleanResult> {
    let mut cmd = Command::new("cargo");
    cmd.arg("clean");

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

    if params.doc {
        cmd.arg("--doc");
    }

    if let Some(target_dir) = params.target_dir {
        cmd.args(["--target-dir", &target_dir]);
    }

    if let Some(ref dir) = params.working_dir {
        cmd.current_dir(dir);
    }

    let output = cmd.output().map_err(|e| {
        crate::error::CommandError::ExecutionFailed(format!("Failed to execute cargo clean: {}", e))
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Determine what was cleaned based on parameters
    let mut cleaned = Vec::new();
    if params.release {
        cleaned.push("release".to_string());
    }
    if params.debug {
        cleaned.push("debug".to_string());
    }
    if params.doc {
        cleaned.push("doc".to_string());
    }
    if params.all_targets {
        cleaned.push("all-targets".to_string());
    }
    if !params.release && !params.debug && !params.doc && !params.all_targets {
        cleaned.push("default".to_string());
    }

    if !output.status.success() {
        let stderr_str = stderr.to_string();
        return Ok(CleanResult {
            success: false,
            cleaned,
            output: Some(stderr_str.clone()),
            error: Some(format!("cargo clean failed: {}", stderr_str)),
        });
    }

    Ok(CleanResult {
        success: true,
        cleaned,
        output: Some(stdout.into_owned()),
        error: None,
    })
}

/// Create JSON schema for cargo clean tool
pub fn create_schema() -> serde_json::Map<String, serde_json::Value> {
    let mut properties = serde_json::Map::new();

    properties.insert(
        "package".to_string(),
        serde_json::Value::Object(create_string_property("Specific package to clean")),
    );

    properties.insert(
        "workspace".to_string(),
        serde_json::Value::Object(create_bool_property("Clean entire workspace")),
    );

    properties.insert(
        "release".to_string(),
        serde_json::Value::Object(create_bool_property("Clean release artifacts only")),
    );

    properties.insert(
        "debug".to_string(),
        serde_json::Value::Object(create_bool_property("Clean debug artifacts only")),
    );

    properties.insert(
        "all_targets".to_string(),
        serde_json::Value::Object(create_bool_property("Clean all target directories")),
    );

    properties.insert(
        "doc".to_string(),
        serde_json::Value::Object(create_bool_property("Clean doc artifacts")),
    );

    properties.insert(
        "target_dir".to_string(),
        serde_json::Value::Object(create_string_property("Target directory to clean")),
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
            "Run cargo clean to remove generated artifacts. Returns what was cleaned and any output.".to_string(),
        ),
    );

    schema
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_clean_params_deserialization() {
        let json = json!({
            "package": "my-package",
            "workspace": true,
            "release": true,
            "debug": false,
            "all_targets": true,
            "doc": true,
            "target_dir": "./target",
            "working_dir": "/path/to/dir"
        });

        let params: CleanParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.package, Some("my-package".to_string()));
        assert!(params.workspace);
        assert!(params.release);
        assert!(!params.debug);
        assert!(params.all_targets);
        assert!(params.doc);
        assert_eq!(params.target_dir, Some("./target".to_string()));
        assert_eq!(params.working_dir, Some("/path/to/dir".to_string()));
    }

    #[test]
    fn test_clean_params_default() {
        let json = json!({});

        let params: CleanParams = serde_json::from_value(json).unwrap();
        assert!(params.package.is_none());
        assert!(!params.workspace);
        assert!(!params.release);
        assert!(!params.debug);
        assert!(!params.all_targets);
        assert!(!params.doc);
        assert!(params.target_dir.is_none());
        assert!(params.working_dir.is_none());
    }

    #[test]
    fn test_clean_result_serialization() {
        let result = CleanResult {
            success: true,
            cleaned: vec!["release".to_string(), "debug".to_string()],
            output: Some("Cleaned artifacts".to_string()),
            error: None,
        };

        let json = serde_json::to_value(&result).unwrap();
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["cleaned"].as_array().unwrap().len(), 2);
        assert_eq!(json["output"], "Cleaned artifacts");
        assert!(json["error"].is_null());
    }

    #[test]
    fn test_clean_result_error_serialization() {
        let result = CleanResult {
            success: false,
            cleaned: vec![],
            output: Some("Clean failed".to_string()),
            error: Some("cargo clean failed".to_string()),
        };

        let json = serde_json::to_value(&result).unwrap();
        assert!(!json["success"].as_bool().unwrap());
        assert_eq!(json["error"], "cargo clean failed");
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
        assert!(properties.contains_key("doc"));
        assert!(properties.contains_key("target_dir"));
    }

    #[test]
    fn test_schema_description() {
        let schema = create_schema();
        let description = schema["description"].as_str().unwrap();

        assert!(!description.is_empty());
        assert!(description.contains("cargo clean"));
        assert!(description.contains("artifacts"));
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
    }
}
