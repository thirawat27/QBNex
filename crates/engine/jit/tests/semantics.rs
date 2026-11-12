//! Focused semantics tests: each runs in the interpreter and the JIT
//! (outcomes must be identical) and checks the expected QB result.

mod common;
use common::*;
use qb_jit::ctx::status;
use qb_jit::mir::*;
use qb_jit::rt::{self, id};

/// `FOR i& = 1 TO n: s& = s& + i& MOD 7: NEXT`
fn loop_mod(n: i64) -> Module {
    let mut f = Fb::new("main");
    let (i, s, lim, one, seven, t, c) =
        (f.slot(Ty::I32), f.slot(Ty::I32), f.slot(Ty::I32), f.slot(Ty::I32), f.slot(Ty::I32), f.slot(Ty::I32), f.slot(Ty::I16));
    let head = f.block(Some(10));
    let body = f.block(None);
    let exit = f.block(None);
    f.at(0);
    f.line(1, 1);
    f.ci(i, 1);
    f.ci(lim, n);
    f.ci(one, 1);
    f.ci(seven, 7);
    f.t(Term::Jmp(head));
    f.at(head);
    f.i(Inst::Cmp { cc: Cc::Le, dst: c, a: i, b: lim });
    f.t(Term::Br { cond: c, then: body, els: exit });
    f.at(body);
    f.line(2, 20);
    f.bin(BinOp::Mod, t, i, seven);
    f.bin(BinOp::Add, s, s, t);
    f.bin(BinOp::Add, i, i, one);
    f.t(Term::Jmp(head));
    f.at(exit);
    f.t(Term::End);
    module(vec![f.f], vec![])
}

#[test]
fn for_loop_sum() {
    let o = both(&loop_mod(1000), 0, &Setup::default());
    assert_eq!(o.status, status::END);
    let expect: i64 = (1..=1000).map(|i| i % 7).sum();
    assert_eq!(o.frame[1] as i64, expect);
}

#[test]
fn integer_wraps_like_c() {
    let mut f = Fb::new("w");
    let (a, b, r, x, y, z) = (f.slot(Ty::I16), f.slot(Ty::I16), f.slot(Ty::I16), f.slot(Ty::I32), f.slot(Ty::I32), f.slot(Ty::I32));
    f.ci(a, 32767);
    f.ci(b, 1);
    f.bin(BinOp::Add, r, a, b);
    f.ci(x, i32::MAX as i64);
    f.ci(y, 2);
    f.bin(BinOp::Mul, z, x, y);
    let o = both(&module(vec![f.f], vec![]), 0, &Setup::default());
    assert_eq!(o.frame[2] as i64, -32768);
    assert_eq!(o.frame[5] as i64, -2);
}

#[test]
fn int64_min_div_minus_one_wraps() {
    let mut f = Fb::new("d");
    let (a, b, q, r) = (f.slot(Ty::I64), f.slot(Ty::I64), f.slot(Ty::I64), f.slot(Ty::I64));
    f.ci(a, i64::MIN);
    f.ci(b, -1);
    f.bin(BinOp::IDiv, q, a, b);
    f.bin(BinOp::Mod, r, a, b);
    let o = both(&module(vec![f.f], vec![]), 0, &Setup::default());
    assert_eq!(o.frame[2] as i64, i64::MIN);
    assert_eq!(o.frame[3], 0);
}

#[test]
fn float_to_int_rounds_half_even() {
    let mut f = Fb::new("r");
    let src = f.slot(Ty::F64);
    let mut outs = vec![];
    for (k, v) in [2.5, 3.5, -2.5, -0.5, 1.4999999999999998, 32767.4].iter().enumerate() {
        let d = f.slot(Ty::I16);
        f.line(k as u32 + 1, k as u32 + 1);
        f.cf(src, *v);
        f.i(Inst::Conv { dst: d, src });
        outs.push(d);
    }
    let big = f.slot(Ty::I64);
    f.cf(src, 1e18 + 0.5);
    f.i(Inst::Conv { dst: big, src });
    let huge = f.slot(Ty::I64);
    f.cf(src, 1e30);
    f.i(Inst::Conv { dst: huge, src });
    let s32 = f.slot(Ty::F32);
    f.cf(src, 2.5);
    f.i(Inst::Conv { dst: s32, src });
    let from32 = f.slot(Ty::I32);
    f.i(Inst::Conv { dst: from32, src: s32 });
    let o = both(&module(vec![f.f], vec![]), 0, &Setup::default());
    let got: Vec<i64> = outs.iter().map(|r| o.frame[*r as usize] as i64).collect();
    assert_eq!(got, vec![2, 4, -2, 0, 1, 32767]);
    assert_eq!(o.frame[big as usize] as i64, qb_runtime::round::qb_qbr_f64(1e18 + 0.5));
    assert_eq!(o.frame[huge as usize] as i64, qb_runtime::round::qb_qbr_f64(1e30));
    assert_eq!(o.frame[from32 as usize] as i64, 2);
}

#[test]
fn overflow_on_store_raises_6_with_line_and_pc() {
    let mut f = Fb::new("o");
    let (src, d) = (f.slot(Ty::F64), f.slot(Ty::I16));
    f.line(7, 70);
    f.cf(src, 32767.5);
    f.i(Inst::Conv { dst: d, src });
    let o = both(&module(vec![f.f], vec![]), 0, &Setup::default());
    assert_eq!((o.status, o.err, o.err_line, o.exit_pc), (status::ERROR, 6, 7, 70));
    // the frame was written back: src holds 32767.5
    assert_eq!(f64s(o.frame[0]), 32767.5);
}

#[test]
fn nan_to_int_overflows() {
    let mut f = Fb::new("n");
    let (z, q, d) = (f.slot(Ty::F64), f.slot(Ty::F64), f.slot(Ty::I32));
    f.cf(z, f64::NAN);
    f.i(Inst::Mov { dst: q, src: z });
    f.i(Inst::Conv { dst: d, src: q });
    let o = both(&module(vec![f.f], vec![]), 0, &Setup::default());
    assert_eq!((o.status, o.err), (status::ERROR, 6));
}

#[test]
fn division_by_zero_is_error_11() {
    for float in [false, true] {
        let mut f = Fb::new("z");
        let t = if float { Ty::F64 } else { Ty::I32 };
        let (a, b, r) = (f.slot(t), f.slot(t), f.slot(t));
        f.line(3, 30);
        f.ci(a, 5);
        f.ci(b, 0);
        f.bin(if float { BinOp::Div } else { BinOp::IDiv }, r, a, b);
        let o = both(&module(vec![f.f], vec![]), 0, &Setup::default());
        assert_eq!((o.status, o.err, o.err_line, o.exit_pc), (status::ERROR, 11, 3, 30));
    }
}

#[test]
fn single_arithmetic_and_pow() {
    let mut f = Fb::new("s");
    let (a, b, r, p, d, e) = (f.slot(Ty::F32), f.slot(Ty::F32), f.slot(Ty::F32), f.slot(Ty::F64), f.slot(Ty::F64), f.slot(Ty::F64));
    f.cf(a, 0.1);
    f.cf(b, 0.2);
    f.bin(BinOp::Add, r, a, b);
    f.cf(d, 2.0);
    f.cf(e, 0.5);
    f.bin(BinOp::Pow, p, d, e);
    let o = both(&module(vec![f.f], vec![]), 0, &Setup::default());
    assert_eq!(f32s(o.frame[2]), 0.1f32 + 0.2f32);
    assert_eq!(f64s(o.frame[3]), 2f64.sqrt());
}

#[test]
fn compare_gives_minus_one_and_logic_ops() {
    let mut f = Fb::new("c");
    let (a, b) = (f.slot(Ty::I32), f.slot(Ty::I32));
    let outs: Vec<Reg> = (0..6).map(|_| f.slot(Ty::I16)).collect();
    let (x, y) = (f.slot(Ty::F64), f.slot(Ty::F64));
    let fne = f.slot(Ty::I16);
    let (eqv, imp, not) = (f.slot(Ty::I32), f.slot(Ty::I32), f.slot(Ty::I32));
    f.ci(a, 3);
    f.ci(b, 5);
    for (k, cc) in [Cc::Eq, Cc::Ne, Cc::Lt, Cc::Le, Cc::Gt, Cc::Ge].into_iter().enumerate() {
        f.i(Inst::Cmp { cc, dst: outs[k], a, b });
    }
    f.cf(x, f64::NAN);
    f.cf(y, f64::NAN);
    f.i(Inst::Cmp { cc: Cc::Ne, dst: fne, a: x, b: y });
    f.bin(BinOp::Eqv, eqv, a, b);
    f.bin(BinOp::Imp, imp, a, b);
    f.i(Inst::Not { dst: not, src: a });
    let o = both(&module(vec![f.f], vec![]), 0, &Setup::default());
    let got: Vec<i64> = outs.iter().map(|r| o.frame[*r as usize] as i64).collect();
    assert_eq!(got, vec![0, -1, -1, -1, 0, 0]);
    assert_eq!(o.frame[fne as usize] as i64, -1);
    assert_eq!(o.frame[eqv as usize] as i64, !(3 ^ 5));
    assert_eq!(o.frame[imp as usize] as i64, !3 | 5);
    assert_eq!(o.frame[not as usize] as i64, -4);
}

/// main: GOSUB sub twice, counter in a global; sub: g = g + 1: RETURN
fn gosub_mod(bad_return: bool) -> Module {
    let mut f = Fb::new("main");
    let (g, one) = (f.slot(Ty::I32), f.slot(Ty::I32));
    let r1 = f.block(Some(11));
    let r2 = f.block(Some(12));
    let sub = f.block(Some(50));
    f.at(0);
    f.line(1, 1);
    f.ci(one, 1);
    if bad_return {
        f.t(Term::Jmp(sub));
    } else {
        f.t(Term::Gosub { target: sub, ret: r1, ret_pc: 11 });
    }
    f.at(r1);
    f.t(Term::Gosub { target: sub, ret: r2, ret_pc: 12 });
    f.at(r2);
    f.t(Term::End);
    f.at(sub);
    f.line(5, 50);
    f.i(Inst::GLoad { dst: g, slot: 0 });
    f.bin(BinOp::Add, g, g, one);
    f.i(Inst::GStore { slot: 0, src: g });
    f.t(Term::Return { pc: 51 });
    module(vec![f.f], vec![Ty::I32])
}

#[test]
fn gosub_return() {
    let o = both(&gosub_mod(false), 0, &Setup::default());
    assert_eq!(o.status, status::END);
    assert_eq!(o.globals[0], 2);
    assert!(o.gosub.is_empty());
}

#[test]
fn return_without_gosub_is_error_3() {
    let o = both(&gosub_mod(true), 0, &Setup::default());
    assert_eq!((o.status, o.err, o.err_line, o.exit_pc), (status::ERROR, 3, 5, 50));
}

#[test]
fn gosub_with_a_full_native_stack_deopts_at_the_gosub() {
    // capacity 64 (common::run_full), all used: the VM makes the GOSUB (no error 7)
    let full = [999u32; 64];
    let o = both(&gosub_mod(false), 0, &Setup { gosub: &full, gosub_base: 64, ..Setup::default() });
    assert_eq!((o.status, o.err, o.exit_pc), (status::DEOPT, 0, 10));
    assert_eq!(o.gosub.len(), 64);
    assert_eq!(o.globals[0], 0);
}

#[test]
fn return_to_vm_only_site_deopts() {
    // The VM pushed return pc 999 before entering the subroutine block natively.
    let o = both(&gosub_mod(false), 0, &Setup { frame: &[0, 1], entry: 50, gosub: &[999], gosub_base: 0, ..Setup::default() });
    assert_eq!((o.status, o.exit_pc), (status::DEOPT, 51));
    assert_eq!(o.gosub, vec![999]);
    assert_eq!(o.globals[0], 1);
}

#[test]
fn event_at_line_exits_and_osr_resumes() {
    let m = loop_mod(100);
    let o = both(&m, 0, &Setup { event: 1, ..Setup::default() });
    assert_eq!((o.status, o.exit_pc), (status::EVENT, 1));
    // Resume at the loop header with a half-done frame: same result as a full run.
    let full = both(&m, 0, &Setup::default());
    let mut part = vec![0u64; 7];
    part[0] = 51; // i
    part[1] = (1..=50).map(|i| i % 7).sum::<i64>() as u64;
    part[2] = 100;
    part[3] = 1;
    part[4] = 7;
    let o = both(&m, 0, &Setup { frame: &part, entry: 10, ..Setup::default() });
    assert_eq!(o.status, status::END);
    assert_eq!(o.frame[1], full.frame[1]);
}

#[test]
fn event_at_back_edge() {
    // Loop whose body has no LINE: the back-edge to the resumable header polls.
    let mut f = Fb::new("spin");
    let (i, one) = (f.slot(Ty::I32), f.slot(Ty::I32));
    let head = f.block(Some(10));
    f.at(0);
    f.ci(one, 1);
    f.t(Term::Jmp(head));
    f.at(head);
    f.bin(BinOp::Add, i, i, one);
    f.t(Term::Jmp(head));
    let o = both(&module(vec![f.f], vec![]), 0, &Setup { event: 1, ..Setup::default() });
    assert_eq!((o.status, o.exit_pc), (status::EVENT, 10));
    assert_eq!(o.frame[0], 1);
}

#[test]
fn unknown_osr_entry_deopts() {
    let o = both(&loop_mod(3), 0, &Setup { entry: 12345, ..Setup::default() });
    assert_eq!((o.status, o.exit_pc), (status::DEOPT, 12345));
}

/// FUNCTION fact&(n&): IF n& <= 1 THEN fact& = 1 ELSE fact& = n& * fact&(n& - 1)
fn fact_func() -> Func {
    let mut f = Fb::new("fact");
    let n = f.param(Ty::I32);
    let (r, one, c, m1, sub) = (f.slot(Ty::I32), f.slot(Ty::I32), f.slot(Ty::I16), f.slot(Ty::I32), f.slot(Ty::I32));
    f.f.ret = Some(r);
    let base = f.block(None);
    let rec = f.block(None);
    f.at(0);
    f.line(20, 200);
    f.ci(one, 1);
    f.i(Inst::Cmp { cc: Cc::Le, dst: c, a: n, b: one });
    f.t(Term::Br { cond: c, then: base, els: rec });
    f.at(base);
    f.i(Inst::Mov { dst: r, src: one });
    f.t(Term::Ret);
    f.at(rec);
    f.bin(BinOp::Sub, m1, n, one);
    f.i(Inst::Call { func: 1, args: vec![Arg::Val(m1)], dst: Some(sub), pc: 201, back: vec![] });
    f.bin(BinOp::Mul, r, n, sub);
    f.t(Term::Ret);
    f.f
}

#[test]
fn recursive_function_call() {
    let mut f = Fb::new("main");
    let (n, r) = (f.slot(Ty::I32), f.slot(Ty::I32));
    f.ci(n, 10);
    f.i(Inst::Call { func: 1, args: vec![Arg::Val(n)], dst: Some(r), pc: 5, back: vec![] });
    let o = both(&module(vec![f.f, fact_func()], vec![]), 0, &Setup::default());
    assert_eq!(o.status, status::END);
    assert_eq!(o.frame[1], 3628800);
    assert!(o.chain.is_empty());
}

#[test]
fn error_in_callee_builds_deopt_chain() {
    // fact(13) overflows LONG? No: LONG multiply wraps. Force an error: n& \ 0 at depth 3.
    let mut g = Fb::new("boom");
    let n = g.param(Ty::I32);
    let (one, z, q, c, m1) = (g.slot(Ty::I32), g.slot(Ty::I32), g.slot(Ty::I32), g.slot(Ty::I16), g.slot(Ty::I32));
    let rec = g.block(None);
    let die = g.block(None);
    g.at(0);
    g.line(30, 300);
    g.ci(one, 1);
    g.i(Inst::Cmp { cc: Cc::Le, dst: c, a: n, b: one });
    g.t(Term::Br { cond: c, then: die, els: rec });
    g.at(rec);
    g.bin(BinOp::Sub, m1, n, one);
    g.i(Inst::Call { func: 1, args: vec![Arg::Val(m1)], dst: None, pc: 301, back: vec![] });
    g.t(Term::Ret);
    g.at(die);
    g.line(31, 310);
    g.bin(BinOp::IDiv, q, n, z);
    g.t(Term::Ret);
    let mut f = Fb::new("main");
    let n0 = f.slot(Ty::I32);
    f.ci(n0, 3);
    f.i(Inst::Call { func: 1, args: vec![Arg::Val(n0)], dst: None, pc: 9, back: vec![] });
    let o = both(&module(vec![f.f, g.f], vec![]), 0, &Setup::default());
    assert_eq!((o.status, o.err, o.err_line, o.exit_pc), (status::ERROR, 11, 31, 310));
    // innermost first: boom(1) called from boom(2) at 301, boom(2) from boom(3) at 301, boom(3) from main at 9
    let rp: Vec<(u32, u32, u64)> = o.chain.iter().map(|c| (c.0, c.1, c.3[0])).collect();
    assert_eq!(rp, vec![(1, 301, 1), (1, 301, 2), (1, 9, 3)]);
}

#[test]
fn byref_parameter() {
    // SUB inc(x&): x& = x& + 1
    let mut g = Fb::new("inc");
    let p = g.param(Ty::Ptr);
    let (v, one) = (g.slot(Ty::I32), g.slot(Ty::I32));
    g.i(Inst::LoadPtr { dst: v, ptr: p });
    g.ci(one, 1);
    g.bin(BinOp::Add, v, v, one);
    g.i(Inst::StorePtr { ptr: p, src: v });
    g.t(Term::Ret);
    let mut f = Fb::new("main");
    let x = f.slot(Ty::I32);
    f.ci(x, 41);
    f.i(Inst::Call { func: 1, args: vec![Arg::Ref(x)], dst: None, pc: 1, back: vec![] });
    f.i(Inst::Call { func: 1, args: vec![Arg::Ref(x)], dst: None, pc: 2, back: vec![] });
    let o = both(&module(vec![f.f, g.f], vec![]), 0, &Setup::default());
    assert_eq!(o.frame[0], 43);
}

#[test]
fn arrays_two_dims_and_bounds() {
    let mut f = Fb::new("arr");
    let (a, el, lb, ub, i, j, v, lim, one, c) = (
        f.slot(Ty::Ptr),
        f.slot(Ty::I32),
        f.slot(Ty::I64),
        f.slot(Ty::I64),
        f.slot(Ty::I16),
        f.slot(Ty::I16),
        f.slot(Ty::F64),
        f.slot(Ty::I16),
        f.slot(Ty::I16),
        f.slot(Ty::I16),
    );
    let sum = f.slot(Ty::F64);
    let head = f.block(None);
    let body = f.block(None);
    let after = f.block(None);
    f.at(0);
    f.line(1, 1);
    f.ci(el, 8);
    f.ci(lb, 1);
    f.ci(ub, 5);
    f.i(Inst::Host { f: id::ARR_NEW2, args: vec![el, lb, ub, lb, ub], dst: Some(a) });
    f.ci(i, 1);
    f.ci(lim, 5);
    f.ci(one, 1);
    f.t(Term::Jmp(head));
    f.at(head);
    f.i(Inst::Cmp { cc: Cc::Le, dst: c, a: i, b: lim });
    f.t(Term::Br { cond: c, then: body, els: after });
    f.at(body);
    f.i(Inst::Conv { dst: v, src: i });
    f.i(Inst::AStore { arr: a, idx: vec![i, i], src: v });
    f.i(Inst::ALoad { dst: v, arr: a, idx: vec![i, i] });
    f.bin(BinOp::Add, sum, sum, v);
    f.bin(BinOp::Add, i, i, one);
    f.t(Term::Jmp(head));
    f.at(after);
    f.line(2, 2);
    f.ci(j, 6);
    f.i(Inst::ALoad { dst: v, arr: a, idx: vec![one, j] });
    let o = both(&module(vec![f.f], vec![]), 0, &Setup::default());
    assert_eq!((o.status, o.err, o.err_line), (status::ERROR, 9, 2));
    assert_eq!(f64s(o.frame[sum as usize]), 15.0);
}

#[test]
fn unallocated_array_is_error_9() {
    let mut f = Fb::new("u");
    let (a, i, v) = (f.slot(Ty::Ptr), f.slot(Ty::I32), f.slot(Ty::I16));
    f.i(Inst::ALoad { dst: v, arr: a, idx: vec![i] });
    let o = both(&module(vec![f.f], vec![]), 0, &Setup::default());
    assert_eq!((o.status, o.err), (status::ERROR, 9));
}

#[test]
fn strings_through_host_calls() {
    static HELLO: &[u8] = b"Hello, ";
    static WORLD: &[u8] = b"world";
    let mut f = Fb::new("s");
    let (p, n, a, b, c, num, s, l) =
        (f.slot(Ty::Ptr), f.slot(Ty::I64), f.slot(Ty::Ptr), f.slot(Ty::Ptr), f.slot(Ty::Ptr), f.slot(Ty::F32), f.slot(Ty::Ptr), f.slot(Ty::I32));
    f.ci(p, HELLO.as_ptr() as i64);
    f.ci(n, HELLO.len() as i64);
    f.i(Inst::Host { f: id::STR_LIT, args: vec![p, n], dst: Some(a) });
    f.ci(p, WORLD.as_ptr() as i64);
    f.ci(n, WORLD.len() as i64);
    f.i(Inst::Host { f: id::STR_LIT, args: vec![p, n], dst: Some(b) });
    f.i(Inst::Host { f: id::STR_CONCAT, args: vec![a, b], dst: Some(c) });
    f.i(Inst::Host { f: id::PRINT_STR, args: vec![c], dst: None });
    f.cf(num, 1.0 / 3.0);
    f.i(Inst::Host { f: id::STR_F32, args: vec![num], dst: Some(s) });
    f.i(Inst::Host { f: id::PRINT_STR, args: vec![s], dst: None });
    f.i(Inst::Host { f: id::PRINT_NL, args: vec![], dst: None });
    f.i(Inst::Host { f: id::STR_LEN, args: vec![c], dst: Some(l) });
    for r in [a, b, c, s] {
        f.i(Inst::Host { f: id::STR_FREE, args: vec![r], dst: None });
    }
    f.line(4, 40);
    f.ci(l, 300);
    f.i(Inst::Host { f: id::STR_CHR, args: vec![l], dst: Some(a) });
    let o = both(&module(vec![f.f], vec![]), 0, &Setup::default());
    assert_eq!(o.out, "Hello, world .3333333\n");
    assert_eq!((o.status, o.err, o.err_line, o.exit_pc), (status::ERROR, 5, 4, 40));
    let _ = rt::bytes(0);
}

#[test]
fn raise_error_n() {
    let mut f = Fb::new("e");
    let c = f.slot(Ty::I16);
    f.line(9, 90);
    f.ci(c, 53);
    f.i(Inst::Raise { code: c });
    let o = both(&module(vec![f.f], vec![]), 0, &Setup::default());
    assert_eq!((o.status, o.err, o.err_line, o.exit_pc), (status::ERROR, 53, 9, 90));
}

#[test]
fn cross_targets_compile() {
    let m = module(vec![loop_mod(10).funcs.remove(0), fact_func()], vec![]);
    for t in [qb_jit::Target::X86_64, qb_jit::Target::Aarch64] {
        qb_jit::compile(&m, t).unwrap_or_else(|e| panic!("{t:?}: {e:?}"));
    }
}

#[test]
fn temporaries_beyond_frame_len_are_never_written_back() {
    let mut f = Fb::new("t");
    let (a, t) = (f.slot(Ty::I64), f.slot(Ty::I64));
    f.f.frame_len = Some(1);
    f.ci(t, 5);
    f.i(Inst::Mov { dst: a, src: t });
    let o = both(&module(vec![f.f], vec![]), 0, &Setup::default());
    assert_eq!(o.frame, vec![5, 0]);
}

/// Embedder-owned array for `VAElem`: a header with an `ArrView` at offset 8.
#[repr(C)]
struct FakeArr {
    tag: u64,
    view: qb_jit::ctx::ArrView,
}

extern "C" fn slow_load(ctx: *mut qb_jit::Ctx, index: i64, i: i64) -> i64 {
    // "auto-DIM" semantics: index 99 is error 9, otherwise 1000 * slot + i
    if i == 99 {
        unsafe { (*ctx).err = 9 };
        return 0;
    }
    1000 * index + i
}

extern "C" fn slow_store(ctx: *mut qb_jit::Ctx, _index: i64, i: i64, _bits: i64) {
    if i == 99 {
        unsafe { (*ctx).err = 9 };
    }
}

#[test]
fn embedder_array_elements_fast_and_slow_paths() {
    let data = vec![0i16; 4];
    let mut data = std::mem::ManuallyDrop::new(data);
    let mut fdata = vec![0f32; 4];
    let a = Box::new(FakeArr { tag: 7, view: qb_jit::ctx::ArrView { data: data.as_mut_ptr() as *mut u8, lbound: 1, count: 4 } });
    let b = Box::new(FakeArr { tag: 8, view: qb_jit::ctx::ArrView { data: fdata.as_mut_ptr() as *mut u8, lbound: 0, count: 4 } });
    let mut table: Vec<u64> = vec![&*a as *const FakeArr as u64, 0, &*b as *const FakeArr as u64];
    let mut f = Fb::new("va");
    let (i, v, out1, out2, x, y) = (f.slot(Ty::I32), f.slot(Ty::I64), f.slot(Ty::I64), f.slot(Ty::I64), f.slot(Ty::F64), f.slot(Ty::F64));
    let mut m = module(vec![], vec![]);
    let hl = m.hosts.len() as u32;
    m.hosts.push(qb_jit::mir::HostFn {
        sig: HostSig { name: "slow_load".into(), params: vec![Ty::I64; 2], ret: Some(Ty::I64), may_error: true },
        addr: slow_load as *const () as usize,
    });
    m.hosts.push(qb_jit::mir::HostFn {
        sig: HostSig { name: "slow_store".into(), params: vec![Ty::I64; 3], ret: None, may_error: true },
        addr: slow_store as *const () as usize,
    });
    let va = |load, reg, index, elem, idx, slow| Inst::VAElem { load, reg, table: 0, index, view_off: 8, elem, idx, slow };
    f.ci(i, 3);
    f.ci(v, 70000); // wraps to i16 on store
    f.i(va(false, v, 0, Ty::I16, i, hl + 1));
    f.i(va(true, out1, 0, Ty::I16, i, hl));
    f.i(va(true, out2, 1, Ty::I16, i, hl)); // null entry: slow path
    f.cf(x, 0.1);
    f.i(va(false, x, 2, Ty::F32, i, hl + 1));
    f.i(va(true, y, 2, Ty::F32, i, hl));
    f.ci(i, 9); // out of bounds: slow path returns 1000*0 + 9
    f.i(va(true, out2, 0, Ty::I16, i, hl));
    f.line(5, 50);
    f.ci(i, 99);
    f.i(va(true, v, 0, Ty::I16, i, hl));
    m.funcs.push(f.f);
    let s = Setup::default();
    for e in [Engine::Interp, Engine::Jit] {
        // SAFETY: the view points into `data`; go through it so no reference aliases native writes.
        unsafe { core::ptr::write_bytes(a.view.data, 0, 8) };
        let o = run_with_tables(&m, &s, e, [table.as_mut_ptr(), core::ptr::null_mut()]);
        assert_eq!((o.status, o.err, o.err_line), (qb_jit::status::ERROR, 9, 5));
        assert_eq!(o.frame[2] as i64, 70000i64 as i16 as i64);
        assert_eq!(o.frame[3] as i64, 9);
        assert_eq!(f64s(o.frame[5]), f64::from(0.1f32));
        assert_eq!(unsafe { core::ptr::read_volatile((a.view.data as *const i16).add(2)) }, 70000i64 as i16);
    }
    let _ = (a.tag, b.tag);
}

#[test]
fn aot_objects_link_for_every_target() {
    let m = module(vec![loop_mod(10).funcs.remove(0), fact_func()], vec![]);
    use qb_jit::Target as T;
    for t in [T::Host, T::X86_64, T::Aarch64, T::X86_64Darwin, T::Aarch64Darwin] {
        let obj = qb_jit::aot::compile_object(&m, t).unwrap_or_else(|e| panic!("{t:?}: {e:?}"));
        let img = qb_link::link(&obj).unwrap_or_else(|e| panic!("{t:?}: {e}"));
        assert_eq!(img.symbols.iter().filter(|(n, _)| n.starts_with("qb_func_")).count(), 2, "{t:?}");
        // names are plain C names on every format (Mach-O's `_` prefix is dropped)
        for r in &img.relocs {
            if let qb_link::Target::External(n) = &r.target {
                assert!(n.starts_with("qb_"), "{t:?}: external {n}");
            }
        }
        assert_eq!(qb_link::Image::from_bytes(&img.to_bytes()).unwrap(), img);
    }
}

/// `rust-lld` of the active Rust toolchain, if present.
fn rust_lld() -> Option<std::path::PathBuf> {
    let out = |args: &[&str]| {
        std::process::Command::new("rustc").args(args).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
    };
    let sysroot = out(&["--print", "sysroot"])?.trim().to_string();
    let host = out(&["-vV"])?.lines().find_map(|l| l.strip_prefix("host: ").map(str::to_string))?;
    let exe = if cfg!(windows) { "rust-lld.exe" } else { "rust-lld" };
    let p = std::path::Path::new(&sysroot).join("lib/rustlib").join(host).join("bin").join(exe);
    p.is_file().then_some(p)
}

/// D15 final for Linux and macOS: the native object (program + entry table)
/// of every ELF and Mach-O target is position-independent and accepted by
/// the system linker (rust-lld as ld.lld / ld64.lld). Runtime symbols are
/// left undefined here; linking the real executables runs in CI
/// (`.github/workflows`, job `native-exe`).
#[test]
fn native_objects_link_with_lld_for_elf_and_macho() {
    use qb_jit::Target as T;
    let Some(lld) = rust_lld() else {
        eprintln!("skipped: rust-lld not found");
        return;
    };
    let m = module(vec![loop_mod(10).funcs.remove(0), fact_func()], vec![]);
    let dir = std::env::temp_dir().join(format!("qb-native-obj-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (t, name) in [(T::X86_64, "x64.o"), (T::Aarch64, "a64.o"), (T::X86_64Darwin, "x64m.o"), (T::Aarch64Darwin, "a64m.o")] {
        let obj = qb_jit::aot::compile_native_object(&m, t, b"QIR 1\n", 7).unwrap_or_else(|e| panic!("{t:?}: {e:?}"));
        let path = dir.join(name);
        std::fs::write(&path, &obj).unwrap();
        let mut cmd = std::process::Command::new(&lld);
        match t {
            T::X86_64 | T::Aarch64 => {
                // `-z text`: refuse text relocations, i.e. the code must be PIC.
                cmd.args(["-flavor", "gnu", "-shared", "-z", "text", "--unresolved-symbols=ignore-all", "-o"]);
                cmd.arg(dir.join(format!("{name}.so")));
            }
            _ => {
                let arch = if t == T::X86_64Darwin { "x86_64" } else { "arm64" };
                cmd.args(["-flavor", "darwin", "-arch", arch, "-platform_version", "macos", "11.0", "11.0"]);
                cmd.args(["-dylib", "-undefined", "dynamic_lookup", "-o"]);
                cmd.arg(dir.join(format!("{name}.dylib")));
            }
        }
        let o = cmd.arg(&path).output().unwrap();
        assert!(o.status.success(), "{t:?}: {}{}", String::from_utf8_lossy(&o.stderr), String::from_utf8_lossy(&o.stdout));
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn byref_copy_out_and_call_budget() {
    // SUB dbl(x&): x& = x& * 2   called with copy-in/copy-out
    let mut g = Fb::new("dbl");
    let p = g.param(Ty::I32);
    let two = g.slot(Ty::I32);
    g.ci(two, 2);
    g.bin(BinOp::Mul, p, p, two);
    g.t(Term::Ret);
    let mut f = Fb::new("main");
    let x = f.slot(Ty::I32);
    f.ci(x, 21);
    f.i(Inst::Call { func: 1, args: vec![Arg::Val(x)], dst: None, pc: 7, back: vec![(0, x)] });
    let m = module(vec![f.f, g.f], vec![]);
    let o = both(&m, 0, &Setup::default());
    assert_eq!(o.frame[0], 42);
}

#[test]
fn recursion_deeper_than_the_budget_deopts_at_the_call() {
    // The reference interpreter recurses on the Rust stack: give it room.
    std::thread::Builder::new().stack_size(512 << 20).spawn(deep_recursion).unwrap().join().unwrap();
}

fn deep_recursion() {
    // FUNCTION down&(n&): IF n& > 0 THEN down& = down&(n& - 1) + 1
    let mut g = Fb::new("down");
    let n = g.param(Ty::I32);
    let (r, one, zero, c, m1, sub) = (g.slot(Ty::I32), g.slot(Ty::I32), g.slot(Ty::I32), g.slot(Ty::I16), g.slot(Ty::I32), g.slot(Ty::I32));
    g.f.ret = Some(r);
    let rec = g.block(None);
    let done = g.block(None);
    g.at(0);
    g.ci(one, 1);
    g.ci(zero, 0);
    g.i(Inst::Cmp { cc: Cc::Gt, dst: c, a: n, b: zero });
    g.t(Term::Br { cond: c, then: rec, els: done });
    g.at(rec);
    g.bin(BinOp::Sub, m1, n, one);
    g.i(Inst::Call { func: 1, args: vec![Arg::Val(m1)], dst: Some(sub), pc: 77, back: vec![] });
    g.bin(BinOp::Add, r, sub, one);
    g.t(Term::Ret);
    g.at(done);
    g.t(Term::Ret);
    let mut f = Fb::new("main");
    let (k, out) = (f.slot(Ty::I32), f.slot(Ty::I32));
    f.ci(k, 5000);
    f.i(Inst::Call { func: 1, args: vec![Arg::Val(k)], dst: Some(out), pc: 3, back: vec![] });
    let m = module(vec![f.f, g.f], vec![]);
    let o = both(&m, 0, &Setup::default());
    // budget 1000: the 1001st nested call leaves for the VM at pc 77 with 1000 frames in the chain
    assert_eq!((o.status, o.exit_pc), (status::DEOPT, 77));
    assert_eq!(o.chain.len(), qb_jit::ctx::DEFAULT_CALL_BUDGET as usize);
    let small = module(vec![{ let mut f = Fb::new("main"); let (k, out) = (f.slot(Ty::I32), f.slot(Ty::I32)); f.ci(k, 500); f.i(Inst::Call { func: 1, args: vec![Arg::Val(k)], dst: Some(out), pc: 3, back: vec![] }); f.f }, m.funcs[1].clone()], vec![]);
    let o = both(&small, 0, &Setup::default());
    assert_eq!((o.status, o.frame[1]), (status::END, 500));
}

#[test]
fn goto_and_brnopoll_are_not_event_safe_points() {
    // i = i + 1 three times through backward Goto / BrNoPoll edges, then a polling Jmp.
    let mut f = Fb::new("np");
    let (i, one, three, c) = (f.slot(Ty::I32), f.slot(Ty::I32), f.slot(Ty::I32), f.slot(Ty::I16));
    let head = f.block(Some(10));
    let mid = f.block(None);
    let tail = f.block(Some(30));
    f.at(0);
    f.ci(one, 1);
    f.ci(three, 3);
    f.t(Term::Jmp(head));
    f.at(head);
    f.bin(BinOp::Add, i, i, one);
    f.t(Term::Goto(mid));
    f.at(mid);
    f.i(Inst::Cmp { cc: Cc::Lt, dst: c, a: i, b: three });
    f.t(Term::BrNoPoll { cond: c, then: head, els: tail });
    f.at(tail);
    f.t(Term::Jmp(head));
    let o = both(&module(vec![f.f], vec![]), 0, &Setup { event: 1, ..Setup::default() });
    // The forward Jmp into `head` (block 0 -> 1) does not poll; the backward BrNoPoll edges do not
    // either; the first poll is the backward Jmp from `tail`, after i reached 3.
    assert_eq!((o.status, o.exit_pc, o.frame[0]), (status::EVENT, 10, 3));
}
