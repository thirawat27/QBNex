//! `TIMER` / `TIMER(accuracy)`: seconds since midnight.
//!
//! Replaces the arithmetic of `func_timer` in libqb.cpp. The clock reads it
//! takes as arguments (`GetTicks` and the start-up time of day) come from
//! [`crate::systime`].

/// Milliseconds in a day; TIMER wraps at midnight.
const DAY_MS: u32 = 86_400_000;

/// QBasic error for `TIMER(accuracy)` with `accuracy <= 0`.
pub const ERR_ILLEGAL_CALL: i32 = 5;

/// `ticks`: current millisecond tick count; `clock_first`: tick count at
/// program start; `day_first`: milliseconds since midnight at program start.
/// `accuracy`: `None` for plain `TIMER` (18.2 Hz, returned as SINGLE).
pub fn timer(ticks: u32, clock_first: u32, day_first: u32, accuracy: Option<f64>) -> Result<f64, i32> {
    let ms = ticks.wrapping_sub(clock_first).wrapping_add(day_first) % DAY_MS;
    let seconds = f64::from(ms) / 1000.0;
    let steps_per_second = match accuracy {
        None => 18.2,
        Some(a) if a <= 0.0 || a.is_nan() => return Err(ERR_ILLEGAL_CALL),
        Some(a) => 1.0 / a,
    };
    let steps = crate::round::qbr(crate::round::f80_from_f64(seconds * steps_per_second));
    let value = steps as f64 / steps_per_second;
    Ok(if accuracy.is_none() { f64::from(value as f32) } else { value })
}

// --- C ABI (include/qb_runtime.h) ---------------------------------------------------

/// Writes TIMER's value to `out` and returns 0, or returns a QBasic error code.
///
/// # Safety
/// `out` must be writable for one `double`.
#[no_mangle]
pub unsafe extern "C" fn qb_timer(
    ticks: u32,
    clock_first: u32,
    day_first: u32,
    accuracy: f64,
    passed: i32,
    out: *mut f64,
) -> i32 {
    let accuracy = (passed != 0).then_some(accuracy);
    let (value, code) = match timer(ticks, clock_first, day_first, accuracy) {
        Ok(v) => (v, 0),
        Err(e) => (0.0, e),
    };
    // SAFETY: caller guarantees `out` is writable.
    unsafe { out.write_unaligned(value) };
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn millisecond_accuracy_is_exact_seconds() {
        assert_eq!(timer(12_345, 0, 0, Some(0.001)), Ok(12.345));
        assert_eq!(timer(0, 0, 0, Some(0.001)), Ok(0.0));
        assert_eq!(timer(1_000, 0, 0, Some(1.0)), Ok(1.0));
        assert_eq!(timer(1_499, 0, 0, Some(1.0)), Ok(1.0));
        assert_eq!(timer(1_500, 0, 0, Some(1.0)), Ok(2.0)); // half to even
        assert_eq!(timer(2_500, 0, 0, Some(1.0)), Ok(2.0));
    }

    #[test]
    fn default_resolution_is_18_2_hz_as_single() {
        for ms in [0u32, 1, 999, 1_000, 54_321, 3_600_000, 86_399_999] {
            let t = timer(ms, 0, 0, None).unwrap();
            let ticks = (f64::from(ms) / 1000.0 * 18.2).round_ties_even();
            assert_eq!(t, f64::from((ticks / 18.2) as f32), "ms={ms}");
            assert_eq!(t, f64::from(t as f32), "value is a SINGLE");
        }
        assert_eq!(timer(1_000, 0, 0, None), Ok(f64::from((18.0f64 / 18.2) as f32)));
    }

    #[test]
    fn counts_from_start_time_and_wraps_at_midnight() {
        // Started at 23:59:59.000 (day_first) when the tick counter read 5_000.
        let start = DAY_MS - 1_000;
        assert_eq!(timer(5_000, 5_000, start, Some(0.001)), Ok(86_399.0));
        assert_eq!(timer(6_500, 5_000, start, Some(0.001)), Ok(0.5));
        // Tick counter overflow (u32 wraps after ~49.7 days) still measures elapsed time.
        assert_eq!(timer(100, u32::MAX - 899, 0, Some(0.001)), Ok(1.0));
    }

    #[test]
    fn c_abi_reports_value_or_error() {
        let mut out = -1.0;
        // SAFETY: `out` is a valid double.
        assert_eq!(unsafe { qb_timer(2_000, 0, 0, 123.0, 0, &mut out) }, 0);
        assert_eq!(Ok(out), timer(2_000, 0, 0, None)); // accuracy ignored when not passed
        // SAFETY: as above.
        assert_eq!(unsafe { qb_timer(2_000, 0, 0, 0.001, 1, &mut out) }, 0);
        assert_eq!(out, 2.0);
        // SAFETY: as above.
        assert_eq!(unsafe { qb_timer(2_000, 0, 0, 0.0, 1, &mut out) }, ERR_ILLEGAL_CALL);
        assert_eq!(out, 0.0);
    }

    #[test]
    fn non_positive_accuracy_is_illegal_function_call() {
        assert_eq!(timer(1, 0, 0, Some(0.0)), Err(ERR_ILLEGAL_CALL));
        assert_eq!(timer(1, 0, 0, Some(-1.0)), Err(ERR_ILLEGAL_CALL));
    }
}
