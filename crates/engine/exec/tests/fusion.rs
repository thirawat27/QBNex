//! Superinstructions for the QIR the real emitter writes (benches/qir/loop.qir
//! = `FOR i& = 1 TO n&: s& = s& + i& MOD 7: NEXT`).

use std::path::Path;

fn loop_qir(start: i32, n: i32, step: i32) -> String {
    let src = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("benches/qir/loop.qir")).unwrap();
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
    let m = qb_exec::compile(&src).unwrap();
    let d = m.disasm();
    assert!(d.contains("for_test.i +4"), "{d}");
    assert!(d.contains("loop_for.i32"), "{d}");
    assert!(d.contains("acc.i32 mod.i32"), "{d}");
    assert_eq!(first_number(&src), "27");
}

/// `FOR i& = start TO n STEP step: s& = s& + i& MOD 7: NEXT` by QBasic rules.
fn expected(start: i64, n: i64, step: i64) -> String {
    let (mut i, mut s) = (start, 0i64);
    while if step < 0 { i >= n } else { i <= n } {
        s = (s + i % 7) as i32 as i64;
        i += step;
    }
    s.to_string()
}

#[test]
fn negative_step_and_empty_loops() {
    for (start, n, step) in [(10, 1, -2), (5, 1, 1), (1, 1, -1), (-3, 3, 2), (1, 10, 3)] {
        let src = loop_qir(start as i32, n as i32, step as i32);
        assert_eq!(first_number(&src), expected(start, n, step), "FOR i = {start} TO {n} STEP {step}");
    }
}
