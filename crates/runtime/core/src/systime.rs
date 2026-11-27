//! Clock reads for TIMER, `_DELAY`, `_LIMIT` and the cursor blink.
//!
//! A monotonic millisecond tick count, the wall clock and the local time of
//! day, and the program clock that `TIME$ =` and `DATE$ =` move.

use crate::error::code;
use jiff::civil::{Date, DateTime, Time};
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};
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

/// Milliseconds past midnight for a local time of day; `None` (localtime
/// failed) is treated as midnight.
pub fn day_ms(hms: Option<(u32, u32, u32)>) -> u32 {
    match hms {
        Some((h, m, s)) => (h * 3600 + m * 60 + s) * 1000,
        None => 0,
    }
}

/// Local hour, minute and second of a Unix time, or `None` where the system
/// `localtime` would fail (before 1970).
pub fn local_hms(unix_seconds: i64) -> Option<(u32, u32, u32)> {
    if unix_seconds < 0 {
        return None;
    }
    let t = TimeZone::system().to_datetime(Timestamp::from_second(unix_seconds).ok()?);
    Some((t.hour() as u32, t.minute() as u32, t.second() as u32))
}

#[cfg(not(target_arch = "wasm32"))]
static ORIGIN: OnceLock<Instant> = OnceLock::new();

// --- program clock (TIME$, DATE$) ---------------------------------------------------

/// Milliseconds that `TIME$ =` and `DATE$ =` added to the program's view of the
/// local clock. QBasic set the DOS clock; the OS clock is never touched here, so
/// TIME$, DATE$ and TIMER read the wall clock plus this offset.
static CLOCK_OFFSET_MS: AtomicI64 = AtomicI64::new(0);


/// The offset (milliseconds) the program clock is ahead of the wall clock.
pub fn clock_offset_ms() -> i64 {
    CLOCK_OFFSET_MS.load(Ordering::Relaxed)
}

/// The program's local date and time (the wall clock plus the offset).
fn program_now() -> DateTime {
    let ms = (unix_time() * 1000.0) as i64;
    // (1970-01-01 00:00 when the clock cannot be read)
    let wall = Timestamp::from_millisecond(ms).map_or(DateTime::constant(1970, 1, 1, 0, 0, 0, 0), |t| TimeZone::system().to_datetime(t));
    wall.checked_add(SignedDuration::from_millis(clock_offset_ms())).unwrap_or(wall)
}

/// Moves the program clock so that it now reads `target`.
fn set_program_clock(target: DateTime) {
    let delta = i64::try_from(target.duration_since(program_now()).as_millis()).unwrap_or(0);
    CLOCK_OFFSET_MS.fetch_add(delta, Ordering::Relaxed);
}

/// `TIME$`: "hh:mm:ss" (24-hour) of the program clock.
pub fn time_string() -> String {
    program_now().strftime("%H:%M:%S").to_string()
}

/// `DATE$`: "mm-dd-yyyy" of the program clock.
pub fn date_string() -> String {
    program_now().strftime("%m-%d-%Y").to_string()
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
pub fn parse_time(s: &[u8]) -> Option<Time> {
    let f = clock_fields(s, b":", 2)?;
    if f.is_empty() || f.len() > 3 {
        return None;
    }
    let part = |k: usize| f.get(k).map_or(0, |p| p.0);
    // (fields have at most 2 digits: they fit an i8)
    Time::new(part(0) as i8, part(1) as i8, part(2) as i8, 0).ok()
}

/// Parses a `DATE$ =` value: "mm-dd-yy", "mm-dd-yyyy", or the same with "/".
/// A two-digit year is 1980..=2079 (80-99 are 19xx); the year must be 1980..=2099.
pub fn parse_date(s: &[u8]) -> Option<Date> {
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
    Date::new(y as i16, m as i8, d as i8).ok()
}

/// `TIME$ = s`: the program clock reads `s` on today's (program) date from now on.
pub fn set_time(s: &[u8]) -> Result<(), i32> {
    let t = parse_time(s).ok_or(code::ILLEGAL_FUNCTION_CALL)?;
    set_program_clock(program_now().date().to_datetime(t));
    Ok(())
}

/// `DATE$ = s`: the program clock reads date `s` at the current (program) time of day.
pub fn set_date(s: &[u8]) -> Result<(), i32> {
    let d = parse_date(s).ok_or(code::ILLEGAL_FUNCTION_CALL)?;
    set_program_clock(d.to_datetime(program_now().time()));
    Ok(())
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
        let now = unix_time() as i64;
        let (h, m, s) = local_hms(now).unwrap();
        assert!(h < 24 && m < 60 && s < 61);
        // Same second-of-minute as UTC (time zones are whole minutes).
        assert_eq!(s as i64, now.rem_euclid(60));
    }

    #[test]
    fn clock_values_parse_like_qbasic() {
        let t = |s: &str| parse_time(s.as_bytes()).map(|t| t.strftime("%H:%M:%S").to_string());
        assert_eq!(t("13:45:07").as_deref(), Some("13:45:07"));
        assert_eq!(t("9:5").as_deref(), Some("09:05:00"));
        assert_eq!(t("23").as_deref(), Some("23:00:00"));
        for bad in ["", "24:00:00", "12:60", "12:00:60", "1:2:3:4", "ab", "12::00", "123:00", " 12:00"] {
            assert_eq!(t(bad), None, "{bad:?}");
        }
        let d = |s: &str| parse_date(s.as_bytes()).map(|d| d.strftime("%m-%d-%Y").to_string());
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
}
