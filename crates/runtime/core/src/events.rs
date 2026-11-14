//! Event trapping state: ON TIMER, ON KEY, ON STRIG.
//!
//! The timer table (`_FREETIMER`, `ON TIMER`, `TIMER ON|OFF|STOP`), the ON KEY
//! and ON STRIG slots and their `KEY(n)`/`STRIG(n) ON|OFF|STOP` transitions,
//! with QB64's semantics. The runner dispatches the handlers at its safe points.
//!
//! Event states: `active` 0 = OFF, 1 = ON, 2 = STOP (events are queued but
//! not dispatched). `state` 0 = idle, 1 = triggered, 2 = in progress (timers);
//! keys and buttons count queued presses in `state`.

#![allow(non_upper_case_globals)]

use core::ptr::addr_of_mut;
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
        // an interval of 0, below 0 or NaN: the handler ran on every tick (or never)
        if seconds.is_nan() || seconds <= 0.0 {
            return Err(5);
        }
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
    /// when an event was triggered, or one queued during `TIMER STOP` can now be
    /// delivered (the caller sets `qbevent`).
    pub fn tick(&mut self, now: f64) -> bool {
        let mut fired = false;
        if self.paused {
            return false;
        }
        for i in 0..self.nextfree {
            let t = self.slot(i);
            if t.allocated && t.id != 0 && t.active == 1 && t.state == 1 {
                fired = true;
            }
            if !(t.allocated && t.id != 0 && t.active != 0 && t.state == 0) {
                continue;
            }
            if t.last_time == 0.0 {
                // First tick: the interval starts now. The clock counts from the start
                // of the program, so this must not wait for `now > seconds` first
                // (ON TIMER(1) at the start fired after 2 s). 0.0 is the "not
                // started" mark, so a start at exactly 0 is kept as the smallest time.
                t.last_time = if now == 0.0 { f64::MIN_POSITIVE } else { now };
            } else if now - t.last_time > t.seconds {
                t.last_time += t.seconds;
                if (now - t.last_time).abs() >= t.seconds {
                    t.last_time = now;
                }
                t.state = 1;
                fired = true;
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
        // also a slot freed (TIMER(n) FREE in its own handler) and perhaps handed out
        // again while the handler ran: left "in progress" it never fired again
        if (0..self.nextfree).contains(&i) && self.slot(i).state == 2 {
            self.slot(i).state = 0;
        }
    }

    /// RUN / CLEAR dropped the GOSUB stack: no handler is running any more.
    pub fn reset_in_progress(&mut self) {
        for t in &mut self.slots {
            if t.state == 2 {
                t.state = 0;
            }
        }
    }
}

static TIMERS: Mutex<TimerTable> = Mutex::new(TimerTable::new());

/// Locks the global ON TIMER table (shared with the timer thread).
pub fn timers() -> MutexGuard<'static, TimerTable> {
    TIMERS.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------- ON PLAY

/// `ON PLAY(n) GOSUB` and `PLAY ON|OFF|STOP`: one trap on the background music queue.
/// The handler is due when the queue drops from `n` or more notes to fewer than `n`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OnPlay {
    /// Handler id (0 = no `ON PLAY`).
    pub id: u32,
    /// The `n` of `ON PLAY(n)`.
    pub n: i32,
    /// 0 OFF, 1 ON, 2 STOP (a drop is remembered and delivered at the next `PLAY ON`).
    pub active: i32,
    /// 0 idle, 1 due, 2 handler running.
    pub state: i32,
    /// Queue length at the previous poll.
    last: i32,
}

impl OnPlay {
    pub const fn new() -> Self {
        OnPlay { id: 0, n: 0, active: 0, state: 0, last: 0 }
    }

    /// `ON PLAY(n) GOSUB`: `n` is 1..32 as in QBasic (error 5 otherwise).
    pub fn setup(&mut self, n: i32, id: u32) -> Result<(), i32> {
        if !(1..=32).contains(&n) {
            return Err(5);
        }
        self.id = id;
        self.n = n;
        if self.state == 1 {
            self.state = 0;
        }
        Ok(())
    }

    /// `PLAY ON|OFF|STOP` (`option` 1..3).
    pub fn statement(&mut self, option: i32) -> Result<(), i32> {
        match option {
            1 => self.active = 1,
            2 => {
                self.active = 0;
                if self.state == 1 {
                    self.state = 0;
                }
            }
            3 => self.active = 2,
            _ => return Err(5),
        }
        Ok(())
    }

    /// Feeds the current queue length; returns the handler id when it should run now
    /// (the trap is then "running" until [`OnPlay::done`]).
    pub fn poll(&mut self, notes: i32) -> Option<u32> {
        if self.id != 0 && self.active != 0 && self.state == 0 && self.last >= self.n && notes < self.n {
            self.state = 1;
        }
        self.last = notes;
        if self.id != 0 && self.active == 1 && self.state == 1 {
            self.state = 2;
            return Some(self.id);
        }
        None
    }

    /// The handler returned.
    pub fn done(&mut self) {
        if self.state == 2 {
            self.state = 0;
        }
    }
}

impl Default for OnPlay {
    fn default() -> Self {
        Self::new()
    }
}

static PLAY_TRAP: Mutex<OnPlay> = Mutex::new(OnPlay::new());

/// Locks the global ON PLAY trap.
pub fn play_trap() -> MutexGuard<'static, OnPlay> {
    PLAY_TRAP.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------- ON KEY / STRIG

/// One ON KEY slot.
#[derive(Clone, Copy)]
pub struct OnKey {
    pub id: u32,
    pub pass: i64,
    pub active: u8,
    pub state: u8,
}

/// One ON STRIG slot.
#[derive(Clone, Copy)]
pub struct OnStrig {
    pub id: u32,
    pub pass: i64,
    pub active: u8,
    pub state: u8,
}

const ONKEY0: OnKey = OnKey { id: 0, pass: 0, active: 0, state: 0 };
const ONSTRIG0: OnStrig = OnStrig { id: 0, pass: 0, active: 0, state: 0 };
/// 256 controllers x 256 buttons.
pub const ONSTRIG_SLOTS: usize = 65_536;

static mut ONKEY_STORAGE: [OnKey; 32] = [ONKEY0; 32];
static mut ONSTRIG_STORAGE: [OnStrig; ONSTRIG_SLOTS] = [ONSTRIG0; ONSTRIG_SLOTS];

/// 1 while an ON KEY handler runs.
pub static mut onkey_inprogress: i32 = 0;
/// 1 while an ON STRIG handler runs.
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

/// `onstrig_setup`: `ON STRIG(i[, controller]) GOSUB/CALL`. Returns the slot
/// that was set (the caller then makes sure devices are scanned), None for a
/// controller or button beyond the 256 that are kept.
pub fn onstrig_setup(
    slots: &mut [OnStrig],
    i: i32,
    controller: i32,
    controller_passed: bool,
    id: u32,
    pass: i64,
) -> Result<Option<usize>, i32> {
    let Some(s) = strig_slot(i, controller, controller_passed)? else { return Ok(None) };
    let t = &mut slots[s];
    t.state = 0;
    t.pass = pass;
    t.id = id;
    Ok(Some(s))
}

/// Button `button` of controller `controller` (both from 0) went down: its
/// ON STRIG event is queued while trapping is on, and one is kept while it is
/// stopped (`STRIG(n) STOP`).
pub fn strig_pressed(slots: &mut [OnStrig], controller: usize, button: usize) {
    if controller >= 256 || button >= 256 {
        return;
    }
    let s = &mut slots[controller * 256 + button];
    if s.id == 0 {
        return;
    }
    match s.active {
        1 => s.state = s.state.saturating_add(1),
        2 => s.state = 1,
        _ => {}
    }
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

/// The 32 ON KEY slots, 0..31 (1-31 used); program thread only.
pub fn onkeys() -> &'static mut [OnKey] {
    // SAFETY: only the program thread touches the storage.
    unsafe { &mut *addr_of_mut!(ONKEY_STORAGE) }
}

/// The ON STRIG slots (`(controller-1)*256 + button-1`); program thread only.
pub fn onstrigs() -> &'static mut [OnStrig] {
    // SAFETY: only the program thread touches the storage.
    unsafe { &mut *addr_of_mut!(ONSTRIG_STORAGE) }
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

    /// The clock counts from the start of the program: ON TIMER(1) set then fires
    /// after 1 s, not 2 (issue #7).
    #[test]
    fn timer_started_with_the_clock_near_zero_fires_after_one_period() {
        for start in [0.0, 0.002] {
            let mut t = TimerTable::new();
            t.setup(0, 1.0, 7, 42).unwrap();
            t.statement(0, 1).unwrap();
            assert!(!t.tick(start)); // records the start
            assert!(!t.tick(start + 0.99));
            assert!(t.tick(start + 1.01), "start {start}");
            assert_eq!(t.take(0), Some((7, 42)));
        }
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
        assert!(!t.tick(15.1)); // queued while stopped: no signal yet
        assert_eq!(t.take(0), None);
        t.statement(0, 1).unwrap();
        assert!(t.tick(15.2)); // ON signals the queued event
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
    fn play_trap_fires_when_the_queue_drops_below_n() {
        let mut p = OnPlay::new();
        assert_eq!(p.setup(0, 1), Err(5));
        p.setup(2, 7).unwrap();
        assert_eq!(p.poll(3), None); // OFF
        p.statement(1).unwrap();
        assert_eq!(p.poll(3), None);
        assert_eq!(p.poll(2), None);
        assert_eq!(p.poll(1), Some(7)); // 2 -> 1
        assert_eq!(p.poll(0), None); // running
        p.done();
        assert_eq!(p.poll(0), None);
        // STOP remembers the drop, ON delivers it
        p.statement(3).unwrap();
        assert_eq!(p.poll(4), None);
        assert_eq!(p.poll(1), None);
        p.statement(1).unwrap();
        assert_eq!(p.poll(1), Some(7));
        p.done();
        // OFF forgets it
        assert_eq!(p.poll(5), None);
        p.statement(2).unwrap();
        assert_eq!(p.poll(0), None);
        p.statement(1).unwrap();
        assert_eq!(p.poll(0), None);
        assert_eq!(p.statement(9), Err(5));
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
        assert_eq!(onstrig_setup(&mut s, 2, 0, false, 5, 1), Ok(Some(256)));
        assert_eq!(s[256].id, 5);
        assert_eq!(onstrig_setup(&mut s, 4, 0, false, 6, 1), Ok(Some(1)));
        assert_eq!(s[1].id, 6);
        assert_eq!(onstrig_setup(&mut s, 0, 3, true, 8, 1), Ok(Some(512)));
        assert_eq!(s[512].id, 8);
        assert_eq!(onstrig_setup(&mut s, 1, 0, false, 1, 0), Err(5));
        assert_eq!(onstrig_setup(&mut s, 0, 0, true, 1, 0), Err(5));
        assert_eq!(onstrig_setup(&mut s, 0, 300, true, 1, 0), Ok(None));
        // a press queues an event only while trapping is on; stopped keeps one
        let mut p = vec![ONSTRIG0; ONSTRIG_SLOTS];
        strig_pressed(&mut p, 0, 1);
        assert_eq!(p[1].state, 0, "no handler: nothing queued");
        onstrig_setup(&mut p, 4, 0, false, 6, 0).unwrap();
        strig_pressed(&mut p, 0, 1);
        assert_eq!(p[1].state, 0, "handler but STRIG OFF");
        strig_statement(&mut p, 4, 0, 1, 1).unwrap();
        strig_pressed(&mut p, 0, 1);
        strig_pressed(&mut p, 0, 1);
        strig_pressed(&mut p, 300, 1);
        assert_eq!((p[1].state, p[0].state), (2, 0));
        strig_statement(&mut p, 4, 0, 3, 1).unwrap();
        strig_pressed(&mut p, 0, 1);
        strig_pressed(&mut p, 0, 1);
        assert_eq!(p[1].state, 1, "stopped: one event waits");
        s[1].state = 2;
        assert_eq!(strig_statement(&mut s, 4, 0, 1, 1), Ok(true));
        assert_eq!(strig_statement(&mut s, 0, 0, 3, 0), Ok(false));
        assert!(s.iter().all(|x| x.active == 2));
        assert_eq!(s[1].state, 1);
        assert_eq!(strig_statement(&mut s, 0, 0, 1, 2), Err(5)); // controller given but 0
    }
}
