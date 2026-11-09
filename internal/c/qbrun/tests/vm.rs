//! QIR interpreter behaviour. Expected output follows QBasic/QB64 PRINT rules
//! (numbers: sign column + trailing space; `,` = 14-column zones).

use qbrun::{run_ir, RunError};

fn hex(s: &str) -> String {
    s.bytes().map(|b| format!("{b:02X}")).collect()
}

fn run(src: &str) -> String {
    let mut out = Vec::new();
    run_ir(src, &mut out).unwrap_or_else(|e| panic!("run failed: {e:?}"));
    String::from_utf8(out).unwrap()
}

fn run_err(src: &str) -> (RunError, String) {
    let mut out = Vec::new();
    let e = run_ir(src, &mut out).expect_err("expected runtime error");
    (e, String::from_utf8(out).unwrap())
}

#[test]
fn print_string_and_numbers() {
    let p = format!(
        "QIR 0\nLINE 1\nPUSH S {}\nPRINT\nPRINTNL\nLINE 2\nPUSH I 5\nPRINT\nPUSH I -3\nPRINT\nPRINTNL\nEND\n",
        hex("hello")
    );
    assert_eq!(run(&p), "hello\n 5 -3 \n");
}

#[test]
fn print_trailing_semicolon_keeps_line() {
    let p = format!("QIR 0\nPUSH S {}\nPRINT\nPUSH S {}\nPRINT\nPRINTNL\nEND\n", hex("a"), hex("b"));
    assert_eq!(run(&p), "ab\n");
}

#[test]
fn print_comma_zones() {
    let p = format!("QIR 0\nPUSH S {}\nPRINT\nPRINTZONE\nPUSH S {}\nPRINT\nPRINTNL\nEND\n", hex("a"), hex("b"));
    assert_eq!(run(&p), format!("a{}b\n", " ".repeat(13)));
}

#[test]
fn single_and_double_formatting_uses_qbrt() {
    // 1/3 is SINGLE, 1#/3 is DOUBLE (legacy prints 17 digits, see numfmt.rs)
    let p = "QIR 0\nPUSH I 1\nPUSH I 3\nOP DIV\nPRINT\nPRINTNL\nPUSH D 1\nPUSH I 3\nOP DIV\nPRINT\nPRINTNL\nEND\n";
    assert_eq!(run(p), " .3333333 \n .33333333333333331 \n");
}

#[test]
fn store_converts_by_suffix_with_bankers_rounding() {
    let p = "QIR 0\nPUSH F 2.5\nSTORE A%\nLOAD A%\nPRINT\nPUSH F 3.5\nSTORE B&\nLOAD B&\nPRINT\nPUSH I 7\nSTORE C!\nLOAD C!\nPUSH I 2\nOP DIV\nPRINT\nPRINTNL\nEND\n";
    assert_eq!(run(p), " 2  4  3.5 \n");
}

#[test]
fn unset_variables_default_to_zero_and_empty() {
    let p = format!("QIR 0\nLOAD X!\nPRINT\nLOAD S$\nPRINT\nPUSH S {}\nPRINT\nPRINTNL\nEND\n", hex("|"));
    assert_eq!(run(&p), " 0 |\n");
}

#[test]
fn integer_ops_and_comparisons() {
    let p = "QIR 0\nPUSH I 7\nPUSH I 2\nOP IDIV\nPRINT\nPUSH I 7\nPUSH I 2\nOP MOD\nPRINT\nPUSH I 2\nPUSH I 2\nOP EQ\nPRINT\nPUSH I 1\nPUSH I 2\nOP GT\nPRINT\nPUSH I 2\nPUSH I 10\nOP POW\nPRINT\nPUSH I 6\nPUSH I 3\nOP AND\nPRINT\nPUSH I 0\nUN NOT\nPRINT\nPRINTNL\nEND\n";
    assert_eq!(run(p), " 3  1 -1  0  1024  2 -1 \n");
}

#[test]
fn string_concat_compare_and_builtins() {
    let p = format!(
        "QIR 0\nPUSH S {h}\nPUSH I 2\nFN LEFT$ 2\nPUSH S {x}\nOP ADD\nPRINT\nPUSH S {h}\nFN LEN 1\nPRINT\nPUSH S {h}\nFN UCASE$ 1\nPRINT\nPUSH S {h}\nPUSH I 2\nPUSH I 3\nFN MID$ 3\nPRINT\nPUSH S {h}\nPUSH I 3\nFN RIGHT$ 2\nPRINT\nPUSH I 65\nFN CHR$ 1\nPRINT\nPUSH S {h}\nFN ASC 1\nPRINT\nPUSH I 42\nFN STR$ 1\nPRINT\nPUSH S {a}\nPUSH S {b}\nOP LT\nPRINT\nPRINTNL\nEND\n",
        h = hex("hello"),
        x = hex("!"),
        a = hex("abc"),
        b = hex("abd")
    );
    assert_eq!(run(&p), "he! 5 HELLOelllloA 104  42-1 \n");
}

#[test]
fn if_with_jz_and_labels() {
    let p = format!(
        "QIR 0\nPUSH I 1\nPUSH I 2\nOP EQ\nJZ ELSE1\nPUSH S {y}\nPRINT\nJMP END1\nLABEL ELSE1\nPUSH S {n}\nPRINT\nLABEL END1\nPRINTNL\nEND\n",
        y = hex("yes"),
        n = hex("no")
    );
    assert_eq!(run(&p), "no\n");
}

#[test]
fn for_loop_with_fortest_positive_and_negative_step() {
    // FOR I% = 1 TO 3 : PRINT I%; : NEXT  then  FOR J% = 3 TO 1 STEP -2
    let body = |v: &str, a: &str, b: &str, s: &str, n: &str| {
        format!(
            "PUSH I {a}\nSTORE {v}\nPUSH I {b}\nSTORE ~L{n}\nPUSH I {s}\nSTORE ~S{n}\nLABEL ~T{n}\nLOAD {v}\nLOAD ~L{n}\nLOAD ~S{n}\nFORTEST\nJZ ~X{n}\nLOAD {v}\nPRINT\nLOAD {v}\nLOAD ~S{n}\nOP ADD\nSTORE {v}\nJMP ~T{n}\nLABEL ~X{n}\n"
        )
    };
    let p = format!(
        "QIR 0\n{}PRINTNL\n{}PRINTNL\nLOAD I%\nPRINT\nPRINTNL\nEND\n",
        body("I%", "1", "3", "1", "1"),
        body("J%", "3", "1", "-2", "2")
    );
    assert_eq!(run(&p), " 1  2  3 \n 3  1 \n 4 \n");
}

#[test]
fn gosub_and_return() {
    let p = format!(
        "QIR 0\nGOSUB G1\nPUSH S {b}\nPRINT\nPRINTNL\nEND\nLABEL G1\nPUSH S {a}\nPRINT\nRETURN\n",
        a = hex("a"),
        b = hex("b")
    );
    assert_eq!(run(&p), "ab\n");
}

#[test]
fn return_without_gosub_is_error_3() {
    let (e, _) = run_err("QIR 0\nLINE 7\nRETURN\nEND\n");
    assert_eq!((e.code, e.line), (3, 7));
}

#[test]
fn sub_call_by_reference_and_shared() {
    // SUB Inc (n%): n% = n% + 1: T& = T& + 10 (T& is DIM SHARED)
    let p = "QIR 0\nSHARED T&\nPUSH I 5\nSTORE A%\nREF A%\nCALL INC 1\nREF A%\nCALL INC 1\nLOAD A%\nPRINT\nLOAD T&\nPRINT\nPRINTNL\nEND\nSUB INC 1 N%\nLOAD N%\nPUSH I 1\nOP ADD\nSTORE N%\nLOAD T&\nPUSH I 10\nOP ADD\nSTORE T&\nLOAD Q!\nPUSH I 1\nOP ADD\nSTORE Q!\nENDSUB\n";
    assert_eq!(run(p), " 7  20 \n");
}

#[test]
fn sub_by_value_argument_and_locals_are_fresh() {
    // SUB Show (s$, k%): PRINT s$; k%; L%: L% = 99
    let p = format!(
        "QIR 0\nPUSH S {x}\nPUSH I 2\nPUSH I 1\nOP ADD\nCALL SHOW 2\nPUSH S {y}\nPUSH I 4\nCALL SHOW 2\nPRINTNL\nEND\nSUB SHOW 2 S$ K%\nLOAD S$\nPRINT\nLOAD K%\nPRINT\nLOAD L%\nPRINT\nPUSH I 99\nSTORE L%\nENDSUB\n",
        x = hex("x"),
        y = hex("y")
    );
    assert_eq!(run(&p), "x 3  0 y 4  0 \n");
}

#[test]
fn sub_bodies_are_skipped_by_main_flow() {
    let p = format!(
        "QIR 0\nSUB NOPE 0\nPUSH S {a}\nPRINT\nENDSUB\nPUSH S {b}\nPRINT\nPRINTNL\n",
        a = hex("inside"),
        b = hex("main")
    );
    assert_eq!(run(&p), "main\n");
}

#[test]
fn on_error_resume_next() {
    // ON ERROR GOTO H: PRINT 1 \ 0: PRINT "after": END: H: PRINT ERR: RESUME NEXT
    let p = format!(
        "QIR 0\nONERROR H\nLINE 2\nPUSH I 1\nPUSH I 0\nOP IDIV\nPRINT\nPRINTNL\nLINE 3\nPUSH S {a}\nPRINT\nPRINTNL\nEND\nLABEL H\nFN ERR 0\nPRINT\nPRINTNL\nRESUME NEXT\n",
        a = hex("after")
    );
    assert_eq!(run(&p), " 11 \nafter\n");
}

#[test]
fn unhandled_error_reports_code_and_line() {
    let (e, out) = run_err("QIR 0\nLINE 4\nPUSH I 1\nPUSH I 0\nOP IDIV\nPRINT\nEND\n");
    assert_eq!((e.code, e.line, out.as_str()), (11, 4, ""));
}

#[test]
fn bad_ir_is_rejected() {
    let mut out = Vec::new();
    assert!(run_ir("NOT QIR\n", &mut out).is_err());
    assert!(run_ir("QIR 0\nBOGUS 1\n", &mut out).is_err());
    assert!(run_ir("QIR 0\nJMP NOWHERE\n", &mut out).is_err());
}
