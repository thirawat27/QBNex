//! Test library for `tests/calls.rs` (built as a cdylib by the test itself).

#![allow(clippy::pedantic)] // test code: casts and short names on purpose

/// # Safety
/// `s` must point to a NUL-terminated string.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn qb_test_mix(a: i16, b: f64, c: i64, d: f32, s: *const u8, e: u8, f: i32, g: f64, h: i64) -> f64 {
    let mut n = 0usize;
    // SAFETY: caller passes a NUL-terminated string.
    while unsafe { *s.add(n) } != 0 {
        n += 1;
    }
    a as f64 + b + c as f64 + d as f64 + n as f64 + e as f64 + f as f64 + g + h as f64
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_test_neg_i16(x: i16) -> i16 {
    -x
}
