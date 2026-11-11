//! Real programs: `tests/fixtures/corpus/*.qir` are conformance cases
//! compiled with the compiler (`qb --ir`). Each must print exactly
//! `<name>.expect` (output, then `<exit n>` or `<error code line n>`), which
//! was recorded from the R11a reference interpreter before it was retired.
//! `QB_BLESS=1` rewrites the files from the VM.
//!
//! Refresh: compile each case with `qb --ir`, copy `internal/temp/program.qir`.

use std::path::Path;

fn outcome(r: Result<i32, qb_exec::RunError>, out: Vec<u8>) -> String {
    let o = String::from_utf8_lossy(&out).into_owned();
    match r {
        Ok(code) => format!("{o}<exit {code}>"),
        Err(e) => format!("{o}<error {} line {}>", e.code, e.line),
    }
}

#[test]
fn corpus_matches_expected_output() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/corpus");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("qir"))
        .collect();
    files.sort();
    assert!(files.len() >= 39, "corpus missing");
    let bless = std::env::var_os("QB_BLESS").is_some();
    let mut bad = Vec::new();
    for f in &files {
        let src = std::fs::read_to_string(f).unwrap();
        let mut b = Vec::new();
        let got = outcome(qb_exec::run_ir_status(&src, &mut b), b);
        let exp = f.with_extension("expect");
        if bless {
            std::fs::write(&exp, &got).unwrap();
            continue;
        }
        let want = std::fs::read_to_string(&exp).unwrap_or_else(|e| panic!("{}: {e}", exp.display()));
        if got != want {
            bad.push(format!("{}:\n--- expected\n{want}\n--- got\n{got}", f.display()));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n\n"));
}
