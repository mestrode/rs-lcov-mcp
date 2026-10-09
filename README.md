# rust-tools-mcp

[![Coverage Status](https://img.shields.io/badge/coverage-78.44%25-brightgreen)](https://github.com/mestrode/rust-tools-mcp)

An MCP (Model Context Protocol) server for Rust development tools. It provides cargo command execution (fmt, check, clippy, test, build) and code coverage analysis using [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov).

## Features

### Cargo Commands
- **cargo_fmt**: Format Rust code according to rustfmt rules
- **cargo_check**: Verify code compiles without running tests
- **cargo_clippy**: Catch common mistakes and improve Rust code
- **cargo_test**: Execute unit and integration tests
- **cargo_build**: Compile your Rust project

### Coverage Analysis (requires cargo-llvm-cov)
- **Code Coverage Summary**: Get overall coverage percentage for your Rust project
- **Per-File Coverage**: Detailed coverage breakdown for each file
- **Line-by-Line Coverage**: See exactly which lines are covered by tests
- **Threshold Checking**: Set minimum coverage thresholds and check if they're met
- **Filtering**: Filter files by coverage percentage

## Installation

### Prerequisites

1. Install [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) (for coverage tools only):
   ```bash
   cargo install cargo-llvm-cov
   ```

2. Ensure you have Rust and Cargo installed (1.60+ recommended)

### Build

```bash
# Clone the repository
git clone https://github.com/mestrode/rust-tools-mcp
cd rust-tools-mcp

# Build in release mode
cargo build --release

# The binary will be at target/release/rust-tools-mcp
```

## Usage

### As an MCP Server

The server communicates via stdio and provides the following tools:

#### Cargo Tools

##### `cargo_fmt`

Format Rust code according to rustfmt rules.

**Parameters:**
- `check` (boolean, optional): Run in check mode (no changes, just verify formatting). Default: `false`
- `verbose` (boolean, optional): Show verbose output. Default: `false`
- `package` (string, optional): Specific package to format
- `workspace` (boolean, optional): Format entire workspace

**Returns:**
- `success`: Whether the command executed successfully
- `formatted`: Whether any files were formatted
- `output`: Command output
- `error`: Error message if the command failed

##### `cargo_check`

Verify that your code compiles without running tests.

**Parameters:**
- `package` (string, optional): Specific package to check
- `workspace` (boolean, optional): Check entire workspace. Default: `false`
- `all_targets` (boolean, optional): Check all targets. Default: `false`
- `all_features` (boolean, optional): Build with all features enabled. Default: `false`
- `no_default_features` (boolean, optional): Build without default features. Default: `false`
- `features` (array of strings, optional): List of features to enable

**Returns:**
- `success`: Whether the command executed successfully
- `warnings`: Number of warnings found
- `errors`: Number of errors found
- `output`: Command output
- `error`: Error message if the command failed

##### `cargo_clippy`

Run the Clippy linter to catch common mistakes and improve your Rust code.

**Parameters:**
- `package` (string, optional): Specific package to lint
- `workspace` (boolean, optional): Lint entire workspace. Default: `false`
- `all_targets` (boolean, optional): Lint all targets. Default: `false`
- `all_features` (boolean, optional): Build with all features enabled. Default: `false`
- `no_default_features` (boolean, optional): Build without default features. Default: `false`
- `features` (array of strings, optional): List of features to enable
- `warnings_as_errors` (boolean, optional): Treat warnings as errors. Default: `false`
- `lints` (array of strings, optional): List of lints to enable or configure

**Returns:**
- `success`: Whether the command executed successfully
- `warnings`: Number of warnings found
- `errors`: Number of errors found
- `lints_found`: List of lint names that were found
- `output`: Command output
- `error`: Error message if the command failed

##### `cargo_test`

Execute unit and integration tests.

**Parameters:**
- `package` (string, optional): Specific package to test
- `workspace` (boolean, optional): Test entire workspace. Default: `false`
- `all_targets` (boolean, optional): Test all targets. Default: `false`
- `all_features` (boolean, optional): Build with all features enabled. Default: `false`
- `no_default_features` (boolean, optional): Build without default features. Default: `false`
- `features` (array of strings, optional): List of features to enable
- `no_run` (boolean, optional): Don't run tests, just compile them. Default: `false`
- `quiet` (boolean, optional): Suppress output. Default: `false`
- `test_threads` (number, optional): Number of test threads to use
- `test_name` (string, optional): Name of specific test to run

**Returns:**
- `success`: Whether the command executed successfully
- `tests_run`: Total number of tests run
- `tests_passed`: Number of tests that passed
- `tests_failed`: Number of tests that failed
- `ignored`: Number of tests that were ignored
- `measured`: Number of tests that were measured (benchmarks)
- `filtered_out`: Number of tests that were filtered out
- `output`: Command output
- `error`: Error message if the command failed

##### `cargo_build`

Compile your Rust project.

**Parameters:**
- `package` (string, optional): Specific package to build
- `workspace` (boolean, optional): Build entire workspace. Default: `false`
- `release` (boolean, optional): Build in release mode. Default: `false`
- `debug` (boolean, optional): Build with debug information. Default: `false`
- `all_targets` (boolean, optional): Build all targets. Default: `false`
- `all_features` (boolean, optional): Build with all features enabled. Default: `false`
- `no_default_features` (boolean, optional): Build without default features. Default: `false`
- `features` (array of strings, optional): List of features to enable
- `locked` (boolean, optional): Require Cargo.lock to be up to date. Default: `false`
- `frozen` (boolean, optional): Require Cargo.lock to exist and be up to date. Default: `false`
- `target` (string, optional): Build for a specific target triple
- `target_dir` (string, optional): Directory for generated artifacts

**Returns:**
- `success`: Whether the command executed successfully
- `release_mode`: Whether this was a release build
- `target`: Target triple if specified
- `output`: Command output
- `error`: Error message if the command failed
- `warnings`: Number of warnings found
- `errors`: Number of errors found

#### Coverage Tools (require cargo-llvm-cov)

##### `get_coverage_summary`

Get overall code coverage summary for your Rust project.

**Parameters:**
- `workspace` (boolean, optional): Analyze entire workspace. Default: `false`
- `package` (string, optional): Specific crate in workspace
- `threshold` (number, optional): Minimum coverage percentage (0-100)

**Returns:**
- `success`: Whether the coverage analysis was successful
- `coverage`: Coverage metrics (percentage, lines_covered, total_lines, functions_covered, total_functions)
- `threshold`: Result of threshold checking if a threshold was specified
- `error`: Error message if the analysis failed

##### `get_files_coverage`

Get detailed coverage for all files.

**Parameters:**
- `workspace` (boolean, optional): Analyze entire workspace. Default: `false`
- `package` (string, optional): Specific crate in workspace
- `file` (string, optional): Specific file to analyze (filters results if provided)
- `coverage_below` (number, optional): Only include files with coverage below this percentage (0-100)

**Returns:**
- `success`: Whether the file coverage analysis was successful
- `files`: Array of file coverage information
- `error`: Error message if the analysis failed

##### `get_file_line_coverage`

Get line-by-line coverage details for a specific file.

**Parameters:**
- `file` (string, required): Specific file to analyze
- `workspace` (boolean, optional): Analyze entire workspace. Default: `false`
- `package` (string, optional): Specific crate in workspace

**Returns:**
- `success`: Whether the line coverage analysis was successful
- `file`: Line-by-line coverage for the requested file
- `error`: Error message if the analysis failed

## Examples

### Using with MCP Client

```json
{
  "id": 1,
  "method": "call_tool",
  "params": {
    "name": "cargo_check",
    "arguments": {
      "workspace": true
    }
  }
}
```

### Get Coverage Summary

```json
{
  "id": 1,
  "method": "call_tool",
  "params": {
    "name": "get_coverage_summary",
    "arguments": {
      "threshold": 80.0
    }
  }
}
```

## Configuration

The server can be configured through the MCP client. All tools support standard parameters as documented above.

## Development

### Running Tests

```bash
cargo test
```

### Running Clippy

```bash
cargo clippy
```

### Building Documentation

```bash
cargo doc --open
```

## License

This project is licensed under either:
- MIT License, or
- Apache License, Version 2.0

at your option.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request on GitHub.

## Acknowledgments

- [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) for coverage analysis
- [MCP (Model Context Protocol)](https://modelcontextprotocol.io/) for the server protocol
- [rmcp](https://crates.io/crates/rmcp) for the Rust MCP implementation
