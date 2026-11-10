//! Host-call trampolines for the reference interpreter.
//!
//! Host functions have arbitrary C signatures; calling them from Rust needs
//! one trampoline per signature: `fn(addr, ctx, args: *const u64) -> u64`.
//! They are generated with the same Cranelift ABI lowering the JIT uses, so
//! the interpreter and the JIT pass arguments identically.

use crate::codegen::{clif_ty, make_isa, Target};
use crate::ctx::Ctx;
use crate::mir::{Module, Ty};
use cranelift_codegen::ir::{types, AbiParam, InstBuilder, MemFlagsData, Signature};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, Linkage, Module as _};

type Tramp = unsafe extern "C" fn(usize, *mut Ctx, *const u64) -> u64;

pub struct Thunks {
    jit: Option<JITModule>,
    tramps: Vec<(usize, Tramp)>,
}

impl Drop for Thunks {
    fn drop(&mut self) {
        if let Some(j) = self.jit.take() {
            // SAFETY: trampolines die with `self`.
            unsafe { j.free_memory() };
        }
    }
}

impl Thunks {
    pub fn new(m: &Module) -> Thunks {
        let isa = make_isa(Target::Host);
        let cc = isa.default_call_conv();
        let frontend = isa.frontend_config();
        let mut jit = JITModule::new(JITBuilder::with_isa(isa, default_libcall_names()));
        let mut ids = Vec::new();
        let mut fctx = FunctionBuilderContext::new();
        for (i, h) in m.hosts.iter().enumerate() {
            let mut tsig = Signature::new(cc);
            for _ in 0..3 {
                tsig.params.push(AbiParam::new(types::I64));
            }
            tsig.returns.push(AbiParam::new(types::I64));
            let id = jit.declare_function(&format!("thunk{i}"), Linkage::Local, &tsig).expect("declare thunk");
            let mut c = jit.make_context();
            c.func.signature = tsig;
            {
                let mut b = FunctionBuilder::new(&mut c.func, &mut fctx);
                let e = b.create_block();
                b.append_block_params_for_function_params(e);
                b.switch_to_block(e);
                let p = b.block_params(e).to_vec();
                let mut hsig = Signature::new(cc);
                hsig.params.push(AbiParam::new(types::I64));
                let mut args = vec![p[1]];
                for (k, t) in h.sig.params.iter().enumerate() {
                    hsig.params.push(abi(*t));
                    let v = b.ins().load(clif_ty(*t), MemFlagsData::trusted(), p[2], (k * 8) as i32);
                    args.push(v);
                }
                if let Some(t) = h.sig.ret {
                    hsig.returns.push(abi(t));
                }
                let sr = b.import_signature(hsig);
                let call = b.ins().call_indirect(sr, p[0], &args);
                let out = match h.sig.ret {
                    None => b.ins().iconst(types::I64, 0),
                    Some(t) => {
                        let r = b.inst_results(call)[0];
                        match t {
                            Ty::I16 | Ty::I32 => b.ins().sextend(types::I64, r),
                            Ty::I64 | Ty::Ptr => r,
                            Ty::F64 => b.ins().bitcast(types::I64, MemFlagsData::new(), r),
                            Ty::F32 => {
                                let i = b.ins().bitcast(types::I32, MemFlagsData::new(), r);
                                b.ins().uextend(types::I64, i)
                            }
                        }
                    }
                };
                b.ins().return_(&[out]);
                b.seal_all_blocks();
                b.finalize(frontend);
            }
            jit.define_function(id, &mut c).expect("define thunk");
            jit.clear_context(&mut c);
            ids.push(id);
        }
        jit.finalize_definitions().expect("finalize thunks");
        let tramps = ids
            .iter()
            .zip(&m.hosts)
            .map(|(id, h)| {
                let p = jit.get_finalized_function(*id);
                // SAFETY: generated above with exactly this signature.
                (h.addr, unsafe { core::mem::transmute::<*const u8, Tramp>(p) })
            })
            .collect();
        Thunks { jit: Some(jit), tramps }
    }

    /// Calls host `h` with raw slot values; returns the result in slot encoding.
    ///
    /// # Safety
    /// The host function must match its declared signature.
    pub unsafe fn call(&self, h: u32, ctx: &mut Ctx, args: &[u64]) -> u64 {
        let (addr, t) = self.tramps[h as usize];
        // SAFETY: caller contract.
        unsafe { t(addr, ctx, args.as_ptr()) }
    }
}

pub(crate) fn abi(t: Ty) -> AbiParam {
    let p = AbiParam::new(clif_ty(t));
    match t {
        Ty::I16 | Ty::I32 => p.sext(),
        _ => p,
    }
}
