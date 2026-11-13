//! Console INPUT, LINE INPUT and INPUT$ of the runner (`qb-exec`) with stdin
//! redirected:
//! - INPUT a$, b, c reads one line; values missing from it are empty / 0, the
//!   next INPUT reads the next line
//! - at the end of stdin INPUT and LINE INPUT give empty values instead of
//!   error 62 "Input past end of file" (Q4)
//! - INPUT$(n) without a file number reads the keyboard / stdin, not error 52 (Q5)

use std::io::Write;
use std::process::{Command, Stdio};

fn run(name: &str, qir: &str, stdin: &[u8]) -> String {
    let dir = std::env::temp_dir().join(format!("qb_exec_console_input_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.qir"));
    std::fs::write(&path, qir).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_qb-exec"))
        .arg(&path)
        .env("QBNEX_NO_ERROR_DIALOG", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stdin).unwrap();
    let out = child.wait_with_output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n");
    let err = String::from_utf8_lossy(&out.stderr).replace("\r\n", "\n");
    format!("{text}{err}<exit {}>", out.status.code().unwrap_or(-1))
}

/// `INPUT a$, b, c` then `INPUT d$`; prints [a$] b c [d$], one per line.
const INPUT3: &str = r#"QIR 1
meta console only
global $A str
global $B f64
global $C f64
global $D str
func main
line 1
const.i32 %0 0
const.str %1 x3F20
rt INPUTPROMPT %1 %0
rt %2 INPUTITEM.s %0
store.str $A %2
rt %3 INPUTITEM.n %0
store.f64 $B %3
rt %4 INPUTITEM.n %0
store.f64 $C %4
line 2
rt INPUTPROMPT %1 %0
rt %5 INPUTITEM.s %0
store.str $D %5
line 3
const.str %6 x0A
const.str %7 x5B
const.str %8 x5D
load.str %10 $A
rt PRINTS %0 %7
rt PRINTS %0 %10
rt PRINTS %0 %8
rt PRINTNL %0
load.f64 %11 $B
cvt.i64.f64 %12 %11
rt %13 STR$.i64 %12
rt PRINTS %0 %13
rt PRINTNL %0
load.f64 %14 $C
cvt.i64.f64 %15 %14
rt %16 STR$.i64 %15
rt PRINTS %0 %16
rt PRINTNL %0
load.str %17 $D
rt PRINTS %0 %7
rt PRINTS %0 %17
rt PRINTS %0 %8
rt PRINTNL %0
system
endfunc
"#;

#[test]
fn input_with_several_variables_reads_one_line() {
    assert_eq!(run("q3", INPUT3, b"1\n7\n8\n"), "? ? [1]\n 0\n 0\n[7]\n<exit 0>");
}

#[test]
fn input_with_several_variables_splits_the_line_at_commas() {
    assert_eq!(run("q3b", INPUT3, b"ab, 2, 3\nz\n"), "? ? [ab]\n 2\n 3\n[z]\n<exit 0>");
}

#[test]
fn input_at_end_of_stdin_gives_empty_values() {
    assert_eq!(run("q4", INPUT3, b""), "? ? []\n 0\n 0\n[]\n<exit 0>");
}

#[test]
fn line_input_at_end_of_stdin_gives_an_empty_line() {
    let qir = "QIR 1\nmeta console only\nfunc main\nline 1\nconst.i32 %0 0\nconst.str %1 x\nrt INPUTPROMPT %1 %0\nrt %2 LINEINPUT %0\nconst.str %3 x5B\nconst.str %4 x5D\nrt PRINTS %0 %3\nrt PRINTS %0 %2\nrt PRINTS %0 %4\nrt PRINTNL %0\nsystem\nendfunc\n";
    assert_eq!(run("q4l", qir, b""), "[]\n<exit 0>");
}

/// `A$ = INPUT$(1)` twice, each printed as [A$].
const INPUTN: &str = r#"QIR 1
meta console only
func main
line 1
const.i32 %0 0
const.i32 %1 1
const.str %3 x5B
const.str %4 x5D
rt %2 INPUT$ %1 %0
rt PRINTS %0 %3
rt PRINTS %0 %2
rt PRINTS %0 %4
rt PRINTNL %0
line 2
rt %5 INPUT$ %1 %0
rt PRINTS %0 %3
rt PRINTS %0 %5
rt PRINTS %0 %4
rt PRINTNL %0
system
endfunc
"#;

#[test]
fn input_dollar_without_a_file_number_reads_stdin() {
    assert_eq!(run("q5", INPUTN, b"xy"), "[x]\n[y]\n<exit 0>");
}

#[test]
fn input_dollar_at_end_of_stdin_is_empty() {
    assert_eq!(run("q5e", INPUTN, b"x"), "[x]\n[]\n<exit 0>");
}
