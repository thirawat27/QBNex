//! `VAL(s$)`: string -> number.
//!
//! Replaces `func_val` in libqb.cpp. Decimal input is normalised the same way
//! the legacy code did and then converted with correct rounding (nearest,
//! ties to even) to the x87 80-bit format, which is what `sscanf("%Lf")` did
//! on the legacy toolchains. `&H`, `&O` and `&B` literals are parsed as
//! 64-bit integers; too many digits is error 6 (overflow).

use crate::round::F80;

/// QBasic error code raised by `VAL` on an oversized `&H`/`&O`/`&B` literal.
pub const ERR_OVERFLOW: i32 = 6;

/// Significant digits kept exactly; any further non-zero digit only makes
/// the value "a little larger" for rounding. Legacy libqb overran a 256-byte
/// buffer beyond 256 digits, so nothing longer had defined behaviour.
const MAX_DIGITS: usize = 1000;

/// Parses like legacy `func_val`. `Err(code)` means the runtime must raise
/// that error and use 0 as the result.
pub fn val(s: &[u8]) -> Result<F80, i32> {
    match scan(s) {
        Scan::Literal(at) => literal(&s[at..]).map(int_to_f80),
        Scan::Decimal(d) => Ok(d.to_f80()),
    }
}

enum Scan {
    /// `&` seen at the start (index of the `&`).
    Literal(usize),
    Decimal(Decimal),
}

#[derive(Default)]
struct Decimal {
    neg: bool,
    /// Significant digits (0..=9), first one non-zero.
    digits: Vec<u8>,
    /// A non-zero digit was dropped after `MAX_DIGITS`.
    inexact_tail: bool,
    /// Integer value of the digits before the point (fast path only).
    int_value: i64,
    /// Number of significant digits before the point; negative for leading
    /// zeros after it (legacy `most_significant_digit_position`).
    point_pos: i64,
    exponent: i64,
    exponent_neg: bool,
}

/// The legacy scanner: steps 0 = start, 1 = integer part, 2 = fraction,
/// 3 = exponent marker seen, 4 = exponent digits. Blanks are ignored
/// everywhere; any other unexpected byte ends the number.
fn scan(s: &[u8]) -> Scan {
    let mut d = Decimal::default();
    let mut step = 0;
    let mut exponent_digits = 0;
    for (i, &c) in s.iter().enumerate() {
        match c {
            b' ' | b'\t' => {}
            b'&' if step == 0 => return Scan::Literal(i),
            b'-' if step == 0 => {
                d.neg = true;
                step = 1;
            }
            b'-' if step == 3 => {
                d.exponent_neg = true;
                step = 4;
            }
            b'+' if step == 0 || step == 3 => step += 1,
            b'0'..=b'9' => {
                let digit = c - b'0';
                let significant = !d.digits.is_empty() || d.inexact_tail || digit > 0;
                if step <= 1 {
                    step = 1;
                    if significant {
                        d.point_pos += 1;
                        d.push_digit(digit);
                        d.int_value = d.int_value.wrapping_mul(10).wrapping_add(i64::from(digit));
                    }
                } else if step == 2 {
                    if significant {
                        d.push_digit(digit);
                    } else {
                        d.point_pos -= 1;
                    }
                } else {
                    step = 4;
                    if exponent_digits > 0 || digit > 0 {
                        if exponent_digits >= 18 {
                            break;
                        }
                        d.exponent = d.exponent * 10 + i64::from(digit);
                        exponent_digits += 1;
                    }
                }
            }
            b'.' if step <= 1 => step = 2,
            b'D' | b'E' | b'd' | b'e' if step <= 2 => step = 3,
            _ => break,
        }
    }
    Scan::Decimal(d)
}

impl Decimal {
    fn push_digit(&mut self, digit: u8) {
        if self.digits.len() < MAX_DIGITS {
            self.digits.push(digit);
        } else if digit != 0 {
            self.inexact_tail = true;
        }
    }

    fn significant_count(&self) -> usize {
        self.digits.len()
    }

    fn to_f80(&self) -> F80 {
        if self.digits.is_empty() {
            return [0; 10];
        }
        // Integers without exponent take the exact int64 path, as in libqb.
        if self.exponent == 0
            && !self.inexact_tail
            && self.significant_count() as i64 == self.point_pos
            && self.significant_count() < 19
        {
            let v = if self.neg { -self.int_value } else { self.int_value };
            return int_to_f80(v);
        }
        // value = D * 10^e10 with D the digit string as an integer.
        // (Fixes legacy defect L-05: the sign applies to the written exponent.)
        let written = if self.exponent_neg { -self.exponent } else { self.exponent };
        let e10 = written + self.point_pos - self.digits.len() as i64;
        decimal_to_f80(self.neg, &self.digits, e10, self.inexact_tail)
    }
}

/// `&H`, `&O`, `&B` literal starting at `&`.
fn literal(s: &[u8]) -> Result<i64, i32> {
    if s.len() < 3 {
        return Ok(0);
    }
    let (bits, radix) = match s[1] {
        b'H' | b'h' => (4, 16),
        b'O' | b'o' => (3, 8),
        b'B' | b'b' => (1, 2),
        _ => return Ok(0),
    };
    let body = &s[2..];
    let mut value: i64 = 0;
    let mut count = 0usize;
    for (k, &c) in body.iter().enumerate() {
        let Some(digit) = (c as char).to_digit(radix) else { break };
        value = (value << bits) | i64::from(digit);
        if count > 0 || digit > 0 {
            count += 1;
        }
        let too_long = match radix {
            16 => count > 16,
            2 => count > 64,
            // 22 octal digits hold 66 bits: only a leading 0 or 1 fits.
            _ => count > 22 || (count == 22 && body[k + 1 - 22] > b'1'),
        };
        if too_long {
            return Err(ERR_OVERFLOW);
        }
    }
    Ok(value)
}

fn int_to_f80(n: i64) -> F80 {
    if n == 0 {
        return [0; 10];
    }
    let m = n.unsigned_abs();
    let lz = m.leading_zeros();
    encode(n < 0, 16383 + 63 - lz, m << lz)
}

fn encode(neg: bool, exp: u32, mant: u64) -> F80 {
    let mut b = [0u8; 10];
    b[..8].copy_from_slice(&mant.to_le_bytes());
    b[8..].copy_from_slice(&(((neg as u16) << 15) | exp as u16).to_le_bytes());
    b
}

/// Correctly rounded (nearest, ties to even) conversion of
/// `(-1)^neg * (digits + tail) * 10^e10` to x87 extended precision.
fn decimal_to_f80(neg: bool, digits: &[u8], e10: i64, inexact_tail: bool) -> F80 {
    let inf = encode(neg, 0x7FFF, 1 << 63);
    let zero = encode(neg, 0, 0);
    let n = digits.len() as i64;
    // f80 spans roughly 3.6e-4951 .. 1.19e4932.
    if e10 + n - 1 > 4933 {
        return inf;
    }
    if e10 + n < -4952 {
        return zero;
    }
    let mut num = Big::from_digits(digits);
    let mut den = Big::one();
    if e10 >= 0 {
        num.mul_pow10(e10 as u32);
    } else {
        den.mul_pow10((-e10) as u32);
    }
    // Scale so that the quotient has 66..=67 bits.
    let b = num.bit_len() as i64 - den.bit_len() as i64;
    let scale = 66 - b; // value = q * 2^-scale
    if scale >= 0 {
        num.shl(scale as u32);
    } else {
        den.shl((-scale) as u32);
    }
    let (q, rem_nonzero) = num.div_small_quotient(&den, 68);
    let sticky = rem_nonzero || inexact_tail;
    let top = 127 - q.leading_zeros() as i64; // q in [2^top, 2^(top+1))
    let e2 = top - scale; // value in [2^e2, 2^(e2+1))
    let biased = e2 + 16383;
    let (mant, exp) = if biased >= 1 {
        let mut m = round_shift(q, (top - 63) as u32, sticky);
        let mut biased = biased;
        if m >> 64 != 0 {
            m >>= 1;
            biased += 1;
        }
        (m as u64, biased)
    } else {
        // Denormal: the unit is 2^-16445.
        let shift = scale - 16445;
        let m = if shift >= 128 { 0 } else { round_shift(q, shift as u32, sticky) };
        let exp = if m >> 63 != 0 { 1 } else { 0 };
        (m as u64, exp)
    };
    if exp >= 0x7FFF {
        return inf;
    }
    if mant == 0 {
        return zero;
    }
    encode(neg, exp as u32, mant)
}

/// `q >> k` rounded to nearest, ties to even; `sticky` = value is a bit more than q.
fn round_shift(q: u128, k: u32, sticky: bool) -> u128 {
    if k == 0 {
        return q;
    }
    if k > 127 {
        return 0;
    }
    let kept = q >> k;
    let rem = q & ((1u128 << k) - 1);
    let half = 1u128 << (k - 1);
    if rem > half || (rem == half && (sticky || kept & 1 == 1)) {
        kept + 1
    } else {
        kept
    }
}

/// Minimal unsigned big integer (little-endian u64 limbs) for `VAL`.
#[derive(Clone, PartialEq, Eq)]
struct Big(Vec<u64>);

impl Big {
    fn one() -> Self {
        Big(vec![1])
    }

    fn from_digits(digits: &[u8]) -> Self {
        let mut b = Big(vec![0]);
        for &d in digits {
            b.mul_add_small(10, u64::from(d));
        }
        b
    }

    fn mul_add_small(&mut self, m: u64, add: u64) {
        let mut carry = u128::from(add);
        for limb in &mut self.0 {
            let t = u128::from(*limb) * u128::from(m) + carry;
            *limb = t as u64;
            carry = t >> 64;
        }
        if carry != 0 {
            self.0.push(carry as u64);
        }
    }

    fn mul_pow10(&mut self, mut e: u32) {
        while e >= 19 {
            self.mul_add_small(10_000_000_000_000_000_000, 0);
            e -= 19;
        }
        if e > 0 {
            self.mul_add_small(10u64.pow(e), 0);
        }
    }

    fn bit_len(&self) -> u32 {
        let mut i = self.0.len();
        while i > 0 && self.0[i - 1] == 0 {
            i -= 1;
        }
        if i == 0 {
            0
        } else {
            (i as u32 - 1) * 64 + (64 - self.0[i - 1].leading_zeros())
        }
    }

    fn shl(&mut self, bits: u32) {
        let limbs = (bits / 64) as usize;
        let rest = bits % 64;
        if rest != 0 {
            let mut carry = 0u64;
            for limb in &mut self.0 {
                let next = *limb >> (64 - rest);
                *limb = (*limb << rest) | carry;
                carry = next;
            }
            if carry != 0 {
                self.0.push(carry);
            }
        }
        if limbs != 0 {
            self.0.splice(0..0, std::iter::repeat_n(0, limbs));
        }
    }

    fn shr1(&mut self) {
        let mut carry = 0u64;
        for limb in self.0.iter_mut().rev() {
            let next = *limb << 63;
            *limb = (*limb >> 1) | carry;
            carry = next;
        }
    }

    fn ge(&self, other: &Big) -> bool {
        let len = self.0.len().max(other.0.len());
        for i in (0..len).rev() {
            let a = self.0.get(i).copied().unwrap_or(0);
            let b = other.0.get(i).copied().unwrap_or(0);
            if a != b {
                return a > b;
            }
        }
        true
    }

    fn sub_assign(&mut self, other: &Big) {
        let mut borrow = false;
        for i in 0..self.0.len() {
            let b = other.0.get(i).copied().unwrap_or(0);
            let (r1, o1) = self.0[i].overflowing_sub(b);
            let (r2, o2) = r1.overflowing_sub(u64::from(borrow));
            self.0[i] = r2;
            borrow = o1 || o2;
        }
    }

    fn is_zero(&self) -> bool {
        self.0.iter().all(|l| *l == 0)
    }

    /// Quotient `self / den`, known to be below 2^`bits`, and whether the
    /// remainder is non-zero. Consumes `self` as the running remainder.
    fn div_small_quotient(mut self, den: &Big, bits: u32) -> (u128, bool) {
        let mut d = den.clone();
        d.shl(bits - 1);
        let mut q = 0u128;
        for bit in (0..bits).rev() {
            if self.ge(&d) {
                self.sub_assign(&d);
                q |= 1u128 << bit;
            }
            d.shr1();
        }
        (q, !self.is_zero())
    }
}

/// Nearest-even conversion of an x87 extended value to `f64`, for platforms
/// whose C `long double` is a plain double.
pub fn f64_from_f80(f: F80) -> f64 {
    let mut m8 = [0u8; 8];
    m8.copy_from_slice(&f[..8]);
    let mant = u64::from_le_bytes(m8);
    let se = u16::from_le_bytes([f[8], f[9]]);
    let sign = u64::from(se >> 15) << 63;
    let exp = i64::from(se & 0x7FFF);
    if exp == 0x7FFF {
        return f64::from_bits(sign | if mant << 1 == 0 { 0x7FF0_0000_0000_0000 } else { 0x7FF8_0000_0000_0000 | ((mant << 1) >> 12) });
    }
    if exp != 0 && mant >> 63 == 0 {
        return f64::NAN; // unnormal: invalid operand on x87
    }
    if mant == 0 {
        return f64::from_bits(sign);
    }
    let lz = i64::from(mant.leading_zeros());
    let m = u128::from(mant << lz);
    let e = exp.max(1) - 16383 - 63 - lz; // value = m * 2^e, m has 64 bits
    let top = e + 63; // value in [2^top, 2^(top+1))
    let bits = if top >= -1022 {
        let mut r = round_shift(m, 11, false);
        let mut top = top;
        if r >> 53 != 0 {
            r >>= 1;
            top += 1;
        }
        if top > 1023 {
            0x7FF0_0000_0000_0000
        } else {
            (((top + 1023) as u64) << 52) | (r as u64 & ((1 << 52) - 1))
        }
    } else {
        let k = -(e + 1074);
        if k > 127 { 0 } else { round_shift(m, k as u32, false) as u64 }
    };
    f64::from_bits(sign | bits)
}

// --- C ABI (include/qb_runtime.h) ---------------------------------------------------

/// Writes the 10-byte x87 result to `out_f80` and returns 0, or returns the
/// QBasic error code to raise (the result is then 0).
///
/// # Safety
/// `chr` must be readable for `len` bytes (or `len <= 0`); `out_f80` writable for 10.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_val(chr: *const u8, len: i32, out_f80: *mut u8) -> i32 {
    let s: &[u8] = if len <= 0 || chr.is_null() {
        &[]
    } else {
        // SAFETY: caller guarantees `len` readable bytes.
        unsafe { core::slice::from_raw_parts(chr, len as usize) }
    };
    let (result, code) = match val(s) {
        Ok(f) => (f, 0),
        Err(code) => ([0; 10], code),
    };
    // SAFETY: caller guarantees 10 writable bytes.
    unsafe { core::ptr::copy_nonoverlapping(result.as_ptr(), out_f80, 10) };
    code
}

/// # Safety
/// `f80` must be readable for 10 bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_f80_to_f64(f80: *const u8) -> f64 {
    let mut b = [0u8; 10];
    // SAFETY: caller guarantees 10 readable bytes.
    unsafe { core::ptr::copy_nonoverlapping(f80, b.as_mut_ptr(), 10) };
    f64_from_f80(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f64_from_f80_rounds_to_nearest_even() {
        for x in [0.1, -2.5, 1e300, -1e-300, f64::MAX, f64::MIN_POSITIVE, 5e-324, 1.0 / 3.0] {
            assert_eq!(f64_from_f80(crate::round::f80_from_f64(x)), x);
        }
        assert_eq!(f64_from_f80(v("0.1")), 0.1);
        assert_eq!(f64_from_f80(v("1E400")), f64::INFINITY);
        assert_eq!(f64_from_f80(v("-1E400")), f64::NEG_INFINITY);
        assert_eq!(f64_from_f80(v("1E-400")), 0.0);
        // 2^-1075 is exactly half of the smallest subnormal: ties to even (0).
        assert_eq!(f64_from_f80(encode(false, 16383 - 1075, 1 << 63)).to_bits(), 0);
        assert_eq!(f64_from_f80(encode(false, 16383 - 1075, (1 << 63) | 1)).to_bits(), 1);
        assert_eq!(f64_from_f80(v("2.5E-324")).to_bits(), 1); // just above the tie
        assert_eq!(f64_from_f80(v("2.4E-324")).to_bits(), 0);
        // 1 + 2^-53 + 2^-60: above the halfway point, must round up.
        assert_eq!(f64_from_f80(encode(false, 16383, (1 << 63) | (1 << 10) | (1 << 3))), 1.0 + f64::EPSILON);
        assert_eq!(f64_from_f80(encode(false, 16383, (1 << 63) | (1 << 10))), 1.0); // exact tie -> even
        assert!(f64_from_f80(encode(false, 0x7FFF, (1 << 63) | 1)).is_nan());
    }

    #[test]
    fn c_abi_writes_result_or_returns_error() {
        let mut out = [0xEEu8; 10];
        let s = b"  -12.5e1xyz";
        // SAFETY: valid buffers.
        let rc = unsafe { qb_val(s.as_ptr(), s.len() as i32, out.as_mut_ptr()) };
        assert_eq!((rc, out), (0, int_to_f80(-125)));
        let s = b"&H10000000000000000";
        // SAFETY: valid buffers.
        let rc = unsafe { qb_val(s.as_ptr(), s.len() as i32, out.as_mut_ptr()) };
        assert_eq!((rc, out), (ERR_OVERFLOW, [0; 10]));
        // SAFETY: zero length never reads chr.
        let rc = unsafe { qb_val(core::ptr::null(), 0, out.as_mut_ptr()) };
        assert_eq!((rc, out), (0, [0; 10]));
        let tenth = v("0.1");
        // SAFETY: 10-byte array.
        assert_eq!(unsafe { qb_f80_to_f64(tenth.as_ptr()) }, 0.1);
    }

    fn raw(neg: bool, exp: u16, mant: u64) -> F80 {
        let mut b = [0u8; 10];
        b[..8].copy_from_slice(&mant.to_le_bytes());
        b[8..].copy_from_slice(&(((neg as u16) << 15) | exp).to_le_bytes());
        b
    }

    fn v(s: &str) -> F80 {
        val(s.as_bytes()).expect("no error")
    }

    fn int(n: i64) -> F80 {
        if n == 0 {
            return [0; 10];
        }
        let m = n.unsigned_abs();
        let lz = m.leading_zeros();
        raw(n < 0, (16383 + 63 - lz) as u16, m << lz)
    }

    const ONE_TENTH: F80 = [0xCD, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xFB, 0x3F];

    #[test]
    fn empty_and_garbage_are_zero() {
        assert_eq!(v(""), [0; 10]);
        assert_eq!(v("x"), [0; 10]);
        assert_eq!(v("-"), [0; 10]);
        assert_eq!(v("E5"), [0; 10]);
        assert_eq!(v("."), [0; 10]);
    }

    #[test]
    fn integers_take_the_exact_fast_path() {
        assert_eq!(v("42"), int(42));
        assert_eq!(v("-42"), int(-42));
        assert_eq!(v("+7"), int(7));
        assert_eq!(v("007"), int(7));
        assert_eq!(v("9223372036854775807"), int(i64::MAX));
        assert_eq!(v("1 2 3"), int(123)); // blanks are skipped anywhere
        assert_eq!(v("\t 12abc"), int(12)); // stops at the first invalid char
        assert_eq!(v("12.5.3"), v("12.5"));
    }

    #[test]
    fn decimals_round_correctly_to_extended_precision() {
        assert_eq!(v("0.1"), ONE_TENTH);
        assert_eq!(v(".1"), ONE_TENTH);
        assert_eq!(v("1E-1"), ONE_TENTH);
        assert_eq!(v("1D-1"), ONE_TENTH);
        assert_eq!(v("0.01e1"), ONE_TENTH);
        assert_eq!(v("-0.5"), raw(true, 0x3FFE, 1 << 63));
        assert_eq!(v("12.5"), raw(false, 0x4002, 0xC8 << 56));
        assert_eq!(v("-3e2"), int(-300));
        assert_eq!(v("1.5E+3"), int(1500));
    }

    #[test]
    fn negative_exponent_applies_to_the_written_mantissa() {
        // Legacy defect L-05: libqb negated the exponent *after* normalising,
        // so VAL("123E-2") was 1.23E-4. QBasic gives 1.23. Intentional deviation.
        assert_eq!(v("123E-2"), v("1.23"));
        assert_eq!(v("0.5E-1"), v("0.05"));
        assert_eq!(v("12E-1"), v("1.2"));
        assert_eq!(v("0.05E-1"), v("0.005"));
        assert_eq!(v("-250E-2"), v("-2.5"));
        assert_eq!(v("1.5E-1"), v("0.15"));
    }

    #[test]
    fn ties_round_to_even() {
        // 2^64 + 1 needs 65 bits; it is exactly half-way, so it rounds down to 2^64.
        assert_eq!(v("18446744073709551617"), raw(false, 16383 + 64, 1 << 63));
        // 2^64 + 3 is half-way between two candidates and rounds up to the even one.
        assert_eq!(v("18446744073709551619"), raw(false, 16383 + 64, (1 << 63) | 2));
    }

    #[test]
    fn extreme_exponents() {
        assert_eq!(v("1E5000"), raw(false, 0x7FFF, 1 << 63)); // +inf
        assert_eq!(v("-1E5000"), raw(true, 0x7FFF, 1 << 63)); // -inf
        assert_eq!(v("1E-5000"), [0; 10]);
        assert_eq!(v("-1E-5000"), raw(true, 0, 0)); // -0
        // Smallest normal 2^-16382 and smallest denormal 2^-16445.
        assert_eq!(v("3.36210314311209350626267781732175260e-4932"), raw(false, 1, 1 << 63));
        assert_eq!(v("3.64519953188247460252840593361941982e-4951"), raw(false, 0, 1));
        assert_eq!(v("1e-4951"), [0; 10]); // below half the smallest denormal
        assert_eq!(v("2e-4951"), raw(false, 0, 1));
        // Largest finite value, and just above it.
        assert_eq!(v("1.18973149535723176502e4932"), raw(false, 0x7FFE, u64::MAX));
        assert_eq!(v("1.2e4932"), raw(false, 0x7FFF, 1 << 63));
    }

    #[test]
    fn hex_octal_binary_literals() {
        assert_eq!(v("&H10"), int(16));
        assert_eq!(v("&hfF"), int(255));
        assert_eq!(v("&HFFFFFFFFFFFFFFFF"), int(-1));
        assert_eq!(v("&O17"), int(15));
        assert_eq!(v("&B1010"), int(10));
        assert_eq!(v("  &H1G"), int(1));
        assert_eq!(v("&H"), [0; 10]);
        assert_eq!(v("&X12"), [0; 10]);
        assert_eq!(v("1&H10"), int(1));
        assert_eq!(v("&H0000000000000000001"), int(1)); // leading zeros do not count
    }

    #[test]
    fn oversized_literals_are_overflow_errors() {
        assert_eq!(val(b"&H10000000000000000"), Err(ERR_OVERFLOW));
        assert_eq!(val(b"&O2000000000000000000000"), Err(ERR_OVERFLOW));
        assert_eq!(v("&O1777777777777777777777"), int(-1));
        assert_eq!(val(&[b"&B1".as_slice(), &[b'0'; 64]].concat()), Err(ERR_OVERFLOW));
    }
}
