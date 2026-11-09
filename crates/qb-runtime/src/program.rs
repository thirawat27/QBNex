//! Program-wide runtime state shared with generated code (R12b).
//!
//! These globals used to be defined in `internal/c/qbx.cpp` / `libqb.cpp`.
//! Generated C++ reads and writes them directly (`if (new_error) ...`,
//! `return_point[next_return_point++]=...`, `exit_code=...`), so they keep
//! their C names and types. The error globals live in [`crate::error`].
//!
//! Threads: the main loop, the program thread (QBMAIN) and the timer thread
//! all touch some of these, exactly as in the legacy runtime (plain globals,
//! no locking). Rust code reads them with volatile accesses.

#![allow(non_upper_case_globals)]

use core::ptr::{addr_of, addr_of_mut};
use std::sync::atomic::{AtomicI32, Ordering};

/// Set to 1 when the program thread must call `evnt()` at the next line.
#[no_mangle]
pub static mut qbevent: u32 = 0;
/// 1 = program must stop (window closed, END finished, fatal error).
#[no_mangle]
pub static mut stop_program: u8 = 0;
/// 1 = program thread is paused (`evnt()` spins).
#[no_mangle]
pub static mut suspend_program: u8 = 0;
/// 1 = the user chose to close the program (error dialog "No", `_EXIT`).
#[no_mangle]
pub static mut close_program: u8 = 0;
/// Bit 0: program thread finished, bit 1: timer thread finished.
#[no_mangle]
pub static mut exit_ok: i32 = 0;
/// Process exit code (`SYSTEM n`, `END n`).
#[no_mangle]
pub static mut exit_code: i64 = 0;
/// `_EXIT` request bits (1 = window close box, 2 = CTRL+BREAK).
#[no_mangle]
pub static mut exit_value: i32 = 0;
/// Set by events and key presses to end a SLEEP early.
#[no_mangle]
pub static mut sleep_break: i32 = 0;

// GOSUB/RETURN stack used by generated code.
const RETURN_POINTS_INITIAL: usize = 16_384;
static mut RETURN_POINT_INITIAL: [u32; RETURN_POINTS_INITIAL] = [0; RETURN_POINTS_INITIAL];
static mut RETURN_POINT_GROWN: Vec<u32> = Vec::new();

/// Index of the next free GOSUB return slot.
#[no_mangle]
pub static mut next_return_point: u32 = 0;
/// GOSUB return stack (`return_points` entries).
#[no_mangle]
pub static mut return_point: *mut u32 = addr_of_mut!(RETURN_POINT_INITIAL) as *mut u32;
/// Capacity of [`return_point`].
#[no_mangle]
pub static mut return_points: u32 = RETURN_POINTS_INITIAL as u32;

/// Volatile read of a global another thread may change.
#[inline]
pub(crate) fn load<T: Copy>(p: *const T) -> T {
    // SAFETY: `p` points at one of the statics above.
    unsafe { core::ptr::read_volatile(p) }
}

/// Volatile write of a global another thread may read.
#[inline]
pub(crate) fn store<T: Copy>(p: *mut T, v: T) {
    // SAFETY: `p` points at one of the statics above.
    unsafe { core::ptr::write_volatile(p, v) }
}

/// `stop_program != 0`.
pub fn stop_requested() -> bool {
    load(addr_of!(stop_program)) != 0
}

/// Sets `qbevent = 1` (the program thread checks events at its next line).
pub fn signal_event() {
    store(addr_of_mut!(qbevent), 1);
}

/// `exit_ok |= bits` (atomic: the program and timer threads both set bits).
pub fn exit_ok_or(bits: i32) {
    // SAFETY: exit_ok is a properly aligned i32 that lives for the whole program.
    unsafe { AtomicI32::from_ptr(addr_of_mut!(exit_ok)) }.fetch_or(bits, Ordering::SeqCst);
}

/// `func__exit`: returns and clears the pending `_EXIT` request bits.
/// (`exit_blocked` stays in libqb.cpp with the window code.)
pub fn take_exit_request() -> i32 {
    // SAFETY: exit_value is a properly aligned i32 that lives for the whole program.
    unsafe { AtomicI32::from_ptr(addr_of_mut!(exit_value)) }.swap(0, Ordering::SeqCst)
}

/// `more_return_points`: doubles the GOSUB return stack, keeping its
/// contents. Returns the BASIC error to raise (256, "Out of stack space")
/// when the stack cannot grow.
pub fn grow_return_points() -> i32 {
    // SAFETY: only the program thread runs GOSUB, so nothing else touches the
    // stack while it is reallocated.
    unsafe {
        let n = return_points;
        if n > 2_147_483_647 {
            return 256;
        }
        let new_len = (n as usize) * 2;
        let old = core::slice::from_raw_parts(return_point, n as usize).to_vec();
        let grown = &mut *addr_of_mut!(RETURN_POINT_GROWN);
        let mut v = Vec::new();
        if v.try_reserve_exact(new_len).is_err() {
            return 256;
        }
        v.extend_from_slice(&old);
        v.resize(new_len, 0);
        *grown = v;
        return_points = new_len as u32;
        return_point = grown.as_mut_ptr();
        0
    }
}

/// C ABI of [`take_exit_request`] (`func__exit`).
#[no_mangle]
pub extern "C" fn qb_take_exit_request() -> i32 {
    take_exit_request()
}

/// C ABI of [`grow_return_points`] (`more_return_points`).
#[no_mangle]
pub extern "C" fn qb_more_return_points() -> i32 {
    grow_return_points()
}

#[cfg(test)]
pub(crate) static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn return_stack_grows_and_keeps_entries() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            *return_point.add(5) = 77;
            *return_point.add(RETURN_POINTS_INITIAL - 1) = 99;
            let before = return_points;
            assert_eq!(grow_return_points(), 0);
            assert_eq!({ return_points }, before * 2);
            assert_eq!(*return_point.add(5), 77);
            assert_eq!(*return_point.add(before as usize - 1), 99);
            assert_eq!(*return_point.add(before as usize), 0);
            *return_point.add(return_points as usize - 1) = 1;
        }
    }

    #[test]
    fn exit_request_is_taken_once() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        store(addr_of_mut!(exit_value), 3);
        assert_eq!(take_exit_request(), 3);
        assert_eq!(take_exit_request(), 0);
        exit_ok_or(2);
        exit_ok_or(1);
        assert_eq!(load(addr_of!(exit_ok)) & 3, 3);
        store(addr_of_mut!(exit_ok), 0);
    }
}
