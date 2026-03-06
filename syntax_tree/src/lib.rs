//! # Syntax Tree
//!
//! This crate provides parsing and Abstract Syntax Tree (AST) representation
//! for QBasic programs.
//!
//! ## Example
//!
//! ```rust
//! use syntax_tree::Parser;
//!
//! let mut parser = Parser::new("PRINT 42".to_string()).unwrap();
//! let program = parser.parse().unwrap();
//! ```

pub mod ast_nodes;
pub mod parser;

pub use ast_nodes::{Expression, FunctionCall, Label, LineNumber, Program, Statement, Variable};
pub use parser::Parser;
