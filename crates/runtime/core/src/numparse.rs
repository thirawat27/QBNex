//! READ/DATA and the "universal number representation".
//!
//! QB64's `n_inputnumberfromdata`, `n_float`, `n_int64`, `n_uint64`,
//! `n_roundincrement`, `sub_read_string`, `func_read_float`,
//! `func_read_int64`, `func_read_uint64` and the conversion half of
//! `func_file_input_float/int64/uint64`.
//!
//! A number item is first split into digits ([`NumRepr`], shared with the
//! INPUT # reader in `fileio.rs`) and then converted to the target type.
//! The legacy `long double n_float_value` held either a `double` (decimal
//! numbers went through `sscanf("%lf")`) or an exact `int64` (hex/oct/bin);
//! [`NumValue`] keeps both forms so no precision is lost.

pub use crate::fileio::NumRepr;
use crate::round::{f80_from_f64, qbr};

/// `ISFLOAT` / `ISUNSIGNED` type flags (common.h).
pub const ISFLOAT: i32 = 536_870_912;
pub const ISUNSIGNED: i32 = 268_435_456;

/// Value of the legacy `long double n_float_value`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NumValue {
    /// Exact integer (hex/oct/bin digits, or an integer READ target).
    Int(i64),
    /// Decimal number parsed as a `double`.
    Float(f64),
}

impl NumValue {
    /// The value as the nearest `f64` (exact for every `Float`).
    pub fn to_f64(self) -> f64 {
        match self {
            NumValue::Int(i) => i as f64,
            NumValue::Float(f) => f,
        }
    }
    /// `qbr(n_float_value)`: x87 round-half-even to int64.
    fn qbr(self) -> i64 {
        match self {
            NumValue::Int(i) => i,
            NumValue::Float(f) => qbr(f80_from_f64(f)),
        }
    }
}

const RANGE_INT64_MAX: &[u8] = b"9223372036854775807";
const RANGE_INT64_NEG_MAX: &[u8] = b"9223372036854775808";
const RANGE_UINT64_MAX: &[u8] = b"18446744073709551615";
const RANGE_FLOAT_MAX: &[u8] = b"17976931348623157";

fn is_type_suffix(c: u8) -> bool {
    matches!(c, b'!' | b'#' | b'$' | b'%' | b'&' | b'`' | b'~')
}

/// `n_inputnumberfromdata`: reads one number item of a DATA block at
/// `*offset`. Returns 0 success, 1 overflow, 2 out of DATA, 3 syntax error.
/// The offset is left where the legacy parser left it; callers restore it.
pub fn number_from_data(data: &[u8], offset: &mut usize, n: &mut NumRepr) -> i32 {
    let size = data.len();
    let mut step = 0;
    let mut negate_exponent = false;
    let mut exponent_value: i64 = 0;
    let mut exponent_digits = 0;
    n.digits = 0;
    n.exp = 0;
    n.neg = 0;
    n.hex = 0;
    if *offset >= size {
        return 2;
    }
    let mut c = data[*offset];

    if c == b'&' {
        *offset += 1;
        if *offset >= size {
            return 3;
        }
        c = data[*offset];
        if c == b',' {
            *offset += 1;
            return 3;
        }
        let (kind, valid): (u8, fn(u8) -> bool) = match c {
            b'H' | b'h' => (1, |c| c.is_ascii_hexdigit()),
            b'O' | b'o' => (2, |c| (b'0'..=b'7').contains(&c)),
            b'B' | b'b' => (3, |c| c == b'0' || c == b'1'),
            _ => return 3,
        };
        loop {
            *offset += 1;
            if *offset >= size {
                break;
            }
            c = data[*offset];
            if c == b',' {
                *offset += 1;
                break;
            }
            if valid(c) {
                if step == 5 {
                    return 3;
                }
                if n.digits == 256 {
                    return 1;
                }
                n.digit[n.digits as usize] = c;
                n.digits += 1;
                n.hex = kind;
                continue;
            }
            if is_type_suffix(c) && step <= 5 {
                step = 5;
                continue;
            }
            return 3;
        }
    } else {
        loop {
            // readnextchr
            if c == b',' {
                *offset += 1;
                break;
            }
            match c {
                b'-' => {
                    if step == 0 {
                        n.neg = 1;
                        step = 1;
                    } else if step == 3 {
                        negate_exponent = true;
                        step = 4;
                    } else {
                        return 3;
                    }
                }
                b'+' => {
                    if step == 0 {
                        step = 1;
                    } else if step == 3 {
                        step = 4;
                    } else {
                        return 3;
                    }
                }
                b'0'..=b'9' => {
                    if step == 5 {
                        return 3;
                    }
                    if step <= 1 {
                        step = 1;
                        if n.digits != 0 || c > b'0' {
                            if n.digits != 0 {
                                n.exp += 1;
                            }
                            if n.digits == 256 {
                                return 1;
                            }
                            n.digit[n.digits as usize] = c;
                            n.digits += 1;
                        }
                    }
                    if step == 2 {
                        if n.digits == 0 && c == b'0' {
                            n.exp -= 1;
                        }
                        if n.digits != 0 || c > b'0' {
                            if n.digits == 256 {
                                return 1;
                            }
                            n.digit[n.digits as usize] = c;
                            n.digits += 1;
                        }
                    }
                    if step >= 3 {
                        step = 4;
                        if exponent_digits != 0 || c > b'0' {
                            if exponent_digits == 18 {
                                return 1;
                            }
                            exponent_value = exponent_value * 10 + i64::from(c - b'0');
                            exponent_digits += 1;
                        }
                    }
                }
                b'.' => {
                    if step > 1 {
                        return 3;
                    }
                    if n.digits == 0 {
                        n.exp = -1;
                    }
                    step = 2;
                }
                b'D' | b'E' | b'd' | b'e' => {
                    if step > 2 {
                        return 3;
                    }
                    step = 3;
                }
                _ if is_type_suffix(c) && step <= 5 => step = 5,
                _ => return 3,
            }
            // nextchr
            *offset += 1;
            if *offset >= size {
                break;
            }
            c = data[*offset];
        }
    }
    // gotnumber
    if negate_exponent {
        n.exp -= exponent_value;
    } else {
        n.exp += exponent_value;
    }
    if n.digits == 0 {
        n.exp = 0;
        n.neg = 0;
    }
    0
}

/// Accumulates hex/oct/bin digits exactly like the legacy loops (wrapping,
/// unrecognised characters are OR-ed in raw).
fn radix_value(n: &NumRepr) -> Option<u64> {
    let digits = &n.digit[..n.digits as usize];
    let mut v: u64 = 0;
    match n.hex {
        1 => {
            if n.digits > 16 {
                return None;
            }
            for &d in digits {
                let mut x = u64::from(d);
                if (48..=57).contains(&d) {
                    x -= 48;
                }
                if (65..=70).contains(&d) {
                    x -= 55;
                }
                if (97..=102).contains(&d) {
                    x -= 87;
                }
                v = (v << 4) | x;
            }
        }
        2 => {
            if n.digits >= 22 && (n.digits > 22 || n.digit[0] > 49) {
                return None;
            }
            for &d in digits {
                v = (v << 3) | (i64::from(d) - 48) as u64;
            }
        }
        _ => {
            if n.digits > 64 {
                return None;
            }
            for &d in digits {
                v = (v << 1) | (i64::from(d) - 48) as u64;
            }
        }
    }
    Some(v)
}

/// `n_roundincrement`: 1 when the first digit after the decimal point is 5+.
///
/// Legacy indexed `n_digit[n_exp+1]` also for `n_exp < -1` (numbers below
/// 0.1), reading memory before the array (undefined). Those numbers have no
/// digit in the first decimal place, so no rounding happens here.
fn round_increment(n: &NumRepr) -> bool {
    if n.digits == 0 {
        return false;
    }
    if i64::from(n.digits) > n.exp + 1 {
        let Ok(i) = usize::try_from(n.exp + 1) else { return false };
        return n.digit[i] >= b'5';
    }
    false
}

/// True when `n` hits the undefined legacy read described at
/// [`round_increment`] (used by the differential test to skip such cases).
pub fn round_reads_before_digits(n: &NumRepr) -> bool {
    n.digits != 0 && i64::from(n.digits) > n.exp + 1 && n.exp + 1 < 0
}

/// Compares the leading digits against a range limit. `Some(true)` =
/// overflow, `Some(false)` = below, `None` = equal over the scanned prefix.
fn cmp_prefix(digits: &[u8], max: &[u8], scan: usize) -> bool {
    for i in 0..scan.min(digits.len()) {
        if digits[i] > max[i] {
            return true;
        }
        if digits[i] < max[i] {
            break;
        }
    }
    false
}

/// `n_float`: `None` on overflow.
pub fn n_float(n: &NumRepr) -> Option<NumValue> {
    if n.digits == 0 {
        return Some(NumValue::Int(0));
    }
    if n.hex != 0 {
        return radix_value(n).map(|v| NumValue::Int(v as i64));
    }
    let digits = &n.digit[..n.digits as usize];
    if n.exp > 308 {
        return None;
    }
    if n.exp == 308 && cmp_prefix(digits, RANGE_FLOAT_MAX, 17) {
        return None;
    }
    if n.exp < -324 {
        return Some(NumValue::Int(0));
    }
    let mut built = Vec::with_capacity(48);
    if n.neg != 0 {
        built.push(b'-');
    }
    built.push(digits[0]);
    built.push(b'.');
    if digits.len() == 1 {
        built.push(b'0');
    } else {
        built.extend_from_slice(&digits[1..digits.len().min(17)]);
    }
    built.push(b'E');
    built.extend_from_slice(n.exp.to_string().as_bytes());
    // Digits are ASCII 0-9 for decimal numbers, so this always parses.
    let s = std::str::from_utf8(&built).unwrap_or("0");
    Some(NumValue::Float(s.parse::<f64>().unwrap_or(0.0)))
}

/// Integral part of a decimal repr (up to `n_exp+1` digits, wrapping).
fn integral(n: &NumRepr) -> u64 {
    let digits = &n.digit[..n.digits as usize];
    let mut u: u64 = 0;
    let mut i: i64 = 0;
    while i < n.exp + 1 {
        u = u.wrapping_mul(10);
        if let Some(&d) = digits.get(i as usize) {
            u = u.wrapping_add(u64::from(d - b'0'));
        }
        i += 1;
    }
    u
}

/// `n_int64`: `None` on overflow.
pub fn n_int64(n: &NumRepr) -> Option<i64> {
    if n.digits == 0 {
        return Some(0);
    }
    if n.hex != 0 {
        return radix_value(n).map(|v| v as i64);
    }
    let digits = &n.digit[..n.digits as usize];
    if n.exp > 18 {
        return None;
    }
    if n.exp == 18 && cmp_prefix(digits, if n.neg != 0 { RANGE_INT64_NEG_MAX } else { RANGE_INT64_MAX }, 19) {
        return None;
    }
    let u = integral(n);
    let mut value = if n.neg != 0 { (u as i64).wrapping_neg() } else { u as i64 };
    if round_increment(n) {
        if n.neg != 0 {
            if value == i64::MIN {
                return None;
            }
            value -= 1;
        } else {
            if value == i64::MAX {
                return None;
            }
            value += 1;
        }
    }
    Some(value)
}

/// `n_uint64`: `None` on overflow (or a negative number).
pub fn n_uint64(n: &NumRepr) -> Option<u64> {
    if n.digits == 0 {
        return Some(0);
    }
    if n.hex != 0 {
        return radix_value(n);
    }
    let digits = &n.digit[..n.digits as usize];
    if n.neg != 0 && n.exp >= 0 {
        return None;
    }
    if n.exp > 19 {
        return None;
    }
    if n.exp == 19 && cmp_prefix(digits, RANGE_UINT64_MAX, 20) {
        return None;
    }
    let mut u = integral(n);
    if round_increment(n) {
        if n.neg != 0 || u == u64::MAX {
            return None;
        }
        u += 1;
    }
    Some(u)
}

/// Range check + conversion of `func_read_float` / `func_file_input_float`.
/// `hex_is_unsigned` is true for READ (signed targets accept the full
/// unsigned range of hex/oct/bin digits), false for INPUT #.
fn convert(v: NumValue, typ: i32, hex: bool, hex_is_unsigned: bool) -> Option<NumValue> {
    let bits = (typ & 511) as u32;
    if typ & ISFLOAT != 0 {
        if bits >= 64 {
            return Some(v);
        }
        if v.to_f64() > 3.402823466E+38 {
            return None;
        }
        return Some(v);
    }
    if let NumValue::Float(f) = v {
        // 9.2233720368547758E+18 in the C source (= 2^63 as a double).
        if !(-9_223_372_036_854_775_808.0..=9_223_372_036_854_775_808.0).contains(&f) {
            return None;
        }
    }
    let mut value = v.qbr();
    let (maxval, minval) = if typ & ISUNSIGNED != 0 || (hex_is_unsigned && hex) {
        (1i64.wrapping_shl(bits).wrapping_sub(1), 0)
    } else {
        let top = 1i64.wrapping_shl(bits.wrapping_sub(1));
        (top.wrapping_sub(1), top.wrapping_neg())
    };
    if value > maxval || value < minval {
        return None;
    }
    if typ & ISUNSIGNED == 0 && hex && 1i64.wrapping_shl(bits.wrapping_sub(1)) & value != 0 {
        value = (maxval ^ -1).wrapping_add(value);
    }
    Some(NumValue::Int(value))
}

/// `func_read_float`: READ into a numeric variable of type `typ`.
/// On error the offset is restored as legacy did (not for "Out of DATA").
pub fn read_float(data: &[u8], offset: &mut usize, typ: i32) -> Result<NumValue, i32> {
    let old = *offset;
    let mut n = NumRepr::default();
    match number_from_data(data, offset, &mut n) {
        2 => return Err(4),
        3 => {
            *offset = old;
            return Err(2);
        }
        1 => {
            *offset = old;
            return Err(6);
        }
        _ => {}
    }
    match n_float(&n).and_then(|v| convert(v, typ, n.hex != 0, true)) {
        Some(v) => Ok(v),
        None => {
            *offset = old;
            Err(6)
        }
    }
}

fn read_with<T>(data: &[u8], offset: &mut usize, f: fn(&NumRepr) -> Option<T>) -> Result<T, i32> {
    let old = *offset;
    let mut n = NumRepr::default();
    match number_from_data(data, offset, &mut n) {
        2 => return Err(4),
        3 => {
            *offset = old;
            return Err(2);
        }
        1 => {
            *offset = old;
            return Err(6);
        }
        _ => {}
    }
    f(&n).ok_or_else(|| {
        *offset = old;
        6
    })
}

/// `func_read_int64`.
pub fn read_int64(data: &[u8], offset: &mut usize) -> Result<i64, i32> {
    read_with(data, offset, n_int64)
}

/// `func_read_uint64`.
pub fn read_uint64(data: &[u8], offset: &mut usize) -> Result<u64, i32> {
    read_with(data, offset, n_uint64)
}

/// `sub_read_string`: READ into a string variable. The string is always
/// assigned (empty on error); the error is 4 ("Out of DATA"), also for the
/// malformed `"abc"x` form.
pub fn read_string(data: &[u8], offset: &mut usize) -> (Vec<u8>, i32) {
    let size = data.len();
    let mut s = Vec::new();
    if *offset >= size {
        return (s, 4);
    }
    let mut inspeechmarks = 0;
    let mut c = data[*offset];
    loop {
        if c == b',' && inspeechmarks != 1 {
            *offset += 1;
            return (s, 0);
        }
        if inspeechmarks == 2 {
            return (Vec::new(), 4);
        }
        let mut skip = false;
        if c == b'"' {
            if inspeechmarks != 0 {
                inspeechmarks = 2;
                skip = true;
            } else if s.is_empty() {
                inspeechmarks = 1;
                skip = true;
            }
        }
        if !skip {
            s.push(c);
        }
        *offset += 1;
        if *offset >= size {
            return (s, 0);
        }
        c = data[*offset];
    }
}

/// `func_file_input_float` after a successful read: conversion to `typ`.
pub fn input_float(n: &NumRepr, typ: i32) -> Result<NumValue, i32> {
    n_float(n).and_then(|v| convert(v, typ, n.hex != 0, false)).ok_or(6)
}

#[cfg(test)]
mod tests {
    use super::*;

    const INT16: i32 = 16;
    const INT32: i32 = 32;
    const UINT8: i32 = 8 | ISUNSIGNED;
    const SINGLE: i32 = 32 | ISFLOAT;
    const DOUBLE: i32 = 64 | ISFLOAT;

    fn rf(d: &str, typ: i32) -> (Result<NumValue, i32>, usize) {
        let mut o = 0;
        let r = read_float(d.as_bytes(), &mut o, typ);
        (r, o)
    }

    #[test]
    fn parses_decimal_items_and_advances_past_comma() {
        let d = b"12,-3.5E2,.25";
        let mut o = 0;
        assert_eq!(read_float(d, &mut o, INT16), Ok(NumValue::Int(12)));
        assert_eq!(o, 3);
        assert_eq!(read_float(d, &mut o, DOUBLE), Ok(NumValue::Float(-350.0)));
        assert_eq!(read_float(d, &mut o, SINGLE), Ok(NumValue::Float(0.25)));
        assert_eq!(o, d.len());
        assert_eq!(read_float(d, &mut o, SINGLE), Err(4));
    }

    #[test]
    fn integer_targets_round_half_even_and_check_range() {
        assert_eq!(rf("2.5", INT16).0, Ok(NumValue::Int(2)));
        assert_eq!(rf("3.5", INT16).0, Ok(NumValue::Int(4)));
        assert_eq!(rf("32767", INT16).0, Ok(NumValue::Int(32767)));
        assert_eq!(rf("32768", INT16), (Err(6), 0));
        assert_eq!(rf("256", UINT8).0, Err(6));
        assert_eq!(rf("-1", UINT8).0, Err(6));
        assert_eq!(rf("4E38", SINGLE).0, Err(6));
    }

    #[test]
    fn hex_into_signed_target_sign_extends() {
        assert_eq!(rf("&HFFFF", INT16).0, Ok(NumValue::Int(-1)));
        assert_eq!(rf("&H8000", INT16).0, Ok(NumValue::Int(-32768)));
        assert_eq!(rf("&H10000", INT16).0, Err(6));
        assert_eq!(rf("&O777", INT32).0, Ok(NumValue::Int(511)));
        assert_eq!(rf("&B101%", INT32).0, Ok(NumValue::Int(5)));
        assert_eq!(rf("&HFFFFFFFFFFFFFFFF", 64 | ISFLOAT).0, Ok(NumValue::Int(-1)));
    }

    #[test]
    fn syntax_errors_restore_offset() {
        assert_eq!(rf("1x", INT16), (Err(2), 0));
        assert_eq!(rf("&", INT16), (Err(2), 0));
        assert_eq!(rf("&Q1", INT16), (Err(2), 0));
        assert_eq!(rf("1%2", INT16), (Err(2), 0));
        assert_eq!(rf("1.2.3", INT16), (Err(2), 0));
        assert_eq!(rf("1E2E3", INT16), (Err(2), 0));
    }

    #[test]
    fn empty_item_is_zero() {
        let d = b",5";
        let mut o = 0;
        assert_eq!(read_float(d, &mut o, INT16), Ok(NumValue::Int(0)));
        assert_eq!(o, 1);
    }

    #[test]
    fn int64_and_uint64_limits() {
        let mut o = 0;
        assert_eq!(read_int64(b"9223372036854775807", &mut o), Ok(i64::MAX));
        o = 0;
        assert_eq!(read_int64(b"-9223372036854775808", &mut o), Ok(i64::MIN));
        o = 0;
        assert_eq!(read_int64(b"9223372036854775808", &mut o), Err(6));
        o = 0;
        assert_eq!(read_int64(b"9223372036854775806.5", &mut o), Ok(i64::MAX));
        o = 0;
        assert_eq!(read_uint64(b"18446744073709551615", &mut o), Ok(u64::MAX));
        o = 0;
        assert_eq!(read_uint64(b"-1", &mut o), Err(6));
        o = 0;
        assert_eq!(read_uint64(b"-0.4", &mut o), Ok(0));
        o = 0;
        assert_eq!(read_int64(b"0.001", &mut o), Ok(0));
    }

    #[test]
    fn strings_quotes_and_errors() {
        let d = br#"abc,"x,y",  sp ,"q"r"#;
        let mut o = 0;
        assert_eq!(read_string(d, &mut o), (b"abc".to_vec(), 0));
        assert_eq!(read_string(d, &mut o), (b"x,y".to_vec(), 0));
        assert_eq!(read_string(d, &mut o), (b"  sp ".to_vec(), 0));
        assert_eq!(read_string(d, &mut o), (Vec::new(), 4));
        let mut o = 3;
        assert_eq!(read_string(b"abc", &mut o), (Vec::new(), 4));
    }

    #[test]
    fn file_input_conversion_does_not_treat_hex_as_unsigned() {
        let mut n = NumRepr::default();
        let mut o = 0;
        assert_eq!(number_from_data(b"&HFFFF", &mut o, &mut n), 0);
        assert_eq!(input_float(&n, INT16), Err(6));
        assert_eq!(input_float(&n, INT32), Ok(NumValue::Int(65535)));
    }

    #[test]
    fn float_extremes() {
        assert_eq!(rf("1.7976931348623157E308", DOUBLE).0, Ok(NumValue::Float(f64::MAX)));
        assert_eq!(rf("1.7976931348623158E308", DOUBLE).0, Err(6));
        assert_eq!(rf("1E-400", DOUBLE).0, Ok(NumValue::Int(0)));
    }
}
