//! Microbenchmarks of the VM + JIT.
//!
//! ```text
//! cargo run --release -p qb-exec --example bench
//! ```
//!
//! `benches/qir/*.qir` are `benches/bas/*.bas` compiled with `qb --ir`; each
//! program times its own loop with `TIMER(.001)` and prints `<result> <ms>`,
//! so the same `.bas` built with `qb` (or run in another BASIC) prints numbers
//! that compare directly. Every case runs interpreted only (`QB_JIT=off`) and
//! with the JIT, best of three; the table shows the VM and JIT times side by side.

use std::path::Path;

use qb_jit::tier::Mode;

fn ms(out: &[u8]) -> f64 {
    String::from_utf8_lossy(out).split_whitespace().last().and_then(|w| w.parse().ok()).unwrap_or(f64::NAN)
}

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("benches/qir");
    let mut files: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).collect();
    files.sort();
    println!("{:<8} {:>10} {:>10}  output", "case", "VM ms", "JIT ms");
    for f in files {
        let src = std::fs::read_to_string(&f).unwrap();
        let m = qb_exec::compile(&src).unwrap_or_else(|e| panic!("{}: {e:?}", f.display()));
        let mut best = [f64::MAX; 2];
        let mut shown = String::new();
        for (k, mode) in [Mode::Off, Mode::Hot].into_iter().enumerate() {
            for _ in 0..3 {
                let mut out = Vec::new();
                {
                    let mut vm = qb_exec::Vm::new(&m, &mut out);
                    vm.set_jit(mode, 1000);
                    vm.run().unwrap();
                }
                best[k] = best[k].min(ms(&out));
                shown = String::from_utf8_lossy(&out).trim().to_string();
            }
        }
        let name = f.file_stem().unwrap().to_string_lossy();
        println!("{:<8} {:>10.0} {:>10.0}  {shown}", name, best[0], best[1]);
    }
}
