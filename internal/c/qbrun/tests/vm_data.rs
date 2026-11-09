//! RND/RANDOMIZE state, DATA/READ/RESTORE, MKx$/CVx and MBF conversions.
//! Expected values come from conformance golden files (legacy C++ runtime).

use qbrun::{run_ir, RunError};

fn hex(s: &str) -> String {
    s.bytes().map(|b| format!("{b:02X}")).collect()
}

fn run(src: &str) -> String {
    let mut out = Vec::new();
    run_ir(src, &mut out).unwrap_or_else(|e| panic!("run failed: {e:?}"));
    String::from_utf8(out).unwrap()
}

fn run_err(src: &str) -> RunError {
    let mut out = Vec::new();
    run_ir(src, &mut out).expect_err("expected runtime error")
}

fn prog(body: &str) -> String {
    format!("QIR 0\n{body}\nEND\n")
}

#[test]
fn rnd_sequence_matches_legacy() {
    // conformance/cases/numeric/rnd_sequence.bas lines 3-7, 8-9
    let p = prog(
        "FN RND 0\nPRINT\nPRINTNL\nFN RND 0\nPRINT\nPRINTNL\nPUSH I 0\nFN RND 1\nPRINT\nPRINTNL\n\
         PUSH I -1\nFN RND 1\nPRINT\nPRINTNL\nFN RND 0\nPRINT\nPRINTNL\n\
         PUSH I 42\nRANDOMIZE 0\nFN RND 0\nPRINT\nPRINTNL\nPUSH I 42\nRANDOMIZE 1\nFN RND 0\nPRINT\nPRINTNL",
    );
    assert_eq!(run(&p), " .7055475 \n .533424 \n .533424 \n .224007 \n 3.584582E-02 \n .8157225 \n .3391077 \n");
}

#[test]
fn data_read_restore() {
    // DATA 1,"one",2,two : READ N!, S$ (x2) : RESTORE : READ N!
    let p = format!(
        "QIR 0\nREAD 0\nSTORE N!\nREAD 1\nSTORE S$\nLOAD N!\nPRINT\nLOAD S$\nPRINT\nREAD 0\nSTORE N!\nREAD 1\nSTORE S$\n\
         LOAD N!\nPRINT\nLOAD S$\nPRINT\nRESTORE\nREAD 0\nPRINT\nPRINTNL\nEND\nDATA {}\nDATA {}\nLABEL SECOND\nDATA {}\nDATA {}\n",
        hex("1"),
        hex("one"),
        hex("2"),
        hex("two")
    );
    assert_eq!(run(&p), " 1 one 2 two 1 \n");
}

#[test]
fn restore_label_and_empty_item() {
    let p = format!("QIR 0\nRESTORE SECOND\nREAD 0\nSTORE Q!\nLOAD Q!\nPRINT\nREAD 1\nPRINT\nPRINTNL\nEND\nDATA {}\nLABEL SECOND\nDATA {}\nDATA\n", hex("5"), hex("99"));
    assert_eq!(run(&p), " 99 \n");
}

#[test]
fn out_of_data_is_error_4() {
    let e = run_err(&prog("LINE 2\nREAD 0\nPRINT"));
    assert_eq!((e.code, e.line), (4, 2));
}

fn bytes_of(expr: &str) -> String {
    format!("{expr}\nSTORE B$\nPUSH I 1\nSTORE K%\nLABEL ~L{n}\nLOAD B$\nLOAD K%\nFN ASC 2\nFN HEX$ 1\nPRINT\nLOAD K%\nPUSH I 1\nOP ADD\nSTORE K%\nLOAD K%\nLOAD B$\nFN LEN 1\nOP LE\nJNZ ~L{n}\n", n = expr.len())
}

#[test]
fn mk_and_cv_round_trip() {
    let mut b = bytes_of("PUSH I -2\nFN MKI$ 1");
    b += "PRINTNL\n";
    b += &bytes_of("PUSH L 305419896\nFN MKL$ 1");
    b += "PRINTNL\n";
    b += "PUSH I 12345\nFN MKI$ 1\nFN CVI 1\nPRINT\nPUSH I -7\nFN MKL$ 1\nFN CVL 1\nPRINT\nPUSH F 2.5\nFN MKS$ 1\nFN CVS 1\nPRINT\nPUSH D 1E300\nFN MKD$ 1\nFN CVD 1\nPRINT\nPRINTNL";
    assert_eq!(run(&prog(&b)), "FEFF\n78563412\n 12345 -7  2.5  1D+300 \n");
}

#[test]
fn mbf_conversions_use_qbrt() {
    let mut b = bytes_of("PUSH F 3.25\nFN MKSMBF$ 1");
    b += "PRINTNL\n";
    b += "PUSH F 3.25\nFN MKSMBF$ 1\nFN CVSMBF 1\nPRINT\nPUSH D -2.5\nFN MKDMBF$ 1\nFN CVDMBF 1\nPRINT\nPRINTNL";
    assert_eq!(run(&prog(&b)), "005082\n 3.25 -2.50 \n");
}

#[test]
fn cv_of_short_string_is_error_5() {
    let e = run_err(&prog(&format!("PUSH S {}\nFN CVSMBF 1", hex("ab"))));
    assert_eq!(e.code, 5);
}
