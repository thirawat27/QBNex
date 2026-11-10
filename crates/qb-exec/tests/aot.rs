//! D15: programs compiled ahead of time (native image built with
//! cranelift-object + qb-link, loaded at start-up) give the VM's results.

use std::path::Path;

use qb_exec::{build_native, run_ir, run_ir_native};

fn programs() -> Vec<String> {
    ["numeric_loop", "arrays_in_loop", "error_resume_next_in_loop", "aot_mixed", "native_byref_and_function"]
        .iter()
        .map(|n| {
            let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/jit").join(format!("{n}.qir"));
            std::fs::read_to_string(p).unwrap()
        })
        .collect()
}

#[test]
fn native_image_matches_the_vm() {
    for p in programs() {
        let mut vm = Vec::new();
        let r1 = run_ir(&p, &mut vm).map_err(|e| (e.code, e.line));
        let img = build_native(&p).unwrap_or_else(|e| panic!("build: {e:?}"));
        let mut nat = Vec::new();
        let r2 = run_ir_native(&p, &img, &mut nat).map(|_| ()).map_err(|e| (e.code, e.line));
        assert_eq!((String::from_utf8_lossy(&vm), r1), (String::from_utf8_lossy(&nat), r2));
    }
}

#[test]
fn image_of_another_program_is_rejected() {
    let ps = programs();
    let img = build_native(&ps[0]).unwrap();
    let mut out = Vec::new();
    assert!(run_ir_native(&ps[3], &img, &mut out).is_err());
}

#[test]
fn qb_exec_build_writes_a_native_executable() {
    let dir = std::env::temp_dir().join(format!("qb-aot-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = &programs()[3];
    let src = dir.join("prog.qir");
    std::fs::write(&src, p).unwrap();
    let exe = dir.join(if cfg!(windows) { "prog.exe" } else { "prog" });
    let st = std::process::Command::new(env!("CARGO_BIN_EXE_qb-exec")).arg("build").arg(&src).arg("-o").arg(&exe).status().unwrap();
    assert!(st.success());
    let out = std::process::Command::new(&exe).output().unwrap();
    let mut vm = Vec::new();
    run_ir(p, &mut vm).unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&vm));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn image_of_a_same_shaped_program_is_rejected() {
    let prog = |v: i32| format!("QIR 1\nfunc main\nconst.i32 %0 0\nconst.i64 %1 {v}\nrt PRINTN.i64 %0 %1\nrt PRINTNL %0\nend\nendfunc\n");
    let img = build_native(&prog(1)).unwrap();
    let mut out = Vec::new();
    let e = run_ir_native(&prog(2), &img, &mut out).unwrap_err();
    assert!(e.msg.contains("another program"), "{e:?}");
}
