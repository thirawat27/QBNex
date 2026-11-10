//! Browser build (R13-wasm): Cranelift cannot emit code that a wasm module
//! could run, so the code generator is replaced by one that always declines.
//! The VM sees a compile failure and keeps interpreting, exactly as when a
//! native compile fails. Same public surface as `codegen.rs`.

use crate::ctx::*;
use crate::mir::Module;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Host,
    X86_64,
    Aarch64,
    X86_64Darwin,
    Aarch64Darwin,
}

pub type NativeFn = unsafe extern "C" fn(*mut Ctx, *mut u64, u32) -> u32;

/// Compiled module (never produced in a browser build, except from an
/// embedder's loaded image, which qb-link also refuses there).
pub struct Compiled {
    _owner: Option<Box<dyn std::any::Any>>,
    pub fns: Vec<NativeFn>,
}

impl Compiled {
    /// # Safety
    /// As in the native build.
    pub unsafe fn from_loaded(fns: Vec<NativeFn>, owner: Box<dyn std::any::Any>) -> Compiled {
        Compiled { _owner: Some(owner), fns }
    }
}

#[derive(Debug)]
pub struct CompileError(pub String);

const NO_JIT: &str = "no native code generation in a browser build";

/// Runtime symbols native code would call (kept for API parity).
pub fn symbols(m: &Module) -> Vec<(String, usize)> {
    let mut v: Vec<(String, usize)> = vec![
        ("qb_jit_qbr_f64".into(), qbr_f64 as *const () as usize),
        ("qb_jit_pow_f32".into(), pow_f32 as *const () as usize),
        ("qb_jit_pow_f64".into(), pow_f64 as *const () as usize),
        ("qb_jit_zero_slots".into(), zero_slots as *const () as usize),
        ("qb_jit_chain_push".into(), chain_push as *const () as usize),
    ];
    for (i, h) in m.hosts.iter().enumerate() {
        v.push((format!("qb_host_{i}"), h.addr));
    }
    if m.vm_op != 0 {
        v.push(("qb_vm_op".into(), m.vm_op));
    }
    v
}

/// Exported name of function `i` of a module.
pub fn func_symbol(i: usize) -> String {
    format!("qb_func_{i}")
}

pub fn compile(_m: &Module, _target: Target) -> Result<Compiled, CompileError> {
    Err(CompileError(NO_JIT.into()))
}

/// Ahead-of-time objects cannot be produced either.
pub mod aot {
    use super::{CompileError, Target, NO_JIT};
    use crate::mir::Module;

    pub fn compile_object(_m: &Module, _target: Target) -> Result<Vec<u8>, CompileError> {
        Err(CompileError(NO_JIT.into()))
    }

    pub fn compile_native_object(_m: &Module, _t: Target, _p: &[u8], _fp: u64) -> Result<Vec<u8>, CompileError> {
        Err(CompileError(NO_JIT.into()))
    }
}
