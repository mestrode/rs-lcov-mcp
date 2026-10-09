//! Cargo clippy command handler
//!
//! This module provides the handler function for the cargo clippy command.

use crate::commands::utils::{create_array_property, create_bool_property, create_string_property};
use crate::error::CommandResult;
use serde::{Deserialize, Serialize};
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
    /// Command output
    pub output: Option<String>,
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

    let output = cmd.output().map_err(|e| {
        crate::error::CommandError::ExecutionFailed(format!(
            "Failed to execute cargo clippy: {}",
            e
        ))
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let (warnings, errors, lints_found) = parse_clippy_output(&stderr);

    if !output.status.success() {
        return Ok(ClippyResult {
            success: false,
            warnings,
            errors,
            lints_found,
            output: Some(stderr.into_owned()),
            error: Some(format!(
                "cargo clippy found {} errors and {} warnings",
                errors, warnings
            )),
        });
    }

    Ok(ClippyResult {
        success: true,
        warnings,
        errors,
        lints_found,
        output: Some(stdout.into_owned()),
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
            "lints": ["clippy::pedantic", "clippy::nursery"]
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
            output: Some("Linting successful".to_string()),
            error: None,
        };

        let json = serde_json::to_value(&result).unwrap();
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["warnings"], 3);
        assert_eq!(json["errors"], 0);
        assert_eq!(json["lints_found"].as_array().unwrap().len(), 2);
        assert_eq!(json["output"], "Linting successful");
        assert!(json["error"].is_null());
    }

    #[test]
    fn test_clippy_result_error_serialization() {
        let result = ClippyResult {
            success: false,
            warnings: 5,
            errors: 2,
            lints_found: vec!["clippy::pedantic".to_string()],
            output: Some("Linting failed".to_string()),
            error: Some("cargo clippy found 2 errors and 5 warnings".to_string()),
        };

        let json = serde_json::to_value(&result).unwrap();
        assert!(!json["success"].as_bool().unwrap());
        assert_eq!(json["warnings"], 5);
        assert_eq!(json["errors"], 2);
        assert_eq!(json["error"], "cargo clippy found 2 errors and 5 warnings");
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
