//! D15: programs compiled ahead of time (native image built with
//! cranelift-object + qb-link, loaded at start-up) give the VM's results.

use qb_exec::{build_native, run_ir, run_ir_native};

fn for_loop(id: u32, v: &str, from: &str, to: &str, step: &str, body: &str) -> String {
    format!(
        "PUSH {from}\nSTORE {v}\nPUSH {to}\nSTORE ~F{id}.L\nPUSH {step}\nSTORE ~F{id}.S\nLABEL ~F{id}.T\n\
         LOAD {v}\nLOAD ~F{id}.L\nLOAD ~F{id}.S\nFORTEST\nJZ ~F{id}.X\n{body}\n\
         LOAD {v}\nLOAD ~F{id}.S\nOP ADD\nSTORE {v}\nJMP ~F{id}.T\nLABEL ~F{id}.X\n"
    )
}

fn programs() -> Vec<String> {
    let mut v = Vec::new();
    let body = "LOAD S&\nLOAD I&\nPUSH I 7\nOP MOD\nOP ADD\nSTORE S&";
    v.push(format!("QIR 0\n{}LOAD S&\nPRINT\nPRINTNL\nEND\n", for_loop(1, "I&", "I 1", "L 100000", "I 1", body)));
    let fill = "LOAD I%\nLOAD I%\nLOAD I%\nOP MUL\nSTOREE A&() 1";
    let sum = "LOAD T&\nLOAD I%\nLOADE A&() 1\nOP ADD\nSTORE T&";
    v.push(format!(
        "QIR 0\nPUSH I 0\nPUSH I 100\nDIMA A&() 1\n{}{}LOAD T&\nPRINT\nPRINTNL\nEND\n",
        for_loop(1, "I%", "I 1", "I 100", "I 1", fill),
        for_loop(2, "I%", "I 1", "I 100", "I 1", sum)
    ));
    let body = "LINE 3\nPUSH I 100\nLOAD I%\nPUSH I 3\nOP MOD\nOP IDIV\nSTORE X%\nLINE 4\nLOAD S%\nLOAD X%\nOP ADD\nSTORE S%";
    v.push(format!(
        "QIR 0\nLINE 1\nONERROR MAIN.H\nLINE 2\n{}LINE 5\nLOAD S%\nPRINT\nLOAD E%\nPRINT\nPRINTNL\nEND\n\
         LABEL MAIN.H\nLINE 7\nLOAD E%\nPUSH I 1\nOP ADD\nSTORE E%\nRESUME NEXT\n",
        for_loop(1, "I%", "I 1", "I 20", "I 1", body)
    ));
    let body = "LOAD S&\nLOAD I%\nPUSH I 8\nOP MOD\nCALL FACT& 1\nOP ADD\nSTORE S&\nGOSUB G";
    v.push(format!(
        "QIR 0\n{}LOAD S&\nPRINT\nLOAD C&\nPRINT\nPRINTNL\nEND\nLABEL G\nLOAD C&\nPUSH I 1\nOP ADD\nSTORE C&\nRETURN\n\
         FUNCTION FACT& 1 N&\nLOAD N&\nPUSH I 1\nOP LE\nJZ E\nPUSH I 1\nSTORE FACT&\nJMP X\nLABEL E\n\
         LOAD N&\nLOAD N&\nPUSH I 1\nOP SUB\nCALL FACT& 1\nOP MUL\nSTORE FACT&\nLABEL X\nENDSUB\n",
        for_loop(1, "I%", "I 1", "I 30", "I 1", body)
    ));
    let body = "LOAD A$\nPUSH I 65\nLOAD I%\nPUSH I 26\nOP MOD\nOP ADD\nFN CHR$ 1\nOP ADD\nSTORE A$\nLOAD X!\nLOAD I%\nOP ADD\nSTORE X!";
    v.push(format!(
        "QIR 0\n{}LOAD A$\nPRINT\nLOAD X!\nPRINT\nPRINTNL\nEND\n",
        for_loop(1, "I%", "I 1", "I 60", "I 1", body)
    ));
    v
}

#[test]
fn native_image_matches_the_vm() {
    for p in programs() {
        let mut vm = Vec::new();
        let r1 = run_ir(&p, &mut vm).map_err(|e| (e.code, e.line));
        let img = build_native(&p).unwrap_or_else(|e| panic!("build: {e:?}\n{p}"));
        let mut nat = Vec::new();
        let r2 = run_ir_native(&p, &img, &mut nat).map(|_| ()).map_err(|e| (e.code, e.line));
        assert_eq!((String::from_utf8_lossy(&vm), r1), (String::from_utf8_lossy(&nat), r2), "{p}");
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
    let a = "QIR 0\nPUSH I 1\nPRINT\nPRINTNL\nEND\n";
    let b = "QIR 0\nPUSH I 2\nPRINT\nPRINTNL\nEND\n";
    let img = build_native(a).unwrap();
    let mut out = Vec::new();
    let e = run_ir_native(b, &img, &mut out).unwrap_err();
    assert!(e.msg.contains("another program"), "{e:?}");
}
