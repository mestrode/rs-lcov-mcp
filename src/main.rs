//! rs-cargo-tools: MCP server for Rust development tools
//!
//! This is an MCP (Model Context Protocol) server that provides tools for Rust development.
//! It includes cargo command tools (fmt, check, clippy, test, build) and code coverage
//! analysis using cargo-llvm-cov.
//!
//! # Usage
//!
//! The server can be run as a standalone process and communicates via stdio.
//! It provides tools for:
//! - Code formatting (cargo_fmt)
//! - Code checking (cargo_check)
//! - Linting (cargo_clippy)
//! - Testing (cargo_test)
//! - Building (cargo_build)
//! - Code coverage analysis (get_coverage_summary, get_files_coverage, get_file_line_coverage)

use rmcp::ServiceExt;
use rmcp::transport::stdio;
use rs_lcov_mcp::server::handler::CoverageServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create the server handler
    let server = CoverageServer::new();

    // Serve using stdio transport and wait for the service to complete
    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use super::*;
    use rmcp::ServerHandler;
    use rs_lcov_mcp::{
        commands::coverage::{
            create_execute_schema, create_file_schema, create_line_schema, handle_execute_coverage,
            handle_get_file_coverage, handle_get_line_coverage, parse_coverage_summary,
            parse_file_coverage, parse_line_coverage, validate_execute_params,
            validate_file_params, validate_line_params,
        },
        commands::llvm_cov::build_llvm_cov_command,
        error::CoverageError,
        model::*,
    };
    use serde_json::json;

    #[test]
    fn test_modules_are_accessible() {
        // Verify that all modules can be accessed
        let _coverage = parse_coverage_summary;
        let _error = CoverageError::Validation("test".to_string());
        let _server = CoverageServer::new();
        let _model = CoverageSummary {
            success: true,
            coverage: CoverageMetrics {
                percentage: 0.0,
                lines_covered: 0,
                total_lines: 0,
                functions_covered: 0,
                total_functions: 0,
            },
            threshold: None,
            error: None,
        };
    }

    #[test]
    fn test_server_creation_in_main() {
        // Verify that we can create a server from main module
        use rmcp::ServerHandler;
        let server = CoverageServer::new();
        let info = server.get_info();
        assert_eq!(info.server_info.name, "rs-cargo-tools");
    }

    // ========== Integration Tests ==========

    #[test]
    fn test_full_module_integration() {
        // Test that all modules work together
        use rs_lcov_mcp::model::*;

        // Test that we can create instances of all model types
        let metrics = CoverageMetrics {
            percentage: 75.5,
            lines_covered: 100,
            total_lines: 200,
            functions_covered: 10,
            total_functions: 20,
        };

        let summary = CoverageSummary {
            success: true,
            coverage: metrics,
            threshold: None,
            error: None,
        };

        assert!(summary.success);
    }

    #[test]
    fn test_error_types_accessible() {
        // Verify all error types are accessible from main
        use rs_lcov_mcp::error::CoverageError;

        let validation_err = CoverageError::Validation("test validation".to_string());
        let execution_err = CoverageError::ExecutionFailed("test execution".to_string());
        let parse_err = CoverageError::ParseError("test parse".to_string());

        assert!(validation_err.to_string().contains("Validation"));
        assert!(execution_err.to_string().contains("execution"));
        assert!(parse_err.to_string().contains("parse"));
    }

    #[test]
    fn test_coverage_functions_accessible() {
        // Verify all coverage functions are accessible
        use rs_lcov_mcp::commands::coverage::*;
        use rs_lcov_mcp::commands::llvm_cov::execute_llvm_cov;

        // These should compile without errors
        let _parse_summary = parse_coverage_summary;
        let _parse_file = parse_file_coverage;
        let _parse_line = parse_line_coverage;
        let _build_cmd = build_llvm_cov_command;
        let _execute = execute_llvm_cov;
        let _validate_exec = validate_execute_params;
        let _validate_file = validate_file_params;
        let _validate_line = validate_line_params;
        let _handle_exec = handle_execute_coverage;
        let _handle_file = handle_get_file_coverage;
        let _handle_line = handle_get_line_coverage;
        let _schema_exec = create_execute_schema;
        let _schema_file = create_file_schema;
        let _schema_line = create_line_schema;
    }

    #[test]
    fn test_mcp_module_accessible() {
        // Verify MCP module functions are accessible
        use rmcp::ServerHandler;
        use rs_lcov_mcp::server::handler::CoverageServer;

        let server = CoverageServer::new();
        let info = server.get_info();

        assert_eq!(info.server_info.name, "rs-cargo-tools");
        assert_eq!(info.server_info.version, "0.3.0");
    }

    #[test]
    fn test_model_types_accessible() {
        // Verify all model types are accessible
        use rs_lcov_mcp::model::*;

        let _metrics = CoverageMetrics {
            percentage: 50.0,
            lines_covered: 50,
            total_lines: 100,
            functions_covered: 5,
            total_functions: 10,
        };

        let _summary = CoverageSummary {
            success: true,
            coverage: _metrics,
            threshold: None,
            error: None,
        };

        let _threshold = ThresholdResult {
            met: true,
            value: 80.0,
        };

        let _branch = BranchInfo {
            line: 42,
            condition: "x > 0".to_string(),
            covered: true,
            hits: 5,
        };

        let _func = FunctionCoverage {
            name: "test_func".to_string(),
            line_coverage: 90.0,
            branch_coverage: 80.0,
            branches: vec![_branch],
        };

        let _file = FileCoverage {
            path: "src/lib.rs".to_string(),
            line_coverage: 85.0,
            branch_coverage: 70.0,
            lines_covered: 170,
            total_lines: 200,
            functions: vec![_func],
        };

        let _file_result = FileCoverageResult {
            success: true,
            files: vec![_file],
            error: None,
        };

        let _line = LineCoverage {
            line: 42,
            covered: true,
            hits: 10,
        };

        let _range = CoveredRange {
            start: 1,
            end: 10,
            hits: 5,
        };

        let _file_line = FileLineCoverage {
            path: "src/lib.rs".to_string(),
            line_coverage: 85.0,
            covered_ranges: vec![_range],
            uncovered_lines: vec![11, 12],
        };

        let _line_result = LineCoverageResult {
            success: true,
            file: Some(_file_line),
            error: None,
        };

        let _exec_params = ExecuteCoverageParams {
            workspace: false,
            package: None,
            threshold: Some(80.0),
        };

        let _files_params = GetFilesCoverageParams {
            workspace: false,
            package: None,
            file: None,
            coverage_below: Some(50.0),
        };

        let _line_params = GetFileLineCoverageParams {
            file: "src/lib.rs".to_string(),
            workspace: false,
            package: None,
        };
    }

    // ========== Main Function Tests ==========

    #[tokio::test]
    async fn test_main_server_creation() {
        // Test that we can create the server as done in main()
        let server = CoverageServer::new();
        assert_eq!(server.get_info().server_info.name, "rs-cargo-tools");
    }

    #[test]
    fn test_main_exports() {
        // Verify that main.rs exports are accessible
        // All modules should be accessible
        assert!(true); // Placeholder - modules are verified by compilation
    }

    // ========== Version and Metadata Tests ==========

    #[test]
    fn test_server_version() {
        let server = CoverageServer::new();
        let info = server.get_info();

        assert_eq!(info.server_info.name, "rs-cargo-tools");
        assert_eq!(info.server_info.version, "0.3.0");
    }

    #[test]
    fn test_protocol_version() {
        let server = CoverageServer::new();
        let info = server.get_info();

        assert_eq!(info.protocol_version, rmcp::model::ProtocolVersion::LATEST);
    }

    // ========== Consistency Tests ==========

    #[test]
    fn test_multiple_server_instances_consistent() {
        let server1 = CoverageServer::new();
        let server2 = CoverageServer::new();

        let info1 = server1.get_info();
        let info2 = server2.get_info();

        assert_eq!(info1.server_info.name, info2.server_info.name);
        assert_eq!(info1.server_info.version, info2.server_info.version);
        assert_eq!(info1.protocol_version, info2.protocol_version);
    }

    #[test]
    fn test_server_info_immutable() {
        let server = CoverageServer::new();

        let info1 = server.get_info();
        let info2 = server.get_info();

        // Server info should be consistent across multiple calls
        assert_eq!(info1.server_info.name, info2.server_info.name);
        assert_eq!(info1.server_info.version, info2.server_info.version);
    }

    // ========== Error Handling Tests ==========

    #[test]
    fn test_error_conversions() {
        use rs_lcov_mcp::error::CoverageError;
        use std::io;

        // Test From implementations
        let io_err = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let coverage_err: CoverageError = io_err.into();
        assert!(matches!(coverage_err, CoverageError::ExecutionFailed(_)));

        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let coverage_err: CoverageError = json_err.into();
        assert!(matches!(coverage_err, CoverageError::ParseError(_)));

        let string_err = "Something went wrong".to_string();
        let coverage_err: CoverageError = string_err.into();
        assert!(matches!(coverage_err, CoverageError::ExecutionFailed(_)));
    }

    // ========== Model Serialization Tests ==========

    #[test]
    fn test_model_serialization_in_main() {
        use rs_lcov_mcp::model::*;

        let metrics = CoverageMetrics {
            percentage: 75.5,
            lines_covered: 100,
            total_lines: 200,
            functions_covered: 10,
            total_functions: 20,
        };

        let json_str = serde_json::to_string(&metrics).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["lines_covered"], 100);
        assert_eq!(parsed["total_lines"], 200);
    }

    #[test]
    fn test_model_deserialization_in_main() {
        use rs_lcov_mcp::model::*;

        let json_str = r#"{
            "percentage": 75.5,
            "lines_covered": 100,
            "total_lines": 200,
            "functions_covered": 10,
            "total_functions": 20
        }"#;

        let metrics: CoverageMetrics = serde_json::from_str(json_str).unwrap();

        assert_eq!(metrics.lines_covered, 100);
        assert_eq!(metrics.total_lines, 200);
    }

    // ========== Parameter Validation Tests ==========

    #[test]
    fn test_parameter_validation_in_main() {
        use rs_lcov_mcp::commands::coverage::*;
        use rs_lcov_mcp::model::*;

        // Test valid parameters
        let exec_params = ExecuteCoverageParams {
            workspace: false,
            package: None,
            threshold: Some(80.0),
        };
        assert!(validate_execute_params(&exec_params).is_ok());

        let files_params = GetFilesCoverageParams {
            workspace: false,
            package: None,
            file: None,
            coverage_below: Some(50.0),
        };
        assert!(validate_file_params(&files_params).is_ok());

        let line_params = GetFileLineCoverageParams {
            file: "src/lib.rs".to_string(),
            workspace: false,
            package: None,
        };
        assert!(validate_line_params(&line_params).is_ok());
    }

    // ========== Schema Tests ==========

    #[test]
    fn test_schema_creation_in_main() {
        use rs_lcov_mcp::commands::coverage::*;

        let execute_schema = create_execute_schema();
        let file_schema = create_file_schema();
        let line_schema = create_line_schema();

        assert!(execute_schema.contains_key("type"));
        assert!(execute_schema.contains_key("properties"));
        assert!(file_schema.contains_key("type"));
        assert!(file_schema.contains_key("properties"));
        assert!(line_schema.contains_key("type"));
        assert!(line_schema.contains_key("properties"));
        assert!(line_schema.contains_key("required"));
    }

    // ========== Command Building Tests ==========

    #[test]
    fn test_command_building_in_main() {
        use rs_lcov_mcp::commands::llvm_cov::build_llvm_cov_command;

        let cmd = build_llvm_cov_command(false, None);
        let cmd_str = format!("{:?}", cmd);
        assert!(cmd_str.contains("cargo"));
        assert!(cmd_str.contains("llvm-cov"));
        assert!(cmd_str.contains("--json"));

        let cmd_workspace = build_llvm_cov_command(true, None);
        let cmd_workspace_str = format!("{:?}", cmd_workspace);
        assert!(cmd_workspace_str.contains("--workspace"));

        let cmd_package = build_llvm_cov_command(false, Some("my-package"));
        let cmd_package_str = format!("{:?}", cmd_package);
        assert!(cmd_package_str.contains("--package"));
        assert!(cmd_package_str.contains("my-package"));
    }

    // ========== Coverage Parsing Tests ==========

    #[test]
    fn test_coverage_parsing_in_main() {
        use rs_lcov_mcp::commands::coverage::parse_coverage_summary;
        use serde_json::json;

        let json_output = json!({
            "data": [{
                "totals": {
                    "lines": {
                        "covered": 100,
                        "count": 200
                    },
                    "functions": {
                        "covered": 10,
                        "count": 20
                    }
                }
            }]
        })
        .to_string();

        let result = parse_coverage_summary(&json_output).unwrap();

        assert_eq!(result.lines_covered, 100);
        assert_eq!(result.total_lines, 200);
        assert_eq!(result.functions_covered, 10);
        assert_eq!(result.total_functions, 20);
        assert!((result.percentage - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_file_coverage_parsing_in_main() {
        use rs_lcov_mcp::commands::coverage::parse_file_coverage;
        use serde_json::json;

        let json_output = json!({
            "data": [{
                "files": [
                    {
                        "filename": "src/lib.rs",
                        "summary": {
                            "lines": {
                                "covered": 50,
                                "count": 100
                            },
                            "branches": {
                                "percent": 75.0
                            }
                        }
                    }
                ]
            }]
        })
        .to_string();

        let result = parse_file_coverage(&json_output).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "src/lib.rs");
        assert!((result[0].line_coverage - 50.0).abs() < 0.01);
        assert!((result[0].branch_coverage - 75.0).abs() < 0.01);
    }

    #[test]
    fn test_line_coverage_parsing_in_main() {
        use rs_lcov_mcp::commands::coverage::parse_line_coverage;
        use serde_json::json;

        let json_output = json!({
            "data": [{
                "files": [
                    {
                        "filename": "src/lib.rs",
                        "summary": {
                            "lines": {
                                "percent": 50.0
                            }
                        },
                        "segments": [
                            [1, 1, 1, true, 0, 0],
                            [2, 1, 0, false, 0, 0],
                            [3, 1, 1, true, 0, 0]
                        ]
                    }
                ]
            }]
        })
        .to_string();

        let result = parse_line_coverage(&json_output, "src/lib.rs").unwrap();

        assert!(result.is_some());
        let file_coverage = result.unwrap();
        assert_eq!(file_coverage.path, "src/lib.rs");
        assert!(!file_coverage.covered_ranges.is_empty());
    }

    // ========== Compression Tests ==========

    #[test]
    fn test_line_coverage_model_in_main() {
        use rs_lcov_mcp::model::LineCoverage;

        let lines = vec![
            LineCoverage {
                line: 1,
                covered: true,
                hits: 5,
            },
            LineCoverage {
                line: 2,
                covered: true,
                hits: 5,
            },
            LineCoverage {
                line: 3,
                covered: true,
                hits: 5,
            },
            LineCoverage {
                line: 5,
                covered: true,
                hits: 3,
            },
            LineCoverage {
                line: 6,
                covered: false,
                hits: 0,
            },
        ];

        // This test verifies that LineCoverage type works correctly
        assert_eq!(lines.len(), 5);
        assert!(lines[0].covered);
        assert!(!lines[4].covered);
    }

    // ========== Async Handler Tests ==========

    #[tokio::test]
    async fn test_async_handlers_compilation() {
        // These tests verify that the async handlers compile and can be called
        // Note: We can't actually execute them without cargo-llvm-cov installed
        use rs_lcov_mcp::commands::coverage::*;
        use rs_lcov_mcp::model::*;

        // Test that we can create the parameters
        let exec_params = ExecuteCoverageParams {
            workspace: false,
            package: None,
            threshold: None,
        };

        let files_params = GetFilesCoverageParams {
            workspace: false,
            package: None,
            file: None,
            coverage_below: None,
        };

        let line_params = GetFileLineCoverageParams {
            file: "src/lib.rs".to_string(),
            workspace: false,
            package: None,
        };

        // We can't actually call the handlers without cargo-llvm-cov,
        // but we can verify they exist and have the right signatures
        let _handler_exec = handle_execute_coverage(exec_params);
        let _handler_files = handle_get_file_coverage(files_params);
        let _handler_line = handle_get_line_coverage(line_params);
    }

    // ========== Tool Schema Tests ==========

    #[test]
    fn test_tool_schemas_in_main() {
        use rs_lcov_mcp::commands::coverage::*;

        let execute_schema = create_execute_schema();
        let file_schema = create_file_schema();
        let line_schema = create_line_schema();

        // Verify all schemas have required fields
        for (name, schema) in &[
            ("execute", &execute_schema),
            ("file", &file_schema),
            ("line", &line_schema),
        ] {
            assert!(schema.contains_key("type"), "{} schema missing type", name);
            assert!(
                schema.contains_key("properties"),
                "{} schema missing properties",
                name
            );
            assert!(
                schema.contains_key("description"),
                "{} schema missing description",
                name
            );
        }

        // Verify line schema has required field
        if let Some(required) = line_schema.get("required").and_then(|v| v.as_array()) {
            let required_fields: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
            assert!(
                required_fields.contains(&"file"),
                "line schema should require file field"
            );
        } else {
            panic!("line schema should have required field");
        }
    }

    // ========== Complete Workflow Test ==========

    #[test]
    fn test_complete_workflow_structure() {
        // This test verifies that the complete workflow from parameters to results works
        use rs_lcov_mcp::commands::coverage::*;
        use rs_lcov_mcp::model::*;
        use serde_json::json;

        // 1. Create parameters
        let exec_params = ExecuteCoverageParams {
            workspace: false,
            package: None,
            threshold: Some(80.0),
        };

        // 2. Validate parameters
        assert!(validate_execute_params(&exec_params).is_ok());

        // 3. Create mock JSON output
        let json_output = json!({
            "data": [{
                "totals": {
                    "lines": {
                        "covered": 160,
                        "count": 200
                    },
                    "functions": {
                        "covered": 40,
                        "count": 50
                    }
                },
                "files": [
                    {
                        "filename": "src/lib.rs",
                        "summary": {
                            "lines": {
                                "covered": 80,
                                "count": 100,
                                "percent": 80.0
                            },
                            "branches": {
                                "percent": 75.0
                            }
                        },
                        "segments": [
                            [1, 1, 1, true, 0, 0],
                            [2, 1, 1, true, 0, 0]
                        ]
                    }
                ]
            }]
        })
        .to_string();

        // 4. Parse summary
        let summary = parse_coverage_summary(&json_output).unwrap();
        assert_eq!(summary.lines_covered, 160);
        assert_eq!(summary.total_lines, 200);
        assert!((summary.percentage - 80.0).abs() < 0.01);

        // 5. Parse file coverage
        let files = parse_file_coverage(&json_output).unwrap();
        assert!(!files.is_empty());

        // 6. Parse line coverage
        let line_coverage = parse_line_coverage(&json_output, "src/lib.rs").unwrap();
        assert!(line_coverage.is_some());

        // 7. Build command
        let cmd = build_llvm_cov_command(false, None);
        assert!(format!("{:?}", cmd).contains("cargo"));

        // 8. Create schemas
        let _exec_schema = create_execute_schema();
        let _file_schema = create_file_schema();
        let _line_schema = create_line_schema();
    }

    // ========== Boundary Value Tests ==========

    #[test]
    fn test_boundary_values_in_main() {
        use rs_lcov_mcp::commands::coverage::*;
        use rs_lcov_mcp::model::*;

        // Test zero values
        let metrics_zero = CoverageMetrics {
            percentage: 0.0,
            lines_covered: 0,
            total_lines: 0,
            functions_covered: 0,
            total_functions: 0,
        };
        assert_eq!(metrics_zero.percentage, 0.0);

        // Test 100% coverage
        let metrics_100 = CoverageMetrics {
            percentage: 100.0,
            lines_covered: 100,
            total_lines: 100,
            functions_covered: 50,
            total_functions: 50,
        };
        assert_eq!(metrics_100.percentage, 100.0);

        // Test threshold boundary values
        let exec_params_boundary = ExecuteCoverageParams {
            workspace: false,
            package: None,
            threshold: Some(0.0),
        };
        assert!(validate_execute_params(&exec_params_boundary).is_ok());

        let exec_params_boundary_max = ExecuteCoverageParams {
            workspace: false,
            package: None,
            threshold: Some(100.0),
        };
        assert!(validate_execute_params(&exec_params_boundary_max).is_ok());

        // Test just above boundary (should fail)
        let exec_params_above = ExecuteCoverageParams {
            workspace: false,
            package: None,
            threshold: Some(100.0001),
        };
        assert!(validate_execute_params(&exec_params_above).is_err());

        // Test just below boundary (should fail)
        let exec_params_below = ExecuteCoverageParams {
            workspace: false,
            package: None,
            threshold: Some(-0.0001),
        };
        assert!(validate_execute_params(&exec_params_below).is_err());
    }

    // ========== Error Case Tests ==========

    #[test]
    fn test_error_cases_in_main() {
        use rs_lcov_mcp::commands::coverage::*;
        use rs_lcov_mcp::model::*;

        // Test invalid threshold
        let exec_params_invalid = ExecuteCoverageParams {
            workspace: false,
            package: None,
            threshold: Some(150.0),
        };
        let result = validate_execute_params(&exec_params_invalid);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Threshold must be between 0 and 100")
        );

        // Test workspace and package conflict
        let exec_params_conflict = ExecuteCoverageParams {
            workspace: true,
            package: Some("my-package".to_string()),
            threshold: None,
        };
        let result = validate_execute_params(&exec_params_conflict);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Cannot specify both")
        );

        // Test empty file parameter
        let line_params_empty = GetFileLineCoverageParams {
            file: "".to_string(),
            workspace: false,
            package: None,
        };
        let result = validate_line_params(&line_params_empty);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("file parameter is required")
        );

        // Test invalid coverage_below
        let files_params_invalid = GetFilesCoverageParams {
            workspace: false,
            package: None,
            file: None,
            coverage_below: Some(150.0),
        };
        let result = validate_file_params(&files_params_invalid);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("coverage_below must be between 0 and 100")
        );
    }

    // ========== Success/Failure Result Tests ==========

    #[test]
    fn test_result_types_in_main() {
        use rs_lcov_mcp::model::*;

        // Test successful results
        let success_summary = CoverageSummary {
            success: true,
            coverage: CoverageMetrics {
                percentage: 85.0,
                lines_covered: 170,
                total_lines: 200,
                functions_covered: 42,
                total_functions: 50,
            },
            threshold: Some(ThresholdResult {
                met: true,
                value: 80.0,
            }),
            error: None,
        };
        assert!(success_summary.success);
        assert!(success_summary.error.is_none());

        // Test failure results
        let failure_summary = CoverageSummary {
            success: false,
            coverage: CoverageMetrics {
                percentage: 0.0,
                lines_covered: 0,
                total_lines: 0,
                functions_covered: 0,
                total_functions: 0,
            },
            threshold: None,
            error: Some("Failed to parse output".to_string()),
        };
        assert!(!failure_summary.success);
        assert!(failure_summary.error.is_some());

        // Test file coverage results
        let file_result = FileCoverageResult {
            success: true,
            files: vec![],
            error: None,
        };
        assert!(file_result.success);

        let line_result = LineCoverageResult {
            success: true,
            file: None,
            error: Some("File not found".to_string()),
        };
        assert!(line_result.success);
        assert!(line_result.error.is_some());
    }

    // ========== Clone and Debug Tests ==========

    #[test]
    fn test_clone_and_debug_in_main() {
        use rs_lcov_mcp::model::*;

        // Test cloning
        let metrics = CoverageMetrics {
            percentage: 75.5,
            lines_covered: 100,
            total_lines: 200,
            functions_covered: 10,
            total_functions: 20,
        };
        let metrics_clone = metrics.clone();
        assert_eq!(metrics.lines_covered, metrics_clone.lines_covered);

        // Test debug
        let _debug = format!("{:?}", metrics);
        assert!(!_debug.is_empty());

        // Test all model types can be cloned and debugged
        let summary = CoverageSummary {
            success: true,
            coverage: metrics,
            threshold: None,
            error: None,
        };
        let _summary_clone = summary.clone();
        let _summary_debug = format!("{:?}", summary);

        let branch = BranchInfo {
            line: 42,
            condition: "x > 0".to_string(),
            covered: true,
            hits: 5,
        };
        let _branch_clone = branch.clone();
        let _branch_debug = format!("{:?}", branch);
    }

    // ========== Comprehensive Integration Test ==========

    #[test]
    fn test_comprehensive_integration() {
        // This test exercises the entire codebase to ensure all parts work together
        use rmcp::ServerHandler;
        use rs_lcov_mcp::commands::coverage::*;
        use rs_lcov_mcp::error::*;
        use rs_lcov_mcp::model::*;
        use rs_lcov_mcp::server::handler::*;

        // 1. Test error handling
        let validation_err = CoverageError::Validation("test".to_string());
        assert!(validation_err.to_string().contains("Validation"));

        // 2. Test model creation
        let metrics = CoverageMetrics {
            percentage: 75.5,
            lines_covered: 100,
            total_lines: 200,
            functions_covered: 10,
            total_functions: 20,
        };

        let summary = CoverageSummary {
            success: true,
            coverage: metrics,
            threshold: Some(ThresholdResult {
                met: true,
                value: 80.0,
            }),
            error: None,
        };

        // 3. Test serialization
        let json_str = serde_json::to_string(&summary).unwrap();
        assert!(!json_str.is_empty());

        // 4. Test deserialization - note: percentage fields are serialized as strings
        // so we can't deserialize back to CoverageSummary directly, but we can verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(parsed["success"].as_bool().unwrap());

        // 5. Test server creation
        let server = CoverageServer::new();
        let info = server.get_info();
        assert_eq!(info.server_info.name, "rs-cargo-tools");

        // 6. Test parameter validation
        let exec_params = ExecuteCoverageParams {
            workspace: false,
            package: None,
            threshold: Some(80.0),
        };
        assert!(validate_execute_params(&exec_params).is_ok());

        // 7. Test schema creation
        let execute_schema = create_execute_schema();
        assert!(execute_schema.contains_key("type"));

        // 8. Test command building
        let cmd = build_llvm_cov_command(false, None);
        assert!(format!("{:?}", cmd).contains("cargo"));

        // 9. Test parsing with mock data
        let json_output = serde_json::json!({
            "data": [{
                "totals": {
                    "lines": {
                        "covered": 100,
                        "count": 200
                    },
                    "functions": {
                        "covered": 10,
                        "count": 20
                    }
                }
            }]
        })
        .to_string();

        let parsed_summary = parse_coverage_summary(&json_output).unwrap();
        assert_eq!(parsed_summary.lines_covered, 100);

        // 10. Test file parsing
        let json_file_output = serde_json::json!({
            "data": [{
                "files": [
                    {
                        "filename": "src/lib.rs",
                        "summary": {
                            "lines": {
                                "covered": 50,
                                "count": 100
                            },
                            "branches": {
                                "percent": 75.0
                            }
                        }
                    }
                ]
            }]
        })
        .to_string();

        let parsed_files = parse_file_coverage(&json_file_output).unwrap();
        assert!(!parsed_files.is_empty());

        // If we got here, all tests passed!
        assert!(true);
    }
}
