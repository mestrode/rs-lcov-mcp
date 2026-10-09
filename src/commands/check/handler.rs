//! Cargo check command handler
//!
//! This module provides the handler function for the cargo check command.

use crate::commands::utils::{
    create_array_property, create_bool_property, create_string_property, parse_cargo_output,
};
use crate::error::CommandResult;
use serde::{Deserialize, Serialize};
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

/// Result of the cargo check tool
#[derive(Debug, Serialize, Clone)]
pub struct CheckResult {
    /// Whether the command executed successfully
    pub success: bool,
    /// Number of warnings found
    pub warnings: u32,
    /// Number of errors found
    pub errors: u32,
    /// Command output
    pub output: Option<String>,
    /// Error message if the command failed
    pub error: Option<String>,
}

/// Handler for cargo check command
pub async fn handle_check(params: CheckParams) -> CommandResult<CheckResult> {
    let mut cmd = Command::new("cargo");
    cmd.arg("check");

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

    let (warnings, errors) = parse_cargo_output(&stderr);

    if !output.status.success() {
        return Ok(CheckResult {
            success: false,
            warnings,
            errors,
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
        output: Some(stdout.into_owned()),
        error: None,
    })
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
            "Run cargo check to verify code compiles without running tests. Returns warnings, errors, and compilation output.".to_string(),
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
            output: Some("Compilation successful".to_string()),
            error: None,
        };

        let json = serde_json::to_value(&result).unwrap();
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["warnings"], 2);
        assert_eq!(json["errors"], 0);
        assert_eq!(json["output"], "Compilation successful");
        assert!(json["error"].is_null());
    }

    #[test]
    fn test_check_result_error_serialization() {
        let result = CheckResult {
            success: false,
            warnings: 5,
            errors: 2,
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
}
