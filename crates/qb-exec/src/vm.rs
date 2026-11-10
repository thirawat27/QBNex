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
use crate::v1::rt::{self as v1rt, Host, RtState};
use crate::v1::{Stop, V};
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

#[derive(Debug, Clone)]
pub(crate) struct Arr {
    dims: Vec<(i64, i64)>,
    data: ArrData,
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
            Ty::F80 => return Err(5), // _FLOAT arrays live in memory mode
        };
        Ok(Arr { dims, data })
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
    /// memory blocks owned by the frame start here in `Vm::owned`
    owned: usize,
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
    /// console/file output and the v1 runtime state (shared with the reference interpreter)
    rt: RtState<'o>,
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
    on_error: Option<u32>,
    /// (pc of the failing instruction, view) while an error handler runs
    in_handler: Option<(u32, usize)>,
    err: i32,
    erl: u32,
    /// source line of the last error (_ERRORLINE)
    errline: u32,
    /// last numeric line label passed (QIR v1 LASTLINE; ERL reports it)
    last_line: u32,
    rnd: qb_runtime::rnd::RndState,
    data_ptr: usize,
    ev: Arc<EvShared>,
    ev_mode: [EvMode; 32],
    ev_handler: [u32; 32],
    in_event: bool,
    /// pending bits examined at safe points
    poll_mask: u32,
    timer_interval: f64,
    timer: Option<Arc<AtomicBool>>,
    /// addressable memory (QIR v1 TYPE / STRING * n / pointers)
    mem: crate::mem::Mem,
    owned: Vec<u64>,
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

/// Integer division: 32-bit `idiv` when both operands fit (much cheaper than 64-bit on x86-64).
#[inline(always)]
fn idiv(x: i64, y: i64, w: W) -> i64 {
    if w == W::U64 {
        return ((x as u64) / (y as u64)) as i64;
    }
    if w != W::W64 && x as i32 as i64 == x && y as i32 as i64 == y {
        (x as i32).wrapping_div(y as i32) as i64
    } else {
        x.wrapping_div(y)
    }
}

#[inline(always)]
fn irem(x: i64, y: i64, w: W) -> i64 {
    if w == W::U64 {
        return ((x as u64) % (y as u64)) as i64;
    }
    if w != W::W64 && x as i32 as i64 == x && y as i32 as i64 == y {
        (x as i32).wrapping_rem(y as i32) as i64
    } else {
        x.wrapping_rem(y)
    }
}

fn stop_code(s: Stop) -> i32 {
    match s {
        Stop::Err(c) => c,
        Stop::Exit(_) => 0,
    }
}

fn rt_f(v: &V) -> f64 {
    match v {
        V::F(f) => *f,
        V::X(x) => x.to_f64(),
        V::I(i) => *i as f64,
        _ => 0.0,
    }
}

fn rt_i(v: &V) -> i64 {
    match v {
        V::I(i) => *i,
        V::F(f) => *f as i64,
        V::X(x) => x.to_i64(qb_float80::Round::Zero),
        V::P(p) => *p as i64,
        V::S(_) => 0,
    }
}

impl<'o> Host<'o> for Vm<'_, 'o> {
    fn rt(&mut self) -> &mut RtState<'o> {
        &mut self.rt
    }
    fn err(&self) -> i32 {
        self.err
    }
    fn erl(&self) -> u32 {
        self.erl
    }
    fn errline(&self) -> u32 {
        self.errline
    }
    fn set_last_line(&mut self, n: u32) {
        self.last_line = n;
    }
    fn bytes(&mut self, p: u64, len: usize) -> crate::v1::R<&mut [u8]> {
        self.mem.bytes(p, len).map_err(Stop::Err)
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
        Ty::F80 => Val::D(f64::from_bits(bits)),
    }
}

/// Builtin result -> register bits of the static result type.
fn from_val(v: &Val, ty: Ty) -> Result<u64, i32> {
    Ok(match ty {
        Ty::I16 | Ty::I32 | Ty::I64 => {
            let x = v.i()?;
            ty.width().wrap(x) as u64
        }
        Ty::F32 | Ty::F64 | Ty::F80 => v.f()?.to_bits(),
        Ty::Str => return Err(13),
    })
}

impl<'m, 'o> Vm<'m, 'o> {
    pub fn new(m: &'m Module, out: &'o mut dyn Write) -> Self {
        let main = &m.funcs[0];
        Vm {
            m,
            rt: RtState::new(out),
            nums: main.num_init.clone(),
            strs: vec![Vec::new(); main.n_str as usize],
            arrs: (0..main.n_arr()).map(|_| None).collect(),
            frames: vec![Frame { owned: 0, func: 0, ret: 0, nb: 0, sb: 0, ab: 0, caller_view: 0, desc: 0 }],
            view: 0,
            nb: 0,
            sb: 0,
            ab: 0,
            gosub: Vec::new(),
            statics: (0..m.funcs.len()).map(|_| None).collect(),
            on_error: None,
            in_handler: None,
            err: 0,
            erl: 0,
            errline: 0,
            last_line: 0,
            rnd: qb_runtime::rnd::RndState::default(),
            data_ptr: 0,
            ev: Arc::new(EvShared { pending: AtomicU32::new(0) }),
            ev_mode: [EvMode::Off; 32],
            ev_handler: [NONE; 32],
            in_event: false,
            poll_mask: u32::MAX,
            timer_interval: 0.0,
            timer: None,
            mem: crate::mem::Mem::new(),
            owned: Vec::new(),
        }
    }

    /// Handle for raising events from other threads.
    pub fn events(&self) -> EventHandle {
        EventHandle(self.ev.clone())
    }

    pub fn run(&mut self) -> Result<(), RunError> {
        self.run_status().map(|_| ())
    }

    /// Runs the program; returns its exit code (`SYSTEM n`).
    pub fn run_status(&mut self) -> Result<i32, RunError> {
        let mut pc = self.m.funcs[0].entry;
        let r = loop {
            match self.exec(pc) {
                Ok(code) => break Ok(code),
                Err((code, at)) => match self.raise(code, at) {
                    Ok(h) => pc = h,
                    Err(e) => break Err(e),
                },
            }
        };
        self.stop_timer();
        self.rt.flush();
        r
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
        self.rt.write(0, bytes).map_err(stop_code)
    }

    /// QIR v1 `rt` builtin through the shared runtime. `Ok(Some(code))` = the program exits.
    fn rt_call(&mut self, f: u32, d: R, args: u32) -> Result<Option<i32>, i32> {
        let m = self.m;
        let (name, rty) = m.rtfns[f as usize];
        let vals: Vec<V> = m.bargs[args as usize]
            .iter()
            .map(|&(r, ty)| match ty {
                Ty::Str => V::S(self.sget(r).to_vec()),
                Ty::F80 => V::X(self.xget(r)),
                Ty::F32 | Ty::F64 => V::F(f64::from_bits(self.nums[self.nb + r as usize])),
                _ => V::I(self.nums[self.nb + r as usize] as i64),
            })
            .collect();
        match v1rt::call(self, name, vals) {
            Ok(v) => {
                if let (Some(ty), Some(v)) = (rty, v) {
                    match (ty, v) {
                        (Ty::Str, V::S(s)) => *self.sdst(d) = s,
                        (Ty::Str, _) => self.sdst(d).clear(),
                        (Ty::F80, v) => {
                            let x = match v {
                                V::X(x) => x,
                                V::F(f) => qb_float80::F80::from_f64(f),
                                V::I(i) => qb_float80::F80::from_i64(i),
                                _ => qb_float80::F80::from_i64(0),
                            };
                            self.xset(d, x);
                        }
                        (Ty::F32 | Ty::F64, v) => self.nums[self.nb + d as usize] = rt_f(&v).to_bits(),
                        (_, v) => self.nums[self.nb + d as usize] = rt_i(&v) as u64,
                    }
                }
                Ok(None)
            }
            Err(Stop::Exit(c)) => Ok(Some(c)),
            Err(Stop::Err(c)) => Err(c),
        }
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
        self.frames.push(Frame { owned: self.owned.len(), func: f, ret, nb, sb, ab, caller_view: self.view, desc });
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
        self.free_owned(fr.owned);
        let depth = self.frames.len();
        self.gosub.retain(|g| g.depth <= depth);
        self.set_view(fr.caller_view);
        Ok(fr.ret)
    }

    fn xget(&self, r: R) -> qb_float80::F80 {
        let b = self.nb + r as usize;
        qb_float80::F80 { mant: self.nums[b], se: self.nums[b + 1] as u16 }
    }

    fn xset(&mut self, r: R, x: qb_float80::F80) {
        let b = self.nb + r as usize;
        self.nums[b] = x.mant;
        self.nums[b + 1] = u64::from(x.se);
    }

    fn free_owned(&mut self, from: usize) {
        while self.owned.len() > from {
            let p = self.owned.pop().unwrap();
            self.mem.free_var(p);
        }
    }

    fn mem_dims(&self, dims: R, n: u32) -> Vec<(i64, i64)> {
        (0..n as usize)
            .map(|k| {
                let b = self.nb + dims as usize + 2 * k;
                (self.nums[b] as i64, self.nums[b + 1] as i64)
            })
            .collect()
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
                self.errline = self.line_of(at);
                self.erl = if self.m.v1 { self.last_line } else { self.errline };
                self.in_handler = Some((at, self.view));
                self.set_view(0);
                self.update_poll_mask();
                Ok(h)
            }
            _ => Err(RunError { code, line: self.line_of(at), msg: format!("error {code}") }),
        }
    }

    fn resume(&mut self, k: ResumeK, t: u32) -> Result<u32, i32> {
        let (at, view) = self.in_handler.take().ok_or(20)?; // RESUME without error
        self.update_poll_mask();
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
                if self.m.v1 {
                    // QIR v1: RESUME label unwinds to the main module
                    while self.frames.len() > 1 {
                        let fr = self.frames.pop().unwrap();
                        self.free_owned(fr.owned);
                        self.nums.truncate(fr.nb);
                        self.strs.truncate(fr.sb);
                        self.arrs.truncate(fr.ab);
                    }
                    self.gosub.retain(|g| g.depth <= 1);
                }
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
                    self.update_poll_mask();
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
    fn exec(&mut self, mut pc: u32) -> Result<i32, (i32, u32)> {
        let m: &'m Module = self.m;
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
        loop {
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
                Ins::IToFU { d, s } => setf!(d, r!(s) as f64),
                Ins::CmpU { c, d, a, b: y } => set!(d, b(c.test(r!(a), r!(y)))),
                Ins::BrU { c, a, b: y, t } => {
                    if c.test(r!(a), r!(y)) {
                        pc = t
                    }
                }
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
                    seti!(d, w.wrap(idiv(ri!(a), y, w)))
                }
                Ins::Mod { d, a, b, w } => {
                    let y = ri!(b);
                    if y == 0 {
                        fail!(11);
                    }
                    seti!(d, w.wrap(irem(ri!(a), y, w)))
                }
                Ins::And { d, a, b } => seti!(d, ri!(a) & ri!(b)),
                Ins::Or { d, a, b } => seti!(d, ri!(a) | ri!(b)),
                Ins::Xor { d, a, b } => seti!(d, ri!(a) ^ ri!(b)),
                Ins::Eqv { d, a, b } => seti!(d, !(ri!(a) ^ ri!(b))),
                Ins::Imp { d, a, b } => seti!(d, !ri!(a) | ri!(b)),
                Ins::Acc { op, d, a, b: y, wi, wo } => {
                    let (x, z) = (ri!(a), ri!(y));
                    let v = match op {
                        AccOp::Mul => x.wrapping_mul(z),
                        AccOp::IDiv | AccOp::Mod => {
                            if z == 0 {
                                fail!(11);
                            }
                            if op == AccOp::Mod {
                                irem(x, z, wi)
                            } else {
                                idiv(x, z, wi)
                            }
                        }
                        AccOp::And => x & z,
                        AccOp::Or => x | z,
                        AccOp::Xor => x ^ z,
                    };
                    seti!(d, wo.wrap(ri!(d).wrapping_add(wi.wrap(v))))
                }
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
                Ins::ForTestKI { v, lim, step, t, skip } => {
                    let (x, l) = (ri!(v), ri!(lim));
                    pc = if if ri!(step) < 0 { x < l } else { x > l } { t } else { pc - 1 + skip as u32 };
                }
                Ins::ForTestKF { v, lim, step, t, skip } => {
                    let (x, l) = (rf!(v), rf!(lim));
                    pc = if if rf!(step) < 0.0 { x < l } else { x > l } { t } else { pc - 1 + skip as u32 };
                }
                Ins::LoopForI { c, iv, lim, body, w } => {
                    let s = ri!(lim + 1);
                    let x = ri!(iv).wrapping_add(s);
                    seti!(c, x);
                    seti!(iv, w.wrap(x));
                    let l = ri!(lim);
                    if if s < 0 { x >= l } else { x <= l } {
                        pc = body;
                        poll!();
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
                            self.update_poll_mask();
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
                }
                Ins::End => return Ok(0),
                Ins::System { s } => return Ok(if s == NONE { 0 } else { ri!(s) as i32 }),
                Ins::Rt { f, d, args } => {
                    if let Some(code) = tryc!(self.rt_call(f, d, args)) {
                        return Ok(code);
                    }
                }
                Ins::CvtFI { d, s, w, trunc } => {
                    let f = rf!(s);
                    let n = if !trunc && w != W::U64 {
                        // x87 FISTP semantics of the C++ backend's qbr()
                        qb_runtime::round::qb_qbr_f64(f)
                    } else {
                        let f = if trunc { f.trunc() } else { f.round_ties_even() };
                        if w == W::U64 || f >= 9.223_372_036_854_776e18 {
                            f as u64 as i64
                        } else {
                            f as i64
                        }
                    };
                    seti!(d, w.wrap(n))
                }
                Ins::Shl { d, a, b: y, w } => seti!(d, w.wrap(ri!(a).wrapping_shl((ri!(y) as u32) % w.bits()))),
                Ins::Shr { d, a, b: y, w } => {
                    let k = (ri!(y) as u32) % w.bits();
                    seti!(d, if w == W::U64 { (r!(a) >> k) as i64 } else { w.wrap(ri!(a) >> k) })
                }
                Ins::DivFI { d, a, b: y } => setf!(d, rf!(a) / rf!(y)),
                Ins::ReturnL { t } => match self.gosub.last().copied() {
                    Some(g) if g.depth == self.frames.len() => {
                        self.gosub.pop();
                        pc = t;
                        if g.event {
                            self.in_event = false;
                            self.update_poll_mask();
                        }
                        if g.view != self.view || g.event {
                            self.set_view(g.view);
                            reload!();
                        }
                    }
                    _ => fail!(3),
                },
                Ins::MovX { d, s } => {
                    let (m, e) = (r!(s), r!(s + 1));
                    set!(d, m);
                    set!(d + 1, e)
                }
                Ins::XBin { op, d, a, b: y } => {
                    let (x, z) = (self.xget(a), self.xget(y));
                    let v = match op {
                        XOp::Add => x + z,
                        XOp::Sub => x - z,
                        XOp::Mul => x * z,
                        XOp::Div => x / z,
                        XOp::Pow => tryc!(qb_float80::qb::pow2(x, z)),
                    };
                    self.xset(d, v);
                }
                Ins::XNeg { d, s } => {
                    let x = self.xget(s);
                    self.xset(d, -x);
                }
                Ins::XCmp { c, d, a, b: y } => {
                    use std::cmp::Ordering::*;
                    let o = self.xget(a).compare(self.xget(y));
                    let v = match (c, o) {
                        (Cmp::Ne, None) => true,
                        (_, None) => false,
                        (Cmp::Eq, Some(o)) => o == Equal,
                        (Cmp::Ne, Some(o)) => o != Equal,
                        (Cmp::Lt, Some(o)) => o == Less,
                        (Cmp::Le, Some(o)) => o != Greater,
                        (Cmp::Gt, Some(o)) => o == Greater,
                        (Cmp::Ge, Some(o)) => o != Less,
                    };
                    set!(d, b(v))
                }
                Ins::XFromI { d, s, u } => {
                    let v = if u { qb_float80::F80::from_u64(r!(s)) } else { qb_float80::F80::from_i64(ri!(s)) };
                    self.xset(d, v);
                }
                Ins::XFromF { d, s } => {
                    let v = qb_float80::F80::from_f64(rf!(s));
                    self.xset(d, v);
                }
                Ins::XToF { d, s, f32 } => {
                    let x = self.xget(s);
                    setf!(d, if f32 { f64::from(x.to_f32()) } else { x.to_f64() })
                }
                Ins::XToI { d, s, w, trunc } => {
                    let x = self.xget(s);
                    let n = if w == W::U64 {
                        x.to_u64_cast() as i64
                    } else if trunc {
                        x.to_i64(qb_float80::Round::Zero)
                    } else {
                        qb_float80::qb::qbr(x)
                    };
                    seti!(d, w.wrap(n))
                }
                Ins::NewV { d, v } => {
                    let p = self.mem.new_var(&m.mvars[v as usize]);
                    self.owned.push(p);
                    set!(d, p)
                }
                Ins::LdM { t, d, p, off } => {
                    let a = r!(p).wrapping_add(u64::from(off));
                    if t == MTy::Str {
                        let v = tryc!(self.mem.ld_str(a)).to_vec();
                        *self.sdst(d) = v;
                    } else if t == MTy::F80 {
                        let raw: [u8; 10] = tryc!(self.mem.bytes(a, 10)).try_into().unwrap();
                        self.xset(d, qb_float80::F80::from_bytes(raw));
                    } else {
                        let v = tryc!(self.mem.ld(t, a));
                        set!(d, v)
                    }
                }
                Ins::StM { t, p, off, s } => {
                    let a = r!(p).wrapping_add(u64::from(off));
                    if t == MTy::Str {
                        let v: *const [u8] = self.sget(s);
                        // SAFETY: the string registers are not touched by memory stores
                        tryc!(self.mem.st_str(a, unsafe { &*v }));
                    } else if t == MTy::F80 {
                        let raw = self.xget(s).to_bytes();
                        tryc!(self.mem.bytes(a, 10)).copy_from_slice(&raw);
                    } else {
                        tryc!(self.mem.st(t, a, r!(s)));
                    }
                }
                Ins::LdFs { d, p, off, len } => {
                    let v = tryc!(self.mem.ldfs(r!(p).wrapping_add(u64::from(off)), len));
                    *self.sdst(d) = v;
                }
                Ins::StFs { p, off, len, s } => {
                    let v: *const [u8] = self.sget(s);
                    // SAFETY: as for StM
                    tryc!(self.mem.stfs(r!(p).wrapping_add(u64::from(off)), len, unsafe { &*v }));
                }
                Ins::PAddM { d, p, o } => set!(d, r!(p).wrapping_add(r!(o))),
                Ins::CopyM { d, s, n } => tryc!(self.mem.copy(r!(d), r!(s), n)),
                Ins::DimM { a, dims, n, mode } => {
                    let b = self.mem_dims(dims, n);
                    tryc!(self.mem.dim(r!(a), mode, b));
                }
                Ins::AAddrM { d, a, i, n } => {
                    let idx: Vec<i64> = (0..n).map(|k| ri!(i + k)).collect();
                    let p = tryc!(self.mem.aaddr(r!(a), &idx));
                    set!(d, p)
                }
                Ins::BoundM { d, a, dim, upper } => {
                    let v = tryc!(self.mem.bound(r!(a), ri!(dim), upper));
                    seti!(d, v)
                }
                Ins::EraseM { a } => tryc!(self.mem.erase(r!(a))),
                Ins::Erase { a } => {
                    let i = self.arr_index(a, self.ab);
                    self.arrs[i] = None;
                }
                Ins::ReadN { d } => {
                    let Some(item) = m.data.get(self.data_ptr) else { fail!(4) };
                    self.data_ptr += 1;
                    let t: Vec<u8> = item.iter().copied().filter(|c| *c != b' ').collect();
                    let v = if t.is_empty() {
                        0.0
                    } else if !v1rt::numeric_text(&t) {
                        fail!(13)
                    } else {
                        v1rt::val(&t)
                    };
                    setf!(d, v)
                }
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
                    if let Err(e) = v1rt::call(self, "PRINTZONE", vec![V::I(0)]) {
                        fail!(stop_code(e));
                    }
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
