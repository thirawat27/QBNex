//! Run-time context shared by the VM, the JIT'd code and host functions.
//!
//! Native code reads/writes the `#[repr(C)]` fields at fixed offsets
//! (`offset_of!`), so field order here is ABI.

use core::sync::atomic::AtomicU32;
use std::ffi::c_void;
use std::mem::offset_of;

/// Return status of compiled code (and of the reference interpreter).
pub mod status {
    /// Function returned normally (END SUB/FUNCTION, EXIT SUB).
    pub const OK: u32 = 0;
    /// END / SYSTEM.
    pub const END: u32 = 1;
    /// QB error: `Ctx::err`, `Ctx::err_line`, `Ctx::exit_pc` (pc of the failing statement).
    pub const ERROR: u32 = 2;
    /// Event pending at a poll point: VM continues at `Ctx::exit_pc`.
    pub const EVENT: u32 = 3;
    /// Native code cannot continue (unknown OSR entry, RETURN to a VM-only site):
    /// VM continues at `Ctx::exit_pc`.
    pub const DEOPT: u32 = 4;
}

/// QB error numbers used by compiled code.
pub mod qberr {
    pub const RETURN_WITHOUT_GOSUB: i32 = 3;
    pub const ILLEGAL_CALL: i32 = 5;
    pub const OVERFLOW: i32 = 6;
    pub const OUT_OF_MEMORY: i32 = 7;
    pub const SUBSCRIPT: i32 = 9;
    pub const DIV_ZERO: i32 = 11;
    pub const OUT_OF_STACK: i32 = 28;
}

pub const MAX_DIMS: usize = 8;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Dim {
    pub lbound: i64,
    pub count: i64,
    /// Elements between consecutive indices of this dimension.
    pub stride: i64,
}

/// Array descriptor. The first index varies fastest (QB64 layout).
#[repr(C)]
#[derive(Debug)]
pub struct ArrDesc {
    /// Null = not DIMensioned (access raises error 9).
    pub data: *mut u8,
    pub ndims: u32,
    pub elem: u32,
    pub dims: [Dim; MAX_DIMS],
}

impl ArrDesc {
    /// Allocates a zeroed array with `bounds[d] = (lbound, ubound)`.
    pub fn new(elem: u32, bounds: &[(i64, i64)]) -> Box<ArrDesc> {
        assert!(!bounds.is_empty() && bounds.len() <= MAX_DIMS);
        let mut dims = [Dim::default(); MAX_DIMS];
        let mut stride = 1i64;
        for (d, &(lb, ub)) in bounds.iter().enumerate() {
            let count = (ub - lb + 1).max(0);
            dims[d] = Dim { lbound: lb, count, stride };
            stride *= count;
        }
        let bytes = (stride as usize) * elem as usize;
        let data = Box::into_raw(vec![0u8; bytes.max(1)].into_boxed_slice()) as *mut u8;
        Box::new(ArrDesc { data, ndims: bounds.len() as u32, elem, dims })
    }

    pub fn len(&self) -> usize {
        (0..self.ndims as usize).map(|d| self.dims[d].count as usize).product()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Element offset (in elements) or error 9.
    pub fn offset(&self, idx: &[i64]) -> Result<usize, i32> {
        if self.data.is_null() || idx.len() != self.ndims as usize {
            return Err(qberr::SUBSCRIPT);
        }
        let mut off = 0i64;
        for (d, &i) in idx.iter().enumerate() {
            let dim = self.dims[d];
            let k = i.wrapping_sub(dim.lbound);
            if (k as u64) >= (dim.count as u64) {
                return Err(qberr::SUBSCRIPT);
            }
            off += k * dim.stride;
        }
        Ok(off as usize)
    }
}

impl Drop for ArrDesc {
    fn drop(&mut self) {
        if !self.data.is_null() {
            let bytes = (self.len() * self.elem as usize).max(1);
            // SAFETY: allocated in `new` as a boxed slice of exactly `bytes` bytes.
            unsafe { drop(Box::from_raw(core::ptr::slice_from_raw_parts_mut(self.data, bytes))) };
        }
    }
}

/// A JIT frame left behind by an abnormal exit (innermost first). The VM
/// materialises it as a frame of `func` whose slots are at `frame`; when that
/// frame returns, the parent continues after the call at `ret_pc`, storing
/// the result (if `dst != NO_DST`) into its slot `dst`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainRec {
    pub func: u32,
    pub frame: *mut u64,
    pub ret_pc: u32,
    pub dst: u32,
}

pub const NO_DST: u32 = u32::MAX;

#[repr(C)]
pub struct Ctx {
    /// Non-zero = an event (ON TIMER/KEY, Ctrl+Break, ...) is pending.
    pub event: AtomicU32,
    pub err: i32,
    pub err_line: u32,
    pub exit_pc: u32,
    pub globals: *mut u64,
    /// Shared GOSUB return stack (VM pcs).
    pub gosub: *mut u32,
    pub gosub_len: u32,
    pub gosub_cap: u32,
    /// GOSUB stack base of the frame being entered at an OSR point.
    pub gosub_base: u32,
    pub _pad: u32,
    /// Frame arena for JIT-to-JIT calls (slots).
    pub arena: *mut u64,
    pub arena_top: u64,
    pub arena_cap: u64,
    /// Host state (VM, runtime).
    pub user: *mut c_void,
    /// Event word polled by native code: pending = `*ev_ptr & ev_mask != 0`.
    /// Defaults to `&event` / all bits; a VM points it at its own pending word.
    pub ev_ptr: *const AtomicU32,
    pub ev_mask: u32,
    pub _pad2: u32,
    /// Embedder tables of array pointers (`VAElem`): entry = null or a
    /// pointer to a structure holding an [`ArrView`] at `view_off`.
    pub tables: [*mut u64; 2],
    /// Set by a `VmOp` callback before it returns: the (possibly moved)
    /// frame and globals of the code that called it. Native code reloads
    /// both after every `VmOp` (a VM call may grow and move its register file).
    pub frame: *mut u64,
    /// Native calls still allowed before native code hands calls to the VM
    /// (bounds the native stack). Decremented per active native call.
    pub call_budget: u32,
    pub _pad3: u32,
    // ---- not accessed by native code ----
    pub chain: Vec<ChainRec>,
    gosub_buf: Vec<u32>,
    arena_buf: Vec<u64>,
}

pub(crate) const OFF_EV_PTR: i32 = offset_of!(Ctx, ev_ptr) as i32;
pub(crate) const OFF_EV_MASK: i32 = offset_of!(Ctx, ev_mask) as i32;
pub(crate) const OFF_TABLES: i32 = offset_of!(Ctx, tables) as i32;
pub(crate) const OFF_FRAME: i32 = offset_of!(Ctx, frame) as i32;
pub(crate) const OFF_CALL_BUDGET: i32 = offset_of!(Ctx, call_budget) as i32;

/// Native call depth allowed per entry into native code.
pub const DEFAULT_CALL_BUDGET: u32 = 1000;
pub(crate) const OFF_VIEW_DATA: i32 = offset_of!(ArrView, data) as i32;
pub(crate) const OFF_VIEW_LB: i32 = offset_of!(ArrView, lbound) as i32;
pub(crate) const OFF_VIEW_COUNT: i32 = offset_of!(ArrView, count) as i32;

/// Non-owning view of a 1-D array an embedder keeps (see `Inst::VAElem`).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ArrView {
    /// Null = no fast path (always take the slow host call).
    pub data: *mut u8,
    pub lbound: i64,
    /// Element count; 0 for arrays with more than one dimension.
    pub count: i64,
}

impl ArrView {
    pub const NONE: ArrView = ArrView { data: core::ptr::null_mut(), lbound: 0, count: 0 };
}
pub(crate) const OFF_ERR: i32 = offset_of!(Ctx, err) as i32;
pub(crate) const OFF_ERR_LINE: i32 = offset_of!(Ctx, err_line) as i32;
pub(crate) const OFF_EXIT_PC: i32 = offset_of!(Ctx, exit_pc) as i32;
pub(crate) const OFF_GLOBALS: i32 = offset_of!(Ctx, globals) as i32;
pub(crate) const OFF_GOSUB: i32 = offset_of!(Ctx, gosub) as i32;
pub(crate) const OFF_GOSUB_LEN: i32 = offset_of!(Ctx, gosub_len) as i32;
pub(crate) const OFF_GOSUB_CAP: i32 = offset_of!(Ctx, gosub_cap) as i32;
pub(crate) const OFF_GOSUB_BASE: i32 = offset_of!(Ctx, gosub_base) as i32;
pub(crate) const OFF_ARENA: i32 = offset_of!(Ctx, arena) as i32;
pub(crate) const OFF_ARENA_TOP: i32 = offset_of!(Ctx, arena_top) as i32;
pub(crate) const OFF_ARENA_CAP: i32 = offset_of!(Ctx, arena_cap) as i32;

pub(crate) const OFF_DESC_DATA: i32 = offset_of!(ArrDesc, data) as i32;
pub(crate) const OFF_DESC_NDIMS: i32 = offset_of!(ArrDesc, ndims) as i32;
pub(crate) const OFF_DESC_DIMS: i32 = offset_of!(ArrDesc, dims) as i32;
pub(crate) const DIM_SIZE: i32 = core::mem::size_of::<Dim>() as i32;
pub(crate) const OFF_DIM_LB: i32 = offset_of!(Dim, lbound) as i32;
pub(crate) const OFF_DIM_COUNT: i32 = offset_of!(Dim, count) as i32;
pub(crate) const OFF_DIM_STRIDE: i32 = offset_of!(Dim, stride) as i32;

impl Ctx {
    /// New context over `globals` (must outlive the context's use).
    pub fn new(globals: *mut u64) -> Box<Ctx> {
        Self::with_capacity(globals, 1 << 16, 1 << 20)
    }

    pub fn with_capacity(globals: *mut u64, gosub_cap: usize, arena_slots: usize) -> Box<Ctx> {
        let mut gosub_buf = vec![0u32; gosub_cap];
        let mut arena_buf = vec![0u64; arena_slots];
        let mut c = Box::new(Ctx {
            event: AtomicU32::new(0),
            err: 0,
            err_line: 0,
            exit_pc: 0,
            globals,
            gosub: gosub_buf.as_mut_ptr(),
            gosub_len: 0,
            gosub_cap: gosub_cap as u32,
            gosub_base: 0,
            _pad: 0,
            arena: arena_buf.as_mut_ptr(),
            arena_top: 0,
            arena_cap: arena_slots as u64,
            user: core::ptr::null_mut(),
            ev_ptr: core::ptr::null(),
            ev_mask: u32::MAX,
            tables: [core::ptr::null_mut(); 2],
            frame: core::ptr::null_mut(),
            call_budget: DEFAULT_CALL_BUDGET,
            _pad3: 0,
            _pad2: 0,
            chain: Vec::new(),
            gosub_buf,
            arena_buf,
        });
        c.ev_ptr = &c.event;
        c
    }

    /// Is an event pending (what native code checks)?
    pub fn event_pending(&self) -> bool {
        // SAFETY: `ev_ptr` points at `self.event` or at a word the embedder keeps alive.
        unsafe { (*self.ev_ptr).load(core::sync::atomic::Ordering::Relaxed) & self.ev_mask != 0 }
    }

    /// Current GOSUB stack (VM pcs), bottom first.
    pub fn gosub_stack(&self) -> &[u32] {
        &self.gosub_buf[..self.gosub_len as usize]
    }

    /// Clears error/exit state and the deopt chain (frames in the arena are released).
    pub fn reset_exit(&mut self) {
        self.err = 0;
        self.err_line = 0;
        self.exit_pc = 0;
        self.chain.clear();
        self.arena_top = 0;
    }
}

/// Called by native code when a callee left abnormally.
#[cfg_attr(feature = "native-syms", unsafe(export_name = "qb_jit_chain_push"))]
pub(crate) extern "C" fn chain_push(ctx: *mut Ctx, func: u32, frame: *mut u64, ret_pc: u32, dst: u32) {
    // SAFETY: native code passes the context it was entered with.
    let ctx = unsafe { &mut *ctx };
    ctx.chain.push(ChainRec { func, frame, ret_pc, dst });
}

/// Zeroes `n` slots (fresh callee frame).
#[cfg_attr(feature = "native-syms", unsafe(export_name = "qb_jit_zero_slots"))]
pub(crate) extern "C" fn zero_slots(p: *mut u64, n: u64) {
    // SAFETY: native code passes a frame inside the arena with `n` slots.
    unsafe { core::ptr::write_bytes(p, 0, n as usize) };
}

#[cfg_attr(feature = "native-syms", unsafe(export_name = "qb_jit_pow_f64"))]
pub extern "C" fn pow_f64(a: f64, b: f64) -> f64 {
    a.powf(b)
}

#[cfg_attr(feature = "native-syms", unsafe(export_name = "qb_jit_pow_f32"))]
pub extern "C" fn pow_f32(a: f32, b: f32) -> f32 {
    a.powf(b)
}

/// `qb_runtime::round::qb_qbr_f64` (exact QB rounding to INTEGER64).
#[cfg_attr(feature = "native-syms", unsafe(export_name = "qb_jit_qbr_f64"))]
pub(crate) extern "C" fn qbr_f64(v: f64) -> i64 {
    qb_runtime::round::qb_qbr_f64(v)
}
