//! `DECLARE LIBRARY` and `DECLARE DYNAMIC LIBRARY` for the QIR backend.
//!
//! The C++ backend let g++ compile a prototype for every declared function
//! and link the library. The QIR backend has no C compiler, so the runner
//! resolves symbols at run time and calls them through hand-written call
//! thunks (`global_asm!`, no libffi, no C):
//!
//! - [`Library::open_dynamic`] finds the library file the way the legacy
//!   compiler did for `DECLARE DYNAMIC LIBRARY "name"` (and for
//!   `DECLARE LIBRARY "name"`, whose static `.a`/`.lib`/`.o` cannot be loaded
//!   at run time) and loads it with `LoadLibraryW` / `dlopen`;
//! - [`find_default_symbol`] serves a header-less `DECLARE LIBRARY` with no
//!   name: the C runtime, the program itself and the system libraries the
//!   legacy build linked (kernel32, user32, gdi32, ... / libc, libm);
//! - [`call`] calls any function pointer with a [`Signature`] of QB types on
//!   Win64, `SysV` x86-64 and `AArch64` (AAPCS64, including Apple's variant).
//!
//! `DECLARE LIBRARY "name"` with a C header (`name.h` / `name.hpp`) is
//! handled by the BASIC compiler: it builds a wrapper library from the header
//! with a C++ compiler and puts it next to the program, whose `meta extfn`
//! names it as `./name` (opened from the program's folder). Headers may
//! contain macros and inline C code, which only a C compiler can use.
//! [`find_header`] and [`header_error`] remain for tools without a C++
//! compiler.
//!
//! See `docs/dev/ffi-float80.md` for the API contract used by the
//! emitter and the VM.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
// clippy::pedantic is clean except for the lints below. Bit-level x87
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
mod abi;
mod loader;

pub use loader::{Library, LibraryError, Os, default_modules, find_default_symbol, find_header, header_error, library_candidates};

use std::ffi::c_void;
use std::fmt;

/// Machine type of one argument or of the return value.
///
/// QB mapping: `_BYTE` I8, `_UNSIGNED _BYTE` U8, `INTEGER` I16,
/// `_UNSIGNED INTEGER` U16, `LONG` I32, `_UNSIGNED LONG` U32, `_INTEGER64` I64,
/// `_UNSIGNED _INTEGER64` U64, `SINGLE` F32, `DOUBLE` F64, `_OFFSET` and every
/// `BYREF` parameter and `BYVAL ... AS STRING` (pointer to the bytes) Ptr,
/// `SUB` Void. `_FLOAT` has no portable C ABI and is rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ty {
    Void,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
    Ptr,
}

impl Ty {
    #[must_use]
    pub fn is_float(self) -> bool {
        matches!(self, Ty::F32 | Ty::F64)
    }
}

/// A function type: parameter types, return type, and the index of the first
/// variadic argument (C `...`), if any.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Signature {
    pub args: Vec<Ty>,
    pub ret: Ty,
    pub variadic_from: Option<usize>,
}

impl Signature {
    #[must_use]
    pub fn new(args: Vec<Ty>, ret: Ty) -> Signature {
        Signature { args, ret, variadic_from: None }
    }

    #[must_use]
    pub fn variadic(args: Vec<Ty>, ret: Ty, fixed: usize) -> Signature {
        Signature { args, ret, variadic_from: Some(fixed) }
    }
}

/// An argument or result. Integers are converted to the declared width
/// (truncated, then sign- or zero-extended); `I`/`U` are interchangeable on
/// input. Results come back as `I` for signed types, `U` for unsigned, `Ptr`,
/// `F32`, `F64` or `Void`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Void,
    I(i64),
    U(u64),
    F32(f32),
    F64(f64),
    Ptr(usize),
}

impl Value {
    /// Register image of an integer-like value (floats are converted by `arg_bits`).
    fn bits(self) -> u64 {
        match self {
            Value::I(v) => v as u64,
            Value::U(v) => v,
            Value::Ptr(p) => p as u64,
            Value::Void | Value::F32(_) | Value::F64(_) => 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallError {
    /// Number of arguments differs from the signature.
    ArgCount { expected: usize, got: usize },
    /// `Ty::Void` used as a parameter type.
    VoidArgument(usize),
    /// A float slot was given a non-float value or the other way round.
    Mismatch(usize),
    /// Null function pointer.
    NullFunction,
}

impl fmt::Display for CallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CallError::ArgCount { expected, got } => write!(f, "expected {expected} arguments, got {got}"),
            CallError::VoidArgument(i) => write!(f, "argument {} has no type", i + 1),
            CallError::Mismatch(i) => write!(f, "argument {} has the wrong kind of value", i + 1),
            CallError::NullFunction => write!(f, "null function pointer"),
        }
    }
}

impl std::error::Error for CallError {}

/// Converts an argument to the 64-bit register image of its declared type.
fn arg_bits(ty: Ty, v: Value, i: usize) -> Result<u64, CallError> {
    let int = |v: Value| -> Result<u64, CallError> {
        match v {
            Value::I(_) | Value::U(_) | Value::Ptr(_) => Ok(v.bits()),
            // QB passes the value converted to the parameter type; accept it.
            Value::F32(x) => Ok((x as f64).round_ties_even() as i64 as u64),
            Value::F64(x) => Ok(x.round_ties_even() as i64 as u64),
            Value::Void => Err(CallError::Mismatch(i)),
        }
    };
    let float = |v: Value| -> Result<f64, CallError> {
        match v {
            Value::F32(x) => Ok(x as f64),
            Value::F64(x) => Ok(x),
            Value::I(x) => Ok(x as f64),
            Value::U(x) => Ok(x as f64),
            Value::Ptr(_) | Value::Void => Err(CallError::Mismatch(i)),
        }
    };
    Ok(match ty {
        Ty::Void => return Err(CallError::VoidArgument(i)),
        Ty::I8 => int(v)? as i8 as i64 as u64,
        Ty::U8 => int(v)? as u8 as u64,
        Ty::I16 => int(v)? as i16 as i64 as u64,
        Ty::U16 => int(v)? as u16 as u64,
        Ty::I32 => int(v)? as i32 as i64 as u64,
        Ty::U32 => int(v)? as u32 as u64,
        Ty::I64 | Ty::U64 | Ty::Ptr => int(v)?,
        Ty::F32 => (float(v)? as f32).to_bits() as u64,
        Ty::F64 => float(v)?.to_bits(),
    })
}

fn ret_value(ty: Ty, gpr: u64, fpr: u64) -> Value {
    match ty {
        Ty::Void => Value::Void,
        Ty::I8 => Value::I(gpr as i8 as i64),
        Ty::U8 => Value::U(gpr as u8 as u64),
        Ty::I16 => Value::I(gpr as i16 as i64),
        Ty::U16 => Value::U(gpr as u16 as u64),
        Ty::I32 => Value::I(gpr as i32 as i64),
        Ty::U32 => Value::U(gpr as u32 as u64),
        Ty::I64 => Value::I(gpr as i64),
        Ty::U64 => Value::U(gpr),
        Ty::Ptr => Value::Ptr(gpr as usize),
        Ty::F32 => Value::F32(f32::from_bits(fpr as u32)),
        Ty::F64 => Value::F64(f64::from_bits(fpr)),
    }
}

/// Calls `func` with `args` as described by `sig`.
///
/// # Errors
/// [`CallError`] for a null pointer, a wrong argument count or a value of the
/// wrong kind for its slot; nothing is called then.
///
/// # Safety
/// `func` must point to a function with exactly this C signature, and every
/// pointer argument must be valid for what the function does with it.
pub unsafe fn call(func: *const c_void, sig: &Signature, args: &[Value]) -> Result<Value, CallError> {
    if func.is_null() {
        return Err(CallError::NullFunction);
    }
    if args.len() != sig.args.len() {
        return Err(CallError::ArgCount { expected: sig.args.len(), got: args.len() });
    }
    let mut slots = Vec::with_capacity(args.len());
    for (i, (&ty, &v)) in sig.args.iter().zip(args).enumerate() {
        slots.push((ty, arg_bits(ty, v, i)?));
    }
    let variadic_from = sig.variadic_from.unwrap_or(usize::MAX);
    // SAFETY: forwarded caller contract.
    let (gpr, fpr) = unsafe { abi::invoke(func, &slots, variadic_from) };
    Ok(ret_value(sig.ret, gpr, fpr))
}

/// A resolved function with its signature (what a QIR `DECLARE` entry turns into).
#[derive(Clone, Debug)]
pub struct Function {
    pub ptr: *const c_void,
    pub sig: Signature,
}

impl Function {
    /// # Errors
    /// As [`call`].
    ///
    /// # Safety
    /// See [`call`].
    pub unsafe fn call(&self, args: &[Value]) -> Result<Value, CallError> {
        // SAFETY: forwarded caller contract.
        unsafe { call(self.ptr, &self.sig, args) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_arguments_are_narrowed_and_extended() {
        assert_eq!(arg_bits(Ty::I8, Value::I(255), 0).unwrap(), u64::MAX);
        assert_eq!(arg_bits(Ty::U8, Value::I(-1), 0).unwrap(), 255);
        assert_eq!(arg_bits(Ty::I16, Value::U(0x18000), 0).unwrap(), (-32768i64) as u64);
        assert_eq!(arg_bits(Ty::U32, Value::I(-1), 0).unwrap(), 0xFFFF_FFFF);
        assert_eq!(arg_bits(Ty::I32, Value::F64(2.5), 0).unwrap(), 2);
        assert_eq!(arg_bits(Ty::F32, Value::I(3), 0).unwrap(), 3.0f32.to_bits() as u64);
        assert_eq!(arg_bits(Ty::F64, Value::Ptr(1), 0), Err(CallError::Mismatch(0)));
        assert_eq!(arg_bits(Ty::Void, Value::I(1), 4), Err(CallError::VoidArgument(4)));
    }

    #[test]
    fn results_follow_the_declared_type() {
        assert_eq!(ret_value(Ty::I16, 0xFFFF_FFFF_0001_FFFF, 0), Value::I(-1));
        assert_eq!(ret_value(Ty::U8, 0x1FF, 0), Value::U(0xFF));
        assert_eq!(ret_value(Ty::F32, 0, 0xDEAD_0000_3FC0_0000), Value::F32(1.5));
    }
}
