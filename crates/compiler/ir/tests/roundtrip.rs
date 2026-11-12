//! qb-ir: parse -> print -> parse round trip, validator diagnostics.

use qb_ir::{parse, print, validate, Inst, Lit, Ty};

const SAMPLE: &str = r#"QIR 1
; comment line
meta console only
type POINT 8
field X i32 0
field Y i32 4
endtype
global $TOTAL& i32
global $NAME$ str
global $P udt:POINT
global $FS fstr:10
global $A%() array i16
datalabel MYDATA
data x3132
data x
data x48656C6C6F
func main
local $I% i16
line 3
const.i32 %0 5
const.i16 %1 -7
const.f64 %2 0.1
const.f32 %3 1.5
const.f80 %4 1.25
const.str %5 x48690A
const.str %6 x
cvt.i16.i32 %7 %0
store.i16 $I% %7
load.i16 %8 $I%
cvt.i32.i16 %9 %8
add.i32 %10 %9 %0
mul.i32 %10 %10 %0
lt.i32 %11 %10 %0
jz %11 L1
cvt.i64.i32 %13 %0
rt PRINTN.i64 %0 %13
label L1
call %14 FACT %0
addr %15 $P
ld.i32 %16 %15 4
st.i32 %15 0 %16
ldfs %17 %15 0 4
stfs %15 0 4 %17
dim $A%() 1 %0 %0
aaddr %18 $A%() %0
ld.i16 %8 %18 0
lbound %19 $A%() %0
erase $A%()
gosub SUBR
onerror H
resume next
read.str %5
restore MYDATA
rt %20 LEFT$ %5 %0
concat %21 %5 %20
eq.str %22 %21 %5
end
label SUBR
gret
label H
resume next
endfunc
func FACT -> i32
param $N& i32 byval
const.i32 %0 1
load.i32 %1 $N&
le.i32 %2 %1 %0
jnz %2 BASE
sub.i32 %3 %1 %0
call %4 FACT %3
mul.i32 %5 %4 %1
ret %5
label BASE
ret %0
endfunc
"#;

#[test]
fn parses_sample() {
    let m = parse(SAMPLE).expect("parse");
    assert_eq!(m.funcs.len(), 2);
    assert_eq!(m.types[0].size, 8);
    assert_eq!(m.data.len(), 3);
    assert_eq!(m.data[0], b"12");
    assert_eq!(m.data_labels[0], ("MYDATA".to_string(), 0));
    let main = &m.funcs[0];
    assert_eq!(main.name, "main");
    assert!(main.body.iter().any(|i| matches!(i, Inst::Line(3))));
    assert!(main
        .body
        .iter()
        .any(|i| matches!(i, Inst::Const { ty: Ty::Str, val: Lit::Str(s), .. } if s == b"Hi\n")));
    assert_eq!(m.funcs[1].ret, Some(Ty::I32));
}

#[test]
fn round_trips() {
    let m = parse(SAMPLE).expect("parse");
    let text = print(&m);
    let m2 = parse(&text).expect("reparse");
    assert_eq!(m, m2);
    assert_eq!(text, print(&m2));
}

#[test]
fn sample_validates() {
    let m = parse(SAMPLE).unwrap();
    validate(&m).expect("valid");
}

#[test]
fn rejects_newer_version() {
    let e = parse("QIR 2\nfunc main\nend\nendfunc\n").unwrap_err();
    assert!(e.msg.contains("version"), "{}", e.msg);
}

#[test]
fn reports_line_of_parse_error() {
    let e = parse("QIR 1\nfunc main\nbogus %1\nendfunc\n").unwrap_err();
    assert_eq!(e.line, 3);
}

fn invalid(body: &str) -> String {
    let src = format!("QIR 1\nfunc main\n{body}\nendfunc\n");
    let m = parse(&src).expect("parse");
    validate(&m).unwrap_err().msg
}

#[test]
fn validator_catches_register_type_conflict() {
    assert!(invalid("const.i32 %0 1\nconst.f64 %0 1").contains("%0"));
}

#[test]
fn validator_catches_operand_type_mismatch() {
    assert!(invalid("const.i32 %0 1\nconst.f64 %1 1\nadd.i32 %2 %0 %1").contains("%1"));
}

#[test]
fn validator_catches_undefined_label() {
    assert!(invalid("jmp NOWHERE").contains("NOWHERE"));
}

#[test]
fn validator_catches_unknown_var() {
    assert!(invalid("const.i32 %0 1\nstore.i32 $X %0").contains("$X"));
}

#[test]
fn validator_catches_float_only_op() {
    assert!(invalid("const.i32 %0 1\ndiv.i32 %1 %0 %0").contains("div"));
}

#[test]
fn validator_checks_call_arity() {
    let src = "QIR 1\nfunc main\ncall F\nend\nendfunc\nfunc F\nparam $X i32 byval\nret\nendfunc\n";
    let e = validate(&parse(src).unwrap()).unwrap_err().msg;
    assert!(e.contains("F"), "{e}");
}

#[test]
fn validator_checks_builtin_signature() {
    assert!(invalid("const.i32 %0 1\nrt %1 LEFT$ %0 %0").contains("LEFT$"));
}

#[test]
fn validator_checks_field_end_within_type() {
    let udt = |fields: &str| format!("QIR 1\ntype T 12\n{fields}endtype\nfunc main\nend\nendfunc\n");
    // Fits exactly: i32 at 0, fstr:8 at 4, ends at 12.
    validate(&parse(&udt("field A i32 0\nfield B fstr:8 4\n")).unwrap()).expect("valid");
    // Offset inside, but the field runs past the end.
    let e = validate(&parse(&udt("field A i32 0\nfield B fstr:10 4\n")).unwrap()).unwrap_err().msg;
    assert!(e.contains("field B"), "{e}");
    let e = validate(&parse(&udt("field A f64 8\n")).unwrap()).unwrap_err().msg;
    assert!(e.contains("field A"), "{e}");
    // Nested UDT counts its full size.
    let src = "QIR 1\ntype P 8\nfield X i32 0\nfield Y i32 4\nendtype\ntype Q 12\nfield A i32 0\nfield B udt:P 4\nendtype\ntype R 10\nfield B udt:P 4\nendtype\nfunc main\nend\nendfunc\n";
    let e = validate(&parse(src).unwrap()).unwrap_err().msg;
    assert!(e.starts_with("R:"), "{e}");
}

#[test]
fn validator_requires_main() {
    let m = parse("QIR 1\nfunc other\nret\nendfunc\n").unwrap();
    assert!(validate(&m).unwrap_err().msg.contains("main"));
}

#[test]
fn byref_param_takes_ptr() {
    let src = "QIR 1\nfunc main\nlocal $X i32\naddr %0 $X\ncall S %0\nend\nendfunc\nfunc S\nparam $A i32 byref\nret\nendfunc\n";
    validate(&parse(src).unwrap()).expect("valid");
    let bad = "QIR 1\nfunc main\nconst.i32 %0 1\ncall S %0\nend\nendfunc\nfunc S\nparam $A i32 byref\nret\nendfunc\n";
    assert!(validate(&parse(bad).unwrap()).is_err());
}

#[test]
fn string_literal_bytes_survive() {
    let all: Vec<u8> = (0u8..=255).collect();
    let hex: String = all.iter().map(|b| format!("{b:02X}")).collect();
    let src = format!("QIR 1\nfunc main\nconst.str %0 x{hex}\nend\nendfunc\n");
    let m = parse(&src).unwrap();
    match &m.funcs[0].body[0] {
        Inst::Const { val: Lit::Str(s), .. } => assert_eq!(s, &all),
        other => panic!("{other:?}"),
    }
    assert_eq!(parse(&print(&m)).unwrap(), m);
}

#[test]
fn non_ascii_hex_float_is_error_not_panic() {
    // 20 bytes after `0x`, with a 2-byte char straddling byte 4.
    let lit = format!("0xaaa\u{e9}{}", "a".repeat(15));
    assert_eq!(lit.len(), 22);
    for ty in ["f80", "f64"] {
        let src = format!("QIR 1\nfunc main\nconst.{ty} %0 {lit}\nend\nendfunc\n");
        let e = parse(&src).unwrap_err();
        assert!(e.msg.contains("float"), "{}", e.msg);
    }
    assert!(qb_ir::f80_bits(&lit).is_none());
}

#[test]
fn float_literals_exact() {
    let src = "QIR 1\nfunc main\nconst.f64 %0 0x3FB999999999999A\nconst.f32 %1 0x3DCCCCCD\nconst.f64 %2 -inf\nend\nendfunc\n";
    let m = parse(src).unwrap();
    validate(&m).unwrap();
    assert_eq!(parse(&print(&m)).unwrap(), m);
    match &m.funcs[0].body[0] {
        Inst::Const { val: Lit::Float(t), .. } => assert_eq!(qb_ir::float_value(t).unwrap(), 0.1),
        o => panic!("{o:?}"),
    }
}
