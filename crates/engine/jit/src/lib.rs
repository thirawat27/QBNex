//! `qb-jit` — Cranelift JIT tier for QBNex.
//!
//! Compiles MIR functions (typed, block-structured; see [`mir`]) to native
//! code in memory: no linker, no C compiler. The VM stays the first tier;
//! [`tier::Tiering`] decides when a function or loop is hot enough to compile,
//! and every exit of native code (error, event, unsupported construct) hands
//! the exact frame state back to the VM at a known pc (see [`ctx`]).
//!
//! The tests carry a reference interpreter of the same MIR
//! (`tests/common/interp.rs`) as the oracle of differential tests; it is not
//! part of the library: the VM is the only interpreter.

// browser build: the code generator's layout tables have no reader
#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

#[cfg(not(target_arch = "wasm32"))]
pub mod aot;
#[cfg(not(target_arch = "wasm32"))]
pub mod codegen;
/// Browser build: no Cranelift; every compile declines.
#[cfg(target_arch = "wasm32")]
#[path = "off.rs"]
pub mod codegen;
#[cfg(target_arch = "wasm32")]
pub use codegen::aot;
pub mod ctx;
pub mod mir;
pub mod rt;
#[cfg(not(target_arch = "wasm32"))]
pub mod thunk;
pub mod tier;

pub use codegen::{compile, func_symbol, host_supported, symbols, CompileError, Compiled, NativeFn, Target};
pub use ctx::{status, ArrDesc, ChainRec, Ctx};

impl Compiled {
    /// Runs function `func` on `frame`, entered at `entry` (`mir::ENTRY_CALL` or a block pc).
    ///
    /// # Safety
    /// `frame` must hold the function's slots, `ctx.globals` the module's
    /// globals, and slot contents must be valid for their types (handles/pointers).
    pub unsafe fn run(&self, ctx: &mut Ctx, func: mir::FuncId, frame: *mut u64, entry: u32) -> u32 {
        // SAFETY: caller contract.
        unsafe { (self.fns[func as usize])(ctx, frame, entry) }
    }
}
