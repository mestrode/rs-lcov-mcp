//! Coverage analysis module for cargo-llvm-cov integration
//!
//! This module provides functionality to execute cargo-llvm-cov and parse its JSON output
//! to extract coverage metrics and detailed branch coverage information.

pub mod handler;
pub mod parser;

pub use handler::*;
pub use parser::*;
