//! DECLARE [DYNAMIC] LIBRARY "name": the emitter writes the library spec into
//! `meta extfn <name> <lib> ...` (hex text, `-` for none) and the runner
//! opens that library (qb-ffi, the legacy file names: name.dll, libname.so,
//! libname.dylib) before falling back to the default system libraries.
//! The libraries used here are not in the default list, so the call only
//! works when the named library is opened.

fn run(src: &str) -> String {
    let mut b = Vec::new();
    let r = qb_exec::run_ir_status(src, &mut b);
    let o = String::from_utf8_lossy(&b).into_owned();
    match r {
        Ok(c) => format!("{o}<exit {c}>"),
        Err(e) => format!("{o}<error {} line {}>", e.code, e.line),
    }
}

fn hex(s: &str) -> String {
    let mut h = String::from("x");
    for b in s.bytes() {
        h.push_str(&format!("{b:02X}"));
    }
    h
}

/// `PRINT <name>(<arg>)` for a function of `lib` returning `ret`, printed with `print`.
fn prog(lib: &str, name: &str, ret: &str, args: &str, call_args: &str, print: &str) -> String {
    format!(
        "QIR 1\nmeta console only\nmeta extfn {name} {lib} {ret}{args}\nfunc main\nline 1\n{call_args}call %1 {name}{regs}\n{print}const.i32 %9 0\nrt PRINTS %9 %3\nrt PRINTNL %9\nend\nendfunc\n",
        regs = if args.is_empty() { "" } else { " %0" },
    )
}

#[cfg(windows)]
#[test]
fn a_function_of_a_named_library_is_called() {
    // shlwapi.dll is not one of the default modules (kernel32, user32, ...)
    let src = prog(
        &hex("shlwapi"),
        "StrToIntA",
        "i32",
        " str",
        "const.str %0 x3132333400\n",
        "cvt.i64.i32 %2 %1\nrt %3 STR$.i64 %2\n",
    );
    assert_eq!(run(&src), " 1234\n<exit 0>");
}

#[cfg(unix)]
#[test]
fn a_function_of_a_named_library_is_called() {
    // libz is not one of the default modules (libc, libm, libdl, libpthread, libSystem)
    let src = prog(&hex("z"), "zlibVersion", "str", "", "", "rt %3 LEFT$ %1 %8\n").replace("line 1\n", "line 1\nconst.i32 %8 1\n");
    assert_eq!(run(&src), "1\n<exit 0>");
}

/// An empty BYVAL STRING argument points at a zero byte, not at the dangling
/// pointer of an empty buffer: `strlen("")` (C runtime, default modules) is 0.
#[test]
fn an_empty_string_argument_points_at_a_zero_byte() {
    let src = prog("-", "strlen", "i64", " str", "const.str %0 x\n", "rt %3 STR$.i64 %1\n");
    assert_eq!(run(&src), " 0\n<exit 0>");
}

#[test]
fn a_missing_function_is_error_453() {
    let src = prog(&hex("qb_no_such_library"), "qb_no_such_function", "i32", "", "", "cvt.i64.i32 %2 %1\nrt %3 STR$.i64 %2\n");
    assert_eq!(run(&src), "<error 453 line 1>");
}
