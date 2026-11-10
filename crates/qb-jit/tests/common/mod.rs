#![allow(dead_code)]
//! Test helpers: a small MIR builder and a harness that runs a module both
//! in the reference interpreter and as native code and compares everything
//! observable.

use qb_jit::ctx::{ChainRec, Ctx};
use qb_jit::interp;
use qb_jit::mir::*;
use qb_jit::rt::{self, Rt};
use qb_jit::thunk::Thunks;
use qb_jit::{compile, Target};

/// qb-runtime refers to these libqb symbols from code the JIT never calls.
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
}

/// Function builder.
pub struct Fb {
    pub f: Func,
    cur: usize,
}

impl Fb {
    pub fn new(name: &str) -> Fb {
        Fb {
            f: Func {
                name: name.into(),
                slots: vec![],
                params: vec![],
                ret: None,
                blocks: vec![Block { pc: Some(0), insts: vec![], term: Term::End }],
                frame_len: None,
            },
            cur: 0,
        }
    }
    pub fn slot(&mut self, t: Ty) -> Reg {
        self.f.slots.push(t);
        (self.f.slots.len() - 1) as Reg
    }
    pub fn param(&mut self, t: Ty) -> Reg {
        let r = self.slot(t);
        self.f.params.push(r);
        r
    }
    /// New block with a VM pc (resumable).
    pub fn block(&mut self, pc: Option<u32>) -> BlockId {
        self.f.blocks.push(Block { pc, insts: vec![], term: Term::End });
        (self.f.blocks.len() - 1) as BlockId
    }
    pub fn at(&mut self, b: BlockId) {
        self.cur = b as usize;
    }
    pub fn i(&mut self, i: Inst) {
        self.f.blocks[self.cur].insts.push(i);
    }
    pub fn t(&mut self, t: Term) {
        self.f.blocks[self.cur].term = t;
    }
    pub fn line(&mut self, line: u32, pc: u32) {
        self.i(Inst::Line { line, pc });
    }
    pub fn ci(&mut self, dst: Reg, v: i64) {
        self.i(Inst::Const { dst, imm: Imm::I(v) });
    }
    pub fn cf(&mut self, dst: Reg, v: f64) {
        self.i(Inst::Const { dst, imm: Imm::F(v) });
    }
    pub fn bin(&mut self, op: BinOp, dst: Reg, a: Reg, b: Reg) {
        self.i(Inst::Bin { op, dst, a, b });
    }
}

pub fn module(funcs: Vec<Func>, globals: Vec<Ty>) -> Module {
    Module { funcs, hosts: rt::host_table(), globals, vm_op: 0 }
}

#[derive(Debug, PartialEq)]
pub struct Outcome {
    pub status: u32,
    pub err: i32,
    pub err_line: u32,
    pub exit_pc: u32,
    pub frame: Vec<u64>,
    pub globals: Vec<u64>,
    pub gosub: Vec<u32>,
    /// (func, ret_pc, dst, callee frame contents)
    pub chain: Vec<(u32, u32, u32, Vec<u64>)>,
    pub out: String,
}

pub struct Setup<'a> {
    pub frame: &'a [u64],
    pub globals: &'a [u64],
    pub entry: u32,
    pub gosub: &'a [u32],
    pub gosub_base: u32,
    pub event: u32,
}

impl Default for Setup<'_> {
    fn default() -> Self {
        Setup { frame: &[], globals: &[], entry: ENTRY_CALL, gosub: &[], gosub_base: 0, event: 0 }
    }
}

fn outcome(m: &Module, func: FuncId, st: u32, ctx: &Ctx, frame: Vec<u64>, globals: Vec<u64>, rt: &Rt) -> Outcome {
    let chain = ctx
        .chain
        .iter()
        .map(|c: &ChainRec| {
            let n = m.funcs[c.func as usize].slots.len();
            // SAFETY: chain frames live in the context arena.
            let fr = unsafe { core::slice::from_raw_parts(c.frame, n) }.to_vec();
            let fr = mask(&m.funcs[c.func as usize].slots, fr);
            (c.func, c.ret_pc, c.dst, fr)
        })
        .collect();
    Outcome {
        status: st,
        err: ctx.err,
        err_line: ctx.err_line,
        exit_pc: ctx.exit_pc,
        frame: mask(&m.funcs[func as usize].slots, frame),
        globals: mask(&m.globals, globals),
        gosub: ctx.gosub_stack().to_vec(),
        chain,
        out: String::from_utf8_lossy(&rt.out).into_owned(),
    }
}

/// Handles/pointers differ between runs: compare only null vs non-null.
fn mask(tys: &[Ty], mut v: Vec<u64>) -> Vec<u64> {
    for (x, t) in v.iter_mut().zip(tys) {
        match t {
            Ty::Ptr => *x = u64::from(*x != 0),
            // NaN sign/payload is platform-dependent (x86 makes -NaN, aarch64 and
            // Cranelift constant folding +NaN): compare NaN-ness only.
            Ty::F64 if f64::from_bits(*x).is_nan() => *x = 0x7FF8_0000_0000_0000,
            Ty::F32 if f32::from_bits(*x as u32).is_nan() => *x = 0x7FC0_0000,
            _ => {}
        }
    }
    v
}

pub enum Engine {
    Interp,
    Jit,
    /// object file -> qb-link image -> bytes -> loaded into executable memory
    Aot,
}

/// Ahead-of-time path: compile to an object, link, serialise, load.
pub fn aot_compile(m: &Module) -> qb_jit::Compiled {
    let obj = qb_jit::aot::compile_object(m, Target::Host).expect("object");
    let img = qb_link::link(&obj).expect("link");
    let img = qb_link::Image::from_bytes(&img.to_bytes()).expect("image bytes");
    let syms: std::collections::HashMap<String, usize> = qb_jit::symbols(m).into_iter().collect();
    let loaded = qb_link::load(&img, &|n| syms.get(n).copied()).expect("load");
    let fns = (0..m.funcs.len())
        .map(|i| {
            let p = loaded.symbol(&qb_jit::func_symbol(i)).expect("function symbol");
            unsafe { core::mem::transmute::<*const u8, qb_jit::NativeFn>(p) }
        })
        .collect();
    unsafe { qb_jit::Compiled::from_loaded(fns, Box::new(loaded)) }
}

pub fn run_one(m: &Module, func: FuncId, s: &Setup, e: Engine) -> Outcome {
    run_full(m, func, s, e, [core::ptr::null_mut(); 2])
}

pub fn run_with_tables(m: &Module, s: &Setup, e: Engine, tables: [*mut u64; 2]) -> Outcome {
    run_full(m, 0, s, e, tables)
}

fn run_full(m: &Module, func: FuncId, s: &Setup, e: Engine, tables: [*mut u64; 2]) -> Outcome {
    let n = m.funcs[func as usize].slots.len();
    let mut frame = vec![0u64; n];
    frame[..s.frame.len()].copy_from_slice(s.frame);
    let mut globals = vec![0u64; m.globals.len().max(1)];
    globals[..s.globals.len()].copy_from_slice(s.globals);
    let mut ctx = Ctx::with_capacity(globals.as_mut_ptr(), 64, 1 << 16);
    let mut rtst = Rt::default();
    ctx.user = &mut rtst as *mut Rt as *mut _;
    for &g in s.gosub {
        unsafe { *ctx.gosub.add(ctx.gosub_len as usize) = g };
        ctx.gosub_len += 1;
    }
    ctx.gosub_base = s.gosub_base;
    ctx.tables = tables;
    ctx.event.store(s.event, core::sync::atomic::Ordering::Relaxed);
    let st = match e {
        Engine::Interp => {
            let th = Thunks::new(m);
            unsafe { interp::run(m, &th, &mut ctx, func, frame.as_mut_ptr(), s.entry) }
        }
        Engine::Jit => {
            let c = compile(m, Target::Host).expect("compile");
            unsafe { c.run(&mut ctx, func, frame.as_mut_ptr(), s.entry) }
        }
        Engine::Aot => {
            let c = aot_compile(m);
            unsafe { c.run(&mut ctx, func, frame.as_mut_ptr(), s.entry) }
        }
    };
    globals.truncate(m.globals.len());
    outcome(m, func, st, &ctx, frame, globals, &rtst)
}

/// Runs in both engines, asserts identical outcomes, returns it.
pub fn both(m: &Module, func: FuncId, s: &Setup) -> Outcome {
    let a = run_one(m, func, s, Engine::Interp);
    let b = run_one(m, func, s, Engine::Jit);
    assert_eq!(a, b, "interp vs jit differ");
    let c = run_one(m, func, s, Engine::Aot);
    assert_eq!(a, c, "interp vs aot differ");
    a
}

pub fn f64s(v: u64) -> f64 {
    f64::from_bits(v)
}
pub fn f32s(v: u64) -> f32 {
    f32::from_bits(v as u32)
}
