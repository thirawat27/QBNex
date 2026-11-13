//! INPUT entry checks: whether the text typed for a variable is valid for its
//! type (an editor refuses characters that would make the entry invalid).

use core::ffi::c_void;

/// Type flags of `qbs_input_variabletypes` (`common.h`).
pub const ISSTRING: i32 = 1073741824;
pub const ISFLOAT: i32 = 536870912;
pub const ISUNSIGNED: i32 = 268435456;
pub const ISOFFSETINBITS: i32 = 16777216;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Check {
    Valid,
    Invalid,
}

/// `hexoct2uint64`: 0 = not &H/&O/&B, 1 = hex (also "" and "&"), 2 = octal,
/// 3 = binary. The value is only meaningful for complete numbers.
pub fn hexoct(h: &[u8]) -> (i32, u64) {
    if h.is_empty() {
        return (1, 0);
    }
    if h[0] != b'&' {
        return (0, 0);
    }
    if h.len() == 1 {
        return (1, 0);
    }
    let t = match h[1] {
        b'H' | b'h' => 1,
        b'O' | b'o' => 2,
        b'B' | b'b' => 3,
        _ => return (0, 0),
    };
    if h.len() == 2 {
        return (t, 0);
    }
    let mut r: u64 = 0;
    match t {
        1 => {
            if h.len() > 18 {
                return (0, 0);
            }
            for &c in &h[2..] {
                r <<= 4;
                let mut d = c as i32;
                if c.is_ascii_hexdigit() {
                    if d >= 97 {
                        d -= 32;
                    }
                    if d >= 65 {
                        d -= 7;
                    }
                    r = r.wrapping_add((d - 48) as u64);
                } else {
                    return (0, 0);
                }
            }
        }
        2 => {
            if h.len() > 24 || (h.len() == 24 && h[2] != b'0' && h[2] != b'1') {
                return (0, 0);
            }
            for &c in &h[2..] {
                r <<= 3;
                if (b'0'..=b'7').contains(&c) {
                    r += (c - b'0') as u64;
                } else {
                    return (0, 0);
                }
            }
        }
        _ => {
            if h.len() > 66 {
                return (0, 0);
            }
            for &c in &h[2..] {
                r <<= 1;
                if c == b'0' || c == b'1' {
                    r += (c - b'0') as u64;
                } else {
                    return (0, 0);
                }
            }
        }
    }
    (t, r)
}

/// Largest values written as digit strings (`uint64_max` etc.).
const UINT64_MAX: &[u8] = b"18446744073709551615";
const INT64_MAX: &[u8] = b"9223372036854775807";
const INT64_MAX_NEG: &[u8] = b"9223372036854775808";
const SINGLE_MAX: &[u8] = b"3402823";
const SINGLE_MAX_NEG: &[u8] = b"1401298";
const DOUBLE_MAX: &[u8] = b"17976931";
const DOUBLE_MAX_NEG: &[u8] = b"4940656";

/// x87 extended value of an unsigned 64-bit integer (exact).
fn f80_from_u64(v: u64) -> crate::round::F80 {
    let mut out = [0u8; 10];
    if v != 0 {
        let lz = v.leading_zeros();
        out[..8].copy_from_slice(&(v << lz).to_le_bytes());
        out[8..].copy_from_slice(&((16383 + 63 - lz) as u16).to_le_bytes());
    }
    out
}

/// Stores a FLOAT (`long double`) variable.
unsafe fn store_long_double(var: *mut c_void, v: crate::round::F80, d: f64) {
    if cfg!(any(target_arch = "x86", target_arch = "x86_64")) {
        core::ptr::copy_nonoverlapping(v.as_ptr(), var as *mut u8, 10);
    } else {
        // long double is double on Windows/macOS ARM (unverified elsewhere)
        (var as *mut f64).write_unaligned(d);
    }
}

/// `sscanf("%f"/"%lf")` on an already validated entry: the longest prefix that
/// is a number (a trailing "E", "E-" or "E+" is not part of it).
fn scan_float(arg: &[u8]) -> Option<f64> {
    let mut end = arg.len();
    while end > 0 {
        let s = core::str::from_utf8(&arg[..end]).ok()?;
        if let Ok(v) = s.parse::<f64>() {
            return Some(v);
        }
        end -= 1;
    }
    None
}

fn scan_f32(arg: &[u8]) -> Option<f32> {
    let mut end = arg.len();
    while end > 0 {
        let s = core::str::from_utf8(&arg[..end]).ok()?;
        if let Ok(v) = s.parse::<f32>() {
            return Some(v);
        }
        end -= 1;
    }
    None
}

/// Validates the entry `arg` of a variable of type `t` and, when `var` is not
/// null, stores the value (the legacy editor did both in one pass, also for
/// incomplete entries). `max` is the "largest value" variable that legacy
/// carried between arguments (see docs/dev/deviations.md).
pub unsafe fn check_and_store(arg: &mut Vec<u8>, t: i32, var: *mut c_void, max: &mut u64) -> Check {
    let l = arg.len();
    let at = |a: &Vec<u8>, i: usize| -> u8 { a.get(i).copied().unwrap_or(0) };
    let bits = t & 511;
    let store = !var.is_null();

    if t & ISSTRING == 0 && t & ISFLOAT == 0 && bits <= 32 {
        let mut finalvalue: i64 = 0;
        if l != 0 {
            *max = (1u64 << bits).wrapping_sub(1);
            if bits == 0 {
                *max = 0;
            }
            let (kind, hexvalue) = hexoct(arg);
            if kind != 0 {
                if hexvalue > *max {
                    return Check::Invalid;
                }
                let (mask, shift, limit) = match kind {
                    1 => (0xF, 4, 16),
                    2 => (0x7, 3, 22),
                    _ => (0x1, 1, 64),
                };
                let mut value = *max;
                let mut digits = 0;
                for k in 1..=limit {
                    if value & mask != 0 {
                        digits = k;
                    }
                    value >>= shift;
                }
                if l > 2 + digits {
                    return Check::Invalid;
                }
                finalvalue = hexvalue as i64;
            } else {
                let max_neg = if t & ISUNSIGNED != 0 {
                    0
                } else {
                    *max >>= 1;
                    *max + 1
                };
                let mut i2 = 0;
                let mut neg = false;
                let mut done = false;
                if t & ISUNSIGNED == 0 && at(arg, 0) == b'-' {
                    if l == 1 {
                        done = true;
                    }
                    i2 = 1;
                    neg = true;
                }
                if !done {
                    if at(arg, i2) == b'0' && l > i2 + 1 {
                        return Check::Invalid;
                    }
                    let mut multiple: u64 = 1;
                    let mut value: u64 = 0;
                    for i in (i2..l).rev() {
                        let d = arg[i] as i32 - 48;
                        if !(0..=9).contains(&d) {
                            return Check::Invalid;
                        }
                        value = value.wrapping_add(multiple.wrapping_mul(d as u64));
                        let limit = if t & ISUNSIGNED == 0 && neg { max_neg } else { *max };
                        if value > limit {
                            return Check::Invalid;
                        }
                        multiple = multiple.wrapping_mul(10);
                    }
                    finalvalue = if neg { (value as i64).wrapping_neg() } else { value as i64 };
                } else {
                    finalvalue = 0;
                }
            }
        }
        if store && t & ISOFFSETINBITS == 0 {
            let n = (bits >> 3) as usize;
            core::ptr::copy_nonoverlapping(finalvalue.to_le_bytes().as_ptr(), var as *mut u8, n.min(8));
        }
        return Check::Valid;
    }

    if t & ISSTRING != 0 {
        // any text is a valid string entry; string variables are not stored here
        return Check::Valid;
    }

    if t & ISFLOAT == 0 && bits == 64 {
        if l == 0 {
            if store {
                (var as *mut i64).write_unaligned(0);
            }
            return Check::Valid;
        }
        let (kind, hexvalue) = hexoct(arg);
        if kind != 0 {
            if hexvalue > *max {
                return Check::Invalid;
            }
            if store {
                (var as *mut u64).write_unaligned(hexvalue);
            }
            return Check::Valid;
        }
        let mut i2 = 0;
        let mut neg = false;
        if t & ISUNSIGNED == 0 && arg[0] == b'-' {
            if l == 1 {
                if store {
                    (var as *mut i64).write_unaligned(0);
                }
                return Check::Valid;
            }
            i2 = 1;
            neg = true;
        }
        if at(arg, i2) == b'0' && l > i2 + 1 {
            return Check::Invalid;
        }
        if arg[i2..].iter().any(|c| !c.is_ascii_digit()) {
            return Check::Invalid;
        }
        let digits = l - i2;
        let (need, limit) = if t & ISUNSIGNED != 0 {
            (20, UINT64_MAX)
        } else {
            (19, if neg { INT64_MAX_NEG } else { INT64_MAX })
        };
        if digits > need {
            return Check::Invalid;
        }
        if digits == need {
            for i in 0..digits {
                if arg[i + i2] < limit[i] {
                    break;
                }
                if arg[i + i2] > limit[i] {
                    return Check::Invalid;
                }
            }
        }
        if store {
            let s = core::str::from_utf8(arg).unwrap_or("0");
            if t & ISUNSIGNED != 0 {
                (var as *mut u64).write_unaligned(s.parse::<u64>().unwrap_or(u64::MAX));
            } else {
                // "%I64i": -9223372036854775808 parses exactly
                (var as *mut i64).write_unaligned(s.parse::<i64>().unwrap_or(i64::MAX));
            }
        }
        return Check::Valid;
    }

    if t & ISFLOAT != 0 {
        if store {
            match bits {
                32 => (var as *mut f32).write_unaligned(0.0),
                64 => (var as *mut f64).write_unaligned(0.0),
                256 => store_long_double(var, [0; 10], 0.0),
                _ => {}
            }
        }
        if l == 0 {
            return Check::Valid;
        }
        let (kind, mut hexvalue) = hexoct(arg);
        if kind != 0 {
            if store {
                const TRANSFER: u64 = 0x7FFF_FFFF_FFFF_FFFF;
                let mut f = 0f32;
                let mut d = 0f64;
                let total = hexvalue;
                while hexvalue > TRANSFER {
                    hexvalue -= TRANSFER;
                    f += TRANSFER as i64 as f32;
                    d += TRANSFER as i64 as f64;
                }
                f += hexvalue as i64 as f32;
                d += hexvalue as i64 as f64;
                match bits {
                    32 => (var as *mut f32).write_unaligned(f),
                    64 => (var as *mut f64).write_unaligned(d),
                    256 => store_long_double(var, f80_from_u64(total), d),
                    _ => {}
                }
            }
            return Check::Valid;
        }
        let mut digits_before = 0i32;
        let mut digits_after = 0i32;
        let mut zeros_after = 0i32;
        let mut neg_power = false;
        let mut value: u64 = 0;
        let mut sig: Vec<u8> = Vec::new();
        let mut i2 = 0usize;
        if arg[0] == b'-' {
            if l == 1 {
                return Check::Valid;
            }
            i2 = 1;
        }
        // states: 0 = assess, 1 = decimal point at i2, 2 = exponent letter at i2
        let mut state;
        if arg[i2] == b'0' {
            if l > i2 + 1 {
                i2 += 1;
                if arg[i2] != b'.' {
                    return Check::Invalid;
                }
                state = 1;
            } else {
                return Check::Valid; // "0" / "-0": stays 0
            }
        } else {
            state = 0;
            let mut i = i2;
            while i < l {
                let c = arg[i];
                if matches!(c, b'd' | b'D' | b'e' | b'E') {
                    if i == i2 {
                        return Check::Invalid;
                    }
                    i2 = i;
                    state = 2;
                    break;
                }
                if c == b'.' {
                    i2 = i;
                    state = 1;
                    break;
                }
                if !c.is_ascii_digit() {
                    return Check::Invalid;
                }
                digits_before += 1;
                sig.push(c);
                i += 1;
            }
        }
        if state == 1 {
            let mut leading = true;
            if i2 == l - 1 {
                state = 0;
            } else {
                i2 += 1;
                state = 0;
                let mut i = i2;
                while i < l {
                    let c = arg[i];
                    if matches!(c, b'd' | b'D' | b'e' | b'E') && !sig.is_empty() {
                        if i == i2 {
                            return Check::Invalid;
                        }
                        i2 = i;
                        state = 2;
                        break;
                    }
                    if !c.is_ascii_digit() {
                        return Check::Invalid;
                    }
                    let d = c - b'0';
                    if d != 0 {
                        leading = false;
                    }
                    if leading {
                        zeros_after += 1;
                    }
                    digits_after += 1;
                    if !sig.is_empty() || d != 0 {
                        sig.push(c);
                    }
                    i += 1;
                }
            }
        }
        if state == 2 {
            if bits == 32 && matches!(arg[i2], b'd' | b'D') {
                return Check::Invalid;
            }
            if matches!(arg[i2], b'd' | b'D') {
                arg[i2] = b'E';
            }
            if i2 != l - 1 {
                i2 += 1;
                let c = arg[i2];
                let mut incomplete = false;
                if c == b'-' {
                    if i2 == l - 1 {
                        incomplete = true;
                    } else {
                        neg_power = true;
                        i2 += 1;
                    }
                }
                if c == b'+' {
                    if i2 == l - 1 {
                        incomplete = true;
                    } else {
                        i2 += 1;
                    }
                }
                if !incomplete {
                    if arg[i2] == b'0' && l > i2 + 1 {
                        return Check::Invalid;
                    }
                    let mut multiple: u64 = 1;
                    value = 0;
                    for i in (i2..l).rev() {
                        let d = arg[i] as i32 - 48;
                        if !(0..=9).contains(&d) {
                            return Check::Invalid;
                        }
                        value = value.wrapping_add(multiple.wrapping_mul(d as u64));
                        multiple = multiple.wrapping_mul(10);
                    }
                }
            }
        }
        // assess_float
        let (zero_limit, max_pos, max_digits, max_prec, min_pos, min_digits, max_s, min_s) = match bits {
            32 => (43, 39, 7, 8, -44, 7, SINGLE_MAX, SINGLE_MAX_NEG),
            64 => (322, 309, 8, 16, -323, 7, DOUBLE_MAX, DOUBLE_MAX_NEG),
            _ => {
                if store && bits == 256 {
                    // legacy scanned into a function-static double and always stored it,
                    // so an entry sscanf cannot read ("." or "-.") repeats the previous value
                    static mut SSCANF_FIX: f64 = 0.0;
                    if let Some(d) = scan_float(arg) {
                        SSCANF_FIX = d;
                    }
                    let d = SSCANF_FIX;
                    store_long_double(var, crate::round::f80_from_f64(d), d);
                }
                return Check::Valid;
            }
        };
        if neg_power {
            value = value.wrapping_neg();
        }
        if zeros_after == 1 && digits_after == 1 {
            digits_after = 0;
            zeros_after = 0;
        }
        if zeros_after > zero_limit {
            return Check::Invalid;
        }
        if !(digits_before == 0 && digits_after == zeros_after) {
            // legacy: int32 i = i + (uint64)value  (wraps modulo 2^32)
            let mut i: i32 = digits_before;
            if i == 0 {
                i = -zeros_after;
            }
            let i = (i as i64 as u64).wrapping_add(value) as u32 as i32;
            if i > max_pos {
                return Check::Invalid;
            }
            if i == max_pos {
                for (k, &d) in sig.iter().take(max_digits).enumerate() {
                    if d > max_s[k] {
                        return Check::Invalid;
                    }
                    if d < max_s[k] {
                        break;
                    }
                }
            }
            if digits_after != 0 {
                let n = if digits_before != 0 { digits_after + digits_before } else { digits_after - zeros_after };
                if n > max_prec {
                    return Check::Invalid;
                }
            }
            if i < min_pos {
                return Check::Invalid;
            }
            if i == min_pos {
                for (k, &d) in sig.iter().take(min_digits).enumerate() {
                    if d < min_s[k] {
                        return Check::Invalid;
                    }
                    if d > min_s[k] {
                        break;
                    }
                }
            }
        }
        if store {
            if bits == 32 {
                if let Some(v) = scan_f32(arg) {
                    (var as *mut f32).write_unaligned(v);
                }
            } else if let Some(v) = scan_float(arg) {
                (var as *mut f64).write_unaligned(v);
            }
        }
        return Check::Valid;
    }
    // other types fall through as valid
    Check::Valid
}

/// Validation only (no variable is written).
pub fn validate(arg: &[u8], t: i32) -> Check {
    let mut a = arg.to_vec();
    let mut max = u64::MAX;
    // SAFETY: a null variable is never dereferenced.
    unsafe { check_and_store(&mut a, t, core::ptr::null_mut(), &mut max) }
}
