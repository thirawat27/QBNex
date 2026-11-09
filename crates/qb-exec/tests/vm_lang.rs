//! QIR v0 extensions: arrays, FUNCTION, STATIC SUB, errors in SUBs, RESUME,
//! JNZ, SWAP and more builtins. Expected values follow the golden output of
//! tests/conformance/cases (captured from the legacy C++ runtime).

use qb_exec::{run_ir, RunError};

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
fn jnz_jumps_on_true() {
    let p = prog(&format!("PUSH I -1\nJNZ A\nPUSH S {}\nPRINT\nLABEL A\nPRINTNL", hex("no")));
    assert_eq!(run(&p), "\n");
}

#[test]
fn two_dimensional_array_with_bounds() {
    // DIM m(1 TO 3, 0 TO 2) AS LONG: m(2,1) = 21: PRINT m(2,1); LBOUND(m,2); UBOUND(m,1)
    let p = prog(
        "PUSH I 1\nPUSH I 3\nPUSH I 0\nPUSH I 2\nDIMA M&() 2\n\
         PUSH I 2\nPUSH I 1\nPUSH I 21\nSTOREE M&() 2\n\
         PUSH I 2\nPUSH I 1\nLOADE M&() 2\nPRINT\n\
         PUSH I 2\nBOUND M&() L\nPRINT\nPUSH I 1\nBOUND M&() U\nPRINT\n\
         PUSH I 3\nPUSH I 2\nLOADE M&() 2\nPRINT\nPRINTNL",
    );
    assert_eq!(run(&p), " 21  0  3  0 \n");
}

#[test]
fn redim_preserve_keeps_values_and_redim_clears() {
    let p = prog(
        "PUSH I 0\nPUSH I 5\nREDIMA V%() 1 0\nPUSH I 5\nPUSH I 25\nSTOREE V%() 1\n\
         PUSH I 0\nPUSH I 8\nREDIMA V%() 1 1\nPUSH I 5\nLOADE V%() 1\nPRINT\nPUSH I 1\nBOUND V%() U\nPRINT\n\
         PUSH I 0\nPUSH I 8\nREDIMA V%() 1 0\nPUSH I 5\nLOADE V%() 1\nPRINT\nPRINTNL",
    );
    assert_eq!(run(&p), " 25  8  0 \n");
}

#[test]
fn array_index_out_of_range_is_error_9() {
    let p = prog("LINE 3\nPUSH I 0\nPUSH I 5\nDIMA A!() 1\nPUSH I 10\nPUSH I 1\nSTOREE A!() 1");
    let e = run_err(&p);
    assert_eq!((e.code, e.line), (9, 3));
}

#[test]
fn undeclared_array_is_auto_dimensioned_0_to_10() {
    let p = prog("PUSH I 10\nPUSH I 7\nSTOREE Z%() 1\nPUSH I 10\nLOADE Z%() 1\nPRINT\nPRINTNL");
    assert_eq!(run(&p), " 7 \n");
}

#[test]
fn string_array_and_swap_of_elements() {
    let p = prog(&format!(
        "PUSH I 0\nPUSH I 2\nDIMA S$() 1\nPUSH I 0\nPUSH S {x}\nSTOREE S$() 1\nPUSH I 2\nPUSH S {z}\nSTOREE S$() 1\n\
         PUSH I 0\nREFE S$() 1\nPUSH I 2\nREFE S$() 1\nSWAP\n\
         PUSH I 0\nLOADE S$() 1\nPRINT\nPUSH I 1\nLOADE S$() 1\nPRINT\nPUSH I 2\nLOADE S$() 1\nPRINT\nPRINTNL",
        x = hex("x"),
        z = hex("z")
    ));
    assert_eq!(run(&p), "zx\n");
}

#[test]
fn array_element_passed_by_reference() {
    // SUB INC (n%): n% = n% + 1 -- called with a(3)
    let p = "QIR 0\nPUSH I 3\nREFE A%() 1\nCALL INC 1\nPUSH I 3\nLOADE A%() 1\nPRINT\nPRINTNL\nEND\n\
             SUB INC 1 N%\nLOAD N%\nPUSH I 1\nOP ADD\nSTORE N%\nENDSUB\n";
    assert_eq!(run(p), " 1 \n");
}

#[test]
fn recursive_function_returns_value() {
    // FUNCTION FACT& (N&): IF N& <= 1 THEN FACT& = 1 ELSE FACT& = N& * FACT&(N& - 1)
    let p = "QIR 0\nPUSH I 10\nCALL FACT& 1\nPRINT\nPRINTNL\nEND\n\
             FUNCTION FACT& 1 N&\nLOAD N&\nPUSH I 1\nOP LE\nJZ E\nPUSH I 1\nSTORE FACT&\nJMP X\nLABEL E\n\
             LOAD N&\nLOAD N&\nPUSH I 1\nOP SUB\nCALL FACT& 1\nOP MUL\nSTORE FACT&\nLABEL X\nENDSUB\n";
    assert_eq!(run(p), " 3628800 \n");
}

#[test]
fn string_function() {
    let p = format!(
        "QIR 0\nPUSH S {}\nCALL TWICE$ 1\nPRINT\nPRINTNL\nEND\nFUNCTION TWICE$ 1 S$\nLOAD S$\nLOAD S$\nOP ADD\nSTORE TWICE$\nENDSUB\n",
        hex("ab")
    );
    assert_eq!(run(&p), "abab\n");
}

#[test]
fn static_sub_keeps_locals_between_calls() {
    let p = "QIR 0\nSHARED T&\nCALL C 0\nCALL C 0\nCALL C 0\nLOAD T&\nPRINT\nPRINTNL\nEND\n\
             SUB C 0 STATIC\nLOAD C!\nPUSH I 1\nOP ADD\nSTORE C!\nLOAD T&\nLOAD C!\nOP ADD\nSTORE T&\nENDSUB\n";
    assert_eq!(run(p), " 6 \n");
}

#[test]
fn recursion_does_not_leak_locals_between_frames() {
    // SUB R (D%): L% = D%: IF D% > 0 THEN R D% - 1: PRINT L%;
    let p = "QIR 0\nPUSH I 3\nCALL R 1\nPRINTNL\nEND\n\
             SUB R 1 D%\nLOAD D%\nSTORE L%\nLOAD D%\nPUSH I 0\nOP GT\nJZ S\nLOAD D%\nPUSH I 1\nOP SUB\nCALL R 1\nLABEL S\nLOAD L%\nPRINT\nENDSUB\n";
    assert_eq!(run(p), " 0  1  2  3 \n");
}

#[test]
fn error_statement_and_resume_retry() {
    // ON ERROR GOTO H: A! = 0: ERROR 61: PRINT "done": END
    // H: PRINT ERR: IF A! = 0 THEN A! = 1: RESUME  ELSE RESUME NEXT
    let p = format!(
        "QIR 0\nONERROR H\nLINE 1\nPUSH I 0\nSTORE A!\nLINE 2\nPUSH I 61\nERROR\nLINE 3\nPUSH S {d}\nPRINT\nPRINTNL\nEND\n\
         LABEL H\nFN ERR 0\nPRINT\nLOAD A!\nPUSH I 0\nOP EQ\nJZ N\nPUSH I 1\nSTORE A!\nRESUME 0\nLABEL N\nRESUME NEXT\n",
        d = hex("done")
    );
    assert_eq!(run(&p), " 61  61 done\n");
}

#[test]
fn error_in_sub_runs_main_handler_and_resumes_inside_sub() {
    // QBasic: handler sees module-level variables; RESUME NEXT continues in the SUB.
    let p = format!(
        "QIR 0\nONERROR H\nPUSH I 5\nSTORE M%\nLINE 2\nCALL WORK 0\nLINE 3\nPUSH S {mc}\nPRINT\nPRINTNL\nEND\n\
         LABEL H\nLOAD M%\nPRINT\nFN ERR 0\nPRINT\nRESUME NEXT\n\
         SUB WORK 0\nPUSH I 9\nSTORE M%\nLINE 10\nPUSH I 7\nERROR\nLINE 11\nLOAD M%\nPRINT\nENDSUB\n",
        mc = hex("main")
    );
    assert_eq!(run(&p), " 5  7  9 main\n");
}

#[test]
fn eqv_imp_and_logic() {
    let p = prog("PUSH I 5\nPUSH I 3\nOP EQV\nPRINT\nPUSH I 5\nPUSH I 3\nOP IMP\nPRINT\nPRINTNL");
    assert_eq!(run(&p), "-7 -5 \n");
}

fn fn1(name: &str, arg: &str) -> String {
    format!("{arg}\nFN {name} 1\nPRINT\n")
}

#[test]
fn math_builtins() {
    let mut b = String::new();
    b += &fn1("ABS", "PUSH I -3");
    b += &fn1("SGN", "PUSH I -2");
    b += &fn1("SQR", "PUSH I 2");
    b += &fn1("SIN", "PUSH I 1");
    b += &fn1("EXP", "PUSH I 1");
    b += &fn1("LOG", "PUSH I 10");
    b += &fn1("INT", "PUSH F -2.5");
    b += &fn1("FIX", "PUSH F -2.5");
    b += &fn1("CINT", "PUSH F 2.5");
    b += &fn1("_ROUND", "PUSH F 3.5");
    b += "PRINTNL";
    assert_eq!(run(&prog(&b)), " 3 -1  1.414214  .841471  2.718282  2.302585 -3 -2  2  4 \n");
}

#[test]
fn hex_oct_follow_operand_width() {
    let p = prog("PUSH I 255\nFN HEX$ 1\nPRINT\nPUSH I -1\nFN HEX$ 1\nPRINT\nPUSH L -1\nFN HEX$ 1\nPRINT\nPUSH I 8\nFN OCT$ 1\nPRINT\nPRINTNL");
    assert_eq!(run(&p), "FFFFFFFFFFFFFF10\n");
}

#[test]
fn string_builtins() {
    let a = hex("Hello, World");
    let p = prog(&format!(
        "PUSH S {a}\nPUSH S {o}\nFN INSTR 2\nPRINT\nPUSH I 6\nPUSH S {a}\nPUSH S {o}\nFN INSTR 3\nPRINT\n\
         PUSH S {sp}\nFN LTRIM$ 1\nPRINT\nPUSH S {sp}\nFN RTRIM$ 1\nPRINT\nPUSH S {sp}\nFN _TRIM$ 1\nPRINT\n\
         PUSH I 3\nPUSH S {star}\nFN STRING$ 2\nPRINT\nPUSH I 2\nPUSH I 65\nFN STRING$ 2\nPRINT\nPUSH I 2\nFN SPACE$ 1\nPRINT\n\
         PUSH S {a}\nPUSH I 2\nFN ASC 2\nPRINT\nPRINTNL",
        o = hex("o"),
        sp = hex(" x "),
        star = hex("*")
    ));
    assert_eq!(run(&p), " 5  9 x  xx***AA   101 \n");
}

#[test]
fn val_uses_qb_runtime_and_returns_double() {
    let p = prog(&format!(
        "PUSH S {}\nFN VAL 1\nPRINT\nPUSH S {}\nFN VAL 1\nPRINT\nPRINTNL",
        hex("  12.5abc"),
        hex("&H10")
    ));
    assert_eq!(run(&p), " 12.50  16.0 \n");
}

#[test]
fn mid_statement_replaces_in_place() {
    // MID$(a$, 1, 5) = "Jello" lowered to FN MID$SET 4 + STORE
    let p = prog(&format!(
        "PUSH S {}\nPUSH I 1\nPUSH I 5\nPUSH S {}\nFN MID$SET 4\nPRINT\nPRINTNL",
        hex("Hello, World"),
        hex("Jello")
    ));
    assert_eq!(run(&p), "Jello, World\n");
}
