//! Loader and validator checks: fusions over a register shared by a widening
//! conversion, malformed QIR the loader refuses, bytecode the validator refuses.

use qb_exec::bytecode::Ins;

fn run(src: &str) -> String {
    let mut out = Vec::new();
    qb_exec::run_ir(src, &mut out).unwrap();
    String::from_utf8(out).unwrap()
}

/// `%5` (a widening cvt of `%4`) reuses `%4`'s VM register. The branch on `%5`
/// is its only use, but `%4` is read again afterwards: the compare must not be
/// fused into the branch (that left `%4`'s register unwritten).
#[test]
fn fused_branch_keeps_a_compare_read_through_a_widening_alias() {
    let src = "QIR 1\nmeta console only\nglobal $X& i32\nglobal $Y& i32\nglobal $B& i32\nfunc main\nline 1\n\
const.i32 %0 3\nstore.i32 $X& %0\nconst.i32 %1 5\nstore.i32 $Y& %1\n\
load.i32 %2 $X&\nload.i32 %3 $Y&\nlt.i32 %4 %2 %3\ncvt.i64.i32 %5 %4\njz %5 L1\nlabel L1\nstore.i32 $B& %4\n\
load.i32 %6 $B&\ncvt.i64.i32 %7 %6\nconst.i32 %8 0\nrt PRINTN.i64 %8 %7\nrt PRINTNL %8\nend\nendfunc\n";
    assert_eq!(run(src).trim(), "-1");
}

#[test]
fn sparse_register_numbers_are_refused() {
    let src = "QIR 1\nmeta console only\nfunc main\nconst.i32 %16000000 1\nend\nendfunc\n";
    let e = qb_exec::compile(src).unwrap_err();
    assert!(format!("{e:?}").contains("far above"), "{e:?}");
}

#[test]
fn memory_variable_index_is_validated() {
    // a TYPE variable: memory mode, storage made by `newv`
    let src = "QIR 1\nmeta console only\ntype P 4\nfield X i32 0\nendtype\nfunc main\nlocal $V udt:P\nline 1\nend\nendfunc\nfunc F\nlocal $W udt:P\nret\nendfunc\n";
    let mut m = qb_exec::compile(src).unwrap();
    m.validate().unwrap();
    let k = m.code.iter().position(|i| matches!(i, Ins::NewV { .. })).expect("newv");
    if let Ins::NewV { v, .. } = &mut m.code[k] {
        *v = 9999;
    }
    assert!(m.validate().unwrap_err().contains("memory variable"));
}
