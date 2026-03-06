//! # Core Types
//!
//! This crate provides shared data types, error handling, and memory management
//! for the QBNex compiler and interpreter.
//!
//! ## Example
//!
//! ```rust
//! use core_types::{QType, QResult, QError};
//!
//! fn my_function() -> QResult<QType> {
//!     Ok(QType::Integer(42))
//! }
//! ```

//! Core types and error handling for QBNex
//!
//! This crate provides fundamental data types, error definitions,
//! and memory management utilities used throughout the QBNex compiler.

pub mod data_types;
pub mod errors;
pub mod memory_map;

pub use data_types::QType;
pub use errors::{QError, QResult};
pub use memory_map::DosMemory;
