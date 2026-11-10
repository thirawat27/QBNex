//! Software x87 80-bit extended precision float (`_FLOAT` in `QBNex`).
//!
//! The legacy runtime stored `_FLOAT` in a C `long double`, which MinGW compiles to
//! the x87 80-bit format with the FPU control word `0x037F` (64-bit significand,
//! round to nearest even, all exceptions masked). [`F80`] reproduces the results of
//! that hardware bit for bit:
//!
//! - arithmetic (`+ - * /`, [`F80::sqrt`], [`F80::round_int`], [`F80::scale`]),
//!   comparisons and conversions are pure software and behave like the x87
//!   instructions `fadd`/`fsub`/`fmul`/`fdiv`/`fsqrt`/`frndint`/`fscale`/`fucomi`,
//!   `fld`/`fst` (f32/f64), `fild`, `fistp`/`fisttp`, including denormals,
//!   pseudo-denormals, unsupported encodings, NaN propagation and the real
//!   indefinite NaN;
//! - [`libm`] transcribes MinGW-w64's `expl`, `exp2l`, `log2l`, `powl`, `__powil`,
//!   `ldexpl`, `floorl`, `ceill` step by step. The only steps whose hardware result
//!   is not the correctly rounded value (`f2xm1`, `fyl2x`, `fyl2xp1`) run on the x87
//!   unit on `x86/x86_64` ([`x87`]); other targets use a software version that is
//!   within one unit in the last place (deviation D-111, see the crate docs);
//! - [`dec`] converts to and from decimal text like MinGW `__mingw_strtold` and
//!   `__mingw_snprintf("%.*Le" / "%.*Lf" / "%.*Lg")`;
//! - [`qb`] has the QBasic-level functions (`EXP`, `^`, `FIX`, `SIN`, ...) with the
//!   legacy error codes.
//!
//! The differential harness `tests/diff/diff_float80.cpp` checks all of this against
//! real `long double` code compiled by MinGW g++.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
// R14 H2: clippy::pedantic is clean except for the lints below. Bit-level x87
// emulation and C ABI glue cast between integer widths on purpose (every cast
// is a deliberate truncation, sign reinterpretation or widening of a field),
// and the one-letter names follow the instruction manuals (a, b, m, e, x, y).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::similar_names,
    // legacy literals and x87 constants are kept digit for digit
    clippy::unreadable_literal,
    // match arms mirror the x87 special-case tables row by row
    clippy::match_same_arms,
    clippy::items_after_statements
)]
pub mod arith;
pub mod capi;
pub mod conv;
pub mod dec;
pub mod libm;
pub mod qb;
pub mod soft;
pub mod x87;

use core::cmp::Ordering;

/// An x87 80-bit extended precision value: 64-bit significand with an explicit
/// integer bit, 15-bit exponent (bias 16383), sign.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct F80 {
    /// Significand, bit 63 is the explicit integer bit.
    pub mant: u64,
    /// Sign (bit 15) and biased exponent (bits 0..15).
    pub se: u16,
}

/// x87 rounding control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Round {
    /// Round to nearest, ties to even (x87 default, RC=00).
    Nearest,
    /// Toward negative infinity (RC=01).
    Down,
    /// Toward positive infinity (RC=10).
    Up,
    /// Toward zero, truncate (RC=11).
    Zero,
}

/// What an encoding means to the x87 arithmetic unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Zero,
    /// Exponent 0 with a non-zero significand (denormal or pseudo-denormal).
    Denormal,
    Normal,
    Infinity,
    QuietNaN,
    SignalingNaN,
    /// Unnormal, pseudo-NaN, pseudo-infinity: an invalid operand for every
    /// arithmetic instruction (the result is the real indefinite).
    Unsupported,
}

pub(crate) const BIAS: i32 = 16383;
pub(crate) const INT_BIT: u64 = 1 << 63;
pub(crate) const QUIET_BIT: u64 = 1 << 62;

impl F80 {
    pub const ZERO: F80 = F80 { mant: 0, se: 0 };
    pub const NEG_ZERO: F80 = F80 { mant: 0, se: 0x8000 };
    pub const ONE: F80 = F80 { mant: INT_BIT, se: 0x3FFF };
    pub const INFINITY: F80 = F80 { mant: INT_BIT, se: 0x7FFF };
    pub const NEG_INFINITY: F80 = F80 { mant: INT_BIT, se: 0xFFFF };
    /// The x87 "real indefinite" (result of every masked invalid operation).
    pub const INDEFINITE: F80 = F80 { mant: INT_BIT | QUIET_BIT, se: 0xFFFF };
    /// A positive quiet NaN (`__builtin_nanl("")`).
    pub const NAN: F80 = F80 { mant: INT_BIT | QUIET_BIT, se: 0x7FFF };
    pub const MAX: F80 = F80 { mant: u64::MAX, se: 0x7FFE };
    pub const MIN_POSITIVE: F80 = F80 { mant: INT_BIT, se: 1 };
    pub const MIN_DENORMAL: F80 = F80 { mant: 1, se: 0 };

    /// Builds a value from its raw fields.
    #[must_use]
    pub const fn from_parts(neg: bool, exp: u16, mant: u64) -> F80 {
        F80 { mant, se: ((neg as u16) << 15) | (exp & 0x7FFF) }
    }

    /// From the 10 little-endian bytes of a C `long double`.
    #[must_use]
    pub const fn from_bytes(b: [u8; 10]) -> F80 {
        let mant = u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
        F80 { mant, se: u16::from_le_bytes([b[8], b[9]]) }
    }

    /// The 10 little-endian bytes of a C `long double`.
    #[must_use]
    pub const fn to_bytes(self) -> [u8; 10] {
        let m = self.mant.to_le_bytes();
        let s = self.se.to_le_bytes();
        [m[0], m[1], m[2], m[3], m[4], m[5], m[6], m[7], s[0], s[1]]
    }

    /// Reads 10 bytes from a possibly unaligned pointer.
    ///
    /// # Safety
    /// `p` must be readable for 10 bytes.
    #[must_use]
    pub unsafe fn read(p: *const u8) -> F80 {
        let mut b = [0u8; 10];
        // SAFETY: caller guarantees 10 readable bytes.
        unsafe { core::ptr::copy_nonoverlapping(p, b.as_mut_ptr(), 10) };
        F80::from_bytes(b)
    }

    /// Writes 10 bytes to a possibly unaligned pointer.
    ///
    /// # Safety
    /// `p` must be writable for 10 bytes.
    pub unsafe fn write(self, p: *mut u8) {
        let b = self.to_bytes();
        // SAFETY: caller guarantees 10 writable bytes.
        unsafe { core::ptr::copy_nonoverlapping(b.as_ptr(), p, 10) };
    }

    #[must_use]
    pub const fn sign(self) -> bool {
        self.se & 0x8000 != 0
    }

    #[must_use]
    pub const fn exp(self) -> u16 {
        self.se & 0x7FFF
    }

    #[must_use]
    pub const fn neg(self) -> F80 {
        F80 { mant: self.mant, se: self.se ^ 0x8000 }
    }

    #[must_use]
    pub const fn abs(self) -> F80 {
        F80 { mant: self.mant, se: self.se & 0x7FFF }
    }

    #[must_use]
    pub const fn with_sign(self, neg: bool) -> F80 {
        F80 { mant: self.mant, se: (self.se & 0x7FFF) | ((neg as u16) << 15) }
    }

    #[must_use]
    pub const fn class(self) -> Class {
        let e = self.exp();
        let int = self.mant & INT_BIT != 0;
        if e == 0 {
            if self.mant == 0 {
                Class::Zero
            } else {
                Class::Denormal
            }
        } else if e == 0x7FFF {
            if !int {
                Class::Unsupported
            } else if self.mant << 1 == 0 {
                Class::Infinity
            } else if self.mant & QUIET_BIT != 0 {
                Class::QuietNaN
            } else {
                Class::SignalingNaN
            }
        } else if int {
            Class::Normal
        } else {
            Class::Unsupported
        }
    }

    #[must_use]
    pub const fn is_nan(self) -> bool {
        matches!(self.class(), Class::QuietNaN | Class::SignalingNaN)
    }

    #[must_use]
    pub const fn is_infinite(self) -> bool {
        matches!(self.class(), Class::Infinity)
    }

    #[must_use]
    pub const fn is_zero(self) -> bool {
        matches!(self.class(), Class::Zero)
    }

    /// Finite and supported (zero, denormal or normal).
    #[must_use]
    pub const fn is_finite(self) -> bool {
        matches!(self.class(), Class::Zero | Class::Denormal | Class::Normal)
    }

    /// MinGW `fpclassify` (used by the libm functions): looks only at the bits,
    /// so unnormals count as normal and pseudo-denormals as normal.
    pub(crate) fn mingw_class(self) -> MingwClass {
        let e = self.exp();
        if e == 0 {
            if self.mant == 0 {
                return MingwClass::Zero;
            }
            if self.mant & INT_BIT == 0 {
                return MingwClass::Subnormal;
            }
        } else if e == 0x7FFF {
            return if self.mant & !INT_BIT == 0 { MingwClass::Infinite } else { MingwClass::NaN };
        }
        MingwClass::Normal
    }

    /// x87 `fucomi`: `None` when unordered (NaN or unsupported operand).
    #[must_use]
    pub fn compare(self, other: F80) -> Option<Ordering> {
        arith::compare(self, other)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MingwClass {
    NaN,
    Normal,
    Infinite,
    Zero,
    Subnormal,
}

impl PartialOrd for F80 {
    fn partial_cmp(&self, other: &F80) -> Option<Ordering> {
        arith::compare(*self, *other)
    }
}

impl core::fmt::Debug for F80 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "F80({:04X}:{:016X} = {})", self.se, self.mant, dec::format_g(*self, 21))
    }
}

impl core::fmt::Display for F80 {
    /// Shortest decimal text that reads back to the same value (`%Lg` style).
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&dec::format_shortest(*self))
    }
}

impl core::ops::Add for F80 {
    type Output = F80;
    fn add(self, rhs: F80) -> F80 {
        arith::add(self, rhs)
    }
}

impl core::ops::Sub for F80 {
    type Output = F80;
    fn sub(self, rhs: F80) -> F80 {
        arith::sub(self, rhs)
    }
}

impl core::ops::Mul for F80 {
    type Output = F80;
    fn mul(self, rhs: F80) -> F80 {
        arith::mul(self, rhs)
    }
}

impl core::ops::Div for F80 {
    type Output = F80;
    fn div(self, rhs: F80) -> F80 {
        arith::div(self, rhs)
    }
}

impl core::ops::Neg for F80 {
    type Output = F80;
    /// `fchs`: flips the sign bit of any encoding.
    fn neg(self) -> F80 {
        F80::neg(self)
    }
}

impl F80 {
    /// `fsqrt`.
    #[must_use]
    pub fn sqrt(self) -> F80 {
        arith::sqrt(self)
    }

    /// `frndint` with the given rounding control.
    #[must_use]
    pub fn round_int(self, mode: Round) -> F80 {
        arith::round_int(self, mode)
    }

    /// `fscale`: `self * 2^trunc(by)`.
    #[must_use]
    pub fn scale(self, by: F80) -> F80 {
        arith::scale(self, by)
    }
}
