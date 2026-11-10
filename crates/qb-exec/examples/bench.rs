//! Microbenchmarks of the typed register VM (R11.5).
//!
//! ```text
//! cargo run --release -p qb-exec --example bench [-- <scale>]
//! ```
//!
//! Each case is the QIR the emitter produces for a small BASIC loop (the
//! same programs live in `benches/bas/*.bas` for timing the C++ backend).
//! Times exclude loading; `scale` multiplies the iteration counts (default 1).

use std::time::Instant;

fn hex(s: &str) -> String {
    s.bytes().map(|b| format!("{b:02X}")).collect()
}

/// `FOR <v> = 1 TO <n>` ... `NEXT` around `body` (QIR v0, as the emitter writes it).
fn for_loop(tag: &str, v: &str, n: u64, body: &str) -> String {
    format!(
        "LINE 1\nPUSH I 1\nSTORE {v}\nPUSH L {n}\nSTORE ~{tag}.L\nPUSH I 1\nSTORE ~{tag}.S\nLABEL ~{tag}.T\n\
         LOAD {v}\nLOAD ~{tag}.L\nLOAD ~{tag}.S\nFORTEST\nJZ ~{tag}.X\n{body}\
         LINE 9\nLOAD {v}\nLOAD ~{tag}.S\nOP ADD\nSTORE {v}\nJMP ~{tag}.T\nLABEL ~{tag}.X\n"
    )
}

struct Case {
    name: &'static str,
    basic: &'static str,
    iters: u64,
    qir: String,
}

fn cases(scale: u64) -> Vec<Case> {
    let mut v = Vec::new();
    let n = 50_000_000 * scale;
    v.push(Case {
        name: "loop",
        basic: "FOR i& = 1 TO n: s& = s& + i& MOD 7: NEXT",
        iters: n,
        qir: format!(
            "QIR 0\n{}LINE 10\nLOAD S&\nPRINT\nPRINTNL\nEND\n",
            for_loop("F1", "I&", n, "LINE 2\nLOAD S&\nLOAD I&\nPUSH I 7\nOP MOD\nOP ADD\nSTORE S&\n")
        ),
    });
    let n = 20_000_000 * scale;
    v.push(Case {
        name: "float",
        basic: "FOR i& = 1 TO n: x# = x# + i& * 0.5: NEXT",
        iters: n,
        qir: format!(
            "QIR 0\n{}LINE 10\nLOAD X#\nPRINT\nPRINTNL\nEND\n",
            for_loop("F1", "I&", n, "LINE 2\nLOAD X#\nLOAD I&\nPUSH D 0.5\nOP MUL\nOP ADD\nSTORE X#\n")
        ),
    });
    let n = 2_000_000 * scale;
    v.push(Case {
        name: "concat",
        basic: "FOR i& = 1 TO n: a$ = a$ + \"x\": NEXT: PRINT LEN(a$)",
        iters: n,
        qir: format!(
            "QIR 0\n{}LINE 10\nLOAD A$\nFN LEN 1\nPRINT\nPRINTNL\nEND\n",
            for_loop("F1", "I&", n, &format!("LINE 2\nLOAD A$\nPUSH S {}\nOP ADD\nSTORE A$\n", hex("x")))
        ),
    });
    let n = 2_000_000 * scale;
    v.push(Case {
        name: "strfn",
        basic: "FOR i& = 1 TO n: b$ = MID$(\"hello world\", i& MOD 5 + 1, 3) + CHR$(65): k& = k& + LEN(b$): NEXT",
        iters: n,
        qir: format!(
            "QIR 0\n{}LINE 10\nLOAD K&\nPRINT\nPRINTNL\nEND\n",
            for_loop(
                "F1",
                "I&",
                n,
                &format!(
                    "LINE 2\nPUSH S {}\nLOAD I&\nPUSH I 5\nOP MOD\nPUSH I 1\nOP ADD\nPUSH I 3\nFN MID$ 3\nPUSH I 65\nFN CHR$ 1\nOP ADD\nSTORE B$\n\
                     LINE 3\nLOAD K&\nLOAD B$\nFN LEN 1\nOP ADD\nSTORE K&\n",
                    hex("hello world")
                )
            )
        ),
    });
    let n = 20_000_000 * scale;
    v.push(Case {
        name: "array",
        basic: "DIM a&(999): FOR i& = 1 TO n: j& = i& MOD 1000: a&(j&) = a&(j&) + i&: NEXT",
        iters: n,
        qir: format!(
            "QIR 0\nPUSH I 0\nPUSH I 999\nDIMA A&() 1\n{}LINE 10\nPUSH I 7\nLOADE A&() 1\nPRINT\nPRINTNL\nEND\n",
            for_loop(
                "F1",
                "I&",
                n,
                "LINE 2\nLOAD I&\nPUSH I 1000\nOP MOD\nSTORE J&\nLINE 3\nLOAD J&\nLOAD J&\nLOADE A&() 1\nLOAD I&\nOP ADD\nSTOREE A&() 1\n"
            )
        ),
    });
    let n = 5_000_000 * scale;
    v.push(Case {
        name: "sub",
        basic: "SUB inc (n&): n& = n& + 1: END SUB ... FOR i& = 1 TO n: inc c&: NEXT",
        iters: n,
        qir: format!(
            "QIR 0\n{}LINE 10\nLOAD C&\nPRINT\nPRINTNL\nEND\nSUB INC 1 N&\nLINE 20\nLOAD N&\nPUSH I 1\nOP ADD\nSTORE N&\nENDSUB\n",
            for_loop("F1", "I&", n, "LINE 2\nREF C&\nCALL INC 1\n")
        ),
    });
    let n = 5_000_000 * scale;
    v.push(Case {
        name: "func",
        basic: "FUNCTION sq& (x&): sq& = x& * x& MOD 1000: END FUNCTION ... FOR i& = 1 TO n: t& = t& + sq&(i&): NEXT",
        iters: n,
        qir: format!(
            "QIR 0\n{}LINE 10\nLOAD T&\nPRINT\nPRINTNL\nEND\nFUNCTION SQ& 1 X&\nLINE 20\nLOAD X&\nLOAD X&\nOP MUL\nPUSH I 1000\nOP MOD\nSTORE SQ&\nENDSUB\n",
            for_loop("F1", "I&", n, "LINE 2\nLOAD T&\nLOAD I&\nCALL SQ& 1\nOP ADD\nSTORE T&\n")
        ),
    });
    let n = 5_000_000 * scale;
    v.push(Case {
        name: "gosub",
        basic: "FOR i& = 1 TO n: GOSUB g: NEXT: ... g: c& = c& + 1: RETURN",
        iters: n,
        qir: format!(
            "QIR 0\n{}LINE 10\nLOAD C&\nPRINT\nPRINTNL\nEND\nLABEL G\nLINE 20\nLOAD C&\nPUSH I 1\nOP ADD\nSTORE C&\nRETURN\n",
            for_loop("F1", "I&", n, "LINE 2\nGOSUB G\n")
        ),
    });
    v
}

fn main() {
    let scale: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let show = std::env::args().any(|a| a == "--disasm");
    println!("{:<8} {:>12} {:>10} {:>9}  output / BASIC", "case", "iterations", "ms", "ns/iter");
    // calibration: the loop case in native Rust with a divisor the compiler cannot see
    let n = 50_000_000 * scale;
    let t = Instant::now();
    let (mut s, k) = (0i32, std::hint::black_box(7i32));
    for i in 1..=n as i32 {
        s = s.wrapping_add(i % k);
    }
    let el = t.elapsed().as_secs_f64();
    println!("{:<8} {:>12} {:>10.1} {:>9.2}  {s} / native Rust, runtime divisor", "native", n, el * 1e3, el * 1e9 / n as f64);
    for c in cases(scale) {
        let m = qb_exec::compile(&c.qir).unwrap_or_else(|e| panic!("{}: {e:?}", c.name));
        if show {
            eprintln!("--- {}\n{}", c.name, m.disasm());
        }
        let mut best = f64::MAX;
        let mut out = Vec::new();
        for _ in 0..3 {
            out.clear();
            let t = Instant::now();
            qb_exec::Vm::new(&m, &mut out).run().unwrap();
            best = best.min(t.elapsed().as_secs_f64());
        }
        let o = String::from_utf8_lossy(&out).trim().to_string();
        println!(
            "{:<8} {:>12} {:>10.1} {:>9.2}  {o} / {}",
            c.name,
            c.iters,
            best * 1e3,
            best * 1e9 / c.iters as f64,
            c.basic
        );
    }
}
