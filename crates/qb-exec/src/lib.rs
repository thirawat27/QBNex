//! `qb-exec` — runs QBNex programs: the compiler (`qb --ir`) emits QIR
//! (`docs/rust-port/qir-v1.md`, crate `qb-ir`), and qb-exec executes it.
//!
//! One pipeline:
//!
//! 1. `qb_ir::parse` + `qb_ir::validate` — QIR text to a checked module
//! 2. [`compile`] (`lower.rs`) — QIR to typed register bytecode ([`bytecode`]):
//!    variables promoted to registers, labels to pcs, builtins resolved,
//!    superinstructions for the hot shapes; memory mode only for what needs
//!    an address (TYPE, `STRING * n`, pointers)
//! 3. [`Vm`] (`vm.rs`) — the register VM runs the bytecode; the QBasic
//!    runtime (`rt/`: PRINT, files, strings, `_MEM`, TCP, sound) sits behind
//!    one `rt::Host` interface
//! 4. `jit.rs` + `qb-jit` — hot loops and numeric SUB/FUNCTIONs of the same
//!    bytecode are compiled to native code with Cranelift (JIT), or ahead of
//!    time into an image stored next to the program ([`build_native`], `qb-exec build`)
//!
//! Environment: `QB_JIT=off|eager|<threshold>`, `QB_EXEC_DISASM=1` prints the bytecode.

pub mod bytecode;
mod jit;
mod lower;
mod mem;
mod rt;
mod vm;

pub use vm::{EventHandle, Vm};
/// Debugger hook (feature `debug`), used by `qb-dap`.
#[cfg(feature = "debug")]
pub use vm::debug;

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
    // R12b core runtime (allocators, errors, events, waits).
    #[unsafe(no_mangle)]
    pub static mut cmem_static_pointer: *mut u8 = core::ptr::null_mut();
    #[unsafe(no_mangle)]
    pub static mut cmem_dynamic_base: *mut u8 = core::ptr::null_mut();
    #[unsafe(no_mangle)]
    pub extern "C" fn qb_host_message_box(_m: *const core::ffi::c_char, _t: *const core::ffi::c_char, _k: i32) -> i32 {
        std::process::abort()
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn qb_host_end() {
        std::process::abort()
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn qb_host_evnt() {
        std::process::abort()
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn qb_host_devices_init() {
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

/// Lowers a parsed QIR module to validated bytecode.
pub fn lower(m: &qb_ir::Module) -> Result<bytecode::Module, RunError> {
    lower::compile(m).map_err(RunError::load)
}

/// Parses QIR text and lowers it to validated bytecode.
pub fn compile(src: &str) -> Result<bytecode::Module, RunError> {
    let m = qb_ir::parse(src).map_err(|e| RunError::load(e.to_string()))?;
    let bc = lower(&m)?;
    if std::env::var_os("QB_EXEC_DISASM").is_some() {
        eprint!("{}", bc.disasm());
    }
    Ok(bc)
}

/// Compiles a QIR program ahead of time into a native image for this
/// executable (D15): `qb-exec build` stores it next to the program, and
/// [`run_ir_native`] runs it without any compilation at start-up.
pub fn build_native(src: &str) -> Result<Vec<u8>, RunError> {
    let m = compile(src)?;
    jit::build_image(&m, &vm::vm_hosts()).map_err(RunError::load)
}

/// Compiles a QIR program into an object file for a standard linker (D15
/// final): its functions, the program text and the `qb_native_entry` table.
/// Linked with the `qb-native` runtime library it is a native executable.
pub fn build_native_object(src: &str) -> Result<Vec<u8>, RunError> {
    let m = compile(src)?;
    jit::build_object(&m, src.as_bytes(), &vm::vm_hosts()).map_err(RunError::load)
}

/// Runs a program linked into this executable (qb-native): `fns` and
/// `fingerprint` come from the object's entry table. Same behaviour as
/// [`run_program`].
pub fn run_linked(src: &str, fns: &[qb_jit::NativeFn], fingerprint: u64, out: &mut dyn Write) -> Result<i32, RunError> {
    let m = compile(src)?;
    let mut vm = Vm::new(&m, out);
    vm.load_linked(fns, fingerprint).map_err(RunError::load)?;
    vm.set_continue_unhandled(true);
    vm.run_status()
}

/// Runs a QIR program with its native image (from [`build_native`]).
pub fn run_ir_native(src: &str, image: &[u8], out: &mut dyn Write) -> Result<i32, RunError> {
    let m = compile(src)?;
    let mut vm = Vm::new(&m, out);
    vm.load_native(image).map_err(RunError::load)?;
    vm.run_status()
}

/// Runs a compiled program as its executable does (`qb-exec`): unhandled
/// errors are reported on stderr and the program continues, like the
/// headless C++ runtime. `image`: native image from [`build_native`].
pub fn run_program(src: &str, image: Option<&[u8]>, out: &mut dyn Write) -> Result<i32, RunError> {
    let m = compile(src)?;
    let mut vm = Vm::new(&m, out);
    if let Some(img) = image {
        vm.load_native(img).map_err(RunError::load)?;
    }
    vm.set_continue_unhandled(true);
    vm.run_status()
}

/// True for a program compiled without `$CONSOLE` / `$CONSOLE:ONLY` (no
/// `meta console` line): its executable shows a window, as a C++-built one does.
pub fn wants_window(src: &str) -> bool {
    !src.lines().map(str::trim).any(|l| l == "meta console" || l.starts_with("meta console "))
}

/// `meta screenhide` (`$SCREENHIDE`): the window starts hidden.
pub fn screen_hidden(src: &str) -> bool {
    src.lines().any(|l| l.trim() == "meta screenhide")
}

/// [`run_program`] for a GUI program: the screen is a window (or the
/// headless stand-in, `QBNEX_HEADLESS=1`). Runs on a program thread while
/// [`run_window_main`] owns the main thread.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_program_window(src: &str, image: Option<&[u8]>, out: &mut dyn Write) -> Result<i32, RunError> {
    let m = compile(src)?;
    let mut vm = Vm::new(&m, out);
    if let Some(img) = image {
        vm.load_native(img).map_err(RunError::load)?;
    }
    vm.set_continue_unhandled(true);
    vm.set_window();
    vm.run_status()
}

/// The window layer of a GUI program, on the main thread (never returns
/// while a window or the headless stand-in runs; the program thread ends the
/// process).
#[cfg(not(target_arch = "wasm32"))]
pub use rt::window::run_main as run_window_main;
/// Parses and runs a QIR program, writing console output to `out`.
pub fn run_ir(src: &str, out: &mut dyn Write) -> Result<(), RunError> {
    run_ir_status(src, out).map(|_| ())
}

/// Like [`run_ir`] but returns the program's exit code (`SYSTEM n`).
pub fn run_ir_status(src: &str, out: &mut dyn Write) -> Result<i32, RunError> {
    let m = compile(src)?;
    Vm::new(&m, out).run_status()
}
