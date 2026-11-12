//! COMMON / CHAIN in register mode: `CHAINGET` / `CHAINPUT` and their array
//! forms take the variable itself (`chain.get` / `chain.put`), so a program
//! with COMMON variables and arrays but nothing else that needs addresses is
//! not lowered in memory mode. The values arrive in the bytes memory mode
//! uses, so either kind of program can CHAIN to the other.

use qb_exec::bytecode::Ins;

/// COMMON n%, s$, a&() with a&() dimensioned 1 TO 3 first; prints them.
const PROG: &str = r#"QIR 1
meta console only
global $N i16
global $S str
global $A array i32
func main
line 1
const.i32 %0 1
const.i32 %1 3
redim $A 1 %0 %1
addr %2 $N
const.i32 %3 2
rt CHAINGET %2 %3
addr %4 $S
rt CHAINGETS %4
aref %5 $A
const.i32 %6 4
rt CHAINGETA %5 %6
line 2
const.i32 %10 0
load.i16 %11 $N
cvt.i64.i16 %12 %11
rt %13 STR$.i64 %12
rt PRINTS %10 %13
rt PRINTNL %10
load.str %14 $S
rt PRINTS %10 %14
rt PRINTNL %10
const.i32 %15 1
lbound %16 $A %15
cvt.i64.i32 %17 %16
rt %18 STR$.i64 %17
rt PRINTS %10 %18
rt PRINTNL %10
ubound %19 $A %15
cvt.i64.i32 %20 %19
rt %21 STR$.i64 %20
rt PRINTS %10 %21
rt PRINTNL %10
const.i32 %22 2
aaddr %23 $A %22
ld.i32 %24 %23 0
cvt.i64.i32 %25 %24
rt %26 STR$.i64 %25
rt PRINTS %10 %26
rt PRINTNL %10
const.i32 %27 3
aaddr %28 $A %27
ld.i32 %29 %28 0
cvt.i64.i32 %30 %29
rt %31 STR$.i64 %30
rt PRINTS %10 %31
rt PRINTNL %10
line 3
aref %32 $A
rt CHAINPUTA %32 %6
addr %33 $N
rt CHAINPUT %33 %3
end
endfunc
"#;

fn item(out: &mut Vec<u8>, kind: u8, b: &[u8]) {
    out.push(kind);
    out.extend_from_slice(&(b.len() as u32).to_le_bytes());
    out.extend_from_slice(b);
}

#[test]
fn common_variables_and_arrays_stay_in_register_mode_and_take_the_passed_values() {
    let m = qb_exec::compile(PROG).unwrap_or_else(|e| panic!("{}", e.msg));
    assert!(m.mvars.is_empty(), "lowered in memory mode");
    let chains = m.code.iter().filter(|i| matches!(i, Ins::Chain { .. })).count();
    assert_eq!(chains, 5, "COMMON / CHAIN instructions");

    // what a program CHAINing here passes: -5 (INTEGER), "abc", and a LONG array 2 TO 3
    let mut data = Vec::new();
    item(&mut data, 1, &(-5i16).to_le_bytes());
    item(&mut data, 2, b"abc");
    let mut arr = Vec::new();
    arr.extend_from_slice(&4u32.to_le_bytes());
    arr.extend_from_slice(&1u32.to_le_bytes());
    arr.extend_from_slice(&2i64.to_le_bytes());
    arr.extend_from_slice(&3i64.to_le_bytes());
    arr.extend_from_slice(&70i32.to_le_bytes());
    arr.extend_from_slice(&(-80i32).to_le_bytes());
    item(&mut data, 3, &arr);
    let file = std::env::temp_dir().join(format!("qbnex-chain-test-{}.bin", std::process::id()));
    std::fs::write(&file, &data).unwrap();
    // SAFETY: the only test of this test binary, nothing else reads the environment meanwhile
    unsafe { std::env::set_var("QBNEX_CHAIN", &file) };
    let mut out = Vec::new();
    let r = qb_exec::run_ir_status(PROG, &mut out);
    let text = String::from_utf8_lossy(&out).into_owned();
    assert_eq!(r.map_err(|e| e.msg).unwrap(), 0, "{text}");
    assert_eq!(text, "-5\nabc\n 2\n 3\n 70\n-80\n");
    assert!(!file.exists(), "the file of COMMON values is removed once read");
}
