//! QBasic-level `_FLOAT` functions with the legacy semantics and error codes
//! (`Err(code)` is the BASIC error the legacy runtime raised; the value is then 0).
//!
//! Which C function the legacy build really called matters:
//!
//! | BASIC | legacy runtime | here |
//! | --- | --- | --- |
//! | `EXP(x##)` | `func_exp_float`: `expl` if `x <= 709.782712893` else error 6 | [`exp_float`] |
//! | `a ^ b` (every numeric type) | `pow2(long double, long double)`: error 5 for `a < 0` and fractional `b`, else `powl` | [`pow2`] |
//! | `FIX(x##)` | `func_fix_float`: `ceill`/`floorl` | [`fix_float`] |
//! | `INT(x##)` | `floorl` | [`int_float`] |
//! | `SIN`/`COS`/`TAN`/`ATN` | the **double** C functions (the `long double` overloads were never selected) | [`sin`] .. [`atn`] |
//! | `LOG`, `SQR` | `func_log(double)`, `func_sqr(double)` | [`log`], [`sqr`] |
//! | `STR$`/`PRINT` of `_FLOAT` | `qbs_str((double)value)` | `qb-runtime` `numfmt::str_f80` |
//!
//! The double functions use the platform `libm` through Rust `f64` methods; on
//! Windows that is the same UCRT the legacy build linked.

use crate::libm::{ceill, expl, floorl, powl};
use crate::{Round, F80};
use core::cmp::Ordering;

/// BASIC error "Illegal function call".
pub const ERR_ILLEGAL_FUNCTION_CALL: i32 = 5;
/// BASIC error "Overflow".
pub const ERR_OVERFLOW: i32 = 6;

/// `EXP` of a `_FLOAT` (`func_exp_float`). The bound is the double literal
/// `709.782712893` widened to long double; NaN fails the test (error 6).
///
/// # Errors
/// 6 (overflow) above the bound or for NaN.
pub fn exp_float(x: F80) -> Result<F80, i32> {
    let limit = F80::from_f64(709.782712893);
    match x.compare(limit) {
        Some(Ordering::Less | Ordering::Equal) => Ok(expl(x)),
        _ => Err(ERR_OVERFLOW),
    }
}

/// The `^` operator (`pow2`).
///
/// # Errors
/// 5 (illegal function call) for a negative base with a non-integer exponent.
pub fn pow2(x: F80, y: F80) -> Result<F80, i32> {
    if F80::ZERO.compare(x) == Some(Ordering::Greater) && y.compare(floorl(y)) != Some(Ordering::Equal) {
        return Err(ERR_ILLEGAL_FUNCTION_CALL);
    }
    Ok(powl(x, y))
}

/// `FIX` of a `_FLOAT`.
#[must_use]
pub fn fix_float(x: F80) -> F80 {
    if F80::ZERO.compare(x) == Some(Ordering::Greater) {
        ceill(x)
    } else {
        floorl(x)
    }
}

/// `INT` of a `_FLOAT` (`floorl`).
#[must_use]
pub fn int_float(x: F80) -> F80 {
    floorl(x)
}

/// `CINT`/`CLNG`-style rounding of `_FLOAT` (`qbr`: `fistp` round to nearest even
/// with the legacy wrap for values in `[2^63, 2^64)`).
#[must_use]
pub fn qbr(x: F80) -> i64 {
    // if (f > 9223372036854775807) { temp = 1; f = f - 9223372036854775808u; }
    let max = F80::from_i64(i64::MAX);
    if x.compare(max) == Some(Ordering::Greater) {
        let two63 = F80::from_u64(1 << 63);
        return (x - two63).to_i64(Round::Nearest) | i64::MIN;
    }
    x.to_i64(Round::Nearest)
}

fn via_f64(x: F80, f: fn(f64) -> f64) -> F80 {
    F80::from_f64(f(x.to_f64()))
}

pub fn sin(x: F80) -> F80 {
    via_f64(x, f64::sin)
}

pub fn cos(x: F80) -> F80 {
    via_f64(x, f64::cos)
}

pub fn tan(x: F80) -> F80 {
    via_f64(x, f64::tan)
}

pub fn atn(x: F80) -> F80 {
    via_f64(x, f64::atan)
}

/// `func_log(double)`.
///
/// # Errors
/// 5 for values `<= 0`.
pub fn log(x: F80) -> Result<F80, i32> {
    let v = x.to_f64();
    if v <= 0.0 {
        return Err(ERR_ILLEGAL_FUNCTION_CALL);
    }
    Ok(F80::from_f64(v.ln()))
}

/// `func_sqr(double)`.
///
/// # Errors
/// 5 for negative values.
pub fn sqr(x: F80) -> Result<F80, i32> {
    let v = x.to_f64();
    if v < 0.0 {
        return Err(ERR_ILLEGAL_FUNCTION_CALL);
    }
    Ok(F80::from_f64(v.sqrt()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(v: f64) -> F80 {
        F80::from_f64(v)
    }

    #[test]
    fn legacy_error_codes() {
        assert_eq!(exp_float(f(710.0)), Err(6));
        assert_eq!(exp_float(F80::NAN), Err(6));
        assert!(exp_float(f(709.0)).is_ok());
        assert_eq!(pow2(f(-8.0), f(0.5)), Err(5));
        assert_eq!(pow2(f(-2.0), f(3.0)), Ok(f(-8.0)));
        assert_eq!(pow2(f(-2.0), F80::NAN), Err(5));
        assert_eq!(log(f(0.0)), Err(5));
        assert_eq!(sqr(f(-1.0)), Err(5));
        assert_eq!(sqr(f(16.0)), Ok(f(4.0)));
    }

    #[test]
    fn fix_int_round() {
        assert_eq!(fix_float(f(-2.7)), f(-2.0));
        assert_eq!(fix_float(f(2.7)), f(2.0));
        assert_eq!(int_float(f(-2.1)), f(-3.0));
        assert_eq!(qbr(f(2.5)), 2);
        assert_eq!(qbr(f(-3.5)), -4);
        assert_eq!(qbr(F80::from_u64(u64::MAX)), -1);
    }
}
