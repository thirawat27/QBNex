//! Software versions of `f2xm1`, `fyl2x`, `fyl2xp1`: about 120 correct bits,
//! then rounded to nearest even. They give the correctly rounded result except in
//! rare near-tie cases; the x87 unit itself is not correctly rounded, so on
//! x86 the hardware is used instead (see [`crate::x87`]).

use crate::arith::{fin_norm, pick_nan, quiet, round_pack, zero};
use crate::{Class, F80};
use core::cmp::Ordering;

/// `(-1)^neg * m * 2^exp`, `m` normalised (bit 127 set) or zero.
#[derive(Clone, Copy, Debug)]
pub(crate) struct W {
    neg: bool,
    exp: i32,
    m: u128,
}

impl W {
    const ZERO: W = W { neg: false, exp: 0, m: 0 };

    fn norm(neg: bool, exp: i32, m: u128) -> W {
        if m == 0 {
            return W::ZERO;
        }
        let lz = m.leading_zeros();
        W { neg, exp: exp - lz as i32, m: m << lz }
    }

    fn from_f80(x: F80) -> W {
        if x.is_zero() {
            return W::ZERO;
        }
        let f = fin_norm(x);
        W::norm(f.neg, f.exp - 64, (f.mant as u128) << 64)
    }

    fn int(v: i64) -> W {
        W::norm(v < 0, 0, v.unsigned_abs() as u128)
    }

    fn to_f80(self) -> F80 {
        if self.m == 0 {
            return F80::ZERO;
        }
        round_pack(self.neg, self.exp, self.m, false)
    }

    fn neg(self) -> W {
        W { neg: !self.neg, ..self }
    }

    fn mul(self, o: W) -> W {
        if self.m == 0 || o.m == 0 {
            return W::ZERO;
        }
        let (hi, _) = mul_128(self.m, o.m);
        W::norm(self.neg != o.neg, self.exp + o.exp + 128, hi)
    }

    fn add(self, o: W) -> W {
        if self.m == 0 {
            return o;
        }
        if o.m == 0 {
            return self;
        }
        let (a, b) = if self.exp >= o.exp { (self, o) } else { (o, self) };
        // One guard bit of headroom: work with m >> 1.
        let d = (a.exp - b.exp) as u32;
        let am = a.m >> 1;
        let bm = if d >= 127 { 0 } else { (b.m >> 1) >> d };
        let e = a.exp + 1;
        if a.neg == b.neg {
            W::norm(a.neg, e, am + bm)
        } else if am >= bm {
            W::norm(a.neg, e, am - bm)
        } else {
            W::norm(b.neg, e, bm - am)
        }
    }

    fn div(self, o: W) -> W {
        if self.m == 0 {
            return W::ZERO;
        }
        // Restoring division of (m1/2) by (m2/2), 127 quotient bits.
        let mut r = self.m >> 1;
        let d = o.m >> 1;
        let mut q: u128 = 0;
        for _ in 0..127 {
            q <<= 1;
            if r >= d {
                r -= d;
                q |= 1;
            }
            r <<= 1;
        }
        W::norm(self.neg != o.neg, self.exp - o.exp - 126, q)
    }

    fn div_small(self, k: u64) -> W {
        if self.m == 0 {
            return W::ZERO;
        }
        let k = k as u128;
        let q = self.m / k;
        let r = self.m % k;
        let lz = q.leading_zeros();
        let q = (q << lz) | ((r << lz) / k);
        W { neg: self.neg, exp: self.exp - lz as i32, m: q }
    }

    fn magnitude_below(self, bits: i32) -> bool {
        self.m == 0 || self.exp + 128 < bits
    }
}

fn mul_128(a: u128, b: u128) -> (u128, u128) {
    let (a1, a0) = (a >> 64, a & u64::MAX as u128);
    let (b1, b0) = (b >> 64, b & u64::MAX as u128);
    let p00 = a0 * b0;
    let p01 = a0 * b1;
    let p10 = a1 * b0;
    let p11 = a1 * b1;
    let mid = (p00 >> 64) + (p01 & u64::MAX as u128) + (p10 & u64::MAX as u128);
    let lo = (p00 & u64::MAX as u128) | (mid << 64);
    let hi = p11 + (p01 >> 64) + (p10 >> 64) + (mid >> 64);
    (hi, lo)
}

/// ln 2 to 128 bits.
const LN2: W = W { neg: false, exp: -128, m: 0xB172_17F7_D1CF_79AB_C9E3_B398_03F2_F6AF };

fn log2e() -> W {
    W::int(1).div(LN2)
}

/// `e^t - 1` for |t| < 0.7.
fn expm1(t: W) -> W {
    let mut sum = t;
    let mut term = t;
    for k in 2..60u64 {
        term = term.mul(t).div_small(k);
        if term.magnitude_below(sum.exp + 128 - 130) {
            break;
        }
        sum = sum.add(term);
    }
    sum
}

/// `2 * atanh(z) = ln((1+z)/(1-z))` for |z| < 0.18.
fn two_atanh(z: W) -> W {
    let z2 = z.mul(z);
    let mut pow = z;
    let mut sum = z;
    for k in 1..80u64 {
        pow = pow.mul(z2);
        let term = pow.div_small(2 * k + 1);
        if term.magnitude_below(sum.exp + 128 - 130) {
            break;
        }
        sum = sum.add(term);
    }
    W { exp: sum.exp + 1, ..sum }
}

/// log2(x) for a finite positive x.
fn log2_w(x: F80) -> W {
    let f = fin_norm(x);
    // x = r * 2^e with r = mant / 2^63 in [1, 2).
    let mut e = f.exp as i64 + 63;
    let mut r = W::norm(false, -63 - 64, (f.mant as u128) << 64);
    // sqrt(2) as a W, compared through the significand of r.
    const SQRT2: u128 = 0xB504_F333_F9DE_6484_597D_89B3_754A_BE9F;
    if r.m > SQRT2 {
        r.exp -= 1;
        e += 1;
    }
    let one = W::int(1);
    let z = r.add(one.neg()).div(r.add(one));
    let ln = two_atanh(z);
    W::int(e).add(ln.mul(log2e()))
}

pub fn f2xm1(x: F80) -> F80 {
    match x.class() {
        Class::Unsupported => F80::INDEFINITE,
        Class::QuietNaN | Class::SignalingNaN => quiet(x),
        Class::Zero => x,
        Class::Infinity => {
            if x.sign() {
                F80::ONE.neg()
            } else {
                x
            }
        }
        _ => expm1(W::from_f80(x).mul(LN2)).to_f80(),
    }
}

pub fn fyl2x(x: F80, y: F80) -> F80 {
    if let Some(r) = nan2(x, y) {
        return r;
    }
    let (cx, cy) = (x.class(), y.class());
    if x.sign() && cx != Class::Zero {
        return F80::INDEFINITE;
    }
    let yinf = cy == Class::Infinity;
    let yzero = cy == Class::Zero;
    if cx == Class::Zero {
        if yzero {
            return F80::INDEFINITE;
        }
        return F80::INFINITY.with_sign(!y.sign());
    }
    if cx == Class::Infinity {
        if yzero {
            return F80::INDEFINITE;
        }
        return F80::INFINITY.with_sign(y.sign());
    }
    let cmp1 = x.compare(F80::ONE);
    if cmp1 == Some(Ordering::Equal) {
        if yinf {
            return F80::INDEFINITE;
        }
        return zero(y.sign());
    }
    let below = cmp1 == Some(Ordering::Less);
    if yinf {
        return F80::INFINITY.with_sign(y.sign() != below);
    }
    if yzero {
        return zero(y.sign() != below);
    }
    log2_w(x).mul(W::from_f80(y)).to_f80()
}

pub fn fyl2xp1(x: F80, y: F80) -> F80 {
    if let Some(r) = nan2(x, y) {
        return r;
    }
    let (cx, cy) = (x.class(), y.class());
    if cx == Class::Zero {
        if cy == Class::Infinity {
            return F80::INDEFINITE;
        }
        return zero(x.sign() != y.sign());
    }
    if cy == Class::Zero {
        return zero(x.sign() != y.sign());
    }
    if cy == Class::Infinity {
        return F80::INFINITY.with_sign(x.sign() != y.sign());
    }
    // ln(1+x) = 2 atanh(x / (2 + x)).
    let wx = W::from_f80(x);
    let z = wx.div(W::int(2).add(wx));
    two_atanh(z).mul(log2e()).mul(W::from_f80(y)).to_f80()
}

fn nan2(x: F80, y: F80) -> Option<F80> {
    let (cx, cy) = (x.class(), y.class());
    if cx == Class::Unsupported || cy == Class::Unsupported {
        return Some(F80::INDEFINITE);
    }
    let nx = matches!(cx, Class::QuietNaN | Class::SignalingNaN);
    let ny = matches!(cy, Class::QuietNaN | Class::SignalingNaN);
    match (nx, ny) {
        (false, false) => None,
        (true, false) => Some(quiet(x)),
        (false, true) => Some(quiet(y)),
        (true, true) => Some(pick_nan(y, x)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    fn ulps(a: F80, b: F80) -> u64 {
        let k = |v: F80| -> i128 {
            let mag = ((v.exp() as i128) << 64) | v.mant as i128;
            if v.sign() {
                -mag
            } else {
                mag
            }
        };
        (k(a) - k(b)).unsigned_abs() as u64
    }

    #[test]
    fn constants_are_right() {
        // fldl2e / fldln2 values (64-bit, nearest).
        assert_eq!(log2e().to_f80(), F80::from_parts(false, 0x3FFF, 0xB8AA_3B29_5C17_F0BC));
        assert_eq!(LN2.to_f80(), F80::from_parts(false, 0x3FFE, 0xB172_17F7_D1CF_79AC));
        assert_eq!(f2xm1(F80::ONE), F80::ONE);
        assert_eq!(fyl2x(F80::from_f64(8.0), F80::ONE), F80::from_f64(3.0));
        assert_eq!(fyl2x(F80::from_f64(0.25), F80::from_f64(-1.5)), F80::from_f64(3.0));
    }

    #[test]
    fn specials() {
        assert_eq!(f2xm1(F80::NEG_INFINITY), F80::ONE.neg());
        assert_eq!(fyl2x(F80::ZERO, F80::ONE), F80::NEG_INFINITY);
        assert_eq!(fyl2x(F80::ONE.neg(), F80::ONE), F80::INDEFINITE);
        assert_eq!(fyl2x(F80::ONE, F80::from_f64(-3.0)), F80::NEG_ZERO);
        assert_eq!(fyl2xp1(F80::NEG_ZERO, F80::ONE), F80::NEG_ZERO);
    }

    /// The software primitives agree with the x87 unit to one unit in the last
    /// place, and exactly on most inputs.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[test]
    fn within_one_ulp_of_hardware() {
        use crate::x87::hw;
        let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s
        };
        let (mut exact, n) = (0, 20_000);
        for i in 0..n {
            let r = next();
            let x = F80::from_parts(r & 1 == 1, 0x3FFE - (i % 40) as u16, next() | (1 << 63));
            let a = f2xm1(x);
            let b = hw::f2xm1(x);
            assert!(ulps(a, b) <= 1, "f2xm1 {x:?}: {a:?} vs {b:?}");
            let p = F80::from_parts(false, 0x3FFF - 40 + (r % 80) as u16, next() | (1 << 63));
            let c = fyl2x(p, F80::ONE);
            let d = hw::fyl2x(p, F80::ONE);
            assert!(ulps(c, d) <= 1, "fyl2x {p:?}: {c:?} vs {d:?}");
            let q = F80::from_parts(r & 2 == 2, 0x3FFD - (i % 50) as u16, next() | (1 << 63));
            let e = fyl2xp1(q, F80::ONE);
            let g = hw::fyl2xp1(q, F80::ONE);
            assert!(ulps(e, g) <= 1, "fyl2xp1 {q:?}: {e:?} vs {g:?}");
            exact += (a == b) as u32 + (c == d) as u32 + (e == g) as u32;
        }
        assert!(exact as f64 / (3 * n) as f64 > 0.9, "exact {exact}/{}", 3 * n);
    }
}
