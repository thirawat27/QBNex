//! `qb-native` — runtime library of native QBNex executables (D15 final).
//!
//! `qb-exec build --native-link` compiles a program into an object file
//! (`qb_jit::aot::compile_native_object`): its functions `qb_func_<i>`, the
//! program text and the entry table `qb_native_entry`. A standard linker
//! (rust-lld) links that object with this static library into an ordinary
//! executable: the program's code is in the executable's code section,
//! nothing is loaded or compiled at start-up.
//!
//! This library provides
//! - the process entry (`qb_native_start` on Windows, `main` elsewhere),
//!   which runs the program with the VM + the linked native functions
//!   (`qb_exec::run_linked`);
//! - every runtime symbol native code imports (`qb_host_<i>`, `qb_vm_op`,
//!   `qb_jit_*`; feature `native-syms` of qb-exec / qb-jit);
//! - on Windows, the few C runtime pieces Rust code expects (`crt.rs`), so
//!   no C runtime library, MSVC or MinGW is needed to link.

use std::io::{BufWriter, Write};

#[cfg(all(windows, target_env = "msvc"))]
mod crt;

unsafe extern "C" {
    /// Defined by the program's object file.
    static qb_native_entry: qb_jit::aot::NativeEntry;
}

fn run() -> i32 {
    // SAFETY: the linker resolved `qb_native_entry` to the program's table;
    // its pointers refer to data/code of this executable.
    let (src, fns, fingerprint) = unsafe {
        let e = &qb_native_entry;
        (
            std::slice::from_raw_parts(e.program, e.program_len as usize),
            std::slice::from_raw_parts(e.funcs, e.nfuncs as usize),
            e.fingerprint,
        )
    };
    let src = String::from_utf8_lossy(src);
    let stdout = std::io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let result = qb_exec::run_linked(&src, fns, fingerprint, &mut out);
    let _ = out.flush();
    match result {
        Ok(code) => code,
        Err(e) => {
            if e.code < 0 {
                eprintln!("qbnex: invalid program: {}", e.msg);
            } else if e.msg != "reported" {
                eprintln!("Unhandled error #{} on line {}", e.code, e.line);
            }
            1
        }
    }
}

/// Process entry point on Windows (`/entry:qb_native_start`): no C runtime
/// start-up code runs before it.
#[cfg(windows)]
#[unsafe(no_mangle)]
pub extern "C" fn qb_native_start() -> ! {
    let code = run();
    #[cfg(target_env = "msvc")]
    crt::run_atexit();
    std::process::exit(code)
}

/// C `main` (the system C library's start-up code calls it).
#[cfg(not(windows))]
#[unsafe(no_mangle)]
pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    std::process::exit(run())
}
