//! `STR$` number formatting (also used by `PRINT` for numbers).
//!
//! Replaces the `qbs_str` overloads of libqb.cpp. The legacy algorithm formats
//! with C `printf` ("% .6E" / "% .15E" / "% .Nf") and then edits the text; the
//! port reproduces that text exactly, including two legacy quirks that golden
//! output depends on:
//!
//! - DOUBLE: the significant-digit count is really a buffer index, so values
//!   with two or more significant digits get one extra decimal
//!   (`STR$(42#)` = " 42.0", `STR$(-0.25#)` = "-.250").
//! - Non-finite values print as " 0".
//!
//! Intentional deviations (see docs/dev/deviations.md):
//! - Legacy built a broken printf format when more than 19 decimals were
//!   needed (`STR$(1.234567890123456D-16)` gave " .0f"); the port prints the
//!   decimals the algorithm asked for.
//! - Legacy SINGLE formatting of infinity/NaN read a stale buffer; the
//!   port prints " 0" like DOUBLE does.

/// Upper bound on the length of any `STR$` result (C callers size buffers with it).
pub const STR_MAX: usize = 64;

/// `STR$` of any signed integer type (C `"% i"`).
pub fn str_i64(v: i64) -> Vec<u8> {
    if v < 0 {
        v.to_string().into_bytes()
    } else {
        format!(" {v}").into_bytes()
    }
}

/// `STR$` of any unsigned integer type (C `" %u"`).
pub fn str_u64(v: u64) -> Vec<u8> {
    format!(" {v}").into_bytes()
}

/// C `printf("% .{prec}E", v)`: sign column, exponent with at least 2 digits.
fn c_sci(v: f64, prec: usize) -> Vec<u8> {
    let sign = if v.is_sign_negative() { '-' } else { ' ' };
    let text = format!("{:.*e}", prec, v.abs());
    let (mantissa, exp) = text.split_once('e').expect("exponent");
    let exp: i32 = exp.parse().expect("exponent digits");
    let exp_sign = if exp < 0 { '-' } else { '+' };
    format!("{sign}{mantissa}E{exp_sign}{:02}", exp.abs()).into_bytes()
}

/// C `printf("% .{decimals}f", v)`.
fn c_fixed(v: f64, decimals: usize) -> Vec<u8> {
    let sign = if v.is_sign_negative() { '-' } else { ' ' };
    format!("{sign}{:.*}", decimals, v.abs()).into_bytes()
}

/// Removes the leading "0" of " 0.xxx" / "-0.xxx" (QBasic prints " .xxx").
fn cull_leading_zero(mut out: Vec<u8>) -> Vec<u8> {
    if out.get(1) == Some(&b'0') && out.len() > 2 {
        out.remove(1);
    }
    out
}

fn zero() -> Vec<u8> {
    b" 0".to_vec()
}

/// `STR$` of a SINGLE.
pub fn str_f32(v: f32) -> Vec<u8> {
    if !v.is_finite() {
        return zero(); // legacy read a stale buffer here
    }
    let v = f64::from(v); // printf promotes float to double
    let mut buf = c_sci(v, 6);
    if buf.len() == 13 {
        buf.insert(11, b'0'); // normalise to a 3-digit exponent
    }
    let mut digits: i32 = 7;
    for i in (1..=8).rev() {
        if buf[i] == b'0' {
            digits -= 1;
        } else if buf[i] != b'.' {
            break;
        }
    }
    if digits == 0 {
        return zero();
    }
    let mut exponent = i32::from(buf[11] - b'0') * 100
        + i32::from(buf[12] - b'0') * 10
        + i32::from(buf[13] - b'0');
    if buf[10] == b'-' {
        exponent = -exponent;
    }
    if exponent <= 6 && exponent - digits >= -8 {
        let decimals = (-(exponent - digits + 1)).max(0) as usize;
        return cull_leading_zero(c_fixed(v, decimals));
    }
    let mut keep = (digits + 2) as usize;
    if digits == 1 {
        keep -= 1; // no decimal point
    }
    let mut out = buf[..keep].to_vec();
    out.extend_from_slice(&buf[9..=10]); // "E" and sign
    let first = if exponent.abs() > 99 { 11 } else { 12 };
    out.extend_from_slice(&buf[first..=13]);
    out
}

/// `STR$` of a DOUBLE (and of a _FLOAT, which legacy converted to double).
pub fn str_f64(v: f64) -> Vec<u8> {
    if !v.is_finite() {
        return zero();
    }
    let buf = c_sci(v, 15);
    let l = buf.len();
    let at = |i: usize| buf.get(i).copied().unwrap_or(0); // index l is the C terminator

    // Exponent: scan backwards to the sign.
    let (mut exponent, mut mult) = (0i32, 1i32);
    for i in (1..=l).rev() {
        let c = at(i);
        if c == 0 || c == b' ' {
            continue;
        }
        if c == b'+' {
            break;
        }
        if c == b'-' {
            exponent = -exponent;
            break;
        }
        exponent += i32::from(c - b'0') * mult;
        mult *= 10;
    }
    // Position of the 'E' and of the last significant mantissa character.
    // Legacy calls that position "digits": it is an index, not a count.
    let (mut digits, mut exp_at) = (0usize, 0usize);
    for i in (1..=l).rev() {
        let c = at(i);
        if exp_at == 0 {
            if c == b'E' {
                exp_at = i;
            }
        } else if c != b'0' && c != b'.' {
            digits = i;
            break;
        }
    }
    if digits == 0 {
        return zero();
    }
    if exponent >= 16 || exponent <= -17 {
        let mut out = buf[..=digits].to_vec();
        out.push(b'D');
        out.extend_from_slice(&buf[exp_at + 1..l]);
        return out;
    }
    let decimals = (-(exponent - digits as i32 + 1)).max(0) as usize; // no 19 limit
    cull_leading_zero(c_fixed(v, decimals))
}

// --- C ABI (include/qb_runtime.h): write into a caller buffer of QB_STR_MAX bytes,
// return the length. The qbs allocation stays in the libqb.cpp shims. ---------

/// Copies `text` (at most `STR_MAX` bytes) to `out` and returns its length.
///
/// # Safety
/// `out` must be writable for `STR_MAX` bytes.
unsafe fn emit(text: &[u8], out: *mut u8) -> i32 {
    let n = text.len().min(STR_MAX);
    // SAFETY: guaranteed by the caller.
    unsafe { core::ptr::copy_nonoverlapping(text.as_ptr(), out, n) };
    n as i32
}

/// # Safety
/// `out` must be writable for `STR_MAX` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_str_i64(v: i64, out: *mut u8) -> i32 {
    // SAFETY: forwarded caller contract.
    unsafe { emit(&str_i64(v), out) }
}

/// # Safety
/// `out` must be writable for `STR_MAX` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_str_u64(v: u64, out: *mut u8) -> i32 {
    // SAFETY: forwarded caller contract.
    unsafe { emit(&str_u64(v), out) }
}

/// # Safety
/// `out` must be writable for `STR_MAX` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_str_f32(v: f32, out: *mut u8) -> i32 {
    // SAFETY: forwarded caller contract.
    unsafe { emit(&str_f32(v), out) }
}

/// # Safety
/// `out` must be writable for `STR_MAX` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_str_f64(v: f64, out: *mut u8) -> i32 {
    // SAFETY: forwarded caller contract.
    unsafe { emit(&str_f64(v), out) }
}

/// `STR$` of a `_FLOAT` given as the 10 bytes of an x87 `long double`: legacy
/// `qbs_str(long double)` formats `(double)value`.
pub fn str_f80(v: [u8; 10]) -> Vec<u8> {
    str_f64(qb_float80::F80::from_bytes(v).to_f64())
}

/// # Safety
/// `v` readable for 10 bytes; `out` writable for `STR_MAX` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_str_f80(v: *const u8, out: *mut u8) -> i32 {
    // SAFETY: caller guarantees 10 readable bytes.
    let x = unsafe { qb_float80::F80::read(v) };
    // SAFETY: caller guarantees STR_MAX writable bytes.
    unsafe { emit(&str_f80(x.to_bytes()), out) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_abi_writes_text_and_returns_length() {
        let mut buf = [0xAAu8; STR_MAX + 1];
        // SAFETY: buffer holds STR_MAX bytes.
        let n = unsafe { qb_str_f64(42.0, buf.as_mut_ptr()) } as usize;
        assert_eq!(&buf[..n], b" 42.0");
        assert_eq!(buf[n], 0xAA, "no terminator or overrun");
        // SAFETY: as above.
        let n = unsafe { qb_str_f32(0.1, buf.as_mut_ptr()) } as usize;
        assert_eq!(&buf[..n], b" .1");
        // SAFETY: as above.
        let n = unsafe { qb_str_i64(-5, buf.as_mut_ptr()) } as usize;
        assert_eq!(&buf[..n], b"-5");
        // SAFETY: as above.
        let n = unsafe { qb_str_u64(7, buf.as_mut_ptr()) } as usize;
        assert_eq!(&buf[..n], b" 7");
    }

    fn s(b: Vec<u8>) -> String {
        String::from_utf8(b).unwrap()
    }

    #[test]
    fn integers_have_a_sign_column() {
        assert_eq!(s(str_i64(0)), " 0");
        assert_eq!(s(str_i64(42)), " 42");
        assert_eq!(s(str_i64(-7)), "-7");
        assert_eq!(s(str_i64(i64::MIN)), "-9223372036854775808");
        assert_eq!(s(str_u64(0)), " 0");
        assert_eq!(s(str_u64(u64::MAX)), " 18446744073709551615");
    }

    #[test]
    fn single_precision_matches_legacy() {
        let cases: [(f32, &str); 13] = [
            (42.0, " 42"),
            (0.1, " .1"),
            (1234567.0, " 1234567"),
            (12345678.0, " 1.234568E+07"),
            (1.5e-8, " 1.5E-08"),
            (-3.25e20, "-3.25E+20"),
            (0.0, " 0"),
            (-0.0, " 0"),
            (1e38, " 1E+38"),
            (f32::from_bits(1), " 1.401298E-45"), // smallest subnormal SINGLE
            (3.5, " 3.5"),
            (-0.25, "-.25"),
            (1.0 / 3.0, " .3333333"),
        ];
        for (v, want) in cases {
            assert_eq!(s(str_f32(v)), want, "STR$({v}!)");
        }
    }

    #[test]
    fn double_precision_matches_legacy_including_extra_decimal_quirk() {
        let cases: [(f64, &str); 16] = [
            (42.0, " 42.0"),
            (7.0, " 7"),
            (-0.25, "-.250"),
            (0.1, " .1"),
            (1e15, " 1000000000000000"),
            (1e16, " 1D+16"),
            (1e-16, " .0000000000000001"),
            (1.5e-10, " .000000000150"),
            (1e-17, " 1D-17"),
            (123456789012345.6, " 123456789012345.59"),
            (0.000123, " .0001230"),
            (1.25e-5, " .00001250"),
            (0.0, " 0"),
            (-1e-17, "-1D-17"),
            (f64::MAX, " 1.797693134862316D+308"),
            (5e-324, " 4.940656458412465D-324"),
        ];
        for (v, want) in cases {
            assert_eq!(s(str_f64(v)), want, "STR$({v:e}#)");
        }
    }

    #[test]
    fn non_finite_values_print_zero() {
        for v in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            assert_eq!(s(str_f64(v)), " 0");
            assert_eq!(s(str_f32(v as f32)), " 0");
        }
    }

    #[test]
    fn more_than_19_decimals_no_longer_breaks_the_format() {
        // Legacy printed " .0f" here. 32 decimals are requested.
        let got = s(str_f64(1.234567890123456e-16));
        assert!(got.starts_with(" .000000000000000123456789012345"), "{got}");
        assert_eq!(got.len(), 2 + 32);
    }

    #[test]
    fn float_formats_through_double() {
        let tenth = qb_float80::dec::parse(b"0.1").0;
        assert_eq!(s(str_f80(tenth.to_bytes())), " .1");
        let third = qb_float80::F80::ONE / qb_float80::F80::from_i64(3);
        assert_eq!(str_f80(third.to_bytes()), str_f64(1.0 / 3.0));
        let mut buf = [0u8; STR_MAX];
        let n = unsafe { qb_str_f80(third.to_bytes().as_ptr(), buf.as_mut_ptr()) } as usize;
        assert_eq!(&buf[..n], str_f64(1.0 / 3.0).as_slice());
    }

    #[test]
    fn results_fit_the_c_buffer() {
        let mut x = 1.0f64;
        for _ in 0..2000 {
            for v in [x, -x, 1.0 / x, -1.0 / x, x * 0.987654321] {
                assert!(str_f64(v).len() <= STR_MAX);
                assert!(str_f32(v as f32).len() <= STR_MAX);
            }
            x *= 1.7;
        }
    }
}
