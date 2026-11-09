//! Event trapping state: ON TIMER, ON KEY, ON STRIG (R12b).
//!
//! Replaces the timer table, `func__freetimer`, `freetimer`,
//! `ontimer_setup`, `sub_timer`, `stop_timers`/`start_timers`, the body of
//! `TIMERTHREAD`, `onkey_setup`, `sub_key`, `onstrig_setup` and `sub_strig`
//! from `qbx.cpp`. The dispatch itself (`events()` in qbx.cpp) stays in C++
//! because it is a `switch` over generated code; it asks Rust which timer
//! fired ([`qb_timer_take`]) and reads the `onkey`/`onstrig` arrays, which
//! keep their C layout (`common.h`) because the keyboard and gamepad code in
//! libqb.cpp updates them.
//!
//! Event states: `active` 0 = OFF, 1 = ON, 2 = STOP (events are queued but
//! not dispatched). `state` 0 = idle, 1 = triggered, 2 = in progress (timers);
//! keys and buttons count queued presses in `state`.

#![allow(non_upper_case_globals)]

use crate::error::new_error;
use crate::program;
use core::ptr::{addr_of, addr_of_mut};
use std::sync::{Mutex, MutexGuard};

/// One ON TIMER slot (index 0 is `ON TIMER(n)`, others come from `_FREETIMER`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Timer {
    pub allocated: bool,
    /// Event id to trigger, 0 = no handler.
    pub id: u32,
    pub pass: i64,
    pub active: u8,
    pub state: u8,
    pub seconds: f64,
    pub last_time: f64,
}

/// The ON TIMER table.
#[derive(Debug, PartialEq)]
pub struct TimerTable {
    pub slots: Vec<Timer>,
    nextfree: i32,
    freelist: Vec<i32>,
    paused: bool,
}

impl Default for TimerTable {
    fn default() -> Self {
        Self::new()
    }
}

impl TimerTable {
    /// Slot 0 exists and is allocated from the start.
    pub const fn new() -> Self {
        TimerTable { slots: Vec::new(), nextfree: 1, freelist: Vec::new(), paused: false }
    }

    fn slot(&mut self, i: i32) -> &mut Timer {
        let i = i as usize;
        if self.slots.is_empty() {
            self.slots.push(Timer { allocated: true, ..Timer::default() });
        }
        if self.slots.len() <= i {
            self.slots.resize(i + 1, Timer::default());
        }
        &mut self.slots[i]
    }

    fn check(&mut self, i: i32) -> Result<&mut Timer, i32> {
        if i < 0 || i >= self.nextfree {
            return Err(5);
        }
        let t = self.slot(i);
        if !t.allocated {
            return Err(5);
        }
        Ok(t)
    }

    /// `_FREETIMER`: a free timer index (reuses freed ones, newest first).
    pub fn alloc(&mut self) -> i32 {
        let i = if let Some(i) = self.freelist.pop() {
            i
        } else {
            let i = self.nextfree;
            self.slot(i).state = 0; // a reused slot keeps its state: its event may still run
            self.nextfree += 1;
            i
        };
        let t = self.slot(i);
        t.active = 0;
        t.id = 0;
        t.allocated = true;
        i
    }

    /// `freetimer`.
    pub fn free(&mut self, i: i32) {
        let t = self.slot(i);
        t.allocated = false;
        t.id = 0;
        self.freelist.push(i);
    }

    /// `ON TIMER(i, sec) GOSUB/CALL ...`.
    pub fn setup(&mut self, i: i32, seconds: f64, id: u32, pass: i64) -> Result<(), i32> {
        let t = self.check(i)?;
        if t.state == 1 {
            t.state = 0;
        }
        t.seconds = seconds;
        t.pass = pass;
        t.last_time = 0.0;
        t.id = id;
        Ok(())
    }

    /// `TIMER[(i)] ON|OFF|STOP|FREE` (`option` 1..4).
    pub fn statement(&mut self, i: i32, option: i32) -> Result<(), i32> {
        let t = self.check(i)?;
        match option {
            1 => t.active = 1,
            2 => {
                t.active = 0;
                if t.state == 1 {
                    t.state = 0;
                }
                t.last_time = 0.0;
            }
            3 => t.active = 2,
            4 => {
                if i == 0 {
                    return Err(5);
                }
                t.active = 0;
                if t.state == 1 {
                    t.state = 0;
                }
                self.free(i);
            }
            _ => {}
        }
        Ok(())
    }

    /// One pass of the timer thread at time `now` (seconds). Returns true
    /// when an event was triggered (the caller sets `qbevent`).
    pub fn tick(&mut self, now: f64) -> bool {
        let mut fired = false;
        if self.paused {
            return false;
        }
        for i in 0..self.nextfree {
            let t = self.slot(i);
            if t.allocated && t.id != 0 && t.active != 0 && t.state == 0 && now - t.last_time > t.seconds {
                if t.last_time == 0.0 {
                    t.last_time = now;
                } else {
                    t.last_time += t.seconds;
                    if (now - t.last_time).abs() >= t.seconds {
                        t.last_time = now;
                    }
                    t.state = 1;
                    fired = true;
                }
            }
        }
        fired
    }

    /// Number of timer indexes ever handed out (loop limit of `events()`).
    pub fn count(&self) -> i32 {
        self.nextfree
    }

    /// Event to dispatch for slot `i`: `(id, pass)` and the slot moves to
    /// "in progress", or `None`. STOPped timers keep their event queued.
    pub fn take(&mut self, i: i32) -> Option<(u32, i64)> {
        if i < 0 || i >= self.nextfree {
            return None;
        }
        let t = self.slot(i);
        if t.allocated && t.id != 0 && t.active == 1 && t.state == 1 {
            t.state = 2;
            return Some((t.id, t.pass));
        }
        None
    }

    /// The handler of slot `i` returned.
    pub fn done(&mut self, i: i32) {
        if i < self.nextfree && self.slot(i).allocated {
            self.slot(i).state = 0;
        }
    }
}

static TIMERS: Mutex<TimerTable> = Mutex::new(TimerTable::new());

/// Locks the global ON TIMER table (shared with the timer thread).
pub fn timers() -> MutexGuard<'static, TimerTable> {
    TIMERS.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------- ON KEY / STRIG

/// `onkey_struct` (common.h).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OnKey {
    pub id: u32,
    pub pass: i64,
    pub active: u8,
    pub state: u8,
    pub keycode: u32,
    pub keycode_alternate: u32,
    pub key_scancode: u8,
    pub key_flags: u8,
    pub text: *mut core::ffi::c_void,
}

/// `onstrig_struct` (common.h).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OnStrig {
    pub id: u32,
    pub pass: i64,
    pub active: u8,
    pub state: u8,
}

const ONKEY0: OnKey = OnKey {
    id: 0,
    pass: 0,
    active: 0,
    state: 0,
    keycode: 0,
    keycode_alternate: 0,
    key_scancode: 0,
    key_flags: 0,
    text: core::ptr::null_mut(),
};
const ONSTRIG0: OnStrig = OnStrig { id: 0, pass: 0, active: 0, state: 0 };
/// 256 controllers x 256 buttons.
pub const ONSTRIG_SLOTS: usize = 65_536;

static mut ONKEY_STORAGE: [OnKey; 32] = [ONKEY0; 32];
static mut ONSTRIG_STORAGE: [OnStrig; ONSTRIG_SLOTS] = [ONSTRIG0; ONSTRIG_SLOTS];

/// ON KEY slots 0..31 (1-31 used), read by libqb.cpp's keyboard code.
#[unsafe(no_mangle)]
pub static mut onkey: *mut OnKey = addr_of_mut!(ONKEY_STORAGE) as *mut OnKey;
/// 1 while an ON KEY handler runs.
#[unsafe(no_mangle)]
pub static mut onkey_inprogress: i32 = 0;
/// ON STRIG slots (`(controller-1)*256 + button-1`), read by the gamepad code.
#[unsafe(no_mangle)]
pub static mut onstrig: *mut OnStrig = addr_of_mut!(ONSTRIG_STORAGE) as *mut OnStrig;
/// 1 while an ON STRIG handler runs.
#[unsafe(no_mangle)]
pub static mut onstrig_inprogress: i32 = 0;

/// Shared ON/OFF/STOP transition of ON KEY and ON STRIG slots. Returns true
/// when a queued event became deliverable (the caller sets `qbevent`).
fn switch(active: &mut u8, state: &mut u8, option: i32) -> bool {
    match option {
        1 => {
            *active = 1;
            *state != 0
        }
        2 => {
            *active = 0;
            *state = 0;
            false
        }
        3 => {
            *active = 2;
            if *state != 0 {
                *state = 1;
            }
            false
        }
        _ => false,
    }
}

/// `onkey_setup`: `ON KEY(i) GOSUB/CALL`.
pub fn onkey_setup(keys: &mut [OnKey], i: i32, id: u32, pass: i64) -> Result<(), i32> {
    if !(1..=31).contains(&i) {
        return Err(5);
    }
    let k = &mut keys[i as usize];
    k.state = 0;
    k.pass = pass;
    k.id = id;
    Ok(())
}

/// `sub_key`: `KEY(i) ON|OFF|STOP`, `i` = 0 for all keys. Returns whether
/// to set `qbevent`.
pub fn key_statement(keys: &mut [OnKey], i: i32, option: i32) -> Result<bool, i32> {
    if !(0..=31).contains(&i) {
        return Err(5);
    }
    let (i1, i2) = if i == 0 { (0, 31) } else { (i, i) };
    let mut ev = false;
    for k in &mut keys[i1 as usize..=i2 as usize] {
        ev |= switch(&mut k.active, &mut k.state, option);
    }
    Ok(ev)
}

/// Maps a STRIG number (and optional controller) to a slot index.
/// `Ok(None)` for ranges that are accepted but not supported (no error).
fn strig_slot(i: i32, controller: i32, controller_passed: bool) -> Result<Option<usize>, i32> {
    if !(0..=65_535).contains(&i) {
        return Err(5);
    }
    let (mut i, mut controller) = (i, controller);
    if controller_passed {
        if !(1..=65_535).contains(&controller) {
            return Err(5);
        }
    } else {
        controller = 1;
        if i & 2 != 0 {
            controller = 2;
            i -= 2;
        }
    }
    let button = (i >> 2) + 1;
    if i & 1 != 0 {
        return Err(5);
    }
    if controller > 256 || button > 256 {
        return Ok(None);
    }
    Ok(Some(((controller - 1) * 256 + (button - 1)) as usize))
}

/// `onstrig_setup`: `ON STRIG(i[, controller]) GOSUB/CALL`. Returns true
/// when the slot was set (the caller then makes sure devices are scanned).
pub fn onstrig_setup(
    slots: &mut [OnStrig],
    i: i32,
    controller: i32,
    controller_passed: bool,
    id: u32,
    pass: i64,
) -> Result<bool, i32> {
    let Some(s) = strig_slot(i, controller, controller_passed)? else { return Ok(false) };
    let t = &mut slots[s];
    t.state = 0;
    t.pass = pass;
    t.id = id;
    Ok(true)
}

/// `sub_strig`: `STRIG[(i[, controller])] ON|OFF|STOP`; `passed` bit 0 = `i`
/// given, bit 1 = controller given. Returns whether to set `qbevent`.
pub fn strig_statement(slots: &mut [OnStrig], i: i32, controller: i32, option: i32, passed: i32) -> Result<bool, i32> {
    let (i1, i2) = if passed > 0 {
        match strig_slot(i, controller, passed & 2 != 0)? {
            Some(s) => (s, s),
            None => return Ok(false),
        }
    } else {
        (0, ONSTRIG_SLOTS - 1)
    };
    let mut ev = false;
    for s in &mut slots[i1..=i2] {
        ev |= switch(&mut s.active, &mut s.state, option);
    }
    Ok(ev)
}

// ---------------------------------------------------------------- C ABI

#[cfg(not(test))]
mod host {
    unsafe extern "C" {
        /// `if (device_last==0) func__devices();` in libqb.cpp.
        fn qb_host_devices_init();
    }
    pub fn devices_init() {
        // SAFETY: plain call into libqb.cpp.
        unsafe { qb_host_devices_init() }
    }
}

#[cfg(test)]
mod host {
    pub fn devices_init() {}
}

fn pending_error() -> bool {
    program::load(addr_of!(new_error)) != 0
}

fn raise(r: Result<bool, i32>) {
    match r {
        Ok(true) => program::signal_event(),
        Ok(false) => {}
        Err(e) => crate::error::raise(e),
    }
}

/// `func__freetimer`.
#[unsafe(no_mangle)]
pub extern "C" fn qb_freetimer_new() -> i32 {
    if pending_error() {
        return 0;
    }
    timers().alloc()
}

/// `freetimer(i)`.
#[unsafe(no_mangle)]
pub extern "C" fn qb_freetimer(i: i32) {
    timers().free(i);
}

/// `ontimer_setup`.
#[unsafe(no_mangle)]
pub extern "C" fn qb_ontimer_setup(i: i32, seconds: f64, id: u32, pass: i64) {
    if pending_error() {
        return;
    }
    raise(timers().setup(i, seconds, id, pass).map(|_| false));
}

/// `sub_timer` (`passed` = 0 for plain `TIMER ON`).
#[unsafe(no_mangle)]
pub extern "C" fn qb_timer_statement(i: i32, option: i32, passed: i32) {
    if pending_error() {
        return;
    }
    let i = if passed == 0 { 0 } else { i };
    raise(timers().statement(i, option).map(|_| false));
}

/// `stop_timers`: the timer thread stops triggering until `start_timers`.
#[unsafe(no_mangle)]
pub extern "C" fn qb_timers_pause(pause: i32) {
    timers().paused = pause != 0;
}

/// `TIMERTHREAD`: runs until `stop_program`, then sets bit 1 of `exit_ok`.
#[unsafe(no_mangle)]
pub extern "C" fn qb_timer_thread() {
    loop {
        let now = crate::systime::ticks() as f64 * 0.001;
        if timers().tick(now) {
            program::signal_event();
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
        if program::stop_requested() {
            program::exit_ok_or(2);
            return;
        }
    }
}

/// Loop limit for the ON TIMER part of `events()`.
#[unsafe(no_mangle)]
pub extern "C" fn qb_timer_count() -> i32 {
    timers().count()
}

/// Takes the pending event of timer `i`; returns its id (0 = none).
///
/// # Safety
/// `pass` valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_timer_take(i: i32, pass: *mut i64) -> u32 {
    match timers().take(i) {
        Some((id, p)) => {
            unsafe { *pass = p };
            id
        }
        None => 0,
    }
}

/// The ON TIMER handler of slot `i` finished.
#[unsafe(no_mangle)]
pub extern "C" fn qb_timer_done(i: i32) {
    timers().done(i);
}

fn onkeys() -> &'static mut [OnKey] {
    // SAFETY: `onkey` always points at the 32-entry storage.
    unsafe { core::slice::from_raw_parts_mut(onkey, 32) }
}

fn onstrigs() -> &'static mut [OnStrig] {
    // SAFETY: `onstrig` always points at the 65536-entry storage.
    unsafe { core::slice::from_raw_parts_mut(onstrig, ONSTRIG_SLOTS) }
}

/// `onkey_setup`.
#[unsafe(no_mangle)]
pub extern "C" fn qb_onkey_setup(i: i32, id: u32, pass: i64) {
    if pending_error() {
        return;
    }
    raise(onkey_setup(onkeys(), i, id, pass).map(|_| false));
}

/// `sub_key`.
#[unsafe(no_mangle)]
pub extern "C" fn qb_key_statement(i: i32, option: i32) {
    if pending_error() {
        return;
    }
    raise(key_statement(onkeys(), i, option));
}

/// `onstrig_setup`.
#[unsafe(no_mangle)]
pub extern "C" fn qb_onstrig_setup(i: i32, controller: i32, controller_passed: i32, id: u32, pass: i64) {
    if pending_error() {
        return;
    }
    match onstrig_setup(onstrigs(), i, controller, controller_passed != 0, id, pass) {
        Ok(true) => host::devices_init(),
        Ok(false) => {}
        Err(e) => crate::error::raise(e),
    }
}

/// `sub_strig`.
#[unsafe(no_mangle)]
pub extern "C" fn qb_strig_statement(i: i32, controller: i32, option: i32, passed: i32) {
    if pending_error() {
        return;
    }
    raise(strig_statement(onstrigs(), i, controller, option, passed));
}

// ------------------------------------------- test hooks (tests/diff only)

/// Clears the global timer table (slot 0 allocated, nothing else).
#[doc(hidden)]
#[unsafe(no_mangle)]
pub extern "C" fn qb_test_timer_reset() {
    *timers() = TimerTable::new();
}

/// One timer-thread pass at `now` seconds; 1 when an event fired.
#[doc(hidden)]
#[unsafe(no_mangle)]
pub extern "C" fn qb_test_timer_tick(now: f64) -> i32 {
    i32::from(timers().tick(now))
}

/// Copies timer slot `i` (`out`: allocated, id, active, state; `pass`;
/// `secs`: seconds, last_time). Returns 0 when the slot does not exist.
///
/// # Safety
/// `out` writable for 4 values, `secs` for 2, `pass` for 1.
#[doc(hidden)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_test_timer_get(i: i32, out: *mut i64, pass: *mut i64, secs: *mut f64) -> i32 {
    let mut t = timers();
    if i < 0 || i >= t.count() {
        return 0;
    }
    let s = *t.slot(i);
    unsafe {
        *out = i64::from(s.allocated);
        *out.add(1) = i64::from(s.id);
        *out.add(2) = i64::from(s.active);
        *out.add(3) = i64::from(s.state);
        *pass = s.pass;
        *secs = s.seconds;
        *secs.add(1) = s.last_time;
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freetimer_reuses_newest_freed_index() {
        let mut t = TimerTable::new();
        assert_eq!(t.alloc(), 1);
        assert_eq!(t.alloc(), 2);
        assert_eq!(t.alloc(), 3);
        t.statement(2, 4).unwrap();
        t.free(1);
        assert_eq!(t.alloc(), 1);
        assert_eq!(t.alloc(), 2);
        assert_eq!(t.alloc(), 4);
        assert_eq!(t.statement(0, 4), Err(5));
        assert_eq!(t.statement(9, 1), Err(5));
        assert_eq!(t.setup(-1, 1.0, 1, 0), Err(5));
        t.free(3);
        assert_eq!(t.setup(3, 1.0, 1, 0), Err(5));
    }

    #[test]
    fn timer_fires_after_period_and_catches_up() {
        let mut t = TimerTable::new();
        t.setup(0, 1.0, 7, 42).unwrap();
        assert!(!t.tick(10.0)); // not active yet
        t.statement(0, 1).unwrap();
        assert!(!t.tick(10.0)); // first pass only records the start time
        assert!(!t.tick(10.9));
        assert!(t.tick(11.01));
        assert_eq!(t.take(0), Some((7, 42)));
        assert_eq!(t.take(0), None);
        assert!(!t.tick(15.0)); // in progress
        t.done(0);
        assert!(t.tick(15.0)); // far behind: re-seeded to now
        assert_eq!(t.slots[0].last_time, 15.0);
        // STOP keeps the event queued, ON delivers it.
        t.statement(0, 3).unwrap();
        assert_eq!(t.take(0), None);
        t.statement(0, 1).unwrap();
        assert_eq!(t.take(0), Some((7, 42)));
        t.done(0);
        // OFF retracts and restarts the period.
        assert!(t.tick(16.5));
        t.statement(0, 2).unwrap();
        assert_eq!(t.slots[0].state, 0);
        assert_eq!(t.slots[0].last_time, 0.0);
        t.paused = true;
        t.statement(0, 1).unwrap();
        assert!(!t.tick(100.0));
    }

    #[test]
    fn key_statements() {
        let mut k = [ONKEY0; 32];
        assert_eq!(onkey_setup(&mut k, 0, 1, 0), Err(5));
        assert_eq!(onkey_setup(&mut k, 32, 1, 0), Err(5));
        onkey_setup(&mut k, 5, 9, 3).unwrap();
        k[5].state = 3;
        assert_eq!(key_statement(&mut k, 5, 3), Ok(false));
        assert_eq!((k[5].active, k[5].state), (2, 1));
        assert_eq!(key_statement(&mut k, 0, 1), Ok(true));
        assert!(k.iter().all(|k| k.active == 1));
        assert_eq!(key_statement(&mut k, 0, 2), Ok(false));
        assert_eq!(k[5].state, 0);
        assert_eq!(key_statement(&mut k, 32, 1), Err(5));
    }

    #[test]
    fn strig_slots() {
        let mut s = vec![ONSTRIG0; ONSTRIG_SLOTS];
        // STRIG(0)/(2): controller 1/2 button 1; STRIG(4)/(6): button 2.
        assert_eq!(onstrig_setup(&mut s, 2, 0, false, 5, 1), Ok(true));
        assert_eq!(s[256].id, 5);
        assert_eq!(onstrig_setup(&mut s, 4, 0, false, 6, 1), Ok(true));
        assert_eq!(s[1].id, 6);
        assert_eq!(onstrig_setup(&mut s, 0, 3, true, 8, 1), Ok(true));
        assert_eq!(s[512].id, 8);
        assert_eq!(onstrig_setup(&mut s, 1, 0, false, 1, 0), Err(5));
        assert_eq!(onstrig_setup(&mut s, 0, 0, true, 1, 0), Err(5));
        assert_eq!(onstrig_setup(&mut s, 0, 300, true, 1, 0), Ok(false));
        s[1].state = 2;
        assert_eq!(strig_statement(&mut s, 4, 0, 1, 1), Ok(true));
        assert_eq!(strig_statement(&mut s, 0, 0, 3, 0), Ok(false));
        assert!(s.iter().all(|x| x.active == 2));
        assert_eq!(s[1].state, 1);
        assert_eq!(strig_statement(&mut s, 0, 0, 1, 2), Err(5)); // controller given but 0
    }
}
