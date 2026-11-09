//! R11.5 typed register VM: what the loader turns QIR into (typed registers,
//! superinstructions) and the runtime features added with it (_INTEGER64,
//! event polling at safe points, zero-copy BYREF strings).

use qb_exec::{compile, run_ir, Vm};

fn hex(s: &str) -> String {
    s.bytes().map(|b| format!("{b:02X}")).collect()
}

fn run(src: &str) -> String {
    let mut out = Vec::new();
    run_ir(src, &mut out).unwrap_or_else(|e| panic!("run failed: {e:?}"));
    String::from_utf8(out).unwrap()
}

/// FOR i& = 1 TO n : s& = s& + i& MOD 7 : NEXT
fn mod_loop(n: u32) -> String {
    format!(
        "QIR 0\nLINE 1\nPUSH I 1\nSTORE I&\nPUSH L {n}\nSTORE ~F1.L\nPUSH I 1\nSTORE ~F1.S\nLABEL ~F1.T\n\
         LOAD I&\nLOAD ~F1.L\nLOAD ~F1.S\nFORTEST\nJZ ~F1.X\nLINE 2\nLOAD S&\nLOAD I&\nPUSH I 7\nOP MOD\nOP ADD\nSTORE S&\n\
         LINE 3\nLOAD I&\nLOAD ~F1.S\nOP ADD\nSTORE I&\nJMP ~F1.T\nLABEL ~F1.X\nLINE 4\nLOAD S&\nPRINT\nPRINTNL\nEND\n"
    )
}

#[test]
fn for_loop_becomes_one_superinstruction_per_iteration() {
    let m = compile(&mod_loop(10)).unwrap();
    let d = m.disasm();
    assert!(d.contains("for_test.i"), "{d}");
    assert!(d.contains("for_next.i32"), "{d}");
    // loop body: MOD + fused ADD into S& + FOR_NEXT, no MOVs, no JMP back
    let body: Vec<&str> = d.lines().filter(|l| l.contains("mod.i32") || l.contains("mov") || l.contains("jmp")).collect();
    assert_eq!(body.len(), 1, "{d}");
    assert_eq!(run(&mod_loop(10)), " 27 \n");
}

#[test]
fn compare_and_branch_fuse() {
    let p = format!("QIR 0\nLOAD A%\nPUSH I 3\nOP LT\nJZ E\nPUSH S {}\nPRINT\nLABEL E\nPRINTNL\nEND\n", hex("lt"));
    let d = compile(&p).unwrap().disasm();
    assert!(d.contains("br.i ge"), "{d}");
    assert!(!d.contains("cmp."), "{d}");
    assert_eq!(run(&p), "lt\n");
}

#[test]
fn integer64_suffix_is_typed() {
    let p = "QIR 0\nPUSH L 2147483647\nSTORE A&&\nLOAD A&&\nPUSH I 1\nOP ADD\nSTORE A&&\nLOAD A&&\nPRINT\nLOAD A&&\nPUSH I 2\nOP MUL\nPRINT\nPRINTNL\nEND\n";
    assert_eq!(run(p), " 2147483648  4294967296 \n");
}

#[test]
fn long_arithmetic_wraps_like_the_cpp_backend() {
    let p = "QIR 0\nPUSH L 2147483647\nSTORE A&\nLOAD A&\nPUSH I 1\nOP ADD\nSTORE A&\nLOAD A&\nPRINT\nPRINTNL\nEND\n";
    assert_eq!(run(p), "-2147483648 \n");
}

#[test]
fn byref_string_is_updated_in_place() {
    // SUB ADD1 (s$): s$ = s$ + "x"  called 3 times on a$
    let p = format!(
        "QIR 0\nPUSH S {a}\nSTORE A$\nREF A$\nCALL ADD1 1\nREF A$\nCALL ADD1 1\nREF A$\nCALL ADD1 1\nLOAD A$\nPRINT\nPRINTNL\nEND\n\
         SUB ADD1 1 S$\nLOAD S$\nPUSH S {x}\nOP ADD\nSTORE S$\nENDSUB\n",
        a = hex("a"),
        x = hex("x")
    );
    assert_eq!(run(&p), "axxx\n");
}

#[test]
fn string_self_concat_is_in_place() {
    let p = format!("QIR 0\nLOAD A$\nPUSH S {x}\nOP ADD\nSTORE A$\nLOAD A$\nPRINT\nPRINTNL\nEND\n", x = hex("x"));
    let d = compile(&p).unwrap().disasm();
    assert!(d.contains("concat s0, s0, k0"), "{d}");
    assert_eq!(run(&p), "x\n");
}

#[test]
fn values_on_the_stack_survive_a_store_to_the_same_variable() {
    // stack keeps the old A% while A% is overwritten: PRINT A% after A% = 5 inside a SWAP-like sequence
    let p = "QIR 0\nPUSH I 1\nSTORE A%\nPUSH I 2\nSTORE B%\nLOAD A%\nLOAD B%\nSTORE A%\nSTORE B%\nLOAD A%\nPRINT\nLOAD B%\nPRINT\nPRINTNL\nEND\n";
    assert_eq!(run(p), " 2  1 \n");
}

#[test]
fn event_is_polled_on_loop_back_edges() {
    // ON EVENT 1 GOSUB H: EVENT(1) ON: DO WHILE D% = 0: LOOP: PRINT "done"
    // H: D% = 1: RETURN
    let p = format!(
        "QIR 0\nONEVENT 1 H\nEVENTON 1\nLABEL T\nLINE 3\nLOAD D%\nPUSH I 0\nOP EQ\nJZ X\nJMP T\nLABEL X\nLINE 4\nPUSH S {d}\nPRINT\nPRINTNL\nEND\n\
         LABEL H\nPUSH I 1\nSTORE D%\nRETURN\n",
        d = hex("done")
    );
    let m = compile(&p).unwrap();
    let mut out = Vec::new();
    let mut vm = Vm::new(&m, &mut out);
    let ev = vm.events();
    let t = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(30));
        ev.raise(1);
    });
    vm.run().unwrap();
    t.join().unwrap();
    drop(vm);
    assert_eq!(String::from_utf8(out).unwrap(), "done\n");
}

#[test]
fn on_timer_fires_from_its_own_thread() {
    // ON TIMER(0.01) GOSUB H: TIMER ON: DO WHILE N% < 3: LOOP: TIMER OFF: PRINT N%
    let p = "QIR 0\nPUSH F 0.01\nONTIMER H\nEVENTON 0\nLABEL T\nLOAD N%\nPUSH I 3\nOP LT\nJZ X\nJMP T\nLABEL X\nEVENTOFF 0\nLOAD N%\nPRINT\nPRINTNL\nEND\n\
             LABEL H\nLOAD N%\nPUSH I 1\nOP ADD\nSTORE N%\nRETURN\n";
    assert_eq!(run(p), " 3 \n");
}

#[test]
fn stopped_event_is_remembered_and_fires_on_resume() {
    // EVENT STOP queues the event; EVENT ON delivers it at the next safe point
    let p = format!(
        "QIR 0\nONEVENT 2 H\nEVENTSTOP 2\nLABEL T\nLOAD C&\nPUSH I 1\nOP ADD\nSTORE C&\nLOAD C&\nPUSH L 200000\nOP LT\nJZ X\nJMP T\nLABEL X\n\
         LOAD D%\nPRINT\nEVENTON 2\nLABEL U\nLOAD D%\nPUSH I 0\nOP EQ\nJZ Y\nJMP U\nLABEL Y\nPUSH S {d}\nPRINT\nPRINTNL\nEND\n\
         LABEL H\nPUSH I 1\nSTORE D%\nRETURN\n",
        d = hex("done")
    );
    let m = compile(&p).unwrap();
    let mut out = Vec::new();
    let mut vm = Vm::new(&m, &mut out);
    vm.events().raise(2);
    vm.run().unwrap();
    drop(vm);
    assert_eq!(String::from_utf8(out).unwrap(), " 0 done\n");
}

#[test]
fn function_result_and_byref_copy_back_in_one_call() {
    // FUNCTION BUMP% (N%): N% = N% + 1: BUMP% = N% * 10
    let p = "QIR 0\nPUSH I 4\nSTORE A%\nREF A%\nCALL BUMP% 1\nPRINT\nLOAD A%\nPRINT\nPRINTNL\nEND\n\
             FUNCTION BUMP% 1 N%\nLOAD N%\nPUSH I 1\nOP ADD\nSTORE N%\nLOAD N%\nPUSH I 10\nOP MUL\nSTORE BUMP%\nENDSUB\n";
    assert_eq!(run(p), " 50  5 \n");
}

#[test]
fn shared_array_inside_sub() {
    let p = "QIR 0\nSHARED A%()\nPUSH I 0\nPUSH I 3\nDIMA A%() 1\nCALL FILL 0\nPUSH I 3\nLOADE A%() 1\nPRINT\nPRINTNL\nEND\n\
             SUB FILL 0\nPUSH I 3\nPUSH I 42\nSTOREE A%() 1\nENDSUB\n";
    assert_eq!(run(p), " 42 \n");
}
