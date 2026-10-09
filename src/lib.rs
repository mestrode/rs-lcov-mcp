//! rs-cov-mcp: MCP server for Rust code coverage analysis
//!
//! This is a library crate that provides the core functionality for the MCP server.
//! It includes modules for coverage analysis and MCP server handling.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(clippy::needless_doctest_main)]

/// Coverage analysis module for cargo-llvm-cov integration
pub mod coverage;

/// Error handling module for the coverage MCP server
pub mod error;

/// Data models for coverage analysis
pub mod model;

/// Server module containing MCP server implementation
pub mod server;

/// MCP server implementation for Rust code coverage analysis
///
/// @deprecated Use `server` module instead
pub mod mcp {
    pub use crate::server::*;
}

/// Re-export commonly used types for convenience
pub use error::{CoverageError, Result};
pub use model::*;
pub use server::handler::CoverageServer;
