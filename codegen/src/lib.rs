//! # Native Codegen
//!
//! This crate provides native code generation and linking for QBasic programs.
//!
//! ## Example
//!
//! ```rust
//! use native_codegen::CodeGenerator;
//!
//! // let mut codegen = CodeGenerator::new();
//! // let code = codegen.generate(&program).unwrap();
//! ```

//! Native code generation for QBasic programs
//!
//! This crate provides production Rust source generation and
//! native executables via rustc/cargo.

pub mod backend;
pub mod codegen;

pub use backend::{
    generate_with_backend, NativeBackendKind, NativeBackendOptions, NativeTextBackend,
    RustTextBackend,
};
pub use codegen::CodeGenerator;
