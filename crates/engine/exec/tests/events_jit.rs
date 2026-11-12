//! An ON TIMER handler is main-module code the VM may run at any safe point,
//! also inside a CALL that native main code made: native main code must not
//! keep main variables in registers then (it would use and write back its
//! stale copy).
//!
//! ```basic
//! ON TIMER(0.05) GOSUB tick: TIMER ON
//! DO: n& = n& + 1: LOOP WHILE n& < 3   ' main runs as native code from here
//! CALL Draw                           ' the handler runs inside this call
//! TIMER OFF: PRINT STR$(done&); " "
//! END
//! tick: done& = -1: fired& = -1: RETURN
//! SUB Draw ' fired& is SHARED; waits for the handler (5 s guard)
//!     s$ = "x": t0# = TIMER
//!     DO: x# = x# + 1: LOOP UNTIL fired& OR TIMER - t0# > 5
//! END SUB
//! ```
//!
//! Each mode runs in its own qb-exec process (the ON TIMER table and its
//! thread are process-wide).

use std::process::{Command, Stdio};

const PROG: &str = "QIR 1
meta console only
global $__LONG_DONE i32
global $__LONG_FIRED i32
global $__LONG_N i32
func main
line 1
const.i32 %0 0
const.f64 %1 0.05E+0
ontimer %0 %1 L_TICK
const.i32 %2 1
const.i32 %3 0
rt sub_timer %0 %2 %3
label ~L1
line 2
load.i32 %4 $__LONG_N
const.i32 %5 1
add.i32 %6 %4 %5
store.i32 $__LONG_N %6
const.i32 %7 3
lt.i32 %8 %6 %7
jnz %8 ~L1
line 3
call SUB_DRAW
line 4
const.i32 %9 2
rt sub_timer %0 %9 %3
load.i32 %10 $__LONG_DONE
cvt.i64.i32 %11 %10
rt %12 STR$.i64 %11
const.str %13 x20
concat %14 %12 %13
const.i32 %15 0
rt PRINTS %15 %14
rt PRINTNL %15
line 5
end
label L_TICK
line 6
const.i32 %16 -1
store.i32 $__LONG_DONE %16
store.i32 $__LONG_FIRED %16
gret
endfunc
func SUB_DRAW
local $_SUB_DRAW_DOUBLE_T0 f64
local $_SUB_DRAW_DOUBLE_X f64
local $_SUB_DRAW_STRING_S str
line 8
const.str %0 x78
store.str $_SUB_DRAW_STRING_S %0
rt %1 TIMER
store.f64 $_SUB_DRAW_DOUBLE_T0 %1
label ~F2.T
line 9
load.f64 %2 $_SUB_DRAW_DOUBLE_X
const.f64 %3 1.0E+0
add.f64 %4 %2 %3
store.f64 $_SUB_DRAW_DOUBLE_X %4
load.i32 %5 $__LONG_FIRED
jnz %5 ~EXIT
rt %6 TIMER
load.f64 %7 $_SUB_DRAW_DOUBLE_T0
sub.f64 %8 %6 %7
const.f64 %9 5.0E+0
gt.f64 %10 %8 %9
jz %10 ~F2.T
label ~EXIT
ret
endfunc
";

fn run(mode: &str) -> String {
    let dir = std::env::temp_dir().join(format!("qb-events-jit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let qir = dir.join(format!("timer_{mode}.qir"));
    std::fs::write(&qir, PROG).unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_qb-exec"))
        .arg(&qir)
        .env("QB_JIT", mode)
        .env("QBNEX_NO_ERROR_DIALOG", "1")
        .env("QBNEX_HEADLESS", "1")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let _ = std::fs::remove_file(&qir);
    assert!(o.status.success(), "QB_JIT={mode}: {}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).replace("\r\n", "\n")
}

#[test]
fn on_timer_handler_inside_a_vm_call_from_native_main_code() {
    let vm = run("off");
    assert_eq!(vm, "-1 \n", "VM");
    assert_eq!(run("eager"), vm, "JIT (eager) vs VM");
}
