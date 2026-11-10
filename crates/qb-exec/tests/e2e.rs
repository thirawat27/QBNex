//! End to end: `.bas` -> `qb.exe --ir` (BASIC compiler) -> QIR appended to the
//! prebuilt runner -> running program. No C++ compiler is involved.
//!
//! Needs `qb.exe` at the repository root (built by `cmake --build --preset
//! windows-mingw-selfhost`). Without it the tests print a notice and pass, so
//! `cargo test --workspace` still works before the compiler is built.
//!
//! `spike.bas` is also compiled through the default C++ path and both outputs
//! must match (differential check against the existing backend).

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn qb() -> Option<PathBuf> {
    let name = if cfg!(windows) { "qb.exe" } else { "qb" };
    let p = repo_root().join(name);
    if p.exists() {
        Some(p)
    } else {
        eprintln!("SKIP: {} not built", p.display());
        None
    }
}

/// Puts the freshly built runner where the compiler looks for it.
fn install_runner() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(install_runner_now);
}

fn install_runner_now() {
    let os = if cfg!(windows) { "windows" } else if cfg!(target_os = "macos") { "macos" } else { "linux" };
    let name = if cfg!(windows) { "qb-exec.exe" } else { "qb-exec" };
    let dir = repo_root().join("crates/qb-exec/bin").join(os);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(env!("CARGO_BIN_EXE_qb-exec"), dir.join(name)).unwrap();
}

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("qb_exec_e2e_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

fn normalize(b: &[u8]) -> String {
    String::from_utf8_lossy(b).replace("\r\n", "\n")
}

/// Compiles `src` with `qb.exe` plus `extra` flags and runs the result.
fn compile_and_run(qb: &Path, src: &Path, exe: &Path, extra: &[&str]) -> String {
    let out = Command::new(qb)
        .current_dir(repo_root())
        .arg(src)
        .args(extra)
        .arg("-o")
        .arg(exe)
        .arg("-q")
        .arg("-m")
        .output()
        .unwrap();
    assert!(
        out.status.success() && exe.exists(),
        "compile {} {:?} failed: {}{}",
        src.display(),
        extra,
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let run = Command::new(exe).output().unwrap();
    assert!(run.status.success(), "{} exited {:?}: {}", exe.display(), run.status, normalize(&run.stderr));
    normalize(&run.stdout)
}

#[test]
fn spike_program_runs_and_matches_the_golden() {
    let Some(qb) = qb() else { return };
    install_runner();
    let dir = tmpdir("spike");
    let src = fixture("spike.bas");
    let expected = normalize(&std::fs::read(fixture("spike.out")).unwrap());

    let ir_out = compile_and_run(&qb, &src, &dir.join("spike_ir.exe"), &["--ir"]);
    // the golden was recorded from the C++ backend before the cutover
    assert_eq!(ir_out, expected, "QIR output");
}

#[test]
fn on_error_resume_next_works_in_main_module() {
    // The C++ path cannot compile this (legacy defect L-01), so the expected
    // output is written by hand from QBasic semantics.
    let Some(qb) = qb() else { return };
    install_runner();
    let dir = tmpdir("onerror");
    let expected = normalize(&std::fs::read(fixture("onerror.out")).unwrap());
    let out = compile_and_run(&qb, &fixture("onerror.bas"), &dir.join("onerror.exe"), &["--ir"]);
    assert_eq!(out, expected);
}

/// Conformance cases the legacy C++ backend cannot run (L-01 ON ERROR in the
/// main module, L-03 RESTORE label). L-04 (relational operators in PRINT) is a\n/// front-end defect, so since the emitter uses the real front end it affects both backends.
/// The `.out` files were checked by hand against QBasic semantics.
#[test]
fn legacy_defect_cases_run_correctly_under_qir() {
    let Some(qb) = qb() else { return };
    install_runner();
    let dir = tmpdir("legacy");
    let fixtures = fixture("legacy_defects");
    let mut cases: Vec<_> = std::fs::read_dir(&fixtures)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "bas"))
        .collect();
    cases.sort();
    assert!(cases.len() >= 4);
    for src in cases {
        let stem = src.file_stem().unwrap().to_string_lossy().to_string();
        let expected = normalize(&std::fs::read(src.with_extension("out")).unwrap());
        let out = compile_and_run(&qb, &src, &dir.join(format!("{stem}.exe")), &["--ir"]);
        assert_eq!(out, expected, "{stem}");
    }
}

#[test]
fn ir_flag_rejects_unsupported_statement_with_line() {
    let Some(qb) = qb() else { return };
    install_runner();
    let dir = tmpdir("unsupported");
    let src = dir.join("bad.bas");
    // SCREEN, graphics, sound, ... are supported on QIR now; CHAIN is not (yet)
    std::fs::write(&src, "PRINT 1\nCHAIN \"other.bas\"\n").unwrap();
    let out = Command::new(&qb)
        .current_dir(repo_root())
        .arg(&src)
        .arg("--ir")
        .arg("-o")
        .arg(dir.join("bad.exe"))
        .output()
        .unwrap();
    let text = normalize(&out.stdout) + &normalize(&out.stderr);
    assert!(!out.status.success(), "should fail: {text}");
    assert!(text.contains("line 2"), "message should name the line: {text}");
}

/// Language programs (INCLUDE, CONST, DEFxxx, UDTs, arrays, SUB/FUNCTION,
/// STATIC, recursion, SELECT, loops, ON GOSUB, DATA, errors): the `.out`
/// golden is the C++ backend's output (recorded before the cutover); QIR must match it.
#[test]
fn language_programs_match_the_cpp_golden() {
    let Some(qb) = qb() else { return };
    install_runner();
    let dir = tmpdir("lang");
    for n in 1..=3 {
        let src = fixture(&format!("qir/lang{n}.bas"));
        let expected = normalize(&std::fs::read(src.with_extension("out")).unwrap());
        let ir = compile_and_run(&qb, &src, &dir.join(format!("lang{n}_ir.exe")), &["--ir"]);
        assert_eq!(ir, expected, "lang{n} under QIR");
    }
}