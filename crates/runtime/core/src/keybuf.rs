//! Keyboard buffers shared between the window thread (producer) and the
//! user program thread (consumer): the `_KEYHIT` ring, the held-key list
//! behind `_KEYDOWN`, and the BIOS shift-state bytes derived from it.
//!
//! Legacy (`libqb.cpp`) kept these in plain globals written by the GLUT thread
//! and read by the program thread without synchronisation (debt F3). Here every
//! operation runs under one mutex, and [`InputLock`] serialises the multi-step
//! `keydown()`/`keyup()` sequences in libqb.cpp against each other.
//!
//! Semantics are ported bug-for-bug and checked by `tests/diff/diff_keybuf.cpp`.

use std::sync::{Condvar, Mutex, MutexGuard};
use std::thread::{self, ThreadId};

use crate::keymap::{qbvk, QBK, VK};

/// `QBK_SCROLL_LOCK_MODE` / `QBK_INSERT_MODE` (libqb.cpp).
pub const QBK_SCROLL_LOCK_MODE: u32 = 11;
pub const QBK_INSERT_MODE: u32 = 12;

const KEYHIT_SIZE: usize = 8192;
const KEYHIT_MASK: usize = KEYHIT_SIZE - 1;

/// All keyboard buffer state.
pub struct KeyState {
    keyhit: Vec<i64>,
    keyhit_next: usize,
    keyhit_nextfree: usize,
    held: Vec<u32>,
    held_bind: Vec<u32>,
}

impl Default for KeyState {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyState {
    pub fn new() -> Self {
        KeyState {
            keyhit: vec![0; KEYHIT_SIZE],
            keyhit_next: 0,
            keyhit_nextfree: 0,
            held: Vec::new(),
            held_bind: Vec::new(),
        }
    }

    /// Adds a `_KEYHIT` message; when full the oldest message is dropped.
    pub fn keyhit_push(&mut self, v: i64) {
        let z = (self.keyhit_nextfree + 1) & KEYHIT_MASK;
        if z == self.keyhit_next {
            // remove oldest message when cyclic buffer is full
            self.keyhit_next = (self.keyhit_next + 1) & KEYHIT_MASK;
        }
        self.keyhit[self.keyhit_nextfree] = v;
        self.keyhit_nextfree = z;
    }

    /// `_KEYHIT`: low 32 bits of the oldest message, or 0 when empty.
    pub fn keyhit_pop(&mut self) -> i32 {
        if self.keyhit_next != self.keyhit_nextfree {
            let x = self.keyhit[self.keyhit_next] as i32; // *(int32*)&keyhit[i] on little endian
            self.keyhit_next = (self.keyhit_next + 1) & KEYHIT_MASK;
            return x;
        }
        0
    }

    pub fn keyhit_clear(&mut self) {
        self.keyhit_nextfree = 0;
        self.keyhit_next = 0;
    }

    /// Exact membership in the held list.
    fn held_raw(&self, x: u32) -> bool {
        self.held.contains(&x)
    }

    /// `keyheld(x)`: held list membership plus the numpad multimapping.
    pub fn keyheld(&self, x: u32) -> bool {
        if self.held_raw(x) {
            return true;
        }
        // check multimapped NUMPAD keys
        if (42..=57).contains(&x) {
            if (48..=57).contains(&x) {
                return self.keyheld(VK + qbvk::KP0 + (x - 48));
            }
            match x {
                46 => return self.keyheld(VK + qbvk::KP_PERIOD),
                47 => return self.keyheld(VK + qbvk::KP_DIVIDE),
                42 => return self.keyheld(VK + qbvk::KP_MULTIPLY),
                45 => return self.keyheld(VK + qbvk::KP_MINUS),
                43 => return self.keyheld(VK + qbvk::KP_PLUS),
                _ => {}
            }
        }
        if x == 13 {
            return self.keyheld(VK + qbvk::KP_ENTER);
        }
        if x & 0xFF00 != 0 {
            let x2 = (x >> 8) & 255;
            let kp = match x2 {
                82 => Some(0),
                79 => Some(1),
                80 => Some(2),
                81 => Some(3),
                75 => Some(4),
                76 => Some(5),
                77 => Some(6),
                71 => Some(7),
                72 => Some(8),
                73 => Some(9),
                83 => Some(qbvk::KP_PERIOD - qbvk::KP0),
                _ => None,
            };
            if let Some(k) = kp {
                return self.keyheld(QBK + k);
            }
        }
        false
    }

    /// `keyheld_add(x)` with the current `bindkey`.
    pub fn keyheld_add(&mut self, x: u32, bind: u32) {
        if self.held_raw(x) {
            return; // already in buffer
        }
        self.held.push(x);
        self.held_bind.push(bind);
    }

    pub fn keyheld_remove(&mut self, x: u32) {
        if let Some(i) = self.held.iter().position(|&k| k == x) {
            self.held.remove(i);
            self.held_bind.remove(i);
        }
    }

    /// `keyheld_unbind(x)`: the held key whose binding is `x` (legacy then
    /// calls `keyup` on it), or `None`.
    pub fn keyheld_bound_to(&self, x: u32) -> Option<u32> {
        self.held_bind
            .iter()
            .position(|&b| b == x)
            .map(|i| self.held[i])
    }

    /// `update_shift_state()`: the bytes for cmem 0x417, 0x418 and 0x496.
    pub fn shift_state(&self) -> [u8; 3] {
        let h = |k: u32| self.keyheld(k);
        let mut a = 0u8; // 0:417h shift status
        if h(VK + qbvk::RSHIFT) {
            a |= 1;
        }
        if h(VK + qbvk::LSHIFT) {
            a |= 2;
        }
        if h(VK + qbvk::LCTRL) || h(VK + qbvk::RCTRL) {
            a |= 4;
        }
        if h(VK + qbvk::LALT) || h(VK + qbvk::RALT) {
            a |= 8;
        }
        if h(QBK + QBK_SCROLL_LOCK_MODE) {
            a |= 16;
        }
        if h(VK + qbvk::NUMLOCK) {
            a |= 32;
        }
        if h(VK + qbvk::CAPSLOCK) {
            a |= 64;
        }
        if h(QBK + QBK_INSERT_MODE) {
            a |= 128;
        }
        let mut b = 0u8; // 0:418h extended shift status
        if h(VK + qbvk::LCTRL) {
            b |= 1;
        }
        if h(VK + qbvk::LALT) {
            b |= 2;
        }
        if h(VK + qbvk::SYSREQ) {
            b |= 4;
        }
        if h(VK + qbvk::PAUSE) {
            b |= 8;
        }
        if h(VK + qbvk::SCROLLOCK) {
            b |= 16;
        }
        if h(0x5200) {
            b |= 128;
        }
        let mut c = 16u8; // 0:496h keyboard status (101-key keyboard)
        if h(VK + qbvk::RCTRL) {
            c |= 1;
        }
        if h(VK + qbvk::RALT) {
            c |= 2;
        }
        [a, b, c]
    }
}

static STATE: Mutex<Option<KeyState>> = Mutex::new(None);

fn state() -> MutexGuard<'static, Option<KeyState>> {
    let mut g = STATE.lock().unwrap_or_else(|e| e.into_inner());
    if g.is_none() {
        *g = Some(KeyState::new());
    }
    g
}

fn with<R>(f: impl FnOnce(&mut KeyState) -> R) -> R {
    let mut g = state();
    f(g.as_mut().expect("initialised"))
}

/// A re-entrant lock usable from C as lock/unlock pairs.
pub struct InputLock {
    owner: Mutex<(Option<ThreadId>, u32)>,
    cv: Condvar,
}

impl InputLock {
    pub const fn new() -> Self {
        InputLock {
            owner: Mutex::new((None, 0)),
            cv: Condvar::new(),
        }
    }

    pub fn lock(&self) {
        let me = thread::current().id();
        let mut g = self.owner.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            match g.0 {
                None => {
                    *g = (Some(me), 1);
                    return;
                }
                Some(t) if t == me => {
                    g.1 += 1;
                    return;
                }
                Some(_) => g = self.cv.wait(g).unwrap_or_else(|e| e.into_inner()),
            }
        }
    }

    /// Releases one level; a call from a thread that does not own the lock
    /// is ignored.
    pub fn unlock(&self) {
        let me = thread::current().id();
        let mut g = self.owner.lock().unwrap_or_else(|e| e.into_inner());
        if g.0 == Some(me) {
            g.1 -= 1;
            if g.1 == 0 {
                g.0 = None;
                self.cv.notify_one();
            }
        }
    }
}

impl Default for InputLock {
    fn default() -> Self {
        Self::new()
    }
}

static INPUT_LOCK: InputLock = InputLock::new();

// ---- C ABI -----------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn qb_input_lock() {
    INPUT_LOCK.lock();
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_input_unlock() {
    INPUT_LOCK.unlock();
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_keyhit_push(v: i64) {
    with(|s| s.keyhit_push(v));
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_keyhit_pop() -> i32 {
    with(|s| s.keyhit_pop())
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_keyhit_clear() {
    with(|s| s.keyhit_clear());
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_keyheld(x: u32) -> i32 {
    with(|s| s.keyheld(x)) as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_keyheld_add(x: u32, bind: u32) {
    with(|s| s.keyheld_add(x, bind));
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_keyheld_remove(x: u32) {
    with(|s| s.keyheld_remove(x));
}

/// Writes the bound held key to `*out` and returns 1, or returns 0.
///
/// # Safety
/// `out` must be valid for a `u32` write.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_keyheld_bound_to(x: u32, out: *mut u32) -> i32 {
    match with(|s| s.keyheld_bound_to(x)) {
        Some(k) => {
            // SAFETY: caller guarantees `out` is writable.
            unsafe { out.write_unaligned(k) };
            1
        }
        None => 0,
    }
}

/// Writes cmem[0x417], cmem[0x418], cmem[0x496] into `out[0..3]`.
///
/// # Safety
/// `out` must be valid for 3 byte writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_shift_state(out: *mut u8) {
    let b = with(|s| s.shift_state());
    // SAFETY: caller guarantees 3 writable bytes.
    unsafe { std::ptr::copy_nonoverlapping(b.as_ptr(), out, 3) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn keyhit_fifo_and_low_32_bits() {
        let mut s = KeyState::new();
        assert_eq!(s.keyhit_pop(), 0);
        s.keyhit_push(65);
        s.keyhit_push(-65 | (1i64 << 32)); // numpad flag in the high half
        assert_eq!(s.keyhit_pop(), 65);
        assert_eq!(s.keyhit_pop(), -65);
        assert_eq!(s.keyhit_pop(), 0);
    }

    #[test]
    fn keyhit_overflow_drops_oldest() {
        let mut s = KeyState::new();
        for i in 1..=8195 {
            s.keyhit_push(i);
        }
        // capacity is 8191 messages; the first four were dropped
        assert_eq!(s.keyhit_pop(), 8195 - 8191 + 1);
        s.keyhit_clear();
        assert_eq!(s.keyhit_pop(), 0);
    }

    #[test]
    fn keyheld_add_remove_and_dedup() {
        let mut s = KeyState::new();
        s.keyheld_add(65, 0);
        s.keyheld_add(65, 7); // already held: ignored, binding kept
        assert!(s.keyheld(65));
        assert_eq!(s.keyheld_bound_to(7), None);
        s.keyheld_add(VK + qbvk::CAPSLOCK, qbvk::CAPSLOCK);
        assert_eq!(
            s.keyheld_bound_to(qbvk::CAPSLOCK),
            Some(VK + qbvk::CAPSLOCK)
        );
        s.keyheld_remove(65);
        assert!(!s.keyheld(65));
        assert!(s.keyheld(VK + qbvk::CAPSLOCK));
    }

    #[test]
    fn keyheld_numpad_multimap() {
        let mut s = KeyState::new();
        s.keyheld_add(VK + qbvk::KP0 + 5, 0);
        assert!(s.keyheld(53)); // "5"
        s.keyheld_add(VK + qbvk::KP_ENTER, 0);
        assert!(s.keyheld(13));
        s.keyheld_add(QBK + 8, 0); // numpad 8 without numlock
        assert!(s.keyheld(72 << 8)); // up arrow
        assert!(!s.keyheld(80 << 8));
        s.keyheld_add(VK + qbvk::KP_DIVIDE, 0);
        assert!(s.keyheld(47));
    }

    #[test]
    fn shift_state_bytes() {
        let mut s = KeyState::new();
        assert_eq!(s.shift_state(), [0, 0, 16]);
        s.keyheld_add(VK + qbvk::LSHIFT, 0);
        s.keyheld_add(VK + qbvk::RCTRL, 0);
        s.keyheld_add(QBK + QBK_INSERT_MODE, 0);
        s.keyheld_add(0x5200, 0);
        assert_eq!(s.shift_state(), [2 | 4 | 128, 128, 16 | 1]);
    }

    #[test]
    fn input_lock_is_reentrant_and_exclusive() {
        let lock = Arc::new(InputLock::new());
        let counter = Arc::new(AtomicU32::new(0));
        lock.lock();
        lock.lock(); // re-entrant
        let (l2, c2) = (lock.clone(), counter.clone());
        let t = thread::spawn(move || {
            l2.lock();
            c2.store(1, Ordering::SeqCst);
            l2.unlock();
        });
        thread::sleep(std::time::Duration::from_millis(50));
        assert_eq!(counter.load(Ordering::SeqCst), 0, "other thread must wait");
        lock.unlock();
        thread::sleep(std::time::Duration::from_millis(20));
        assert_eq!(counter.load(Ordering::SeqCst), 0, "still held once");
        lock.unlock();
        t.join().unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
