//! The C runtime pieces Rust code for `*-windows-msvc` refers to, so a native
//! program links without any C runtime library (`/nodefaultlib`).
//!
//! - Thread-local storage: Rust's `thread_local!` uses the PE TLS directory
//!   (`_tls_used`, `_tls_index`), normally supplied by the C runtime.
//! - `_fltused`, `atexit`, the `type_info` vtable and `__CxxFrameHandler3`
//!   (panics abort, so no C++-style unwinding ever reaches it).
//! - `memcpy`, `memset`, `strlen`, `__chkstk`, ... come from `ntdll.dll`
//!   through the import libraries of the native kit.

use std::sync::Mutex;

/// Floating-point use marker the MSVC toolchain expects.
#[unsafe(no_mangle)]
#[used]
pub static _fltused: i32 = 0x9875;

/// TLS slot of this module; set by the Windows loader.
#[unsafe(no_mangle)]
#[used]
pub static mut _tls_index: u32 = 0;

// The linker orders `.tls`, `.tls$` (Rust's thread locals), `.tls$ZZZ`: the
// TLS template is everything between these two markers. Non-zero values keep
// both in initialized writable data, like the variables between them.
#[unsafe(link_section = ".tls")]
#[used]
static mut TLS_START: u64 = 1;
#[unsafe(link_section = ".tls$ZZZ")]
#[used]
static mut TLS_END: u64 = 1;

// TLS callbacks: `.CRT$XLA` (this null), `.CRT$XLB` .. (std's), `.CRT$XLZ`
// (null terminator). The list starts after XLA.
#[unsafe(link_section = ".CRT$XLA")]
#[used]
static XLA: usize = 0;
#[unsafe(link_section = ".CRT$XLZ")]
#[used]
static XLZ: usize = 0;

/// `IMAGE_TLS_DIRECTORY64`
#[repr(C)]
pub struct TlsDirectory {
    start: *const u64,
    end: *const u64,
    index: *const u32,
    callbacks: *const usize,
    zero_fill: u32,
    characteristics: u32,
}

// SAFETY: read-only addresses.
unsafe impl Sync for TlsDirectory {}

/// The linker points the image's TLS directory at this symbol.
#[unsafe(no_mangle)]
#[used]
pub static _tls_used: TlsDirectory = TlsDirectory {
    start: &raw const TLS_START,
    end: &raw const TLS_END,
    index: &raw const _tls_index,
    // SAFETY: one past XLA = first entry of `.CRT$XLB`..`.CRT$XLZ`.
    callbacks: unsafe { (&raw const XLA).add(1) },
    zero_fill: 0,
    characteristics: 0,
};

#[allow(dead_code)]
fn keep_xlz() -> usize {
    XLZ
}

static AT_EXIT: Mutex<Vec<extern "C" fn()>> = Mutex::new(Vec::new());

/// C `atexit`: functions run (last registered first) when the program ends.
#[unsafe(no_mangle)]
pub extern "C" fn atexit(f: extern "C" fn()) -> i32 {
    match AT_EXIT.lock() {
        Ok(mut v) => {
            v.push(f);
            0
        }
        Err(_) => -1,
    }
}

pub(crate) fn run_atexit() {
    let fs = AT_EXIT.lock().map(|mut v| std::mem::take(&mut *v)).unwrap_or_default();
    for f in fs.into_iter().rev() {
        f();
    }
}

/// vtable of C++ `type_info`, referenced by the catch descriptors of Rust's
/// MSVC unwinding code; never used (panics abort).
#[unsafe(export_name = "??_7type_info@@6B@")]
#[used]
pub static TYPE_INFO_VTABLE: [usize; 2] = [0; 2];

/// C++ exception frame handler named by Rust's unwind tables. Panics abort,
/// so it only sees foreign exceptions: never handled here.
#[unsafe(no_mangle)]
pub extern "C" fn __CxxFrameHandler3(
    _record: *mut core::ffi::c_void,
    _frame: *mut core::ffi::c_void,
    _context: *mut core::ffi::c_void,
    _dispatch: *mut core::ffi::c_void,
) -> i32 {
    1 // ExceptionContinueSearch
}
