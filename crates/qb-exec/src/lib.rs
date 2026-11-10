//! `qb-exec` — runs QIR, the IR the BASIC compiler emits with `--ir`
//! (see docs/rust-port/r11-emitter-design.md for the format).
//!
//! R11.5: the program is loaded once into typed register bytecode
//! ([`bytecode`], produced by `lower.rs`) and executed by the register VM
//! ([`Vm`]). Labels are resolved to instruction indices and names to
//! registers at load time, so GOTO/GOSUB/RESUME/ON ERROR are plain jumps and
//! the interpreter never looks at names or type tags.
//!
//! Numbers keep their QBasic type statically (INTEGER, LONG, _INTEGER64,
//! SINGLE, DOUBLE). SINGLE values are held at double precision until they are
//! stored or printed, matching what the C++ backend does (`y# = 1 / 3` keeps
//! 53 bits). Formatting and rounding come from `qb-runtime`, the same code the
//! C++ path links.

pub mod bytecode;
mod builtins;
mod jit;
mod lower;
pub mod v1;
mod value;
mod vm;

pub use vm::{EventHandle, Vm};

/// `qb-runtime` is linked as a whole: its string engine (R3) refers to symbols that
/// libqb.cpp/qbx.cpp define for C++ programs. qb-exec never calls that code, so
/// it only needs the symbols to exist.
#[allow(non_upper_case_globals)]
mod libqb_host {
    #[unsafe(no_mangle)]
    pub static mut cmem_sp: u32 = 0;
    #[unsafe(no_mangle)]
    pub static mut dblock: isize = 0;
    #[unsafe(no_mangle)]
    pub static mut cmem: [u8; 1] = [0];
    #[unsafe(no_mangle)]
    pub extern "C" fn qb_host_error(_n: i32) {
        std::process::abort()
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn qb_host_field_free(_s: *mut core::ffi::c_void) {
        std::process::abort()
    }
}

use std::io::Write;

/// Runtime error raised by a QIR program (QBasic error code + source line).
/// `code < 0` = the program could not be loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunError {
    pub code: i32,
    pub line: u32,
    pub msg: String,
}

impl RunError {
    pub(crate) fn load(msg: impl Into<String>) -> Self {
        RunError { code: -1, line: 0, msg: msg.into() }
    }
}

/// Loads QIR text into typed register bytecode (validated, ready to run).
pub fn compile(src: &str) -> Result<bytecode::Module, RunError> {
    lower::compile_v0(src)
}

/// Parses and runs a QIR program, writing console output to `out`.
pub fn run_ir(src: &str, out: &mut dyn Write) -> Result<(), RunError> {
    run_ir_status(src, out).map(|_| ())
}

/// Like [`run_ir`] but returns the program's exit code (`SYSTEM n`).
/// The `QIR <n>` header selects the loader.
pub fn run_ir_status(src: &str, out: &mut dyn Write) -> Result<i32, RunError> {
    let head = src.trim_start().lines().find(|l| !l.trim().is_empty() && !l.trim_start().starts_with(';'));
    if head.is_some_and(|h| h.split_whitespace().nth(1) != Some("0")) {
        let m = qb_ir::parse(src).map_err(|e| RunError::load(e.to_string()))?;
        return v1::run(&m, out);
    }
    let m = compile(src)?;
    let mut vm = Vm::new(&m, out);
    vm.run().map(|_| 0)
}
