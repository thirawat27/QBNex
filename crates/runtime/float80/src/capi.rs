//! C ABI over 10-byte little-endian buffers (the `long double` layout), for code
//! that cannot use the Rust API (Cranelift JIT, C callers). Pointers may be
//! unaligned; `out` may alias an input. Functions that can raise a BASIC error
//! return it (0 = success) and then write 0 to `out`.

use crate::{dec, libm, qb, Round, F80};

/// # Safety
/// `p` readable for 10 bytes.
unsafe fn rd(p: *const u8) -> F80 {
    // SAFETY: caller contract.
    unsafe { F80::read(p) }
}

/// # Safety
/// `p` writable for 10 bytes.
unsafe fn wr(p: *mut u8, v: F80) {
    // SAFETY: caller contract.
    unsafe { v.write(p) }
}

macro_rules! binop {
    ($name:ident, $op:tt) => {
        /// # Safety
        /// `a`, `b` readable and `out` writable for 10 bytes.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(a: *const u8, b: *const u8, out: *mut u8) {
            // SAFETY: caller contract.
            unsafe { wr(out, rd(a) $op rd(b)) }
        }
    };
}

binop!(qb_f80_add, +);
binop!(qb_f80_sub, -);
binop!(qb_f80_mul, *);
binop!(qb_f80_div, /);

/// # Safety
/// `a` readable and `out` writable for 10 bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_f80_neg(a: *const u8, out: *mut u8) {
    // SAFETY: caller contract.
    unsafe { wr(out, -rd(a)) }
}

/// `fucomi`: -1 less, 0 equal, 1 greater, 2 unordered.
///
/// # Safety
/// `a`, `b` readable for 10 bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_f80_cmp(a: *const u8, b: *const u8) -> i32 {
    // SAFETY: caller contract.
    match unsafe { rd(a).compare(rd(b)) } {
        Some(core::cmp::Ordering::Less) => -1,
        Some(core::cmp::Ordering::Equal) => 0,
        Some(core::cmp::Ordering::Greater) => 1,
        None => 2,
    }
}

/// # Safety
/// `out` writable for 10 bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_f80_from_i64(v: i64, out: *mut u8) {
    // SAFETY: caller contract.
    unsafe { wr(out, F80::from_i64(v)) }
}

/// # Safety
/// `out` writable for 10 bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_f80_from_f64(v: f64, out: *mut u8) {
    // SAFETY: caller contract.
    unsafe { wr(out, F80::from_f64(v)) }
}

/// # Safety
/// `a` readable for 10 bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_f80_to_f64(a: *const u8) -> f64 {
    // SAFETY: caller contract.
    unsafe { rd(a) }.to_f64()
}

/// C cast `(int64_t)x` (`fisttp qword`).
///
/// # Safety
/// `a` readable for 10 bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_f80_to_i64_trunc(a: *const u8) -> i64 {
    // SAFETY: caller contract.
    unsafe { rd(a) }.to_i64(Round::Zero)
}

macro_rules! fallible1 {
    ($name:ident, $f:path) => {
        /// Returns the BASIC error code (0 = ok).
        ///
        /// # Safety
        /// `a` readable and `out` writable for 10 bytes.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(a: *const u8, out: *mut u8) -> i32 {
            // SAFETY: caller contract.
            match $f(unsafe { rd(a) }) {
                Ok(v) => {
                    unsafe { wr(out, v) };
                    0
                }
                Err(e) => {
                    unsafe { wr(out, F80::ZERO) };
                    e
                }
            }
        }
    };
}

fallible1!(qb_f80_exp, qb::exp_float);
fallible1!(qb_f80_log, qb::log);
fallible1!(qb_f80_sqr, qb::sqr);

/// `a ^ b`; returns the BASIC error code (0 = ok).
///
/// # Safety
/// `a`, `b` readable and `out` writable for 10 bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_f80_pow(a: *const u8, b: *const u8, out: *mut u8) -> i32 {
    // SAFETY: caller contract.
    match qb::pow2(unsafe { rd(a) }, unsafe { rd(b) }) {
        Ok(v) => {
            unsafe { wr(out, v) };
            0
        }
        Err(e) => {
            unsafe { wr(out, F80::ZERO) };
            e
        }
    }
}

macro_rules! unary {
    ($name:ident, $f:path) => {
        /// # Safety
        /// `a` readable and `out` writable for 10 bytes.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(a: *const u8, out: *mut u8) {
            // SAFETY: caller contract.
            unsafe { wr(out, $f(rd(a))) }
        }
    };
}

unary!(qb_f80_fix, qb::fix_float);
unary!(qb_f80_int, qb::int_float);
unary!(qb_f80_sin, qb::sin);
unary!(qb_f80_cos, qb::cos);
unary!(qb_f80_tan, qb::tan);
unary!(qb_f80_atn, qb::atn);
unary!(qb_f80_expl, libm::expl);

/// `strtold`: parses `len` bytes, writes the value, returns bytes consumed.
///
/// # Safety
/// `s` readable for `len` bytes (or `len <= 0`), `out` writable for 10 bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_f80_parse(s: *const u8, len: i32, out: *mut u8) -> i32 {
    let bytes: &[u8] = if len <= 0 || s.is_null() {
        &[]
    } else {
        // SAFETY: caller contract.
        unsafe { core::slice::from_raw_parts(s, len as usize) }
    };
    let (v, n) = dec::parse(bytes);
    // SAFETY: caller contract.
    unsafe { wr(out, v) };
    n as i32
}

/// `snprintf(buf, cap, "%.*L<conv>", prec, x)` with `conv` one of `b'e'`,
/// `b'f'`, `b'g'`. Returns the full text length (like `snprintf`); writes at
/// most `cap - 1` bytes plus a NUL when `cap > 0`.
///
/// # Safety
/// `a` readable for 10 bytes, `buf` writable for `cap` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_f80_format(a: *const u8, conv: u8, prec: i32, buf: *mut u8, cap: usize) -> i32 {
    // SAFETY: caller contract.
    let x = unsafe { rd(a) };
    let p = prec.max(0) as usize;
    let s = match conv {
        b'f' => dec::format_f(x, p),
        b'g' => dec::format_g(x, p),
        _ => dec::format_e(x, p),
    };
    if cap > 0 && !buf.is_null() {
        let n = s.len().min(cap - 1);
        // SAFETY: n + 1 <= cap bytes.
        unsafe {
            core::ptr::copy_nonoverlapping(s.as_ptr(), buf, n);
            *buf.add(n) = 0;
        }
    }
    s.len() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_abi_round_trip() {
        let mut a = [0u8; 10];
        let mut b = [0u8; 10];
        let mut out = [0u8; 11];
        unsafe {
            qb_f80_from_i64(7, a.as_mut_ptr());
            qb_f80_from_f64(0.5, b.as_mut_ptr());
            qb_f80_add(a.as_ptr(), b.as_ptr(), out.as_mut_ptr().add(1));
            assert_eq!(qb_f80_to_f64(out.as_ptr().add(1)), 7.5);
            assert_eq!(qb_f80_cmp(a.as_ptr(), b.as_ptr()), 1);
            assert_eq!(qb_f80_pow(b.as_ptr(), a.as_ptr(), out.as_mut_ptr()), 0);
            assert_eq!(qb_f80_to_f64(out.as_ptr()), 0.0078125);
            qb_f80_from_f64(-2.0, a.as_mut_ptr());
            assert_eq!(qb_f80_pow(a.as_ptr(), b.as_ptr(), out.as_mut_ptr()), 5);
            let text = b"  12.5e1xyz";
            assert_eq!(qb_f80_parse(text.as_ptr(), text.len() as i32, out.as_mut_ptr()), 8);
            assert_eq!(qb_f80_to_f64(out.as_ptr()), 125.0);
            let mut buf = [0u8; 32];
            let n = qb_f80_format(out.as_ptr(), b'e', 3, buf.as_mut_ptr(), buf.len());
            assert_eq!(&buf[..n as usize], b"1.250e+02");
        }
    }
}

/// Operation codes of [`qb_f80_diff_op`].
#[cfg(feature = "diff-hooks")]
pub mod diff_ops {
    pub const ADD: u32 = 0;
    pub const SUB: u32 = 1;
    pub const MUL: u32 = 2;
    pub const DIV: u32 = 3;
    pub const SQRT: u32 = 4;
    /// frndint, `i` = rounding control 0..=3.
    pub const RNDINT: u32 = 5;
    pub const SCALE: u32 = 6;
    /// returns -1/0/1/2.
    pub const CMP: u32 = 7;
    /// `i` = f64 bits -> out.
    pub const FROM_F64: u32 = 8;
    /// `i` = f32 bits -> out.
    pub const FROM_F32: u32 = 9;
    /// returns f64 bits.
    pub const TO_F64: u32 = 10;
    /// returns f32 bits.
    pub const TO_F32: u32 = 11;
    pub const FROM_I64: u32 = 12;
    pub const FROM_U64: u32 = 13;
    /// `i` = width (16/32/64) + 1000 * rounding control; returns the integer.
    pub const TO_INT: u32 = 14;
    pub const TO_U64: u32 = 15;
    pub const EXPL: u32 = 16;
    pub const EXP2L: u32 = 17;
    pub const LOG2L: u32 = 18;
    pub const POWL: u32 = 19;
    /// `i` = exponent.
    pub const POWIL: u32 = 20;
    /// `i` = exponent.
    pub const LDEXPL: u32 = 21;
    pub const FLOORL: u32 = 22;
    pub const CEILL: u32 = 23;
    /// returns BASIC error.
    pub const QB_EXP: u32 = 24;
    /// returns BASIC error.
    pub const QB_POW: u32 = 25;
    pub const QB_FIX: u32 = 26;
    pub const QB_QBR: u32 = 27;
    pub const SOFT_F2XM1: u32 = 28;
    pub const SOFT_FYL2X: u32 = 29;
    pub const SOFT_FYL2XP1: u32 = 30;
    pub const F2XM1: u32 = 31;
    pub const FYL2X: u32 = 32;
    pub const FYL2XP1: u32 = 33;
}

/// Runs one operation for the differential harness. `a`/`b` are inputs, `out`
/// receives an `F80` result; integer results are returned.
///
/// # Safety
/// `a`, `b` readable and `out` writable for 10 bytes.
#[cfg(feature = "diff-hooks")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_f80_diff_op(op: u32, a: *const u8, b: *const u8, i: i64, out: *mut u8) -> i64 {
    use diff_ops::*;
    // SAFETY: caller contract.
    let (x, y) = unsafe { (rd(a), rd(b)) };
    let rc = |v: i64| match v & 3 {
        0 => Round::Nearest,
        1 => Round::Down,
        2 => Round::Up,
        _ => Round::Zero,
    };
    let mut ret = 0i64;
    let v: F80 = match op {
        ADD => x + y,
        SUB => x - y,
        MUL => x * y,
        DIV => x / y,
        SQRT => x.sqrt(),
        RNDINT => x.round_int(rc(i)),
        SCALE => x.scale(y),
        CMP => {
            ret = match x.compare(y) {
                Some(core::cmp::Ordering::Less) => -1,
                Some(core::cmp::Ordering::Equal) => 0,
                Some(core::cmp::Ordering::Greater) => 1,
                None => 2,
            };
            F80::ZERO
        }
        FROM_F64 => F80::from_f64(f64::from_bits(i as u64)),
        FROM_F32 => F80::from_f32(f32::from_bits(i as u32)),
        TO_F64 => {
            ret = x.to_f64().to_bits() as i64;
            F80::ZERO
        }
        TO_F32 => {
            ret = x.to_f32().to_bits() as i64;
            F80::ZERO
        }
        FROM_I64 => F80::from_i64(i),
        FROM_U64 => F80::from_u64(i as u64),
        TO_INT => {
            let mode = rc(i / 1000);
            ret = match i % 1000 {
                16 => x.to_i16(mode) as i64,
                32 => x.to_i32(mode) as i64,
                _ => x.to_i64(mode),
            };
            F80::ZERO
        }
        TO_U64 => {
            ret = x.to_u64_cast() as i64;
            F80::ZERO
        }
        EXPL => libm::expl(x),
        EXP2L => libm::exp2l(x),
        LOG2L => libm::log2l(x),
        POWL => libm::powl(x, y),
        POWIL => libm::powil(x, i as i32),
        LDEXPL => libm::ldexpl(x, i as i32),
        FLOORL => libm::floorl(x),
        CEILL => libm::ceill(x),
        QB_EXP => match qb::exp_float(x) {
            Ok(v) => v,
            Err(e) => {
                ret = e as i64;
                F80::ZERO
            }
        },
        QB_POW => match qb::pow2(x, y) {
            Ok(v) => v,
            Err(e) => {
                ret = e as i64;
                F80::ZERO
            }
        },
        QB_FIX => qb::fix_float(x),
        QB_QBR => {
            ret = qb::qbr(x);
            F80::ZERO
        }
        SOFT_F2XM1 => crate::soft::f2xm1(x),
        SOFT_FYL2X => crate::soft::fyl2x(x, y),
        SOFT_FYL2XP1 => crate::soft::fyl2xp1(x, y),
        F2XM1 => crate::x87::f2xm1(x),
        FYL2X => crate::x87::fyl2x(x, y),
        FYL2XP1 => crate::x87::fyl2xp1(x, y),
        _ => F80::INDEFINITE,
    };
    // SAFETY: caller contract.
    unsafe { wr(out, v) };
    ret
}
