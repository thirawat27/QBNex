//! # Native Codegen
//!
//! This crate provides native code generation and linking for QBasic programs.
//!
//! ## Example
//!
//! ```rust
//! use native_codegen::{CodeGenerator, Linker};
//!
//! // let mut codegen = CodeGenerator::new();
//! // let code = codegen.generate(&program).unwrap();
//! ```

//! Native code generation for QBasic programs
//!
//! This crate provides code generation to Rust source code and
//! native executables via rustc, with future LLVM backend support.

pub mod codegen;
pub mod linker;
pub mod llvm_builder;

pub use codegen::CodeGenerator;
pub use linker::Linker;
pub use llvm_builder::LLVMBuilder;
