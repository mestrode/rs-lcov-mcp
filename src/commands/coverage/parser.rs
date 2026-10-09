//! Coverage parsing module for cargo-llvm-cov JSON output
//!
//! This module provides functionality to parse cargo-llvm-cov JSON output
//! and extract coverage metrics and detailed branch coverage information.

use crate::error::Result;
use crate::model::{
    BranchInfo, CoverageMetrics, CoveredRange, FileCoverage, FileLineCoverage, FunctionCoverage,
    LineCoverage,
};
use serde_json::Value;
use std::collections::HashMap;

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
/// Ranges are sorted by hit count (ascending) so zero-hit and low-hit ranges appear first
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

    // Sort ranges by hit count (ascending) so zero-hit and low-hit ranges appear first
    covered_ranges.sort_by_key(|a| a.hits);

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
                let mut lines_map: HashMap<u32, LineCoverage> = HashMap::new();
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

#[cfg(test)]
mod tests {
    use super::*;
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

        // Check ranges - now sorted by hits (ascending)
        // hits=2 comes first
        assert_eq!(covered_ranges[0].start, 7);
        assert_eq!(covered_ranges[0].end, 8);
        assert_eq!(covered_ranges[0].hits, 2);

        // hits=3 comes next
        assert_eq!(covered_ranges[1].start, 5);
        assert_eq!(covered_ranges[1].end, 5);
        assert_eq!(covered_ranges[1].hits, 3);

        // hits=5 comes last
        assert_eq!(covered_ranges[2].start, 1);
        assert_eq!(covered_ranges[2].end, 3);
        assert_eq!(covered_ranges[2].hits, 5);
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
}
