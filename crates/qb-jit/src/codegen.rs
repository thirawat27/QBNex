//! MIR -> Cranelift IR -> native code (in memory, no linker).
//!
//! Every native function has the signature
//! `extern "C" fn(ctx: *mut Ctx, frame: *mut u64, entry: u32) -> u32`
//! (`entry` = `ENTRY_CALL` or the VM pc of a block to enter: OSR).
//!
//! Frame slots live in Cranelift variables (registers) between the entry
//! load and the exits; a slot whose address is taken (`AddrOf`, BYREF
//! argument) stays in memory. Every exit except a normal return writes the
//! variables back, so the VM sees the exact frame state at the exit pc.

use crate::ctx::*;
use crate::mir::*;
use crate::thunk::abi;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{self, types, AbiParam, Block as CBlock, InstBuilder, MemFlagsData, Signature, Value};
use cranelift_codegen::isa::{CallConv, OwnedTargetIsa};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Switch, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, FuncId as CFuncId, Linkage, Module as _};
use std::collections::{BTreeSet, HashMap, HashSet};

/// Functions with more slots than this keep every slot in memory (keeps
/// SSA construction cheap for huge module-level code).
const MAX_REG_SLOTS: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Host,
    /// Cross targets: code is generated (not run) to check the lowering.
    X86_64,
    Aarch64,
}

pub(crate) fn make_isa(t: Target) -> OwnedTargetIsa {
    let mut fb = settings::builder();
    fb.set("opt_level", "speed").expect("flag");
    fb.set("is_pic", "false").expect("flag");
    fb.set("use_colocated_libcalls", "false").expect("flag");
    let flags = settings::Flags::new(fb);
    match t {
        Target::Host => cranelift_native::builder().expect("host ISA").finish(flags).expect("isa"),
        Target::X86_64 => cranelift_codegen::isa::lookup_by_name("x86_64-unknown-linux-gnu")
            .expect("x86_64")
            .finish(flags)
            .expect("isa"),
        Target::Aarch64 => cranelift_codegen::isa::lookup_by_name("aarch64-unknown-linux-gnu")
            .expect("aarch64")
            .finish(flags)
            .expect("isa"),
    }
}

pub(crate) fn clif_ty(t: Ty) -> ir::Type {
    match t {
        Ty::I16 => types::I16,
        Ty::I32 => types::I32,
        Ty::I64 | Ty::Ptr => types::I64,
        Ty::F32 => types::F32,
        Ty::F64 => types::F64,
    }
}

pub type NativeFn = unsafe extern "C" fn(*mut Ctx, *mut u64, u32) -> u32;

/// Compiled module: owns the code memory (released on drop).
pub struct Compiled {
    jit: Option<JITModule>,
    pub fns: Vec<NativeFn>,
}

impl Drop for Compiled {
    fn drop(&mut self) {
        if let Some(j) = self.jit.take() {
            // SAFETY: the function pointers die with `self`; nothing can call them afterwards.
            unsafe { j.free_memory() };
        }
    }
}

#[derive(Debug)]
pub struct CompileError(pub String);

fn entry_sig(cc: CallConv) -> Signature {
    let mut s = Signature::new(cc);
    s.params.push(AbiParam::new(types::I64));
    s.params.push(AbiParam::new(types::I64));
    s.params.push(AbiParam::new(types::I32));
    s.returns.push(AbiParam::new(types::I32));
    s
}

/// Every external function native code calls, by stable name: runtime
/// helpers, the module's host functions (`qb_host_<index>`) and the VM
/// callback (`qb_vm_op`). JIT code resolves them to these addresses; AOT
/// objects carry relocations against the names.
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

/// Declares and defines every function of `m` in `module`.
pub(crate) fn define_all<M: cranelift_module::Module>(
    module: &mut M,
    m: &Module,
    linkage: Linkage,
) -> Result<Vec<CFuncId>, CompileError> {
    m.validate().map_err(|e| CompileError(e.0))?;
    let isa = module.isa();
    if isa.pointer_bits() != 64 {
        return Err(CompileError("64-bit targets only".into()));
    }
    let cc = isa.default_call_conv();
    let frontend = isa.frontend_config();
    let sig = entry_sig(cc);
    let ids: Vec<CFuncId> = (0..m.funcs.len())
        .map(|i| module.declare_function(&func_symbol(i), linkage, &sig))
        .collect::<Result<_, _>>()
        .map_err(|e| CompileError(e.to_string()))?;
    let mut syms = HashMap::new();
    for (name, addr) in symbols(m) {
        syms.entry(addr).or_insert(name);
    }
    let mut fctx = FunctionBuilderContext::new();
    let mut cctx = module.make_context();
    for (i, f) in m.funcs.iter().enumerate() {
        cctx.func.signature = sig.clone();
        {
            let b = FunctionBuilder::new(&mut cctx.func, &mut fctx);
            let mut g = Gen::new(m, f, b, cc, frontend, module, &syms);
            let frefs: Vec<ir::FuncRef> = ids.iter().map(|id| g.module.declare_func_in_func(*id, g.b.func)).collect();
            g.frefs = frefs;
            g.emit();
        }
        module.define_function(ids[i], &mut cctx).map_err(|e| CompileError(format!("{}: {e:?}", f.name)))?;
        module.clear_context(&mut cctx);
    }
    Ok(ids)
}

pub fn compile(m: &Module, target: Target) -> Result<Compiled, CompileError> {
    let isa = make_isa(target);
    let mut jb = JITBuilder::with_isa(isa, default_libcall_names());
    for (name, addr) in symbols(m) {
        jb.symbol(name, addr as *const u8);
    }
    let mut jit = JITModule::new(jb);
    let ids = define_all(&mut jit, m, Linkage::Local)?;
    jit.finalize_definitions().map_err(|e| CompileError(e.to_string()))?;
    let fns = ids
        .iter()
        .map(|id| {
            let p = jit.get_finalized_function(*id);
            // SAFETY: every function was defined with `entry_sig`.
            unsafe { core::mem::transmute::<*const u8, NativeFn>(p) }
        })
        .collect();
    Ok(Compiled { jit: Some(jit), fns })
}

struct Gen<'a, M: cranelift_module::Module> {
    module: &'a mut M,
    /// address -> import name (see [`symbols`])
    syms: &'a HashMap<usize, String>,
    imported: HashMap<String, ir::FuncRef>,
    m: &'a Module,
    f: &'a Func,
    b: FunctionBuilder<'a>,
    cc: CallConv,
    frefs: Vec<ir::FuncRef>,
    /// Slots kept in memory.
    resident: Vec<bool>,
    /// Slots written somewhere (written back on exit).
    defined: Vec<bool>,
    blocks: Vec<CBlock>,
    ctx: Value,
    v_frame: Variable,
    v_globals: Variable,
    v_line: Variable,
    v_pc: Variable,
    v_base: Variable,
    err_blk: CBlock,
    host_err_blk: CBlock,
    exit_blk: CBlock,
    event_blk: CBlock,
    deopt_blk: CBlock,
    cur: usize,
    vars: Vec<Variable>,
    frontend: cranelift_codegen::isa::TargetFrontendConfig,
    /// Event-poll edge blocks, emitted after the body: (poll block, pc, target).
    polls: Vec<(CBlock, u32, CBlock)>,
}



impl<'a, M: cranelift_module::Module> Gen<'a, M> {
    fn new(
        m: &'a Module,
        f: &'a Func,
        b: FunctionBuilder<'a>,
        cc: CallConv,
        frontend: cranelift_codegen::isa::TargetFrontendConfig,
        module: &'a mut M,
        syms: &'a HashMap<usize, String>,
    ) -> Gen<'a, M> {
        let n = f.slots.len();
        let nf = f.frame_len();
        let mut resident: Vec<bool> = (0..n).map(|r| r < nf && nf > MAX_REG_SLOTS).collect();
        let mut defined = vec![false; n];
        for blk in &f.blocks {
            for i in &blk.insts {
                match i {
                    Inst::AddrOf { slot, .. } => resident[*slot as usize] = true,
                    Inst::Call { args, .. } => {
                        for a in args {
                            if let Arg::Ref(r) = a {
                                resident[*r as usize] = true;
                            }
                        }
                    }
                    _ => {}
                }
                if let Some(d) = inst_def(i) {
                    defined[d as usize] = true;
                }
                if let Inst::VmOp { writes, .. } = i {
                    for w in writes {
                        defined[*w as usize] = true;
                    }
                }
            }
        }
        let dummy = CBlock::from_u32(0);
        let z = Value::from_u32(0);
        Gen {
            module,
            syms,
            imported: HashMap::new(),
            m,
            f,
            b,
            cc,
            frefs: Vec::new(),
            resident,
            defined,
            blocks: Vec::new(),
            ctx: z,
            v_frame: Variable::from_u32(0),
            v_globals: Variable::from_u32(0),
            v_line: Variable::from_u32(0),
            v_pc: Variable::from_u32(0),
            v_base: Variable::from_u32(0),
            err_blk: dummy,
            host_err_blk: dummy,
            exit_blk: dummy,
            event_blk: dummy,
            deopt_blk: dummy,
            cur: 0,
            vars: Vec::new(),
            frontend,
            polls: Vec::new(),
        }
    }

    fn ty(&self, r: Reg) -> Ty {
        self.f.slots[r as usize]
    }

    // ---------------------------------------------------------- memory helpers

    fn mem_load(&mut self, t: Ty, addr: Value, off: i32) -> Value {
        self.b.ins().load(clif_ty(t), MemFlagsData::new().with_notrap(), addr, off)
    }

    /// Natural-size store.
    fn mem_store(&mut self, t: Ty, v: Value, addr: Value, off: i32) {
        let _ = t;
        self.b.ins().store(MemFlagsData::new().with_notrap(), v, addr, off);
    }

    /// Slot store: integers sign-extended to 8 bytes.
    fn slot_store(&mut self, t: Ty, v: Value, addr: Value, off: i32) {
        let v = match t {
            Ty::I16 | Ty::I32 => self.b.ins().sextend(types::I64, v),
            _ => v,
        };
        self.b.ins().store(MemFlagsData::trusted(), v, addr, off);
    }

    fn get(&mut self, r: Reg) -> Value {
        if self.resident[r as usize] {
            let t = self.ty(r);
            let fr = self.fr();
            self.b.ins().load(clif_ty(t), MemFlagsData::trusted(), fr, (r * 8) as i32)
        } else {
            self.b.use_var(self.vars[(r) as usize])
        }
    }

    fn set(&mut self, r: Reg, v: Value) {
        if self.resident[r as usize] {
            let t = self.ty(r);
            let fr = self.fr();
            self.slot_store(t, v, fr, (r * 8) as i32);
        } else {
            self.b.def_var(self.vars[(r) as usize], v);
        }
    }

    fn fr(&mut self) -> Value {
        self.b.use_var(self.v_frame)
    }

    fn gl(&mut self) -> Value {
        self.b.use_var(self.v_globals)
    }

    fn ctx_load(&mut self, t: ir::Type, off: i32) -> Value {
        let c = self.ctx;
        self.b.ins().load(t, MemFlagsData::trusted(), c, off)
    }

    /// `*ctx.ev_ptr & ctx.ev_mask` (non-zero = event pending).
    fn event_word(&mut self) -> Value {
        let p = self.ctx_load(types::I64, OFF_EV_PTR);
        let w = self.b.ins().load(types::I32, MemFlagsData::trusted(), p, 0);
        let m = self.ctx_load(types::I32, OFF_EV_MASK);
        self.b.ins().band(w, m)
    }

    fn ctx_store(&mut self, v: Value, off: i32) {
        let c = self.ctx;
        self.b.ins().store(MemFlagsData::trusted(), v, c, off);
    }

    // ---------------------------------------------------------- control helpers

    /// Branches to the error exit with QB error `code` when `cond` is true.
    fn guard(&mut self, cond: Value, code: i32) {
        let ok = self.b.create_block();
        let bad = self.b.create_block();
        self.b.set_cold_block(bad);
        self.b.ins().brif(cond, bad, &[], ok, &[]);
        self.b.switch_to_block(bad);
        let c = self.b.ins().iconst(types::I32, i64::from(code));
        self.b.ins().jump(self.err_blk, &[c.into()]);
        self.b.switch_to_block(ok);
    }

    /// Edge `cur -> t`, polling events on back-edges to resumable blocks.
    fn edge(&mut self, t: BlockId) -> CBlock {
        let tb = &self.f.blocks[t as usize];
        match tb.pc {
            Some(pc) if (t as usize) <= self.cur => {
                let poll = self.b.create_block();
                let target = self.blocks[t as usize];
                self.polls.push((poll, pc, target));
                poll
            }
            _ => self.blocks[t as usize],
        }
    }

    // ---------------------------------------------------------- function

    fn emit(mut self) {
        let entry = self.b.create_block();
        self.b.append_block_params_for_function_params(entry);
        self.b.switch_to_block(entry);
        let p = self.b.block_params(entry).to_vec();
        self.ctx = p[0];
        self.v_frame = self.b.declare_var(types::I64);
        self.b.def_var(self.v_frame, p[1]);
        let entry_pc = p[2];

        self.v_line = self.b.declare_var(types::I32);
        self.v_pc = self.b.declare_var(types::I32);
        self.v_base = self.b.declare_var(types::I32);
        for r in 0..self.f.slots.len() as u32 {
            let v = self.b.declare_var(clif_ty(self.ty(r)));
            self.vars.push(v);
        }

        self.v_globals = self.b.declare_var(types::I64);
        let g = self.ctx_load(types::I64, OFF_GLOBALS);
        self.b.def_var(self.v_globals, g);
        // Load register slots; temporaries start at zero.
        let nf = self.f.frame_len();
        for r in 0..self.f.slots.len() as u32 {
            if !self.resident[r as usize] {
                let t = self.ty(r);
                let v = if (r as usize) < nf {
                    let fr = self.fr();
                    self.b.ins().load(clif_ty(t), MemFlagsData::trusted(), fr, (r * 8) as i32)
                } else {
                    match t {
                        Ty::F32 => self.b.ins().f32const(0.0),
                        Ty::F64 => self.b.ins().f64const(0.0),
                        _ => self.b.ins().iconst(clif_ty(t), 0),
                    }
                };
                self.b.def_var(self.vars[(r) as usize], v);
            }
        }
        let zero = self.b.ins().iconst(types::I32, 0);
        self.b.def_var(self.v_line, zero);
        let is_call = self.b.ins().icmp_imm_s(IntCC::Equal, entry_pc, i64::from(ENTRY_CALL));
        let pc0 = self.b.ins().select(is_call, zero, entry_pc);
        self.b.def_var(self.v_pc, pc0);
        let glen = self.ctx_load(types::I32, OFF_GOSUB_LEN);
        let gbase = self.ctx_load(types::I32, OFF_GOSUB_BASE);
        let base = self.b.ins().select(is_call, glen, gbase);
        self.b.def_var(self.v_base, base);

        // Shared exit blocks.
        self.exit_blk = self.b.create_block();
        self.b.append_block_param(self.exit_blk, types::I32);
        self.err_blk = self.b.create_block();
        self.b.append_block_param(self.err_blk, types::I32);
        self.host_err_blk = self.b.create_block();
        self.event_blk = self.b.create_block();
        self.b.append_block_param(self.event_blk, types::I32);
        self.deopt_blk = self.b.create_block();
        self.b.append_block_param(self.deopt_blk, types::I32);
        for blk in [self.err_blk, self.host_err_blk, self.event_blk, self.deopt_blk, self.exit_blk] {
            self.b.set_cold_block(blk);
        }

        self.blocks = (0..self.f.blocks.len()).map(|_| self.b.create_block()).collect();

        // Entry dispatch.
        let osr = self.b.create_block();
        let b0 = self.blocks[0];
        self.b.ins().brif(is_call, b0, &[], osr, &[]);
        self.b.switch_to_block(osr);
        let mut sw = Switch::new();
        for (i, blk) in self.f.blocks.iter().enumerate() {
            if let Some(pc) = blk.pc {
                if pc != ENTRY_CALL && !sw.entries().contains_key(&u128::from(pc)) {
                    sw.set_entry(u128::from(pc), self.blocks[i]);
                }
            }
        }
        let unknown = self.b.create_block();
        sw.emit(&mut self.b, entry_pc, unknown);
        self.b.switch_to_block(unknown);
        self.b.ins().jump(self.deopt_blk, &[entry_pc.into()]);

        for i in 0..self.f.blocks.len() {
            self.cur = i;
            let cb = self.blocks[i];
            self.b.switch_to_block(cb);
            let blk = &self.f.blocks[i];
            for inst in &blk.insts {
                self.inst(inst);
            }
            self.term(&blk.term);
        }

        self.emit_exits();
        self.b.seal_all_blocks();
        self.b.finalize(self.frontend);
    }

    fn emit_exits(&mut self) {
        for (poll, pc, target) in std::mem::take(&mut self.polls) {
            self.b.switch_to_block(poll);
            let ev = self.event_word();
            let pcv = self.b.ins().iconst(types::I32, i64::from(pc));
            self.b.ins().brif(ev, self.event_blk, &[pcv.into()], target, &[]);
        }
        // err(code): set err, err_line, exit_pc.
        self.b.switch_to_block(self.err_blk);
        let code = self.b.block_params(self.err_blk)[0];
        self.ctx_store(code, OFF_ERR);
        self.b.ins().jump(self.host_err_blk, &[]);

        // host_err: err already set.
        self.b.switch_to_block(self.host_err_blk);
        let l = self.b.use_var(self.v_line);
        self.ctx_store(l, OFF_ERR_LINE);
        let pc = self.b.use_var(self.v_pc);
        self.ctx_store(pc, OFF_EXIT_PC);
        let st = self.b.ins().iconst(types::I32, i64::from(status::ERROR));
        self.b.ins().jump(self.exit_blk, &[st.into()]);

        self.b.switch_to_block(self.event_blk);
        let pc = self.b.block_params(self.event_blk)[0];
        self.ctx_store(pc, OFF_EXIT_PC);
        let st = self.b.ins().iconst(types::I32, i64::from(status::EVENT));
        self.b.ins().jump(self.exit_blk, &[st.into()]);

        self.b.switch_to_block(self.deopt_blk);
        let pc = self.b.block_params(self.deopt_blk)[0];
        self.ctx_store(pc, OFF_EXIT_PC);
        let st = self.b.ins().iconst(types::I32, i64::from(status::DEOPT));
        self.b.ins().jump(self.exit_blk, &[st.into()]);

        // exit(status): write back every register slot that may have changed.
        self.b.switch_to_block(self.exit_blk);
        let st = self.b.block_params(self.exit_blk)[0];
        for r in 0..self.f.slots.len() as u32 {
            if !self.resident[r as usize] && self.defined[r as usize] && (r as usize) < self.f.frame_len() {
                let v = self.b.use_var(self.vars[(r) as usize]);
                let t = self.ty(r);
                let fr = self.fr();
                self.slot_store(t, v, fr, (r * 8) as i32);
            }
        }
        self.b.ins().return_(&[st]);
    }

    fn term(&mut self, t: &Term) {
        match t {
            Term::Jmp(to) => {
                let e = self.edge(*to);
                self.b.ins().jump(e, &[]);
            }
            Term::Br { cond, then, els } => {
                let c = self.get(*cond);
                let t1 = self.edge(*then);
                let t2 = self.edge(*els);
                self.b.ins().brif(c, t1, &[], t2, &[]);
            }
            Term::Ret => {
                if let Some(r) = self.f.ret {
                    if !self.resident[r as usize] {
                        let v = self.b.use_var(self.vars[(r) as usize]);
                        let t = self.ty(r);
                        let fr = self.fr();
                        self.slot_store(t, v, fr, (r * 8) as i32);
                    }
                }
                let st = self.b.ins().iconst(types::I32, i64::from(status::OK));
                self.b.ins().return_(&[st]);
            }
            Term::Dispatch { val, cases } => {
                let v = self.get(*val);
                let v32 = match self.ty(*val) {
                    Ty::I16 => self.b.ins().uextend(types::I32, v),
                    Ty::I32 => v,
                    _ => self.b.ins().ireduce(types::I32, v),
                };
                let mut sw = Switch::new();
                for (c, t) in cases {
                    sw.set_entry(u128::from(*c), self.blocks[*t as usize]);
                }
                let other = self.b.create_block();
                self.b.set_cold_block(other);
                sw.emit(&mut self.b, v32, other);
                self.b.switch_to_block(other);
                self.b.ins().jump(self.deopt_blk, &[v32.into()]);
            }
            Term::Exit { pc } => {
                let pcv = self.b.ins().iconst(types::I32, i64::from(*pc));
                self.b.ins().jump(self.deopt_blk, &[pcv.into()]);
            }
            Term::End => {
                let z = self.b.ins().iconst(types::I32, 0);
                self.ctx_store(z, OFF_EXIT_PC);
                let st = self.b.ins().iconst(types::I32, i64::from(status::END));
                self.b.ins().jump(self.exit_blk, &[st.into()]);
            }
            Term::Gosub { target, ret_pc, .. } => {
                let len = self.ctx_load(types::I32, OFF_GOSUB_LEN);
                let cap = self.ctx_load(types::I32, OFF_GOSUB_CAP);
                let full = self.b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, len, cap);
                self.guard(full, qberr::OUT_OF_MEMORY);
                let stack = self.ctx_load(types::I64, OFF_GOSUB);
                let len64 = self.b.ins().uextend(types::I64, len);
                let off = self.b.ins().ishl_imm_s(len64, 2);
                let addr = self.b.ins().iadd(stack, off);
                let rp = self.b.ins().iconst(types::I32, i64::from(*ret_pc));
                self.b.ins().store(MemFlagsData::trusted(), rp, addr, 0);
                let len1 = self.b.ins().iadd_imm_s(len, 1);
                self.ctx_store(len1, OFF_GOSUB_LEN);
                let e = self.edge(*target);
                self.b.ins().jump(e, &[]);
            }
            Term::Return { pc } => {
                let len = self.ctx_load(types::I32, OFF_GOSUB_LEN);
                let base = self.b.use_var(self.v_base);
                let empty = self.b.ins().icmp(IntCC::UnsignedLessThanOrEqual, len, base);
                self.guard(empty, qberr::RETURN_WITHOUT_GOSUB);
                let stack = self.ctx_load(types::I64, OFF_GOSUB);
                let len1 = self.b.ins().iadd_imm_s(len, -1);
                let l64 = self.b.ins().uextend(types::I64, len1);
                let off = self.b.ins().ishl_imm_s(l64, 2);
                let addr = self.b.ins().iadd(stack, off);
                let to = self.b.ins().load(types::I32, MemFlagsData::trusted(), addr, 0);
                let mut sites: Vec<(u32, BlockId)> = Vec::new();
                let mut seen = HashSet::new();
                for blk in &self.f.blocks {
                    if let Term::Gosub { ret, ret_pc, .. } = blk.term {
                        if seen.insert(ret_pc) {
                            sites.push((ret_pc, ret));
                        }
                    }
                }
                let mut sw = Switch::new();
                let mut pending = Vec::new();
                for (rpc, rblk) in sites {
                    let pop = self.b.create_block();
                    sw.set_entry(u128::from(rpc), pop);
                    pending.push((pop, rblk));
                }
                let unknown = self.b.create_block();
                self.b.set_cold_block(unknown);
                sw.emit(&mut self.b, to, unknown);
                for (pop, rblk) in pending {
                    self.b.switch_to_block(pop);
                    self.ctx_store(len1, OFF_GOSUB_LEN);
                    let e = self.edge(rblk);
                    self.b.ins().jump(e, &[]);
                }
                self.b.switch_to_block(unknown);
                let pcv = self.b.ins().iconst(types::I32, i64::from(*pc));
                self.b.ins().jump(self.deopt_blk, &[pcv.into()]);
            }
        }
    }

    fn inst(&mut self, i: &Inst) {
        match i {
            Inst::Line { line, pc } => {
                let l = self.b.ins().iconst(types::I32, i64::from(*line));
                self.b.def_var(self.v_line, l);
                let p = self.b.ins().iconst(types::I32, i64::from(*pc));
                self.b.def_var(self.v_pc, p);
                let ev = self.event_word();
                let cont = self.b.create_block();
                self.b.ins().brif(ev, self.event_blk, &[p.into()], cont, &[]);
                self.b.switch_to_block(cont);
            }
            Inst::Const { dst, imm } => {
                let t = self.ty(*dst);
                let v = match (*imm, t) {
                    (Imm::I(v), Ty::F64) => self.b.ins().f64const(v as f64),
                    (Imm::I(v), Ty::F32) => self.b.ins().f32const(v as f32),
                    (Imm::F(v), Ty::F32) => self.b.ins().f32const(v as f32),
                    (Imm::F(v), _) => self.b.ins().f64const(v),
                    (Imm::I(v), t) => self.b.ins().iconst(clif_ty(t), wrap_imm(t, v)),
                };
                self.set(*dst, v);
            }
            Inst::Mov { dst, src } => {
                let v = self.get(*src);
                self.set(*dst, v);
            }
            Inst::Conv { dst, src } => {
                let v = self.get(*src);
                let r = self.conv(self.ty(*src), self.ty(*dst), v);
                self.set(*dst, r);
            }
            Inst::Bin { op, dst, a, b } => {
                let x = self.get(*a);
                let y = self.get(*b);
                let r = self.bin(*op, self.ty(*dst), x, y);
                self.set(*dst, r);
            }
            Inst::Cmp { cc, dst, a, b } => {
                let x = self.get(*a);
                let y = self.get(*b);
                let t = self.ty(*a);
                let c = if t.is_float() {
                    let fc = match cc {
                        Cc::Eq => FloatCC::Equal,
                        Cc::Ne => FloatCC::NotEqual,
                        Cc::Lt => FloatCC::LessThan,
                        Cc::Le => FloatCC::LessThanOrEqual,
                        Cc::Gt => FloatCC::GreaterThan,
                        Cc::Ge => FloatCC::GreaterThanOrEqual,
                    };
                    self.b.ins().fcmp(fc, x, y)
                } else {
                    let ic = match cc {
                        Cc::Eq => IntCC::Equal,
                        Cc::Ne => IntCC::NotEqual,
                        Cc::Lt => IntCC::SignedLessThan,
                        Cc::Le => IntCC::SignedLessThanOrEqual,
                        Cc::Gt => IntCC::SignedGreaterThan,
                        Cc::Ge => IntCC::SignedGreaterThanOrEqual,
                    };
                    self.b.ins().icmp(ic, x, y)
                };
                let dt = clif_ty(self.ty(*dst));
                let m1 = self.b.ins().iconst(dt, -1);
                let z = self.b.ins().iconst(dt, 0);
                let r = self.b.ins().select(c, m1, z);
                self.set(*dst, r);
            }
            Inst::Neg { dst, src } => {
                let v = self.get(*src);
                let r = if self.ty(*src).is_float() { self.b.ins().fneg(v) } else { self.b.ins().ineg(v) };
                self.set(*dst, r);
            }
            Inst::Not { dst, src } => {
                let v = self.get(*src);
                let r = self.b.ins().bnot(v);
                self.set(*dst, r);
            }
            Inst::GLoad { dst, slot } => {
                let t = self.ty(*dst);
                let g = self.gl();
                let v = self.b.ins().load(clif_ty(t), MemFlagsData::trusted(), g, (*slot * 8) as i32);
                self.set(*dst, v);
            }
            Inst::GStore { slot, src } => {
                let t = self.ty(*src);
                let v = self.get(*src);
                let g = self.gl();
                self.slot_store(t, v, g, (*slot * 8) as i32);
            }
            Inst::AddrOf { dst, slot } => {
                let fr = self.fr();
                let a = self.b.ins().iadd_imm_s(fr, i64::from(*slot) * 8);
                self.set(*dst, a);
            }
            Inst::GAddrOf { dst, slot } => {
                let g = self.gl();
                let a = self.b.ins().iadd_imm_s(g, i64::from(*slot) * 8);
                self.set(*dst, a);
            }
            Inst::LoadPtr { dst, ptr } => {
                let p = self.get(*ptr);
                let v = self.mem_load(self.ty(*dst), p, 0);
                self.set(*dst, v);
            }
            Inst::StorePtr { ptr, src } => {
                let p = self.get(*ptr);
                let v = self.get(*src);
                self.mem_store(self.ty(*src), v, p, 0);
            }
            Inst::ALoad { dst, arr, idx } => {
                let t = self.ty(*dst);
                let a = self.elem_addr(*arr, idx, t);
                let v = self.mem_load(t, a, 0);
                self.set(*dst, v);
            }
            Inst::AStore { arr, idx, src } => {
                let t = self.ty(*src);
                let a = self.elem_addr(*arr, idx, t);
                let v = self.get(*src);
                self.mem_store(t, v, a, 0);
            }
            Inst::Call { func, args, dst, pc } => self.call(*func, args, *dst, *pc),
            Inst::Host { f, args, dst } => {
                let h = &self.m.hosts[*f as usize];
                let mut sig = Signature::new(self.cc);
                sig.params.push(AbiParam::new(types::I64));
                let mut vals = vec![self.ctx];
                for (a, t) in args.iter().zip(&h.sig.params) {
                    sig.params.push(abi(*t));
                    let v = self.get(*a);
                    vals.push(v);
                }
                if let Some(t) = h.sig.ret {
                    sig.returns.push(abi(t));
                }
                let fref = self.import(h.addr, sig);
                let call = self.b.ins().call(fref, &vals);
                if let Some(d) = dst {
                    let r = self.b.inst_results(call)[0];
                    self.set(*d, r);
                }
                if h.sig.may_error {
                    let e = self.ctx_load(types::I32, OFF_ERR);
                    let cont = self.b.create_block();
                    self.b.ins().brif(e, self.host_err_blk, &[], cont, &[]);
                    self.b.switch_to_block(cont);
                }
            }
            Inst::At { pc } => {
                let p = self.b.ins().iconst(types::I32, i64::from(*pc));
                self.b.def_var(self.v_pc, p);
            }
            Inst::Bitcast { dst, src } => {
                let v = self.get(*src);
                let to = if self.ty(*dst) == Ty::F64 { types::F64 } else { types::I64 };
                let r = self.b.ins().bitcast(to, MemFlagsData::new(), v);
                self.set(*dst, r);
            }
            Inst::ConvWrap { dst, src } => {
                let (from, to) = (self.ty(*src), self.ty(*dst));
                let v = self.get(*src);
                let r = if from.is_float() && !to.is_float() && to != Ty::I64 && to != Ty::Ptr {
                    let q = self.conv(from, Ty::I64, v);
                    self.b.ins().ireduce(clif_ty(to), q)
                } else {
                    self.conv(from, to, v)
                };
                self.set(*dst, r);
            }
            Inst::Trunc { dst, src } => {
                let to = self.ty(*dst);
                let v = self.get(*src);
                let d = if self.ty(*src) == Ty::F32 { self.b.ins().fpromote(types::F64, v) } else { v };
                let r = match to {
                    Ty::I16 => {
                        let i = self.b.ins().fcvt_to_sint_sat(types::I32, d);
                        let lo = self.b.ins().iconst(types::I32, -32768);
                        let hi = self.b.ins().iconst(types::I32, 32767);
                        let c = self.b.ins().smax(i, lo);
                        let c = self.b.ins().smin(c, hi);
                        self.b.ins().ireduce(types::I16, c)
                    }
                    t => self.b.ins().fcvt_to_sint_sat(clif_ty(t), d),
                };
                self.set(*dst, r);
            }
            Inst::Math { op, dst, src } => {
                let v = self.get(*src);
                let st = self.ty(*src);
                let dt = clif_ty(self.ty(*dst));
                let r = match op {
                    MathOp::Abs if st.is_float() => self.b.ins().fabs(v),
                    MathOp::Abs => self.b.ins().iabs(v),
                    MathOp::Sgn => {
                        let (gt, lt) = if st.is_float() {
                            let z = if st == Ty::F32 { self.b.ins().f32const(0.0) } else { self.b.ins().f64const(0.0) };
                            (self.b.ins().fcmp(FloatCC::GreaterThan, v, z), self.b.ins().fcmp(FloatCC::LessThan, v, z))
                        } else {
                            (self.b.ins().icmp_imm_s(IntCC::SignedGreaterThan, v, 0), self.b.ins().icmp_imm_s(IntCC::SignedLessThan, v, 0))
                        };
                        let one = self.b.ins().iconst(dt, 1);
                        let m1 = self.b.ins().iconst(dt, -1);
                        let z = self.b.ins().iconst(dt, 0);
                        let n = self.b.ins().select(lt, m1, z);
                        self.b.ins().select(gt, one, n)
                    }
                    MathOp::Floor => self.b.ins().floor(v),
                    MathOp::Fix => self.b.ins().trunc(v),
                    MathOp::Sqrt => {
                        let z = if st == Ty::F32 { self.b.ins().f32const(0.0) } else { self.b.ins().f64const(0.0) };
                        let neg = self.b.ins().fcmp(FloatCC::LessThan, v, z);
                        self.guard(neg, qberr::ILLEGAL_CALL);
                        self.b.ins().sqrt(v)
                    }
                    MathOp::RoundF32 => {
                        let f = self.b.ins().fdemote(types::F32, v);
                        self.b.ins().fpromote(types::F64, f)
                    }
                    MathOp::Wrap16 => {
                        let n = self.b.ins().ireduce(types::I16, v);
                        self.b.ins().sextend(types::I64, n)
                    }
                    MathOp::Wrap32 => {
                        let n = self.b.ins().ireduce(types::I32, v);
                        self.b.ins().sextend(types::I64, n)
                    }
                };
                self.set(*dst, r);
            }
            Inst::VmOp { op, reads, writes } => {
                for r in reads.iter().chain(writes) {
                    if !self.resident[*r as usize] {
                        let v = self.b.use_var(self.vars[(*r) as usize]);
                        let t = self.ty(*r);
                        let fr = self.fr();
                        self.slot_store(t, v, fr, (*r * 8) as i32);
                    }
                }
                let (ctx, fr) = (self.ctx, self.fr());
                let opv = self.b.ins().iconst(types::I32, i64::from(*op));
                let st = self
                    .helper_call(self.m.vm_op, &[types::I64, types::I64, types::I32], Some(types::I32), &[ctx, fr, opv])
                    .expect("status");
                // The VM may have moved its register file (calls).
                let fr = self.ctx_load(types::I64, OFF_FRAME);
                self.b.def_var(self.v_frame, fr);
                let g = self.ctx_load(types::I64, OFF_GLOBALS);
                self.b.def_var(self.v_globals, g);
                for w in writes {
                    if !self.resident[*w as usize] {
                        let t = self.ty(*w);
                        let v = self.b.ins().load(clif_ty(t), MemFlagsData::trusted(), fr, (*w * 8) as i32);
                        self.b.def_var(self.vars[(*w) as usize], v);
                    }
                }
                let cont = self.b.create_block();
                self.b.ins().brif(st, self.exit_blk, &[st.into()], cont, &[]);
                self.b.switch_to_block(cont);
            }
            Inst::VAElem { load, reg, table, index, view_off, elem, idx, slow } => {
                self.va_elem(*load, *reg, *table, *index, *view_off, *elem, *idx, *slow)
            }
            Inst::Raise { code } => {
                let v = self.get(*code);
                let c = match self.ty(*code) {
                    Ty::I16 => self.b.ins().sextend(types::I32, v),
                    Ty::I32 => v,
                    _ => self.b.ins().ireduce(types::I32, v),
                };
                self.b.ins().jump(self.err_blk, &[c.into()]);
                // Unreachable continuation for any following instructions.
                let dead = self.b.create_block();
                self.b.switch_to_block(dead);
            }
        }
    }

    fn helper_call(&mut self, addr: usize, params: &[ir::Type], ret: Option<ir::Type>, args: &[Value]) -> Option<Value> {
        let mut sig = Signature::new(self.cc);
        for p in params {
            sig.params.push(AbiParam::new(*p));
        }
        if let Some(r) = ret {
            sig.returns.push(AbiParam::new(r));
        }
        let fref = self.import(addr, sig);
        let c = self.b.ins().call(fref, args);
        self.b.inst_results(c).first().copied()
    }

    /// Function reference for the external function at `addr` (by its symbol name).
    fn import(&mut self, addr: usize, sig: Signature) -> ir::FuncRef {
        let name = self.syms.get(&addr).expect("every called address has a symbol").clone();
        if let Some(f) = self.imported.get(&name) {
            return *f;
        }
        let id = self.module.declare_function(&name, Linkage::Import, &sig).expect("consistent import signature");
        let f = self.module.declare_func_in_func(id, self.b.func);
        self.imported.insert(name, f);
        f
    }

    fn conv(&mut self, from: Ty, to: Ty, v: Value) -> Value {
        if !from.is_float() {
            return match to {
                Ty::F64 => self.b.ins().fcvt_from_sint(types::F64, v),
                Ty::F32 => {
                    let d = self.b.ins().fcvt_from_sint(types::F64, v);
                    self.b.ins().fdemote(types::F32, d)
                }
                t => {
                    let (fb, tb) = (clif_ty(from).bits(), clif_ty(t).bits());
                    if fb == tb {
                        v
                    } else if fb < tb {
                        self.b.ins().sextend(clif_ty(t), v)
                    } else {
                        self.b.ins().ireduce(clif_ty(t), v)
                    }
                }
            };
        }
        let d = if from == Ty::F32 { self.b.ins().fpromote(types::F64, v) } else { v };
        match to {
            Ty::F64 => d,
            Ty::F32 => self.b.ins().fdemote(types::F32, d),
            Ty::I16 | Ty::I32 => {
                let (lo, hi) = if to == Ty::I16 { (-32768.0, 32767.0) } else { (-2147483648.0, 2147483647.0) };
                let r = self.b.ins().nearest(d);
                let lov = self.b.ins().f64const(lo);
                let hiv = self.b.ins().f64const(hi);
                let ge = self.b.ins().fcmp(FloatCC::GreaterThanOrEqual, r, lov);
                let le = self.b.ins().fcmp(FloatCC::LessThanOrEqual, r, hiv);
                let ok = self.b.ins().band(ge, le);
                let bad = self.b.ins().bxor_imm_s(ok, 1);
                self.guard(bad, qberr::OVERFLOW);
                let i = self.b.ins().fcvt_to_sint_sat(types::I32, r);
                if to == Ty::I16 {
                    self.b.ins().ireduce(types::I16, i)
                } else {
                    i
                }
            }
            _ => {
                // INTEGER64: inline when |rounded| < 2^63, else the exact runtime helper.
                let r = self.b.ins().nearest(d);
                let lim = self.b.ins().f64const(9223372036854775808.0);
                let neg = self.b.ins().f64const(-9223372036854775808.0);
                let lt = self.b.ins().fcmp(FloatCC::LessThan, r, lim);
                let ge = self.b.ins().fcmp(FloatCC::GreaterThanOrEqual, r, neg);
                let ok = self.b.ins().band(lt, ge);
                let fast = self.b.create_block();
                let slow = self.b.create_block();
                let join = self.b.create_block();
                self.b.append_block_param(join, types::I64);
                self.b.set_cold_block(slow);
                self.b.ins().brif(ok, fast, &[], slow, &[]);
                self.b.switch_to_block(fast);
                let i = self.b.ins().fcvt_to_sint_sat(types::I64, r);
                self.b.ins().jump(join, &[i.into()]);
                self.b.switch_to_block(slow);
                let s = self
                    .helper_call(qbr_f64 as *const () as usize, &[types::F64], Some(types::I64), &[d])
                    .expect("result");
                self.b.ins().jump(join, &[s.into()]);
                self.b.switch_to_block(join);
                self.b.block_params(join)[0]
            }
        }
    }

    fn bin(&mut self, op: BinOp, t: Ty, x: Value, y: Value) -> Value {
        use BinOp::*;
        if t.is_float() {
            return match op {
                Add => self.b.ins().fadd(x, y),
                Sub => self.b.ins().fsub(x, y),
                Mul => self.b.ins().fmul(x, y),
                Div => {
                    let z = if t == Ty::F32 { self.b.ins().f32const(0.0) } else { self.b.ins().f64const(0.0) };
                    let isz = self.b.ins().fcmp(FloatCC::Equal, y, z);
                    self.guard(isz, qberr::DIV_ZERO);
                    self.b.ins().fdiv(x, y)
                }
                FDiv => self.b.ins().fdiv(x, y),
                Pow => {
                    let (addr, ct) =
                        if t == Ty::F32 { (pow_f32 as *const () as usize, types::F32) } else { (pow_f64 as *const () as usize, types::F64) };
                    self.helper_call(addr, &[ct, ct], Some(ct), &[x, y]).expect("result")
                }
                _ => unreachable!("validated"),
            };
        }
        match op {
            Add => self.b.ins().iadd(x, y),
            Sub => self.b.ins().isub(x, y),
            Mul => self.b.ins().imul(x, y),
            And => self.b.ins().band(x, y),
            Or => self.b.ins().bor(x, y),
            Xor => self.b.ins().bxor(x, y),
            Eqv => {
                let v = self.b.ins().bxor(x, y);
                self.b.ins().bnot(v)
            }
            Imp => {
                let n = self.b.ins().bnot(x);
                self.b.ins().bor(n, y)
            }
            IDiv | Mod => {
                let isz = self.b.ins().icmp_imm_s(IntCC::Equal, y, 0);
                self.guard(isz, qberr::DIV_ZERO);
                if t == Ty::I64 {
                    // i64::MIN / -1 traps in hardware; QB wraps.
                    let m1 = self.b.ins().icmp_imm_s(IntCC::Equal, y, -1);
                    let one = self.b.ins().iconst(types::I64, 1);
                    let ys = self.b.ins().select(m1, one, y);
                    if op == IDiv {
                        let q = self.b.ins().sdiv(x, ys);
                        let nx = self.b.ins().ineg(x);
                        self.b.ins().select(m1, nx, q)
                    } else {
                        let r = self.b.ins().srem(x, ys);
                        let z = self.b.ins().iconst(types::I64, 0);
                        self.b.ins().select(m1, z, r)
                    }
                } else {
                    let ct = clif_ty(t);
                    let x64 = self.b.ins().sextend(types::I64, x);
                    let y64 = self.b.ins().sextend(types::I64, y);
                    let r = if op == IDiv { self.b.ins().sdiv(x64, y64) } else { self.b.ins().srem(x64, y64) };
                    self.b.ins().ireduce(ct, r)
                }
            }
            Div | FDiv | Pow => unreachable!("validated"),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn va_elem(&mut self, load: bool, reg: Reg, table: u32, index: u32, view_off: u32, elem: Ty, idx: Reg, slow: HostId) {
        let iv = self.get(idx);
        let i = match self.ty(idx) {
            Ty::I16 | Ty::I32 => self.b.ins().sextend(types::I64, iv),
            _ => iv,
        };
        let tab = self.ctx_load(types::I64, OFF_TABLES + table as i32 * 8);
        let p = self.b.ins().load(types::I64, MemFlagsData::trusted(), tab, (index * 8) as i32);
        let fast = self.b.create_block();
        let fast2 = self.b.create_block();
        let slowb = self.b.create_block();
        let join = self.b.create_block();
        self.b.set_cold_block(slowb);
        let rt = clif_ty(self.ty(reg));
        if load {
            self.b.append_block_param(join, rt);
        }
        self.b.ins().brif(p, fast, &[], slowb, &[]);
        // fast: in-bounds element
        self.b.switch_to_block(fast);
        let vo = view_off as i32;
        let data = self.b.ins().load(types::I64, MemFlagsData::trusted(), p, vo + OFF_VIEW_DATA);
        let lb = self.b.ins().load(types::I64, MemFlagsData::trusted(), p, vo + OFF_VIEW_LB);
        let cnt = self.b.ins().load(types::I64, MemFlagsData::trusted(), p, vo + OFF_VIEW_COUNT);
        let k = self.b.ins().isub(i, lb);
        let inb = self.b.ins().icmp(IntCC::UnsignedLessThan, k, cnt);
        let nz = self.b.ins().icmp_imm_s(IntCC::NotEqual, data, 0);
        let ok = self.b.ins().band(inb, nz);
        self.b.ins().brif(ok, fast2, &[], slowb, &[]);
        self.b.switch_to_block(fast2);
        let off = self.b.ins().ishl_imm_s(k, i64::from(elem.size().trailing_zeros()));
        let ea = self.b.ins().iadd(data, off);
        if load {
            let x = self.mem_load(elem, ea, 0);
            let v = match elem {
                Ty::I16 | Ty::I32 => self.b.ins().sextend(types::I64, x),
                Ty::F32 => self.b.ins().fpromote(types::F64, x),
                _ => x,
            };
            self.b.ins().jump(join, &[v.into()]);
        } else {
            let x = self.get(reg);
            let v = match elem {
                Ty::I16 | Ty::I32 => self.b.ins().ireduce(clif_ty(elem), x),
                Ty::F32 => self.b.ins().fdemote(types::F32, x),
                _ => x,
            };
            self.mem_store(elem, v, ea, 0);
            self.b.ins().jump(join, &[]);
        }
        // slow: the embedder's full semantics
        self.b.switch_to_block(slowb);
        let h = &self.m.hosts[slow as usize];
        let addr = h.addr;
        let ctx = self.ctx;
        let ix = self.b.ins().iconst(types::I64, i64::from(index));
        let mut args = vec![ctx, ix, i];
        let mut params = vec![types::I64, types::I64, types::I64];
        if !load {
            let x = self.get(reg);
            let bits = if rt == types::F64 { self.b.ins().bitcast(types::I64, MemFlagsData::new(), x) } else { x };
            args.push(bits);
            params.push(types::I64);
        }
        let r = self.helper_call(addr, &params, load.then_some(types::I64), &args);
        let e = self.ctx_load(types::I32, OFF_ERR);
        let cont = self.b.create_block();
        self.b.ins().brif(e, self.host_err_blk, &[], cont, &[]);
        self.b.switch_to_block(cont);
        if load {
            let bits = r.expect("result");
            let v = if rt == types::F64 { self.b.ins().bitcast(types::F64, MemFlagsData::new(), bits) } else { bits };
            self.b.ins().jump(join, &[v.into()]);
        } else {
            self.b.ins().jump(join, &[]);
        }
        self.b.switch_to_block(join);
        if load {
            let v = self.b.block_params(join)[0];
            self.set(reg, v);
        }
    }

    /// Bounds-checked element address.
    fn elem_addr(&mut self, arr: Reg, idx: &[Reg], t: Ty) -> Value {
        let d = self.get(arr);
        let null = self.b.ins().icmp_imm_s(IntCC::Equal, d, 0);
        self.guard(null, qberr::SUBSCRIPT);
        let data = self.b.ins().load(types::I64, MemFlagsData::trusted(), d, OFF_DESC_DATA);
        let nd = self.b.ins().load(types::I32, MemFlagsData::trusted(), d, OFF_DESC_NDIMS);
        let dnull = self.b.ins().icmp_imm_s(IntCC::Equal, data, 0);
        let badn = self.b.ins().icmp_imm_s(IntCC::NotEqual, nd, idx.len() as i64);
        let bad = self.b.ins().bor(dnull, badn);
        self.guard(bad, qberr::SUBSCRIPT);
        let mut off: Option<Value> = None;
        for (k, r) in idx.iter().enumerate() {
            let iv = self.get(*r);
            let i64v = match self.ty(*r) {
                Ty::I16 | Ty::I32 => self.b.ins().sextend(types::I64, iv),
                _ => iv,
            };
            let base = OFF_DESC_DIMS + k as i32 * DIM_SIZE;
            let lb = self.b.ins().load(types::I64, MemFlagsData::trusted(), d, base + OFF_DIM_LB);
            let cnt = self.b.ins().load(types::I64, MemFlagsData::trusted(), d, base + OFF_DIM_COUNT);
            let rel = self.b.ins().isub(i64v, lb);
            let oob = self.b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, rel, cnt);
            self.guard(oob, qberr::SUBSCRIPT);
            let term = if k == 0 {
                rel
            } else {
                let st = self.b.ins().load(types::I64, MemFlagsData::trusted(), d, base + OFF_DIM_STRIDE);
                self.b.ins().imul(rel, st)
            };
            off = Some(match off {
                None => term,
                Some(o) => self.b.ins().iadd(o, term),
            });
        }
        let off = off.expect("at least one index");
        let shift = t.size().trailing_zeros();
        let bytes = self.b.ins().ishl_imm_s(off, i64::from(shift));
        self.b.ins().iadd(data, bytes)
    }

    fn call(&mut self, func: FuncId, args: &[Arg], dst: Option<Reg>, call_pc: u32) {
        let callee = &self.m.funcs[func as usize];
        let need = callee.slots.len() as i64;
        let top = self.ctx_load(types::I64, OFF_ARENA_TOP);
        let cap = self.ctx_load(types::I64, OFF_ARENA_CAP);
        let end = self.b.ins().iadd_imm_s(top, need);
        let over = self.b.ins().icmp(IntCC::UnsignedGreaterThan, end, cap);
        self.guard(over, qberr::OUT_OF_STACK);
        let arena = self.ctx_load(types::I64, OFF_ARENA);
        let off = self.b.ins().ishl_imm_s(top, 3);
        let cf = self.b.ins().iadd(arena, off);
        self.ctx_store(end, OFF_ARENA_TOP);
        if need <= 16 {
            let z = self.b.ins().iconst(types::I64, 0);
            for s in 0..need {
                self.b.ins().store(MemFlagsData::trusted(), z, cf, (s * 8) as i32);
            }
        } else {
            let n = self.b.ins().iconst(types::I64, need);
            self.helper_call(zero_slots as *const () as usize, &[types::I64, types::I64], None, &[cf, n]);
        }
        for (a, p) in args.iter().zip(&callee.params) {
            let pt = callee.slots[*p as usize];
            let v = match a {
                Arg::Val(r) => self.get(*r),
                Arg::Ref(r) => {
                    let fr = self.fr();
                    self.b.ins().iadd_imm_s(fr, i64::from(*r) * 8)
                }
            };
            self.slot_store(pt, v, cf, (*p * 8) as i32);
        }
        let entry = self.b.ins().iconst(types::I32, i64::from(ENTRY_CALL));
        let ctx = self.ctx;
        let c = self.b.ins().call(self.frefs[func as usize], &[ctx, cf, entry]);
        let st = self.b.inst_results(c)[0];
        let bad = self.b.create_block();
        let ok = self.b.create_block();
        self.b.set_cold_block(bad);
        self.b.ins().brif(st, bad, &[], ok, &[]);
        self.b.switch_to_block(bad);
        let fid = self.b.ins().iconst(types::I32, i64::from(func));
        let rpc = self.b.ins().iconst(types::I32, i64::from(call_pc));
        let dv = self.b.ins().iconst(types::I32, i64::from(dst.unwrap_or(NO_DST)));
        self.helper_call(
            chain_push as *const () as usize,
            &[types::I64, types::I32, types::I64, types::I32, types::I32],
            None,
            &[ctx, fid, cf, rpc, dv],
        );
        self.b.ins().jump(self.exit_blk, &[st.into()]);
        self.b.switch_to_block(ok);
        if let (Some(d), Some(r)) = (dst, callee.ret) {
            let t = callee.slots[r as usize];
            let v = self.b.ins().load(clif_ty(t), MemFlagsData::trusted(), cf, (r * 8) as i32);
            self.set(d, v);
        }
        self.ctx_store(top, OFF_ARENA_TOP);
    }
}

fn wrap_imm(t: Ty, v: i64) -> i64 {
    match t {
        Ty::I16 => i64::from(v as i16),
        Ty::I32 => i64::from(v as i32),
        _ => v,
    }
}

fn inst_def(i: &Inst) -> Option<Reg> {
    match i {
        Inst::Const { dst, .. }
        | Inst::Mov { dst, .. }
        | Inst::Conv { dst, .. }
        | Inst::Bin { dst, .. }
        | Inst::Cmp { dst, .. }
        | Inst::Neg { dst, .. }
        | Inst::Not { dst, .. }
        | Inst::GLoad { dst, .. }
        | Inst::AddrOf { dst, .. }
        | Inst::GAddrOf { dst, .. }
        | Inst::LoadPtr { dst, .. }
        | Inst::ALoad { dst, .. }
        | Inst::ConvWrap { dst, .. }
        | Inst::Trunc { dst, .. }
        | Inst::Math { dst, .. }
        | Inst::Bitcast { dst, .. } => Some(*dst),
        Inst::VAElem { load: true, reg, .. } => Some(*reg),
        Inst::Call { dst, .. } | Inst::Host { dst, .. } => *dst,
        _ => None,
    }
}

/// Collects the set of slots a function touches (for diagnostics/tests).
pub fn used_slots(f: &Func) -> BTreeSet<Reg> {
    let mut s = BTreeSet::new();
    for b in &f.blocks {
        for i in &b.insts {
            if let Some(d) = inst_def(i) {
                s.insert(d);
            }
        }
    }
    s
}
