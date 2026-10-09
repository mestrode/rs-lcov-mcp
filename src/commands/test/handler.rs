//! Cargo test command handler
//!
//! This module provides the handler function for the cargo test command.

use crate::commands::utils::{
    create_array_property, create_bool_property, create_number_property, create_string_property,
    parse_test_output,
};
use crate::error::CommandResult;
use serde::{Deserialize, Serialize};
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
    /// Command output
    pub output: Option<String>,
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

    let output = cmd.output().map_err(|e| {
        crate::error::CommandError::ExecutionFailed(format!("Failed to execute cargo test: {}", e))
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined_output = format!("{}\n{}", stdout, stderr);

    let test_summary = parse_test_output(&combined_output);

    if !output.status.success() {
        return Ok(TestResult {
            success: false,
            tests_run: test_summary.tests_run,
            tests_passed: test_summary.tests_passed,
            tests_failed: test_summary.tests_failed,
            ignored: test_summary.ignored,
            measured: test_summary.measured,
            filtered_out: test_summary.filtered_out,
            output: Some(stderr.into_owned()),
            error: Some("cargo test failed".to_string()),
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
        output: Some(stdout.into_owned()),
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
            output: Some("Tests completed".to_string()),
            error: None,
        };

        let json = serde_json::to_value(&result).unwrap();
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["tests_run"], 10);
        assert_eq!(json["tests_passed"], 8);
        assert_eq!(json["tests_failed"], 2);
        assert_eq!(json["ignored"], 1);
        assert_eq!(json["output"], "Tests completed");
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
            output: Some("Tests failed".to_string()),
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
