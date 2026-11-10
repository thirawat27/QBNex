//! `qb-runtime` — the QBNex runtime, ported from `internal/c/libqb.cpp` to Rust one
//! subsystem at a time.
//!
//! The BASIC compiler still emits C++ that is compiled together with
//! `internal/c/qbx.cpp` and `libqb.cpp`. Each ported subsystem exports the
//! exact C symbol that libqb.cpp used to define, declared in
//! `include/qb_runtime.h`. The legacy C++ stays available behind a
//! `QBNEX_LEGACY_<SUBSYSTEM>` define so both implementations can be compared.
//!
//! Rules for every module (see docs/rust-port/port-plan.md, Revision 2):
//! - Behaviour is ported bug-for-bug; deviations need a conformance note.
//! - Exported functions never unwind across the FFI boundary (panic = abort).
//! - Pointers coming from C may be unaligned (they often point into string
//!   buffers), so reads and writes go through byte copies.

#![deny(unsafe_op_in_unsafe_fn)]

pub mod alloc;
pub mod arrays;
pub mod clipboard;
pub mod compress;
pub mod console;
mod cmem;
pub mod env;
pub mod error;
pub mod events;
pub mod fileio;
pub mod font;
pub mod fsops;
pub mod gamepad;
pub mod gfs;
pub mod gfx;
pub mod image;
pub mod keybuf;
pub mod keymap;
pub mod mathfn;
pub mod mbf;
pub mod mem;
pub mod net;
pub mod numfmt;
pub mod numparse;
pub mod pixels;
pub mod printer;
pub mod program;
pub mod qbs;
pub mod rnd;
pub mod round;
pub mod screen;
pub mod shell;
pub mod systime;
pub mod timer;
pub mod val;
pub mod waits;
#[cfg(windows)]
pub mod win32;

// Audio runtime (R6): linked into libqb_runtime.a but kept in its own crate (own
// object file) so only programs that reference qb_snd_* pull it in.
extern crate qb_audio;
pub mod raster;
pub mod script;
pub mod window;

/// Version of the C ABI exposed by this crate. Bumped whenever a declaration in
/// `include/qb_runtime.h` changes incompatibly.
pub const QB_ABI_VERSION: u32 = 1;

/// Returns [`QB_ABI_VERSION`]; lets libqb.cpp assert it links the runtime it
/// was compiled against.
#[unsafe(no_mangle)]
pub extern "C" fn qb_abi_version() -> u32 {
    QB_ABI_VERSION
}

/// Borrows `len` bytes at `p` (a null or dangling pointer is fine when `len` is 0).
///
/// # Safety
/// `p` must be readable for `len` bytes when `len > 0`.
pub(crate) unsafe fn ffi_bytes<'a>(p: *const u8, len: usize) -> &'a [u8] {
    if len == 0 || p.is_null() {
        &[]
    } else {
        // SAFETY: caller contract.
        unsafe { std::slice::from_raw_parts(p, len) }
    }
}

/// Mutable variant of [`ffi_bytes`].
///
/// # Safety
/// `p` must be writable for `len` bytes when `len > 0`.
pub(crate) unsafe fn ffi_bytes_mut<'a>(p: *mut u8, len: usize) -> &'a mut [u8] {
    if len == 0 || p.is_null() {
        &mut []
    } else {
        // SAFETY: caller contract.
        unsafe { std::slice::from_raw_parts_mut(p, len) }
    }
}

#[cfg(test)]
mod robustness_tests;
