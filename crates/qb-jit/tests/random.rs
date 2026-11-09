//! Randomized differential tests: generated MIR programs (typed arithmetic,
//! conversions, comparisons, math, loops, errors) must leave identical
//! state in the reference interpreter and in native code.

mod common;
use common::*;
use qb_jit::mir::*;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn pick<T: Copy>(&mut self, v: &[T]) -> T {
        v[self.below(v.len() as u64) as usize]
    }
}

const TYS: [Ty; 5] = [Ty::I16, Ty::I32, Ty::I64, Ty::F32, Ty::F64];

fn int_val(r: &mut Rng) -> i64 {
    match r.below(6) {
        0 => r.pick(&[0, 1, -1, 2, 7, -7]),
        1 => r.pick(&[i16::MIN as i64, i16::MAX as i64, i32::MIN as i64, i32::MAX as i64, i64::MIN, i64::MAX]),
        2 => (r.next() as i64) >> r.below(63),
        _ => r.below(2000) as i64 - 1000,
    }
}

fn float_val(r: &mut Rng) -> f64 {
    match r.below(7) {
        0 => r.pick(&[0.0, -0.0, 0.5, 1.5, 2.5, -2.5, -0.5, 3.5]),
        1 => r.pick(&[f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e300, -1e300, 9.2e18, -9.3e18, 1.8e19]),
        2 => r.pick(&[32767.5, -32768.5, 32766.5, 2147483647.5, -2147483648.5, 2147483646.5]),
        3 => (r.next() as i64 as f64) / 1e6,
        _ => (r.below(200000) as f64 - 100000.0) / 64.0,
    }
}

/// Builds one random program; returns the module.
fn gen(seed: u64) -> Module {
    let mut r = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
    let mut f = Fb::new("rand");
    let mut by_ty: Vec<(Ty, Vec<Reg>)> = Vec::new();
    for t in TYS {
        let n = 2 + r.below(3) as usize;
        let regs: Vec<Reg> = (0..n).map(|_| f.slot(t)).collect();
        by_ty.push((t, regs));
    }
    let slots = |t: Ty, by: &Vec<(Ty, Vec<Reg>)>| by.iter().find(|(x, _)| *x == t).expect("type").1.clone();
    let ctr = f.slot(Ty::I32);
    let lim = f.slot(Ty::I32);
    let one = f.slot(Ty::I32);
    let c = f.slot(Ty::I16);
    let head = f.block(Some(100));
    let body = f.block(Some(200));
    let exit = f.block(None);
    f.at(0);
    f.line(1, 1);
    for (t, regs) in by_ty.clone() {
        for reg in regs {
            if t.is_float() {
                f.cf(reg, float_val(&mut r));
            } else {
                f.ci(reg, int_val(&mut r));
            }
        }
    }
    f.ci(lim, 1 + r.below(4) as i64);
    f.ci(one, 1);
    f.t(Term::Jmp(head));
    f.at(head);
    f.i(Inst::Cmp { cc: Cc::Lt, dst: c, a: ctr, b: lim });
    f.t(Term::Br { cond: c, then: body, els: exit });
    f.at(body);
    let n = 5 + r.below(30);
    for k in 0..n {
        if r.below(6) == 0 {
            f.line(10 + k as u32, 1000 + k as u32);
        }
        let t = r.pick(&TYS);
        let regs = slots(t, &by_ty);
        let (d, a, b) = (r.pick(&regs), r.pick(&regs), r.pick(&regs));
        match r.below(10) {
            0..=3 => {
                let ops: &[BinOp] = if t.is_float() {
                    &[BinOp::Add, BinOp::Sub, BinOp::Mul, BinOp::Div, BinOp::FDiv, BinOp::Pow]
                } else {
                    &[
                        BinOp::Add,
                        BinOp::Sub,
                        BinOp::Mul,
                        BinOp::IDiv,
                        BinOp::Mod,
                        BinOp::And,
                        BinOp::Or,
                        BinOp::Xor,
                        BinOp::Eqv,
                        BinOp::Imp,
                    ]
                };
                f.bin(r.pick(ops), d, a, b);
            }
            4 => {
                let dt = r.pick(&[Ty::I16, Ty::I32, Ty::I64]);
                let dst = r.pick(&slots(dt, &by_ty));
                let cc = r.pick(&[Cc::Eq, Cc::Ne, Cc::Lt, Cc::Le, Cc::Gt, Cc::Ge]);
                f.i(Inst::Cmp { cc, dst, a, b });
            }
            5 | 6 => {
                let dt = r.pick(&TYS);
                let dst = r.pick(&slots(dt, &by_ty));
                match r.below(3) {
                    0 => f.i(Inst::Conv { dst, src: a }),
                    1 => f.i(Inst::ConvWrap { dst, src: a }),
                    _ => {
                        if t.is_float() && !dt.is_float() {
                            f.i(Inst::Trunc { dst, src: a })
                        } else {
                            f.i(Inst::Conv { dst, src: a })
                        }
                    }
                }
            }
            7 => {
                if t.is_float() {
                    let op = r.pick(&[MathOp::Abs, MathOp::Floor, MathOp::Fix, MathOp::Sqrt]);
                    f.i(Inst::Math { op, dst: d, src: a });
                    if t == Ty::F64 && r.below(2) == 0 {
                        f.i(Inst::Math { op: MathOp::RoundF32, dst: d, src: a });
                    }
                } else {
                    f.i(Inst::Math { op: MathOp::Abs, dst: d, src: a });
                    let st = r.pick(&TYS);
                    let src = r.pick(&slots(st, &by_ty));
                    f.i(Inst::Math { op: MathOp::Sgn, dst: d, src });
                }
            }
            8 => {
                if t.is_float() {
                    f.i(Inst::Neg { dst: d, src: a });
                } else if r.below(2) == 0 {
                    f.i(Inst::Not { dst: d, src: a });
                } else {
                    f.i(Inst::Neg { dst: d, src: a });
                }
            }
            _ => f.i(Inst::Mov { dst: d, src: a }),
        }
    }
    f.bin(BinOp::Add, ctr, ctr, one);
    f.t(Term::Jmp(head));
    f.at(exit);
    f.t(Term::End);
    module(vec![f.f], vec![])
}

#[test]
fn random_programs_match() {
    let n: u64 = std::env::var("QB_JIT_RANDOM").ok().and_then(|v| v.parse().ok()).unwrap_or(1500);
    let mut ended = 0;
    let mut errored = 0;
    for seed in 0..n {
        let m = gen(seed);
        let a = run_one(&m, 0, &Setup::default(), Engine::Interp);
        let b = run_one(&m, 0, &Setup::default(), Engine::Jit);
        if a != b {
            panic!("seed {seed}: interp vs jit differ\ninterp: {a:?}\njit:    {b:?}\nprogram: {:#?}", m.funcs[0]);
        }
        if a.status == qb_jit::status::END {
            ended += 1;
        } else {
            errored += 1;
        }
    }
    // Both paths are exercised.
    assert!(ended > n / 10 && errored > n / 20, "ended {ended}, errored {errored}");
}
