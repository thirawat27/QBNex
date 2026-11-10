//! Differential test over real QIR v1 programs: `tests/fixtures/qir1/*.qir`
//! were produced by the compiler (`qb --ir`, R11a emitter) from
//! `tests/conformance/cases`. Each one must give the same output, exit code
//! and error through the typed VM (`run_ir_status`) as through the R11a
//! reference interpreter (`qb_exec::v1::run`).
//!
//! Refresh: compile each case with `qb --ir` and copy `internal/temp/program.qir`.

use std::path::Path;

fn outcome(r: Result<i32, qb_exec::RunError>, out: Vec<u8>) -> String {
    let o = String::from_utf8_lossy(&out).into_owned();
    match r {
        Ok(code) => format!("{o}<exit {code}>"),
        Err(e) => format!("{o}<error {} line {}>", e.code, e.line),
    }
}

#[test]
fn corpus_matches_reference_interpreter() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/qir1");
    let mut files: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).collect();
    files.sort();
    assert!(files.len() >= 30, "corpus missing");
    let mut bad = Vec::new();
    let mut on_vm = 0;
    for f in &files {
        let src = std::fs::read_to_string(f).unwrap();
        let m = qb_ir::parse(&src).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        let mut a = Vec::new();
        let ra = qb_exec::v1::run(&m, &mut a);
        let reference = outcome(ra, a);
        let mut b = Vec::new();
        let rb = qb_exec::run_ir_status(&src, &mut b);
        let vm = outcome(rb, b);
        if qb_exec::v1_on_vm(&m) {
            on_vm += 1;
        }
        if reference != vm {
            bad.push(format!("{}:\n--- reference\n{reference}\n--- vm\n{vm}", f.display()));
        }
    }
    eprintln!("qir1 corpus: {} programs, {on_vm} on the typed VM", files.len());
    assert!(bad.is_empty(), "{}", bad.join("\n\n"));
}
