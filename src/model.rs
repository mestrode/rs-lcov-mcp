use serde::{Deserialize, Serialize};

/// Output for the execute_coverage tool (summary)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoverageSummary {
    /// Whether the coverage analysis was successful
    pub success: bool,
    /// Coverage metrics for the project
    pub coverage: CoverageMetrics,
    /// Result of threshold checking if a threshold was specified
    pub threshold: Option<ThresholdResult>,
    /// Error message if the analysis failed
    pub error: Option<String>,
}

/// Coverage metrics for a project or package
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoverageMetrics {
    /// Overall coverage percentage (0-100)
    pub percentage: f64,
    /// Number of lines covered by tests
    pub lines_covered: u32,
    /// Total number of lines in the codebase
    pub total_lines: u32,
    /// Number of functions covered by tests
    pub functions_covered: u32,
    /// Total number of functions in the codebase
    pub total_functions: u32,
}

/// Result of threshold checking
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThresholdResult {
    /// Whether the coverage percentage met the threshold
    pub met: bool,
    /// The threshold value that was checked
    pub value: f64,
}

/// Output for the get_file_coverage tool (detailed with branch coverage)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileCoverageResult {
    /// Whether the file coverage analysis was successful
    pub success: bool,
    /// Coverage information for each file
    pub files: Vec<FileCoverage>,
    /// Error message if the analysis failed
    pub error: Option<String>,
}

/// Coverage information for a single file
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileCoverage {
    /// File path
    pub path: String,
    /// Line coverage percentage for this file
    pub line_coverage: f64,
    /// Branch coverage percentage for this file
    pub branch_coverage: f64,
    /// Number of lines covered in this file
    pub lines_covered: u32,
    /// Total number of lines in this file
    pub total_lines: u32,
    /// Coverage information for each function in the file
    pub functions: Vec<FunctionCoverage>,
}

/// Coverage information for a function
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCoverage {
    /// Function name
    pub name: String,
    /// Line coverage percentage for this function
    pub line_coverage: f64,
    /// Branch coverage percentage for this function
    pub branch_coverage: f64,
    /// Information about each branch point in the function
    pub branches: Vec<BranchInfo>,
}

/// Information about a branch point
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BranchInfo {
    /// Line number where the branch occurs
    pub line: u32,
    /// Condition for this branch
    pub condition: String,
    /// Whether this branch was covered by tests
    pub covered: bool,
    /// Number of times this branch was hit
    pub hits: u32,
}

/// Information about a single line's coverage
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LineCoverage {
    /// Line number
    pub line: u32,
    /// Whether this line was covered by tests
    pub covered: bool,
    /// Number of times this line was hit
    pub hits: u32,
}

/// A range of consecutive covered lines with the same hit count
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoveredRange {
    /// Starting line number (inclusive)
    pub start: u32,
    /// Ending line number (inclusive)
    pub end: u32,
    /// Number of times these lines were hit
    pub hits: u32,
}

/// Detailed line-by-line coverage for a file using hybrid format
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileLineCoverage {
    /// File path
    pub path: String,
    /// Overall line coverage percentage for this file
    pub line_coverage: f64,
    /// Ranges of consecutive covered lines grouped by hit count
    pub covered_ranges: Vec<CoveredRange>,
    /// Individual line numbers that are NOT covered
    pub uncovered_lines: Vec<u32>,
}

/// Result for get_line_coverage tool
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LineCoverageResult {
    /// Whether the line coverage analysis was successful
    pub success: bool,
    /// Line-by-line coverage for the requested file
    pub file: Option<FileLineCoverage>,
    /// Error message if the analysis failed
    pub error: Option<String>,
}

/// Parameters for get_file_line_coverage tool
#[derive(Debug, Deserialize, Clone)]
pub struct GetFileLineCoverageParams {
    /// Specific file to analyze (required)
    pub file: String,
    /// Analyze entire workspace
    #[serde(default)]
    pub workspace: bool,
    /// Specific crate in workspace
    pub package: Option<String>,
}

/// Parameters for execute_coverage tool
#[derive(Debug, Deserialize, Clone)]
pub struct ExecuteCoverageParams {
    /// Analyze entire workspace
    #[serde(default)]
    pub workspace: bool,
    /// Specific crate in workspace
    pub package: Option<String>,
    /// Minimum coverage percentage (0-100)
    pub threshold: Option<f64>,
}

/// Parameters for get_files_coverage tool
#[derive(Debug, Deserialize, Clone)]
pub struct GetFilesCoverageParams {
    /// Analyze entire workspace
    #[serde(default)]
    pub workspace: bool,
    /// Specific crate in workspace
    pub package: Option<String>,
    /// Specific file to analyze (filters results if provided)
    pub file: Option<String>,
    /// Only include files with coverage below this percentage (0-100)
    pub coverage_below: Option<f64>,
}
