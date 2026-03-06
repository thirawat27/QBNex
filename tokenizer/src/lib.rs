//! # Tokenizer
//!
//! This crate provides lexical analysis (tokenization) for QBasic source code.
//!
//! ## Example
//!
//! ```rust
//! use tokenizer::Scanner;
//!
//! let mut scanner = Scanner::new("PRINT \"Hello\"".to_string());
//! let tokens = scanner.tokenize().unwrap();
//! ```

pub mod scanner;
pub mod tokens;

pub use scanner::Scanner;
pub use tokens::Token;
