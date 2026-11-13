//! `dims`: DIM of a static array. It is dimensioned once (a second `dims`, as
//! in a `SUB ... STATIC` called again, keeps it and its values) and ERASE only
//! clears it, keeping its dimensions (as in QBasic).

fn run(src: &str) -> String {
    let mut b = Vec::new();
    let r = qb_exec::run_ir_status(src, &mut b);
    let o = String::from_utf8_lossy(&b).into_owned();
    match r {
        Ok(c) => format!("{o}<exit {c}>"),
        Err(e) => format!("{o}<error {} line {}>", e.code, e.line),
    }
}

/// `dims $V` (0 TO 3), V(2) = 5, then `<between>`, then PRINT V(2)
fn prog(between: &str) -> String {
    format!(
        "QIR 1\nmeta console only\nglobal $V array i16\nfunc main\nline 1\nconst.i64 %0 0\nconst.i64 %1 3\ndims $V 1 %0 %1\nconst.i32 %2 2\naaddr %3 $V %2\nconst.i16 %4 5\nst.i16 %3 0 %4\nline 2\n{between}line 3\nconst.i32 %5 2\naaddr %6 $V %5\nld.i16 %7 %6 0\ncvt.i64.i16 %8 %7\nrt %9 STR$.i64 %8\nconst.i32 %10 0\nrt PRINTS %10 %9\nrt PRINTNL %10\nend\nendfunc\n"
    )
}

#[test]
fn a_second_dims_keeps_the_array_and_its_values() {
    assert_eq!(run(&prog("dims $V 1 %0 %1\n")), " 5\n<exit 0>");
}

#[test]
fn erase_of_a_static_array_clears_it_and_keeps_its_dimensions() {
    assert_eq!(run(&prog("erase $V\n")), " 0\n<exit 0>");
}

#[test]
fn dim_of_a_dimensioned_dynamic_array_is_still_a_duplicate_definition() {
    let src = prog("dim $V 1 %0 %1\n").replacen("dims $V", "dim $V", 1);
    assert_eq!(run(&src), "<error 10 line 2>");
}
