//! QIR features beyond plain registers (u64, memory-backed variables,
//! _FLOAT): each program runs on the VM and must give the output recorded in
//! `tests/fixtures/cover/<test>.expect` (captured from the R11a reference
//! interpreter before it was retired). `QB_BLESS=1` rewrites the files.

use std::path::PathBuf;

fn run(src: &str) -> String {
    let mut b = Vec::new();
    let r = qb_exec::run_ir_status(src, &mut b);
    let o = String::from_utf8_lossy(&b).into_owned();
    match r {
        Ok(c) => format!("{o}<exit {c}>"),
        Err(e) => format!("{o}<error {} line {}>", e.code, e.line),
    }
}

fn same_full(src: &str) {
    let name = std::thread::current().name().unwrap_or("x").rsplit("::").next().unwrap().to_string();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cover").join(format!("{name}.expect"));
    let got = run(src);
    if std::env::var_os("QB_BLESS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &got).unwrap();
        return;
    }
    let want = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert_eq!(got, want, "{src}");
}

fn same(body: &str) {
    same_full(&format!("QIR 1\nfunc main\nconst.i32 %900 0\n{body}\nrt PRINTNL %900\nend\nendfunc\n"));
}

#[test]
fn unsigned_64_bit_arithmetic() {
    same(
        "const.u64 %0 18446744073709551615\nconst.u64 %1 3\nidiv.u64 %2 %0 %1\nrt PRINTN.u64 %900 %2\n\
         mod.u64 %3 %0 %1\nrt PRINTN.u64 %900 %3\nconst.u64 %4 1\nadd.u64 %5 %0 %4\nrt PRINTN.u64 %900 %5\n\
         gt.u64 %6 %0 %1\ncvt.i64.i32 %7 %6\nrt PRINTN.i64 %900 %7\nconst.u64 %8 60\nshr.u64 %9 %0 %8\nrt PRINTN.u64 %900 %9\n\
         cvt.f64.u64 %10 %0\nrt PRINTN.f64 %900 %10\nconst.f64 %11 1.5E+19\ncvt.u64.f64 %12 %11\nrt PRINTN.u64 %900 %12\n\
         rt %13 STR$.u64 %0\nrt PRINTS %900 %13",
    );
}

#[test]
fn unsigned_compare_branches() {
    same(
        "const.u64 %0 18446744073709551615\nconst.u64 %1 1\nlt.u64 %2 %1 %0\njz %2 NO\nconst.str %3 x6C74\nrt PRINTS %900 %3\nlabel NO",
    );
}

#[test]
fn udt_fields_byref_udt_and_udt_arrays() {
    same_full(
        "QIR 1
type PT 12
field X i32 0
field N fstr:8 4
endtype
global $P udt:PT
global $A array udt:PT
func main
const.i32 %0 0
addr %1 $P
const.i32 %2 41
st.i32 %1 0 %2
const.str %3 x6162
stfs %1 4 8 %3
call BUMP %1
ld.i32 %4 %1 0
cvt.i64.i32 %5 %4
rt PRINTN.i64 %0 %5
ldfs %6 %1 4 8
rt PRINTS %0 %6
const.i32 %7 1
const.i32 %8 3
dim $A 1 %7 %8
const.i32 %9 2
aaddr %10 $A %9
copy %10 %1 12
ld.i32 %11 %10 0
cvt.i64.i32 %12 %11
rt PRINTN.i64 %0 %12
const.i32 %13 3
aaddr %14 $A %13
const.i32 %15 4
padd %16 %14 %15
ldfs %17 %16 0 8
rt PRINTS %0 %17
rt PRINTNL %0
line 9
const.i32 %18 4
aaddr %19 $A %18
end
endfunc
func BUMP
param $Q udt:PT byref
addr %0 $Q
ld.i32 %1 %0 0
const.i32 %2 1
add.i32 %3 %1 %2
st.i32 %0 0 %3
ret
endfunc
",
    );
}

#[test]
fn fixed_length_strings_pad_and_truncate() {
    same_full(
        "QIR 1
global $F fstr:4
func main
const.i32 %0 0
const.str %1 x6162
store.str $F %1
load.str %2 $F
rt PRINTS %0 %2
const.str %3 x31323334353637
store.str $F %3
load.str %4 $F
rt PRINTS %0 %4
call SHOW
rt PRINTNL %0
end
endfunc
func SHOW
load.str %0 $F
const.i32 %1 0
rt PRINTS %1 %0
ret
endfunc
",
    );
}

#[test]
fn byref_scalars_in_memory_mode() {
    // a TYPE makes the module use memory mode; scalars passed BYREF are then real pointers
    same_full(
        "QIR 1
type T 4
field V i32 0
endtype
global $U udt:T
global $N i32
global $S str
func main
const.i32 %0 0
const.i32 %1 5
store.i32 $N %1
const.str %2 x61
store.str $S %2
addr %3 $N
addr %4 $S
call TWICE %3 %4
load.i32 %5 $N
cvt.i64.i32 %6 %5
rt PRINTN.i64 %0 %6
load.str %7 $S
rt PRINTS %0 %7
rt PRINTNL %0
end
endfunc
func TWICE
param $X i32 byref
param $Y str byref
load.i32 %0 $X
add.i32 %1 %0 %0
store.i32 $X %1
load.str %2 $Y
concat %3 %2 %2
store.str $Y %3
ret
endfunc
",
    );
}

#[test]
fn float80_arithmetic_is_bit_exact() {
    // 1/3, 2^0.5, compare, conversions, a _FLOAT global seen from a SUB
    same_full(
        "QIR 1
global $X f80
func main
const.i32 %0 0
const.f80 %1 1
const.f80 %2 3
div.f80 %3 %1 %2
store.f80 $X %3
rt %4 STR$.f80 %3
rt PRINTS %0 %4
const.f80 %5 2
const.f80 %6 0.5
pow.f80 %7 %5 %6
rt PRINTN.f80 %0 %7
lt.f80 %8 %3 %7
cvt.i64.i32 %9 %8
rt PRINTN.i64 %0 %9
cvt.f64.f80 %10 %3
rt PRINTN.f64 %0 %10
cvt.i64.f80 %11 %7
rt PRINTN.i64 %0 %11
neg.f80 %12 %7
cvt.f80.i32 %13 %0
sub.f80 %14 %13 %12
rt PRINTN.f80 %0 %14
call SHOW
rt PRINTNL %0
end
endfunc
func SHOW
load.f80 %0 $X
const.f80 %1 3
mul.f80 %2 %0 %1
const.i32 %3 0
rt PRINTN.f80 %3 %2
ret
endfunc
",
    );
}

#[test]
fn float80_byval_parameters_and_function_results() {
    same_full(
        "QIR 1
func main
const.i32 %0 0
const.f80 %1 1
const.f80 %2 7
div.f80 %3 %1 %2
const.i32 %4 2
call %5 TWICE %3 %4
rt PRINTN.f80 %0 %5
rt PRINTNL %0
end
endfunc
func TWICE -> f80
param $V f80 byval
param $N i32 byval
load.f80 %0 $V
load.i32 %1 $N
cvt.f80.i32 %2 %1
mul.f80 %3 %0 %2
ret %3
endfunc
",
    );
}
