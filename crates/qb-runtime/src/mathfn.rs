//! Math functions with BASIC error checks and `HEX$`/`OCT$`/`_BIN$` (R12b).
//!
//! Replaces `func_log`, `func_exp_single`, `func_sqr`, `func_fix_double`,
//! `func_hex`, `func_oct` and `func__bin` from libqb.cpp. The `long double`
//! variants (`func_fix_float`, `func_exp_float`) and the float-to-integer
//! front ends of `HEX$`/`OCT$`/`_BIN$` (x87 rounding of a `long double`)
//! stay in the libqb.cpp shims until `_FLOAT` is emulated in Rust (D12).

/// `LOG`: `Err(5)` for x <= 0.
pub fn log(x: f64) -> Result<f64, i32> {
    if x <= 0.0 {
        Err(5)
    } else {
        Ok(x.ln())
    }
}

/// `EXP` for SINGLE/DOUBLE arguments: `Err(6)` above 88.02969.
pub fn exp_single(x: f64) -> Result<f64, i32> {
    if x <= 88.02969 {
        Ok(x.exp())
    } else {
        Err(6)
    }
}

/// `SQR`: `Err(5)` for x < 0.
pub fn sqr(x: f64) -> Result<f64, i32> {
    if x < 0.0 {
        Err(5)
    } else {
        Ok(x.sqrt())
    }
}

/// `FIX` (truncate toward zero; NaN stays NaN).
pub fn fix(x: f64) -> f64 {
    if x.is_nan() {
        return x; // C floor() returns a NaN unchanged (also a signalling one)
    }
    if x < 0.0 {
        x.ceil()
    } else {
        x.floor()
    }
}

/// Number of significant bits of a negative value's two's complement
/// representation, widened to `neg_bits` or the next 16/32/64 boundary.
fn neg_width(value: i64, neg_bits: i32) -> i32 {
    let mut i2 = 0;
    let mut v = value;
    for i in 1..=64 {
        if v & 1 == 0 {
            i2 = i;
        }
        v >>= 1;
    }
    if i2 >= neg_bits {
        if i2 < 16 {
            16
        } else if i2 < 32 {
            32
        } else {
            64
        }
    } else {
        neg_bits
    }
}

fn pos_width(value: i64) -> i32 {
    let mut i2 = 0;
    let mut v = value;
    for i in 1..=64 {
        if v & 1 != 0 {
            i2 = i;
        }
        v >>= 1;
    }
    i2
}

/// `_BIN$` (`func__bin`).
pub fn bin(value: i64, neg_bits: i32) -> Vec<u8> {
    let bits = if value < 0 { neg_width(value, neg_bits) } else { pos_width(value) };
    if bits == 0 {
        return b"0".to_vec();
    }
    let mut v = value;
    let mut out = vec![0u8; bits as usize];
    for c in out.iter_mut().rev() {
        *c = (v & 1) as u8 + b'0';
        v >>= 1;
    }
    out
}

/// `OCT$` (`func_oct`).
pub fn oct(value: i64, neg_bits: i32) -> Vec<u8> {
    let bits = if value < 0 { neg_width(value, neg_bits) } else { pos_width(value) };
    if bits == 0 {
        return b"0".to_vec();
    }
    let n = ((bits + 2) / 3) as usize;
    let mut out = vec![0u8; n];
    let mut idx = n;
    let mut v = value;
    let (mut x, mut x2) = (0, 0u8);
    for _ in 0..bits {
        if v & 1 != 0 {
            x2 |= 1 << x;
        }
        v >>= 1;
        x += 1;
        if x == 3 {
            idx -= 1;
            out[idx] = x2 + b'0';
            x2 = 0;
            x = 0;
        }
    }
    if x != 0 {
        out[idx - 1] = x2 + b'0';
    }
    out
}

/// `HEX$` (`func_hex`).
pub fn hex(value: i64, neg_size: i32) -> Vec<u8> {
    let mut s = [0u8; 16];
    let mut v = value;
    let (mut i2, mut i3) = (0, 0);
    for i in 1..=16 {
        if v & 15 != 0 {
            i2 = i;
        }
        if v & 15 != 15 {
            i3 = i;
            if v & 8 == 0 && i != 16 {
                i3 += 1;
            }
        }
        let x = (v & 15) as u8;
        s[16 - i as usize] = if x > 9 { x + 55 } else { x + 48 };
        v >>= 4;
    }
    if i2 == 0 {
        return b"0".to_vec();
    }
    if value < 0 {
        i2 = if i3 <= neg_size {
            neg_size
        } else if i3 <= 4 {
            4
        } else if i3 <= 8 {
            8
        } else {
            16
        };
    }
    // Callers pass 4, 8 or 16; a wider minimum read before the legacy buffer.
    s[(16 - i2.min(16)) as usize..].to_vec()
}

// ---------------------------------------------------------------- C ABI

/// `func_log`; returns the BASIC error (0 = ok, result in `*out`).
///
/// # Safety
/// `out` valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_log(x: f64, out: *mut f64) -> i32 {
    put(log(x), out)
}

/// `func_exp_single`; returns the BASIC error.
///
/// # Safety
/// `out` valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_exp_single(x: f64, out: *mut f64) -> i32 {
    put(exp_single(x), out)
}

/// `func_sqr`; returns the BASIC error.
///
/// # Safety
/// `out` valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_sqr(x: f64, out: *mut f64) -> i32 {
    put(sqr(x), out)
}

/// `func_fix_double`.
#[unsafe(no_mangle)]
pub extern "C" fn qb_fix_double(x: f64) -> f64 {
    fix(x)
}

fn put(r: Result<f64, i32>, out: *mut f64) -> i32 {
    match r {
        Ok(v) => {
            // SAFETY: caller passes a valid pointer.
            unsafe { *out = v };
            0
        }
        Err(e) => {
            // SAFETY: as above.
            unsafe { *out = 0.0 };
            e
        }
    }
}

/// `HEX$`/`OCT$`/`_BIN$` digits (`radix` 16, 8 or 2) into `buf` (64 bytes).
/// Returns the length.
///
/// # Safety
/// `buf` writable for 64 bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_radix_string(value: i64, neg_bits: i32, radix: i32, buf: *mut u8) -> i32 {
    let s = match radix {
        16 => hex(value, neg_bits),
        8 => oct(value, neg_bits),
        _ => bin(value, neg_bits),
    };
    unsafe { core::ptr::copy_nonoverlapping(s.as_ptr(), buf, s.len()) };
    s.len() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_functions() {
        assert_eq!(log(0.0), Err(5));
        assert_eq!(log(1.0), Ok(0.0));
        assert_eq!(sqr(-1.0), Err(5));
        assert_eq!(sqr(9.0), Ok(3.0));
        assert_eq!(exp_single(88.1), Err(6));
        assert_eq!(exp_single(0.0), Ok(1.0));
        assert_eq!(fix(-2.7), -2.0);
        assert_eq!(fix(2.7), 2.0);
    }

    #[test]
    fn hex_widths() {
        assert_eq!(hex(0, 8), b"0");
        assert_eq!(hex(255, 4), b"FF");
        assert_eq!(hex(-1, 4), b"FFFF");
        assert_eq!(hex(-1, 8), b"FFFFFFFF");
        assert_eq!(hex(-32768, 4), b"8000");
        assert_eq!(hex(-32769, 4), b"FFFF7FFF");
        assert_eq!(hex(i64::MIN, 4), b"8000000000000000");
    }

    #[test]
    fn oct_and_bin_widths() {
        assert_eq!(oct(8, 16), b"10");
        assert_eq!(oct(-1, 16), b"177777");
        assert_eq!(oct(0, 16), b"0");
        assert_eq!(bin(5, 16), b"101");
        assert_eq!(bin(-1, 16), b"1111111111111111");
        assert_eq!(bin(-1, 8), b"11111111");
        assert_eq!(bin(-200, 8), b"1111111100111000");
    }
}
