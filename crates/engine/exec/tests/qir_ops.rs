//! QIR instructions and builtins end to end (`qb_exec::run_ir`).

fn run(body: &str) -> String {
    run_full(&format!("QIR 1\nfunc main\n{body}\nend\nendfunc\n"))
}

fn run_full(src: &str) -> String {
    let mut out = Vec::new();
    match qb_exec::run_ir(src, &mut out) {
        Ok(()) => String::from_utf8_lossy(&out).into_owned(),
        Err(e) => format!("{}ERR {} line {}: {}", String::from_utf8_lossy(&out), e.code, e.line, e.msg),
    }
}

#[test]
fn prints_numbers_and_strings() {
    let o = run("const.i32 %0 0\nconst.i64 %1 -5\nrt PRINTN.i64 %0 %1\nconst.f32 %2 1.5\nrt PRINTN.f32 %0 %2\nconst.str %3 x6869\nrt PRINTS %0 %3\nrt PRINTNL %0");
    assert_eq!(o, "-5  1.5 hi\n");
}

#[test]
fn int_arith_wraps_by_type() {
    let o = run("const.i16 %0 32767\nconst.i16 %1 1\nadd.i16 %2 %0 %1\ncvt.i64.i16 %3 %2\nconst.i32 %9 0\nrt PRINTN.i64 %9 %3\nrt PRINTNL %9");
    assert_eq!(o, "-32768 \n");
}

#[test]
fn cvt_rounds_half_even_trunc_truncates() {
    let o = run("const.f64 %0 2.5\ncvt.i64.f64 %1 %0\ntrunc.i64.f64 %2 %0\nconst.f64 %3 3.5\ncvt.i64.f64 %4 %3\nconst.i32 %9 0\nrt PRINTN.i64 %9 %1\nrt PRINTN.i64 %9 %2\nrt PRINTN.i64 %9 %4\nrt PRINTNL %9");
    assert_eq!(o, " 2  2  4 \n");
}

#[test]
fn idiv_by_zero_is_error_11() {
    let o = run("line 7\nconst.i32 %0 1\nconst.i32 %1 0\nidiv.i32 %2 %0 %1");
    assert!(o.contains("ERR 11 line 7"), "{o}");
}

#[test]
fn compare_gives_minus_one() {
    let o = run("const.str %0 x61\nconst.str %1 x62\nlt.str %2 %0 %1\ncvt.i64.i32 %3 %2\nconst.i32 %9 0\nrt PRINTN.i64 %9 %3\nrt PRINTNL %9");
    assert_eq!(o, "-1 \n");
}

#[test]
fn globals_byref_and_recursion() {
    let src = "QIR 1
global $X i32
func main
const.i32 %0 5
store.i32 $X %0
addr %1 $X
call INC %1
load.i32 %2 $X
cvt.i64.i32 %3 %2
const.i32 %9 0
rt PRINTN.i64 %9 %3
const.i32 %4 10
call %5 FACT %4
cvt.i64.i32 %6 %5
rt PRINTN.i64 %9 %6
rt PRINTNL %9
end
endfunc
func INC
param $A i32 byref
load.i32 %0 $A
const.i32 %1 1
add.i32 %2 %0 %1
store.i32 $A %2
ret
endfunc
func FACT -> i32
param $N i32 byval
local $R i32
load.i32 %0 $N
const.i32 %1 1
le.i32 %2 %0 %1
jz %2 REC
ret %1
label REC
sub.i32 %3 %0 %1
call %4 FACT %3
mul.i32 %5 %4 %0
ret %5
endfunc
";
    assert_eq!(run_full(src), " 6  3628800 \n");
}

#[test]
fn arrays_dim_bounds_and_error_9() {
    let src = "QIR 1
global $A array str
func main
const.i32 %0 1
const.i32 %1 3
dim $A 1 %0 %1
const.i32 %2 2
aaddr %3 $A %2
const.str %4 x4F4B
st.str %3 0 %4
ld.str %5 %3 0
const.i32 %9 0
rt PRINTS %9 %5
ubound %6 $A %0
cvt.i64.i32 %7 %6
rt PRINTN.i64 %9 %7
rt PRINTNL %9
line 9
const.i32 %8 4
aaddr %3 $A %8
end
endfunc
";
    let o = run_full(src);
    assert!(o.starts_with("OK 3 \nERR 9 line 9"), "{o}");
}

#[test]
fn udt_fields_and_fixed_strings() {
    let src = "QIR 1
type T 14
field N i32 0
field S fstr:10 4
endtype
global $V udt:T
global $F fstr:3
func main
addr %0 $V
const.i32 %1 42
st.i32 %0 0 %1
const.str %2 x6162
stfs %0 4 10 %2
ld.i32 %3 %0 0
ldfs %4 %0 4 10
const.i32 %9 0
cvt.i64.i32 %5 %3
rt PRINTN.i64 %9 %5
rt PRINTS %9 %4
const.str %6 x78797A77
store.str $F %6
load.str %7 $F
rt PRINTS %9 %7
rt PRINTNL %9
end
endfunc
";
    assert_eq!(run_full(src), " 42 ab        xyz\n");
}

#[test]
fn gosub_return_and_error_3() {
    let o = run("const.i32 %9 0\ngosub S\nconst.str %0 x42\nrt PRINTS %9 %0\nrt PRINTNL %9\nline 4\ngret\nlabel S\nconst.str %1 x41\nrt PRINTS %9 %1\ngret");
    assert!(o.starts_with("AB\nERR 3 line 4"), "{o}");
}

#[test]
fn on_error_in_sub_resume_next_returns_to_sub() {
    let src = "QIR 1
func main
onerror H
call S
const.i32 %9 0
const.str %0 x656E64
rt PRINTS %9 %0
rt PRINTNL %9
end
label H
const.i32 %1 0
rt %2 ERR
cvt.i64.i32 %3 %2
rt PRINTN.i64 %1 %3
resume next
endfunc
func S
line 20
const.i32 %0 5
error %0
line 21
const.i32 %9 0
const.str %1 x696E
rt PRINTS %9 %1
ret
endfunc
";
    assert_eq!(run_full(src), " 5 inend\n");
}

#[test]
fn data_read_restore() {
    let src = "QIR 1
data x3132
datalabel L2
data x2D332E35
data x6869
func main
const.i32 %9 0
read.i32 %0
read.f64 %1
read.str %2
restore L2
read.f64 %3
cvt.i64.i32 %4 %0
rt PRINTN.i64 %9 %4
rt PRINTN.f64 %9 %1
rt PRINTS %9 %2
rt PRINTN.f64 %9 %3
rt PRINTNL %9
end
endfunc
";
    assert_eq!(run_full(src), " 12 -3.5 hi-3.5 \n");
}

#[test]
fn string_builtins() {
    let o = run("const.str %0 x48656C6C6F\nconst.i32 %1 2\nrt %2 LEFT$ %0 %1\nrt %3 MID$.n %0 %1 %1\nconcat %4 %2 %3\nrt %5 LEN %4\nconst.i32 %9 0\nrt PRINTS %9 %4\ncvt.i64.i32 %6 %5\nrt PRINTN.i64 %9 %6\nrt PRINTNL %9");
    assert_eq!(o, "Heel 4 \n");
}

#[test]
fn print_zone() {
    let o = run("const.i32 %9 0\nconst.str %0 x41\nrt PRINTS %9 %0\nrt PRINTZONE %9\nrt PRINTS %9 %0\nrt PRINTNL %9");
    assert_eq!(o, "A             A\n");
}

/// REDIM _PRESERVE of a 2-D array keeps each element's column-major position
/// (QB64 keeps the bytes; first subscript fastest), in register mode (LONG
/// array) and memory mode (TYPE array) alike:
/// `REDIM a(1 TO 2, 1 TO 2): a(2,1)=21: a(1,2)=12: a(2,2)=22: REDIM _PRESERVE a(1 TO 2, 1 TO 3)`.
#[test]
fn redim_preserve_2d_keeps_column_major_positions() {
    let prog = |decl: &str| {
        // a TYPE puts the program in memory mode
        let ty = if decl.starts_with("udt") { "type T 4\nfield N i32 0\nendtype\n" } else { "" };
        let mut s = format!("QIR 1\n{ty}global $A array {decl}\nfunc main\n");
        s += "const.i32 %0 1\nconst.i32 %1 2\nconst.i32 %2 3\nconst.i32 %9 0\nredim $A 2 %0 %1 %0 %1\n";
        for (k, (i, j, v)) in [(1, 0, 21), (0, 1, 12), (1, 1, 22)].into_iter().enumerate() {
            let r = 10 + 2 * k;
            s += &format!("const.i32 %{r} {v}\naaddr %{} $A %{i} %{j}\nst.i32 %{} 0 %{r}\n", r + 1, r + 1);
        }
        s += "redimp $A 2 %0 %1 %0 %2\n";
        for (k, (i, j)) in [(0, 0), (1, 0), (0, 1), (1, 1), (0, 2), (1, 2)].into_iter().enumerate() {
            let r = 20 + 3 * k;
            s += &format!("aaddr %{r} $A %{i} %{j}\nld.i32 %{} %{r} 0\ncvt.i64.i32 %{} %{}\nrt PRINTN.i64 %9 %{}\n", r + 1, r + 2, r + 1, r + 2);
        }
        s + "rt PRINTNL %9\nend\nendfunc\n"
    };
    let expect = " 0  21  12  22  0  0 \n";
    assert_eq!(run_full(&prog("i32")), expect, "register array");
    assert_eq!(run_full(&prog("udt:T")), expect, "memory-mode array");
}

#[test]
fn huge_spc_and_tab_take_the_line_width_mod() {
    // SPC(1E12): 1E12 MOD 80 = 0 spaces; TAB(1E12 + 5) -> column 5 (no abort)
    let o = run("const.i32 %9 0\nconst.i64 %0 1000000000000\nrt PRINTSPC %9 %0\nconst.str %1 x41\nrt PRINTS %9 %1\nconst.i64 %2 1000000000005\nrt PRINTTAB %9 %2\nrt PRINTS %9 %1\nrt PRINTNL %9");
    assert_eq!(o, "A   A\n");
}

#[test]
fn inflate_output_over_the_cap_is_error_7() {
    let o = run("line 3\nconst.str %0 x789C\nconst.i64 %1 10000000000\nrt %2 _INFLATE$.n %0 %1");
    assert!(o.contains("ERR 7 line 3"), "{o}");
}

#[test]
fn print_zone_wraps_when_no_zone_is_left() {
    // Five zones fill columns 1..70; a comma after column 57 (no whole 14-column
    // zone left in 80 columns) starts a new line, as in QBasic and qb-runtime.
    let mut src = String::from("const.i32 %9 0\nconst.str %0 x41\n");
    for _ in 0..5 {
        src.push_str("rt PRINTS %9 %0\nrt PRINTZONE %9\n");
    }
    src.push_str("rt PRINTS %9 %0\nrt PRINTNL %9");
    let o = run(&src);
    let lines: Vec<&str> = o.split('\n').collect();
    assert_eq!(lines[0], format!("A{0}A{0}A{0}A{0}A", " ".repeat(13)));
    assert_eq!(lines[1], "A");
}
#[test]
fn erl_is_last_numeric_label_errorline_is_source_line() {
    let src = "QIR 1
func main
onerror H
rt LASTLINE %9
line 7
const.i32 %0 6
error %0
end
label H
const.i32 %1 0
rt %2 ERL
cvt.i64.i32 %3 %2
rt PRINTN.i64 %1 %3
rt %4 _ERRORLINE
cvt.i64.i32 %5 %4
rt PRINTN.i64 %1 %5
rt PRINTNL %1
end
endfunc
";
    let src = src.replace("rt LASTLINE %9", "const.i32 %9 40\nrt LASTLINE %9");
    assert_eq!(run_full(&src), " 40  7 \n");
}

#[test]
fn float80_is_exact_and_rounds_like_qbr() {
    let o = run("const.f80 %0 9223372036854775807\ncvt.i64.f80 %1 %0\nconst.i32 %9 0\nrt PRINTN.i64 %9 %1\nconst.f80 %2 4611686018427387905.5\ncvt.i64.f80 %3 %2\nrt PRINTN.i64 %9 %3\nrt PRINTNL %9");
    assert_eq!(o, " 9223372036854775807  4611686018427387906 \n");
}