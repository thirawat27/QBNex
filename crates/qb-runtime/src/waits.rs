//! `_DELAY`, `_LIMIT` and the timed part of `SLEEP` (R12b).
//!
//! The waits poll the millisecond clock, sleep in slices of at most 9 ms and
//! call `evnt()` between slices so ON TIMER/KEY handlers and window events
//! keep running. The clock, the sleep and `evnt()` come from a [`WaitHost`]
//! so the loops can be replayed against a scripted clock (unit tests and the
//! differential test). The console branch of SLEEP (waiting on the Windows
//! console input handle) stays in libqb.cpp with the text console (R12a).

/// Clock, sleep and event pump used by the waits.
pub trait WaitHost {
    /// `GetTicks()`: milliseconds.
    fn ticks(&mut self) -> i64;
    /// `Sleep(ms)`.
    fn sleep(&mut self, ms: i64);
    /// `evnt(0)`.
    fn evnt(&mut self);
    /// `sleep_break` (a key press or event ended SLEEP).
    fn sleep_break(&mut self) -> bool {
        false
    }
    /// `stop_program`.
    fn stop_program(&mut self) -> bool {
        false
    }
}

/// `sub__delay`: waits `seconds`. `Err(5)` outside 0..2147483.647.
pub fn delay(h: &mut impl WaitHost, seconds: f64) -> Result<(), i32> {
    let mut base = h.ticks() as f64;
    if seconds < 0.0 || seconds > 2147483.647 {
        return Err(5);
    }
    let ms = seconds * 1000.0;
    let mut now = base;
    loop {
        let prev_now = now;
        now = h.ticks() as f64;
        let elapsed = now - base;
        if elapsed < 0.0 {
            base = now - (prev_now - base);
        }
        if elapsed < ms {
            let mut wait = (ms - elapsed) as i64;
            if wait == 0 {
                wait = 1;
            }
            if wait >= 10 {
                h.sleep(9);
                h.evnt();
                continue;
            }
            h.sleep(wait);
        }
        return Ok(());
    }
}

/// `_LIMIT` state: the time of the previous frame (0 = first call).
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Limit {
    pub prev: f64,
}

impl Limit {
    /// `sub__limit`: waits until 1/`fps` seconds after the previous call.
    /// `Err(5)` for fps <= 0 or below 1/60 frames per second.
    pub fn wait(&mut self, h: &mut impl WaitHost, fps: f64) -> Result<(), i32> {
        if fps <= 0.0 {
            return Err(5);
        }
        let ms = 1000.0 / fps;
        if ms > 60000.0 {
            return Err(5);
        }
        loop {
            let now = h.ticks() as f64;
            if self.prev == 0.0 || now < self.prev {
                self.prev = now;
                return Ok(());
            }
            let elapsed = now - self.prev;
            if elapsed == ms {
                self.prev += ms;
                return Ok(());
            }
            if elapsed < ms {
                let mut wait = (ms - elapsed) as i64;
                if wait == 0 {
                    wait = 1;
                }
                if wait >= 10 {
                    h.sleep(9);
                    h.evnt();
                } else {
                    h.sleep(wait);
                }
                continue;
            }
            // Overshoot up to 32 ms is recovered, otherwise the clock is re-seeded.
            if elapsed <= ms + 32.0 {
                self.prev += ms;
            } else {
                self.prev = now;
            }
            return Ok(());
        }
    }
}

/// Timed/untimed SLEEP after the console check: `prev` is the clock when
/// SLEEP started, `ms` the requested time (<= 0 = until a key or event).
pub fn sleep(h: &mut impl WaitHost, prev: f64, ms: f64) {
    loop {
        h.evnt();
        if h.sleep_break() || h.stop_program() {
            return;
        }
        if ms <= 0.0 {
            h.sleep(9);
            continue;
        }
        let now = h.ticks() as f64;
        if now < prev {
            return;
        }
        let elapsed = now - prev;
        if elapsed < ms {
            let mut wait = (ms - elapsed) as i64;
            if wait == 0 {
                wait = 1;
            }
            if wait >= 10 {
                h.sleep(9);
                continue;
            }
            h.sleep(wait);
        }
        return;
    }
}

// ---------------------------------------------------------------- C ABI

/// The real clock, `Sleep` and `evnt(0)` of a compiled program.
pub struct RuntimeHost;

#[cfg(not(test))]
unsafe extern "C" {
    /// `evnt(0)` in qbx.cpp (handles errors and dispatches events).
    fn qb_host_evnt();
}

impl WaitHost for RuntimeHost {
    fn ticks(&mut self) -> i64 {
        crate::systime::ticks()
    }
    fn sleep(&mut self, ms: i64) {
        std::thread::sleep(std::time::Duration::from_millis(ms.max(0) as u64));
    }
    fn evnt(&mut self) {
        // SAFETY: plain call into qbx.cpp.
        #[cfg(not(test))]
        unsafe {
            qb_host_evnt()
        }
    }
    fn sleep_break(&mut self) -> bool {
        crate::program::load(core::ptr::addr_of!(crate::program::sleep_break)) != 0
    }
    fn stop_program(&mut self) -> bool {
        crate::program::stop_requested()
    }
}

static mut LIMIT: Limit = Limit { prev: 0.0 };

/// `sub__delay`; returns the BASIC error.
#[unsafe(no_mangle)]
pub extern "C" fn qb_delay(seconds: f64) -> i32 {
    delay(&mut RuntimeHost, seconds).err().unwrap_or(0)
}

/// `sub__limit`; returns the BASIC error.
#[unsafe(no_mangle)]
pub extern "C" fn qb_limit(fps: f64) -> i32 {
    // SAFETY: `_LIMIT` runs on the program thread only.
    let l = unsafe { &mut *core::ptr::addr_of_mut!(LIMIT) };
    l.wait(&mut RuntimeHost, fps).err().unwrap_or(0)
}

/// Timed part of `sub_sleep` (see [`sleep`]).
#[unsafe(no_mangle)]
pub extern "C" fn qb_sleep_wait(prev: f64, ms: f64) {
    sleep(&mut RuntimeHost, prev, ms);
}

// ------------------------------------------- test hooks (tests/diff only)

/// Scripted host for the differential test: `ticks()` returns the next
/// value of `times` (the last one repeats); `sleep_break`/`stop_program`
/// turn on after the given number of `evnt()` calls (negative = never).
/// Trace: `ms` >= 0 for `Sleep(ms)`, -1 for `evnt()`.
struct TraceHost<'a> {
    times: &'a [i64],
    at: usize,
    trace: Vec<i64>,
    evnts: i64,
    break_after: i64,
    stop_after: i64,
}

impl WaitHost for TraceHost<'_> {
    fn ticks(&mut self) -> i64 {
        let t = self.times[self.at.min(self.times.len() - 1)];
        self.at += 1;
        t
    }
    fn sleep(&mut self, ms: i64) {
        self.trace.push(ms);
    }
    fn evnt(&mut self) {
        self.evnts += 1;
        self.trace.push(-1);
    }
    fn sleep_break(&mut self) -> bool {
        self.break_after >= 0 && self.evnts >= self.break_after
    }
    fn stop_program(&mut self) -> bool {
        self.stop_after >= 0 && self.evnts >= self.stop_after
    }
}

/// Runs `_DELAY a` (kind 0), `_LIMIT a` with `*prev` as the previous frame
/// time (kind 1) or the SLEEP loop with prev `a`, ms `b` (kind 2) against a
/// scripted clock. Returns the trace length (capped at `cap`); `*err` gets
/// the BASIC error, `*ticks_used` the number of clock reads.
///
/// # Safety
/// `times` readable for `n` (> 0) values, `trace` writable for `cap`,
/// `prev`/`err`/`ticks_used` valid.
#[doc(hidden)]
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn qb_test_wait(
    kind: i32,
    a: f64,
    b: f64,
    times: *const i64,
    n: i32,
    break_after: i64,
    stop_after: i64,
    prev: *mut f64,
    trace: *mut i64,
    cap: i32,
    err: *mut i32,
    ticks_used: *mut i32,
) -> i32 {
    let times = unsafe { core::slice::from_raw_parts(times, n.max(1) as usize) };
    let mut h = TraceHost { times, at: 0, trace: Vec::new(), evnts: 0, break_after, stop_after };
    let e = match kind {
        0 => delay(&mut h, a).err().unwrap_or(0),
        1 => {
            let mut l = Limit { prev: unsafe { *prev } };
            let e = l.wait(&mut h, a).err().unwrap_or(0);
            unsafe { *prev = l.prev };
            e
        }
        _ => {
            sleep(&mut h, a, b);
            0
        }
    };
    let len = h.trace.len().min(cap.max(0) as usize);
    unsafe {
        core::ptr::copy_nonoverlapping(h.trace.as_ptr(), trace, len);
        *err = e;
        *ticks_used = h.at as i32;
    }
    len as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scripted clock: every `ticks()` returns the next value (the last one
    /// repeats); `sleep` advances nothing on its own. Records calls.
    struct Script {
        times: Vec<i64>,
        at: usize,
        log: Vec<String>,
        break_after_evnt: Option<usize>,
        evnts: usize,
    }

    impl Script {
        fn new(times: &[i64]) -> Self {
            Script { times: times.to_vec(), at: 0, log: Vec::new(), break_after_evnt: None, evnts: 0 }
        }
    }

    impl WaitHost for Script {
        fn ticks(&mut self) -> i64 {
            let t = self.times[self.at.min(self.times.len() - 1)];
            self.at += 1;
            t
        }
        fn sleep(&mut self, ms: i64) {
            self.log.push(format!("S{ms}"));
        }
        fn evnt(&mut self) {
            self.evnts += 1;
            self.log.push("E".into());
        }
        fn sleep_break(&mut self) -> bool {
            self.break_after_evnt.is_some_and(|n| self.evnts >= n)
        }
    }

    #[test]
    fn delay_slices_and_errors() {
        let mut s = Script::new(&[1000, 1000, 1009, 1018, 1027]);
        delay(&mut s, 0.03).unwrap();
        assert_eq!(s.log, ["S9", "E", "S9", "E", "S9", "E", "S3"]);
        assert_eq!(delay(&mut Script::new(&[0]), -1.0), Err(5));
        assert_eq!(delay(&mut Script::new(&[0]), 2147484.0), Err(5));
        let mut s = Script::new(&[5, 5]);
        delay(&mut s, 0.0).unwrap();
        assert!(s.log.is_empty());
    }

    #[test]
    fn limit_paces_frames() {
        let mut l = Limit::default();
        let mut s = Script::new(&[100]);
        l.wait(&mut s, 50.0).unwrap(); // first call only records the time
        assert_eq!(l.prev, 100.0);
        let mut s = Script::new(&[105, 114, 117, 120]);
        l.wait(&mut s, 50.0).unwrap();
        assert_eq!(s.log, ["S9", "E", "S6", "S3"]);
        assert_eq!(l.prev, 120.0);
        let mut s = Script::new(&[150]); // 10 ms late: recovered
        l.wait(&mut s, 50.0).unwrap();
        assert_eq!(l.prev, 140.0);
        let mut s = Script::new(&[300]); // far behind: re-seeded
        l.wait(&mut s, 50.0).unwrap();
        assert_eq!(l.prev, 300.0);
        assert_eq!(l.wait(&mut s, 0.0), Err(5));
        assert_eq!(l.wait(&mut s, 0.01), Err(5));
    }

    #[test]
    fn sleep_timed_and_untimed() {
        let mut s = Script::new(&[0, 9, 18, 25]);
        sleep(&mut s, 0.0, 25.0);
        assert_eq!(s.log, ["E", "S9", "E", "S9", "E", "S7"]);
        let mut s = Script::new(&[0]);
        s.break_after_evnt = Some(3);
        sleep(&mut s, 0.0, 0.0);
        assert_eq!(s.log, ["E", "S9", "E", "S9", "E"]);
    }
}
