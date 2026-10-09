//! Error handling module for the coverage MCP server
//!
//! This module defines custom error types and conversions for coverage analysis errors
//! and cargo command execution errors.

use thiserror::Error;

/// Custom error types for the coverage MCP server
#[derive(Debug, Error)]
pub enum CoverageError {
    /// Validation error for input parameters
    #[error("Validation error: {0}")]
    Validation(String),

    /// Error when cargo-llvm-cov command execution fails
    #[error("cargo-llvm-cov execution failed: {0}")]
    ExecutionFailed(String),

    /// Error when parsing cargo-llvm-cov JSON output fails
    #[error("Failed to parse cargo-llvm-cov output: {0}")]
    ParseError(String),
}

/// Error types for cargo command execution
#[derive(Debug, Error)]
pub enum CommandError {
    /// Validation error for input parameters
    #[error("Validation error: {0}")]
    Validation(String),

    /// Error when cargo command execution fails
    #[error("Command execution failed: {0}")]
    ExecutionFailed(String),

    /// Error when parsing command output fails
    #[error("Failed to parse command output: {0}")]
    ParseError(String),

    /// Error when a required tool is not installed
    #[error("Required tool not installed: {0}")]
    ToolNotInstalled(String),
}

/// Convenience type alias for CoverageError results
pub type Result<T> = std::result::Result<T, CoverageError>;

/// Convenience type alias for CommandError results
pub type CommandResult<T> = std::result::Result<T, CommandError>;

impl From<std::io::Error> for CoverageError {
    fn from(err: std::io::Error) -> Self {
        CoverageError::ExecutionFailed(err.to_string())
    }
}

impl From<serde_json::Error> for CoverageError {
    fn from(err: serde_json::Error) -> Self {
        CoverageError::ParseError(err.to_string())
    }
}

impl From<String> for CoverageError {
    fn from(err: String) -> Self {
        CoverageError::ExecutionFailed(err)
    }
}

// Implement From traits for CommandError
impl From<std::io::Error> for CommandError {
    fn from(err: std::io::Error) -> Self {
        CommandError::ExecutionFailed(err.to_string())
    }
}

impl From<serde_json::Error> for CommandError {
    fn from(err: serde_json::Error) -> Self {
        CommandError::ParseError(err.to_string())
    }
}

impl From<String> for CommandError {
    fn from(err: String) -> Self {
        CommandError::ExecutionFailed(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_coverage_error_variants() {
        let validation_err = CoverageError::Validation("Invalid input".to_string());
        assert!(validation_err.to_string().contains("Validation error"));

        let execution_err = CoverageError::ExecutionFailed("Command failed".to_string());
        assert!(
            execution_err
                .to_string()
                .contains("cargo-llvm-cov execution failed")
        );

        let parse_err = CoverageError::ParseError("Invalid JSON".to_string());
        assert!(parse_err.to_string().contains("Failed to parse"));
    }

    #[test]
    fn test_from_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let coverage_err: CoverageError = io_err.into();

        assert!(matches!(coverage_err, CoverageError::ExecutionFailed(_)));
        assert!(coverage_err.to_string().contains("File not found"));
    }

    #[test]
    fn test_from_serde_json_error() {
        // Create a serde_json error by attempting to parse invalid JSON
        let json_err: serde_json::Error =
            serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let coverage_err: CoverageError = json_err.into();

        assert!(matches!(coverage_err, CoverageError::ParseError(_)));
    }

    #[test]
    fn test_from_string() {
        let string_err = "Something went wrong".to_string();
        let coverage_err: CoverageError = string_err.into();

        assert!(matches!(coverage_err, CoverageError::ExecutionFailed(_)));
        assert!(coverage_err.to_string().contains("Something went wrong"));
    }

    #[test]
    fn test_result_type() {
        let success: Result<i32> = Ok(42);
        assert!(success.is_ok());
        assert_eq!(success.unwrap(), 42);

        let error: Result<i32> = Err(CoverageError::Validation("Test error".to_string()));
        assert!(error.is_err());
    }

    #[test]
    fn test_error_display() {
        let err = CoverageError::Validation("test validation error".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Validation error"));
        assert!(display.contains("test validation error"));
    }

    #[test]
    fn test_error_debug() {
        let err = CoverageError::ExecutionFailed("test execution error".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("ExecutionFailed"));
    }

    #[test]
    fn test_error_equality() {
        let err1 = CoverageError::Validation("error".to_string());
        let err2 = CoverageError::Validation("error".to_string());

        // Errors with same content should be equal
        assert!(format!("{}", err1) == format!("{}", err2));
    }

    // ========== CommandError Tests ==========

    #[test]
    fn test_command_error_variants() {
        let validation_err = CommandError::Validation("Invalid input".to_string());
        assert!(validation_err.to_string().contains("Validation error"));

        let execution_err = CommandError::ExecutionFailed("Command failed".to_string());
        assert!(
            execution_err
                .to_string()
                .contains("Command execution failed")
        );

        let parse_err = CommandError::ParseError("Invalid JSON".to_string());
        assert!(parse_err.to_string().contains("Failed to parse"));

        let tool_err = CommandError::ToolNotInstalled("cargo-fmt not found".to_string());
        assert!(tool_err.to_string().contains("Required tool not installed"));
    }

    #[test]
    fn test_command_error_from_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let command_err: CommandError = io_err.into();

        assert!(matches!(command_err, CommandError::ExecutionFailed(_)));
        assert!(command_err.to_string().contains("File not found"));
    }

    #[test]
    fn test_command_error_from_serde_json_error() {
        let json_err: serde_json::Error =
            serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let command_err: CommandError = json_err.into();

        assert!(matches!(command_err, CommandError::ParseError(_)));
    }

    #[test]
    fn test_command_error_from_string() {
        let string_err = "Something went wrong".to_string();
        let command_err: CommandError = string_err.into();

        assert!(matches!(command_err, CommandError::ExecutionFailed(_)));
        assert!(command_err.to_string().contains("Something went wrong"));
    }

    #[test]
    fn test_command_result_type() {
        let success: CommandResult<i32> = Ok(42);
        assert!(success.is_ok());
        assert_eq!(success.unwrap(), 42);

        let error: CommandResult<i32> = Err(CommandError::Validation("Test error".to_string()));
        assert!(error.is_err());
    }

    #[test]
    fn test_command_error_display() {
        let err = CommandError::Validation("test validation error".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Validation error"));
        assert!(display.contains("test validation error"));
    }

    #[test]
    fn test_command_error_debug() {
        let err = CommandError::ExecutionFailed("test execution error".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("ExecutionFailed"));
    }
}
