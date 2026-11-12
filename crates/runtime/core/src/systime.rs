//! Clock reads for TIMER, `_DELAY`, `_LIMIT` and the cursor blink.
//!
//! Replaces `GetTicks` (three per-OS versions: `clock()` on Windows,
//! `CLOCK_MONOTONIC` on Linux, `mach_absolute_time` on macOS) and the
//! start-of-day setup in libqb.cpp's `main` (wait for the wall clock's second
//! to tick over, then record the tick count and the local time of day).

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeDelta};
use std::sync::atomic::{AtomicI64, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::OnceLock;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Milliseconds since the first clock read of the process (monotonic).
///
/// Legacy Windows used `clock()`, which also counts from process start;
/// Linux/macOS used an arbitrary monotonic origin. Only differences are ever
/// observable (TIMER subtracts the start value), so one origin serves all OSes.
#[cfg(not(target_arch = "wasm32"))]
pub fn ticks() -> i64 {
    ORIGIN.get_or_init(Instant::now).elapsed().as_millis() as i64
}

/// Browser build: the page's monotonic clock (`performance.now()`).
#[cfg(target_arch = "wasm32")]
pub fn ticks() -> i64 {
    web::monotonic_ms() as i64
}

/// Wall-clock seconds since 1970-01-01 UTC (fractional), 0 when unknown.
pub fn unix_time() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        web::now_ms() / 1000.0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
    }
}

/// Blocks the program for `ms` milliseconds (`_DELAY`, `SLEEP`, `_LIMIT`).
pub fn sleep_ms(ms: u64) {
    #[cfg(target_arch = "wasm32")]
    web::sleep_ms(ms as f64);
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::sleep(std::time::Duration::from_millis(ms));
}

/// Browser build: wasm32-unknown-unknown has no clock or sleep,
/// so the page supplies them as imports of module `qbnex` (see
/// `crates/web/www/qbnex.js`).
#[cfg(target_arch = "wasm32")]
pub mod web {
    #[link(wasm_import_module = "qbnex")]
    unsafe extern "C" {
        /// `Date.now()`
        safe fn qb_web_now_ms() -> f64;
        /// `performance.now()`
        safe fn qb_web_monotonic_ms() -> f64;
        /// Waits (a worker can block; the main thread returns at once).
        safe fn qb_web_sleep_ms(ms: f64);
        /// A frame is ready (`_DISPLAY`): the page may draw the screen now.
        safe fn qb_web_present();
        /// Next keyboard/mouse event into `out[0..6]`; 0 when there is none
        /// (layout: `crates/web/www/qbnex.js`, `EVENT_*`).
        fn qb_web_poll_event(out: *mut i32) -> i32;
        /// 1 when the page delivers live keyboard events (INPUT may wait for typing).
        safe fn qb_web_interactive() -> i32;
    }
    pub fn now_ms() -> f64 {
        qb_web_now_ms()
    }
    pub fn monotonic_ms() -> f64 {
        qb_web_monotonic_ms()
    }
    pub fn sleep_ms(ms: f64) {
        qb_web_sleep_ms(ms)
    }
    pub fn present() {
        qb_web_present()
    }
    /// The page delivers live keyboard events.
    pub fn interactive() -> bool {
        qb_web_interactive() != 0
    }
    /// Next page input event: `[kind, a, b, c, d, mods]`.
    pub fn poll_event() -> Option<[i32; 6]> {
        let mut e = [0i32; 6];
        // SAFETY: `e` has the 6 slots the page writes.
        (unsafe { qb_web_poll_event(e.as_mut_ptr()) } != 0).then_some(e)
    }
}

/// Busy-waits until `now()` (wall-clock seconds, `-1` = unknown) changes and
/// returns the new value, like the legacy `time()` loop. If the clock is
/// unknown from the start, returns 0 (midnight, January 1, 1970).
pub fn wait_for_second_change(mut now: impl FnMut() -> i64) -> i64 {
    let old = now();
    if old == -1 {
        return 0;
    }
    loop {
        let t = now();
        if t != old {
            return t;
        }
    }
}

/// Milliseconds past midnight for a local time of day; `None` (localtime
/// failed) is treated as midnight.
pub fn day_ms(hms: Option<(u32, u32, u32)>) -> u32 {
    match hms {
        Some((h, m, s)) => (h * 3600 + m * 60 + s) * 1000,
        None => 0,
    }
}

/// Local hour, minute and second of a Unix time, or `None` where the C
/// `localtime` would fail (before 1970).
pub fn local_hms(unix_seconds: i64) -> Option<(u32, u32, u32)> {
    use chrono::{Local, TimeZone, Timelike};
    if unix_seconds < 0 {
        return None;
    }
    let t = Local.timestamp_opt(unix_seconds, 0).single()?;
    Some((t.hour(), t.minute(), t.second()))
}

fn wall_seconds() -> i64 {
    #[cfg(target_arch = "wasm32")]
    {
        (web::now_ms() / 1000.0) as i64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(-1)
    }
}

/// Start-up reference for TIMER: `(clock_first, day_first)` = tick count and
/// milliseconds past local midnight, taken right after a second boundary.
pub fn time_init() -> (u32, u32) {
    let secs = wait_for_second_change(wall_seconds);
    let clock_first = ticks() as u32;
    (clock_first, day_ms(local_hms(secs)))
}

#[cfg(not(target_arch = "wasm32"))]
static ORIGIN: OnceLock<Instant> = OnceLock::new();

// --- program clock (TIME$, DATE$) ---------------------------------------------------

/// Milliseconds that `TIME$ =` and `DATE$ =` added to the program's view of the
/// local clock. QBasic set the DOS clock; the OS clock is never touched here, so
/// TIME$, DATE$ and TIMER read the wall clock plus this offset.
static CLOCK_OFFSET_MS: AtomicI64 = AtomicI64::new(0);

/// QBasic error for a badly formed `TIME$ =` / `DATE$ =` value.
const ERR_ILLEGAL_CALL: i32 = 5;

/// The offset (milliseconds) the program clock is ahead of the wall clock.
pub fn clock_offset_ms() -> i64 {
    CLOCK_OFFSET_MS.load(Ordering::Relaxed)
}

/// The program's local date and time (the wall clock plus the offset).
fn program_now() -> NaiveDateTime {
    use chrono::{Local, TimeZone};
    let ms = (unix_time() * 1000.0) as i64;
    let wall = Local
        .timestamp_millis_opt(ms)
        .earliest()
        .map(|t| t.naive_local())
        .unwrap_or_default();
    TimeDelta::try_milliseconds(clock_offset_ms())
        .and_then(|d| wall.checked_add_signed(d))
        .unwrap_or(wall)
}

/// Moves the program clock so that it now reads `target`.
fn set_program_clock(target: NaiveDateTime) {
    let delta = target.signed_duration_since(program_now()).num_milliseconds();
    CLOCK_OFFSET_MS.fetch_add(delta, Ordering::Relaxed);
}

/// `TIME$`: "hh:mm:ss" (24-hour) of the program clock.
pub fn time_string() -> String {
    program_now().format("%H:%M:%S").to_string()
}

/// `DATE$`: "mm-dd-yyyy" of the program clock.
pub fn date_string() -> String {
    program_now().format("%m-%d-%Y").to_string()
}

/// Splits `s` at any of `seps` into numbers of 1..=`max_digits` digits each.
fn clock_fields(s: &[u8], seps: &[u8], max_digits: usize) -> Option<Vec<(u32, usize)>> {
    s.split(|c| seps.contains(c))
        .map(|f| {
            if f.is_empty() || f.len() > max_digits || !f.iter().all(u8::is_ascii_digit) {
                return None;
            }
            Some((f.iter().fold(0u32, |n, d| n * 10 + u32::from(d - b'0')), f.len()))
        })
        .collect()
}

/// Parses a `TIME$ =` value: "hh", "hh:mm" or "hh:mm:ss".
pub fn parse_time(s: &[u8]) -> Option<NaiveTime> {
    let f = clock_fields(s, b":", 2)?;
    if f.is_empty() || f.len() > 3 {
        return None;
    }
    let part = |k: usize| f.get(k).map_or(0, |p| p.0);
    NaiveTime::from_hms_opt(part(0), part(1), part(2))
}

/// Parses a `DATE$ =` value: "mm-dd-yy", "mm-dd-yyyy", or the same with "/".
/// A two-digit year is 1980..=2079 (80-99 are 19xx); the year must be 1980..=2099.
pub fn parse_date(s: &[u8]) -> Option<NaiveDate> {
    let f = clock_fields(s, b"-/", 4)?;
    let [(m, ml), (d, dl), (y, yl)] = f[..] else { return None };
    if ml > 2 || dl > 2 || (yl != 2 && yl != 4) {
        return None;
    }
    let y = match yl {
        2 if y >= 80 => 1900 + y,
        2 => 2000 + y,
        _ => y,
    };
    if !(1980..=2099).contains(&y) {
        return None;
    }
    NaiveDate::from_ymd_opt(y as i32, m, d)
}

/// `TIME$ = s`: the program clock reads `s` on today's (program) date from now on.
pub fn set_time(s: &[u8]) -> Result<(), i32> {
    let t = parse_time(s).ok_or(ERR_ILLEGAL_CALL)?;
    set_program_clock(program_now().date().and_time(t));
    Ok(())
}

/// `DATE$ = s`: the program clock reads date `s` at the current (program) time of day.
pub fn set_date(s: &[u8]) -> Result<(), i32> {
    let d = parse_date(s).ok_or(ERR_ILLEGAL_CALL)?;
    set_program_clock(d.and_time(program_now().time()));
    Ok(())
}

// --- C ABI (include/qb_runtime.h) ---------------------------------------------------

/// `GetTicks()`: monotonic milliseconds.
#[unsafe(no_mangle)]
pub extern "C" fn qb_get_ticks() -> i64 {
    ticks()
}

/// Fills TIMER's start-up reference values.
///
/// # Safety
/// Both pointers must be writable for one `uint32_t`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_time_init(clock_first: *mut u32, day_first: *mut u32) {
    let (c, d) = time_init();
    // SAFETY: caller guarantees both pointers are writable.
    unsafe {
        clock_first.write_unaligned(c);
        day_first.write_unaligned(d);
    }
}

/// `day_ms` for the differential test: `valid == 0` means localtime failed.
#[unsafe(no_mangle)]
pub extern "C" fn qb_day_ms(hour: u32, minute: u32, second: u32, valid: i32) -> u32 {
    day_ms((valid != 0).then_some((hour, minute, second)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn ticks_are_monotonic_milliseconds() {
        let a = ticks();
        std::thread::sleep(Duration::from_millis(60));
        let b = ticks();
        assert!(b - a >= 60, "{a} -> {b}");
        assert!(b - a < 5_000, "{a} -> {b}");
        let mut prev = ticks();
        for _ in 0..10_000 {
            let t = ticks();
            assert!(t >= prev);
            prev = t;
        }
    }

    #[test]
    fn waits_for_the_next_second() {
        let mut seq = [100, 100, 100, 101, 102].into_iter();
        assert_eq!(wait_for_second_change(|| seq.next().unwrap()), 101);
        let mut seq = [7, 8].into_iter();
        assert_eq!(wait_for_second_change(|| seq.next().unwrap()), 8);
    }

    #[test]
    fn unknown_clock_means_midnight_1970() {
        let mut calls = 0;
        assert_eq!(wait_for_second_change(|| { calls += 1; -1 }), 0);
        assert_eq!(calls, 1, "legacy reads time() once when it fails");
    }

    #[test]
    fn day_ms_counts_from_midnight() {
        assert_eq!(day_ms(Some((0, 0, 0))), 0);
        assert_eq!(day_ms(Some((0, 0, 1))), 1_000);
        assert_eq!(day_ms(Some((1, 2, 3))), 3_723_000);
        assert_eq!(day_ms(Some((23, 59, 59))), 86_399_000);
        assert_eq!(day_ms(None), 0);
    }

    #[test]
    fn local_time_matches_the_wall_clock() {
        assert_eq!(local_hms(-1), None);
        let now = wall_seconds();
        let (h, m, s) = local_hms(now).unwrap();
        assert!(h < 24 && m < 60 && s < 61);
        // Same second-of-minute as UTC (time zones are whole minutes).
        assert_eq!(s as i64, now.rem_euclid(60));
    }

    #[test]
    fn clock_values_parse_like_qbasic() {
        let t = |s: &str| parse_time(s.as_bytes()).map(|t| t.format("%H:%M:%S").to_string());
        assert_eq!(t("13:45:07").as_deref(), Some("13:45:07"));
        assert_eq!(t("9:5").as_deref(), Some("09:05:00"));
        assert_eq!(t("23").as_deref(), Some("23:00:00"));
        for bad in ["", "24:00:00", "12:60", "12:00:60", "1:2:3:4", "ab", "12::00", "123:00", " 12:00"] {
            assert_eq!(t(bad), None, "{bad:?}");
        }
        let d = |s: &str| parse_date(s.as_bytes()).map(|d| d.format("%m-%d-%Y").to_string());
        assert_eq!(d("07-04-1999").as_deref(), Some("07-04-1999"));
        assert_eq!(d("2/29/2000").as_deref(), Some("02-29-2000"));
        assert_eq!(d("12-31-85").as_deref(), Some("12-31-1985"));
        assert_eq!(d("01-01-05").as_deref(), Some("01-01-2005"));
        for bad in ["", "13-01-2000", "02-30-2000", "01-01-1979", "01-01-2100", "1-1-200", "01-01", "01.01.2000", "001-01-2000"] {
            assert_eq!(d(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn setting_the_clock_moves_only_the_program_clock() {
        let before = clock_offset_ms();
        set_time(b"03:04:05").unwrap();
        assert!(time_string().starts_with("03:04"), "{}", time_string());
        set_date(b"06-15-2001").unwrap();
        assert_eq!(date_string(), "06-15-2001");
        assert!(time_string().starts_with("03:04"), "{}", time_string());
        assert_eq!(set_time(b"25:00"), Err(5));
        assert_eq!(set_date(b"6/31/2001"), Err(5));
        assert_eq!(date_string(), "06-15-2001");
        CLOCK_OFFSET_MS.store(before, Ordering::Relaxed);
    }

    #[test]
    fn time_init_is_consistent_with_timer() {
        let (clock_first, day_first) = time_init();
        assert!(day_first < 86_400_000);
        assert_eq!(day_first % 1000, 0);
        let t = crate::timer::timer(ticks() as u32, clock_first, day_first, Some(0.001)).unwrap();
        assert!((t * 1000.0 - f64::from(day_first)).abs() < 2_000.0 || t < 2.0, "{t} {day_first}");
    }
}
