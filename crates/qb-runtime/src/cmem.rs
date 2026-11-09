//! Buffers handed to C++ that libqb.cpp releases with `free()`.
//!
//! The legacy decoders and the FreeType wrapper returned `malloc`ed memory and
//! libqb.cpp frees it, so the Rust replacements allocate from the C heap too.

use std::os::raw::c_void;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

/// Copies `bytes` into a new `malloc` block. Returns null if allocation fails.
/// A zero-length slice still gets a (1-byte) block so the pointer is never
/// confused with failure.
pub fn to_c_heap(bytes: &[u8]) -> *mut u8 {
    // SAFETY: malloc has no preconditions; the copy stays within both buffers.
    unsafe {
        let p = malloc(bytes.len().max(1)) as *mut u8;
        if !p.is_null() {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
        }
        p
    }
}
