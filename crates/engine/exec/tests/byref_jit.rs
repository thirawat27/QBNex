//! BYREF parameters that refer to their argument (`Slot::R`): native code must
//! give exactly the VM's results, also for calls made from native code into
//! a procedure with such a parameter and errors raised inside it.

use std::path::Path;

use qb_exec::{compile, Vm};
use qb_jit::tier::Mode;

fn run_mode(src: &str, mode: Mode) -> (String, Result<i32, (i32, u32)>, u64) {
    let m = compile(src).unwrap_or_else(|e| panic!("load: {e:?}"));
    let mut out = Vec::new();
    let (r, entries) = {
        let mut vm = Vm::new(&m, &mut out);
        vm.set_jit(mode, 1);
        let r = vm.run_status().map_err(|e| (e.code, e.line));
        assert!(vm.jit_failure().is_none(), "JIT compile failed: {:?}", vm.jit_failure());
        (r, vm.jit_entries())
    };
    (String::from_utf8(out).unwrap(), r, entries)
}

#[test]
fn reference_parameter_called_from_native_code() {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/jit/byref_reference_param.qir");
    let src = std::fs::read_to_string(&p).unwrap();
    let vm = run_mode(&src, Mode::Off);
    assert_eq!(vm.0, " 3330  4838160  3 \n");
    for mode in [Mode::Hot, Mode::Eager] {
        let n = run_mode(&src, mode);
        assert_eq!((&n.0, &n.1), (&vm.0, &vm.1), "{mode:?}");
        assert!(n.2 > 0, "native code never entered");
    }
}
