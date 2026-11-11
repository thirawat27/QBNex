//! Strings kept in memory (array elements, STATIC strings) and global strings:
//! loads and stores reuse buffers (R14 performance work), so appending to an
//! element, copying one element to another, assigning a string to itself and
//! comparing elements must keep QBasic's values, interpreted and with the JIT.
//! `memory_strings.qir` is the compiler's output for `memory_strings.bas`.

use qb_exec::{compile, Vm};
use qb_jit::tier::Mode;

const PROG: &str = include_str!("fixtures/memstr/memory_strings.qir");
const EXPECTED: &str = " 80 -1  2 \nzzz 3 \nzzzzzz 6 \nxyxy\n";

fn run(mode: Mode) -> (String, Result<i32, (i32, u32)>) {
    let m = compile(PROG).unwrap_or_else(|e| panic!("load: {e:?}"));
    let mut out = Vec::new();
    let r = {
        let mut vm = Vm::new(&m, &mut out);
        vm.set_jit(mode, 1);
        vm.run_status().map_err(|e| (e.code, e.line))
    };
    (String::from_utf8_lossy(&out).replace("\r\n", "\n"), r)
}

#[test]
fn memory_and_global_strings_keep_their_values() {
    assert_eq!(run(Mode::Off), (EXPECTED.to_string(), Ok(0)));
}

#[test]
fn memory_and_global_strings_keep_their_values_jit() {
    assert_eq!(run(Mode::Eager), (EXPECTED.to_string(), Ok(0)));
}
