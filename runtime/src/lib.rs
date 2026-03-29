//! # VM Engine
//!
//! This crate provides bytecode compilation and virtual machine execution
//! for QBasic programs.
//!
//! ## Example
//!
//! ```rust
//! use vm_engine::{BytecodeCompiler, VM};
//!
//! // let mut compiler = BytecodeCompiler::new(program);
//! // let bytecode = compiler.compile().unwrap();
//! // let mut vm = VM::new(bytecode);
//! // vm.run().unwrap();
//! ```

pub mod builtin_functions;
pub mod compiler;
pub mod opcodes;
pub mod runtime;

pub use builtin_functions::{compile_builtin_function, is_builtin_function};
pub use compiler::BytecodeCompiler;
pub use opcodes::OpCode;
pub use runtime::{RuntimeState, VM};
