//! END in a console program prints "Press any key to continue" and waits for a
//! key, as QB64 console programs do; the runner decides with the runtime's
//! END policy (end_prompt_wanted) whether anybody could press it. SYSTEM
//! ends without the prompt.

use std::sync::atomic::{AtomicU32, Ordering};

// one counter per test: the tests run in parallel
static WAITS: AtomicU32 = AtomicU32::new(0);
static SYSTEM_WAITS: AtomicU32 = AtomicU32::new(0);

fn count_wait() {
    WAITS.fetch_add(1, Ordering::SeqCst);
}

fn count_system_wait() {
    SYSTEM_WAITS.fetch_add(1, Ordering::SeqCst);
}

fn run(src: &str, wait: Option<fn()>) -> String {
    let m = qb_exec::compile(src).unwrap();
    let mut b = Vec::new();
    let r = {
        let mut vm = qb_exec::Vm::new(&m, &mut b);
        vm.set_end_prompt(wait);
        vm.run_status()
    };
    let o = String::from_utf8_lossy(&b).into_owned();
    format!("{o}<exit {}>", r.unwrap())
}

const END: &str = "QIR 1\nmeta console only\nfunc main\nline 1\nconst.str %0 x78\nconst.i32 %1 0\nrt PRINTS %1 %0\nrt PRINTNL %1\nline 2\nend\nendfunc\n";

#[test]
fn end_prints_the_prompt_and_waits_when_wanted() {
    let before = WAITS.load(Ordering::SeqCst);
    let prompt = if cfg!(windows) { "Press any key to continue" } else { "Press enter to continue" };
    assert_eq!(run(END, Some(count_wait)), format!("x\n\n{prompt}<exit 0>"));
    assert_eq!(WAITS.load(Ordering::SeqCst), before + 1);
}

#[test]
fn end_is_silent_when_nobody_could_press_a_key() {
    assert_eq!(run(END, None), "x\n<exit 0>");
}

#[test]
fn system_never_prompts() {
    let src = END.replace("line 2\nend\n", "line 2\nsystem\n");
    assert_eq!(run(&src, Some(count_system_wait)), "x\n<exit 0>");
    assert_eq!(SYSTEM_WAITS.load(Ordering::SeqCst), 0);
}
