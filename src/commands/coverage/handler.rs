//! Coverage handler module for cargo-llvm-cov integration
//!
//! This module provides the handler functions for coverage analysis.

use crate::commands::coverage::parser::{parse_file_coverage, parse_line_coverage};
use crate::commands::llvm_cov::execute_llvm_cov;
use crate::error::Result;
use crate::model::{
    CoverageMetrics, CoverageSummary, ExecuteCoverageParams, FileCoverageResult,
    GetFileLineCoverageParams, GetFilesCoverageParams, LineCoverageResult, ThresholdResult,
};
use serde_json::Map;

/// Validate execute coverage parameters
pub fn validate_execute_params(params: &ExecuteCoverageParams) -> Result<()> {
    if params.workspace && params.package.is_some() {
        return Err(crate::error::CoverageError::Validation(
            "Cannot specify both 'workspace' and 'package'. Use one or the other, or neither for the current package.".to_string(),
        ));
    }

    if let Some(threshold) = params.threshold
        && !(0.0..=100.0).contains(&threshold)
    {
        return Err(crate::error::CoverageError::Validation(
            "Threshold must be between 0 and 100".to_string(),
        ));
    }

    Ok(())
}

/// Validate get files coverage parameters
pub fn validate_file_params(params: &GetFilesCoverageParams) -> Result<()> {
    if params.workspace && params.package.is_some() {
        return Err(crate::error::CoverageError::Validation(
            "Cannot specify both 'workspace' and 'package'. Use one or the other, or neither for the current package.".to_string(),
        ));
    }

    if let Some(coverage_below) = params.coverage_below
        && !(0.0..=100.0).contains(&coverage_below)
    {
        return Err(crate::error::CoverageError::Validation(
            "coverage_below must be between 0 and 100".to_string(),
        ));
    }

    Ok(())
}

/// Validate get file line coverage parameters
pub fn validate_line_params(params: &GetFileLineCoverageParams) -> Result<()> {
    if params.workspace && params.package.is_some() {
        return Err(crate::error::CoverageError::Validation(
            "Cannot specify both 'workspace' and 'package'. Use one or the other, or neither for the current package.".to_string(),
        ));
    }

    if params.file.is_empty() {
        return Err(crate::error::CoverageError::Validation(
            "file parameter is required".to_string(),
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
fn create_property(type_str: &str, description: &str) -> Map<String, serde_json::Value> {
    let mut props = Map::new();
    props.insert(
        "type".to_string(),
        serde_json::Value::String(type_str.to_string()),
    );
    props.insert(
        "description".to_string(),
        serde_json::Value::String(description.to_string()),
    );
    props
}

/// Create schema for get_coverage_summary tool
pub fn create_execute_schema() -> Map<String, serde_json::Value> {
    let mut properties = Map::new();

    properties.insert(
        "workspace".to_string(),
        serde_json::Value::Object(create_property("boolean", "Analyze entire workspace")),
    );

    properties.insert(
        "package".to_string(),
        serde_json::Value::Object(create_property("string", "Specific crate in workspace")),
    );

    properties.insert(
        "threshold".to_string(),
        serde_json::Value::Object(create_property(
            "number",
            "Minimum coverage percentage (0-100)",
        )),
    );

    let mut schema = Map::new();
    schema.insert(
        "type".to_string(),
        serde_json::Value::String("object".to_string()),
    );
    schema.insert(
        "properties".to_string(),
        serde_json::Value::Object(properties),
    );
    schema.insert(
        "description".to_string(),
        serde_json::Value::String("Returns overall coverage summary. Coverage percentage is calculated as: (total lines covered / total executable lines) * 100. Note: Files with no executable code (e.g., struct definitions only) may be excluded from the analysis by cargo-llvm-cov.".to_string()),
    );

    schema
}

/// Create schema for get_files_coverage tool
pub fn create_file_schema() -> Map<String, serde_json::Value> {
    let mut properties = Map::new();

    properties.insert(
        "workspace".to_string(),
        serde_json::Value::Object(create_property("boolean", "Analyze entire workspace")),
    );

    properties.insert(
        "package".to_string(),
        serde_json::Value::Object(create_property("string", "Specific crate in workspace")),
    );

    properties.insert(
        "file".to_string(),
        serde_json::Value::Object(create_property(
            "string",
            "Specific file to analyze (filters results if provided)",
        )),
    );

    properties.insert(
        "coverage_below".to_string(),
        serde_json::Value::Object(create_property(
            "number",
            "Only include files with coverage below this percentage (0-100)",
        )),
    );

    let mut schema = Map::new();
    schema.insert(
        "type".to_string(),
        serde_json::Value::String("object".to_string()),
    );
    schema.insert(
        "properties".to_string(),
        serde_json::Value::Object(properties),
    );
    schema.insert(
        "description".to_string(),
        serde_json::Value::String("Returns per-file coverage details. Each file's coverage percentage is calculated as: (file lines covered / file total executable lines) * 100. Overall percentage in summary uses: (sum of all covered lines / sum of all executable lines) * 100. Note: Files with no executable code may be excluded by cargo-llvm-cov.".to_string()),
    );

    schema
}

/// Create schema for get_line_coverage tool
pub fn create_line_schema() -> Map<String, serde_json::Value> {
    let mut properties = Map::new();

    properties.insert(
        "file".to_string(),
        serde_json::Value::Object(create_property(
            "string",
            "Specific file to analyze (required)",
        )),
    );

    properties.insert(
        "workspace".to_string(),
        serde_json::Value::Object(create_property("boolean", "Analyze entire workspace")),
    );

    properties.insert(
        "package".to_string(),
        serde_json::Value::Object(create_property("string", "Specific crate in workspace")),
    );

    let mut schema = Map::new();
    schema.insert(
        "type".to_string(),
        serde_json::Value::String("object".to_string()),
    );
    schema.insert(
        "properties".to_string(),
        serde_json::Value::Object(properties),
    );
    schema.insert(
        "description".to_string(),
        serde_json::Value::String("Returns line-by-line coverage for a specific file. Each line's coverage indicates whether it was executed during tests. Note: Only lines with executable code are included in the analysis.".to_string()),
    );

    // Mark file as required
    let required = vec![serde_json::Value::String("file".to_string())];
    schema.insert("required".to_string(), serde_json::Value::Array(required));

    schema
}
