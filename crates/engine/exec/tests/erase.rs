//! ERASE of a dynamic array: the array has no dimensions afterwards, so an
//! element access raises "Subscript out of range" (9) like the C++ backend,
//! instead of implicitly dimensioning it 0 TO 10 (lang3 of tests/e2e.rs).

fn run(src: &str) -> String {
    let mut b = Vec::new();
    let r = qb_exec::run_ir_status(src, &mut b);
    let o = String::from_utf8_lossy(&b).into_owned();
    match r {
        Ok(c) => format!("{o}<exit {c}>"),
        Err(e) => format!("{o}<error {} line {}>", e.code, e.line),
    }
}

const PROG: &str = r#"QIR 1
meta console only
global $V array i16
func main
line 2
const.i32 %0 1
const.i32 %1 3
redim $V 1 %0 %1
line 3
erase $V
line 4
onerror H
line 5
const.i32 %6 1
aaddr %7 $V %6
ld.i16 %8 %7 0
cvt.i64.i16 %9 %8
rt %10 STR$.i64 %9
const.i32 %13 0
rt PRINTS %13 %10
rt PRINTNL %13
line 6
const.str %15 x736B6970
rt PRINTS %13 %15
rt PRINTNL %13
line 7
end
label H
line 8
rt %21 ERR
cvt.i64.i32 %22 %21
rt %23 STR$.i64 %22
const.i32 %26 0
rt PRINTS %26 %23
rt PRINTNL %26
line 9
resume next
endfunc
"#;

#[test]
fn element_of_an_erased_array_is_subscript_out_of_range() {
    assert_eq!(run(PROG), " 9\nskip\n<exit 0>");
}

#[test]
fn erased_array_can_be_redimensioned() {
    let src = PROG.replace("line 3\nerase $V\n", "line 3\nerase $V\nconst.i32 %30 1\nconst.i32 %31 2\nredim $V 1 %30 %31\n");
    assert_eq!(run(&src), " 0\nskip\n<exit 0>");
}
