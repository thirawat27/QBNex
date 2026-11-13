//! The QBasic runtime as the VM calls it: the `rt` builtins of
//! `qb_ir::builtins::TABLE` (PRINT, files, strings, `_MEM`, TCP, sound, ...)
//! and the values they exchange with the executor.
//!
//! The executor reaches the builtins through [`builtins::call`] and provides
//! what they need from it through [`builtins::Host`].

use qb_float80::{F80, Round};
use qb_ir::Ty;

mod audiox;
mod cpu;
mod devices;
pub(crate) mod display;
pub(crate) mod dosx;
mod input;
mod lprint;
mod scancodes;
pub(crate) mod builtins;
mod files;
mod memx;
mod netx;
mod scene3d;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod window;
mod winx;

pub(crate) use builtins::{call, str_store_hook, Host, RtState};
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use window::{activate as window_activate, closed as window_closed, pixels as window_pixels, present as window_present, present_pixels as window_present_pixels, release_exit as window_release_exit, set_title as window_set_title, PRESENT_EVENT};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum V {
    I(i64),
    F(f64),
    /// `_FLOAT`: x87 80-bit extended (bit-exact with real x87 `long double`)
    X(F80),
    S(Vec<u8>),
    P(u64),
}

impl V {
    fn i(&self) -> i64 {
        match self {
            V::I(v) => *v,
            V::F(f) => *f as i64,
            V::X(x) => x.to_i64(Round::Zero),
            V::P(p) => *p as i64,
            V::S(_) => 0,
        }
    }
    pub(crate) fn f(&self) -> f64 {
        match self {
            V::F(f) => *f,
            V::X(x) => x.to_f64(),
            V::I(v) => *v as f64,
            _ => 0.0,
        }
    }
    pub(crate) fn x(&self) -> F80 {
        match self {
            V::X(x) => *x,
            V::F(f) => F80::from_f64(*f),
            V::I(v) => F80::from_i64(*v),
            _ => F80::from_i64(0),
        }
    }
    fn s(&self) -> &[u8] {
        match self {
            V::S(s) => s,
            _ => &[],
        }
    }
    fn p(&self) -> u64 {
        match self {
            V::P(p) => *p,
            V::I(v) => *v as u64,
            _ => 0,
        }
    }
}

/// Normalizes an integer to the range of `t` (two's complement wrap).
pub(crate) fn wrap(t: Ty, v: i64) -> i64 {
    match t {
        Ty::I8 => v as i8 as i64,
        Ty::U8 => v as u8 as i64,
        Ty::I16 => v as i16 as i64,
        Ty::U16 => v as u16 as i64,
        Ty::I32 => v as i32 as i64,
        Ty::U32 => v as u32 as i64,
        _ => v,
    }
}

/// Float result in the precision of `t` (`f80` is carried as f64 here).
pub(crate) fn fround(t: Ty, v: f64) -> f64 {
    if t == Ty::F32 {
        f64::from(v as f32)
    } else {
        v
    }
}

pub(crate) enum Stop {
    Err(i32),
}

pub(crate) type R<T> = Result<T, Stop>;

pub(crate) fn e<T>(code: i32) -> R<T> {
    Err(Stop::Err(code))
}

pub(crate) fn cvt(to: Ty, from: Ty, v: &V, round: bool) -> V {
    if to == Ty::F80 {
        return V::X(match from {
            Ty::F80 => v.x(),
            Ty::F32 | Ty::F64 => F80::from_f64(v.f()),
            Ty::U64 => F80::from_u64(v.i() as u64),
            _ => F80::from_i64(v.i()),
        });
    }
    if from == Ty::F80 {
        let x = v.x();
        if to.is_float() {
            return V::F(if to == Ty::F32 { f64::from(x.to_f32()) } else { x.to_f64() });
        }
        if to == Ty::U64 {
            return V::I(x.to_u64_cast() as i64);
        }
        let n = if round { qb_float80::qb::qbr(x) } else { x.to_i64(Round::Zero) };
        return V::I(wrap(to, n));
    }
    if to.is_float() {
        let f = if from.is_float() {
            v.f()
        } else if from == Ty::U64 {
            v.i() as u64 as f64
        } else {
            v.i() as f64
        };
        return V::F(fround(to, f));
    }
    if from.is_float() {
        let f = v.f();
        if round && to != Ty::U64 {
            // x87 FISTP semantics (`qb_runtime::round::qbr`)
            return V::I(wrap(to, qb_runtime::round::qb_qbr_f64(f)));
        }
        let f = if round { f.round_ties_even() } else { f.trunc() };
        let n = if to == Ty::U64 { f as u64 as i64 } else { f as i64 };
        return V::I(wrap(to, n));
    }
    V::I(wrap(to, v.i()))
}
