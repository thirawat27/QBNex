//! QIR v1 features that need more than registers (u64, memory-backed
//! variables): each program must run on the typed VM and give exactly what
//! the R11a reference interpreter gives.

fn both(src: &str) -> (String, String) {
    let m = qb_ir::parse(src).unwrap();
    assert!(qb_exec::v1_on_vm(&m), "not on the VM: {:?}", qb_exec::compile_v1(&m).err());
    let show = |r: Result<i32, qb_exec::RunError>, o: Vec<u8>| {
        let o = String::from_utf8_lossy(&o).into_owned();
        match r {
            Ok(c) => format!("{o}<exit {c}>"),
            Err(e) => format!("{o}<error {} line {}>", e.code, e.line),
        }
    };
    let mut a = Vec::new();
    let ra = qb_exec::v1::run(&m, &mut a);
    let mut b = Vec::new();
    let rb = qb_exec::run_ir_status(src, &mut b);
    (show(ra, a), show(rb, b))
}

fn same(body: &str) {
    let src = format!("QIR 1\nfunc main\nconst.i32 %900 0\n{body}\nrt PRINTNL %900\nend\nendfunc\n");
    let (reference, vm) = both(&src);
    assert_eq!(vm, reference, "{src}");
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
