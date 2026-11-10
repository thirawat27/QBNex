//! Superinstructions for the QIR v1 the real emitter writes (benches/qir1/loop.qir
//! = `FOR i& = 1 TO n&: s& = s& + i& MOD 7: NEXT`).

use std::path::Path;

fn loop_qir(start: i32, n: i32, step: i32) -> String {
    let src = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("benches/qir1/loop.qir")).unwrap();
    let src = src.replace("\r\n", "\n");
    assert!(src.contains("const.i32 %3 1\n") && src.contains("const.i32 %7 1\n"));
    src.replace("const.i32 %0 50000000", &format!("const.i32 %0 {n}"))
        .replace("const.i32 %3 1\n", &format!("const.i32 %3 {start}\n"))
        .replace("const.i32 %7 1\n", &format!("const.i32 %7 {step}\n"))
}

fn first_number(src: &str) -> String {
    let mut out = Vec::new();
    qb_exec::run_ir(src, &mut out).unwrap();
    String::from_utf8(out).unwrap().split_whitespace().next().unwrap().to_string()
}

#[test]
fn emitter_for_loop_is_two_instructions_per_iteration() {
    let src = loop_qir(1, 10, 1);
    let m = qb_exec::compile_v1(&qb_ir::parse(&src).unwrap()).unwrap();
    let d = m.disasm();
    assert!(d.contains("for_test.i +4"), "{d}");
    assert!(d.contains("loop_for.i32"), "{d}");
    assert!(d.contains("acc.i32 mod.i32"), "{d}");
    assert_eq!(first_number(&src), "27");
}

#[test]
fn negative_step_and_empty_loops_match_the_reference() {
    for (start, n, step) in [(10, 1, -2), (5, 1, 1), (1, 1, -1), (-3, 3, 2)] {
        let src = loop_qir(start, n, step);
        let mut a = Vec::new();
        qb_exec::v1::run(&qb_ir::parse(&src).unwrap(), &mut a).unwrap();
        let reference = String::from_utf8(a).unwrap().split_whitespace().next().unwrap().to_string();
        assert_eq!(first_number(&src), reference, "FOR i = {start} TO {n} STEP {step}");
    }
}
