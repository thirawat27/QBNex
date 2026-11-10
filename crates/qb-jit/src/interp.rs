//! Reference interpreter for MIR — the oracle the JIT is tested against.
//!
//! It is deliberately simple and written straight from the semantics in
//! `mir.rs`; every rule here has a twin in `codegen.rs`. It uses the same
//! frame encoding, frame arena, GOSUB stack, deopt chain and exit protocol,
//! so the observable state after a run can be compared field by field.

use crate::ctx::{qberr, status, ArrDesc, ChainRec, Ctx, NO_DST};
use crate::mir::*;
use crate::thunk::Thunks;


/// VM callback executing one VM instruction on a frame.
pub type VmOpFn = unsafe extern "C" fn(*mut Ctx, *mut u64, u32) -> u32;

#[derive(Debug, Clone, Copy, PartialEq)]
enum V {
    I(i64),
    F32(f32),
    F64(f64),
}

impl V {
    fn i(self) -> i64 {
        match self {
            V::I(v) => v,
            _ => unreachable!("validated: integer expected"),
        }
    }
}

/// Reads a slot / memory cell of type `t` (natural size at offset 0).
///
/// # Safety
/// `p` must be valid for the type's size.
unsafe fn read(p: *const u8, t: Ty) -> V {
    // SAFETY: caller contract; unaligned reads for array elements.
    unsafe {
        match t {
            Ty::I16 => V::I(i64::from((p as *const i16).read_unaligned())),
            Ty::I32 => V::I(i64::from((p as *const i32).read_unaligned())),
            Ty::I64 | Ty::Ptr => V::I((p as *const i64).read_unaligned()),
            Ty::F32 => V::F32((p as *const f32).read_unaligned()),
            Ty::F64 => V::F64((p as *const f64).read_unaligned()),
        }
    }
}

/// Writes memory of natural size (array elements, BYREF targets).
///
/// # Safety
/// `p` must be valid for the type's size.
unsafe fn write_mem(p: *mut u8, t: Ty, v: V) {
    // SAFETY: caller contract.
    unsafe {
        match (t, v) {
            (Ty::I16, V::I(x)) => (p as *mut i16).write_unaligned(x as i16),
            (Ty::I32, V::I(x)) => (p as *mut i32).write_unaligned(x as i32),
            (Ty::I64 | Ty::Ptr, V::I(x)) => (p as *mut i64).write_unaligned(x),
            (Ty::F32, V::F32(x)) => (p as *mut f32).write_unaligned(x),
            (Ty::F64, V::F64(x)) => (p as *mut f64).write_unaligned(x),
            _ => unreachable!("validated: store type"),
        }
    }
}

/// Writes a frame/global slot: integers sign-extended to 8 bytes, SINGLE 4 bytes.
///
/// # Safety
/// `p` must point at a slot.
unsafe fn write_slot(p: *mut u64, t: Ty, v: V) {
    // SAFETY: caller contract.
    unsafe {
        match (t, v) {
            (Ty::I16, V::I(x)) => *p = (x as i16) as i64 as u64,
            (Ty::I32, V::I(x)) => *p = (x as i32) as i64 as u64,
            _ => write_mem(p as *mut u8, t, v),
        }
    }
}

/// Integer value wrapped to the width of `t`.
fn wrap(t: Ty, v: i64) -> i64 {
    match t {
        Ty::I16 => i64::from(v as i16),
        Ty::I32 => i64::from(v as i32),
        _ => v,
    }
}

fn as_f64(v: V) -> f64 {
    match v {
        V::F32(x) => f64::from(x),
        V::F64(x) => x,
        V::I(x) => x as f64,
    }
}

/// `Conv` semantics.
fn conv(from: Ty, to: Ty, v: V) -> Result<V, i32> {
    Ok(match (from.is_float(), to) {
        (false, Ty::F64) => V::F64(v.i() as f64),
        (false, Ty::F32) => V::F32(v.i() as f64 as f32),
        (false, t) => V::I(wrap(t, v.i())),
        (true, Ty::F64) => V::F64(as_f64(v)),
        (true, Ty::F32) => V::F32(as_f64(v) as f32),
        (true, Ty::I16) => {
            let r = qb_runtime::round::qb_qbr_f64(as_f64(v));
            V::I(i64::from(i16::try_from(r).map_err(|_| qberr::OVERFLOW)?))
        }
        (true, Ty::I32) => {
            let r = qb_runtime::round::qb_qbr_f64(as_f64(v));
            V::I(i64::from(i32::try_from(r).map_err(|_| qberr::OVERFLOW)?))
        }
        (true, _) => V::I(qb_runtime::round::qb_qbr_f64(as_f64(v))),
    })
}

fn conv_wrap(from: Ty, to: Ty, v: V) -> V {
    if from.is_float() && !to.is_float() {
        V::I(wrap(to, qb_runtime::round::qb_qbr_f64(as_f64(v))))
    } else {
        conv(from, to, v).expect("non-checking conversion")
    }
}

fn trunc(to: Ty, v: V) -> V {
    let x = as_f64(v);
    V::I(match to {
        Ty::I16 => i64::from(x as i16),
        Ty::I32 => i64::from(x as i32),
        _ => x as i64,
    })
}

fn math(op: MathOp, t: Ty, v: V) -> Result<V, i32> {
    Ok(match (op, v) {
        (MathOp::Abs, V::I(x)) => V::I(wrap(t, x.wrapping_abs())),
        (MathOp::Abs, V::F32(x)) => V::F32(x.abs()),
        (MathOp::Abs, V::F64(x)) => V::F64(x.abs()),
        (MathOp::Sgn, V::I(x)) => V::I(x.signum()),
        (MathOp::Sgn, f) => {
            let x = as_f64(f);
            V::I(if x > 0.0 {
                1
            } else if x < 0.0 {
                -1
            } else {
                0
            })
        }
        (MathOp::Floor, V::F32(x)) => V::F32(x.floor()),
        (MathOp::Floor, V::F64(x)) => V::F64(x.floor()),
        (MathOp::Fix, V::F32(x)) => V::F32(x.trunc()),
        (MathOp::Fix, V::F64(x)) => V::F64(x.trunc()),
        (MathOp::Sqrt, f) => {
            if as_f64(f) < 0.0 {
                return Err(qberr::ILLEGAL_CALL);
            }
            match f {
                V::F32(x) => V::F32(x.sqrt()),
                _ => V::F64(as_f64(f).sqrt()),
            }
        }
        (MathOp::RoundF32, V::F64(x)) => V::F64(f64::from(x as f32)),
        (MathOp::Wrap16, V::I(x)) => V::I(i64::from(x as i16)),
        (MathOp::Wrap32, V::I(x)) => V::I(i64::from(x as i32)),
        _ => unreachable!("validated"),
    })
}

fn bin(op: BinOp, t: Ty, a: V, b: V) -> Result<V, i32> {
    use BinOp::*;
    Ok(match (a, b) {
        (V::I(x), V::I(y)) => V::I(wrap(
            t,
            match op {
                Add => x.wrapping_add(y),
                Sub => x.wrapping_sub(y),
                Mul => x.wrapping_mul(y),
                IDiv | Mod => {
                    if y == 0 {
                        return Err(qberr::DIV_ZERO);
                    }
                    if op == IDiv {
                        x.wrapping_div(y)
                    } else {
                        x.wrapping_rem(y)
                    }
                }
                And => x & y,
                Or => x | y,
                Xor => x ^ y,
                Eqv => !(x ^ y),
                Imp => !x | y,
                Div | FDiv | Pow => unreachable!("validated"),
            },
        )),
        (V::F64(x), V::F64(y)) => V::F64(match op {
            Add => x + y,
            Sub => x - y,
            Mul => x * y,
            Div => {
                if y == 0.0 {
                    return Err(qberr::DIV_ZERO);
                }
                x / y
            }
            Pow => crate::ctx::pow_f64(x, y),
            FDiv => x / y,
            _ => unreachable!("validated"),
        }),
        (V::F32(x), V::F32(y)) => V::F32(match op {
            Add => x + y,
            Sub => x - y,
            Mul => x * y,
            Div => {
                if y == 0.0 {
                    return Err(qberr::DIV_ZERO);
                }
                x / y
            }
            Pow => crate::ctx::pow_f32(x, y),
            FDiv => x / y,
            _ => unreachable!("validated"),
        }),
        _ => unreachable!("validated"),
    })
}

fn cmp(cc: Cc, a: V, b: V) -> bool {
    match (a, b) {
        (V::I(x), V::I(y)) => match cc {
            Cc::Eq => x == y,
            Cc::Ne => x != y,
            Cc::Lt => x < y,
            Cc::Le => x <= y,
            Cc::Gt => x > y,
            Cc::Ge => x >= y,
        },
        _ => {
            let (x, y) = (as_f64(a), as_f64(b));
            match cc {
                Cc::Eq => x == y,
                Cc::Ne => x != y,
                Cc::Lt => x < y,
                Cc::Le => x <= y,
                Cc::Gt => x > y,
                Cc::Ge => x >= y,
            }
        }
    }
}

/// Runs `func` of `m` on `frame` (slots), entered at `entry` (`ENTRY_CALL` or a block pc).
///
/// # Safety
/// `frame` must hold `m.funcs[func].slots.len()` slots and `ctx.globals` the
/// module's globals; host functions must match their signatures.
pub unsafe fn run(m: &Module, thunks: &Thunks, ctx: &mut Ctx, func: FuncId, frame: *mut u64, entry: u32) -> u32 {
    let f = &m.funcs[func as usize];
    let (mut bi, gosub_base, mut pc) = if entry == ENTRY_CALL {
        (0usize, ctx.gosub_len, 0u32)
    } else {
        match f.blocks.iter().position(|b| b.pc == Some(entry)) {
            Some(b) => (b, ctx.gosub_base, entry),
            None => {
                ctx.exit_pc = entry;
                return status::DEOPT;
            }
        }
    };
    let mut line = 0u32;
    let ty = |r: Reg| f.slots[r as usize];
    // SAFETY (all slot accesses below): validated slot indices into `frame`.
    let nf = f.frame_len();
    let mut temps = vec![0u64; f.slots.len() - nf];
    let tp = temps.as_mut_ptr();
    let slot = |r: Reg| unsafe {
        if (r as usize) < nf {
            frame.add(r as usize)
        } else {
            tp.add(r as usize - nf)
        }
    };
    let get = |r: Reg| unsafe { read(slot(r) as *const u8, ty(r)) };
    let set = |r: Reg, v: V| unsafe { write_slot(slot(r), ty(r), v) };
    let globals = ctx.globals;
    let gslot = move |s: u32| unsafe { globals.add(s as usize) };
    macro_rules! fail {
        ($code:expr) => {{
            ctx.err = $code;
            ctx.err_line = line;
            ctx.exit_pc = pc;
            return status::ERROR;
        }};
    }
    macro_rules! tryq {
        ($e:expr) => {
            match $e {
                Ok(v) => v,
                Err(c) => fail!(c),
            }
        };
    }
    loop {
        let b = &f.blocks[bi];
        for inst in &b.insts {
            match inst {
                Inst::Line { line: l, pc: p } => {
                    line = *l;
                    pc = *p;
                    if ctx.event_pending() {
                        ctx.exit_pc = pc;
                        return status::EVENT;
                    }
                }
                Inst::Const { dst, imm } => set(
                    *dst,
                    match (*imm, ty(*dst)) {
                        (Imm::I(v), Ty::F64) => V::F64(v as f64),
                        (Imm::I(v), Ty::F32) => V::F32(v as f32),
                        (Imm::I(v), t) => V::I(wrap(t, v)),
                        (Imm::F(v), Ty::F32) => V::F32(v as f32),
                        (Imm::F(v), _) => V::F64(v),
                    },
                ),
                Inst::Mov { dst, src } => set(*dst, get(*src)),
                Inst::Conv { dst, src } => {
                    let v = tryq!(conv(ty(*src), ty(*dst), get(*src)));
                    set(*dst, v)
                }
                Inst::Bin { op, dst, a, b } => {
                    let v = tryq!(bin(*op, ty(*dst), get(*a), get(*b)));
                    set(*dst, v)
                }
                Inst::Cmp { cc, dst, a, b } => set(*dst, V::I(if cmp(*cc, get(*a), get(*b)) { -1 } else { 0 })),
                Inst::Neg { dst, src } => set(
                    *dst,
                    match get(*src) {
                        V::I(x) => V::I(wrap(ty(*dst), x.wrapping_neg())),
                        V::F32(x) => V::F32(-x),
                        V::F64(x) => V::F64(-x),
                    },
                ),
                Inst::Not { dst, src } => set(*dst, V::I(wrap(ty(*dst), !get(*src).i()))),
                Inst::GLoad { dst, slot: s } => set(*dst, unsafe { read(gslot(*s) as *const u8, ty(*dst)) }),
                Inst::GStore { slot: s, src } => unsafe { write_slot(gslot(*s), ty(*src), get(*src)) },
                Inst::AddrOf { dst, slot: s } => set(*dst, V::I(slot(*s) as i64)),
                Inst::GAddrOf { dst, slot: s } => set(*dst, V::I(gslot(*s) as i64)),
                Inst::LoadPtr { dst, ptr } => {
                    let p = get(*ptr).i() as *const u8;
                    set(*dst, unsafe { read(p, ty(*dst)) })
                }
                Inst::StorePtr { ptr, src } => {
                    let p = get(*ptr).i() as *mut u8;
                    unsafe { write_mem(p, ty(*src), get(*src)) }
                }
                Inst::ALoad { dst, arr, idx } | Inst::AStore { arr, idx, src: dst } => {
                    let d = get(*arr).i() as *const ArrDesc;
                    let ix: Vec<i64> = idx.iter().map(|r| get(*r).i()).collect();
                    let off = if d.is_null() {
                        Err(qberr::SUBSCRIPT)
                    } else {
                        // SAFETY: a non-null array slot holds a live descriptor.
                        unsafe { (*d).offset(&ix) }
                    };
                    let off = tryq!(off);
                    let t = ty(*dst);
                    // SAFETY: offset checked against the descriptor bounds.
                    let p = unsafe { (*d).data.add(off * t.size() as usize) };
                    if matches!(inst, Inst::ALoad { .. }) {
                        set(*dst, unsafe { read(p, t) })
                    } else {
                        unsafe { write_mem(p, t, get(*dst)) }
                    }
                }
                Inst::Call { func: cf, args, dst, pc: call_pc } => {
                    let callee = &m.funcs[*cf as usize];
                    let need = callee.slots.len() as u64;
                    let top = ctx.arena_top;
                    if top + need > ctx.arena_cap {
                        fail!(qberr::OUT_OF_STACK);
                    }
                    // SAFETY: within the arena (checked above).
                    let cfr = unsafe { ctx.arena.add(top as usize) };
                    ctx.arena_top = top + need;
                    unsafe { core::ptr::write_bytes(cfr, 0, need as usize) };
                    for (a, p) in args.iter().zip(&callee.params) {
                        let pt = callee.slots[*p as usize];
                        let v = match a {
                            Arg::Val(r) => get(*r),
                            Arg::Ref(r) => V::I(slot(*r) as i64),
                        };
                        unsafe { write_slot(cfr.add(*p as usize), pt, v) };
                    }
                    let st = unsafe { run(m, thunks, ctx, *cf, cfr, ENTRY_CALL) };
                    if st != status::OK {
                        ctx.chain.push(ChainRec {
                            func: *cf,
                            frame: cfr,
                            ret_pc: *call_pc,
                            dst: dst.unwrap_or(NO_DST),
                        });
                        return st;
                    }
                    if let (Some(d), Some(r)) = (dst, callee.ret) {
                        let v = unsafe { read(cfr.add(r as usize) as *const u8, callee.slots[r as usize]) };
                        set(*d, v);
                    }
                    ctx.arena_top = top;
                }
                Inst::Host { f: h, args, dst } => {
                    let hf = &m.hosts[*h as usize];
                    let raw: Vec<u64> = args.iter().map(|r| unsafe { *slot(*r) }).collect();
                    let ret = unsafe { thunks.call(*h, ctx, &raw) };
                    if let (Some(d), Some(t)) = (dst, hf.sig.ret) {
                        let v = unsafe { read(&ret as *const u64 as *const u8, t) };
                        set(*d, v);
                    }
                    if hf.sig.may_error && ctx.err != 0 {
                        ctx.err_line = line;
                        ctx.exit_pc = pc;
                        return status::ERROR;
                    }
                }
                Inst::Raise { code } => fail!(get(*code).i() as i32),
                Inst::VAElem { load, reg, table, index, view_off, elem, idx, slow } => {
                    let i = get(*idx).i();
                    // SAFETY: the embedder keeps the table and the views valid while code runs.
                    let fast = unsafe {
                        let p = *ctx.tables[*table as usize].add(*index as usize);
                        if p == 0 {
                            None
                        } else {
                            let v = *((p as usize + *view_off as usize) as *const crate::ctx::ArrView);
                            let k = i.wrapping_sub(v.lbound);
                            (!v.data.is_null() && (k as u64) < (v.count as u64))
                                .then(|| v.data.add(k as usize * elem.size() as usize))
                        }
                    };
                    match fast {
                        Some(ep) => {
                            if *load {
                                let x = unsafe { read(ep, *elem) };
                                set(*reg, if elem.is_float() { V::F64(as_f64(x)) } else { x })
                            } else {
                                let x = get(*reg);
                                let x = match (*elem, x) {
                                    (Ty::F32, V::F64(f)) => V::F32(f as f32),
                                    (_, v) => v,
                                };
                                unsafe { write_mem(ep, *elem, x) }
                            }
                        }
                        None => {
                            let bits = unsafe { *slot(*reg) };
                            let args = if *load { vec![u64::from(*index), i as u64] } else { vec![u64::from(*index), i as u64, bits] };
                            let ret = unsafe { thunks.call(*slow, ctx, &args) };
                            if *load {
                                unsafe { *slot(*reg) = ret };
                            }
                            if ctx.err != 0 {
                                ctx.err_line = line;
                                ctx.exit_pc = pc;
                                return status::ERROR;
                            }
                        }
                    }
                }
                Inst::ConvWrap { dst, src } => set(*dst, conv_wrap(ty(*src), ty(*dst), get(*src))),
                Inst::At { pc: p } => pc = *p,
                Inst::Bitcast { dst, src } => set(
                    *dst,
                    match get(*src) {
                        V::I(x) => V::F64(f64::from_bits(x as u64)),
                        V::F64(x) => V::I(x.to_bits() as i64),
                        _ => unreachable!("validated"),
                    },
                ),
                Inst::Trunc { dst, src } => set(*dst, trunc(ty(*dst), get(*src))),
                Inst::Math { op, dst, src } => {
                    let v = tryq!(math(*op, ty(*dst), get(*src)));
                    set(*dst, v)
                }
                Inst::VmOp { op, .. } => {
                    // SAFETY: validated non-zero callback with the documented signature.
                    let cb: VmOpFn = unsafe { core::mem::transmute::<usize, VmOpFn>(m.vm_op) };
                    let st = unsafe { cb(ctx, frame, *op) };
                    if st != status::OK {
                        return st;
                    }
                }
            }
        }
        let poll = |ctx: &Ctx, t: BlockId| -> Option<u32> {
            let tb = &f.blocks[t as usize];
            match tb.pc {
                Some(p) if (t as usize) <= bi && ctx.event_pending() => Some(p),
                _ => None,
            }
        };
        let next = match &b.term {
            Term::Jmp(t) => *t,
            Term::Br { cond, then, els } => {
                if get(*cond).i() != 0 {
                    *then
                } else {
                    *els
                }
            }
            Term::Ret => return status::OK,
            Term::Exit { pc: p } => {
                ctx.exit_pc = *p;
                return status::DEOPT;
            }
            Term::End => {
                ctx.exit_pc = 0;
                return status::END;
            }
            Term::Gosub { target, ret_pc, .. } => {
                if ctx.gosub_len >= ctx.gosub_cap {
                    fail!(qberr::OUT_OF_MEMORY);
                }
                // SAFETY: below capacity.
                unsafe { *ctx.gosub.add(ctx.gosub_len as usize) = *ret_pc };
                ctx.gosub_len += 1;
                *target
            }
            Term::Return { pc: rpc } => {
                if ctx.gosub_len <= gosub_base {
                    fail!(qberr::RETURN_WITHOUT_GOSUB);
                }
                // SAFETY: len > 0.
                let to = unsafe { *ctx.gosub.add(ctx.gosub_len as usize - 1) };
                let site = f.blocks.iter().find_map(|b| match b.term {
                    Term::Gosub { ret, ret_pc, .. } if ret_pc == to => Some(ret),
                    _ => None,
                });
                match site {
                    Some(r) => {
                        ctx.gosub_len -= 1;
                        r
                    }
                    None => {
                        ctx.exit_pc = *rpc;
                        return status::DEOPT;
                    }
                }
            }
        };
        if let Some(p) = poll(ctx, next) {
            ctx.exit_pc = p;
            return status::EVENT;
        }
        bi = next as usize;
    }
}
