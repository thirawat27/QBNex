//! Real calls through the qb-ffi thunks: in-process `extern "C"` functions with
//! every argument kind and enough arguments to spill to the stack, C runtime
//! and system library symbols found without a header, and a DLL / shared
//! object built from Rust source during the test.

use qb_ffi::{Library, Signature, Ty, Value, call, find_default_symbol};
use std::ffi::c_void;

fn sig(args: &[Ty], ret: Ty) -> Signature {
    Signature::new(args.to_vec(), ret)
}

extern "C" fn add_mixed(a: i8, b: u8, c: i16, d: u16, e: i32, f: u32, g: i64, h: u64, x: f32, y: f64) -> f64 {
    a as f64 + b as f64 + c as f64 + d as f64 + e as f64 + f as f64 + g as f64 + h as f64 + x as f64 + y
}

#[allow(clippy::too_many_arguments)]
extern "C" fn many(
    a1: i64, f1: f64, a2: i32, f2: f32, a3: i64, f3: f64, a4: i64, f4: f64, a5: i64, f5: f64, a6: i64, f6: f64,
    a7: i64, f7: f64, a8: i64, f8: f64, a9: i64, f9: f64, a10: i16, f10: f32,
) -> i64 {
    let ints = a1 + 2 * a2 as i64 + 3 * a3 + 4 * a4 + 5 * a5 + 6 * a6 + 7 * a7 + 8 * a8 + 9 * a9 + 10 * a10 as i64;
    let fl = f1 + 2.0 * f2 as f64 + 3.0 * f3 + 4.0 * f4 + 5.0 * f5 + 6.0 * f6 + 7.0 * f7 + 8.0 * f8 + 9.0 * f9 + 10.0 * f10 as f64;
    ints * 1000 + fl as i64
}

extern "C" fn ret_i8() -> i8 {
    -5
}
extern "C" fn ret_u16() -> u16 {
    65000
}
extern "C" fn ret_f32(x: f32) -> f32 {
    x * 2.0
}
extern "C" fn ret_i32(x: i32) -> i32 {
    x.wrapping_mul(3)
}
extern "C" fn byref(p: *mut i32, s: *const u8) -> i32 {
    unsafe {
        *p += 1;
        let mut n = 0;
        while *s.add(n) != 0 {
            n += 1;
        }
        n as i32
    }
}
extern "C" fn nothing(p: *mut i64) {
    unsafe { *p = 42 }
}
extern "C" fn sum_doubles(n: i32, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64, g: f64, h: f64, i: f64, j: f64) -> f64 {
    let v = [a, b, c, d, e, f, g, h, i, j];
    v[..n as usize].iter().sum()
}

fn ptr(f: *const ()) -> *const c_void {
    f as *const c_void
}

#[test]
fn every_scalar_kind_reaches_the_callee() {
    let s = sig(&[Ty::I8, Ty::U8, Ty::I16, Ty::U16, Ty::I32, Ty::U32, Ty::I64, Ty::U64, Ty::F32, Ty::F64], Ty::F64);
    let args = [
        Value::I(-1),
        Value::I(255),
        Value::I(-300),
        Value::I(60000),
        Value::I(-70000),
        Value::U(4_000_000_000),
        Value::I(-5_000_000_000),
        Value::U(6_000_000_000),
        Value::F32(0.5),
        Value::F64(0.25),
    ];
    let r = unsafe { call(ptr(add_mixed as *const ()), &s, &args) }.unwrap();
    let want = -1.0 + 255.0 - 300.0 + 60000.0 - 70000.0 + 4e9 - 5e9 + 6e9 + 0.5 + 0.25;
    assert_eq!(r, Value::F64(want));
}

#[test]
fn arguments_spill_to_the_stack_in_order() {
    let mut tys = Vec::new();
    for k in 0..10 {
        tys.push(if k == 1 { Ty::I32 } else if k == 9 { Ty::I16 } else { Ty::I64 });
        tys.push(if k == 1 || k == 9 { Ty::F32 } else { Ty::F64 });
    }
    let s = sig(&tys, Ty::I64);
    let mut args = Vec::new();
    for k in 1..=10i64 {
        args.push(Value::I(k * 11));
        args.push(if k == 2 || k == 10 { Value::F32(k as f32 * 0.5) } else { Value::F64(k as f64 * 0.5) });
    }
    let r = unsafe { call(ptr(many as *const ()), &s, &args) }.unwrap();
    let ints: i64 = (1..=10).map(|k| k * k * 11).sum();
    let fl: f64 = (1..=10).map(|k| (k * k) as f64 * 0.5).sum();
    assert_eq!(r, Value::I(ints * 1000 + fl as i64));
}

#[test]
fn return_values_are_truncated_and_extended_by_type() {
    unsafe {
        assert_eq!(call(ptr(ret_i8 as *const ()), &sig(&[], Ty::I8), &[]).unwrap(), Value::I(-5));
        assert_eq!(call(ptr(ret_u16 as *const ()), &sig(&[], Ty::U16), &[]).unwrap(), Value::U(65000));
        assert_eq!(call(ptr(ret_f32 as *const ()), &sig(&[Ty::F32], Ty::F32), &[Value::F32(1.25)]).unwrap(), Value::F32(2.5));
        assert_eq!(
            call(ptr(ret_i32 as *const ()), &sig(&[Ty::I32], Ty::I32), &[Value::I(0x7000_0000)]).unwrap(),
            Value::I(0x7000_0000i32.wrapping_mul(3) as i64)
        );
        let mut out = 0i64;
        let r = call(ptr(nothing as *const ()), &sig(&[Ty::Ptr], Ty::Void), &[Value::Ptr(&mut out as *mut i64 as usize)]);
        assert_eq!(r.unwrap(), Value::Void);
        assert_eq!(out, 42);
    }
}

#[test]
fn byref_and_strings_are_pointers() {
    let mut n = 41i32;
    let text = b"hello\0";
    let r = unsafe {
        call(
            ptr(byref as *const ()),
            &sig(&[Ty::Ptr, Ty::Ptr], Ty::I32),
            &[Value::Ptr(&mut n as *mut i32 as usize), Value::Ptr(text.as_ptr() as usize)],
        )
    }
    .unwrap();
    assert_eq!((r, n), (Value::I(5), 42));
}

#[test]
fn ten_doubles_use_registers_then_stack() {
    let mut tys = vec![Ty::I32];
    tys.extend(std::iter::repeat_n(Ty::F64, 10));
    let mut args = vec![Value::I(10)];
    args.extend((1..=10).map(|k| Value::F64(k as f64)));
    let r = unsafe { call(ptr(sum_doubles as *const ()), &sig(&tys, Ty::F64), &args) }.unwrap();
    assert_eq!(r, Value::F64(55.0));
}

#[test]
fn argument_count_mismatch_is_an_error() {
    let r = unsafe { call(ptr(ret_i8 as *const ()), &sig(&[Ty::I32], Ty::I8), &[]) };
    assert!(r.is_err());
}

#[test]
fn headerless_declare_library_finds_c_runtime_symbols() {
    // The compiler itself uses `DECLARE LIBRARY: FUNCTION getpid& ()`.
    let getpid = find_default_symbol("getpid").expect("getpid");
    let pid = unsafe { call(getpid, &sig(&[], Ty::I32), &[]) }.unwrap();
    assert_eq!(pid, Value::I(std::process::id() as i64));
    let strlen = find_default_symbol("strlen").expect("strlen");
    let s = b"QBNex\0";
    let n = unsafe { call(strlen, &sig(&[Ty::Ptr], Ty::U64), &[Value::Ptr(s.as_ptr() as usize)]) }.unwrap();
    assert_eq!(n, Value::U(5));
    assert!(find_default_symbol("qb_no_such_symbol_anywhere").is_none());
}

#[cfg(windows)]
#[test]
fn windows_system_functions() {
    let gcpi = find_default_symbol("GetCurrentProcessId").expect("kernel32");
    let pid = unsafe { call(gcpi, &sig(&[], Ty::U32), &[]) }.unwrap();
    assert_eq!(pid, Value::U(std::process::id() as u64));
    let user32 = Library::open_dynamic("user32", None).expect("user32");
    let beep = user32.symbol("MessageBeep").expect("MessageBeep");
    // MessageBeep(0xFFFFFFFF): simple beep; returns non-zero on success.
    let r = unsafe { call(beep, &sig(&[Ty::U32], Ty::I32), &[Value::U(0xFFFF_FFFF)]) }.unwrap();
    assert_ne!(r, Value::I(0));
}

#[cfg(unix)]
#[test]
fn unix_libc_by_name() {
    let libc = Library::open_dynamic("c:6", None).or_else(|_| Library::open_dynamic("System.B", None)).expect("libc");
    let getpid = libc.symbol("getpid").expect("getpid");
    let pid = unsafe { call(getpid, &sig(&[], Ty::I32), &[]) }.unwrap();
    assert_eq!(pid, Value::I(std::process::id() as i64));
}

/// Builds `tests/testlib.rs` into a DLL / shared object with rustc and calls it
/// through `DECLARE DYNAMIC LIBRARY` resolution.
#[test]
fn dynamic_library_built_from_rust() {
    let dir = std::env::temp_dir().join(format!("qb_ffi_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("testlib.rs");
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let status = std::process::Command::new(rustc)
        .args(["--crate-type", "cdylib", "--crate-name", "qbffitest", "-O", "--edition", "2024"])
        .arg(&src)
        .arg("--out-dir")
        .arg(&dir)
        // Cross runs (e.g. AArch64 under qemu): "--target aarch64-unknown-linux-gnu -C linker=...".
        .args(std::env::var("QB_FFI_TEST_RUSTC_ARGS").unwrap_or_default().split_whitespace())
        .status()
        .expect("run rustc");
    assert!(status.success());
    let lib = Library::open_dynamic(&format!("{}/qbffitest", dir.display()), None).expect("open test library");
    let mix = lib.symbol("qb_test_mix").expect("qb_test_mix");
    let s = sig(&[Ty::I16, Ty::F64, Ty::I64, Ty::F32, Ty::Ptr, Ty::U8, Ty::I32, Ty::F64, Ty::I64], Ty::F64);
    let text = b"abc\0";
    let args = [
        Value::I(-2),
        Value::F64(1.5),
        Value::I(1 << 40),
        Value::F32(0.25),
        Value::Ptr(text.as_ptr() as usize),
        Value::I(200),
        Value::I(-7),
        Value::F64(100.0),
        Value::I(9),
    ];
    let r = unsafe { call(mix, &s, &args) }.unwrap();
    let want = -2.0 + 1.5 + (1u64 << 40) as f64 + 0.25 + 3.0 + 200.0 - 7.0 + 100.0 + 9.0;
    assert_eq!(r, Value::F64(want));
    let neg = lib.symbol("qb_test_neg_i16").expect("neg");
    assert_eq!(unsafe { call(neg, &sig(&[Ty::I16], Ty::I16), &[Value::I(1234)]) }.unwrap(), Value::I(-1234));
    assert!(lib.symbol("qb_test_missing").is_none());
    drop(lib);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_library_reports_the_paths_tried() {
    let e = Library::open_dynamic("qb_definitely_missing_lib", None).expect_err("error");
    let text = e.to_string();
    assert!(text.contains("qb_definitely_missing_lib"), "{text}");
}
