//! Commands module for cargo command implementations
//!
//! This module contains all cargo command handlers and related functionality.
//! Each submodule represents a specific cargo command (fmt, check, clippy, test, build, clean)
//! or command category (coverage, llvm_cov).

pub mod build;
pub mod check;
pub mod clean;
pub mod clippy;
pub mod coverage;
pub mod fmt;
pub mod llvm_cov;
pub mod test;
pub mod utils;

pub use build::{BuildParams, BuildResult, create_schema as create_build_schema, handle_build};
/// Re-export commonly used types from submodules
pub use check::{CheckParams, CheckResult, create_schema as create_check_schema, handle_check};
pub use clean::{CleanParams, CleanResult, create_schema as create_clean_schema, handle_clean};
pub use clippy::{
    ClippyParams, ClippyResult, create_schema as create_clippy_schema, handle_clippy,
};
pub use coverage::handler::*;
pub use coverage::parser::*;
pub use fmt::{FmtParams, FmtResult, create_schema as create_fmt_schema, handle_fmt};
pub use llvm_cov::handler::*;
pub use test::{TestParams, TestResult, create_schema as create_test_schema, handle_test};
pub use utils::*;
