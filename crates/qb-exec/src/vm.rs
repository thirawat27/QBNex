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
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use crate::bytecode::*;
use crate::rt::{self, Host, RtState, Stop, V};
use crate::RunError;

#[cfg(feature = "debug")]
#[path = "debug.rs"]
pub mod debug;

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
    /// static array (`dims`): a second DIM keeps it and ERASE only clears it
    fixed: bool,
}

impl Clone for Arr {
    fn clone(&self) -> Arr {
        let mut a = Arr::with_data(self.dims.clone(), self.data.clone());
        a.fixed = self.fixed;
        a
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
            Ty::F80 => return Err(5), // _FLOAT arrays live in memory mode
        };
        Ok(Arr::with_data(dims, data))
    }

    /// CLEAR: every element back to 0 / "" (the storage stays where it is).
    fn zero(&mut self) {
        match &mut self.data {
            ArrData::I16(v) => v.fill(0),
            ArrData::I32(v) => v.fill(0),
            ArrData::I64(v) => v.fill(0),
            ArrData::F32(v) => v.fill(0.0),
            ArrData::F64(v) => v.fill(0.0),
            ArrData::Str(v) => v.iter_mut().for_each(Vec::clear),
        }
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
        Arr { dims, data, view, fixed: false }
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

/// `Gs::ev` of an ON KEY(i) handler: `KEY_EV - i` (timers use their slot >= 0, -1 = plain GOSUB).
const KEY_EV: i32 = -2;

#[derive(Debug, Clone, Copy)]
struct Gs {
    /// ON TIMER slot whose handler this GOSUB runs (-1 = a plain GOSUB)
    ev: i32,
    ret: u32,
    depth: usize,
    view: usize,
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

pub struct Vm<'m, 'o> {
    m: &'m Module,
    /// console/file output and the runtime state of the builtins
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
    on_error: Option<u32>,
    /// (pc of the failing instruction, view) while an error handler runs
    in_handler: Option<(u32, usize)>,
    err: i32,
    erl: u32,
    /// SYSTEM n / exit code of the program once it ended
    exit_code: i32,
    /// the program ended with END (or by running off main), not SYSTEM
    ended: bool,
    /// END's "Press any key to continue": None = no prompt (nobody could press
    /// a key); Some(wait) = print it, then wait for a key with `wait`
    end_prompt: Option<fn()>,
    /// the program ended with SYSTEM (a window closes at once; END keeps it open)
    by_system: bool,
    /// source line of the last error (_ERRORLINE)
    errline: u32,
    /// last numeric line label passed (QIR LASTLINE; ERL reports it)
    last_line: u32,
    /// the ON TIMER ticker thread runs
    timer_thread: bool,
    /// ON KEY was set up: safe points look at the keys (and a thread wakes them)
    key_poll: bool,
    /// DECLARE LIBRARY: resolved function addresses (per `Module::xfns`) and loaded libraries
    xptrs: Vec<usize>,
    xlibs: Vec<qb_ffi::Library>,
    /// unhandled errors continue after the statement (the program executable, headless)
    continue_unhandled: bool,
    data_ptr: usize,
    ev: Arc<EvShared>,
    /// pending bits examined at safe points
    poll_mask: u32,
    /// JIT tier (R11.6); `None` = interpret only (`QB_JIT=off`).
    jit: Option<Box<crate::jit::JitState>>,
    /// `exec` returns when a RET brings the frame count back to this
    /// (calls made from native code run the callee to completion).
    stop_depth: usize,
    /// native -> VM -> native nesting of such calls
    jit_nest: u32,
    /// addressable memory (QIR TYPE / STRING * n / pointers)
    mem: crate::mem::Mem,
    owned: Vec<u32>,
    /// attached debugger (feature `debug`)
    #[cfg(feature = "debug")]
    dbg: Option<debug::DebugState>,
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
    fn bytes(&mut self, p: u64, len: usize) -> crate::rt::R<&mut [u8]> {
        self.mem.bytes(p, len).map_err(Stop::Err)
    }
    fn array_extent(&mut self, p: u64) -> crate::rt::R<(u64, u64)> {
        self.mem.array_extent(p).map_err(Stop::Err)
    }
    fn str_data(&mut self, p: u64) -> crate::rt::R<u64> {
        self.mem.str_data(p).map_err(Stop::Err)
    }
    fn load_str(&mut self, p: u64) -> crate::rt::R<Vec<u8>> {
        self.mem.ld_str(p).map(|s| s.to_vec()).map_err(Stop::Err)
    }
    fn store_str(&mut self, p: u64, s: &[u8]) -> crate::rt::R<()> {
        rt::str_store_hook(&mut self.rt, p, s.len());
        self.mem.st_str(p, s).map_err(Stop::Err)
    }
}

impl<'m, 'o> Vm<'m, 'o> {
    pub fn new(m: &'m Module, out: &'o mut dyn Write) -> Self {
        let main = &m.funcs[0];
        let mut rt = RtState::new(out);
        rt.console_on = m.console.is_some();
        rt.console_only = m.console == Some(true);
        rt.display.screen_hide = m.screenhide;
        Vm {
            m,
            rt,
            nums: main.num_init.clone(),
            strs: vec![Vec::new(); main.n_str as usize],
            arrs: (0..main.n_arr()).map(|_| None).collect(),
            frames: vec![Frame { owned: 0, func: 0, ret: 0, nb: 0, sb: 0, ab: 0, caller_view: 0, desc: 0 }],
            view: 0,
            nb: 0,
            sb: 0,
            ab: 0,
            gosub: Vec::new(),
            on_error: None,
            in_handler: None,
            err: 0,
            erl: 0,
            exit_code: 0,
            ended: false,
            end_prompt: None,
            by_system: false,
            errline: 0,
            last_line: 0,
            continue_unhandled: false,
            timer_thread: false,
            key_poll: false,
            xptrs: vec![0; m.xfns.len()],
            xlibs: Vec::new(),
            data_ptr: 0,
            ev: Arc::new(EvShared { pending: AtomicU32::new(0) }),
            poll_mask: u32::MAX,
            jit: crate::jit::JitState::from_env(m),
            stop_depth: usize::MAX,
            jit_nest: 0,
            mem: crate::mem::Mem::new(),
            owned: Vec::new(),
            #[cfg(feature = "debug")]
            dbg: None,
        }
    }

    /// Uses native code compiled ahead of time (an image made by
    /// [`crate::build_native`] for this program); no JIT compilation happens.
    /// Uses native functions linked into this executable (qb-native).
    pub fn load_linked(&mut self, fns: &[qb_jit::NativeFn], fingerprint: u64) -> Result<(), String> {
        self.jit = Some(crate::jit::JitState::from_linked(self.m, fns, fingerprint, &vm_hosts())?);
        Ok(())
    }

    pub fn load_native(&mut self, image: &[u8]) -> Result<(), String> {
        let st = crate::jit::JitState::from_image(self.m, image, &vm_hosts())?;
        self.jit = Some(st);
        Ok(())
    }

    /// Unhandled (non-critical) errors print the C++ runtime's dialog text on
    /// stderr and the program continues after the statement, as a headless
    /// C++ program does; off by default (the error ends the run).
    pub fn set_continue_unhandled(&mut self, on: bool) {
        self.continue_unhandled = on;
    }

    /// END's "Press any key to continue" (sub_end of the C++ runtime): `None`
    /// ends silently, `Some(wait)` prints the prompt and calls `wait` for the key.
    pub fn set_end_prompt(&mut self, wait: Option<fn()>) {
        self.end_prompt = wait;
    }

    /// Browser runner (qb-web, R13-wasm): the program runs on the screen page
    /// from its first statement (PRINT draws there, as in a window), INPUT
    /// reads `stdin` and echoes what it consumed on the screen, and only the
    /// VM runs (no native code in a browser).
    pub fn set_web_console(&mut self, stdin: &[u8]) {
        self.jit = None;
        let rt = &mut self.rt;
        rt.set_stdin(stdin);
        rt.web = true;
        let cmem = rt.cmem.as_mut_ptr();
        let (on, only) = (rt.console_on, rt.console_only);
        rt.display.start(cmem, on, only);
    }

    /// GUI program (no `$CONSOLE`) with a window (rt/window.rs): the screen
    /// starts at once (PRINT draws on it), INPUT reads typed keys, frames are
    /// presented while `_AUTODISPLAY` is on, and closing the window ends the
    /// program. The window layer itself runs on the main thread
    /// ([`crate::run_window_main`]).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_window(&mut self) {
        let rt = &mut self.rt;
        rt.window = true;
        rt.display.window = true;
        let cmem = rt.cmem.as_mut_ptr();
        let (on, only) = (rt.console_on, rt.console_only);
        rt.display.start(cmem, on, only);
        if !rt.display.title.is_empty() {
            crate::rt::window_set_title(&rt.display.title);
        }
        crate::rt::window_activate(self.events());
        crate::rt::window_present(&self.rt.display);
    }
    /// The visible screen page as a `QBFB` record (format of
    /// `qb_runtime::gfx::capture`: size, mode, pixels or text cells, palette).
    pub fn screen_record(&self) -> Option<Vec<u8>> {
        if !self.rt.display.started {
            return None;
        }
        // SAFETY: the display index is a live registry entry once the screen started.
        let im = unsafe { &*qb_runtime::gfx::registry::img.add(self.rt.display.display_index as usize) };
        qb_runtime::gfx::capture::record(im, qb_runtime::gfx::capture::KIND_DISPLAY)
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
        let hosts = vm_hosts();
        let arrs = self.arrs.as_mut_ptr() as *mut u64;
        // SAFETY: `ab` + the frame's array count is inside `arrs`; Option<Box<Arr>> is one nullable pointer.
        let tables = [unsafe { arrs.add(self.ab) }, arrs];
        // This frame's GOSUB entries move to the native stack for the native session.
        let base = js.gosub_len();
        let (depth, view) = (self.frames.len(), self.view);
        let n = self.gosub.len();
        let k = self.gosub.iter().rev().take_while(|g| g.depth == depth && g.view == view).count();
        if (base + k as u32 + 1) >= crate::jit::GOSUB_CAP {
            return None;
        }
        // an ON TIMER/KEY handler's GOSUB keeps its slot (done() at RETURN): stay in the VM
        if self.gosub[n - k..].iter().any(|g| g.ev != -1) {
            return None;
        }
        if self.gosub[..n - k].last().is_some_and(|g| g.depth == depth) {
            js.gosub_push(crate::jit::GOSUB_VM_BELOW);
        }
        for g in &self.gosub[n - k..] {
            js.gosub_push(g.ret);
        }
        self.gosub.truncate(n - k);
        js.set_gosub_base(base);
        let st = unsafe { js.enter(pc, np, globals, ev, mask, me, &hosts, tables) };
        let js: &mut crate::jit::JitState = self.jit.as_deref_mut()?;
        for r in js.gosub_take(base) {
            if r != crate::jit::GOSUB_VM_BELOW {
                self.gosub.push(Gs { ev: -1, ret: r, depth, view });
            }
        }
        let st = st?;
        let frames = js.take_frames(self.m.funcs.len());
        if !frames.is_empty() {
            self.push_native_frames(frames);
        }
        Some(st)
    }

    /// Rebuilds the VM frames of native calls that were still active when
    /// native code stopped (error, event, deopt), so the VM continues inside
    /// the innermost callee exactly as if it had made the calls itself.
    #[cold]
    fn push_native_frames(&mut self, frames: Vec<(u32, Vec<u64>, u32)>) {
        for (f, regs, call_pc) in frames {
            let Ins::Call { desc, .. } = self.m.code[call_pc as usize] else { unreachable!("chain points at a CALL") };
            let func = &self.m.funcs[f as usize];
            let (nb, sb, ab) = (self.nums.len(), self.strs.len(), self.arrs.len());
            self.nums.extend_from_slice(&regs[..func.n_num() as usize]);
            self.strs.resize_with(sb + func.n_str as usize, Vec::new);
            self.arrs.resize_with(ab + func.n_arr() as usize, || None);
            self.frames.push(Frame { owned: self.owned.len(), func: f, ret: call_pc + 1, nb, sb, ab, caller_view: self.view, desc });
            self.set_view(self.frames.len() - 1);
        }
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
        // u32::MAX: the program ended at the CALL's event poll (window closed), with
        // the callee's frame already pushed; there is nothing to run
        if self.frames.len() == depth || entry == u32::MAX {
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
        self.run_status().map(|_| ())
    }

    /// Runs the program; returns its exit code (`SYSTEM n`).
    pub fn run_status(&mut self) -> Result<i32, RunError> {
        let mut pc = self.m.funcs[0].entry;
        let r = loop {
            match self.exec(pc) {
                Ok(code) => {
                    if let Some((k, run)) = self.rt.dos.restart.take() {
                        self.restart(k, run);
                        pc = self.m.funcs[0].entry;
                        continue;
                    }
                    break Ok(code);
                }
                Err((code, at)) => match self.raise(code, at) {
                    Ok(h) => pc = h,
                    Err(e) => break Err(e),
                },
            }
        };
        // frame record of the screen at END/SYSTEM (end() of the C++ runtime)
        self.rt.display.capture(qb_runtime::gfx::capture::KIND_END);
        // console program: END's "Press any key to continue" (console branch of
        // sub_end), only when somebody could press the key (see set_end_prompt)
        if !self.rt.window && r.is_ok() && self.ended {
            if let Some(wait) = self.end_prompt {
                let text: &[u8] = if cfg!(windows) { b"\nPress any key to continue" } else { b"\nPress enter to continue" };
                let _ = self.rt.write(0, text);
                self.rt.flush();
                wait();
            }
        }
        self.rt.flush();
        // with a window, END keeps it open until it is closed (end() waits for stop_program)
        #[cfg(not(target_arch = "wasm32"))]
        if self.rt.window && r.is_ok() && !self.by_system {
            crate::rt::window_present(&self.rt.display);
            let mut at = qb_runtime::systime::ticks();
            while !crate::rt::window_closed() {
                qb_runtime::systime::sleep_ms(16);
                let now = qb_runtime::systime::ticks();
                if now - at >= 32 && self.rt.display.autodisplay != 0 {
                    at = now;
                    crate::rt::window_present(&self.rt.display);
                }
            }
        }
        r
    }

    /// RUN / CLEAR (`rt::dosx`): `main` starts over with fresh variables (STATIC
    /// ones live in its frame too); its first statement (RUNFROM) jumps to
    /// target `k` (0 = the start). RUN also forgets array dimensions (the DIMs
    /// run again); CLEAR keeps them with zero elements, as static arrays.
    fn restart(&mut self, k: i32, run: bool) {
        let m = self.m;
        let main = &m.funcs[0];
        self.free_owned(0);
        self.frames.truncate(1);
        self.set_view(0);
        self.nums.clear();
        self.nums.extend_from_slice(&main.num_init);
        self.strs.clear();
        self.strs.resize_with(main.n_str as usize, Vec::new);
        let n = main.n_arr() as usize;
        self.arrs.truncate(n);
        for a in &mut self.arrs {
            match a {
                Some(x) if !run => x.zero(),
                _ => *a = None,
            }
        }
        self.gosub.clear();
        self.on_error = None;
        self.in_handler = None;
        self.err = 0;
        self.data_ptr = 0;
        self.stop_depth = usize::MAX;
        self.jit_nest = 0;
        self.rt.dos.run_from = k;
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

    /// QIR `rt` builtin through the shared runtime. `Ok(Some(code))` = the program exits.
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
        match rt::call(self, name, vals) {
            // RUN, CLEAR, RUN file$, CHAIN: leave like END (run_status restarts the program)
            Ok(_) if self.rt.dos.stops() => Ok(Some(0)),
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
            Err(Stop::Err(c)) => Err(c),
        }
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
        let dimensioned = self.arrs[i].as_ref().is_some_and(|a| !a.dims.is_empty());
        if mode == DimMode::Static && dimensioned {
            return Ok(()); // a static array is dimensioned once (SUB ... STATIC, hoisted DIM)
        }
        if mode == DimMode::Dim && dimensioned {
            return Err(10); // duplicate definition
        }
        let mut new = Arr::new(ty, dims)?;
        new.fixed = mode == DimMode::Static;
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
        if let (Some(d), Some((slot, rty))) = (cd.dest, func.ret) {
            match slot {
                Slot::N(r) => {
                    self.nums[cx.nb + d as usize] = self.nums[fr.nb + r as usize];
                    if rty == Ty::F80 {
                        self.nums[cx.nb + d as usize + 1] = self.nums[fr.nb + r as usize + 1];
                    }
                }
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
        let unhandled = !matches!((self.on_error, self.in_handler), (Some(_), None));
        if unhandled && self.continue_unhandled {
            // the C++ runtime's fix_error: with QBNEX_NO_ERROR_DIALOG or redirected output the
            // dialog becomes one stderr line and the program ends with exit code 1; otherwise
            // (headless, MessageBox2 gets no answer) a non-critical error continues after the statement
            let cmd0 = std::env::args().next().unwrap_or_default();
            let d = qb_runtime::error::error_dialog(code as u32, self.line_of(at), 0, None, cmd0.as_bytes());
            let nd = qb_runtime::error::no_dialog_now();
            if nd || d.critical {
                self.rt.flush();
                let line = qb_runtime::error::report_line(&d.title, &d.body);
                use std::io::Write;
                let mut e = std::io::stderr().lock();
                let _ = e.write_all(&line);
                let _ = e.write_all(b"\n");
                return Err(RunError { code, line: self.line_of(at), msg: "reported".into() });
            }
            eprintln!("{}: {}", String::from_utf8_lossy(&d.title), String::from_utf8_lossy(&d.body));
            return Ok(self.next_stmt(at));
        }
        match (self.on_error, self.in_handler) {
            (Some(h), None) => {
                self.err = code;
                self.errline = self.line_of(at);
                self.erl = self.last_line;
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
                // RESUME label unwinds to the main module
                while self.frames.len() > 1 {
                    let fr = self.frames.pop().unwrap();
                    self.free_owned(fr.owned);
                    self.nums.truncate(fr.nb);
                    self.strs.truncate(fr.sb);
                    self.arrs.truncate(fr.ab);
                }
                self.gosub.retain(|g| g.depth <= 1);
                self.set_view(0);
                t
            }
        })
    }

    // ------------------------------------------------------------ events

    /// Safe point with pending events: discard events that are OFF, run the
    /// handler of one that is ON as an implicit GOSUB at module level.
    #[cold]
    fn dispatch_events(&mut self, pc: u32) -> Option<u32> {
        let bits = self.ev.pending.swap(0, Ordering::AcqRel);
        #[cfg(not(target_arch = "wasm32"))]
        if self.rt.window {
            if crate::rt::window_closed() {
                // the close box ends the program (no `_EXIT` in QIR programs)
                self.exit_code = 0;
                return Some(u32::MAX);
            }
            if bits & (1 << crate::rt::PRESENT_EVENT) != 0 && self.rt.display.autodisplay != 0 {
                crate::rt::window_present(&self.rt.display);
            }
        }
        let _ = bits;
        // no new handlers while an ON ERROR handler runs (events() of qbx.cpp)
        if self.in_handler.is_some() {
            return None;
        }
        let mut t = qb_runtime::events::timers();
        for i in 0..t.count() {
            if let Some((id, _)) = t.take(i) {
                drop(t);
                // the handler runs as a GOSUB at module level and returns here
                self.gosub.push(Gs { ev: i, ret: pc, depth: self.frames.len(), view: self.view });
                if self.view != 0 {
                    self.set_view(0);
                }
                return Some(id - 1);
            }
        }
        drop(t);
        if self.key_poll {
            // ON KEY: keys pressed since the last safe point (libqb.cpp events())
            let rt = &mut self.rt;
            rt.input.pump(&mut rt.cmem);
            // SAFETY: program thread only.
            if unsafe { qb_runtime::events::onkey_inprogress } == 0 {
                for (i, k) in qb_runtime::events::onkeys().iter_mut().enumerate() {
                    if k.id != 0 && k.active == 1 && k.state > 0 {
                        k.state -= 1;
                        let id = k.id;
                        // SAFETY: as above.
                        unsafe { qb_runtime::events::onkey_inprogress = 1 };
                        self.gosub.push(Gs { ev: KEY_EV - i as i32, ret: pc, depth: self.frames.len(), view: self.view });
                        if self.view != 0 {
                            self.set_view(0);
                        }
                        return Some(id - 1);
                    }
                }
            }
        }
        None
    }

    /// ON KEY: a thread raises event 0 every 10 ms so the VM looks at the keys.
    fn start_key_poll(&mut self) {
        if self.key_poll {
            return;
        }
        self.key_poll = true;
        let ev = self.events();
        std::thread::spawn(move || loop {
            ev.raise(0);
            qb_runtime::systime::sleep_ms(10);
        });
    }

    /// Address of DECLARE LIBRARY function `i`: the runner's own qb-ext symbols,
    /// then the named library or the default system libraries. 453 = not found.
    fn xresolve(&mut self, i: usize) -> Result<usize, i32> {
        if self.xptrs[i] != 0 {
            return Ok(self.xptrs[i]);
        }
        let xf = &self.m.xfns[i];
        let mut p = crate::ext_symbol(&xf.name).map(|p| p as usize);
        if p.is_none() && !xf.lib.is_empty() {
            if let Ok(l) = qb_ffi::Library::open_dynamic(&xf.lib, None) {
                p = l.symbol(&xf.name).map(|p| p as usize);
                self.xlibs.push(l);
            }
        }
        if p.is_none() {
            p = qb_ffi::find_default_symbol(&xf.name).map(|p| p as usize);
        }
        let p = p.ok_or(453)?;
        self.xptrs[i] = p;
        Ok(p)
    }

    /// TIMERTHREAD: ticks the ON TIMER table every millisecond and raises event 0.
    fn start_timer_thread(&mut self) {
        if self.timer_thread {
            return;
        }
        self.timer_thread = true;
        let ev = self.events();
        std::thread::spawn(move || loop {
            let now = qb_runtime::systime::ticks() as f64 * 0.001;
            if qb_runtime::events::timers().tick(now) {
                ev.raise(0);
            }
            qb_runtime::systime::sleep_ms(1);
        });
    }

    // ------------------------------------------------------------ builtins

    // ------------------------------------------------------------ hot loop

    /// Runs from `pc` until END (`Ok`) or an error (`Err((code, pc))`).
    fn exec(&mut self, pc: u32) -> Result<i32, (i32, u32)> {
        self.exec_impl::<false>(pc).map(|_| self.exit_code)
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
                        if h == u32::MAX {
                            return Ok(u32::MAX);
                        }
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
            #[cfg(feature = "debug")]
            if !STEP && self.dbg.is_some() {
                self.debug_statement(pc);
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
                Ins::IToFU { d, s } => setf!(d, r!(s) as f64),
                Ins::CmpU { c, d, a, b: y } => set!(d, b(c.test(r!(a), r!(y)))),
                Ins::BrU { c, a, b: y, t } => {
                    if c.test(r!(a), r!(y)) {
                        pc = t
                    }
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
                Ins::PowF { d, a, b } => setf!(d, rf!(a).powf(rf!(b))),
                Ins::Un { op, d, s } => match op {
                    Un::NegI(w) => seti!(d, w.wrap(ri!(s).wrapping_neg())),
                    Un::NegF => setf!(d, -rf!(s)),
                    Un::Not => seti!(d, !ri!(s)),
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
                        jit!();
                    }
                }
                Ins::Gosub { t } => {
                    self.gosub.push(Gs { ev: -1, ret: pc, depth: self.frames.len(), view: self.view });
                    pc = t;
                }
                Ins::Return => match self.gosub.last().copied() {
                    Some(g) if g.depth == self.frames.len() => {
                        self.gosub.pop();
                        if g.ev >= 0 {
                            qb_runtime::events::timers().done(g.ev);
                        } else if g.ev <= KEY_EV {
                            // SAFETY: program thread only.
                            unsafe { qb_runtime::events::onkey_inprogress = 0 };
                        }
                        pc = g.ret;
                        if g.view != self.view {
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
                Ins::End => {
                    self.exit_code = 0;
                    self.ended = true;
                    return Ok(u32::MAX);
                }
                Ins::System { s } => {
                    self.exit_code = if s == NONE { 0 } else { ri!(s) as i32 };
                    self.by_system = true;
                    return Ok(u32::MAX);
                }
                Ins::Rt { f, d, args } => {
                    if let Some(code) = tryc!(self.rt_call(f, d, args)) {
                        self.exit_code = code;
                        return Ok(u32::MAX);
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
                        if g.view != self.view {
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
                    let (p, id) = self.mem.new_var(&m.mvars[v as usize]);
                    self.owned.push(id);
                    set!(d, p)
                }
                Ins::LdM { t, d, p, off } => {
                    let a = r!(p).wrapping_add(u64::from(off));
                    if t == MTy::Str {
                        // into the register's own buffer (no allocation per load, R14)
                        let src: *const [u8] = tryc!(self.mem.ld_str(a));
                        let dst = self.sdst(d);
                        dst.clear();
                        // SAFETY: memory strings and string registers are separate buffers
                        dst.extend_from_slice(unsafe { &*src });
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
                        let v = unsafe { &*v };
                        rt::str_store_hook(&mut self.rt, a, v.len());
                        tryc!(self.mem.st_str(a, v));
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
                    // subscripts on the stack (a heap Vec per element access was a hot allocation, R14)
                    let mut buf = [0i64; 8];
                    let p = if (n as usize) <= buf.len() {
                        for k in 0..n {
                            buf[k as usize] = ri!(i + k);
                        }
                        tryc!(self.mem.aaddr(r!(a), &buf[..n as usize]))
                    } else {
                        let idx: Vec<i64> = (0..n).map(|k| ri!(i + k)).collect();
                        tryc!(self.mem.aaddr(r!(a), &idx))
                    };
                    set!(d, p)
                }
                Ins::BoundM { d, a, dim, upper } => {
                    let v = tryc!(self.mem.bound(r!(a), ri!(dim), upper));
                    seti!(d, v)
                }
                Ins::EraseM { a } => tryc!(self.mem.erase(r!(a))),
                Ins::Erase { a } => {
                    // an erased array has no dimensions (subscript out of range until
                    // REDIM); None would mean "never dimensioned" (implicit 0 TO 10)
                    let i = self.arr_index(a, self.ab);
                    let f = if a & G != 0 { 0 } else { self.frames[self.view].func as usize };
                    let ty = self.m.funcs[f].arr_tys[(a & !G) as usize];
                    // a static array (also one passed as a() to the SUB doing the ERASE)
                    // keeps its dimensions and is only cleared, as in QBasic
                    let keep = match self.arrs[i].as_deref() {
                        Some(old) if old.fixed => Some(old.dims.clone()),
                        _ => None,
                    };
                    let mut new = tryc!(Arr::new(ty, keep.clone().unwrap_or_default()));
                    new.fixed = keep.is_some();
                    self.arrs[i] = Some(Box::new(new));
                }
                Ins::ReadN { d, typ } => {
                    use qb_runtime::numparse as np_;
                    let Some(item) = m.data.get(self.data_ptr) else { fail!(4) };
                    let t: Vec<u8> = item.iter().copied().filter(|c| *c != b' ').collect();
                    let mut off = 0;
                    let float = typ & np_::ISFLOAT != 0;
                    // an empty item reads as 0
                    let r = if t.is_empty() {
                        Ok(if float { 0f64.to_bits() } else { 0 })
                    } else if typ == 64 {
                        np_::read_int64(&t, &mut off).map(|v| v as u64)
                    } else if typ == 64 | np_::ISUNSIGNED {
                        np_::read_uint64(&t, &mut off)
                    } else {
                        np_::read_float(&t, &mut off, typ).map(|v| {
                            if !float {
                                match v {
                                    np_::NumValue::Int(i) => i as u64,
                                    np_::NumValue::Float(f) => f as i64 as u64,
                                }
                            } else if typ & 511 == 32 {
                                f64::from(v.to_f64() as f32).to_bits()
                            } else {
                                v.to_f64().to_bits()
                            }
                        })
                    };
                    match r {
                        Ok(v) => {
                            self.data_ptr += 1;
                            set!(d, v)
                        }
                        // the C++ READ assigns 0 and the error is raised after the store (READCHK);
                        // Out of DATA stops at once
                        Err(4) => fail!(4),
                        Err(c) => {
                            set!(d, 0);
                            self.rt.read_err = c;
                        }
                    }
                }
                Ins::OnError { t } => self.on_error = (t != NONE).then_some(t),
                Ins::XCall { c } => {
                    let xc = &m.xcalls[c as usize];
                    let xf = &m.xfns[xc.f as usize];
                    let fp = tryc!(self.xresolve(xc.f as usize));
                    let mut vals = Vec::with_capacity(xc.args.len());
                    for (&r, &t) in xc.args.iter().zip(&xf.args) {
                        use crate::bytecode::XTy as T;
                        vals.push(match t {
                            T::Str => qb_ffi::Value::Ptr(self.sget(r).as_ptr() as usize),
                            T::F32 => qb_ffi::Value::F32(rf!(r) as f32),
                            T::F64 => qb_ffi::Value::F64(rf!(r)),
                            T::Ptr => qb_ffi::Value::Ptr(r!(r) as usize),
                            T::U8 | T::U16 | T::U32 | T::U64 => qb_ffi::Value::U(r!(r)),
                            _ => qb_ffi::Value::I(ri!(r)),
                        });
                    }
                    let sig = qb_ffi::Signature::new(xf.args.iter().map(|&t| xty(t)).collect(), xty(xf.ret));
                    // SAFETY: the program declared this signature for the symbol (DECLARE LIBRARY)
                    let v = match unsafe { qb_ffi::call(fp as *const core::ffi::c_void, &sig, &vals) } {
                        Ok(v) => v,
                        Err(_) => fail!(5),
                    };
                    if let Some(d) = xc.dest {
                        match (xf.ret, v) {
                            (crate::bytecode::XTy::Str, qb_ffi::Value::Ptr(p)) => {
                                let s = if p == 0 {
                                    Vec::new()
                                } else {
                                    // SAFETY: a char* result is a NUL-terminated C string
                                    unsafe { std::ffi::CStr::from_ptr(p as *const core::ffi::c_char) }.to_bytes().to_vec()
                                };
                                *self.sdst(d) = s;
                            }
                            (_, qb_ffi::Value::I(x)) => seti!(d, x),
                            (_, qb_ffi::Value::U(x)) => set!(d, x),
                            (_, qb_ffi::Value::Ptr(x)) => set!(d, x as u64),
                            (_, qb_ffi::Value::F32(x)) => setf!(d, f64::from(x)),
                            (_, qb_ffi::Value::F64(x)) => setf!(d, x),
                            (_, qb_ffi::Value::Void) => {}
                        }
                    }
                }
                Ins::OnTimer { s, x, t } => {
                    // event id = handler pc + 1 (0 = no handler)
                    tryc!(qb_runtime::events::timers().setup(ri!(s) as i32, rf!(x), t + 1, 0));
                    self.start_timer_thread();
                }
                Ins::OnKey { s, t } => {
                    tryc!(qb_runtime::events::onkey_setup(qb_runtime::events::onkeys(), ri!(s) as i32, t + 1, 0));
                    self.start_key_poll();
                }
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
                    // into the global's own buffer (no allocation per store, R14)
                    let src: *const [u8] = self.sget(s);
                    let dst = &mut self.strs[g as usize];
                    if !std::ptr::eq(dst.as_slice(), src) {
                        dst.clear();
                        // SAFETY: distinct buffers (checked above)
                        dst.extend_from_slice(unsafe { &*src });
                    }
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
                    // qbs_left: the count is clamped
                    let len = self.sget(s).len();
                    let k = ri!(n).clamp(0, len as i64) as usize;
                    self.ssub(d, s, 0, k);
                }
                Ins::Right { d, s, n } => {
                    let len = self.sget(s).len();
                    let k = ri!(n).clamp(0, len as i64) as usize;
                    self.ssub(d, s, len - k, len);
                }
                Ins::Mid { d, s, start, n } => {
                    // qb_func_mid: no errors
                    let len = self.sget(s).len();
                    let l = if n == NONE { None } else { Some(ri!(n)) };
                    let (from, to) = rt::builtins::mid_range(len, ri!(start), l);
                    self.ssub(d, s, from, to);
                }
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
                Ins::ReadS { d } => {
                    let Some(item) = m.data.get(self.data_ptr) else { fail!(4) }; // out of DATA
                    self.data_ptr += 1;
                    let dst = self.sdst(d);
                    dst.clear();
                    dst.extend_from_slice(item);
                }
                Ins::Restore { i } => self.data_ptr = i as usize,
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
#[cfg_attr(feature = "native-syms", unsafe(export_name = "qb_vm_op"))]
extern "C" fn vm_op(ctx: *mut qb_jit::Ctx, frame: *mut u64, pc: u32) -> u32 {
    // SAFETY: `user` was set by `jit_enter` to the VM that entered native code.
    let ctx = unsafe { &mut *ctx };
    let vm = unsafe { &mut *(ctx.user as *mut Vm<'static, 'static>) };
    debug_assert_eq!(frame, vm.np());
    let r = if matches!(vm.m.code[pc as usize], Ins::Call { .. }) {
        // Hand the native GOSUB entries of this frame to the VM around the call.
        let (depth, view) = (vm.frames.len(), vm.view);
        let base = ctx.gosub_base;
        let js = vm.jit.as_deref_mut().expect("native code runs with a JIT state");
        let mine = js.gosub_take(base);
        let below = mine.first() == Some(&crate::jit::GOSUB_VM_BELOW);
        if below {
            js.gosub_push(crate::jit::GOSUB_VM_BELOW);
        }
        let moved: Vec<u32> = mine.into_iter().filter(|r| *r != crate::jit::GOSUB_VM_BELOW).collect();
        let count = moved.len();
        vm.gosub.extend(moved.into_iter().map(|ret| Gs { ev: -1, ret, depth, view }));
        let r = vm.call_from_native(pc);
        // Back to native: the same entries are on top again after a normal return.
        let n = vm.gosub.len();
        let k = vm.gosub.iter().rev().take(count).take_while(|g| g.depth == depth && g.view == view).count();
        let js = vm.jit.as_deref_mut().expect("JIT state");
        for g in &vm.gosub[n - k..] {
            js.gosub_push(g.ret);
        }
        vm.gosub.truncate(n - k);
        // (ctx may have been reused by nested native code: restore this session's base)
        js.set_gosub_base(base);
        r
    } else {
        vm.step_one(pc)
    };
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
#[cfg_attr(feature = "native-syms", unsafe(export_name = "qb_host_0"))]
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

#[cfg_attr(feature = "native-syms", unsafe(export_name = "qb_host_1"))]
extern "C" fn jit_ste1(ctx: *mut qb_jit::Ctx, a: i64, i: i64, v: i64) {
    let (ctx, vm) = host_vm(ctx);
    if let Err(c) = vm.arr(a as R, 1).and_then(|arr| arr.off1(i).map(|o| arr.set(o, v as u64))) {
        ctx.err = c;
    }
}

#[cfg_attr(feature = "native-syms", unsafe(export_name = "qb_host_2"))]
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

#[cfg_attr(feature = "native-syms", unsafe(export_name = "qb_host_3"))]
extern "C" fn jit_ste2(ctx: *mut qb_jit::Ctx, a: i64, i: i64, j: i64, v: i64) {
    let (ctx, vm) = host_vm(ctx);
    if let Err(c) = vm.arr(a as R, 2).and_then(|arr| arr.off([i, j].into_iter(), 2).map(|o| arr.set(o, v as u64))) {
        ctx.err = c;
    }
}

#[cfg_attr(feature = "native-syms", unsafe(export_name = "qb_host_4"))]
extern "C" fn jit_lde1g(ctx: *mut qb_jit::Ctx, a: i64, i: i64) -> i64 {
    jit_lde1(ctx, a | G as i64, i)
}

#[cfg_attr(feature = "native-syms", unsafe(export_name = "qb_host_5"))]
extern "C" fn jit_ste1g(ctx: *mut qb_jit::Ctx, a: i64, i: i64, v: i64) {
    jit_ste1(ctx, a | G as i64, i, v)
}

// Native code reads `Option<Box<Arr>>` table entries as plain pointers
// (no native code in a browser build, where pointers are 4 bytes).
#[cfg(not(target_arch = "wasm32"))]
const _: () = assert!(core::mem::size_of::<Option<Box<Arr>>>() == 8);

/// The VM entry points native code calls (same addresses for JIT and AOT code
/// of this executable).
pub(crate) fn vm_hosts() -> crate::jit::Hosts {
    crate::jit::Hosts {
        vm_op,
        lde1: jit_lde1 as *const () as usize,
        ste1: jit_ste1 as *const () as usize,
        lde2: jit_lde2 as *const () as usize,
        ste2: jit_ste2 as *const () as usize,
        lde1g: jit_lde1g as *const () as usize,
        cmps: jit_cmps as *const () as usize,
        ste1g: jit_ste1g as *const () as usize,
        view_off: core::mem::offset_of!(Arr, view) as u32,
    }
}

/// Native string compare: `a c b` on the current frame's string operands
/// (constants allowed), -1 / 0 like `Ins::CmpS`.
#[cfg_attr(feature = "native-syms", unsafe(export_name = "qb_host_6"))]
extern "C" fn jit_cmps(ctx: *mut qb_jit::Ctx, c: i64, a: i64, b: i64) -> i64 {
    let (_, vm) = host_vm(ctx);
    let t = crate::jit::cmp_of(c).test(vm.sget(a as R), vm.sget(b as R));
    if t {
        -1
    } else {
        0
    }
}

fn xty(t: crate::bytecode::XTy) -> qb_ffi::Ty {
    use crate::bytecode::XTy as T;
    match t {
        T::Void => qb_ffi::Ty::Void,
        T::I8 => qb_ffi::Ty::I8,
        T::U8 => qb_ffi::Ty::U8,
        T::I16 => qb_ffi::Ty::I16,
        T::U16 => qb_ffi::Ty::U16,
        T::I32 => qb_ffi::Ty::I32,
        T::U32 => qb_ffi::Ty::U32,
        T::I64 => qb_ffi::Ty::I64,
        T::U64 => qb_ffi::Ty::U64,
        T::F32 => qb_ffi::Ty::F32,
        T::F64 => qb_ffi::Ty::F64,
        T::Ptr | T::Str => qb_ffi::Ty::Ptr,
    }
}
