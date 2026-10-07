//! rs-lcov-mcp: MCP server for Rust code coverage analysis using cargo-llvm-cov
//!
//! This is an MCP (Model Context Protocol) server that provides tools for analyzing
//! Rust code coverage. It uses cargo-llvm-cov under the hood to generate coverage
//! reports and exposes them through the MCP protocol.
//!
//! # Usage
//!
//! The server can be run as a standalone process and communicates via stdio.
//! It provides two main tools:
//! - `execute_coverage`: Get code coverage summary for a Rust project
//! - `get_file_coverage`: Get detailed branch coverage per file

pub mod coverage;
pub mod error;
pub mod mcp;
pub mod model;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modules_are_accessible() {
        // Verify that all modules can be accessed
        let _coverage = coverage::parse_coverage_summary;
        let _error = error::CoverageError::Validation("test".to_string());
        let _mcp = mcp::CoverageServer::new();
        let _model = model::CoverageSummary {
            success: true,
            coverage: model::CoverageMetrics {
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
        let server = mcp::CoverageServer::new();
        let info = server.get_info();
        assert_eq!(info.server_info.name, "rs-lcov-mcp");
    }
}

use mcp::CoverageServer;
use rmcp::ServiceExt;
use rmcp::transport::stdio;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create the server handler
    let server = CoverageServer::new();

    // Serve using stdio transport and wait for the service to complete
    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}
