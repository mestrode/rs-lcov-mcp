//! LLVM coverage execution handler
//!
//! This module provides the execution logic for cargo-llvm-cov commands.

use crate::error::{CoverageError, Result};
use std::process::Command;

/// Build cargo llvm-cov command with parameters
pub fn build_llvm_cov_command(workspace: bool, package: Option<&str>) -> Command {
    let mut cmd = Command::new("cargo");
    cmd.arg("llvm-cov");

    if workspace {
        cmd.arg("--workspace");
    } else if let Some(pkg) = package {
        cmd.args(["--package", pkg]);
    }

    cmd.arg("--json");

    cmd
}

/// Execute cargo llvm-cov and return the JSON output
pub fn execute_llvm_cov(workspace: bool, package: Option<&str>) -> Result<String> {
    let mut cmd = build_llvm_cov_command(workspace, package);

    let output = cmd.output().map_err(|e| {
        let error_string = e.to_string();
        if error_string.contains("not found") || error_string.contains("No such file or directory") {
            CoverageError::ExecutionFailed(
                "Error: cargo-llvm-cov is not installed. Install it with: `cargo install cargo-llvm-cov`".to_string()
            )
        } else {
            CoverageError::ExecutionFailed(format!(
                "Failed to execute cargo llvm-cov with workspace={}, package={:?}: {}\nSuggestion: Run `cargo check` and `cargo test` first to ensure your project compiles and tests pass before running coverage analysis.",
                workspace, package, e
            ))
        }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr_str = stderr.to_string();

        // Check if the error indicates cargo-llvm-cov is not installed
        if stderr_str.contains("not found")
            || stderr_str.contains("cargo-llvm-cov") && stderr_str.contains("not installed")
        {
            return Err(CoverageError::ExecutionFailed(
                "Error: cargo-llvm-cov is not installed. Install it with: `cargo install cargo-llvm-cov`".to_string()
            ));
        }

        return Err(CoverageError::ExecutionFailed(format!(
            "cargo llvm-cov failed: {}\nSuggestion: Run `cargo check` and `cargo test` first to ensure your project compiles and tests pass before running coverage analysis.",
            stderr
        )));
    }

    // cargo-llvm-cov outputs JSON to stderr in some versions
    // Combine both stdout and stderr to get the full JSON output
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);

    // Try to parse stdout first, if it's valid JSON
    if !stdout_str.trim().is_empty() {
        return Ok(stdout_str.into_owned());
    }

    // If stdout is empty, try stderr
    if !stderr_str.trim().is_empty() {
        return Ok(stderr_str.into_owned());
    }

    // If both are empty, return an error
    Err(CoverageError::ParseError(
        "No output from cargo llvm-cov".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_llvm_cov_command_workspace() {
        let cmd = build_llvm_cov_command(true, None);

        // Check that the command includes --workspace
        let cmd_str = format!("{:?}", cmd);
        assert!(
            cmd_str.contains("--workspace"),
            "Should include --workspace flag"
        );
    }

    #[test]
    fn test_build_llvm_cov_command_package() {
        let cmd = build_llvm_cov_command(false, Some("my-package"));

        // Check that the command includes --package
        let cmd_str = format!("{:?}", cmd);
        assert!(
            cmd_str.contains("--package"),
            "Should include --package flag"
        );
        assert!(
            cmd_str.contains("my-package"),
            "Should include package name"
        );
    }

    #[test]
    fn test_build_llvm_cov_command_default() {
        let cmd = build_llvm_cov_command(false, None);

        // Check that the command includes cargo llvm-cov
        let cmd_str = format!("{:?}", cmd);
        assert!(cmd_str.contains("cargo"), "Should include cargo command");
        assert!(
            cmd_str.contains("llvm-cov"),
            "Should include llvm-cov subcommand"
        );
        assert!(cmd_str.contains("--json"), "Should include --json flag");
    }
}
