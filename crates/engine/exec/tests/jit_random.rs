//! Randomized differential test of native calls: generated QIR programs with
//! a hot loop calling SUBs/FUNCTIONs (BYVAL, BYREF, SHARED, recursion,
//! errors with and without a handler) must give the VM's output in hot,
//! eager and AOT mode.

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

/// QIR text of one function body; hands out registers.
struct F {
    code: String,
    reg: u32,
}

impl F {
    fn new() -> F {
        F { code: String::new(), reg: 0 }
    }
    fn r(&mut self) -> u32 {
        self.reg += 1;
        self.reg
    }
    fn line(&mut self, s: &str) {
        self.code += s;
        self.code.push('\n');
    }
    fn konst(&mut self, v: i64) -> u32 {
        let r = self.r();
        self.line(&format!("const.i32 %{r} {v}"));
        r
    }
    fn load(&mut self, var: &str) -> u32 {
        let r = self.r();
        self.line(&format!("load.i32 %{r} ${var}"));
        r
    }
    fn bin(&mut self, op: &str, a: u32, b: u32) -> u32 {
        let r = self.r();
        self.line(&format!("{op}.i32 %{r} %{a} %{b}"));
        r
    }
    /// Random integer expression over the given variables.
    fn expr(&mut self, g: &mut Rng, vars: &[&str], depth: u32) -> u32 {
        if depth == 0 || g.below(3) == 0 {
            return if g.below(2) == 0 {
                let v = vars[g.below(vars.len() as u64) as usize];
                self.load(v)
            } else {
                self.konst(g.below(19) as i64 - 9)
            };
        }
        let op = ["add", "sub", "mul", "mod", "idiv", "and", "xor"][g.below(7) as usize];
        let a = self.expr(g, vars, depth - 1);
        let b = self.expr(g, vars, depth - 1);
        self.bin(op, a, b)
    }
}

fn program(seed: u64) -> String {
    let mut g = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
    let handler = g.below(2) == 0;
    let n = 20 + g.below(200) as i64;
    // SUB P(x& BYREF, y& BYVAL): x& = f(x, y, G&) MOD 9973: G& = G& + 1
    let mut p = F::new();
    p.line("line 30");
    let e = p.expr(&mut g, &["X", "Y", "G"], 3);
    let k = p.konst(9973);
    let m = p.bin("mod", e, k);
    p.line(&format!("store.i32 $X %{m}"));
    p.line("line 31");
    let a = p.load("G");
    let one = p.konst(1);
    let s = p.bin("add", a, one);
    p.line(&format!("store.i32 $G %{s}"));
    p.line("ret");
    // FUNCTION F&(n& BYVAL): IF n& <= 0 THEN F& = 1 ELSE F& = (F&(n& - 1) + f(n)) MOD 30011
    let mut f = F::new();
    f.line("line 40");
    let nn = f.load("N");
    let z = f.konst(0);
    let c = f.bin("le", nn, z);
    f.line(&format!("jz %{c} REC"));
    let one = f.konst(1);
    f.line(&format!("ret %{one}"));
    f.line("label REC");
    f.line("line 41");
    let nn = f.load("N");
    let one = f.konst(1);
    let arg = f.bin("sub", nn, one);
    let res = f.r();
    f.line(&format!("call %{res} F %{arg}"));
    let e = f.expr(&mut g, &["N"], 2);
    let s = f.bin("add", res, e);
    let k = f.konst(30011);
    let m = f.bin("mod", s, k);
    f.line(&format!("ret %{m}"));
    // main
    let mut mn = F::new();
    mn.line("line 1");
    if handler {
        mn.line("onerror H");
    }
    let c3 = mn.konst(3);
    mn.line(&format!("store.i32 $A %{c3}"));
    let c5 = mn.konst(5);
    mn.line(&format!("store.i32 $B %{c5}"));
    let one = mn.konst(1);
    mn.line(&format!("store.i32 $I %{one}"));
    mn.line("label T");
    let i = mn.load("I");
    let lim = mn.konst(n);
    let c = mn.bin("gt", i, lim);
    mn.line(&format!("jnz %{c} X"));
    for k in 0..3 + g.below(4) {
        mn.line(&format!("line {}", 10 + k));
        match g.below(4) {
            0 | 3 => {
                let ptr = mn.r();
                let target = if g.below(2) == 0 { "A" } else { "C" };
                mn.line(&format!("addr %{ptr} ${target}"));
                let v = mn.load(if target == "A" { "B" } else { "I" });
                mn.line(&format!("call P %{ptr} %{v}"));
            }
            1 => {
                let c = mn.load("C");
                let e = mn.expr(&mut g, &["A", "B", "C"], 1);
                let s = mn.bin("add", c, e);
                let k7 = mn.konst(7);
                let a = mn.bin("mod", s, k7);
                let r = mn.r();
                mn.line(&format!("call %{r} F %{a}"));
                mn.line(&format!("store.i32 $B %{r}"));
            }
            _ => {
                let e = mn.expr(&mut g, &["A", "B", "C"], 3);
                let k = mn.konst(30011);
                let m = mn.bin("mod", e, k);
                mn.line(&format!("store.i32 $C %{m}"));
            }
        }
    }
    mn.line("line 49");
    let i = mn.load("I");
    let one = mn.konst(1);
    let s = mn.bin("add", i, one);
    mn.line(&format!("store.i32 $I %{s}"));
    mn.line("jmp T");
    mn.line("label X");
    mn.line("line 50");
    let ch = mn.konst(0);
    for v in ["A", "B", "C", "G", "E"] {
        let r = mn.load(v);
        let w = mn.r();
        mn.line(&format!("cvt.i64.i32 %{w} %{r}"));
        mn.line(&format!("rt PRINTN.i64 %{ch} %{w}"));
    }
    mn.line(&format!("rt PRINTNL %{ch}"));
    mn.line("end");
    mn.line("label H");
    mn.line("line 60");
    let e = mn.load("E");
    let one = mn.konst(1);
    let s = mn.bin("add", e, one);
    mn.line(&format!("store.i32 $E %{s}"));
    mn.line("resume next");
    format!(
        "QIR 1\nglobal $A i32\nglobal $B i32\nglobal $C i32\nglobal $G i32\nglobal $E i32\nglobal $I i32\n\
         func main\n{}endfunc\nfunc P\nparam $X i32 byref\nparam $Y i32 byval\n{}endfunc\n\
         func F -> i32\nparam $N i32 byval\n{}endfunc\n",
        mn.code, p.code, f.code
    )
}

fn run(src: &str, mode: Mode) -> (String, Result<i32, i32>) {
    let m = compile(src).unwrap_or_else(|e| panic!("load: {e:?}\n{src}"));
    let mut out = Vec::new();
    let r = {
        let mut vm = Vm::new(&m, &mut out);
        vm.set_jit(mode, 1);
        vm.run_status().map_err(|e| e.code)
    };
    (String::from_utf8_lossy(&out).into_owned(), r)
}

/// Negation in a native loop wraps to the variable's width like the VM
/// (`-u` of an `_UNSIGNED LONG` 5 is 4294967291, `-(-128)` of a `_BYTE` is -128).
#[test]
fn negation_wraps_to_every_integer_width() {
    let cases = [
        ("i8", -128i64),
        ("u8", 5),
        ("i16", -32768),
        ("u16", 5),
        ("i32", -2147483648),
        ("u32", 5),
        ("i64", 5),
        ("u64", 5),
    ];
    for (ty, v) in cases {
        let print = match ty {
            "i64" | "u64" => format!("rt PRINTN.{ty} %20 %10\n"),
            _ => format!("cvt.i64.{ty} %11 %10\nrt PRINTN.i64 %20 %11\n"),
        };
        // V = v: FOR I = 1 TO 3: V = -V: NEXT: PRINT V
        let p = format!(
            "QIR 1\nglobal $V {ty}\nglobal $I i32\nfunc main\nline 1\nconst.{ty} %0 {v}\nstore.{ty} $V %0\n\
             const.i32 %1 1\nstore.i32 $I %1\nlabel T\nload.i32 %2 $I\nconst.i32 %3 3\ngt.i32 %4 %2 %3\njnz %4 X\n\
             line 2\nload.{ty} %5 $V\nneg.{ty} %6 %5\nstore.{ty} $V %6\nload.i32 %7 $I\nconst.i32 %8 1\n\
             add.i32 %9 %7 %8\nstore.i32 $I %9\njmp T\nlabel X\nline 3\nload.{ty} %10 $V\nconst.i32 %20 0\n\
             {print}rt PRINTNL %20\nend\nendfunc\n"
        );
        let vm = run(&p, Mode::Off);
        let expect = match ty {
            "i8" => "-128 \n",
            "u8" => " 251 \n",
            "i16" => "-32768 \n",
            "u16" => " 65531 \n",
            "i32" => "-2147483648 \n",
            "u32" => " 4294967291 \n",
            "i64" => "-5 \n",
            _ => " 18446744073709551611 \n",
        };
        assert_eq!(vm, (expect.to_string(), Ok(0)), "{ty} VM");
        assert_eq!(vm, run(&p, Mode::Hot), "{ty} hot");
        assert_eq!(vm, run(&p, Mode::Eager), "{ty} eager");
    }
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
            let r = run_ir_native(&p, &img, &mut out).map_err(|e| e.code);
            assert_eq!(vm, (String::from_utf8_lossy(&out).into_owned(), r), "seed {seed} aot\n{p}");
        }
    }
    // Unhandled errors and handled ones both occur.
    assert!(errs > 0 && handled > 0, "errors {errs}, handled {handled}");
}
