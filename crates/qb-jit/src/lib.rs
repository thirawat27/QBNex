//! `qb-jit` — Cranelift JIT tier for QBNex (R11.6).
//!
//! Compiles MIR functions (typed, block-structured; see [`mir`]) to native
//! code in memory: no linker, no C compiler. The VM stays the first tier;
//! [`tier::Tiering`] decides when a function or loop is hot enough to compile,
//! and every exit of native code (error, event, unsupported construct) hands
//! the exact frame state back to the VM at a known pc (see [`ctx`]).
//!
//! [`interp`] is a reference interpreter of the same MIR used as the oracle
//! in differential tests.

pub mod aot;
pub mod codegen;
pub mod ctx;
pub mod interp;
pub mod mir;
pub mod rt;
pub mod thunk;
pub mod tier;

pub use codegen::{compile, func_symbol, symbols, CompileError, Compiled, NativeFn, Target};
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

/// qb-runtime refers to libqb symbols from code the JIT never calls; the
/// embedder (qb-exec) defines them. Unit tests of this crate need them too.
#[cfg(test)]
#[allow(non_upper_case_globals)]
mod libqb_host {
    #[unsafe(no_mangle)]
    pub static mut cmem_sp: u32 = 0;
    #[unsafe(no_mangle)]
    pub static mut dblock: isize = 0;
    #[unsafe(no_mangle)]
    pub static mut cmem: [u8; 1] = [0];
    #[unsafe(no_mangle)]
    pub extern "C" fn qb_host_error(_n: i32) {
        std::process::abort()
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn qb_host_field_free(_s: *mut core::ffi::c_void) {
        std::process::abort()
    }
    // R12b core runtime (allocators, errors, events, waits).
    #[unsafe(no_mangle)]
    pub static mut cmem_static_pointer: *mut u8 = core::ptr::null_mut();
    #[unsafe(no_mangle)]
    pub static mut cmem_dynamic_base: *mut u8 = core::ptr::null_mut();
    #[unsafe(no_mangle)]
    pub extern "C" fn qb_host_message_box(_m: *const core::ffi::c_char, _t: *const core::ffi::c_char, _k: i32) -> i32 {
        std::process::abort()
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn qb_host_end() {
        std::process::abort()
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn qb_host_evnt() {
        std::process::abort()
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn qb_host_devices_init() {
        std::process::abort()
    }}
