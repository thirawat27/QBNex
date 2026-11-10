//! Typed register interpreter for [`crate::bytecode::Module`].
//!
//! - Registers: three banks (`nums` / `strs` / `arrs`) used as stacks of
//!   frames; a frame is a window starting at `nb`/`sb`/`ab`. The hot loop
//!   keeps a raw pointer to the current numeric window (refreshed whenever a
//!   frame is pushed/popped or the view changes) and indexes it without bounds
//!   checks; [`Module::validate`] proved every operand is inside its frame.
//! - Error handling: an error stops the hot loop with `(code, pc)`; `raise`
//!   either jumps to the ON ERROR handler (which sees module-level variables:
//!   the *view* switches to frame 0 without unwinding) or ends the program.
//!   Statement starts come from the `stmts` side table, so `LINE` costs nothing.
//! - BYREF arguments are copied in at the call and back at return (strings and
//!   arrays are moved, never copied). Visible difference from pointer
//!   semantics only when a SUB changes the same variable both through a
//!   parameter and through SHARED.
//! - Events (ON TIMER/ON KEY/...): other threads set bits in an atomic mask;
//!   the VM looks at it only at safe points (backward jumps, NEXT, CALL) and
//!   runs the handler as an implicit GOSUB at module level.

use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use crate::bytecode::*;
use crate::value::{str_of, Val};
use crate::RunError;

// ---------------------------------------------------------------- arrays

#[derive(Debug, Clone)]
enum ArrData {
    I16(Vec<i16>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    Str(Vec<Vec<u8>>),
}

#[derive(Debug)]
pub(crate) struct Arr {
    dims: Vec<(i64, i64)>,
    data: ArrData,
    /// What JIT'd code reads for inline 1-D numeric element access
    /// (`data` never reallocates: REDIM builds a new `Arr`).
    pub(crate) view: qb_jit::ctx::ArrView,
}

impl Clone for Arr {
    fn clone(&self) -> Arr {
        Arr::with_data(self.dims.clone(), self.data.clone())
    }
}

impl Arr {
    fn new(ty: Ty, dims: Vec<(i64, i64)>) -> Result<Arr, i32> {
        let mut count: usize = 1;
        for &(lb, ub) in &dims {
            if ub < lb {
                return Err(9);
            }
            count = count.checked_mul(usize::try_from(ub - lb + 1).map_err(|_| 7)?).ok_or(7)?;
        }
        if count > (1 << 31) {
            return Err(7); // out of memory
        }
        let data = match ty {
            Ty::I16 => ArrData::I16(vec![0; count]),
            Ty::I32 => ArrData::I32(vec![0; count]),
            Ty::I64 => ArrData::I64(vec![0; count]),
            Ty::F32 => ArrData::F32(vec![0.0; count]),
            Ty::F64 => ArrData::F64(vec![0.0; count]),
            Ty::Str => ArrData::Str(vec![Vec::new(); count]),
        };
        Ok(Arr::with_data(dims, data))
    }

    fn with_data(dims: Vec<(i64, i64)>, mut data: ArrData) -> Arr {
        let ptr = match &mut data {
            ArrData::I16(v) => v.as_mut_ptr() as *mut u8,
            ArrData::I32(v) => v.as_mut_ptr() as *mut u8,
            ArrData::I64(v) => v.as_mut_ptr() as *mut u8,
            ArrData::F32(v) => v.as_mut_ptr() as *mut u8,
            ArrData::F64(v) => v.as_mut_ptr() as *mut u8,
            ArrData::Str(_) => core::ptr::null_mut(),
        };
        let view = match dims.as_slice() {
            [(lb, ub)] if !ptr.is_null() => qb_jit::ctx::ArrView { data: ptr, lbound: *lb, count: ub - lb + 1 },
            _ => qb_jit::ctx::ArrView::NONE,
        };
        Arr { dims, data, view }
    }

    #[cfg(test)]
    fn ty(&self) -> Ty {
        match self.data {
            ArrData::I16(_) => Ty::I16,
            ArrData::I32(_) => Ty::I32,
            ArrData::I64(_) => Ty::I64,
            ArrData::F32(_) => Ty::F32,
            ArrData::F64(_) => Ty::F64,
            ArrData::Str(_) => Ty::Str,
        }
    }

    #[inline(always)]
    fn off1(&self, i: i64) -> Result<usize, i32> {
        match self.dims.as_slice() {
            [(lb, ub)] if i >= *lb && i <= *ub => Ok((i - lb) as usize),
            _ => Err(9),
        }
    }

    fn off(&self, idx: impl Iterator<Item = i64>, n: usize) -> Result<usize, i32> {
        if self.dims.len() != n {
            return Err(9);
        }
        let mut off: usize = 0;
        for (i, &(lb, ub)) in idx.zip(&self.dims) {
            if i < lb || i > ub {
                return Err(9);
            }
            off = off * (ub - lb + 1) as usize + (i - lb) as usize;
        }
        Ok(off)
    }

    /// numeric element as register bits
    #[inline(always)]
    fn get(&self, o: usize) -> u64 {
        match &self.data {
            ArrData::I16(v) => v[o] as i64 as u64,
            ArrData::I32(v) => v[o] as i64 as u64,
            ArrData::I64(v) => v[o] as u64,
            ArrData::F32(v) => f64::from(v[o]).to_bits(),
            ArrData::F64(v) => v[o].to_bits(),
            ArrData::Str(_) => 0,
        }
    }

    /// store register bits (already converted to the element type)
    #[inline(always)]
    fn set(&mut self, o: usize, b: u64) {
        match &mut self.data {
            ArrData::I16(v) => v[o] = b as i64 as i16,
            ArrData::I32(v) => v[o] = b as i64 as i32,
            ArrData::I64(v) => v[o] = b as i64,
            ArrData::F32(v) => v[o] = f64::from_bits(b) as f32,
            ArrData::F64(v) => v[o] = f64::from_bits(b),
            ArrData::Str(_) => {}
        }
    }

    fn strs(&mut self) -> &mut Vec<Vec<u8>> {
        match &mut self.data {
            ArrData::Str(v) => v,
            _ => unreachable!("string access to numeric array"),
        }
    }

    /// QB64 `REDIM _PRESERVE`: keeps elements by linear position.
    fn preserve_from(&mut self, old: Arr) {
        match (&mut self.data, old.data) {
            (ArrData::I16(d), ArrData::I16(s)) => copy_prefix(d, s),
            (ArrData::I32(d), ArrData::I32(s)) => copy_prefix(d, s),
            (ArrData::I64(d), ArrData::I64(s)) => copy_prefix(d, s),
            (ArrData::F32(d), ArrData::F32(s)) => copy_prefix(d, s),
            (ArrData::F64(d), ArrData::F64(s)) => copy_prefix(d, s),
            (ArrData::Str(d), ArrData::Str(s)) => copy_prefix(d, s),
            _ => {}
        }
    }
}

fn copy_prefix<T>(d: &mut [T], s: Vec<T>) {
    for (x, y) in d.iter_mut().zip(s) {
        *x = y;
    }
}

// ---------------------------------------------------------------- events

struct EvShared {
    pending: AtomicU32,
}

/// Lets other threads (timer, keyboard, ...) raise events. Event 0 is TIMER.
#[derive(Clone)]
pub struct EventHandle(Arc<EvShared>);

impl EventHandle {
    pub fn raise(&self, id: u32) {
        if id < 32 {
            self.0.pending.fetch_or(1 << id, Ordering::Release);
        }
    }
}

// ---------------------------------------------------------------- machine

#[derive(Debug, Clone, Copy)]
struct Frame {
    func: u32,
    ret: u32,
    nb: usize,
    sb: usize,
    ab: usize,
    caller_view: usize,
    desc: u32,
}

#[derive(Debug, Clone, Copy)]
struct Gs {
    ret: u32,
    depth: usize,
    view: usize,
    event: bool,
}

/// Register window of a frame (for places evaluated in the caller).
#[derive(Clone, Copy)]
struct Ctx {
    nb: usize,
    sb: usize,
    ab: usize,
    func: u32,
}

enum PVal {
    N(u64),
    S(Vec<u8>),
    A(Option<Box<Arr>>),
}

struct Saved {
    nums: Vec<u64>,
    strs: Vec<Vec<u8>>,
    arrs: Vec<Option<Box<Arr>>>,
}

pub struct Vm<'m, 'o> {
    m: &'m Module,
    out: &'o mut dyn Write,
    nums: Vec<u64>,
    strs: Vec<Vec<u8>>,
    arrs: Vec<Option<Box<Arr>>>,
    frames: Vec<Frame>,
    view: usize,
    nb: usize,
    sb: usize,
    ab: usize,
    gosub: Vec<Gs>,
    statics: Vec<Option<Saved>>,
    col: usize,
    on_error: Option<u32>,
    /// (pc of the failing instruction, view) while an error handler runs
    in_handler: Option<(u32, usize)>,
    err: i32,
    rnd: qb_runtime::rnd::RndState,
    data_ptr: usize,
    ev: Arc<EvShared>,
    ev_mode: [EvMode; 32],
    ev_handler: [u32; 32],
    in_event: bool,
    /// pending bits examined at safe points
    poll_mask: u32,
    /// JIT tier (R11.6); `None` = interpret only (`QB_JIT=off`).
    jit: Option<Box<crate::jit::JitState>>,
    /// `exec` returns when a RET brings the frame count back to this
    /// (calls made from native code run the callee to completion).
    stop_depth: usize,
    /// native -> VM -> native nesting of such calls
    jit_nest: u32,
    timer_interval: f64,
    timer: Option<Arc<AtomicBool>>,
}

impl Drop for Vm<'_, '_> {
    fn drop(&mut self) {
        self.stop_timer();
    }
}

#[inline(always)]
fn b(v: bool) -> u64 {
    if v {
        -1i64 as u64
    } else {
        0
    }
}

fn to_val(bits: u64, ty: Ty) -> Val {
    match ty {
        Ty::I16 => Val::I(bits as i64 as i16),
        Ty::I32 => Val::L(bits as i64 as i32),
        Ty::I64 => Val::Q(bits as i64),
        Ty::F32 => Val::F(f64::from_bits(bits)),
        Ty::F64 => Val::D(f64::from_bits(bits)),
        Ty::Str => Val::S(Vec::new()),
    }
}

/// Builtin result -> register bits of the static result type.
fn from_val(v: &Val, ty: Ty) -> Result<u64, i32> {
    Ok(match ty {
        Ty::I16 | Ty::I32 | Ty::I64 => {
            let x = v.i()?;
            ty.width().wrap(x) as u64
        }
        Ty::F32 | Ty::F64 => v.f()?.to_bits(),
        Ty::Str => return Err(13),
    })
}

impl<'m, 'o> Vm<'m, 'o> {
    pub fn new(m: &'m Module, out: &'o mut dyn Write) -> Self {
        let main = &m.funcs[0];
        Vm {
            m,
            out,
            nums: main.num_init.clone(),
            strs: vec![Vec::new(); main.n_str as usize],
            arrs: (0..main.n_arr()).map(|_| None).collect(),
            frames: vec![Frame { func: 0, ret: 0, nb: 0, sb: 0, ab: 0, caller_view: 0, desc: 0 }],
            view: 0,
            nb: 0,
            sb: 0,
            ab: 0,
            gosub: Vec::new(),
            statics: (0..m.funcs.len()).map(|_| None).collect(),
            col: 1,
            on_error: None,
            in_handler: None,
            err: 0,
            rnd: qb_runtime::rnd::RndState::default(),
            data_ptr: 0,
            ev: Arc::new(EvShared { pending: AtomicU32::new(0) }),
            ev_mode: [EvMode::Off; 32],
            ev_handler: [NONE; 32],
            in_event: false,
            poll_mask: u32::MAX,
            jit: crate::jit::JitState::from_env(m),
            stop_depth: usize::MAX,
            jit_nest: 0,
            timer_interval: 0.0,
            timer: None,
        }
    }

    /// Selects the JIT mode (overrides `QB_JIT`).
    pub fn set_jit(&mut self, mode: qb_jit::tier::Mode, threshold: u32) {
        self.jit = crate::jit::JitState::new(self.m, mode, threshold);
    }

    /// Why the JIT is not running native code, if it tried and failed.
    pub fn jit_failure(&self) -> Option<String> {
        self.jit.as_ref().and_then(|j| j.failure())
    }

    /// Number of times native code was entered.
    pub fn jit_entries(&self) -> u64 {
        self.jit.as_ref().map_or(0, |j| j.entries)
    }

    /// Runs native code for the function containing `pc`, entered at `pc`
    /// (a back-edge target) on the current register window. `None` = not
    /// compiled (yet); `Ok((pc, event))` = continue interpreting at `pc`
    /// (dispatch events first when `event`).
    #[inline(never)]
    fn jit_enter(&mut self, pc: u32, np: *mut u64) -> Option<Result<(u32, bool), (i32, u32)>> {
        if !self.jit.as_deref()?.wants(pc) {
            return None;
        }
        let me = self as *mut Self as *mut core::ffi::c_void;
        let globals = self.nums.as_mut_ptr();
        let ev: *const AtomicU32 = &self.ev.pending;
        let mask = self.poll_mask;
        let js: &mut crate::jit::JitState = self.jit.as_deref_mut()?;
        // SAFETY: native code works on the current window `np` and calls back
        // into this VM only through `vm_op` (one non-control instruction).
        let hosts = crate::jit::Hosts {
            vm_op,
            lde1: jit_lde1 as *const () as usize,
            ste1: jit_ste1 as *const () as usize,
            lde2: jit_lde2 as *const () as usize,
            ste2: jit_ste2 as *const () as usize,
            lde1g: jit_lde1g as *const () as usize,
            gosub: jit_gosub as *const () as usize,
            greturn: jit_return as *const () as usize,
            ste1g: jit_ste1g as *const () as usize,
            view_off: core::mem::offset_of!(Arr, view) as u32,
        };
        let arrs = self.arrs.as_mut_ptr() as *mut u64;
        // SAFETY: `ab` + the frame's array count is inside `arrs`; Option<Box<Arr>> is one nullable pointer.
        let tables = [unsafe { arrs.add(self.ab) }, arrs];
        let st = unsafe { js.enter(pc, np, globals, ev, mask, me, &hosts, tables)? };
        Some(st)
    }

    /// CALL at `pc` made from native code: the VM pushes the frame and runs
    /// the callee to its RET (entering native code again for its loops).
    /// `Ok(next)`: back in the caller; `Ok(u32::MAX)`: END inside the callee.
    /// Deep native/VM nesting is left to the VM (`Ok(pc)`: not executed).
    fn call_from_native(&mut self, pc: u32) -> Result<u32, (i32, u32)> {
        if self.jit_nest >= 16 {
            return Ok(pc);
        }
        let depth = self.frames.len();
        let entry = self.step_one(pc)?;
        if self.frames.len() == depth {
            return Ok(entry);
        }
        let saved = self.stop_depth;
        self.stop_depth = depth;
        self.jit_nest += 1;
        let r = self.exec_impl::<false>(entry);
        self.jit_nest -= 1;
        self.stop_depth = saved;
        r
    }

    /// Executes exactly the instruction at `pc` (used by native code for
    /// everything it does not compile itself). Returns the next pc.
    pub(crate) fn step_one(&mut self, pc: u32) -> Result<u32, (i32, u32)> {
        self.exec_impl::<true>(pc)
    }

    /// Handle for raising events from other threads.
    pub fn events(&self) -> EventHandle {
        EventHandle(self.ev.clone())
    }

    pub fn run(&mut self) -> Result<(), RunError> {
        let mut pc = self.m.funcs[0].entry;
        let r = loop {
            match self.exec(pc) {
                Ok(()) => break Ok(()),
                Err((code, at)) => match self.raise(code, at) {
                    Ok(h) => pc = h,
                    Err(e) => break Err(e),
                },
            }
        };
        self.stop_timer();
        let f = self.out.flush().map_err(|e| RunError { code: 57, line: 0, msg: e.to_string() });
        r.and(f)
    }

    #[inline(always)]
    fn np(&mut self) -> *mut u64 {
        // SAFETY: nb + frame size <= nums.len() (frames are pushed with their full size)
        unsafe { self.nums.as_mut_ptr().add(self.nb) }
    }

    fn set_view(&mut self, v: usize) {
        self.view = v;
        let f = self.frames[v];
        self.nb = f.nb;
        self.sb = f.sb;
        self.ab = f.ab;
        self.update_poll_mask();
    }

    fn ctx(&self, view: usize) -> Ctx {
        let f = self.frames[view];
        Ctx { nb: f.nb, sb: f.sb, ab: f.ab, func: f.func }
    }

    // ------------------------------------------------------------ strings

    #[inline(always)]
    fn sget(&self, r: R) -> &[u8] {
        if r & K != 0 {
            &self.m.kstr[(r & !K) as usize]
        } else {
            &self.strs[self.sb + r as usize]
        }
    }

    fn sget_in(&self, sb: usize, r: R) -> &[u8] {
        if r & K != 0 {
            &self.m.kstr[(r & !K) as usize]
        } else {
            &self.strs[sb + r as usize]
        }
    }

    #[inline(always)]
    fn sdst(&mut self, r: R) -> &mut Vec<u8> {
        let i = self.sb + r as usize;
        &mut self.strs[i]
    }

    /// d = s (copy, reusing d's buffer)
    fn smov(&mut self, d: R, s: R) {
        if s == d {
            return;
        }
        let src: *const [u8] = self.sget(s);
        let dst = self.sdst(d);
        dst.clear();
        // SAFETY: s != d, so src is another buffer (or a constant) that `dst` does not alias
        dst.extend_from_slice(unsafe { &*src });
    }

    fn concat(&mut self, d: R, a: R, bb: R) {
        if a == d {
            if bb == d {
                let dst = self.sdst(d);
                let n = dst.len();
                dst.extend_from_within(..n);
            } else {
                let src: *const [u8] = self.sget(bb);
                // SAFETY: bb != d
                self.sdst(d).extend_from_slice(unsafe { &*src });
            }
        } else if bb == d {
            let x = self.sget(a);
            let y = self.sget(bb);
            let mut v = Vec::with_capacity(x.len() + y.len());
            v.extend_from_slice(x);
            v.extend_from_slice(y);
            *self.sdst(d) = v;
        } else {
            let x: *const [u8] = self.sget(a);
            let y: *const [u8] = self.sget(bb);
            let dst = self.sdst(d);
            dst.clear();
            // SAFETY: a != d and bb != d
            unsafe {
                dst.extend_from_slice(&*x);
                dst.extend_from_slice(&*y);
            }
        }
    }

    /// d = s[from..to] (d may be s)
    fn ssub(&mut self, d: R, s: R, from: usize, to: usize) {
        if d == s {
            let v = self.sdst(d);
            v.truncate(to);
            v.drain(..from);
        } else {
            let src: *const [u8] = &self.sget(s)[from..to];
            let dst = self.sdst(d);
            dst.clear();
            // SAFETY: d != s
            dst.extend_from_slice(unsafe { &*src });
        }
    }

    fn emit(&mut self, bytes: &[u8]) -> Result<(), i32> {
        self.out.write_all(bytes).map_err(|_| 57)?;
        match bytes.iter().rposition(|&c| c == b'\n') {
            Some(p) => self.col = bytes.len() - p,
            None => self.col += bytes.len(),
        }
        Ok(())
    }

    fn print_s(&mut self, s: R) -> Result<(), i32> {
        let p: *const [u8] = self.sget(s);
        // SAFETY: emit does not touch the string registers
        self.emit(unsafe { &*p })
    }

    fn print_n(&mut self, bits: u64, ty: Ty) -> Result<(), i32> {
        let mut t = str_of(&to_val(bits, ty))?;
        t.push(b' ');
        self.emit(&t)
    }

    // ------------------------------------------------------------ arrays

    fn arr_index(&self, a: R, ab: usize) -> usize {
        if a & G != 0 {
            (a & !G) as usize
        } else {
            ab + a as usize
        }
    }

    /// Array for element access; an undeclared array gets QBasic's implicit
    /// `DIM name(10)` per subscript.
    fn arr_at(&mut self, a: R, n: usize, cx: Ctx) -> Result<&mut Arr, i32> {
        let i = self.arr_index(a, cx.ab);
        if self.arrs[i].is_none() {
            let f = if a & G != 0 { 0 } else { cx.func as usize };
            let ty = self.m.funcs[f].arr_tys[(a & !G) as usize];
            self.arrs[i] = Some(Box::new(Arr::new(ty, vec![(0, 10); n])?));
        }
        Ok(self.arrs[i].as_deref_mut().unwrap())
    }

    #[inline(always)]
    fn arr(&mut self, a: R, n: usize) -> Result<&mut Arr, i32> {
        let cx = self.cur_ctx();
        self.arr_at(a, n, cx)
    }

    fn cur_ctx(&self) -> Ctx {
        Ctx { nb: self.nb, sb: self.sb, ab: self.ab, func: self.frames[self.view].func }
    }

    fn dim(&mut self, a: R, dims_r: R, n: u32, mode: DimMode) -> Result<(), i32> {
        let mut dims = Vec::with_capacity(n as usize);
        for k in 0..n as usize {
            let lb = self.nums[self.nb + dims_r as usize + 2 * k] as i64;
            let ub = self.nums[self.nb + dims_r as usize + 2 * k + 1] as i64;
            dims.push((lb, ub));
        }
        let cx = self.cur_ctx();
        let i = self.arr_index(a, cx.ab);
        let f = if a & G != 0 { 0 } else { cx.func as usize };
        let ty = self.m.funcs[f].arr_tys[(a & !G) as usize];
        if mode == DimMode::Dim && self.arrs[i].is_some() {
            return Err(10); // duplicate definition
        }
        let mut new = Arr::new(ty, dims)?;
        if mode == DimMode::RedimPreserve {
            if let Some(old) = self.arrs[i].take() {
                new.preserve_from(*old);
            }
        }
        self.arrs[i] = Some(Box::new(new));
        Ok(())
    }

    fn elem_off(&mut self, a: R, i: R, n: u32, cx: Ctx) -> Result<(usize, usize), i32> {
        let start = cx.nb + i as usize;
        let idx: Vec<i64> = self.nums[start..start + n as usize].iter().map(|&x| x as i64).collect();
        let ai = self.arr_index(a, cx.ab);
        let arr = self.arr_at(a, n as usize, cx)?;
        let o = arr.off(idx.into_iter(), n as usize)?;
        Ok((ai, o))
    }

    // ------------------------------------------------------------ places

    fn place_take(&mut self, p: Place, cx: Ctx) -> Result<PVal, i32> {
        Ok(match p {
            Place::N(r, _) => PVal::N(self.nums[cx.nb + r as usize]),
            Place::NG(g, _) => PVal::N(self.nums[g as usize]),
            Place::S(r) => PVal::S(std::mem::take(&mut self.strs[cx.sb + r as usize])),
            Place::SG(g) => PVal::S(std::mem::take(&mut self.strs[g as usize])),
            Place::A(a) => {
                let i = self.arr_index(a, cx.ab);
                PVal::A(self.arrs[i].take())
            }
            Place::E { a, i, n, ty } => {
                let (ai, o) = self.elem_off(a, i, n, cx)?;
                let arr = self.arrs[ai].as_deref_mut().unwrap();
                if ty == Ty::Str {
                    PVal::S(std::mem::take(&mut arr.strs()[o]))
                } else {
                    PVal::N(arr.get(o))
                }
            }
        })
    }

    fn place_put(&mut self, p: Place, cx: Ctx, v: PVal) -> Result<(), i32> {
        match (p, v) {
            (Place::N(r, _), PVal::N(x)) => self.nums[cx.nb + r as usize] = x,
            (Place::NG(g, _), PVal::N(x)) => self.nums[g as usize] = x,
            (Place::S(r), PVal::S(s)) => self.strs[cx.sb + r as usize] = s,
            (Place::SG(g), PVal::S(s)) => self.strs[g as usize] = s,
            (Place::A(a), PVal::A(x)) => {
                let i = self.arr_index(a, cx.ab);
                self.arrs[i] = x;
            }
            (Place::E { a, i, n, .. }, v) => {
                let (ai, o) = self.elem_off(a, i, n, cx)?;
                let arr = self.arrs[ai].as_deref_mut().unwrap();
                match v {
                    PVal::S(s) => arr.strs()[o] = s,
                    PVal::N(x) => arr.set(o, x),
                    PVal::A(_) => return Err(13),
                }
            }
            _ => return Err(13),
        }
        Ok(())
    }

    fn swap(&mut self, p: u32, q: u32) -> Result<(), i32> {
        let (p, q) = (self.m.places[p as usize], self.m.places[q as usize]);
        let cx = self.cur_ctx();
        let x = self.place_take(p, cx)?;
        let y = match self.place_take(q, cx) {
            Ok(y) => y,
            Err(e) => {
                self.place_put(p, cx, x)?;
                return Err(e);
            }
        };
        self.place_put(p, cx, y)?;
        self.place_put(q, cx, x)
    }

    // ------------------------------------------------------------ calls

    fn slot_put(&mut self, s: Slot, nb: usize, sb: usize, ab: usize, v: PVal) -> Result<(), i32> {
        match (s, v) {
            (Slot::N(r), PVal::N(x)) => self.nums[nb + r as usize] = x,
            (Slot::S(r), PVal::S(x)) => self.strs[sb + r as usize] = x,
            (Slot::A(r), PVal::A(x)) => self.arrs[ab + r as usize] = x,
            _ => return Err(13),
        }
        Ok(())
    }

    fn slot_take(&mut self, s: Slot, nb: usize, sb: usize, ab: usize) -> PVal {
        match s {
            Slot::N(r) => PVal::N(self.nums[nb + r as usize]),
            Slot::S(r) => PVal::S(std::mem::take(&mut self.strs[sb + r as usize])),
            Slot::A(r) => PVal::A(self.arrs[ab + r as usize].take()),
        }
    }

    fn call(&mut self, f: u32, desc: u32, ret: u32) -> Result<u32, i32> {
        let m = self.m;
        let func = &m.funcs[f as usize];
        let cd = &m.calls[desc as usize];
        let cx = self.cur_ctx();
        let (nb, sb, ab) = (self.nums.len(), self.strs.len(), self.arrs.len());
        if self.frames.len() > 100_000 {
            return Err(7); // out of stack space (QB: error 7 / 28)
        }
        self.nums.extend_from_slice(&func.num_init);
        self.strs.resize_with(sb + func.n_str as usize, Vec::new);
        self.arrs.resize_with(ab + func.n_arr() as usize, || None);
        if func.is_static {
            if let Some(s) = self.statics[f as usize].take() {
                self.nums[nb..].copy_from_slice(&s.nums);
                for (d, x) in self.strs[sb..].iter_mut().zip(s.strs) {
                    *d = x;
                }
                for (d, x) in self.arrs[ab..].iter_mut().zip(s.arrs) {
                    *d = x;
                }
                match func.ret {
                    Some((Slot::N(r), _)) => self.nums[nb + r as usize] = 0,
                    Some((Slot::S(r), _)) => self.strs[sb + r as usize].clear(),
                    _ => {}
                }
            }
        }
        for (arg, &slot) in cd.args.iter().zip(&func.params) {
            let v = match *arg {
                Arg::N(r) => PVal::N(self.nums[cx.nb + r as usize]),
                Arg::S(r) => PVal::S(self.sget_in(cx.sb, r).to_vec()),
                Arg::SMove(r) => PVal::S(std::mem::take(&mut self.strs[cx.sb + r as usize])),
                Arg::Ref(p) => match self.place_take(m.places[p as usize], cx) {
                    Ok(v) => v,
                    Err(e) => {
                        self.nums.truncate(nb);
                        self.strs.truncate(sb);
                        self.arrs.truncate(ab);
                        return Err(e);
                    }
                },
            };
            self.slot_put(slot, nb, sb, ab, v)?;
        }
        self.frames.push(Frame { func: f, ret, nb, sb, ab, caller_view: self.view, desc });
        self.set_view(self.frames.len() - 1);
        Ok(func.entry)
    }

    fn ret(&mut self) -> Result<u32, i32> {
        let m = self.m;
        if self.frames.len() < 2 {
            return Err(51);
        }
        let fr = self.frames.pop().unwrap();
        let func = &m.funcs[fr.func as usize];
        let cd = &m.calls[fr.desc as usize];
        let cx = self.ctx(fr.caller_view);
        if let (Some(d), Some((slot, _))) = (cd.dest, func.ret) {
            match slot {
                Slot::N(r) => self.nums[cx.nb + d as usize] = self.nums[fr.nb + r as usize],
                Slot::S(r) => self.strs[cx.sb + d as usize] = std::mem::take(&mut self.strs[fr.sb + r as usize]),
                Slot::A(_) => {}
            }
        }
        for (arg, &slot) in cd.args.iter().zip(&func.params) {
            if let Arg::Ref(p) = *arg {
                let v = self.slot_take(slot, fr.nb, fr.sb, fr.ab);
                // the element may be gone (array erased/redimmed inside the SUB)
                let _ = self.place_put(m.places[p as usize], cx, v);
            }
        }
        if func.is_static {
            self.statics[fr.func as usize] = Some(Saved {
                nums: self.nums[fr.nb..].to_vec(),
                strs: self.strs.drain(fr.sb..).collect(),
                arrs: self.arrs.drain(fr.ab..).collect(),
            });
        }
        self.nums.truncate(fr.nb);
        self.strs.truncate(fr.sb);
        self.arrs.truncate(fr.ab);
        let depth = self.frames.len();
        self.gosub.retain(|g| g.depth <= depth);
        self.set_view(fr.caller_view);
        Ok(fr.ret)
    }

    // ------------------------------------------------------------ errors

    fn func_of(&self, pc: u32) -> &Func {
        let fs = &self.m.funcs;
        let i = fs.partition_point(|f| f.entry <= pc).saturating_sub(1);
        &fs[i]
    }

    fn line_of(&self, pc: u32) -> u32 {
        let s = &self.m.stmts;
        let i = s.partition_point(|x| x.0 <= pc);
        if i == 0 {
            0
        } else {
            s[i - 1].1
        }
    }

    fn stmt_start(&self, pc: u32) -> u32 {
        let f = self.func_of(pc);
        let s = &self.m.stmts;
        let i = s.partition_point(|x| x.0 <= pc);
        if i > 0 && s[i - 1].0 >= f.entry {
            s[i - 1].0
        } else {
            f.entry
        }
    }

    fn next_stmt(&self, pc: u32) -> u32 {
        let f = self.func_of(pc);
        let start = self.stmt_start(pc);
        let s = &self.m.stmts;
        let i = s.partition_point(|x| x.0 <= start);
        match s.get(i) {
            Some(&(p, _)) if p < f.end => p,
            _ => f.end - 1,
        }
    }

    /// Error dispatch: jump to the ON ERROR handler (module-level view, SUB
    /// frames kept so RESUME can return into them) or stop the program.
    fn raise(&mut self, code: i32, at: u32) -> Result<u32, RunError> {
        match (self.on_error, self.in_handler) {
            (Some(h), None) => {
                self.err = code;
                self.in_handler = Some((at, self.view));
                self.set_view(0);
                Ok(h)
            }
            _ => Err(RunError { code, line: self.line_of(at), msg: format!("error {code}") }),
        }
    }

    fn resume(&mut self, k: ResumeK, t: u32) -> Result<u32, i32> {
        let (at, view) = self.in_handler.take().ok_or(20)?; // RESUME without error
        Ok(match k {
            ResumeK::Same => {
                self.set_view(view);
                self.stmt_start(at)
            }
            ResumeK::Next => {
                self.set_view(view);
                self.next_stmt(at)
            }
            ResumeK::Label => {
                self.set_view(0);
                t
            }
        })
    }

    // ------------------------------------------------------------ events

    fn update_poll_mask(&mut self) {
        let mut on = 0u32;
        let mut off = 0u32;
        for (i, m) in self.ev_mode.iter().enumerate() {
            match m {
                EvMode::On => on |= 1 << i,
                EvMode::Off => off |= 1 << i,
                EvMode::Stop => {}
            }
        }
        self.poll_mask = if self.in_event || self.in_handler.is_some() { off } else { on | off };
    }

    /// Safe point with pending events: discard events that are OFF, run the
    /// handler of one that is ON as an implicit GOSUB at module level.
    #[cold]
    fn dispatch_events(&mut self, pc: u32) -> Option<u32> {
        let pending = self.ev.pending.load(Ordering::Acquire) & self.poll_mask;
        for id in 0..32u32 {
            if pending & (1 << id) == 0 {
                continue;
            }
            match self.ev_mode[id as usize] {
                EvMode::Off => {
                    self.ev.pending.fetch_and(!(1 << id), Ordering::AcqRel);
                }
                EvMode::Stop => {}
                EvMode::On => {
                    self.ev.pending.fetch_and(!(1 << id), Ordering::AcqRel);
                    let h = self.ev_handler[id as usize];
                    if h == NONE {
                        continue;
                    }
                    self.gosub.push(Gs { ret: pc, depth: self.frames.len(), view: self.view, event: true });
                    self.in_event = true;
                    self.set_view(0);
                    return Some(h);
                }
            }
        }
        None
    }

    fn event_ctl(&mut self, id: u32, mode: EvMode) {
        self.ev_mode[id as usize] = mode;
        if mode == EvMode::Off {
            self.ev.pending.fetch_and(!(1 << id), Ordering::AcqRel);
        }
        if id == 0 {
            match mode {
                EvMode::Off => self.stop_timer(),
                _ => self.start_timer(),
            }
        }
        self.update_poll_mask();
    }

    fn start_timer(&mut self) {
        if self.timer.is_some() || self.timer_interval <= 0.0 || !self.timer_interval.is_finite() {
            return;
        }
        let stop = Arc::new(AtomicBool::new(false));
        let (s, ev) = (stop.clone(), self.ev.clone());
        let iv = std::time::Duration::from_secs_f64(self.timer_interval);
        std::thread::spawn(move || {
            while !s.load(Ordering::Acquire) {
                std::thread::sleep(iv);
                if s.load(Ordering::Acquire) {
                    break;
                }
                ev.pending.fetch_or(1, Ordering::Release);
            }
        });
        self.timer = Some(stop);
    }

    fn stop_timer(&mut self) {
        if let Some(s) = self.timer.take() {
            s.store(true, Ordering::Release);
        }
    }

    // ------------------------------------------------------------ builtins

    fn builtin(&mut self, f: u32, d: R, args: u32) -> Result<(), i32> {
        let m = self.m;
        let (name, rt) = &m.bfns[f as usize];
        let vals: Vec<Val> = m.bargs[args as usize]
            .iter()
            .map(|&(r, ty)| if ty == Ty::Str { Val::S(self.sget(r).to_vec()) } else { to_val(self.nums[self.nb + r as usize], ty) })
            .collect();
        let v = crate::builtins::call(name, vals)?;
        if *rt == Ty::Str {
            *self.sdst(d) = v.s()?;
        } else {
            let bits = from_val(&v, *rt)?;
            self.nums[self.nb + d as usize] = bits;
        }
        Ok(())
    }

    // ------------------------------------------------------------ hot loop

    /// Runs from `pc` until END (`Ok`) or an error (`Err((code, pc))`).
    fn exec(&mut self, pc: u32) -> Result<(), (i32, u32)> {
        self.exec_impl::<false>(pc).map(|_| ())
    }

    /// The interpreter loop. `STEP`: return after one instruction with the next pc.
    #[inline(always)]
    fn exec_impl<const STEP: bool>(&mut self, mut pc: u32) -> Result<u32, (i32, u32)> {
        let m: &'m Module = self.m;
        let mut stepped = false;
        let code = m.code.as_slice();
        let evp: *const AtomicU32 = &self.ev.pending;
        let mut np = self.np();
        macro_rules! r {
            ($x:expr) => {
                // SAFETY: operand < frame size (Module::validate), window inside `nums`
                unsafe { *np.add($x as usize) }
            };
        }
        macro_rules! ri {
            ($x:expr) => {
                (r!($x) as i64)
            };
        }
        macro_rules! rf {
            ($x:expr) => {
                f64::from_bits(r!($x))
            };
        }
        macro_rules! set {
            ($x:expr, $v:expr) => {{
                let v: u64 = $v;
                // SAFETY: as for r!
                unsafe { *np.add($x as usize) = v }
            }};
        }
        macro_rules! seti {
            ($x:expr, $v:expr) => {
                set!($x, ($v) as u64)
            };
        }
        macro_rules! setf {
            ($x:expr, $v:expr) => {
                set!($x, f64::to_bits($v))
            };
        }
        macro_rules! fail {
            ($c:expr) => {
                return Err(($c, pc - 1))
            };
        }
        macro_rules! tryc {
            ($e:expr) => {
                match $e {
                    Ok(v) => v,
                    Err(c) => fail!(c),
                }
            };
        }
        macro_rules! reload {
            () => {
                np = self.np()
            };
        }
        macro_rules! poll {
            () => {
                // SAFETY: evp points into self.ev, alive as long as self
                if unsafe { (*evp).load(Ordering::Relaxed) } & self.poll_mask != 0 {
                    if let Some(h) = self.dispatch_events(pc) {
                        pc = h;
                        reload!();
                    }
                }
            };
        }
        macro_rules! jit {
            () => {
                if !STEP && self.jit.as_deref().is_some_and(|j| j.wants(pc)) {
                    match self.jit_enter(pc, np) {
                        None => {}
                        Some(Ok((p, _))) if p == u32::MAX => return Ok(u32::MAX),
                        Some(Ok((p, ev))) => {
                            pc = p;
                            reload!();
                            if ev {
                                poll!();
                            }
                        }
                        Some(Err(e)) => return Err(e),
                    }
                }
            };
        }
        loop {
            if STEP {
                if stepped {
                    return Ok(pc);
                }
                stepped = true;
            }
            // SAFETY: every jump target is inside its function and every function
            // ends with ret/end (Module::validate), so pc stays in bounds
            let ins = unsafe { *code.get_unchecked(pc as usize) };
            pc += 1;
            match ins {
                Ins::Ldi { d, v } => seti!(d, v as i64),
                Ins::Ldk { d, k } => set!(d, m.nconst[k as usize]),
                Ins::Mov { d, s } => set!(d, r!(s)),
                Ins::GetG { d, g } => set!(d, self.nums[g as usize]),
                Ins::SetG { g, s } => {
                    let v = r!(s);
                    self.nums[g as usize] = v;
                }
                Ins::IToF { d, s } => setf!(d, ri!(s) as f64),
                Ins::FToI { d, s, w } => {
                    let x = qb_runtime::round::qb_qbr_f64(rf!(s));
                    if w != W::W64 && w.wrap(x) != x {
                        fail!(6);
                    }
                    seti!(d, x)
                }
                Ins::Wrap { d, s, w } => seti!(d, w.wrap(ri!(s))),
                Ins::RoundF32 { d, s } => setf!(d, f64::from(rf!(s) as f32)),
                Ins::AddI16 { d, a, b } => seti!(d, (ri!(a) + ri!(b)) as i16 as i64),
                Ins::AddI32 { d, a, b } => seti!(d, (ri!(a) + ri!(b)) as i32 as i64),
                Ins::AddI64 { d, a, b } => seti!(d, ri!(a).wrapping_add(ri!(b))),
                Ins::SubI16 { d, a, b } => seti!(d, (ri!(a) - ri!(b)) as i16 as i64),
                Ins::SubI32 { d, a, b } => seti!(d, (ri!(a) - ri!(b)) as i32 as i64),
                Ins::SubI64 { d, a, b } => seti!(d, ri!(a).wrapping_sub(ri!(b))),
                Ins::MulI16 { d, a, b } => seti!(d, (ri!(a) * ri!(b)) as i16 as i64),
                Ins::MulI32 { d, a, b } => seti!(d, ri!(a).wrapping_mul(ri!(b)) as i32 as i64),
                Ins::MulI64 { d, a, b } => seti!(d, ri!(a).wrapping_mul(ri!(b))),
                Ins::IDiv { d, a, b, w } => {
                    let y = ri!(b);
                    if y == 0 {
                        fail!(11);
                    }
                    seti!(d, w.wrap(ri!(a).wrapping_div(y)))
                }
                Ins::Mod { d, a, b, w } => {
                    let y = ri!(b);
                    if y == 0 {
                        fail!(11);
                    }
                    seti!(d, w.wrap(ri!(a).wrapping_rem(y)))
                }
                Ins::And { d, a, b } => seti!(d, ri!(a) & ri!(b)),
                Ins::Or { d, a, b } => seti!(d, ri!(a) | ri!(b)),
                Ins::Xor { d, a, b } => seti!(d, ri!(a) ^ ri!(b)),
                Ins::Eqv { d, a, b } => seti!(d, !(ri!(a) ^ ri!(b))),
                Ins::Imp { d, a, b } => seti!(d, !ri!(a) | ri!(b)),
                Ins::AddF { d, a, b } => setf!(d, rf!(a) + rf!(b)),
                Ins::SubF { d, a, b } => setf!(d, rf!(a) - rf!(b)),
                Ins::MulF { d, a, b } => setf!(d, rf!(a) * rf!(b)),
                Ins::DivF { d, a, b } => {
                    let y = rf!(b);
                    if y == 0.0 {
                        fail!(11);
                    }
                    setf!(d, rf!(a) / y)
                }
                Ins::PowF { d, a, b } => setf!(d, rf!(a).powf(rf!(b))),
                Ins::Un { op, d, s } => match op {
                    Un::NegI(w) => seti!(d, w.wrap(ri!(s).wrapping_neg())),
                    Un::NegF => setf!(d, -rf!(s)),
                    Un::Not => seti!(d, !ri!(s)),
                    Un::AbsI(w) => seti!(d, w.wrap(ri!(s).wrapping_abs())),
                    Un::AbsF => setf!(d, rf!(s).abs()),
                    Un::SgnI => seti!(d, ri!(s).signum()),
                    Un::SgnF => {
                        let x = rf!(s);
                        seti!(d, if x > 0.0 { 1i64 } else if x < 0.0 { -1 } else { 0 })
                    }
                    Un::Int => setf!(d, rf!(s).floor()),
                    Un::Fix => setf!(d, rf!(s).trunc()),
                    Un::Sqr => {
                        let x = rf!(s);
                        if x < 0.0 {
                            fail!(5);
                        }
                        setf!(d, x.sqrt())
                    }
                    Un::Log => {
                        let x = rf!(s);
                        if x <= 0.0 {
                            fail!(5);
                        }
                        setf!(d, x.ln())
                    }
                    Un::Exp => setf!(d, rf!(s).exp()),
                    Un::Sin => setf!(d, rf!(s).sin()),
                    Un::Cos => setf!(d, rf!(s).cos()),
                    Un::Tan => setf!(d, rf!(s).tan()),
                    Un::Atn => setf!(d, rf!(s).atan()),
                },
                Ins::CmpI { c, d, a, b: y } => set!(d, b(c.test(ri!(a), ri!(y)))),
                Ins::CmpF { c, d, a, b: y } => set!(d, b(c.test(rf!(a), rf!(y)))),
                Ins::CmpS { c, d, a, b: y } => {
                    let v = c.test(self.sget(a), self.sget(y));
                    set!(d, b(v))
                }
                Ins::Jmp { t } => pc = t,
                Ins::Loop { t } => {
                    pc = t;
                    poll!();
                    jit!();
                }
                Ins::JzI { s, t } => {
                    if r!(s) == 0 {
                        pc = t
                    }
                }
                Ins::JnzI { s, t } => {
                    if r!(s) != 0 {
                        pc = t
                    }
                }
                Ins::JzF { s, t } => {
                    if rf!(s) == 0.0 {
                        pc = t
                    }
                }
                Ins::JnzF { s, t } => {
                    if rf!(s) != 0.0 {
                        pc = t
                    }
                }
                Ins::BrI { c, a, b: y, t } => {
                    if c.test(ri!(a), ri!(y)) {
                        pc = t
                    }
                }
                Ins::BrF { c, a, b: y, t } => {
                    if c.test(rf!(a), rf!(y)) {
                        pc = t
                    }
                }
                Ins::BrS { c, a, b: y, t } => {
                    if c.test(self.sget(a), self.sget(y)) {
                        pc = t
                    }
                }
                Ins::ForTestI { v, lim, step, t } => {
                    let (x, l) = (ri!(v), ri!(lim));
                    if if ri!(step) < 0 { x < l } else { x > l } {
                        pc = t
                    }
                }
                Ins::ForTestF { v, lim, step, t } => {
                    let (x, l) = (rf!(v), rf!(lim));
                    if if rf!(step) < 0.0 { x < l } else { x > l } {
                        pc = t
                    }
                }
                Ins::ForNextI { v, step, lim, body, w } => {
                    let s = ri!(step);
                    let x = w.wrap(ri!(v).wrapping_add(s));
                    seti!(v, x);
                    let l = ri!(lim);
                    if if s < 0 { x >= l } else { x <= l } {
                        pc = body;
                        poll!();
                        jit!();
                    }
                }
                Ins::Gosub { t } => {
                    self.gosub.push(Gs { ret: pc, depth: self.frames.len(), view: self.view, event: false });
                    pc = t;
                }
                Ins::Return => match self.gosub.last().copied() {
                    Some(g) if g.depth == self.frames.len() => {
                        self.gosub.pop();
                        pc = g.ret;
                        if g.event {
                            self.in_event = false;
                        }
                        if g.view != self.view || g.event {
                            self.set_view(g.view);
                            reload!();
                        }
                    }
                    _ => fail!(3),
                },
                Ins::Call { f, desc } => {
                    pc = tryc!(self.call(f, desc, pc));
                    reload!();
                    poll!();
                }
                Ins::Ret => {
                    pc = tryc!(self.ret());
                    reload!();
                    if !STEP && self.frames.len() == self.stop_depth {
                        return Ok(pc);
                    }
                }
                Ins::End => return Ok(u32::MAX),
                Ins::OnError { t } => self.on_error = (t != NONE).then_some(t),
                Ins::Resume { k, t } => {
                    pc = tryc!(self.resume(k, t));
                    reload!();
                }
                Ins::Raise { s } => {
                    let c = ri!(s);
                    if !(1..=255).contains(&c) {
                        fail!(5);
                    }
                    fail!(c as i32);
                }
                Ins::RaiseK { code } => fail!(code as i32),
                Ins::MovS { d, s } => self.smov(d, s),
                Ins::TakeS { d, s } => {
                    let (i, j) = (self.sb + d as usize, self.sb + s as usize);
                    self.strs.swap(i, j);
                }
                Ins::GetGS { d, g } => {
                    let src: *const [u8] = self.strs[g as usize].as_slice();
                    let dst = self.sdst(d);
                    if !std::ptr::eq(dst.as_slice(), src) {
                        dst.clear();
                        // SAFETY: distinct registers
                        dst.extend_from_slice(unsafe { &*src });
                    }
                }
                Ins::SetGS { g, s } => {
                    let v = self.sget(s).to_vec();
                    self.strs[g as usize] = v;
                }
                Ins::Concat { d, a, b: y } => self.concat(d, a, y),
                Ins::LenS { d, s } => seti!(d, self.sget(s).len() as i64),
                Ins::Chr { d, s } => {
                    let c = ri!(s);
                    if !(0..=255).contains(&c) {
                        fail!(5);
                    }
                    let dst = self.sdst(d);
                    dst.clear();
                    dst.push(c as u8);
                }
                Ins::Asc { d, s } => {
                    let c = match self.sget(s).first() {
                        Some(&c) => c,
                        None => fail!(5),
                    };
                    seti!(d, c as i64)
                }
                Ins::Left { d, s, n } => {
                    let k = ri!(n);
                    if k < 0 {
                        fail!(5);
                    }
                    let len = self.sget(s).len();
                    self.ssub(d, s, 0, (k as usize).min(len));
                }
                Ins::Right { d, s, n } => {
                    let k = ri!(n);
                    if k < 0 {
                        fail!(5);
                    }
                    let len = self.sget(s).len();
                    self.ssub(d, s, len - (k as usize).min(len), len);
                }
                Ins::Mid { d, s, start, n } => {
                    let st = ri!(start);
                    if st < 1 {
                        fail!(5);
                    }
                    let len = self.sget(s).len();
                    let from = (st as usize - 1).min(len);
                    let to = if n == NONE {
                        len
                    } else {
                        let k = ri!(n);
                        if k < 0 {
                            fail!(5);
                        }
                        from.saturating_add(k as usize).min(len)
                    };
                    self.ssub(d, s, from, to);
                }
                Ins::Bi { f, d, args } => tryc!(self.builtin(f, d, args)),
                Ins::PrintS { s } => tryc!(self.print_s(s)),
                Ins::PrintN { s, ty } => tryc!(self.print_n(r!(s), ty)),
                Ins::PrintZone => {
                    let next = ((self.col - 1) / 14 + 1) * 14 + 1;
                    let pad = vec![b' '; next - self.col];
                    tryc!(self.emit(&pad));
                }
                Ins::PrintNl => tryc!(self.emit(b"\n")),
                Ins::Dim { a, dims, n, mode } => tryc!(self.dim(a, dims, n, mode)),
                Ins::LdE1 { d, a, i } => {
                    let x = ri!(i);
                    let arr = tryc!(self.arr(a, 1));
                    let o = tryc!(arr.off1(x));
                    let v = arr.get(o);
                    set!(d, v)
                }
                Ins::StE1 { a, i, s } => {
                    let (x, v) = (ri!(i), r!(s));
                    let arr = tryc!(self.arr(a, 1));
                    let o = tryc!(arr.off1(x));
                    arr.set(o, v);
                }
                Ins::LdE1S { d, a, i } => {
                    let x = ri!(i);
                    let arr = tryc!(self.arr(a, 1));
                    let o = tryc!(arr.off1(x));
                    let v = arr.strs()[o].clone();
                    *self.sdst(d) = v;
                }
                Ins::StE1S { a, i, s } => {
                    let x = ri!(i);
                    let v = self.sget(s).to_vec();
                    let arr = tryc!(self.arr(a, 1));
                    let o = tryc!(arr.off1(x));
                    arr.strs()[o] = v;
                }
                Ins::LdE { d, a, i, n } => {
                    let cx = self.cur_ctx();
                    let (ai, o) = tryc!(self.elem_off(a, i, n, cx));
                    let v = self.arrs[ai].as_deref().unwrap().get(o);
                    set!(d, v)
                }
                Ins::StE { a, i, n, s } => {
                    let v = r!(s);
                    let cx = self.cur_ctx();
                    let (ai, o) = tryc!(self.elem_off(a, i, n, cx));
                    self.arrs[ai].as_deref_mut().unwrap().set(o, v);
                }
                Ins::LdES { d, a, i, n } => {
                    let cx = self.cur_ctx();
                    let (ai, o) = tryc!(self.elem_off(a, i, n, cx));
                    let v = self.arrs[ai].as_deref_mut().unwrap().strs()[o].clone();
                    *self.sdst(d) = v;
                }
                Ins::StES { a, i, n, s } => {
                    let v = self.sget(s).to_vec();
                    let cx = self.cur_ctx();
                    let (ai, o) = tryc!(self.elem_off(a, i, n, cx));
                    self.arrs[ai].as_deref_mut().unwrap().strs()[o] = v;
                }
                Ins::Bound { d, a, dim, upper } => {
                    let k = ri!(dim);
                    let i = self.arr_index(a, self.ab);
                    let Some(arr) = self.arrs[i].as_deref() else { fail!(9) };
                    if k < 1 || k as usize > arr.dims.len() {
                        fail!(9);
                    }
                    let (lb, ub) = arr.dims[k as usize - 1];
                    seti!(d, if upper { ub } else { lb } as i32 as i64)
                }
                Ins::Swap { p, q } => {
                    tryc!(self.swap(p, q));
                }
                Ins::Read { d, str } => {
                    let Some(item) = m.data.get(self.data_ptr) else { fail!(4) }; // out of DATA
                    self.data_ptr += 1;
                    if str {
                        let dst = self.sdst(d);
                        dst.clear();
                        dst.extend_from_slice(item);
                    } else {
                        let v = tryc!(qb_runtime::val::val(item));
                        setf!(d, qb_runtime::val::f64_from_f80(v))
                    }
                }
                Ins::Restore { i } => self.data_ptr = i as usize,
                Ins::Randomize { s, using } => {
                    let seed = rf!(s);
                    if using {
                        self.rnd.randomize_using(seed);
                    } else {
                        self.rnd.randomize(seed);
                    }
                }
                Ins::Rnd { d, s } => {
                    let n = (s != NONE).then(|| rf!(s) as f32);
                    setf!(d, f64::from(self.rnd.rnd(n)))
                }
                Ins::Err { d } => seti!(d, self.err as i64),
                Ins::Erl { d } => seti!(d, 0i64),
                Ins::OnEvent { id, t } => self.ev_handler[id as usize] = t,
                Ins::EventCtl { id, mode } => self.event_ctl(id, mode),
                Ins::OnTimer { s, t } => {
                    self.timer_interval = rf!(s);
                    self.ev_handler[0] = t;
                    if self.timer.is_some() {
                        self.stop_timer();
                        self.start_timer();
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruction_stays_small() {
        assert!(std::mem::size_of::<Ins>() <= 20, "{}", std::mem::size_of::<Ins>());
    }

    #[test]
    fn arrays_store_native_element_types() {
        let mut a = Arr::new(Ty::I16, vec![(0, 3)]).unwrap();
        a.set(2, (-5i64) as u64);
        assert_eq!(a.get(2) as i64, -5);
        assert_eq!(a.ty(), Ty::I16);
        let mut f = Arr::new(Ty::F32, vec![(1, 2)]).unwrap();
        f.set(f.off1(2).unwrap(), 0.1f64.to_bits());
        assert_eq!(f64::from_bits(f.get(1)), f64::from(0.1f32));
        assert_eq!(f.off1(3), Err(9));
    }
}

/// VmOp callback for native code: executes the VM instruction `pc` on the
/// window `frame` (which is the VM's current one).
extern "C" fn vm_op(ctx: *mut qb_jit::Ctx, frame: *mut u64, pc: u32) -> u32 {
    // SAFETY: `user` was set by `jit_enter` to the VM that entered native code.
    let ctx = unsafe { &mut *ctx };
    let vm = unsafe { &mut *(ctx.user as *mut Vm<'static, 'static>) };
    debug_assert_eq!(frame, vm.np());
    let r = if matches!(vm.m.code[pc as usize], Ins::Call { .. }) { vm.call_from_native(pc) } else { vm.step_one(pc) };
    // Nested native code may have reused the context; the register file may have moved.
    ctx.user = vm as *mut Vm<'static, 'static> as *mut core::ffi::c_void;
    ctx.frame = vm.np();
    ctx.globals = vm.nums.as_mut_ptr();
    let arrs = vm.arrs.as_mut_ptr() as *mut u64;
    // SAFETY: `ab` is inside `arrs`.
    ctx.tables = [unsafe { arrs.add(vm.ab) }, arrs];
    ctx.ev_ptr = &vm.ev.pending;
    ctx.ev_mask = vm.poll_mask;
    match r {
        Ok(next) if next == pc + 1 => {
            ctx.err = 0;
            qb_jit::status::OK
        }
        Ok(u32::MAX) => {
            ctx.exit_pc = 0;
            qb_jit::status::END
        }
        Ok(next) => {
            ctx.exit_pc = next;
            qb_jit::status::DEOPT
        }
        Err((code, at)) => {
            ctx.err = code;
            ctx.exit_pc = at;
            qb_jit::status::ERROR
        }
    }
}

fn host_vm<'a>(ctx: *mut qb_jit::Ctx) -> (&'a mut qb_jit::Ctx, &'a mut Vm<'static, 'static>) {
    // SAFETY: called only by native code entered through `jit_enter`, which set `user`.
    let ctx = unsafe { &mut *ctx };
    let vm = unsafe { &mut *(ctx.user as *mut Vm<'static, 'static>) };
    (ctx, vm)
}

/// Native `ld_e1` (numeric element of a 1-D array) as register bits.
extern "C" fn jit_lde1(ctx: *mut qb_jit::Ctx, a: i64, i: i64) -> i64 {
    let (ctx, vm) = host_vm(ctx);
    match vm.arr(a as R, 1).and_then(|arr| arr.off1(i).map(|o| arr.get(o))) {
        Ok(v) => v as i64,
        Err(c) => {
            ctx.err = c;
            0
        }
    }
}

extern "C" fn jit_ste1(ctx: *mut qb_jit::Ctx, a: i64, i: i64, v: i64) {
    let (ctx, vm) = host_vm(ctx);
    if let Err(c) = vm.arr(a as R, 1).and_then(|arr| arr.off1(i).map(|o| arr.set(o, v as u64))) {
        ctx.err = c;
    }
}

extern "C" fn jit_lde2(ctx: *mut qb_jit::Ctx, a: i64, i: i64, j: i64) -> i64 {
    let (ctx, vm) = host_vm(ctx);
    match vm.arr(a as R, 2).and_then(|arr| arr.off([i, j].into_iter(), 2).map(|o| arr.get(o))) {
        Ok(v) => v as i64,
        Err(c) => {
            ctx.err = c;
            0
        }
    }
}

extern "C" fn jit_ste2(ctx: *mut qb_jit::Ctx, a: i64, i: i64, j: i64, v: i64) {
    let (ctx, vm) = host_vm(ctx);
    if let Err(c) = vm.arr(a as R, 2).and_then(|arr| arr.off([i, j].into_iter(), 2).map(|o| arr.set(o, v as u64))) {
        ctx.err = c;
    }
}

extern "C" fn jit_lde1g(ctx: *mut qb_jit::Ctx, a: i64, i: i64) -> i64 {
    jit_lde1(ctx, a | G as i64, i)
}

extern "C" fn jit_ste1g(ctx: *mut qb_jit::Ctx, a: i64, i: i64, v: i64) {
    jit_ste1(ctx, a | G as i64, i, v)
}

// Native code reads `Option<Box<Arr>>` table entries as plain pointers.
const _: () = assert!(core::mem::size_of::<Option<Box<Arr>>>() == 8);

/// Native GOSUB: push the return pc like `Ins::Gosub`.
extern "C" fn jit_gosub(ctx: *mut qb_jit::Ctx, ret: i64) {
    let (_, vm) = host_vm(ctx);
    vm.gosub.push(Gs { ret: ret as u32, depth: vm.frames.len(), view: vm.view, event: false });
}

/// Native RETURN at pc `at`: the pc to continue at. When the VM has to do
/// more than pop (event handler return, view switch) nothing is popped and
/// `at` comes back, so native code exits and the VM executes the RETURN.
extern "C" fn jit_return(ctx: *mut qb_jit::Ctx, at: i64) -> i64 {
    let (ctx, vm) = host_vm(ctx);
    match vm.gosub.last().copied() {
        Some(g) if g.depth == vm.frames.len() => {
            if g.event || g.view != vm.view {
                at
            } else {
                vm.gosub.pop();
                i64::from(g.ret)
            }
        }
        _ => {
            ctx.err = 3;
            0
        }
    }
}
