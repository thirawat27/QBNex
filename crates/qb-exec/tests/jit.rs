//! R11.6: the JIT tier must give exactly the VM's results. Every program
//! runs interpreted only and with native code entered at the first loop
//! back-edge; output, exit status and error must match, and native code must
//! actually have run.

use qb_exec::{compile, Vm};
use qb_jit::tier::Mode;

fn hex(s: &str) -> String {
    s.bytes().map(|b| format!("{b:02X}")).collect()
}

/// `FOR v = from TO to STEP step : body : NEXT` in QIR v0.
fn for_loop(id: u32, v: &str, from: &str, to: &str, step: &str, body: &str) -> String {
    format!(
        "PUSH {from}\nSTORE {v}\nPUSH {to}\nSTORE ~F{id}.L\nPUSH {step}\nSTORE ~F{id}.S\nLABEL ~F{id}.T\n\
         LOAD {v}\nLOAD ~F{id}.L\nLOAD ~F{id}.S\nFORTEST\nJZ ~F{id}.X\n{body}\n\
         LOAD {v}\nLOAD ~F{id}.S\nOP ADD\nSTORE {v}\nJMP ~F{id}.T\nLABEL ~F{id}.X\n"
    )
}

#[derive(Debug, PartialEq)]
struct Run {
    out: String,
    result: Result<(), (i32, u32)>,
    entries: u64,
}

fn run_mode(src: &str, mode: Mode) -> Run {
    let m = compile(src).unwrap_or_else(|e| panic!("load: {e:?}"));
    let mut out = Vec::new();
    let (result, entries, fail) = {
        let mut vm = Vm::new(&m, &mut out);
        vm.set_jit(mode, 1);
        let r = vm.run().map_err(|e| (e.code, e.line));
        (r, vm.jit_entries(), vm.jit_failure())
    };
    assert!(fail.is_none(), "JIT compile failed: {fail:?}");
    Run { out: String::from_utf8(out).unwrap(), result, entries }
}

/// Runs both ways, checks equality and that native code ran; returns the output.
fn same(src: &str) -> String {
    let vm = run_mode(src, Mode::Off);
    let jit = run_mode(src, Mode::Hot);
    assert_eq!((&vm.out, &vm.result), (&jit.out, &jit.result), "VM vs JIT\n{src}");
    assert!(jit.entries > 0, "native code never entered\n{src}");
    vm.out
}

#[test]
fn numeric_loop() {
    let body = "LOAD S&\nLOAD I&\nPUSH I 7\nOP MOD\nOP ADD\nSTORE S&";
    let p = format!("QIR 0\n{}LOAD S&\nPRINT\nPRINTNL\nEND\n", for_loop(1, "I&", "I 1", "L 100000", "I 1", body));
    let expect: i64 = (1..=100000).map(|i| i % 7).sum();
    assert_eq!(same(&p), format!(" {expect} \n"));
}

#[test]
fn integer_wrap_and_negative_step() {
    // FOR i% = 10 TO 1 STEP -3: k% = k% * 3 + i%: NEXT  (k% wraps)
    let body = "LOAD K%\nPUSH I 3\nOP MUL\nLOAD I%\nOP ADD\nSTORE K%";
    let p = format!(
        "QIR 0\nPUSH I 30000\nSTORE K%\n{}LOAD K%\nPRINT\nLOAD I%\nPRINT\nPRINTNL\nEND\n",
        for_loop(1, "I%", "I 10", "I 1", "I -3", body)
    );
    same(&p);
}

#[test]
fn single_and_double_loop() {
    // FOR x! = 1 TO 0 STEP -.1: s! = s! + x! * x!: d# = d# + 1 / (x! + 1): NEXT
    let body = "LOAD S!\nLOAD X!\nLOAD X!\nOP MUL\nOP ADD\nSTORE S!\nLOAD D#\nPUSH I 1\nLOAD X!\nPUSH I 1\nOP ADD\nOP DIV\nOP ADD\nSTORE D#";
    let p = format!(
        "QIR 0\n{}LOAD S!\nPRINT\nLOAD D#\nPRINT\nLOAD X!\nPRINT\nPRINTNL\nEND\n",
        for_loop(1, "X!", "I 1", "I 0", "F -0.1", body)
    );
    same(&p);
}

#[test]
fn strings_in_a_loop_go_through_the_vm() {
    // FOR i% = 1 TO 60: a$ = a$ + CHR$(65 + i% MOD 26): NEXT: PRINT a$; LEN(a$)
    let body = "LOAD A$\nPUSH I 65\nLOAD I%\nPUSH I 26\nOP MOD\nOP ADD\nFN CHR$ 1\nOP ADD\nSTORE A$";
    let p = format!(
        "QIR 0\n{}LOAD A$\nPRINT\nLOAD A$\nFN LEN 1\nPRINT\nPRINTNL\nEND\n",
        for_loop(1, "I%", "I 1", "I 60", "I 1", body)
    );
    let out = same(&p);
    assert!(out.starts_with("BCD"), "{out}");
}

#[test]
fn arrays_in_a_loop() {
    // DIM a&(100): FOR i = 1 TO 100: a&(i) = i * i: NEXT: FOR i = 1 TO 100: t& = t& + a&(i): NEXT
    let fill = "LOAD I%\nLOAD I%\nLOAD I%\nOP MUL\nSTOREE A&() 1";
    let sum = "LOAD T&\nLOAD I%\nLOADE A&() 1\nOP ADD\nSTORE T&";
    let p = format!(
        "QIR 0\nPUSH I 0\nPUSH I 100\nDIMA A&() 1\n{}{}LOAD T&\nPRINT\nPRINTNL\nEND\n",
        for_loop(1, "I%", "I 1", "I 100", "I 1", fill),
        for_loop(2, "I%", "I 1", "I 100", "I 1", sum)
    );
    assert_eq!(same(&p), " 338350 \n");
}

#[test]
fn error_inside_native_loop_resumes_next() {
    // ON ERROR GOTO H: FOR i% = 1 TO 20: x% = 100 \ (i% MOD 3): s% = s% + x%: NEXT: PRINT s%; e%: END
    // H: e% = e% + 1: RESUME NEXT
    let body = "LINE 3\nPUSH I 100\nLOAD I%\nPUSH I 3\nOP MOD\nOP IDIV\nSTORE X%\nLINE 4\nLOAD S%\nLOAD X%\nOP ADD\nSTORE S%";
    let p = format!(
        "QIR 0\nLINE 1\nONERROR MAIN.H\nLINE 2\n{}LINE 5\nLOAD S%\nPRINT\nLOAD E%\nPRINT\nPRINTNL\nEND\n\
         LABEL MAIN.H\nLINE 7\nLOAD E%\nPUSH I 1\nOP ADD\nSTORE E%\nRESUME NEXT\n",
        for_loop(1, "I%", "I 1", "I 20", "I 1", body)
    );
    same(&p);
}

#[test]
fn unhandled_overflow_in_native_loop() {
    // FOR i& = 1 TO 10: x% = i& * 5000.5: NEXT   -> error 6 at i& = 7
    let body = "LINE 2\nLOAD I&\nPUSH D 5000.5\nOP MUL\nSTORE X%";
    let p = format!("QIR 0\nLINE 1\n{}LOAD X%\nPRINT\nPRINTNL\nEND\n", for_loop(1, "I&", "I 1", "I 10", "I 1", body));
    let r = run_mode(&p, Mode::Hot);
    assert_eq!(r.result.map_err(|e| e.0), Err(6));
    same(&p);
}

#[test]
fn sub_called_from_a_hot_loop() {
    // FOR i% = 1 TO 50: CALL ADDTO(i%): NEXT: PRINT T&  with SUB ADDTO(n%): SHARED T&: T& = T& + n% * 2
    let body = "LOAD I%\nCALL ADDTO 1";
    let p = format!(
        "QIR 0\nSHARED T&\n{}LOAD T&\nPRINT\nPRINTNL\nEND\n\
         SUB ADDTO 1 N%\nLOAD T&\nLOAD N%\nPUSH I 2\nOP MUL\nOP ADD\nSTORE T&\nENDSUB\n",
        for_loop(1, "I%", "I 1", "I 50", "I 1", body)
    );
    assert_eq!(same(&p), " 2550 \n");
}

#[test]
fn print_inside_loop_and_hex_strings() {
    let body = format!("LOAD I%\nPRINT\nPUSH S {}\nPRINT", hex(","));
    let p = format!("QIR 0\n{}PRINTNL\nEND\n", for_loop(1, "I%", "I 1", "I 5", "I 1", &body));
    assert_eq!(same(&p), " 1 , 2 , 3 , 4 , 5 ,\n");
}
