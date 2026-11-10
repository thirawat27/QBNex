//! Conversions between [`F80`] and integers, `f32`, `f64` with x87 semantics.

use crate::arith::{fin_norm, round_pack, shift_mode, zero};
use crate::{Class, Round, F80, INT_BIT, QUIET_BIT};

impl F80 {
    /// `fild` (exact).
    pub fn from_i64(v: i64) -> F80 {
        if v == 0 {
            return F80::ZERO;
        }
        round_pack(v < 0, 0, v.unsigned_abs() as u128, false)
    }

    pub fn from_i32(v: i32) -> F80 {
        F80::from_i64(v as i64)
    }

    pub fn from_i16(v: i16) -> F80 {
        F80::from_i64(v as i64)
    }

    /// Exact (what g++ emits: `fild` plus 2^64 when the top bit is set).
    pub fn from_u64(v: u64) -> F80 {
        if v == 0 {
            return F80::ZERO;
        }
        round_pack(false, 0, v as u128, false)
    }

    /// `fld qword` (exact; a signalling NaN becomes quiet).
    pub fn from_f64(v: f64) -> F80 {
        let bits = v.to_bits();
        let neg = bits >> 63 != 0;
        let e = ((bits >> 52) & 0x7FF) as i32;
        let frac = bits & ((1 << 52) - 1);
        if e == 0x7FF {
            if frac == 0 {
                return F80::INFINITY.with_sign(neg);
            }
            return F80::from_parts(neg, 0x7FFF, INT_BIT | QUIET_BIT | (frac << 11));
        }
        if e == 0 {
            if frac == 0 {
                return zero(neg);
            }
            return round_pack(neg, -1074, frac as u128, false);
        }
        round_pack(neg, e - 1075, (frac | (1 << 52)) as u128, false)
    }

    /// `fld dword` (exact; a signalling NaN becomes quiet).
    pub fn from_f32(v: f32) -> F80 {
        let bits = v.to_bits();
        let neg = bits >> 31 != 0;
        let e = ((bits >> 23) & 0xFF) as i32;
        let frac = (bits & ((1 << 23) - 1)) as u64;
        if e == 0xFF {
            if frac == 0 {
                return F80::INFINITY.with_sign(neg);
            }
            return F80::from_parts(neg, 0x7FFF, INT_BIT | QUIET_BIT | (frac << 40));
        }
        if e == 0 {
            if frac == 0 {
                return zero(neg);
            }
            return round_pack(neg, -149, frac as u128, false);
        }
        round_pack(neg, e - 150, (frac | (1 << 23)) as u128, false)
    }

    /// `fst qword`: round to nearest even.
    pub fn to_f64(self) -> f64 {
        f64::from_bits(narrow(self, 53, 1023, 11))
    }

    /// `fst dword`: round to nearest even.
    pub fn to_f32(self) -> f32 {
        f32::from_bits(narrow(self, 24, 127, 8) as u32)
    }

    /// `fistp word` with rounding `mode` (`Nearest` = `fistp` under the default
    /// control word, `Zero` = `fisttp`, the C cast). Out of range, NaN, infinity
    /// and unsupported values give the integer indefinite `-32768`.
    pub fn to_i16(self, mode: Round) -> i16 {
        match to_int(self, mode, 15) {
            Some(v) => v as i16,
            None => i16::MIN,
        }
    }

    /// `fistp dword` / `fisttp dword`; indefinite is `i32::MIN`.
    pub fn to_i32(self, mode: Round) -> i32 {
        match to_int(self, mode, 31) {
            Some(v) => v as i32,
            None => i32::MIN,
        }
    }

    /// `fistp qword` / `fisttp qword`; indefinite is `i64::MIN`.
    pub fn to_i64(self, mode: Round) -> i64 {
        match to_int(self, mode, 63) {
            Some(v) => v as i64,
            None => i64::MIN,
        }
    }

    /// The C cast `(unsigned long long)x` as MinGW g++ compiles it: values below
    /// 2^63 (and NaN) go through `fisttp qword`, others through
    /// `fisttp(x - 2^63) ^ 2^63`.
    pub fn to_u64_cast(self) -> u64 {
        let two63 = F80::from_parts(false, 0x3FFF + 63, INT_BIT);
        match self.compare(two63) {
            Some(core::cmp::Ordering::Less) | None => self.to_i64(Round::Zero) as u64,
            _ => ((self - two63).to_i64(Round::Zero) as u64) ^ (1 << 63),
        }
    }
}

/// Rounds to an integer of `bits` magnitude bits plus sign; `None` = indefinite.
fn to_int(x: F80, mode: Round, bits: u32) -> Option<i128> {
    match x.class() {
        Class::Zero => return Some(0),
        Class::Denormal | Class::Normal => {}
        _ => return None,
    }
    let f = fin_norm(x);
    let mag: u128 = if f.exp >= 0 {
        if f.exp > 64 {
            return None;
        }
        (f.mant as u128) << f.exp
    } else {
        shift_mode(f.mant as u128, (-f.exp) as u32, false, mode, f.neg)
    };
    let limit = 1u128 << bits;
    if f.neg {
        if mag > limit {
            return None;
        }
        Some(-(mag as i128))
    } else {
        if mag >= limit {
            return None;
        }
        Some(mag as i128)
    }
}

/// Rounds to an IEEE binary format with `p` significand bits, exponent bias
/// `bias`, `ebits` exponent bits; returns the raw bits.
fn narrow(x: F80, p: u32, bias: i32, ebits: u32) -> u64 {
    let sign_bit = 1u64 << (p - 1 + ebits);
    let emax_field = (1u64 << ebits) - 1;
    let frac_mask = (1u64 << (p - 1)) - 1;
    let s = if x.sign() { sign_bit } else { 0 };
    match x.class() {
        Class::Zero => return s,
        Class::Infinity => return s | (emax_field << (p - 1)),
        Class::QuietNaN | Class::SignalingNaN => {
            let top = (x.mant << 1) >> (64 - (p - 1));
            return s | (emax_field << (p - 1)) | top | (1 << (p - 2));
        }
        Class::Unsupported => return sign_bit | (emax_field << (p - 1)) | (1 << (p - 2)),
        Class::Denormal | Class::Normal => {}
    }
    let f = fin_norm(x);
    let lead = f.exp + 63; // value in [2^lead, 2^(lead+1))
    let min_normal = 1 - bias;
    if lead >= min_normal {
        let mut m = shift_mode(f.mant as u128, 64 - p, false, Round::Nearest, f.neg) as u64;
        let mut l = lead;
        if m >> p != 0 {
            m >>= 1;
            l += 1;
        }
        if l > bias {
            return s | (emax_field << (p - 1));
        }
        s | (((l + bias) as u64) << (p - 1)) | (m & frac_mask)
    } else {
        // Unit of the subnormals: 2^(min_normal - (p-1)).
        let unit = min_normal - (p as i32 - 1);
        let k = (unit - f.exp) as u32;
        let m = shift_mode(f.mant as u128, k, false, Round::Nearest, f.neg) as u64;
        // m == 2^(p-1) is the smallest normal: the exponent field becomes 1.
        s | m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_round_trips() {
        for v in [0i64, 1, -1, 12345, i64::MAX, i64::MIN, i32::MIN as i64] {
            assert_eq!(F80::from_i64(v).to_i64(Round::Nearest), v);
        }
        assert_eq!(F80::from_u64(u64::MAX).to_u64_cast(), u64::MAX);
        assert_eq!(F80::from_u64(u64::MAX).mant, u64::MAX);
        assert_eq!(F80::from_i64(1), F80::ONE);
    }

    #[test]
    fn fistp_rounds_and_saturates_to_indefinite() {
        let h = F80::from_f64(2.5);
        assert_eq!(h.to_i32(Round::Nearest), 2);
        assert_eq!(F80::from_f64(3.5).to_i32(Round::Nearest), 4);
        assert_eq!(F80::from_f64(-2.7).to_i32(Round::Zero), -2);
        assert_eq!(F80::from_f64(32767.6).to_i16(Round::Nearest), i16::MIN);
        assert_eq!(F80::from_f64(-32768.4).to_i16(Round::Nearest), -32768);
        assert_eq!(F80::NAN.to_i64(Round::Zero), i64::MIN);
        assert_eq!(F80::from_f64(-1.0).to_u64_cast(), u64::MAX);
        assert_eq!(F80::from_f64(1e30).to_u64_cast(), 0);
    }

    #[test]
    fn f64_round_trip_and_rounding() {
        for v in [0.1, -0.0, 1e300, 5e-324, f64::MAX, f64::MIN_POSITIVE, f64::from_bits(0x000F_FFFF_FFFF_FFFF)] {
            assert_eq!(F80::from_f64(v).to_f64().to_bits(), v.to_bits());
        }
        // 1 + 2^-53 exactly: tie, stays at 1.0; 1 + 2^-53 + 2^-63 rounds up.
        let tie = F80::from_parts(false, 0x3FFF, INT_BIT | (1 << 10));
        assert_eq!(tie.to_f64(), 1.0);
        let above = F80::from_parts(false, 0x3FFF, INT_BIT | (1 << 10) | 1);
        assert_eq!(above.to_f64(), 1.0 + f64::EPSILON);
        assert_eq!(F80::MAX.to_f64(), f64::INFINITY);
        assert_eq!(F80::MIN_POSITIVE.to_f64(), 0.0);
        assert_eq!(F80::INDEFINITE.to_f64().to_bits(), 0xFFF8_0000_0000_0000);
    }

    #[test]
    fn f32_conversions() {
        for v in [0.1f32, 1e-45, 3.4e38, -7.25] {
            assert_eq!(F80::from_f32(v).to_f32().to_bits(), v.to_bits());
        }
        assert_eq!(F80::from_f64(0.1).to_f32(), 0.1f32);
        let snan = f32::from_bits(0x7F80_0001);
        assert_eq!(F80::from_f32(snan).class(), Class::QuietNaN);
    }
}
