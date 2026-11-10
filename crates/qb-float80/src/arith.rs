//! x87 arithmetic in software: 64-bit significand, round to nearest even
//! (control word 0x037F), exceptions masked.

use crate::{Class, Round, F80, BIAS, QUIET_BIT};
use core::cmp::Ordering;

/// A finite non-zero value `mant * 2^exp` (`mant` not necessarily normalised).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Fin {
    pub neg: bool,
    pub exp: i32,
    pub mant: u64,
}

/// Decodes a denormal or normal encoding.
pub(crate) fn fin(x: F80) -> Fin {
    let e = x.exp() as i32;
    let exp = if e == 0 { 1 - BIAS - 63 } else { e - BIAS - 63 };
    Fin { neg: x.sign(), exp, mant: x.mant }
}

/// Same, with the significand shifted so that bit 63 is set.
pub(crate) fn fin_norm(x: F80) -> Fin {
    let mut f = fin(x);
    let lz = f.mant.leading_zeros();
    f.mant <<= lz;
    f.exp -= lz as i32;
    f
}

/// `q >> k` rounded with `mode` for a value of sign `neg`.
pub(crate) fn shift_mode(q: u128, k: u32, sticky: bool, mode: Round, neg: bool) -> u128 {
    if k == 0 {
        // Exact bits kept; a sticky remainder only matters for directed modes.
        return match (mode, sticky) {
            (Round::Up, true) if !neg => q + 1,
            (Round::Down, true) if neg => q + 1,
            _ => q,
        };
    }
    let (kept, rem_nonzero, half_cmp) = if k >= 129 {
        (0u128, q != 0 || sticky, Ordering::Less)
    } else if k == 128 {
        let half = 1u128 << 127;
        let c = if q > half || (q == half && sticky) {
            Ordering::Greater
        } else if q == half {
            Ordering::Equal
        } else {
            Ordering::Less
        };
        (0, q != 0 || sticky, c)
    } else {
        let kept = q >> k;
        let rem = q & ((1u128 << k) - 1);
        let half = 1u128 << (k - 1);
        let c = if rem > half || (rem == half && sticky) {
            Ordering::Greater
        } else if rem == half {
            Ordering::Equal
        } else {
            Ordering::Less
        };
        (kept, rem != 0 || sticky, c)
    };
    let up = match mode {
        Round::Nearest => half_cmp == Ordering::Greater || (half_cmp == Ordering::Equal && kept & 1 == 1),
        Round::Zero => false,
        Round::Up => rem_nonzero && !neg,
        Round::Down => rem_nonzero && neg,
    };
    kept + up as u128
}

/// Rounds `(sig + sticky*epsilon) * 2^exp` to the 80-bit format. `sig != 0`.
pub(crate) fn round_pack(neg: bool, exp: i32, sig: u128, sticky: bool) -> F80 {
    round_pack_mode(neg, exp, sig, sticky, Round::Nearest)
}

pub(crate) fn round_pack_mode(neg: bool, exp: i32, sig: u128, sticky: bool, mode: Round) -> F80 {
    debug_assert!(sig != 0);
    let lz = sig.leading_zeros();
    let sig = sig << lz;
    let exp = exp - lz as i32;
    // Leading bit weight is 2^(exp+127).
    let biased = exp + 127 + BIAS;
    if biased >= 1 {
        let mut m = shift_mode(sig, 64, sticky, mode, neg);
        let mut b = biased;
        if m >> 64 != 0 {
            m >>= 1;
            b += 1;
        }
        if b >= 0x7FFF {
            return overflow(neg, mode);
        }
        F80::from_parts(neg, b as u16, m as u64)
    } else {
        // Denormal: unit 2^(1-BIAS-63); shift = 64 + (1 - biased).
        let k = (1 - biased) as u32 + 64;
        let m = shift_mode(sig, k, sticky, mode, neg);
        let e = u16::from(m >> 63 != 0);
        F80::from_parts(neg, e, m as u64)
    }
}

fn overflow(neg: bool, mode: Round) -> F80 {
    let to_inf = match mode {
        Round::Nearest => true,
        Round::Zero => false,
        Round::Up => !neg,
        Round::Down => neg,
    };
    if to_inf {
        F80::INFINITY.with_sign(neg)
    } else {
        F80::MAX.with_sign(neg)
    }
}

pub(crate) fn zero(neg: bool) -> F80 {
    F80::ZERO.with_sign(neg)
}

pub(crate) fn quiet(x: F80) -> F80 {
    F80 { mant: x.mant | QUIET_BIT, se: x.se }
}

/// NaN / unsupported handling shared by two-operand instructions. `a` is the
/// destination operand. Returns `None` when both operands are ordinary values.
pub(crate) fn special2(a: F80, b: F80) -> Option<F80> {
    let ca = a.class();
    let cb = b.class();
    if ca == Class::Unsupported || cb == Class::Unsupported {
        return Some(F80::INDEFINITE);
    }
    let na = matches!(ca, Class::QuietNaN | Class::SignalingNaN);
    let nb = matches!(cb, Class::QuietNaN | Class::SignalingNaN);
    match (na, nb) {
        (false, false) => None,
        (true, false) => Some(quiet(a)),
        (false, true) => Some(quiet(b)),
        (true, true) => Some(pick_nan(a, b)),
    }
}

/// Both operands are NaNs (x87 rule): a quiet NaN beats a signalling one;
/// between two of the same kind the larger significand wins.
pub(crate) fn pick_nan(a: F80, b: F80) -> F80 {
    let qa_in = a.mant & QUIET_BIT != 0;
    let qb_in = b.mant & QUIET_BIT != 0;
    if qa_in != qb_in {
        return if qa_in { a } else { b };
    }
    let qa = quiet(a);
    let qb = quiet(b);
    match qa.mant.cmp(&qb.mant) {
        Ordering::Greater => qa,
        Ordering::Less => qb,
        // Equal significands: the positive one, else the destination.
        Ordering::Equal => {
            if qa.sign() && !qb.sign() {
                qb
            } else {
                qa
            }
        }
    }
}

pub(crate) fn special1(a: F80) -> Option<F80> {
    match a.class() {
        Class::Unsupported => Some(F80::INDEFINITE),
        Class::QuietNaN | Class::SignalingNaN => Some(quiet(a)),
        _ => None,
    }
}

#[must_use]
pub fn add(a: F80, b: F80) -> F80 {
    add_signed(a, b, false)
}

#[must_use]
pub fn sub(a: F80, b: F80) -> F80 {
    add_signed(a, b, true)
}

fn add_signed(a: F80, b: F80, negate_b: bool) -> F80 {
    if let Some(r) = special2(a, b) {
        return r;
    }
    let b = if negate_b { b.neg() } else { b };
    let (ca, cb) = (a.class(), b.class());
    match (ca, cb) {
        (Class::Infinity, Class::Infinity) => {
            return if a.sign() == b.sign() { a } else { F80::INDEFINITE };
        }
        (Class::Infinity, _) => return a,
        (_, Class::Infinity) => return b,
        (Class::Zero, Class::Zero) => return zero(a.sign() && b.sign()),
        _ => {}
    }
    if ca == Class::Zero {
        let f = fin(b);
        return round_pack(f.neg, f.exp, f.mant as u128, false);
    }
    if cb == Class::Zero {
        let f = fin(a);
        return round_pack(f.neg, f.exp, f.mant as u128, false);
    }
    let (mut x, mut y) = (fin_norm(a), fin_norm(b));
    if y.exp > x.exp {
        core::mem::swap(&mut x, &mut y);
    }
    // Significands at bit 126; value = s * 2^(exp-63).
    let sx = (x.mant as u128) << 63;
    let d = (x.exp - y.exp) as u32;
    let full = (y.mant as u128) << 63;
    let (mut sy, sticky) = if d >= 127 {
        (0u128, true)
    } else {
        let s = full >> d;
        (s, (s << d) != full)
    };
    if sticky {
        sy |= 1;
    }
    let e = x.exp - 63;
    if x.neg == y.neg {
        round_pack(x.neg, e, sx + sy, false)
    } else if sx > sy {
        round_pack(x.neg, e, sx - sy, false)
    } else if sy > sx {
        round_pack(y.neg, e, sy - sx, false)
    } else {
        zero(false)
    }
}

#[must_use]
pub fn mul(a: F80, b: F80) -> F80 {
    if let Some(r) = special2(a, b) {
        return r;
    }
    let neg = a.sign() != b.sign();
    let (ca, cb) = (a.class(), b.class());
    match (ca, cb) {
        (Class::Infinity, Class::Zero) | (Class::Zero, Class::Infinity) => return F80::INDEFINITE,
        (Class::Infinity, _) | (_, Class::Infinity) => return F80::INFINITY.with_sign(neg),
        (Class::Zero, _) | (_, Class::Zero) => return zero(neg),
        _ => {}
    }
    let (x, y) = (fin(a), fin(b));
    round_pack(neg, x.exp + y.exp, x.mant as u128 * y.mant as u128, false)
}

#[must_use]
pub fn div(a: F80, b: F80) -> F80 {
    if let Some(r) = special2(a, b) {
        return r;
    }
    let neg = a.sign() != b.sign();
    let (ca, cb) = (a.class(), b.class());
    match (ca, cb) {
        (Class::Infinity, Class::Infinity) | (Class::Zero, Class::Zero) => return F80::INDEFINITE,
        (Class::Infinity, _) => return F80::INFINITY.with_sign(neg),
        (_, Class::Infinity) => return zero(neg),
        (_, Class::Zero) => return F80::INFINITY.with_sign(neg),
        (Class::Zero, _) => return zero(neg),
        _ => {}
    }
    let (x, y) = (fin_norm(a), fin_norm(b));
    let n = (x.mant as u128) << 63;
    let d = y.mant as u128;
    let q1 = n / d;
    let r1 = n % d;
    let n2 = r1 << 64;
    let q2 = n2 / d;
    let r2 = n2 % d;
    let q = (q1 << 64) | q2;
    round_pack(neg, x.exp - y.exp - 63 - 64, q, r2 != 0)
}

#[must_use]
pub fn sqrt(a: F80) -> F80 {
    if let Some(r) = special1(a) {
        return r;
    }
    match a.class() {
        Class::Zero => return a,
        Class::Infinity => return if a.sign() { F80::INDEFINITE } else { a },
        _ => {}
    }
    if a.sign() {
        return F80::INDEFINITE;
    }
    let f = fin_norm(a);
    // value = m * 2^e with e even; radicand m * 2^(2K) gives a 66+ bit root.
    let (m, e) = if f.exp & 1 != 0 { ((f.mant as u128) << 1, f.exp - 1) } else { (f.mant as u128, f.exp) };
    const K: u32 = 36;
    // Digit-by-digit square root over the bits of m << 2K.
    let total_bits = 128 + 2 * K; // even
    let mut root: u128 = 0;
    let mut rem: u128 = 0;
    let mut i = total_bits;
    while i >= 2 {
        i -= 2;
        let pair = bits2(m, i, K);
        rem = (rem << 2) | pair;
        let trial = (root << 2) | 1;
        if rem >= trial {
            rem -= trial;
            root = (root << 1) | 1;
        } else {
            root <<= 1;
        }
    }
    round_pack(false, (e - 2 * K as i32) / 2, root, rem != 0)
}

/// Bits `i+1..=i` of `m << 2k` (a 2-bit digit), for the square root.
fn bits2(m: u128, i: u32, k: u32) -> u128 {
    let shift = 2 * k;
    if i + 2 <= shift {
        return 0;
    }
    let pos = i - shift; // bit index into m (i >= shift because shift is even)
    if pos >= 128 {
        0
    } else {
        (m >> pos) & 3
    }
}

/// `frndint`.
#[must_use]
pub fn round_int(a: F80, mode: Round) -> F80 {
    if let Some(r) = special1(a) {
        return r;
    }
    match a.class() {
        Class::Zero | Class::Infinity => return a,
        _ => {}
    }
    let f = fin(a);
    if f.exp >= 0 {
        return round_pack(f.neg, f.exp, f.mant as u128, false);
    }
    let k = (-f.exp) as u32;
    let int = shift_mode(f.mant as u128, k, false, mode, f.neg);
    if int == 0 {
        zero(f.neg)
    } else {
        round_pack(f.neg, 0, int, false)
    }
}

/// `fscale`: `a * 2^trunc(b)`.
#[must_use]
pub fn scale(a: F80, b: F80) -> F80 {
    if let Some(r) = special2(a, b) {
        return r;
    }
    let (ca, cb) = (a.class(), b.class());
    match (ca, cb) {
        (Class::Zero, Class::Infinity) => return if b.sign() { a } else { F80::INDEFINITE },
        (Class::Infinity, Class::Infinity) => return if b.sign() { F80::INDEFINITE } else { a },
        (Class::Zero | Class::Infinity, _) => return a,
        (_, Class::Infinity) => return if b.sign() { zero(a.sign()) } else { F80::INFINITY.with_sign(a.sign()) },
        _ => {}
    }
    let n: i64 = if cb == Class::Zero { 0 } else {
        let fb = fin(b);
        let mag: i64 = if fb.exp >= 0 {
            1 << 40
        } else if fb.exp <= -64 {
            0
        } else {
            (fb.mant >> (-fb.exp) as u32).min(1 << 40) as i64
        };
        if fb.neg {
            -mag
        } else {
            mag
        }
    };
    let f = fin(a);
    let e = (f.exp as i64 + n).clamp(-(1 << 20), 1 << 20) as i32;
    round_pack(f.neg, e, f.mant as u128, false)
}

/// `fucomi`: `None` when unordered.
#[must_use]
pub fn compare(a: F80, b: F80) -> Option<Ordering> {
    let (ca, cb) = (a.class(), b.class());
    let bad = |c| matches!(c, Class::Unsupported | Class::QuietNaN | Class::SignalingNaN);
    if bad(ca) || bad(cb) {
        return None;
    }
    let za = ca == Class::Zero;
    let zb = cb == Class::Zero;
    if za && zb {
        return Some(Ordering::Equal);
    }
    let key = |x: F80, z: bool| -> (i8, i32, u64) {
        if z {
            return (0, 0, 0);
        }
        let s = if x.sign() { -1 } else { 1 };
        if x.class() == Class::Infinity {
            return (s, i32::MAX, 0);
        }
        let f = fin_norm(x);
        (s, f.exp, f.mant)
    };
    let (ka, kb) = (key(a, za), key(b, zb));
    if ka.0 != kb.0 {
        return Some(ka.0.cmp(&kb.0));
    }
    let mag = (ka.1, ka.2).cmp(&(kb.1, kb.2));
    Some(if ka.0 < 0 { mag.reverse() } else { mag })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(v: f64) -> F80 {
        F80::from_f64(v)
    }

    #[test]
    fn basic_arithmetic_is_exact_where_it_should_be() {
        assert_eq!(f(1.5) + f(2.25), f(3.75));
        assert_eq!(f(1.0) - f(1.0), F80::ZERO);
        assert_eq!(f(-0.0) + f(-0.0), F80::NEG_ZERO);
        assert_eq!(f(-0.0) + f(0.0), F80::ZERO);
        assert_eq!(f(3.0) * f(-7.0), f(-21.0));
        assert_eq!(f(1.0) / f(4.0), f(0.25));
        assert_eq!(f(2.0).sqrt(), F80::from_parts(false, 0x3FFF, 0xB504_F333_F9DE_6484));
        assert_eq!(f(16.0).sqrt(), f(4.0));
    }

    #[test]
    fn one_third_rounds_to_nearest() {
        let third = F80::ONE / f(3.0);
        assert_eq!(third, F80::from_parts(false, 0x3FFD, 0xAAAA_AAAA_AAAA_AAAB));
        let tenth = F80::ONE / f(10.0);
        assert_eq!(tenth, F80::from_parts(false, 0x3FFB, 0xCCCC_CCCC_CCCC_CCCD));
    }

    #[test]
    fn invalid_operations_give_indefinite() {
        assert_eq!(F80::INFINITY - F80::INFINITY, F80::INDEFINITE);
        assert_eq!(F80::ZERO * F80::INFINITY, F80::INDEFINITE);
        assert_eq!(F80::ZERO / F80::ZERO, F80::INDEFINITE);
        assert_eq!(f(-1.0).sqrt(), F80::INDEFINITE);
        assert_eq!(f(-0.0).sqrt(), F80::NEG_ZERO);
        assert_eq!(f(1.0) / F80::ZERO, F80::INFINITY);
        let unnormal = F80::from_parts(false, 0x3FFF, 1);
        assert_eq!(unnormal + F80::ONE, F80::INDEFINITE);
    }

    #[test]
    fn overflow_and_denormals() {
        assert_eq!(F80::MAX + F80::MAX, F80::INFINITY);
        assert_eq!(F80::MIN_POSITIVE / f(2.0), F80::from_parts(false, 0, 1 << 62));
        assert_eq!(F80::MIN_DENORMAL / f(2.0), F80::ZERO); // tie to even
        assert_eq!(F80::MIN_DENORMAL * f(1.5), F80::from_parts(false, 0, 2));
        let pseudo = F80::from_parts(false, 0, 1 << 63);
        assert_eq!(pseudo + F80::ZERO, F80::MIN_POSITIVE);
    }

    #[test]
    fn nan_propagation() {
        let snan = F80::from_parts(false, 0x7FFF, (1 << 63) | 5);
        assert_eq!(snan + F80::ONE, F80::from_parts(false, 0x7FFF, (1 << 63) | (1 << 62) | 5));
        let big = F80::from_parts(true, 0x7FFF, u64::MAX);
        assert_eq!(F80::NAN * big, big);
    }

    #[test]
    fn rounding_and_scaling() {
        assert_eq!(f(2.5).round_int(Round::Nearest), f(2.0));
        assert_eq!(f(-2.5).round_int(Round::Down), f(-3.0));
        assert_eq!(f(-0.2).round_int(Round::Zero), F80::NEG_ZERO);
        assert_eq!(f(3.0).scale(f(2.9)), f(12.0));
        assert_eq!(f(3.0).scale(f(-1.0)), f(1.5));
        assert!(f(1.0) < f(2.0));
        assert_eq!(F80::NAN.partial_cmp(&F80::ONE), None);
        assert_eq!(F80::ZERO.partial_cmp(&F80::NEG_ZERO), Some(Ordering::Equal));
    }
}
