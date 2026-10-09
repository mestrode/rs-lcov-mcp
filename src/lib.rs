//! rs-cov-mcp: MCP server for Rust code coverage analysis
//!
//! This is a library crate that provides the core functionality for the MCP server.
//! It exposes modules for coverage analysis and MCP server handling.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(clippy::needless_doctest_main)]

/// Coverage analysis module for cargo-llvm-cov integration
pub mod coverage;

/// Error handling module for the coverage MCP server
pub mod error;

/// Data models for coverage analysis
pub mod model;

/// MCP server implementation for Rust code coverage analysis
pub mod mcp;

/// Re-export commonly used types for convenience
pub use error::{CoverageError, Result};
pub use mcp::CoverageServer;
pub use model::*;
