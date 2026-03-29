//! # Analyzer
//!
//! This crate provides semantic analysis and type checking for QBasic programs.
//!
//! ## Example
//!
//! ```rust
//! use analyzer::{SymbolTable, TypeChecker};
//!
//! let symbol_table = SymbolTable::new();
//! let mut type_checker = TypeChecker::new(symbol_table);
//! // type_checker.check_program(&program).unwrap();
//! ```

pub mod scope;
pub mod type_checker;

pub use scope::SymbolTable;
pub use type_checker::TypeChecker;
