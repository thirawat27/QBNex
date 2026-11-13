//! `qb-exec` — runs QBNex programs: the compiler (`qb --ir`) emits QIR
//! (`docs/dev/qir.md`, crate `qb-ir`), and qb-exec executes it.
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
//! Environment: `QB_JIT=off|eager|<threshold>`, `QB_EXEC_DISASM=1` prints the bytecode,
//! `QB_PROFILE=<file>` writes the hottest functions and source lines (a sample every
//! millisecond; use it with `QB_JIT=off`).

pub mod bytecode;
mod jit;
include!(concat!(env!("OUT_DIR"), "/ext_symbols.rs"));
mod lower;
mod mem;
mod rt;
mod vm;

pub use vm::{EventHandle, Vm};

/// Sets the program's argv as BASIC sees it (COMMAND$, _COMMANDCOUNT): [program, args...].
pub fn set_program_args(args: Vec<String>) {
    rt::builtins::set_program_args(args)
}
/// Debugger hook (feature `debug`), used by `qb-dap`.
#[cfg(feature = "debug")]
pub use vm::debug;

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
    if qb_runtime::error::end_prompt_wanted() {
        vm.set_end_prompt(Some(wait_end_key));
    }
    vm.run_status()
}

/// Program stdout of the runner and of native executables. On Windows a line
/// ends with CR LF, as the C runtime's text-mode stdout wrote it (tools such
/// as `findstr /x` need the CR on redirected output).
pub struct Stdout<W: Write>(pub W);

impl<W: Write> Write for Stdout<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if cfg!(windows) {
            for (k, seg) in buf.split(|&c| c == b'\n').enumerate() {
                if k > 0 {
                    self.0.write_all(b"\r\n")?;
                }
                self.0.write_all(seg)?;
            }
            Ok(buf.len())
        } else {
            self.0.write(buf)
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

/// Runs a QIR program with its native image (from [`build_native`]).
pub fn run_ir_native(src: &str, image: &[u8], out: &mut dyn Write) -> Result<i32, RunError> {
    let m = compile(src)?;
    let mut vm = Vm::new(&m, out);
    vm.load_native(image).map_err(RunError::load)?;
    vm.run_status()
}

/// Runs a compiled program as its executable does (`qb-exec`): unhandled
/// errors are reported on stderr and the program continues, as a QB64
/// program does without a window. `image`: native image from [`build_native`].
pub fn run_program(src: &str, image: Option<&[u8]>, out: &mut dyn Write) -> Result<i32, RunError> {
    let m = compile(src)?;
    let mut vm = Vm::new(&m, out);
    if let Some(img) = image {
        vm.load_native(img).map_err(RunError::load)?;
    }
    vm.set_continue_unhandled(true);
    // END waits for a key only when somebody could press one (the runtime's END
    // policy: stdin and stdout on the console, QBNEX_NO_ERROR_DIALOG unset)
    if qb_runtime::error::end_prompt_wanted() {
        vm.set_end_prompt(Some(wait_end_key));
    }
    vm.run_status()
}

/// The key END waits for (console branch of sub_end).
fn wait_end_key() {
    #[cfg(windows)]
    {
        use qb_runtime::console::stdio;
        stdio::flush_input();
        while stdio::console_input() != 1 {}
    }
    #[cfg(not(windows))]
    {
        let mut line = String::new();
        let _ = std::io::stdin().read_line(&mut line);
    }
}

/// True for a program whose screen is a window: every program but a
/// `$CONSOLE:ONLY` one (`meta console only`). `$CONSOLE` alone gives the
/// program a console next to its window, as in QB64.
pub fn wants_window(src: &str) -> bool {
    !src.lines().any(|l| l.trim() == "meta console only")
}

/// True for a window program that also asked for the console (`$CONSOLE`).
pub fn wants_console(src: &str) -> bool {
    src.lines().any(|l| l.trim().starts_with("meta console"))
}

/// `meta resize on|stretch|smooth` (`$RESIZE`): 0 without it, else 1, 2, 3.
pub fn window_resize(src: &str) -> i32 {
    match src.lines().find_map(|l| l.trim().strip_prefix("meta resize ")) {
        Some("stretch") => 2,
        Some("smooth") => 3,
        Some(_) => 1,
        None => 0,
    }
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
