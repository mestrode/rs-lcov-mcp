//! Cargo clippy command module
//!
//! This module provides the implementation for the cargo clippy command,
//! which runs the Clippy linter to catch common mistakes and improve Rust code.

pub mod handler;

pub use handler::*;
