//! Commands module for cargo command implementations
//!
//! This module contains all cargo command handlers and related functionality.
//! Each submodule represents a specific cargo command (fmt, check, clippy, test, build)
//! or command category (coverage, llvm_cov).

pub mod check;
pub mod coverage;
pub mod fmt;
pub mod llvm_cov;
pub mod utils;

/// Re-export commonly used types from submodules
pub use check::{CheckParams, CheckResult, create_schema as create_check_schema, handle_check};
pub use coverage::handler::*;
pub use coverage::parser::*;
pub use fmt::{FmtParams, FmtResult, create_schema as create_fmt_schema, handle_fmt};
pub use llvm_cov::handler::*;
pub use utils::*;
