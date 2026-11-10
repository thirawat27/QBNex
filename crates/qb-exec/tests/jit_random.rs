//! Randomized differential test of native calls: generated programs with a
//! hot loop calling SUBs/FUNCTIONs (BYVAL, BYREF, SHARED, recursion, errors
//! with and without a handler) must give the VM's output in hot, eager and
//! AOT mode.

use qb_exec::{build_native, compile, run_ir_native, Vm};
use qb_jit::tier::Mode;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
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
}

const VARS: [&str; 3] = ["A&", "B&", "C&"];

/// Random integer expression over the given variables (QIR v0 stack code).
fn expr(r: &mut Rng, vars: &[&str], depth: u32) -> String {
    if depth == 0 || r.below(3) == 0 {
        return if r.below(2) == 0 {
            format!("LOAD {}\n", vars[r.below(vars.len() as u64) as usize])
        } else {
            format!("PUSH I {}\n", r.below(19) as i64 - 9)
        };
    }
    let op = ["ADD", "SUB", "MUL", "MOD", "IDIV", "AND", "XOR"][r.below(7) as usize];
    format!("{}{}OP {op}\n", expr(r, vars, depth - 1), expr(r, vars, depth - 1))
}

fn program(seed: u64) -> String {
    let mut r = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
    let handler = r.below(2) == 0;
    let n = 20 + r.below(200);
    // SUB P(x&, y&): x& = f(x, y, G&) MOD 9973: G& = G& + 1
    let p_body = format!(
        "LINE 30\n{}PUSH I 9973\nOP MOD\nSTORE X&\nLINE 31\nLOAD G&\nPUSH I 1\nOP ADD\nSTORE G&\n",
        expr(&mut r, &["X&", "Y&", "G&"], 3)
    );
    // FUNCTION F&(n&): recursive on n& MOD 5
    let f_body = format!(
        "LINE 40\nLOAD N&\nPUSH I 0\nOP LE\nJZ ~R\nPUSH I 1\nSTORE F&\nJMP ~X\nLABEL ~R\nLINE 41\n\
         LOAD N&\nPUSH I 1\nOP SUB\nCALL F& 1\n{}OP ADD\nPUSH I 30011\nOP MOD\nSTORE F&\nLABEL ~X\n",
        expr(&mut r, &["N&"], 2)
    );
    let mut body = String::new();
    for k in 0..3 + r.below(4) {
        body += &format!("LINE {}\n", 10 + k);
        match r.below(4) {
            0 => body += "REF A&\nLOAD B&\nCALL P 2\n",
            1 => body += &format!("LOAD C&\n{}OP ADD\nPUSH I 7\nOP MOD\nCALL F& 1\nSTORE B&\n", expr(&mut r, &VARS, 1)),
            2 => body += &format!("{}PUSH I 30011\nOP MOD\nSTORE C&\n", expr(&mut r, &VARS, 3)),
            _ => body += "REF C&\nLOAD I&\nCALL P 2\n",
        }
    }
    let lp = format!(
        "PUSH I 1\nSTORE I&\nPUSH L {n}\nSTORE ~F1.L\nPUSH I 1\nSTORE ~F1.S\nLABEL ~F1.T\n\
         LOAD I&\nLOAD ~F1.L\nLOAD ~F1.S\nFORTEST\nJZ ~F1.X\n{body}\
         LOAD I&\nLOAD ~F1.S\nOP ADD\nSTORE I&\nJMP ~F1.T\nLABEL ~F1.X\n"
    );
    let on = if handler { "ONERROR MAIN.H\n" } else { "" };
    format!(
        "QIR 0\nSHARED G&\nLINE 1\n{on}PUSH I 3\nSTORE A&\nPUSH I 5\nSTORE B&\n{lp}LINE 50\nLOAD A&\nPRINT\nLOAD B&\nPRINT\nLOAD C&\nPRINT\nLOAD G&\nPRINT\nLOAD E&\nPRINT\nPRINTNL\nEND\n\
         LABEL MAIN.H\nLINE 60\nLOAD E&\nPUSH I 1\nOP ADD\nSTORE E&\nRESUME NEXT\n\
         SUB P 2 X& Y&\n{p_body}ENDSUB\nFUNCTION F& 1 N&\n{f_body}ENDSUB\n"
    )
}

fn run(src: &str, mode: Mode) -> (String, Result<(), i32>) {
    let m = compile(src).unwrap_or_else(|e| panic!("load: {e:?}\n{src}"));
    let mut out = Vec::new();
    let r = {
        let mut vm = Vm::new(&m, &mut out);
        vm.set_jit(mode, 1);
        vm.run().map_err(|e| e.code)
    };
    (String::from_utf8_lossy(&out).into_owned(), r)
}

#[test]
fn random_call_programs_match_the_vm() {
    let n: u64 = std::env::var("QB_JIT_RANDOM").ok().and_then(|v| v.parse().ok()).unwrap_or(300);
    let (mut errs, mut handled) = (0, 0);
    for seed in 0..n {
        let p = program(seed);
        let vm = run(&p, Mode::Off);
        errs += usize::from(vm.1.is_err());
        handled += usize::from(!vm.0.ends_with(" 0 \n"));
        assert_eq!(vm, run(&p, Mode::Hot), "seed {seed} hot\n{p}");
        assert_eq!(vm, run(&p, Mode::Eager), "seed {seed} eager\n{p}");
        if seed % 10 == 0 {
            let img = build_native(&p).unwrap();
            let mut out = Vec::new();
            let r = run_ir_native(&p, &img, &mut out).map(|_| ()).map_err(|e| e.code);
            assert_eq!(vm, (String::from_utf8_lossy(&out).into_owned(), r), "seed {seed} aot\n{p}");
        }
    }
    // Unhandled errors and handled ones both occur.
    assert!(errs > 0 && handled > 0, "errors {errs}, handled {handled}");
}
