//! The JIT tier must give exactly the VM's results. Every program in
//! `tests/fixtures/jit/` (BASIC source + the QIR `qb --ir` made of it) runs
//! interpreted only and with native code entered at the first loop back-edge
//! (hot and eager); output, exit status and error must match, and native code
//! must actually have run.

use std::path::Path;

use qb_exec::{compile, Vm};
use qb_jit::tier::Mode;

fn src(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/jit").join(format!("{name}.qir"));
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

#[derive(Debug, PartialEq)]
struct Run {
    out: String,
    result: Result<i32, (i32, u32)>,
    entries: u64,
}

fn run_mode(src: &str, mode: Mode) -> Run {
    let m = compile(src).unwrap_or_else(|e| panic!("load: {e:?}"));
    let mut out = Vec::new();
    let (result, entries, fail) = {
        let mut vm = Vm::new(&m, &mut out);
        vm.set_jit(mode, 1);
        let r = vm.run_status().map_err(|e| (e.code, e.line));
        (r, vm.jit_entries(), vm.jit_failure())
    };
    assert!(fail.is_none(), "JIT compile failed: {fail:?}");
    Run { out: String::from_utf8(out).unwrap(), result, entries }
}

/// Runs a fixture all three ways, checks equality and that native code ran.
fn same(name: &str) -> Run {
    let p = src(name);
    let vm = run_mode(&p, Mode::Off);
    let hot = run_mode(&p, Mode::Hot);
    let eager = run_mode(&p, Mode::Eager);
    assert_eq!((&vm.out, &vm.result), (&hot.out, &hot.result), "VM vs JIT (hot): {name}");
    assert_eq!((&vm.out, &vm.result), (&eager.out, &eager.result), "VM vs JIT (eager): {name}");
    assert!(eager.entries > 0, "native code never entered: {name}");
    vm
}

#[test]
fn numeric_loop() {
    let expect: i64 = (1..=100000).map(|i| i % 7).sum();
    assert_eq!(same("numeric_loop").out, format!(" {expect} \n"));
}

#[test]
fn integer_wrap_and_negative_step() {
    same("integer_wrap_negative_step");
}

#[test]
fn single_and_double_loop() {
    same("single_double_loop");
}

#[test]
fn strings_in_a_loop_go_through_the_vm() {
    let out = same("strings_in_loop").out;
    assert!(out.starts_with("BCD"), "{out}");
}

#[test]
fn arrays_in_a_loop() {
    assert_eq!(same("arrays_in_loop").out, " 338350 \n");
}

#[test]
fn error_inside_native_loop_resumes_next() {
    same("error_resume_next_in_loop");
}

#[test]
fn float_to_integer_store_wraps_like_the_cpp_backend() {
    same("unhandled_overflow_in_loop");
}

#[test]
fn sub_called_from_a_hot_loop() {
    assert_eq!(same("sub_from_hot_loop").out, " 2550 \n");
}

#[test]
fn print_inside_loop() {
    assert_eq!(same("print_in_loop").out, " 1 , 2 , 3 , 4 , 5 ,\n");
}

#[test]
fn array_bounds_errors_from_native_code() {
    same("array_bounds_errors");
}

#[test]
fn gosub_return_in_a_hot_loop() {
    let c: i64 = (1..=300).sum::<i64>() + (1..=300).filter(|i| i % 7 == 0).sum::<i64>();
    assert_eq!(same("gosub_return_in_loop").out, format!(" {c}  42 \n"));
}

#[test]
fn return_without_gosub_from_native_code() {
    assert_eq!(same("return_without_gosub").result.map_err(|e| e.0), Err(3));
}

#[test]
fn shared_variable_changed_by_sub_called_from_native_main() {
    same("shared_changed_by_sub");
}

#[test]
fn recursive_function_from_native_loop_and_end_inside_sub() {
    let out = same("recursion_and_end_in_sub").out;
    assert!(!out.contains("no"), "{out}");
}

#[test]
fn native_sub_with_byref_and_function_results() {
    let t: i64 = (1..=2000i64).map(|i| i * i % 1000).sum();
    assert_eq!(same("native_byref_and_function").out, format!(" 2000  {t} \n"));
}

#[test]
fn error_inside_native_callee_resumes_next_in_the_callee() {
    same("error_inside_callee");
}

#[test]
fn deep_native_recursion_hands_over_to_the_vm() {
    assert_eq!(same("deep_recursion").out, " 9000 \n");
}

#[test]
fn end_and_gosub_inside_native_callee() {
    let out = same("end_and_gosub_in_callee").out;
    assert!(!out.contains("no"), "{out}");
}

#[test]
fn byref_shared_variable_through_two_native_levels() {
    same("byref_shared_two_levels");
}

#[test]
fn return_from_native_code_to_a_gosub_the_vm_made() {
    assert_eq!(same("gosub_into_native_loop").out, " 500500  1 \n");
}

#[test]
fn gosub_entries_survive_a_vm_call_from_native_code() {
    same("gosub_calls_vm_sub");
}

#[test]
fn string_select_case_and_rt_builtins_in_native_code() {
    let b: i64 = (1..=3000i64).filter(|i| i % 3 == 1).sum();
    assert_eq!(same("select_string_in_loop").out, format!(" 1000  {b}  1000 \n"));
}
