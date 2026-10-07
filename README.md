# rs-lcov-mcp

[![Coverage Status](https://img.shields.io/badge/coverage-78.44%25-brightgreen)](https://github.com/mestrode/rs-lcov-mcp)

An MCP (Model Context Protocol) server for Rust code coverage analysis using [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov).

## Features

- **Code Coverage Summary**: Get overall coverage percentage for your Rust project
- **Per-File Coverage**: Detailed coverage breakdown for each file
- **Line-by-Line Coverage**: See exactly which lines are covered by tests
- **Threshold Checking**: Set minimum coverage thresholds and check if they're met
- **Filtering**: Filter files by coverage percentage

## Installation

### Prerequisites

1. Install [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov):
   ```bash
   cargo install cargo-llvm-cov
   ```

2. Ensure you have Rust and Cargo installed (1.60+ recommended)

### Build

```bash
# Clone the repository
git clone https://github.com/mestrode/rs-lcov-mcp
cd rs-lcov-mcp

# Build in release mode
cargo build --release

# The binary will be at target/release/rs-lcov-mcp
```

## Usage

### As an MCP Server

The server communicates via stdio and provides the following tools:

#### `get_coverage_summary`

Get overall code coverage summary for your Rust project.

**Parameters:**
- `workspace` (boolean, optional): Analyze entire workspace. Default: `false`
- `package` (string, optional): Specific crate in workspace
- `threshold` (number, optional): Minimum coverage percentage (0-100). If provided, checks if coverage meets the threshold

**Example:**
```json
{
  "workspace": false,
  "threshold": 80.0
}
```

**Response:**
```json
{
  "success": true,
  "coverage": {
    "percentage": 78.44,
    "lines_covered": 1084,
    "total_lines": 1382,
    "functions_covered": 0,
    "total_functions": 0
  },
  "threshold": {
    "met": false,
    "value": 80.0
  },
  "error": null
}
```

**Calculation Method:**
Percentage = (total covered lines / total executable lines) * 100

> **Note:** Files with no executable code (e.g., struct definitions only) may be excluded from the analysis by cargo-llvm-cov.

#### `get_files_coverage`

Get detailed coverage for all files in your project.

**Parameters:**
- `workspace` (boolean, optional): Analyze entire workspace. Default: `false`
- `package` (string, optional): Specific crate in workspace
- `file` (string, optional): Specific file to analyze (filters results if provided)
- `coverage_below` (number, optional): Only include files with coverage below this percentage (0-100)

**Example:**
```json
{
  "workspace": true,
  "coverage_below": 50.0
}
```

**Response:**
```json
{
  "success": true,
  "files": [
    {
      "path": "src/lib.rs",
      "line_coverage": 85.5,
      "branch_coverage": 75.0,
      "lines_covered": 150,
      "total_lines": 180,
      "functions": []
    }
  ],
  "error": null
}
```

**Calculation Method:**
- Each file's percentage = (file lines covered / file total executable lines) * 100
- Overall percentage in summary uses: (sum of all covered lines / sum of all executable lines) * 100

> **Note:** Files with no executable code may be excluded by cargo-llvm-cov.

#### `get_file_line_coverage`

Get line-by-line coverage details for a specific file.

**Parameters:**
- `file` (string, **required**): Specific file to analyze
- `workspace` (boolean, optional): Analyze entire workspace. Default: `false`
- `package` (string, optional): Specific crate in workspace

**Example:**
```json
{
  "file": "src/lib.rs",
  "workspace": false
}
```

**Response:**
```json
{
  "success": true,
  "file": {
    "path": "src/lib.rs",
    "line_coverage": 85.5,
    "lines": [
      {
        "line": 10,
        "covered": true,
        "hits": 5
      },
      {
        "line": 20,
        "covered": false,
        "hits": 0
      }
    ]
  },
  "error": null
}
```

> **Note:** Only lines with executable code are included in the analysis.

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Check coverage
cargo llvm-cov --open
```

### Project Structure

```
rs-lcov-mcp/
├── src/
│   ├── main.rs          # Entry point
│   ├── mcp.rs           # MCP server implementation
│   ├── coverage.rs      # Coverage parsing and handling
│   ├── error.rs         # Error types and handling
│   └── model.rs         # Data models
├── tests/
│   └── basic_test.rs    # Basic integration tests
├── Cargo.toml
└── README.md
```

### Code Coverage

Current test coverage: **78.44%**

- `coverage.rs`: 80.85%
- `error.rs`: 100%
- `main.rs`: 73.33%
- `mcp.rs`: 69.92%

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

## License

This project is licensed under the MIT License or Apache-2.0 at your choice.

## Acknowledgments

- [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) - The underlying coverage tool
- [rmcp](https://github.com/modelcontextprotocol/rust-sdk/) - The utilized mcp server crate
- [MCP (Model Context Protocol)](https://modelcontextprotocol.io/) - The protocol this server implements
