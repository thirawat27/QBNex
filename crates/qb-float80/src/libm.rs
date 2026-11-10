//! MinGW-w64 (v13, `x86_64`) `long double` math functions, transcribed instruction
//! by instruction from `libmingwex.a` (sources: `math/x86/exp.def.h`,
//! `exp2l.S`, `log2l.S`, `pow.def.h`, `powi.def.h`, `ldexpl.c`, `floorl.S`,
//! `ceill.S`). The legacy runtime reaches `expl` through `EXP` of a `_FLOAT`
//! and `powl` through every `^` (all operands are promoted to `long double`).
//!
//! `errno` and `_matherr` side effects are not modelled; they never reach BASIC.

use crate::arith::{add, div, mul, round_int, scale, sqrt, sub};
use crate::x87::{f2xm1, fyl2x, fyl2xp1};
use crate::{Class, MingwClass, Round, F80};
use core::cmp::Ordering;

/// `__FLT_MAXLOG` for long double (`.rdata` of `expl.o`).
const MAXLOG: F80 = F80::from_parts(false, 0x400C, 0xB172_17F7_D1CF_79AC);
/// `__FLT_MINLOG` for long double.
const MINLOG: F80 = F80::from_parts(true, 0x400C, 0xB16C_8C67_1210_EB30);
/// `c0 = 1.44268798828125L`.
const C0: F80 = F80::from_parts(false, 0x3FFF, 0xB8AA_0000_0000_0000);
/// `c1 = 7.05260771340735992468e-6L`.
const C1: F80 = F80::from_parts(false, 0x3FED, 0xECA5_705F_C2EE_FA20);
/// `fldl2e` with round to nearest.
const L2E: F80 = F80::from_parts(false, 0x3FFF, 0xB8AA_3B29_5C17_F0BC);
/// `limit` in `log2l.S`: the double 1 - sqrt(2)/2.
const LOG2_LIMIT: f64 = 0.29289321881345247561810596348408353;

fn gt(a: F80, b: F80) -> bool {
    a.compare(b) == Some(Ordering::Greater)
}

fn eq(a: F80, b: F80) -> bool {
    a.compare(b) == Some(Ordering::Equal)
}

/// `floorl`.
#[must_use]
pub fn floorl(x: F80) -> F80 {
    round_int(x, Round::Down)
}

/// `ceill`.
#[must_use]
pub fn ceill(x: F80) -> F80 {
    round_int(x, Round::Up)
}

/// `ldexpl(x, n)`.
#[must_use]
pub fn ldexpl(x: F80, n: i32) -> F80 {
    if x.exp() == 0x7FFF || eq(x, F80::ZERO) {
        return x;
    }
    scale(x, F80::from_i32(n))
}

/// `internal_modf` of `pow.def.h`: fractional part, integer part through `iptr`.
fn internal_modf(value: F80, iptr: &mut F80) -> F80 {
    let int_part = round_int(value, Round::Zero);
    *iptr = int_part;
    let is_inf = value.exp() == 0x7FFF && value.mant << 1 == 0;
    if is_inf {
        F80::ZERO
    } else {
        sub(value, int_part)
    }
}

/// `__expl_internal`: the x87 sequence of `exp.def.h`.
fn expl_internal(x: F80) -> F80 {
    let t = mul(L2E, x); // fldl2e; fmul st, st(1)
    let i = round_int(t, Round::Zero); // frndint (chop)
    let xi = round_int(x, Round::Zero); // fld st(1); frndint
    let f = sub(mul(C0, xi), i); // c0*xi - i
    let xf = sub(x, xi); // x - xi
    let f = add(f, mul(C0, xf)); // f + c0*xf
    let f = add(f, mul(C1, x)); // f + c1*x
    let p = add(f2xm1(f), F80::ONE); // 2^f
    scale(p, i)
}

/// `expl`.
#[must_use]
pub fn expl(x: F80) -> F80 {
    match x.mingw_class() {
        MingwClass::NaN => x,
        MingwClass::Infinite => {
            if x.sign() {
                F80::ZERO
            } else {
                F80::INFINITY
            }
        }
        MingwClass::Zero => F80::ONE,
        _ => {
            if gt(x, MAXLOG) {
                F80::INFINITY
            } else if gt(MINLOG, x) {
                F80::ZERO
            } else {
                expl_internal(x)
            }
        }
    }
}

/// `exp2l` (`exp2l.S`).
#[must_use]
pub fn exp2l(x: F80) -> F80 {
    if x.class() == Class::Infinity {
        return if x.sign() { F80::ZERO } else { x };
    }
    let i = round_int(x, Round::Zero);
    let frac = sub(x, i);
    let p = add(f2xm1(frac), F80::ONE);
    scale(p, i)
}

/// `log2l` (`log2l.S`).
#[must_use]
pub fn log2l(x: F80) -> F80 {
    // fxam: C0 is set for NaN (and empty); NaN is returned unchanged.
    if x.is_nan() {
        return x;
    }
    let t = sub(x, F80::ONE);
    let limit = F80::from_f64(LOG2_LIMIT);
    let small = match t.abs().compare(limit) {
        Some(Ordering::Greater) => false,
        _ => true, // less, equal or unordered
    };
    if small {
        fyl2xp1(t, F80::ONE)
    } else {
        fyl2x(x, F80::ONE)
    }
}

fn do_powi_iter(mut d: F80, y: i32) -> F80 {
    let mut u = y as u32;
    let mut rslt = if u & 1 != 0 { d } else { F80::ONE };
    u >>= 1;
    loop {
        d = mul(d, d);
        if u & 1 != 0 {
            rslt = mul(rslt, d);
        }
        u >>= 1;
        if u == 0 {
            break;
        }
    }
    rslt
}

/// `__powil(x, y)`.
#[must_use]
pub fn powil(x: F80, mut y: i32) -> F80 {
    let x_class = x.mingw_class();
    let odd_y = y & 1 != 0;
    if y == 0 || eq(x, F80::ONE) {
        return F80::ONE;
    }
    match x_class {
        MingwClass::NaN => return if x.sign() { F80::NAN.neg() } else { F80::NAN },
        MingwClass::Zero => {
            if y >= 0 {
                return if !odd_y || !x.sign() { F80::ZERO } else { F80::NEG_ZERO };
            }
            if !odd_y || !x.sign() {
                return F80::INFINITY;
            }
            return if x.sign() { F80::NEG_INFINITY } else { F80::INFINITY };
        }
        MingwClass::Infinite => {
            let s = x.sign();
            if s && y < 0 && !odd_y {
                return F80::ZERO;
            }
            if s && y >= 0 && odd_y {
                return F80::NEG_INFINITY;
            }
            if s && y >= 0 && !odd_y {
                return F80::INFINITY;
            }
            if y < 0 {
                return if odd_y && s { F80::NEG_ZERO } else { F80::ZERO };
            }
            return if odd_y && s { F80::NEG_INFINITY } else { F80::INFINITY };
        }
        _ => {}
    }
    let mut d = x.abs();
    let mut recip = false;
    if y < 0 {
        recip = true;
        y = y.wrapping_neg();
    }
    let mut rslt = if y == 0 {
        F80::ONE
    } else if y == 1 {
        d
    } else {
        let mut r = do_powi_iter(d, y);
        if recip && r.mingw_class() == MingwClass::Infinite && gt(d, F80::ONE) {
            d = div(F80::ONE, d);
            recip = false;
            r = do_powi_iter(d, y);
        }
        r
    };
    if recip {
        rslt = div(F80::ONE, rslt);
    }
    if x.sign() && odd_y {
        rslt = rslt.neg();
    }
    rslt
}

/// `powl(x, y)`.
#[must_use]
pub fn powl(x: F80, y: F80) -> F80 {
    let x_class = x.mingw_class();
    let y_class = y.mingw_class();
    let mut d = F80::ZERO;
    if y_class == MingwClass::Zero || eq(x, F80::ONE) {
        return F80::ONE;
    }
    if x_class == MingwClass::NaN {
        return x;
    }
    if y_class == MingwClass::NaN {
        return y;
    }
    let half_odd = |y: F80, d: &mut F80| !eq(internal_modf(ldexpl(y, -1), d), F80::ZERO);
    if x_class == MingwClass::Zero {
        if y_class == MingwClass::Infinite {
            return if y.sign() { F80::INFINITY } else { F80::ZERO };
        }
        if x.sign() && !eq(internal_modf(y, &mut d), F80::ZERO) {
            return if y.sign() { div(F80::ONE, x.neg()) } else { F80::ZERO };
        }
        let odd_y = half_odd(y, &mut d);
        if !y.sign() {
            return if !odd_y || !x.sign() { F80::ZERO } else { F80::NEG_ZERO };
        }
        if !odd_y || !x.sign() {
            return F80::INFINITY;
        }
        return if x.sign() { F80::NEG_INFINITY } else { F80::INFINITY };
    }
    if y_class == MingwClass::Infinite {
        if x_class == MingwClass::Infinite {
            return if y.sign() { F80::ZERO } else { F80::INFINITY };
        }
        let a_x = x.abs();
        if eq(a_x, F80::ONE) {
            return F80::ONE;
        }
        if gt(a_x, F80::ONE) {
            return if y.sign() { F80::ZERO } else { F80::INFINITY };
        }
        return if y.sign() { F80::INFINITY } else { F80::ZERO };
    }
    if x_class == MingwClass::Infinite {
        if x.sign() && !eq(internal_modf(y, &mut d), F80::ZERO) {
            return if y.sign() { div(F80::ONE, x.neg()) } else { x.neg() };
        }
        let odd_y = half_odd(y, &mut d);
        let (sx, sy) = (x.sign(), y.sign());
        if sx && sy && !odd_y {
            return F80::ZERO;
        }
        if sx && !sy && odd_y {
            return F80::NEG_INFINITY;
        }
        if sx && !sy && !odd_y {
            return F80::INFINITY;
        }
        if sy {
            return if odd_y && sx { F80::NEG_ZERO } else { F80::ZERO };
        }
        return if odd_y && sx { F80::NEG_INFINITY } else { F80::INFINITY };
    }
    if eq(internal_modf(y, &mut d), F80::ZERO) {
        let int_max = F80::from_i32(i32::MAX);
        let int_min = F80::from_i32(i32::MIN);
        let le = |a: F80, b: F80| matches!(a.compare(b), Some(Ordering::Less | Ordering::Equal));
        if le(d, int_max) && le(int_min, d) {
            return powil(x, y.to_i32(Round::Zero));
        }
    } else {
        if x.sign() {
            return F80::INDEFINITE; // -__FLT_NAN
        }
        if eq(y, F80::from_f64(0.5)) {
            return sqrt(x);
        }
    }
    let mut rslt = exp2l(mul(y, log2l(x.abs())));
    if x.sign() && half_odd(y, &mut d) {
        rslt = rslt.neg();
    }
    rslt
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(v: f64) -> F80 {
        F80::from_f64(v)
    }

    #[test]
    fn integer_powers_are_exact_products() {
        assert_eq!(powl(f(2.0), f(10.0)), f(1024.0));
        assert_eq!(powl(f(-3.0), f(3.0)), f(-27.0));
        assert_eq!(powl(f(2.0), f(-2.0)), f(0.25));
        assert_eq!(powl(f(10.0), f(-1.0)), div(F80::ONE, f(10.0)));
        assert_eq!(powl(f(0.0), f(-1.0)), F80::INFINITY);
        assert_eq!(powl(f(-0.0), f(-3.0)), F80::NEG_INFINITY);
        assert_eq!(powl(f(-8.0), f(1.0 / 3.0)), F80::INDEFINITE);
        assert_eq!(powl(f(9.0), f(0.5)), f(3.0));
        assert_eq!(powl(F80::NAN, f(0.0)), F80::ONE);
        assert_eq!(powl(f(1.0), F80::NAN), F80::ONE);
    }

    #[test]
    fn exp_and_logs() {
        assert_eq!(expl(F80::ZERO), F80::ONE);
        assert_eq!(expl(f(20000.0)), F80::INFINITY);
        assert_eq!(expl(f(-20000.0)), F80::ZERO);
        assert_eq!(expl(F80::NEG_INFINITY), F80::ZERO);
        let e = expl(F80::ONE);
        assert_eq!(e.to_f64(), core::f64::consts::E);
        assert_eq!(exp2l(f(10.0)), f(1024.0));
        assert_eq!(exp2l(F80::NEG_INFINITY), F80::ZERO);
        assert_eq!(log2l(f(1024.0)), f(10.0));
        assert_eq!(log2l(F80::ONE), F80::ZERO);
        assert!((powl(f(2.0), f(0.5)).to_f64() - core::f64::consts::SQRT_2).abs() < 1e-15);
        assert!((powl(f(10.0), f(2.5)).to_f64() - 316.22776601683796).abs() < 1e-12);
    }

    #[test]
    fn floor_ceil_ldexp() {
        assert_eq!(floorl(f(-2.5)), f(-3.0));
        assert_eq!(ceill(f(-2.5)), f(-2.0));
        assert_eq!(ceill(f(-0.5)), F80::NEG_ZERO);
        assert_eq!(ldexpl(f(3.0), -1), f(1.5));
        assert_eq!(ldexpl(F80::INFINITY, -1), F80::INFINITY);
    }

    #[cfg(all(any(target_arch = "x86", target_arch = "x86_64"), not(feature = "soft-transcendentals")))]
    #[test]
    fn l2e_matches_fldl2e() {
        assert_eq!(L2E, crate::x87::hw::fldl2e());
    }
}
