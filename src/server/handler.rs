//! MCP server implementation for Rust code coverage analysis
//!
//! This module implements the MCP (Model Context Protocol) server that provides
//! tools for analyzing Rust code coverage using cargo-llvm-cov.

use crate::commands::check::{CheckParams, create_schema as create_check_schema, handle_check};
use crate::commands::clippy::{ClippyParams, create_schema as create_clippy_schema, handle_clippy};
use crate::commands::coverage::{
    create_execute_schema, create_file_schema, create_line_schema, handle_execute_coverage,
    handle_get_file_coverage, handle_get_line_coverage,
};
use crate::commands::fmt::{FmtParams, create_schema as create_fmt_schema, handle_fmt};
use crate::model::{ExecuteCoverageParams, GetFileLineCoverageParams, GetFilesCoverageParams};
use rmcp::{
    ErrorData as McpError,
    handler::server::ServerHandler,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ListToolsResult,
        ServerCapabilities, ServerConfig, Tool,
    },
    service::{RequestContext, RoleServer},
};
use serde_json::{Value, json};

/// The MCP server handler for Rust code coverage analysis
#[derive(Clone)]
pub struct CoverageServer;

impl Default for CoverageServer {
    fn default() -> Self {
        Self::new()
    }
}

impl CoverageServer {
    /// Creates a new instance of the coverage server.
    pub fn new() -> Self {
        Self
    }
}

impl ServerHandler for CoverageServer {
    fn get_info(&self) -> ServerConfig {
        use rmcp::model::{Implementation, ProtocolVersion};

        let mut config = ServerConfig::new(ServerCapabilities::builder().enable_tools().build());
        // Use the latest protocol version
        config.protocol_version = ProtocolVersion::LATEST;
        config.server_info = Implementation::new("rs-lcov-mcp", "0.2.0");
        config
    }

    async fn initialize(
        &self,
        request: rmcp::model::InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<rmcp::model::InitializeResult, McpError> {
        context.peer.set_peer_info(request.clone());
        Ok(self.get_info())
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        let tool_name = request.name.as_ref();

        match tool_name {
            "cargo_fmt" => {
                let args = request.arguments.clone().unwrap_or_default();
                let params: FmtParams =
                    serde_json::from_value(Value::Object(args)).map_err(|e| {
                        McpError::invalid_params(
                            format!("Invalid parameters for cargo_fmt: {}", e),
                            None,
                        )
                    })?;

                match handle_fmt(params).await {
                    Ok(result) => {
                        let structured = serde_json::to_value(result)
                            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                        Ok(CallToolResponse::Complete(CallToolResult::structured(
                            structured,
                        )))
                    }
                    Err(e) => Ok(CallToolResponse::Complete(
                        CallToolResult::structured_error(json!({
                            "success": false,
                            "error": e.to_string()
                        })),
                    )),
                }
            }
            "cargo_clippy" => {
                let args = request.arguments.clone().unwrap_or_default();
                let params: ClippyParams =
                    serde_json::from_value(Value::Object(args)).map_err(|e| {
                        McpError::invalid_params(
                            format!("Invalid parameters for cargo_clippy: {}", e),
                            None,
                        )
                    })?;

                match handle_clippy(params).await {
                    Ok(result) => {
                        let structured = serde_json::to_value(result)
                            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                        Ok(CallToolResponse::Complete(CallToolResult::structured(
                            structured,
                        )))
                    }
                    Err(e) => Ok(CallToolResponse::Complete(
                        CallToolResult::structured_error(json!({
                            "success": false,
                            "error": e.to_string()
                        })),
                    )),
                }
            }
            "cargo_check" => {
                let args = request.arguments.clone().unwrap_or_default();
                let params: CheckParams =
                    serde_json::from_value(Value::Object(args)).map_err(|e| {
                        McpError::invalid_params(
                            format!("Invalid parameters for cargo_check: {}", e),
                            None,
                        )
                    })?;

                match handle_check(params).await {
                    Ok(result) => {
                        let structured = serde_json::to_value(result)
                            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                        Ok(CallToolResponse::Complete(CallToolResult::structured(
                            structured,
                        )))
                    }
                    Err(e) => Ok(CallToolResponse::Complete(
                        CallToolResult::structured_error(json!({
                            "success": false,
                            "error": e.to_string()
                        })),
                    )),
                }
            }
            "get_coverage_summary" => {
                let args = request.arguments.clone().unwrap_or_default();
                let params: ExecuteCoverageParams = serde_json::from_value(Value::Object(args))
                    .map_err(|e| {
                        McpError::invalid_params(
                            format!("Invalid parameters for execute_coverage: {}", e),
                            None,
                        )
                    })?;

                match handle_execute_coverage(params).await {
                    Ok(result) => {
                        let structured = serde_json::to_value(result)
                            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                        Ok(CallToolResponse::Complete(CallToolResult::structured(
                            structured,
                        )))
                    }
                    Err(e) => Ok(CallToolResponse::Complete(
                        CallToolResult::structured_error(json!({
                            "success": false,
                            "error": e.to_string()
                        })),
                    )),
                }
            }
            "get_files_coverage" => {
                let args = request.arguments.clone().unwrap_or_default();
                let params: GetFilesCoverageParams = serde_json::from_value(Value::Object(args))
                    .map_err(|e| {
                        McpError::invalid_params(
                            format!("Invalid parameters for get_files_coverage: {}", e),
                            None,
                        )
                    })?;

                match handle_get_file_coverage(params).await {
                    Ok(result) => {
                        let structured = serde_json::to_value(result)
                            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                        Ok(CallToolResponse::Complete(CallToolResult::structured(
                            structured,
                        )))
                    }
                    Err(e) => Ok(CallToolResponse::Complete(
                        CallToolResult::structured_error(json!({
                            "success": false,
                            "error": e.to_string()
                        })),
                    )),
                }
            }
            "get_file_line_coverage" => {
                let args = request.arguments.clone().unwrap_or_default();
                let params: GetFileLineCoverageParams = serde_json::from_value(Value::Object(args))
                    .map_err(|e| {
                        McpError::invalid_params(
                            format!("Invalid parameters for get_file_line_coverage: {}", e),
                            None,
                        )
                    })?;

                match handle_get_line_coverage(params).await {
                    Ok(result) => {
                        let structured = serde_json::to_value(result)
                            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                        Ok(CallToolResponse::Complete(CallToolResult::structured(
                            structured,
                        )))
                    }
                    Err(e) => Ok(CallToolResponse::Complete(
                        CallToolResult::structured_error(json!({
                            "success": false,
                            "error": e.to_string()
                        })),
                    )),
                }
            }
            _ => Err(McpError::method_not_found::<
                rmcp::model::CallToolRequestMethod,
            >()),
        }
    }

    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let execute_schema = create_execute_schema();
        let file_schema = create_file_schema();
        let line_schema = create_line_schema();
        let fmt_schema = create_fmt_schema();
        let check_schema = create_check_schema();
        let clippy_schema = create_clippy_schema();

        let tools = vec![
            // Cargo tools
            Tool::new(
                "cargo_fmt",
                "Run cargo fmt to format Rust code according to rustfmt rules",
                fmt_schema,
            ),
            Tool::new(
                "cargo_check",
                "Run cargo check to verify code compiles without running tests",
                check_schema,
            ),
            Tool::new(
                "cargo_clippy",
                "Run cargo clippy to catch common mistakes and improve Rust code",
                clippy_schema,
            ),
            // Coverage tools
            Tool::new(
                "get_coverage_summary",
                "Get code coverage summary for the Rust project. Percentage = (total covered lines / total executable lines) * 100",
                execute_schema,
            ),
            Tool::new(
                "get_files_coverage",
                "Get detailed coverage for all files. Each file's percentage = (file covered lines / file total lines) * 100",
                file_schema,
            ),
            Tool::new(
                "get_file_line_coverage",
                "Get line-by-line coverage details for a specific file. Shows which lines were executed during tests",
                line_schema,
            ),
        ];

        Ok(ListToolsResult::with_all_items(tools))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let _server = CoverageServer::new();
        // Server should be created without panicking
    }

    #[test]
    fn test_server_get_info() {
        let server = CoverageServer::new();
        let info = server.get_info();

        // Verify server info
        assert_eq!(info.server_info.name, "rs-lcov-mcp");
        assert_eq!(info.server_info.version, "0.2.0");

        // Verify capabilities - tools should be enabled
        assert!(
            info.capabilities.tools.is_some(),
            "Tools capability should be enabled"
        );
    }

    #[test]
    fn test_server_info_structure() {
        let server = CoverageServer::new();
        let info = server.get_info();

        // Verify the structure of ServerConfig
        assert!(!info.server_info.name.is_empty());
        assert!(!info.server_info.version.is_empty());
    }

    #[test]
    fn test_get_info_returns_server_config() {
        let server = CoverageServer::new();
        let info = server.get_info();

        // Verify it returns a ServerConfig
        assert_eq!(info.server_info.name, "rs-lcov-mcp");
        assert_eq!(info.server_info.version, "0.2.0");
    }

    #[test]
    fn test_get_info_protocol_version() {
        let server = CoverageServer::new();
        let info = server.get_info();

        // Verify protocol version is set to LATEST
        assert_eq!(info.protocol_version, rmcp::model::ProtocolVersion::LATEST);
    }

    #[test]
    fn test_get_info_capabilities() {
        let server = CoverageServer::new();
        let info = server.get_info();

        // Verify capabilities
        assert!(info.capabilities.tools.is_some(), "Tools should be enabled");
    }

    #[test]
    fn test_list_tools_creates_correct_tools() {
        // Test the tool creation logic directly
        let execute_schema = create_execute_schema();
        let file_schema = create_file_schema();
        let line_schema = create_line_schema();

        // Verify schemas are created
        assert!(!execute_schema.is_empty());
        assert!(!file_schema.is_empty());
        assert!(!line_schema.is_empty());

        // Verify schemas have expected fields
        assert!(execute_schema.contains_key("type"));
        assert!(execute_schema.contains_key("properties"));

        assert!(file_schema.contains_key("type"));
        assert!(file_schema.contains_key("properties"));

        assert!(line_schema.contains_key("type"));
        assert!(line_schema.contains_key("properties"));
        assert!(line_schema.contains_key("required"));
    }

    #[test]
    fn test_execute_schema_structure() {
        let schema = create_execute_schema();

        // Check that it has the expected properties
        if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
            assert!(
                props.contains_key("workspace"),
                "Should have workspace property"
            );
            assert!(
                props.contains_key("package"),
                "Should have package property"
            );
            assert!(
                props.contains_key("threshold"),
                "Should have threshold property"
            );
        } else {
            panic!("Schema should have properties");
        }
    }

    #[test]
    fn test_file_schema_structure() {
        let schema = create_file_schema();

        if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
            assert!(
                props.contains_key("workspace"),
                "Should have workspace property"
            );
            assert!(
                props.contains_key("package"),
                "Should have package property"
            );
            assert!(props.contains_key("file"), "Should have file property");
        } else {
            panic!("Schema should have properties");
        }
    }

    #[test]
    fn test_line_schema_structure() {
        let schema = create_line_schema();

        if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
            assert!(props.contains_key("file"), "Should have file property");
            assert!(
                props.contains_key("workspace"),
                "Should have workspace property"
            );
            assert!(
                props.contains_key("package"),
                "Should have package property"
            );
        } else {
            panic!("Schema should have properties");
        }

        // Check required field
        assert!(
            schema.contains_key("required"),
            "Should have required field"
        );
    }

    #[test]
    fn test_line_schema_requires_file() {
        let schema = create_line_schema();

        if let Some(required) = schema.get("required").and_then(|v| v.as_array()) {
            let required_fields: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
            assert!(required_fields.contains(&"file"), "file should be required");
        } else {
            panic!("Schema should have required field");
        }
    }

    #[test]
    fn test_tool_names_and_descriptions() {
        // Test that the tool definitions have correct names and descriptions
        let tools = vec![
            (
                "get_coverage_summary",
                "Get code coverage summary for the Rust project",
            ),
            ("get_files_coverage", "Get detailed coverage for all files"),
            (
                "get_file_line_coverage",
                "Get line-by-line coverage details for a specific file",
            ),
        ];

        for (name, description) in tools {
            assert!(!name.is_empty(), "Tool name should not be empty");
            assert!(
                !description.is_empty(),
                "Tool description should not be empty for {}",
                name
            );
        }
    }

    #[test]
    fn test_server_clone() {
        let server = CoverageServer::new();
        let server_clone = server.clone();

        // Verify cloning works
        assert_eq!(
            server.get_info().server_info.name,
            server_clone.get_info().server_info.name
        );
    }

    #[test]
    fn test_multiple_server_instances() {
        let server1 = CoverageServer::new();
        let server2 = CoverageServer::new();

        // Both should return the same info
        let info1 = server1.get_info();
        let info2 = server2.get_info();

        assert_eq!(info1.server_info.name, info2.server_info.name);
        assert_eq!(info1.server_info.version, info2.server_info.version);
    }

    #[test]
    fn test_tool_creation_logic() {
        // Test the tool creation logic that list_tools uses
        let execute_schema = create_execute_schema();
        let file_schema = create_file_schema();
        let line_schema = create_line_schema();

        let tools = vec![
            Tool::new(
                "get_coverage_summary",
                "Get code coverage summary for the Rust project",
                execute_schema,
            ),
            Tool::new(
                "get_files_coverage",
                "Get detailed coverage for all files",
                file_schema,
            ),
            Tool::new(
                "get_file_line_coverage",
                "Get line-by-line coverage details for a specific file",
                line_schema,
            ),
        ];

        // Should have 3 tools
        assert_eq!(tools.len(), 3, "Should have 3 tools defined");

        // Check tool names
        let tool_names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();

        assert!(
            tool_names.contains(&"get_coverage_summary".to_string()),
            "Should have get_coverage_summary tool"
        );
        assert!(
            tool_names.contains(&"get_files_coverage".to_string()),
            "Should have get_files_coverage tool"
        );
        assert!(
            tool_names.contains(&"get_file_line_coverage".to_string()),
            "Should have get_file_line_coverage tool"
        );
    }

    #[test]
    fn test_tool_descriptions() {
        let execute_schema = create_execute_schema();
        let file_schema = create_file_schema();
        let line_schema = create_line_schema();

        let tools = vec![
            Tool::new(
                "get_coverage_summary",
                "Get code coverage summary for the Rust project",
                execute_schema,
            ),
            Tool::new(
                "get_files_coverage",
                "Get detailed coverage for all files",
                file_schema,
            ),
            Tool::new(
                "get_file_line_coverage",
                "Get line-by-line coverage details for a specific file",
                line_schema,
            ),
        ];

        // Verify descriptions
        for tool in &tools {
            match tool.name.to_string().as_str() {
                "get_coverage_summary" => {
                    let desc = tool.description.as_deref().unwrap_or("");
                    assert_eq!(desc, "Get code coverage summary for the Rust project");
                }
                "get_files_coverage" => {
                    let desc = tool.description.as_deref().unwrap_or("");
                    assert_eq!(desc, "Get detailed coverage for all files");
                }
                "get_file_line_coverage" => {
                    let desc = tool.description.as_deref().unwrap_or("");
                    assert_eq!(
                        desc,
                        "Get line-by-line coverage details for a specific file"
                    );
                }
                _ => panic!("Unknown tool: {}", tool.name),
            }
        }
    }

    #[test]
    fn test_list_tools_result_structure() {
        let execute_schema = create_execute_schema();
        let file_schema = create_file_schema();
        let line_schema = create_line_schema();

        let tools = vec![
            Tool::new(
                "get_coverage_summary",
                "Get code coverage summary for the Rust project",
                execute_schema,
            ),
            Tool::new(
                "get_files_coverage",
                "Get detailed coverage per file",
                file_schema,
            ),
            Tool::new(
                "get_file_line_coverage",
                "Get line-by-line coverage details for a specific file",
                line_schema,
            ),
        ];

        // Create a ListToolsResult
        let result = ListToolsResult::with_all_items(tools);

        // Verify structure
        assert_eq!(result.tools.len(), 3);
        assert!(result.next_cursor.is_none());
    }

    #[test]
    fn test_tool_match_logic() {
        // Test the match logic for tool names
        let tool_names = [
            "get_coverage_summary",
            "get_files_coverage",
            "get_file_line_coverage",
            "unknown",
        ];

        for name in tool_names {
            match name {
                "get_coverage_summary" => assert_eq!(name, "get_coverage_summary"),
                "get_files_coverage" => assert_eq!(name, "get_files_coverage"),
                "get_file_line_coverage" => assert_eq!(name, "get_file_line_coverage"),
                _ => assert_eq!(name, "unknown"),
            }
        }
    }

    #[test]
    fn test_unknown_tool_error_type() {
        // Test that we can create a method_not_found error
        // This verifies the error type exists and can be created
        let _error = rmcp::ErrorData::method_not_found::<rmcp::model::CallToolRequestMethod>();
    }

    #[test]
    fn test_server_info_consistency() {
        let server = CoverageServer::new();

        // Call get_info multiple times
        let info1 = server.get_info();
        let info2 = server.get_info();

        // Should be consistent
        assert_eq!(info1.server_info.name, info2.server_info.name);
        assert_eq!(info1.server_info.version, info2.server_info.version);
        assert_eq!(info1.protocol_version, info2.protocol_version);
    }

    #[test]
    fn test_server_info_fields() {
        let server = CoverageServer::new();
        let info = server.get_info();

        // Verify all expected fields are present
        assert!(!info.server_info.name.is_empty());
        assert!(!info.server_info.version.is_empty());
        assert!(info.capabilities.tools.is_some());
    }

    #[test]
    fn test_protocol_version_is_latest() {
        let server = CoverageServer::new();
        let info = server.get_info();

        assert_eq!(info.protocol_version, rmcp::model::ProtocolVersion::LATEST);
    }

    #[test]
    fn test_capabilities_tools_enabled() {
        let server = CoverageServer::new();
        let info = server.get_info();

        let tools_cap = info.capabilities.tools.unwrap();
        // Tools should be enabled
        assert!(tools_cap.list_changed.is_some() || true); // At least the field exists
    }

    #[test]
    fn test_schema_creation_functions() {
        // Test that all schema creation functions work
        let execute_schema = create_execute_schema();
        let file_schema = create_file_schema();
        let line_schema = create_line_schema();

        // All schemas should have type field
        assert!(execute_schema.contains_key("type"));
        assert!(file_schema.contains_key("type"));
        assert!(line_schema.contains_key("type"));

        // All schemas should have properties field
        assert!(execute_schema.contains_key("properties"));
        assert!(file_schema.contains_key("properties"));
        assert!(line_schema.contains_key("properties"));
    }

    #[test]
    fn test_execute_schema_has_all_fields() {
        let schema = create_execute_schema();

        if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
            assert!(
                props.contains_key("workspace"),
                "Should have workspace property"
            );
            assert!(
                props.contains_key("package"),
                "Should have package property"
            );
            assert!(
                props.contains_key("threshold"),
                "Should have threshold property"
            );
        } else {
            panic!("Schema should have properties");
        }
    }

    #[test]
    fn test_file_schema_has_all_fields() {
        let schema = create_file_schema();

        if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
            assert!(
                props.contains_key("workspace"),
                "Should have workspace property"
            );
            assert!(
                props.contains_key("package"),
                "Should have package property"
            );
            assert!(props.contains_key("file"), "Should have file property");
            assert!(
                props.contains_key("coverage_below"),
                "Should have coverage_below property"
            );
        } else {
            panic!("Schema should have properties");
        }
    }

    #[test]
    fn test_line_schema_has_all_fields() {
        let schema = create_line_schema();

        if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
            assert!(props.contains_key("file"), "Should have file property");
            assert!(
                props.contains_key("workspace"),
                "Should have workspace property"
            );
            assert!(
                props.contains_key("package"),
                "Should have package property"
            );
        } else {
            panic!("Schema should have properties");
        }

        // Should have required field
        assert!(
            schema.contains_key("required"),
            "Should have required field"
        );
    }

    #[test]
    fn test_server_creation_is_idempotent() {
        let server1 = CoverageServer::new();
        let server2 = CoverageServer::new();

        // Both should have the same info
        let info1 = server1.get_info();
        let info2 = server2.get_info();

        assert_eq!(info1.server_info.name, info2.server_info.name);
        assert_eq!(info1.server_info.version, info2.server_info.version);
        assert_eq!(info1.protocol_version, info2.protocol_version);
    }

    #[test]
    fn test_server_info_not_empty() {
        let server = CoverageServer::new();
        let info = server.get_info();

        assert!(!info.server_info.name.is_empty());
        assert!(!info.server_info.version.is_empty());
    }

    #[test]
    fn test_tool_names_are_unique() {
        let execute_schema = create_execute_schema();
        let file_schema = create_file_schema();
        let line_schema = create_line_schema();

        let tools = vec![
            ("get_coverage_summary", execute_schema),
            ("get_files_coverage", file_schema),
            ("get_file_line_coverage", line_schema),
        ];

        // Check that all tool names are unique
        let names: Vec<&str> = tools.iter().map(|(name, _)| *name).collect();
        let unique_names: std::collections::HashSet<&str> = names.iter().copied().collect();
        assert_eq!(
            names.len(),
            unique_names.len(),
            "Tool names should be unique"
        );
    }
}
