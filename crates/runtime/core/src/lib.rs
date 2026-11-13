//! `qb-runtime` — the QBNex runtime library (console, graphics, files, input,
//! networking, memory, number formatting) that the QIR runner `qb-exec` calls.
//! Many entry points keep the C ABI and the names of the QB64 runtime they
//! replaced, so native code produced by the JIT and the native linker can call
//! them directly.
//!
//! Rules for every module:
//! - Behaviour matches QB64/QBasic bug-for-bug; every intentional difference
//!   has an ID in docs/dev/deviations.md and a conformance case.
//! - Exported functions never unwind across the FFI boundary (panic = abort).
//! - Pointers coming from C may be unaligned (they often point into string
//!   buffers), so reads and writes go through byte copies.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![deny(unsafe_op_in_unsafe_fn)]

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
#[cfg(not(target_arch = "wasm32"))]
pub mod gamepad;
pub mod gfs;
pub mod gfx;
#[cfg(feature = "gpu")]
pub mod gpu_present;
#[cfg(all(test, feature = "gpu"))]
mod gpu_tests;
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
pub mod rnd;
pub mod round;
pub mod scene3d;
pub mod screen;
pub mod shell;
pub mod systime;
pub mod timer;
pub mod val;
#[cfg(windows)]
pub mod win32;

// Audio runtime: kept in its own crate (own object file); linked into the runner.
extern crate qb_audio;
// Extension library: the Rust side of the stdlib's ext.* modules.
#[cfg(feature = "ext")]
extern crate qb_ext;
#[cfg(not(target_arch = "wasm32"))]
pub mod gl;
pub mod raster;
pub mod script;
#[cfg(not(target_arch = "wasm32"))]
pub mod window;

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
#[cfg(test)]
mod props_tests;
