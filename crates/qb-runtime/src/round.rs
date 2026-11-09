//! QBasic-compatible float -> integer rounding.
//!
//! Replaces `qbr`, `qbr_longdouble_to_uint64`, `qbr_float_to_long` and
//! `qbr_double_to_long` from libqb.cpp, which used x87 `fistp` with the FPU in
//! round-to-nearest-even mode (control word 0x37F, all exceptions masked).
//! `fistp` stores the "integer indefinite" value (the most negative integer)
//! for NaN, infinities and out-of-range results.

/// Raw bytes of an x87 80-bit extended value, little-endian, as stored in a
/// C `long double` on x86 (8-byte significand with explicit integer bit,
/// then 2 bytes of sign + 15-bit exponent).
pub type F80 = [u8; 10];

/// Exact `double` -> x87 extended conversion, for platforms whose C
/// `long double` is not the 80-bit format (e.g. AArch64).
pub fn f80_from_f64(v: f64) -> F80 {
    let bits = v.to_bits();
    let sign = ((bits >> 63) as u16) << 15;
    let exp = ((bits >> 52) & 0x7FF) as i32;
    let frac = bits & ((1u64 << 52) - 1);
    let (exp80, mant) = match (exp, frac) {
        (0, 0) => (0u16, 0u64),
        (0x7FF, _) => (0x7FFF, (1u64 << 63) | (frac << 11)),
        (0, _) => {
            // Subnormal double: value = frac * 2^-1074, normal as f80.
            let lz = frac.leading_zeros() as i32;
            ((16383 - 1074 + 63 - lz) as u16, frac << lz)
        }
        _ => ((exp - 1023 + 16383) as u16, (1u64 << 63) | (frac << 11)),
    };
    let mut out = [0u8; 10];
    out[..8].copy_from_slice(&mant.to_le_bytes());
    out[8..].copy_from_slice(&(sign | exp80).to_le_bytes());
    out
}

/// What `fistp` stores for NaN, infinities and out-of-range values.
const INDEFINITE_I64: i64 = i64::MIN;
const INDEFINITE_I32: i32 = i32::MIN;

/// Decoded finite f80: value = (-1)^neg * mant * 2^exp2.
struct Finite {
    neg: bool,
    mant: u64,
    exp2: i32,
}

/// `None` for operands `fistp` rejects as invalid: NaN, infinities and
/// unnormals (non-zero exponent with the explicit integer bit clear).
fn decode(f: F80) -> Option<Finite> {
    let mut m = [0u8; 8];
    m.copy_from_slice(&f[..8]);
    let mant = u64::from_le_bytes(m);
    let se = u16::from_le_bytes([f[8], f[9]]);
    let neg = se & 0x8000 != 0;
    let exp = i32::from(se & 0x7FFF);
    if exp == 0x7FFF || (exp != 0 && mant >> 63 == 0) {
        return None;
    }
    // Biased exponent 0 (denormal) uses the same scale as exponent 1.
    let exp2 = exp.max(1) - 16383 - 63;
    Some(Finite { neg, mant, exp2 })
}

/// Rounds `mant * 2^exp2` to the nearest integer, ties to even.
/// `None` when the magnitude does not fit in a u128.
fn round_magnitude(mant: u64, exp2: i32) -> Option<u128> {
    if exp2 >= 0 {
        return if exp2 > 64 { None } else { Some(u128::from(mant) << exp2) };
    }
    let shift = -exp2;
    if shift > 64 {
        return Some(0); // magnitude < 0.5
    }
    let m = u128::from(mant);
    let q = m >> shift;
    let rem = m & ((1u128 << shift) - 1);
    let half = 1u128 << (shift - 1);
    Some(if rem > half || (rem == half && q & 1 == 1) { q + 1 } else { q })
}

/// `fistp` to a 64-bit integer.
pub fn fistp_i64(f: F80) -> i64 {
    let Some(v) = decode(f) else { return INDEFINITE_I64 };
    let Some(mag) = round_magnitude(v.mant, v.exp2) else { return INDEFINITE_I64 };
    if v.neg {
        if mag > 1u128 << 63 {
            INDEFINITE_I64
        } else {
            (mag as i128).wrapping_neg() as i64
        }
    } else if mag > i64::MAX as u128 {
        INDEFINITE_I64
    } else {
        mag as i64
    }
}

/// libqb `qbr(long double)`: like `fistp_i64`, but values above `i64::MAX`
/// are reduced by 2^63 first and returned with the top bit set, so callers
/// storing into `_UNSIGNED _INTEGER64` get the right bit pattern.
pub fn qbr(f: F80) -> i64 {
    // NaN/inf: fistp of either the value or (inf - 2^63) is indefinite.
    let Some(v) = decode(f) else { return INDEFINITE_I64 };
    // Only values with exp2 >= -1 can exceed i64::MAX (mant < 2^64).
    if v.neg || v.exp2 < -1 {
        return fistp_i64(f);
    }
    if v.exp2 + 1 > 64 {
        // value >= 2^128 / 2: subtracting 2^63 leaves it far out of range.
        return INDEFINITE_I64;
    }
    // Work in halves so that every relevant value is an exact integer.
    let twice = i128::try_from(u128::from(v.mant) << (v.exp2 + 1)).unwrap_or(i128::MAX);
    let twice_max = 2 * i128::from(i64::MAX);
    if twice <= twice_max {
        return fistp_i64(f);
    }
    // (value - 2^63) * 2; exact, as in x87 extended precision for every value
    // where the result can still fit in an int64.
    let d2 = twice - (1i128 << 64);
    let q = d2.div_euclid(2);
    let r = d2.rem_euclid(2);
    let rounded = if r == 1 && q & 1 == 1 { q + 1 } else { q };
    if rounded > i128::from(i64::MAX) || rounded < i128::from(i64::MIN) {
        return INDEFINITE_I64;
    }
    (rounded as i64) | i64::MIN
}

/// `fistp` to a 32-bit integer from a float or double (both widen exactly).
pub fn fistp_i32(f: f64) -> i32 {
    if f.is_nan() {
        return INDEFINITE_I32;
    }
    let r = f.round_ties_even();
    if !(-2147483648.0..=2147483647.0).contains(&r) {
        INDEFINITE_I32
    } else {
        r as i32
    }
}

// --- C ABI (declared in include/qb_runtime.h; called by the qbr* shims in libqb.cpp) ---

/// Reads a possibly unaligned 10-byte x87 value.
///
/// # Safety
/// `p` must be readable for 10 bytes.
unsafe fn read_f80(p: *const u8) -> F80 {
    let mut b = [0u8; 10];
    // SAFETY: guaranteed by the caller; byte copy has no alignment requirement.
    unsafe { core::ptr::copy_nonoverlapping(p, b.as_mut_ptr(), 10) };
    b
}

/// # Safety
/// `f80` must be readable for 10 bytes (an x87 `long double`).
#[no_mangle]
pub unsafe extern "C" fn qb_qbr_f80(f80: *const u8) -> i64 {
    // SAFETY: forwarded caller contract.
    qbr(unsafe { read_f80(f80) })
}

/// # Safety
/// `f80` must be readable for 10 bytes (an x87 `long double`).
#[no_mangle]
pub unsafe extern "C" fn qb_fistp_i64_f80(f80: *const u8) -> i64 {
    // SAFETY: forwarded caller contract.
    fistp_i64(unsafe { read_f80(f80) })
}

/// `qbr` for platforms whose `long double` is not x87 extended.
#[no_mangle]
pub extern "C" fn qb_qbr_f64(v: f64) -> i64 {
    qbr(f80_from_f64(v))
}

/// `fistp` to int64 for platforms whose `long double` is not x87 extended.
#[no_mangle]
pub extern "C" fn qb_fistp_i64_f64(v: f64) -> i64 {
    fistp_i64(f80_from_f64(v))
}

#[no_mangle]
pub extern "C" fn qb_fistp_i32(v: f64) -> i32 {
    fistp_i32(v)
}

/// Puts the x87 FPU into the mode QBasic arithmetic expects (was
/// `fpu_reinit` in libqb.cpp). No-op on other architectures.
#[no_mangle]
pub extern "C" fn qb_fpu_reinit() {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        // Extended precision, round to nearest even, all exceptions masked.
        let mode: u16 = 0x037F;
        // SAFETY: fldcw only reads the control word from the given address.
        unsafe { core::arch::asm!("fldcw [{}]", in(reg) &mode, options(nostack, readonly)) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exact f64 -> f80 conversion (every f64 is representable as f80).
    fn f80(v: f64) -> F80 {
        let bits = v.to_bits();
        let sign = (bits >> 63) as u16;
        let exp = ((bits >> 52) & 0x7FF) as i32;
        let frac = bits & ((1u64 << 52) - 1);
        let (e80, mant): (u16, u64) = if exp == 0x7FF {
            (0x7FFF, (1u64 << 63) | (frac << 11))
        } else if exp == 0 && frac == 0 {
            (0, 0)
        } else if exp == 0 {
            // f64 subnormal: normalise.
            let shift = frac.leading_zeros();
            let mant = frac << shift;
            ((-1022 - (shift as i32 - 11) + 16383) as u16, mant)
        } else {
            ((exp - 1023 + 16383) as u16, (1u64 << 63) | (frac << 11))
        };
        raw(sign, e80, mant)
    }

    fn raw(sign: u16, exp: u16, mant: u64) -> F80 {
        let mut b = [0u8; 10];
        b[..8].copy_from_slice(&mant.to_le_bytes());
        b[8..].copy_from_slice(&((sign << 15) | exp).to_le_bytes());
        b
    }

    /// Exact non-negative integer as f80 (needs up to 64 significant bits).
    fn int80(neg: bool, v: u64) -> F80 {
        assert!(v != 0);
        let lz = v.leading_zeros();
        raw(neg as u16, (16383 + 63 - lz) as u16, v << lz)
    }

    const INDEFINITE: i64 = i64::MIN;

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    fn read_x87_control_word() -> u16 {
        let mut cw: u16 = 0;
        // SAFETY: fnstcw only stores the control word to the given address.
        unsafe { core::arch::asm!("fnstcw [{}]", in(reg) &mut cw, options(nostack)) };
        cw
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[test]
    fn fpu_reinit_sets_extended_precision_round_nearest() {
        let other: u16 = 0x027F; // 53-bit precision, as some Windows CRTs set it
        // SAFETY: loads a valid x87 control word.
        unsafe { core::arch::asm!("fldcw [{}]", in(reg) &other, options(nostack)) };
        assert_eq!(read_x87_control_word(), 0x027F);
        qb_fpu_reinit();
        assert_eq!(read_x87_control_word(), 0x037F);
    }

    #[test]
    fn c_abi_matches_rust_api() {
        let big = int80(false, u64::MAX);
        let mut unaligned = [0u8; 11];
        unaligned[1..].copy_from_slice(&big);
        // SAFETY: 10 readable bytes at offset 1.
        assert_eq!(unsafe { qb_qbr_f80(unaligned.as_ptr().add(1)) }, -1);
        let half = f80(2.5);
        // SAFETY: 10-byte array.
        assert_eq!(unsafe { qb_fistp_i64_f80(half.as_ptr()) }, 2);
        assert_eq!(qb_qbr_f64(1.5e19), qbr(f80(1.5e19)));
        assert_eq!(qb_qbr_f64(-3.5), -4);
        assert_eq!(qb_fistp_i64_f64(1e19), i64::MIN);
        assert_eq!(qb_fistp_i64_f64(-0.5), 0);
        assert_eq!(qb_fistp_i32(6.5), 6);
    }

    #[test]
    fn f80_from_f64_encodes_exactly() {
        assert_eq!(f80_from_f64(1.0), raw(0, 0x3FFF, 1u64 << 63));
        assert_eq!(f80_from_f64(-2.5), raw(1, 0x4000, 0b101u64 << 61));
        assert_eq!(f80_from_f64(0.0), raw(0, 0, 0));
        assert_eq!(f80_from_f64(-0.0), raw(1, 0, 0));
        assert_eq!(f80_from_f64(f64::INFINITY), raw(0, 0x7FFF, 1u64 << 63));
        // Smallest f64 subnormal 2^-1074 is a normal f80.
        assert_eq!(f80_from_f64(f64::from_bits(1)), raw(0, (16383 - 1074) as u16, 1u64 << 63));
        assert_eq!(f80_from_f64(f64::NAN)[9] & 0x7F, 0x7F);
        // Matches the independent test encoder for ordinary values.
        for v in [3.75, -1e300, 6.02e23, 1.0 / 3.0] {
            assert_eq!(f80_from_f64(v), f80(v));
        }
    }

    #[test]
    fn halves_round_to_even() {
        let cases = [
            (0.5, 0), (1.5, 2), (2.5, 2), (3.5, 4), (-0.5, 0), (-1.5, -2), (-2.5, -2),
            (2.4999, 2), (2.5000001, 3), (-7.75, -8), (0.0, 0), (-0.0, 0),
        ];
        for (v, want) in cases {
            assert_eq!(fistp_i64(f80(v)), want, "fistp_i64({v})");
            assert_eq!(qbr(f80(v)), want, "qbr({v})");
        }
    }

    #[test]
    fn fistp_i64_limits_and_specials() {
        assert_eq!(fistp_i64(int80(false, i64::MAX as u64)), i64::MAX);
        assert_eq!(fistp_i64(int80(true, 1u64 << 63)), i64::MIN);
        assert_eq!(fistp_i64(int80(false, 1u64 << 63)), INDEFINITE);
        assert_eq!(fistp_i64(int80(true, (1u64 << 63) + 1)), INDEFINITE);
        assert_eq!(fistp_i64(f80(1e30)), INDEFINITE);
        assert_eq!(fistp_i64(f80(f64::NAN)), INDEFINITE);
        assert_eq!(fistp_i64(f80(f64::INFINITY)), INDEFINITE);
        assert_eq!(fistp_i64(f80(f64::NEG_INFINITY)), INDEFINITE);
        assert_eq!(fistp_i64(f80(f64::MIN_POSITIVE)), 0);
        assert_eq!(fistp_i64(raw(0, 1, 1u64 << 63)), 0); // tiniest normal f80
        assert_eq!(fistp_i64(raw(0, 0, 1)), 0); // f80 denormal
    }

    #[test]
    fn fistp_i64_needs_full_64_bit_precision() {
        // 2^62 + 1.5 is exact in f80 but not in f64; must round to even (2^62 + 2).
        let v = raw(0, 16383 + 62, (1u64 << 63) | 0b11); // (2^63 + 3) * 2^-1
        assert_eq!(fistp_i64(v), (1i64 << 62) + 2);
        let v = raw(0, 16383 + 62, (1u64 << 63) | 0b01); // 2^62 + 0.5
        assert_eq!(fistp_i64(v), 1i64 << 62);
    }

    #[test]
    fn qbr_maps_values_above_i64_max_to_unsigned_bit_pattern() {
        assert_eq!(qbr(int80(false, 1u64 << 63)), i64::MIN); // 2^63 -> 0x8000..
        assert_eq!(qbr(int80(false, u64::MAX)), -1); // 2^64-1 -> 0xFFFF..
        assert_eq!(qbr(int80(false, (1u64 << 63) + 5)), i64::MIN + 5);
        // 2^63 - 0.5 is > i64::MAX: (f - 2^63) = -0.5 rounds to 0, then the high bit is set.
        assert_eq!(qbr(raw(0, 16383 + 62, u64::MAX)), i64::MIN);
        assert_eq!(qbr(f80(2f64.powi(64))), INDEFINITE);
        assert_eq!(qbr(f80(f64::INFINITY)), INDEFINITE);
        assert_eq!(qbr(f80(f64::NAN)), INDEFINITE);
        assert_eq!(qbr(int80(false, i64::MAX as u64)), i64::MAX);
    }

    #[test]
    fn fistp_i32_rounds_to_even_and_saturates_to_indefinite() {
        let cases = [
            (2.5, 2), (3.5, 4), (-2.5, -2), (-3.5, -4), (0.49999999999999994, 0),
            (2147483647.0, i32::MAX), (2147483647.4, i32::MAX),
            (2147483647.5, i32::MIN), (-2147483648.0, i32::MIN),
            (-2147483648.5, i32::MIN), (-2147483649.0, i32::MIN),
            (1e10, i32::MIN), (f64::NAN, i32::MIN), (f64::INFINITY, i32::MIN),
        ];
        for (v, want) in cases {
            assert_eq!(fistp_i32(v), want, "fistp_i32({v})");
        }
        // f32 inputs widen exactly to f64.
        assert_eq!(fistp_i32(f64::from(16777217f32)), 16777216);
        assert_eq!(fistp_i32(f64::from(-0.5f32)), 0);
    }
}
