//! D15 final: programs linked by rust-lld with the qb-native runtime library
//! into standard executables give the VM's output.
//!
//! Needs a native kit (`QBNEX_NATIVE_KIT`, built by `scripts/native-kit.ps1`);
//! without one the test only reports that it was skipped.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const PROGRAMS: &[(&str, &str)] = &[
    ("hello", "native/hello.qir"),
    ("loops_math", "corpus/numeric_math_funcs.qir"),
    ("control_loops", "corpus/control_loops.qir"),
    ("strings", "corpus/strings_basic_funcs.qir"),
    ("file_io", "native/file_io.qir"),
    ("subs_functions", "corpus/control_subs_functions.qir"),
    ("native_calls", "jit/native_byref_and_function.qir"),
];

/// The program as a console program (`meta console only`): without it the
/// payload runner opens a window (GUI program) and stdout stays empty.
fn fixture(p: &str) -> String {
    let src = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(p)).unwrap();
    if src.lines().any(|l| l.starts_with("meta console")) {
        return src;
    }
    let (head, rest) = src.split_once('\n').unwrap();
    format!("{head}\nmeta console only\n{rest}")
}

fn run(exe: &Path, dir: &Path) -> (String, Option<i32>) {
    let o = Command::new(exe)
        .current_dir(dir)
        .env("QBNEX_NO_ERROR_DIALOG", "1")
        // never a window on the screen, whatever the program is
        .env("QBNEX_HEADLESS", "1")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    // line ends compared as LF (the payload runner writes CR LF on Windows)
    (String::from_utf8_lossy(&o.stdout).replace("\r\n", "\n"), o.status.code())
}

#[test]
fn linked_executables_match_the_vm() {
    let Some(kit) = std::env::var_os("QBNEX_NATIVE_KIT").map(PathBuf::from) else {
        eprintln!("skipped: QBNEX_NATIVE_KIT not set");
        return;
    };
    assert!(kit.join("kit.txt").is_file(), "no kit.txt in {}", kit.display());
    let dir = std::env::temp_dir().join(format!("qb-native-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (name, path) in PROGRAMS {
        let src = fixture(path);
        let qir = dir.join(format!("{name}.qir"));
        std::fs::write(&qir, &src).unwrap();
        let exe = dir.join(format!("{name}.exe"));
        let pay = dir.join(format!("{name}_payload.exe"));
        let b = |out: &Path, mode: &str| {
            let o = Command::new(env!("CARGO_BIN_EXE_qb-exec")).arg("build").arg(&qir).arg("-o").arg(out).arg(mode).output().unwrap();
            assert!(o.status.success(), "{name} {mode}: {}", String::from_utf8_lossy(&o.stderr));
        };
        b(&exe, "--native-link");
        b(&pay, "--payload");
        let bytes = std::fs::read(&exe).unwrap();
        assert!(!bytes.ends_with(b"QBNEXIR0") && !bytes.ends_with(b"QBNEXAO1"), "{name}: payload, not linked");
        let native = run(&exe, &dir);
        let payload = run(&pay, &dir);
        let mut vm = Vec::new();
        let _ = qb_exec::run_program(&src, None, &mut vm);
        assert_eq!(native.0, String::from_utf8_lossy(&vm), "{name}: native vs VM");
        assert_eq!(native, payload, "{name}: native vs payload runner");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
