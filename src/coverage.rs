//! Coverage analysis module for cargo-llvm-cov integration
//!
//! This module provides functionality to execute cargo-llvm-cov and parse its JSON output
//! to extract coverage metrics and detailed branch coverage information.

use crate::error::{CoverageError, Result};
use crate::model::{
    BranchInfo, CoverageMetrics, CoverageSummary, CoveredRange, ExecuteCoverageParams,
    FileCoverage, FileCoverageResult, FileLineCoverage, FunctionCoverage,
    GetFileLineCoverageParams, GetFilesCoverageParams, LineCoverage, LineCoverageResult,
    ThresholdResult,
};
use serde_json::{Map, Value};
use std::process::Command;

/// Parse cargo llvm-cov JSON output to extract summary metrics
pub fn parse_coverage_summary(output: &str) -> Result<CoverageMetrics> {
    let json: Value = serde_json::from_str(output)?;

    // cargo-llvm-cov outputs JSON with structure: {"data": [{"totals": {...}}], ...}
    // Try to get totals from json["data"][0]["totals"] first
    if let Some(data_array) = json["data"].as_array()
        && !data_array.is_empty()
    {
        let first_data = &data_array[0];
        if let Some(totals) = first_data["totals"].as_object() {
            let lines_obj = totals["lines"].as_object();
            let functions_obj = totals["functions"].as_object();

            let lines_covered = lines_obj.and_then(|l| l["covered"].as_u64()).unwrap_or(0) as u32;
            let total_lines = lines_obj.and_then(|l| l["count"].as_u64()).unwrap_or(0) as u32;
            let functions_covered = functions_obj
                .and_then(|f| f["covered"].as_u64())
                .unwrap_or(0) as u32;
            let total_functions =
                functions_obj.and_then(|f| f["count"].as_u64()).unwrap_or(0) as u32;

            // Calculate percentage from covered/total
            let percentage = if total_lines > 0 {
                (lines_covered as f64 / total_lines as f64) * 100.0
            } else {
                0.0
            };

            return Ok(CoverageMetrics {
                percentage,
                lines_covered,
                total_lines,
                functions_covered,
                total_functions,
            });
        }
    }

    // Fallback: cargo-llvm-cov outputs an array of coverage reports
    // The last report contains the totals we need
    let reports: Vec<&Value> = if json.is_array() {
        json.as_array().unwrap().iter().collect()
    } else {
        vec![&json]
    };

    // Get the last report which should contain the totals
    let last_report = reports.last().unwrap_or(&reports[0]);

    // Try to get totals from the totals section
    if let Some(totals) = last_report["totals"].as_object() {
        let lines_obj = totals["lines"].as_object();
        let functions_obj = totals["functions"].as_object();

        let lines_covered = lines_obj.and_then(|l| l["covered"].as_u64()).unwrap_or(0) as u32;
        let total_lines = lines_obj.and_then(|l| l["count"].as_u64()).unwrap_or(0) as u32;
        let functions_covered = functions_obj
            .and_then(|f| f["covered"].as_u64())
            .unwrap_or(0) as u32;
        let total_functions = functions_obj.and_then(|f| f["count"].as_u64()).unwrap_or(0) as u32;

        // Calculate percentage from covered/total
        let percentage = if total_lines > 0 {
            (lines_covered as f64 / total_lines as f64) * 100.0
        } else {
            0.0
        };

        return Ok(CoverageMetrics {
            percentage,
            lines_covered,
            total_lines,
            functions_covered,
            total_functions,
        });
    }

    // If we still haven't found totals, return zeros
    Ok(CoverageMetrics {
        percentage: 0.0,
        lines_covered: 0,
        total_lines: 0,
        functions_covered: 0,
        total_functions: 0,
    })
}

/// Parse detailed branch coverage from cargo llvm-cov JSON output
pub fn parse_file_coverage(output: &str) -> Result<Vec<FileCoverage>> {
    let json: Value = serde_json::from_str(output)?;

    let mut files = Vec::new();

    // Handle the data structure: cargo-llvm-cov outputs {"data": [{"files": [...], ...}], ...}
    let reports: Vec<&Value> = if let Some(data_array) = json["data"].as_array() {
        if data_array.is_empty() {
            vec![&json]
        } else {
            data_array.iter().collect()
        }
    } else if json.is_array() {
        json.as_array().unwrap().iter().collect()
    } else {
        vec![&json]
    };

    for report in reports {
        if let Some(files_array) = report["files"].as_array() {
            for file_json in files_array {
                let path = file_json["filename"].as_str().unwrap_or("").to_string();

                // Get coverage from summary field - use raw counts to calculate percentage
                // to be consistent with parse_coverage_summary
                let summary = file_json["summary"].as_object();
                let lines_summary = summary.and_then(|s| s["lines"].as_object());

                let lines_covered = lines_summary
                    .and_then(|l| l["covered"].as_u64())
                    .unwrap_or(0) as u32;
                let total_lines =
                    lines_summary.and_then(|l| l["count"].as_u64()).unwrap_or(0) as u32;

                // Calculate line coverage percentage from raw counts (consistent with parse_coverage_summary)
                let line_coverage = if total_lines > 0 {
                    (lines_covered as f64 / total_lines as f64) * 100.0
                } else {
                    0.0
                };

                let branch_coverage = file_json["summary"]["branches"]["percent"]
                    .as_f64()
                    .unwrap_or(0.0);

                let mut functions = Vec::new();

                // Parse functions if available
                if let Some(funcs) = file_json["functions"].as_array() {
                    for func_json in funcs {
                        let name = func_json["name"].as_str().unwrap_or("").to_string();

                        let func_line_coverage = func_json["line_coverage"].as_f64().unwrap_or(0.0);

                        let func_branch_coverage =
                            func_json["branch_coverage"].as_f64().unwrap_or(0.0);

                        let mut branches = Vec::new();

                        // Parse branches if available
                        if let Some(branches_array) = func_json["branches"].as_array() {
                            for branch_json in branches_array {
                                let line =
                                    branch_json["line"].as_u64().map(|v| v as u32).unwrap_or(0);

                                let condition =
                                    branch_json["condition"].as_str().unwrap_or("").to_string();

                                let covered = branch_json["covered"].as_bool().unwrap_or(false);

                                let hits =
                                    branch_json["hits"].as_u64().map(|v| v as u32).unwrap_or(0);

                                branches.push(BranchInfo {
                                    line,
                                    condition,
                                    covered,
                                    hits,
                                });
                            }
                        }

                        functions.push(FunctionCoverage {
                            name,
                            line_coverage: func_line_coverage,
                            branch_coverage: func_branch_coverage,
                            branches,
                        });
                    }
                }

                files.push(FileCoverage {
                    path,
                    line_coverage,
                    branch_coverage,
                    lines_covered,
                    total_lines,
                    functions,
                });
            }
        }
    }

    Ok(files)
}

/// Compress a sorted vector of LineCoverage into covered ranges and uncovered lines
fn compress_line_coverage(lines: Vec<LineCoverage>) -> (Vec<CoveredRange>, Vec<u32>) {
    let mut covered_ranges: Vec<CoveredRange> = Vec::new();
    let mut uncovered_lines: Vec<u32> = Vec::new();

    let mut iter = lines.into_iter().peekable();

    while let Some(line) = iter.next() {
        if !line.covered {
            // Uncovered line - add to uncovered_lines
            uncovered_lines.push(line.line);
        } else {
            // Covered line - start or continue a range
            let start_line = line.line;
            let current_hits = line.hits;
            let mut end_line = line.line;

            // Continue the range as long as the next line is consecutive and has the same hit count
            while let Some(next_line) = iter.peek() {
                if next_line.covered
                    && next_line.line == end_line + 1
                    && next_line.hits == current_hits
                {
                    end_line = next_line.line;
                    iter.next(); // consume this line
                } else {
                    break;
                }
            }

            covered_ranges.push(CoveredRange {
                start: start_line,
                end: end_line,
                hits: current_hits,
            });
        }
    }

    (covered_ranges, uncovered_lines)
}

/// Parse line-by-line coverage from cargo llvm-cov JSON output for a specific file
pub fn parse_line_coverage(output: &str, target_file: &str) -> Result<Option<FileLineCoverage>> {
    let json: Value = serde_json::from_str(output)?;

    // Handle the data structure: cargo-llvm-cov outputs {"data": [{"files": [...], ...}], ...}
    let reports: Vec<&Value> = if let Some(data_array) = json["data"].as_array() {
        if data_array.is_empty() {
            vec![&json]
        } else {
            data_array.iter().collect()
        }
    } else if json.is_array() {
        json.as_array().unwrap().iter().collect()
    } else {
        vec![&json]
    };

    for report in reports {
        if let Some(files_array) = report["files"].as_array() {
            for file_json in files_array {
                let path = file_json["filename"].as_str().unwrap_or("");

                // Check if this is the target file
                if !path.contains(target_file) {
                    continue;
                }

                // Get overall line coverage percentage
                let line_coverage = file_json["summary"]["lines"]["percent"]
                    .as_f64()
                    .unwrap_or(0.0);

                // Parse segments to get line-by-line coverage
                let mut lines_map: std::collections::HashMap<u32, LineCoverage> =
                    std::collections::HashMap::new();
                if let Some(segments) = file_json["segments"].as_array() {
                    for segment in segments {
                        if let Some(segment_array) = segment.as_array()
                            && segment_array.len() >= 6
                        {
                            let line = segment_array[0].as_u64().map(|v| v as u32).unwrap_or(0);
                            let hits = segment_array[2].as_u64().map(|v| v as u32).unwrap_or(0);
                            let covered = segment_array[3].as_bool().unwrap_or(false);

                            // Only include the line if it has executable code (hits > 0 or covered)
                            if hits > 0 || covered {
                                // For each line, keep the entry with the highest hits
                                // If hits are equal, prefer covered=true
                                let existing = lines_map.get_mut(&line);
                                match existing {
                                    Some(existing_line) => {
                                        if hits > existing_line.hits
                                            || (hits == existing_line.hits
                                                && covered
                                                && !existing_line.covered)
                                        {
                                            *existing_line = LineCoverage {
                                                line,
                                                covered,
                                                hits,
                                            };
                                        }
                                    }
                                    None => {
                                        lines_map.insert(
                                            line,
                                            LineCoverage {
                                                line,
                                                covered,
                                                hits,
                                            },
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                // Convert the map to a sorted vector
                let mut lines: Vec<LineCoverage> = lines_map.into_values().collect();
                lines.sort_by_key(|a| a.line);

                // Compress into ranges and uncovered lines
                let (covered_ranges, uncovered_lines) = compress_line_coverage(lines);

                return Ok(Some(FileLineCoverage {
                    path: path.to_string(),
                    line_coverage,
                    covered_ranges,
                    uncovered_lines,
                }));
            }
        }
    }

    Ok(None)
}

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
                "Failed to execute cargo llvm-cov with workspace={}, package={:?}: {}",
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
            "cargo llvm-cov failed: {}",
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

/// Validate execute coverage parameters
pub fn validate_execute_params(params: &ExecuteCoverageParams) -> Result<()> {
    if params.workspace && params.package.is_some() {
        return Err(CoverageError::Validation(
            "Cannot specify both 'workspace' and 'package'. Use one or the other, or neither for the current package.".to_string(),
        ));
    }

    if let Some(threshold) = params.threshold
        && !(0.0..=100.0).contains(&threshold)
    {
        return Err(CoverageError::Validation(
            "Threshold must be between 0 and 100".to_string(),
        ));
    }

    Ok(())
}

/// Validate get files coverage parameters
pub fn validate_file_params(params: &GetFilesCoverageParams) -> Result<()> {
    if params.workspace && params.package.is_some() {
        return Err(CoverageError::Validation(
            "Cannot specify both 'workspace' and 'package'. Use one or the other, or neither for the current package.".to_string(),
        ));
    }

    if let Some(coverage_below) = params.coverage_below
        && !(0.0..=100.0).contains(&coverage_below)
    {
        return Err(CoverageError::Validation(
            "coverage_below must be between 0 and 100".to_string(),
        ));
    }

    Ok(())
}

/// Handler for execute_coverage tool
pub async fn handle_execute_coverage(params: ExecuteCoverageParams) -> Result<CoverageSummary> {
    validate_execute_params(&params)?;

    // Execute cargo llvm-cov
    let output = execute_llvm_cov(params.workspace, params.package.as_deref())?;

    // Parse files to calculate consistent metrics
    let files = parse_file_coverage(&output)?;

    // Calculate overall metrics from files to ensure consistency with get_files_coverage
    let total_lines_covered: u32 = files.iter().map(|f| f.lines_covered).sum();
    let total_lines: u32 = files.iter().map(|f| f.total_lines).sum();
    let functions_covered: u32 = files
        .iter()
        .map(|f| {
            f.functions
                .iter()
                .filter(|func| func.line_coverage > 0.0)
                .count() as u32
        })
        .sum();
    let total_functions: u32 = files.iter().map(|f| f.functions.len() as u32).sum();

    let percentage = if total_lines > 0 {
        (total_lines_covered as f64 / total_lines as f64) * 100.0
    } else {
        0.0
    };

    let metrics = CoverageMetrics {
        percentage,
        lines_covered: total_lines_covered,
        total_lines,
        functions_covered,
        total_functions,
    };

    // Check threshold
    let threshold_result = params.threshold.map(|threshold| ThresholdResult {
        met: metrics.percentage >= threshold,
        value: threshold,
    });

    Ok(CoverageSummary {
        success: true,
        coverage: metrics,
        threshold: threshold_result,
        error: None,
    })
}

/// Handler for get_files_coverage tool
pub async fn handle_get_file_coverage(
    params: GetFilesCoverageParams,
) -> Result<FileCoverageResult> {
    validate_file_params(&params)?;

    // Execute cargo llvm-cov
    let output = execute_llvm_cov(params.workspace, params.package.as_deref())?;

    // Parse the detailed output
    let files = parse_file_coverage(&output)?;

    // Filter by file if specified
    let files = if let Some(file_path) = params.file {
        files
            .into_iter()
            .filter(|f| f.path.contains(&file_path))
            .collect()
    } else {
        files
    };

    // Filter by coverage_below if specified
    let files = if let Some(coverage_below) = params.coverage_below {
        files
            .into_iter()
            .filter(|f| f.line_coverage < coverage_below)
            .collect()
    } else {
        files
    };

    Ok(FileCoverageResult {
        success: true,
        files,
        error: None,
    })
}

/// Validate get file line coverage parameters
pub fn validate_line_params(params: &GetFileLineCoverageParams) -> Result<()> {
    if params.workspace && params.package.is_some() {
        return Err(CoverageError::Validation(
            "Cannot specify both 'workspace' and 'package'. Use one or the other, or neither for the current package.".to_string(),
        ));
    }

    if params.file.is_empty() {
        return Err(CoverageError::Validation(
            "file parameter is required".to_string(),
        ));
    }

    Ok(())
}

/// Handler for get_file_line_coverage tool
pub async fn handle_get_line_coverage(
    params: GetFileLineCoverageParams,
) -> Result<LineCoverageResult> {
    validate_line_params(&params)?;

    // Execute cargo llvm-cov
    let output = execute_llvm_cov(params.workspace, params.package.as_deref())?;

    // Parse the line-by-line coverage for the specific file
    let file_coverage = parse_line_coverage(&output, &params.file)?;

    Ok(LineCoverageResult {
        success: true,
        file: file_coverage,
        error: None,
    })
}

/// Helper function to create a property map with type and description
fn create_property(type_str: &str, description: &str) -> Map<String, Value> {
    let mut props = Map::new();
    props.insert("type".to_string(), Value::String(type_str.to_string()));
    props.insert(
        "description".to_string(),
        Value::String(description.to_string()),
    );
    props
}

/// Create schema for get_coverage_summary tool
pub fn create_execute_schema() -> Map<String, Value> {
    let mut properties = Map::new();

    properties.insert(
        "workspace".to_string(),
        Value::Object(create_property("boolean", "Analyze entire workspace")),
    );

    properties.insert(
        "package".to_string(),
        Value::Object(create_property("string", "Specific crate in workspace")),
    );

    properties.insert(
        "threshold".to_string(),
        Value::Object(create_property(
            "number",
            "Minimum coverage percentage (0-100)",
        )),
    );

    let mut schema = Map::new();
    schema.insert("type".to_string(), Value::String("object".to_string()));
    schema.insert("properties".to_string(), Value::Object(properties));
    schema.insert(
        "description".to_string(),
        Value::String("Returns overall coverage summary. Coverage percentage is calculated as: (total lines covered / total executable lines) * 100. Note: Files with no executable code (e.g., struct definitions only) may be excluded from the analysis by cargo-llvm-cov.".to_string()),
    );

    schema
}

/// Create schema for get_files_coverage tool
pub fn create_file_schema() -> Map<String, Value> {
    let mut properties = Map::new();

    properties.insert(
        "workspace".to_string(),
        Value::Object(create_property("boolean", "Analyze entire workspace")),
    );

    properties.insert(
        "package".to_string(),
        Value::Object(create_property("string", "Specific crate in workspace")),
    );

    properties.insert(
        "file".to_string(),
        Value::Object(create_property(
            "string",
            "Specific file to analyze (filters results if provided)",
        )),
    );

    properties.insert(
        "coverage_below".to_string(),
        Value::Object(create_property(
            "number",
            "Only include files with coverage below this percentage (0-100)",
        )),
    );

    let mut schema = Map::new();
    schema.insert("type".to_string(), Value::String("object".to_string()));
    schema.insert("properties".to_string(), Value::Object(properties));
    schema.insert(
        "description".to_string(),
        Value::String("Returns per-file coverage details. Each file's coverage percentage is calculated as: (file lines covered / file total executable lines) * 100. Overall percentage in summary uses: (sum of all covered lines / sum of all executable lines) * 100. Note: Files with no executable code may be excluded by cargo-llvm-cov.".to_string()),
    );

    schema
}

/// Create schema for get_line_coverage tool
pub fn create_line_schema() -> Map<String, Value> {
    let mut properties = Map::new();

    properties.insert(
        "file".to_string(),
        Value::Object(create_property(
            "string",
            "Specific file to analyze (required)",
        )),
    );

    properties.insert(
        "workspace".to_string(),
        Value::Object(create_property("boolean", "Analyze entire workspace")),
    );

    properties.insert(
        "package".to_string(),
        Value::Object(create_property("string", "Specific crate in workspace")),
    );

    let mut schema = Map::new();
    schema.insert("type".to_string(), Value::String("object".to_string()));
    schema.insert("properties".to_string(), Value::Object(properties));
    schema.insert(
        "description".to_string(),
        Value::String("Returns line-by-line coverage for a specific file. Each line's coverage indicates whether it was executed during tests. Note: Only lines with executable code are included in the analysis.".to_string()),
    );

    // Mark file as required
    let required = vec![Value::String("file".to_string())];
    schema.insert("required".to_string(), Value::Array(required));

    schema
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ExecuteCoverageParams, GetFileLineCoverageParams, GetFilesCoverageParams};
    use serde_json::json;

    #[test]
    fn test_parse_coverage_summary_basic() {
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
    fn test_parse_coverage_summary_zero_lines() {
        let json_output = json!({
            "data": [{
                "totals": {
                    "lines": {
                        "covered": 0,
                        "count": 0
                    },
                    "functions": {
                        "covered": 0,
                        "count": 0
                    }
                }
            }]
        })
        .to_string();

        let result = parse_coverage_summary(&json_output).unwrap();

        assert_eq!(result.lines_covered, 0);
        assert_eq!(result.total_lines, 0);
        assert_eq!(result.percentage, 0.0);
    }

    #[test]
    fn test_parse_coverage_summary_fallback() {
        let json_output = json!([
            {
                "totals": {
                    "lines": {
                        "covered": 50,
                        "count": 100
                    },
                    "functions": {
                        "covered": 5,
                        "count": 10
                    }
                }
            }
        ])
        .to_string();

        let result = parse_coverage_summary(&json_output).unwrap();

        assert_eq!(result.lines_covered, 50);
        assert_eq!(result.total_lines, 100);
        assert_eq!(result.functions_covered, 5);
        assert_eq!(result.total_functions, 10);
        assert!((result.percentage - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_file_coverage_basic() {
        let json_output = json!({
            "data": [{
                "files": [
                    {
                        "filename": "src/lib.rs",
                        "summary": {
                            "lines": {
                                "covered": 50,
                                "count": 100,
                                "percent": 50.0
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
        assert_eq!(result[0].lines_covered, 50);
        assert_eq!(result[0].total_lines, 100);
    }

    #[test]
    fn test_parse_file_coverage_multiple_files() {
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
                    },
                    {
                        "filename": "src/main.rs",
                        "summary": {
                            "lines": {
                                "covered": 25,
                                "count": 50
                            },
                            "branches": {
                                "percent": 80.0
                            }
                        }
                    }
                ]
            }]
        })
        .to_string();

        let result = parse_file_coverage(&json_output).unwrap();

        assert_eq!(result.len(), 2);

        let total_covered: u32 = result.iter().map(|f| f.lines_covered).sum();
        let total_lines: u32 = result.iter().map(|f| f.total_lines).sum();
        let overall_percentage = (total_covered as f64 / total_lines as f64) * 100.0;

        assert_eq!(total_covered, 75);
        assert_eq!(total_lines, 150);
        assert!((overall_percentage - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_file_coverage_zero_lines() {
        let json_output = json!({
            "data": [{
                "files": [
                    {
                        "filename": "src/model.rs",
                        "summary": {
                            "lines": {
                                "covered": 0,
                                "count": 0
                            },
                            "branches": {
                                "percent": 0.0
                            }
                        }
                    }
                ]
            }]
        })
        .to_string();

        let result = parse_file_coverage(&json_output).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "src/model.rs");
        assert_eq!(result[0].line_coverage, 0.0);
        assert_eq!(result[0].lines_covered, 0);
        assert_eq!(result[0].total_lines, 0);
    }

    #[test]
    fn test_parse_line_coverage_basic() {
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

        // With hybrid format: lines 1 and 3 are covered (each with hits=1)
        // Line 2 has hits=0 and covered=false, so it's not included at all
        // Lines 1 and 3 are consecutive in terms of included lines, but not consecutive line numbers
        // So we should have 2 separate ranges (1-1 and 3-3)
        assert_eq!(file_coverage.covered_ranges.len(), 2);
        assert_eq!(file_coverage.uncovered_lines.len(), 0);

        // Check covered ranges
        let covered_lines: Vec<u32> = file_coverage
            .covered_ranges
            .iter()
            .flat_map(|r| r.start..=r.end)
            .collect();
        assert!(covered_lines.contains(&1));
        assert!(covered_lines.contains(&3));
    }

    #[test]
    fn test_parse_line_coverage_not_found() {
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
                        "segments": []
                    }
                ]
            }]
        })
        .to_string();

        let result = parse_line_coverage(&json_output, "src/other.rs").unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn test_validate_execute_params_valid() {
        let params = ExecuteCoverageParams {
            workspace: false,
            package: None,
            threshold: Some(80.0),
        };

        let result = validate_execute_params(&params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_execute_params_workspace_and_package_error() {
        let params = ExecuteCoverageParams {
            workspace: true,
            package: Some("my-package".to_string()),
            threshold: None,
        };

        let result = validate_execute_params(&params);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("Cannot specify both 'workspace' and 'package'")
        );
    }

    #[test]
    fn test_validate_execute_params_invalid_threshold() {
        let params = ExecuteCoverageParams {
            workspace: false,
            package: None,
            threshold: Some(150.0),
        };

        let result = validate_execute_params(&params);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("Threshold must be between 0 and 100")
        );
    }

    #[test]
    fn test_validate_file_params_valid() {
        let params = GetFilesCoverageParams {
            workspace: false,
            package: None,
            file: None,
            coverage_below: Some(50.0),
        };

        let result = validate_file_params(&params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_file_params_workspace_and_package_error() {
        let params = GetFilesCoverageParams {
            workspace: true,
            package: Some("my-package".to_string()),
            file: None,
            coverage_below: None,
        };

        let result = validate_file_params(&params);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("Cannot specify both 'workspace' and 'package'")
        );
    }

    #[test]
    fn test_validate_file_params_invalid_coverage_below() {
        let params = GetFilesCoverageParams {
            workspace: false,
            package: None,
            file: None,
            coverage_below: Some(150.0),
        };

        let result = validate_file_params(&params);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("coverage_below must be between 0 and 100")
        );
    }

    #[test]
    fn test_validate_line_params_valid() {
        let params = GetFileLineCoverageParams {
            file: "src/lib.rs".to_string(),
            workspace: false,
            package: None,
        };

        let result = validate_line_params(&params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_line_params_workspace_and_package_error() {
        let params = GetFileLineCoverageParams {
            file: "src/lib.rs".to_string(),
            workspace: true,
            package: Some("my-package".to_string()),
        };

        let result = validate_line_params(&params);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("Cannot specify both 'workspace' and 'package'")
        );
    }

    #[test]
    fn test_validate_line_params_empty_file() {
        let params = GetFileLineCoverageParams {
            file: "".to_string(),
            workspace: false,
            package: None,
        };

        let result = validate_line_params(&params);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.to_string().contains("file parameter is required"));
    }

    #[test]
    fn test_parse_coverage_summary_missing_fields() {
        // Test with missing totals field
        let json_output = json!({
            "data": [{
                "other_field": "value"
            }]
        })
        .to_string();

        let result = parse_coverage_summary(&json_output).unwrap();

        assert_eq!(result.lines_covered, 0);
        assert_eq!(result.total_lines, 0);
        assert_eq!(result.functions_covered, 0);
        assert_eq!(result.total_functions, 0);
        assert_eq!(result.percentage, 0.0);
    }

    #[test]
    fn test_parse_file_coverage_empty_files() {
        let json_output = json!({
            "data": [{
                "files": []
            }]
        })
        .to_string();

        let result = parse_file_coverage(&json_output).unwrap();

        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_line_coverage_empty_segments() {
        let json_output = json!({
            "data": [{
                "files": [
                    {
                        "filename": "src/lib.rs",
                        "summary": {
                            "lines": {
                                "percent": 0.0
                            }
                        },
                        "segments": []
                    }
                ]
            }]
        })
        .to_string();

        let result = parse_line_coverage(&json_output, "src/lib.rs").unwrap();

        assert!(result.is_some());
        let file_coverage = result.unwrap();
        assert!(file_coverage.covered_ranges.is_empty());
        assert!(file_coverage.uncovered_lines.is_empty());
    }

    #[test]
    fn test_validate_params_boundary_values() {
        let params1 = ExecuteCoverageParams {
            workspace: false,
            package: None,
            threshold: Some(0.0),
        };
        assert!(validate_execute_params(&params1).is_ok());

        let params2 = ExecuteCoverageParams {
            workspace: false,
            package: None,
            threshold: Some(100.0),
        };
        assert!(validate_execute_params(&params2).is_ok());

        let params3 = ExecuteCoverageParams {
            workspace: false,
            package: None,
            threshold: Some(100.0001),
        };
        assert!(validate_execute_params(&params3).is_err());

        let params4 = ExecuteCoverageParams {
            workspace: false,
            package: None,
            threshold: Some(-0.0001),
        };
        assert!(validate_execute_params(&params4).is_err());
    }

    #[test]
    fn test_coverage_below_boundary_values() {
        let params1 = GetFilesCoverageParams {
            workspace: false,
            package: None,
            file: None,
            coverage_below: Some(0.0),
        };
        assert!(validate_file_params(&params1).is_ok());

        let params2 = GetFilesCoverageParams {
            workspace: false,
            package: None,
            file: None,
            coverage_below: Some(100.0),
        };
        assert!(validate_file_params(&params2).is_ok());

        let params3 = GetFilesCoverageParams {
            workspace: false,
            package: None,
            file: None,
            coverage_below: Some(100.0001),
        };
        assert!(validate_file_params(&params3).is_err());
    }

    #[test]
    fn test_create_execute_schema() {
        let schema = create_execute_schema();

        assert!(schema.contains_key("type"));
        assert!(schema.contains_key("properties"));
        assert!(schema.contains_key("description"));

        if let Some(Value::Object(props)) = schema.get("properties") {
            assert!(props.contains_key("workspace"));
            assert!(props.contains_key("package"));
            assert!(props.contains_key("threshold"));
        } else {
            panic!("Schema should have properties");
        }
    }

    #[test]
    fn test_create_file_schema() {
        let schema = create_file_schema();

        assert!(schema.contains_key("type"));
        assert!(schema.contains_key("properties"));
        assert!(schema.contains_key("description"));

        if let Some(Value::Object(props)) = schema.get("properties") {
            assert!(props.contains_key("workspace"));
            assert!(props.contains_key("package"));
            assert!(props.contains_key("file"));
            assert!(props.contains_key("coverage_below"));
        } else {
            panic!("Schema should have properties");
        }
    }

    #[test]
    fn test_create_line_schema() {
        let schema = create_line_schema();

        assert!(schema.contains_key("type"));
        assert!(schema.contains_key("properties"));
        assert!(schema.contains_key("description"));
        assert!(schema.contains_key("required"));

        if let Some(Value::Object(props)) = schema.get("properties") {
            assert!(props.contains_key("file"));
            assert!(props.contains_key("workspace"));
            assert!(props.contains_key("package"));
        } else {
            panic!("Schema should have properties");
        }

        // Check that file is required
        if let Some(Value::Array(required)) = schema.get("required") {
            let required_fields: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
            assert!(required_fields.contains(&"file"));
        } else {
            panic!("Schema should have required field");
        }
    }

    #[test]
    fn test_schema_descriptions_are_not_empty() {
        let execute_schema = create_execute_schema();
        let file_schema = create_file_schema();
        let line_schema = create_line_schema();

        if let Some(Value::String(desc)) = execute_schema.get("description") {
            assert!(
                !desc.is_empty(),
                "execute schema description should not be empty"
            );
        } else {
            panic!("execute schema should have description");
        }

        if let Some(Value::String(desc)) = file_schema.get("description") {
            assert!(
                !desc.is_empty(),
                "file schema description should not be empty"
            );
        } else {
            panic!("file schema should have description");
        }

        if let Some(Value::String(desc)) = line_schema.get("description") {
            assert!(
                !desc.is_empty(),
                "line schema description should not be empty"
            );
        } else {
            panic!("line schema should have description");
        }
    }

    #[test]
    fn test_schema_type_is_object() {
        let execute_schema = create_execute_schema();
        let file_schema = create_file_schema();
        let line_schema = create_line_schema();

        assert_eq!(
            execute_schema.get("type").and_then(|v| v.as_str()),
            Some("object")
        );
        assert_eq!(
            file_schema.get("type").and_then(|v| v.as_str()),
            Some("object")
        );
        assert_eq!(
            line_schema.get("type").and_then(|v| v.as_str()),
            Some("object")
        );
    }

    #[test]
    fn test_create_property_function() {
        let prop = create_property("string", "Test description");

        assert!(prop.contains_key("type"));
        assert!(prop.contains_key("description"));

        if let Some(Value::String(prop_type)) = prop.get("type") {
            assert_eq!(prop_type, "string");
        } else {
            panic!("Property should have type field");
        }

        if let Some(Value::String(desc)) = prop.get("description") {
            assert_eq!(desc, "Test description");
        } else {
            panic!("Property should have description field");
        }
    }

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

    #[test]
    fn test_parse_coverage_summary_empty_data_array() {
        // Test with empty data array - should use fallback path
        let json_output = json!({
            "data": []
        })
        .to_string();

        let result = parse_coverage_summary(&json_output).unwrap();

        // Should return zeros when data is empty
        assert_eq!(result.lines_covered, 0);
        assert_eq!(result.total_lines, 0);
        assert_eq!(result.percentage, 0.0);
    }

    #[test]
    fn test_parse_file_coverage_empty_data_array() {
        // Test with empty data array
        let json_output = json!({
            "data": []
        })
        .to_string();

        let result = parse_file_coverage(&json_output).unwrap();

        // Should return empty vec when data is empty
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_line_coverage_missing_segments() {
        // Test with file that has no segments
        let json_output = json!({
            "data": [{
                "files": [
                    {
                        "filename": "src/lib.rs",
                        "summary": {
                            "lines": {
                                "percent": 0.0
                            }
                        }
                    }
                ]
            }]
        })
        .to_string();

        let result = parse_line_coverage(&json_output, "src/lib.rs").unwrap();

        assert!(result.is_some());
        let file_coverage = result.unwrap();
        assert!(file_coverage.covered_ranges.is_empty());
        assert!(file_coverage.uncovered_lines.is_empty());
    }

    #[test]
    fn test_compress_line_coverage_basic() {
        // Test compression of consecutive covered lines with same hits
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
            LineCoverage {
                line: 7,
                covered: true,
                hits: 2,
            },
            LineCoverage {
                line: 8,
                covered: true,
                hits: 2,
            },
        ];

        let (covered_ranges, uncovered_lines) = compress_line_coverage(lines);

        // Should have 3 covered ranges and 1 uncovered line
        assert_eq!(covered_ranges.len(), 3);
        assert_eq!(uncovered_lines.len(), 1);
        assert_eq!(uncovered_lines[0], 6);

        // Check ranges
        assert_eq!(covered_ranges[0].start, 1);
        assert_eq!(covered_ranges[0].end, 3);
        assert_eq!(covered_ranges[0].hits, 5);

        assert_eq!(covered_ranges[1].start, 5);
        assert_eq!(covered_ranges[1].end, 5);
        assert_eq!(covered_ranges[1].hits, 3);

        assert_eq!(covered_ranges[2].start, 7);
        assert_eq!(covered_ranges[2].end, 8);
        assert_eq!(covered_ranges[2].hits, 2);
    }

    #[test]
    fn test_compress_line_coverage_all_covered() {
        // Test when all lines are covered with same hits
        let lines = vec![
            LineCoverage {
                line: 10,
                covered: true,
                hits: 1,
            },
            LineCoverage {
                line: 11,
                covered: true,
                hits: 1,
            },
            LineCoverage {
                line: 12,
                covered: true,
                hits: 1,
            },
        ];

        let (covered_ranges, uncovered_lines) = compress_line_coverage(lines);

        assert_eq!(covered_ranges.len(), 1);
        assert_eq!(uncovered_lines.len(), 0);
        assert_eq!(covered_ranges[0].start, 10);
        assert_eq!(covered_ranges[0].end, 12);
        assert_eq!(covered_ranges[0].hits, 1);
    }

    #[test]
    fn test_compress_line_coverage_all_uncovered() {
        // Test when all lines are uncovered
        let lines = vec![
            LineCoverage {
                line: 1,
                covered: false,
                hits: 0,
            },
            LineCoverage {
                line: 3,
                covered: false,
                hits: 0,
            },
            LineCoverage {
                line: 5,
                covered: false,
                hits: 0,
            },
        ];

        let (covered_ranges, uncovered_lines) = compress_line_coverage(lines);

        assert_eq!(covered_ranges.len(), 0);
        assert_eq!(uncovered_lines.len(), 3);
        assert_eq!(uncovered_lines, vec![1, 3, 5]);
    }

    #[test]
    fn test_compress_line_coverage_mixed_hits() {
        // Test compression stops when hit count changes
        let lines = vec![
            LineCoverage {
                line: 1,
                covered: true,
                hits: 1,
            },
            LineCoverage {
                line: 2,
                covered: true,
                hits: 1,
            },
            LineCoverage {
                line: 3,
                covered: true,
                hits: 2,
            }, // hits change
            LineCoverage {
                line: 4,
                covered: true,
                hits: 2,
            },
        ];

        let (covered_ranges, uncovered_lines) = compress_line_coverage(lines);

        assert_eq!(covered_ranges.len(), 2);
        assert_eq!(uncovered_lines.len(), 0);

        assert_eq!(covered_ranges[0].start, 1);
        assert_eq!(covered_ranges[0].end, 2);
        assert_eq!(covered_ranges[0].hits, 1);

        assert_eq!(covered_ranges[1].start, 3);
        assert_eq!(covered_ranges[1].end, 4);
        assert_eq!(covered_ranges[1].hits, 2);
    }
}
