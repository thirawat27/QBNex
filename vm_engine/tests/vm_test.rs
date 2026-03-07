use core_types::QType;
use std::time::{Duration, Instant};
use syntax_tree::Parser;
use vm_engine::builtin_functions::get_builtin_arity;
use vm_engine::opcodes::ByRefTarget;
use vm_engine::{BytecodeCompiler, OpCode, VM};

fn numeric_equals(value: &QType, expected: f64) -> bool {
    match value {
        QType::Integer(v) => (*v as f64 - expected).abs() < f64::EPSILON,
        QType::Long(v) => (*v as f64 - expected).abs() < f64::EPSILON,
        QType::Single(v) => (*v as f64 - expected).abs() < f64::EPSILON,
        QType::Double(v) => (*v - expected).abs() < f64::EPSILON,
        _ => false,
    }
}

#[test]
fn test_vm_basic_arithmetic() {
    let bytecode = vec![
        OpCode::LoadConstant(core_types::QType::Integer(10)),
        OpCode::LoadConstant(core_types::QType::Integer(20)),
        OpCode::Add,
        OpCode::End,
    ];

    let mut vm = VM::new(bytecode);
    let result = vm.run();
    assert!(result.is_ok());
}

#[test]
fn test_vm_variable_storage() {
    let bytecode = vec![
        OpCode::LoadConstant(core_types::QType::Integer(42)),
        OpCode::StoreVariable("x".to_string()),
        OpCode::LoadVariable("x".to_string()),
        OpCode::End,
    ];

    let mut vm = VM::new(bytecode);
    let result = vm.run();
    assert!(result.is_ok());
}

#[test]
fn test_compile_simple_program() {
    let source = "x = 10\ny = 20\nz = x + y";
    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile();
    assert!(bytecode.is_ok());
}

#[test]
fn test_vm_comparison() {
    let bytecode = vec![
        OpCode::LoadConstant(core_types::QType::Integer(10)),
        OpCode::LoadConstant(core_types::QType::Integer(20)),
        OpCode::LessThan,
        OpCode::End,
    ];

    let mut vm = VM::new(bytecode);
    let result = vm.run();
    assert!(result.is_ok());
}

#[test]
fn test_vm_string_operations() {
    let bytecode = vec![
        OpCode::LoadConstant(core_types::QType::String("Hello".to_string())),
        OpCode::LoadConstant(core_types::QType::String(" World".to_string())),
        OpCode::Add,
        OpCode::End,
    ];

    let mut vm = VM::new(bytecode);
    let result = vm.run();
    assert!(result.is_ok());
}

#[test]
fn test_compile_graphics_image_and_view_statements() {
    let source = "\
VIEW (1, 2)-(30, 40), 5, 6
WINDOW (0, 0)-(319, 199)
GET (0, 0)-(10, 10), sprite
PUT (20, 20), sprite, XOR";

    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    assert!(bytecode.iter().any(|op| matches!(op, OpCode::View { .. })));
    assert!(bytecode
        .iter()
        .any(|op| matches!(op, OpCode::Window { .. })));
    assert!(bytecode
        .iter()
        .any(|op| matches!(op, OpCode::GetImage { array, .. } if array == "sprite")));
    assert!(bytecode.iter().any(|op| matches!(op, OpCode::PutImage { array, action, .. } if array == "sprite" && action == "XOR")));
}

#[test]
fn test_compile_vm_supports_extended_control_and_system_statements() {
    let source = "\
CLEAR
WIDTH 80
KEY ON
ON choice GOTO One, Two
DO
    EXIT DO
LOOP
One:
PRINT \"one\"
Two:
PRINT \"two\"";

    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    assert!(bytecode.iter().any(|op| matches!(op, OpCode::Clear)));
    assert!(bytecode.iter().any(|op| matches!(op, OpCode::Dup)));
}

#[test]
fn test_vm_random_field_round_trip() {
    let path = std::env::temp_dir().join(format!("qbnex_vm_field_{}.dat", std::process::id()));
    let path_str = path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        "OPEN \"{}\" FOR RANDOM AS #1 LEN = 4\nFIELD #1, 4 AS A$\nLSET A$ = \"XY\"\nPUT #1, 1\nLSET A$ = \"\"\nGET #1, 1\nCLOSE #1",
        path_str
    );

    let mut parser = Parser::new(source).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| matches!(value, QType::String(s) if s.starts_with("XY"))));

    let _ = std::fs::remove_file(path);
}

#[test]
fn test_vm_input_file_dynamic_round_trip() {
    let path = std::env::temp_dir().join(format!("qbnex_vm_input_{}.txt", std::process::id()));
    std::fs::write(&path, "\"hello\",42\n").unwrap();
    let path_str = path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        "OPEN \"{}\" FOR INPUT AS #1\nINPUT #1, A$, N\nCLOSE #1",
        path_str
    );

    let mut parser = Parser::new(source).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| matches!(value, QType::String(s) if s == "hello")));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| matches!(value, QType::String(s) if s == "42")));

    let _ = std::fs::remove_file(path);
}

#[test]
fn test_vm_compile_for_each() {
    let source = "DIM arr(3)\nFOR EACH item IN arr\nPRINT item\nNEXT";
    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    assert!(bytecode.iter().any(|op| matches!(op, OpCode::UBound(_, _))));
    assert!(bytecode
        .iter()
        .any(|op| matches!(op, OpCode::ArrayLoad(_, 1))));
}

#[test]
fn test_vm_user_defined_function_call_round_trip() {
    let source = "\
DECLARE FUNCTION INC!(X!)
A = INC!(41)
FUNCTION INC!(X!)
INC! = X! + 1
END FUNCTION";
    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    assert!(bytecode
        .iter()
        .any(|op| matches!(op, OpCode::DefineFunction { name, .. } if name == "INC!")));
    assert!(bytecode
        .iter()
        .any(|op| matches!(op, OpCode::CallFunction { name, .. } if name == "INC!")));

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    assert!(vm.runtime.globals.iter().any(|value| match value {
        QType::Integer(v) => *v == 42,
        QType::Long(v) => *v == 42,
        QType::Single(v) => (*v - 42.0).abs() < f32::EPSILON,
        QType::Double(v) => (*v - 42.0).abs() < f64::EPSILON,
        _ => false,
    }));
}

#[test]
fn test_vm_user_defined_sub_call_round_trip() {
    let source = "\
DECLARE SUB SETX(X!)
A = 5
CALL SETX(7)
SUB SETX(X!)
    A = X! * 2
END SUB";
    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    assert!(bytecode
        .iter()
        .any(|op| matches!(op, OpCode::DefineSub { name, .. } if name == "SETX")));
    assert!(bytecode
        .iter()
        .any(|op| matches!(op, OpCode::CallSub { name, .. } if name == "SETX")));

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    assert!(vm.runtime.globals.iter().any(|value| match value {
        QType::Integer(v) => *v == 14,
        QType::Long(v) => *v == 14,
        QType::Single(v) => (*v - 14.0).abs() < f32::EPSILON,
        QType::Double(v) => (*v - 14.0).abs() < f64::EPSILON,
        _ => false,
    }));
}

#[test]
fn test_vm_user_defined_function_call_is_case_insensitive() {
    let source = "\
DECLARE FUNCTION Inc!(X!)
A = inc!(41)
FUNCTION INC!(X!)
INC! = X! + 1
END FUNCTION";
    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    assert!(vm.runtime.globals.iter().any(|value| match value {
        QType::Integer(v) => *v == 42,
        QType::Long(v) => *v == 42,
        QType::Single(v) => (*v - 42.0).abs() < f32::EPSILON,
        QType::Double(v) => (*v - 42.0).abs() < f64::EPSILON,
        _ => false,
    }));
}

#[test]
fn test_vm_sub_call_copies_back_byref_argument() {
    let source = "\
DECLARE SUB INC(X!)
A = 10
CALL INC(A)
SUB INC(X!)
    X! = X! + 1
END SUB";
    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    assert!(bytecode.iter().any(
        |op| matches!(op, OpCode::CallSub { name, by_ref } if name == "INC" && by_ref.iter().any(|slot| !matches!(slot, ByRefTarget::None)))
    ));

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    assert!(vm.runtime.globals.iter().any(|value| match value {
        QType::Integer(v) => *v == 11,
        QType::Long(v) => *v == 11,
        QType::Single(v) => (*v - 11.0).abs() < f32::EPSILON,
        QType::Double(v) => (*v - 11.0).abs() < f64::EPSILON,
        _ => false,
    }));
}

#[test]
fn test_vm_function_call_copies_back_byref_argument() {
    let source = "\
DECLARE FUNCTION BUMP!(X!)
A = 10
B = BUMP!(A)
FUNCTION BUMP!(X!)
    X! = X! + 1
    BUMP! = X!
END FUNCTION";
    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    assert!(bytecode.iter().any(
        |op| matches!(op, OpCode::CallFunction { name, by_ref } if name == "BUMP!" && by_ref.iter().any(|slot| !matches!(slot, ByRefTarget::None)))
    ));

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    let elevens = vm
        .runtime
        .globals
        .iter()
        .filter(|value| match value {
            QType::Integer(v) => *v == 11,
            QType::Long(v) => *v == 11,
            QType::Single(v) => (*v - 11.0).abs() < f32::EPSILON,
            QType::Double(v) => (*v - 11.0).abs() < f64::EPSILON,
            _ => false,
        })
        .count();
    assert!(elevens >= 2);
}

#[test]
fn test_vm_sub_call_copies_back_byref_array_element_argument() {
    let source = "\
DECLARE SUB INC(X!)
DIM A(2)
A(1) = 10
CALL INC(A(1))
SUB INC(X!)
    X! = X! + 5
END SUB";
    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    assert!(bytecode.iter().any(
        |op| matches!(op, OpCode::CallSub { name, by_ref } if name == "INC" && by_ref.iter().any(|slot| matches!(slot, ByRefTarget::ArrayElement { .. })))
    ));

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    let array = vm.runtime.arrays.get("A").unwrap();
    assert!(array
        .get(1)
        .is_some_and(|value| numeric_equals(value, 15.0)));
}

#[test]
fn test_vm_function_call_copies_back_byref_array_element_argument() {
    let source = "\
DECLARE FUNCTION BUMP!(X!)
DIM A(2)
A(1) = 10
R = BUMP!(A(1))
FUNCTION BUMP!(X!)
    X! = X! + 2
    BUMP! = X!
END FUNCTION";
    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    let array = vm.runtime.arrays.get("A").unwrap();
    assert!(array
        .get(1)
        .is_some_and(|value| numeric_equals(value, 12.0)));
    assert!(vm.runtime.globals.iter().any(|value| match value {
        QType::Integer(v) => *v == 12,
        QType::Long(v) => *v == 12,
        QType::Single(v) => (*v - 12.0).abs() < f32::EPSILON,
        QType::Double(v) => (*v - 12.0).abs() < f64::EPSILON,
        _ => false,
    }));
}

#[test]
fn test_vm_sub_call_copies_back_byref_string_array_element_argument() {
    let source = "\
DECLARE SUB APPENDTXT(X$)
DIM A$(2)
A$(1) = \"HI\"
CALL APPENDTXT(A$(1))
SUB APPENDTXT(X$)
    X$ = X$ + \"!\"
END SUB";
    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    let array = vm.runtime.arrays.get("A$").unwrap();
    assert!(matches!(array.get(1), Some(QType::String(s)) if s == "HI!"));
}

#[test]
fn test_vm_nested_sub_calls_preserve_byref_array_element_updates() {
    let source = "\
DECLARE SUB OUTER(X!)
DECLARE SUB INNER(X!)
DIM A(2)
A(1) = 3
CALL OUTER(A(1))
SUB OUTER(X!)
    CALL INNER(X!)
END SUB
SUB INNER(X!)
    X! = X! * 4
END SUB";
    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    let array = vm.runtime.arrays.get("A").unwrap();
    assert!(array
        .get(1)
        .is_some_and(|value| numeric_equals(value, 12.0)));
}

#[test]
fn test_vm_cursor_builtins_follow_runtime_cursor_state() {
    let bytecode = vec![
        OpCode::Locate(5, 10),
        OpCode::CsrLinFunc,
        OpCode::StoreVariable("ROW".to_string()),
        OpCode::PosFunc(0),
        OpCode::StoreVariable("COL".to_string()),
        OpCode::LoadConstant(QType::String("ABC".to_string())),
        OpCode::Print,
        OpCode::PosFunc(0),
        OpCode::StoreVariable("AFTERCOL".to_string()),
        OpCode::End,
    ];

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    assert!(matches!(
        vm.runtime.variables.get("ROW"),
        Some(QType::Integer(5))
    ));
    assert!(matches!(
        vm.runtime.variables.get("COL"),
        Some(QType::Integer(10))
    ));
    assert!(matches!(
        vm.runtime.variables.get("AFTERCOL"),
        Some(QType::Integer(13))
    ));
}

#[test]
fn test_vm_command_func_returns_process_arguments() {
    let bytecode = vec![
        OpCode::CommandFunc,
        OpCode::StoreVariable("CMD$".to_string()),
        OpCode::End,
    ];

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    let expected = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    assert!(matches!(
        vm.runtime.variables.get("CMD$"),
        Some(QType::String(value)) if value == &expected
    ));
}

#[test]
fn test_builtin_arity_matches_zero_arg_runtime_functions() {
    assert_eq!(get_builtin_arity("TIMER"), Some((0, 0)));
    assert_eq!(get_builtin_arity("DATE$"), Some((0, 0)));
    assert_eq!(get_builtin_arity("TIME$"), Some((0, 0)));
    assert_eq!(get_builtin_arity("CSRLIN"), Some((0, 0)));
    assert_eq!(get_builtin_arity("COMMAND$"), Some((0, 0)));
}

#[test]
fn test_vm_defseg_poke_peek_round_trip() {
    let bytecode = vec![
        OpCode::DefSeg(0x2000),
        OpCode::PokeFunc(16, 123),
        OpCode::PeekFunc(16),
        OpCode::StoreVariable("V".to_string()),
        OpCode::End,
    ];

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    assert!(matches!(
        vm.runtime.variables.get("V"),
        Some(QType::Integer(123))
    ));
    assert_eq!(vm.memory.peek(0x2000, 16), 123);
}

#[test]
fn test_vm_pointer_compatibility_helpers_allocate_stable_pseudo_storage() {
    let bytecode = vec![
        OpCode::VarPtrFunc("A".to_string()),
        OpCode::StoreVariable("PTR".to_string()),
        OpCode::VarSegFunc("A".to_string()),
        OpCode::StoreVariable("SEG".to_string()),
        OpCode::SaddFunc("A$".to_string()),
        OpCode::StoreVariable("ADDR".to_string()),
        OpCode::VarPtrStrFunc("A".to_string()),
        OpCode::StoreVariable("PTR$".to_string()),
        OpCode::End,
    ];

    let mut vm = VM::new(bytecode);
    vm.runtime
        .variables
        .insert("A".to_string(), QType::Integer(42));
    vm.runtime
        .variables
        .insert("A$".to_string(), QType::String("HI".to_string()));
    vm.run().unwrap();

    let ptr = match vm.runtime.variables.get("PTR") {
        Some(QType::Long(value)) => *value,
        other => panic!("unexpected PTR value: {other:?}"),
    };
    let seg = match vm.runtime.variables.get("SEG") {
        Some(QType::Integer(value)) => *value,
        other => panic!("unexpected SEG value: {other:?}"),
    };
    let addr = match vm.runtime.variables.get("ADDR") {
        Some(QType::Long(value)) => *value,
        other => panic!("unexpected ADDR value: {other:?}"),
    };

    assert!(ptr > 0);
    assert_eq!(seg, 0x6000);
    assert_eq!(vm.memory.peek(seg as u16, ptr as u16), 42);
    assert_eq!(
        addr,
        core_types::DosMemory::absolute_address(0x6000, 4) as i32
    );
    assert!(matches!(
        vm.runtime.variables.get("PTR$"),
        Some(QType::String(value)) if value.len() == 4
    ));
}

#[test]
fn test_vm_compiler_emits_pointer_and_dynamic_peek_builtins() {
    let source = "\
A = 42
P = VARPTR(A)
S = VARSEG(A)
Q$ = VARPTR$(A)
X = PEEK(16)";

    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    assert!(bytecode
        .iter()
        .any(|op| matches!(op, OpCode::VarPtrFunc(var) if var.ends_with(":A"))));
    assert!(bytecode
        .iter()
        .any(|op| matches!(op, OpCode::VarSegFunc(var) if var.ends_with(":A"))));
    assert!(bytecode
        .iter()
        .any(|op| matches!(op, OpCode::VarPtrStrFunc(var) if var.ends_with(":A"))));
    assert!(bytecode.iter().any(|op| matches!(op, OpCode::PeekDynamic)));
}

#[test]
fn test_vm_inkey_returns_string_without_error() {
    let bytecode = vec![
        OpCode::InKeyFunc,
        OpCode::StoreVariable("K$".to_string()),
        OpCode::End,
    ];

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    assert!(matches!(
        vm.runtime.variables.get("K$"),
        Some(QType::String(_))
    ));
}

#[test]
fn test_vm_poke_statement_round_trip() {
    let source = "\
DEF SEG = 8192
POKE 16, 123
X = PEEK(16)";

    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    assert!(bytecode.iter().any(|op| matches!(op, OpCode::PokeDynamic)));

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    assert_eq!(vm.memory.peek(8192, 16), 123);
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 123.0)));
}

#[test]
fn test_vm_wait_statement_returns_when_mask_matches() {
    let source = "\
DEF SEG = 8192
POKE 16, 4
WAIT 16, 4
X = PEEK(16)";

    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    assert!(bytecode
        .iter()
        .any(|op| matches!(op, OpCode::WaitDynamic { has_xor: false })));

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 4.0)));
}

#[test]
fn test_vm_bsave_bload_round_trip() {
    let path = std::env::temp_dir().join(format!("qbnex_vm_bload_{}.bin", std::process::id()));
    let path_str = path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        "DEF SEG = 8192\nPOKE 16, 123\nBSAVE \"{}\", 16, 1\nPOKE 16, 0\nBLOAD \"{}\", 16\nX = PEEK(16)",
        path_str, path_str
    );

    let mut parser = Parser::new(source).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    assert_eq!(vm.memory.peek(8192, 16), 123);
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 123.0)));

    let _ = std::fs::remove_file(path);
}

#[test]
fn test_vm_out_inp_round_trip() {
    let source = "\
OUT 100, 77
X = INP(100)";

    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    assert!(bytecode.iter().any(|op| matches!(op, OpCode::OutDynamic)));
    assert!(bytecode.iter().any(|op| matches!(op, OpCode::InpDynamic)));

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 77.0)));
}

#[test]
fn test_vm_lprint_and_lpos_compile_and_run() {
    let source = "\
LPRINT \"X\"
P = LPOS(0)";

    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    assert!(bytecode.iter().any(|op| matches!(op, OpCode::PosFunc(0))));

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 1.0)));
}

#[test]
fn test_vm_view_print_updates_runtime_state_without_marker_output() {
    let mut vm = VM::new(vec![OpCode::ViewPrint { top: 2, bottom: 20 }, OpCode::End]);
    vm.run().unwrap();

    assert_eq!(vm.runtime.view_print_top, Some(2));
    assert_eq!(vm.runtime.view_print_bottom, Some(20));

    let bytecode = vec![
        OpCode::ViewPrint { top: 2, bottom: 20 },
        OpCode::ViewReset,
        OpCode::End,
    ];

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    assert_eq!(vm.runtime.view_print_top, None);
    assert_eq!(vm.runtime.view_print_bottom, None);
}

#[test]
fn test_vm_view_reset_clears_view_print_state() {
    let bytecode = vec![
        OpCode::ViewPrint { top: 2, bottom: 20 },
        OpCode::ViewReset,
        OpCode::End,
    ];

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    assert_eq!(vm.runtime.view_print_top, None);
    assert_eq!(vm.runtime.view_print_bottom, None);
}

#[test]
fn test_vm_sleep_negative_does_not_fake_one_second_delay_in_noninteractive_mode() {
    let bytecode = vec![
        OpCode::LoadConstant(QType::Single(-1.0)),
        OpCode::Sleep,
        OpCode::End,
    ];

    let started = Instant::now();
    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    assert!(
        started.elapsed() < Duration::from_millis(250),
        "negative SLEEP should not block in non-interactive test mode"
    );
}

#[test]
fn test_vm_fixed_length_string_dim_assignment_is_padded_and_truncated() {
    let source = "\
DIM A$ AS STRING * 4
A$ = \"ABCDE\"";

    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    assert!(bytecode.iter().any(
        |op| matches!(op, OpCode::SetStringWidth { width, .. } if *width == 4)
    ));

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| matches!(value, QType::String(s) if s == "ABCD")));
}

#[test]
fn test_vm_fixed_length_string_array_assignment_is_padded() {
    let source = "\
DIM A$(2) AS STRING * 4
A$(1) = \"X\"";

    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    assert!(bytecode.iter().any(
        |op| matches!(op, OpCode::SetStringArrayWidth { name, width } if name == "A$" && *width == 4)
    ));

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    let array = vm.runtime.arrays.get("A$").unwrap();
    assert!(matches!(array.get(1), Some(QType::String(s)) if s == "X   "));
}

#[test]
fn test_vm_fixed_length_string_function_return_and_param_widths_are_preserved() {
    let source = "\
DECLARE FUNCTION PAD$(A$ AS STRING * 4)
DIM S$
S$ = \"Z\"
R$ = PAD$(S$)
FUNCTION PAD$(A$ AS STRING * 4) AS STRING * 4
    A$ = \"ABCDE\"
    PAD$ = A$
END FUNCTION";

    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    let strings: Vec<_> = vm
        .runtime
        .globals
        .iter()
        .filter_map(|value| match value {
            QType::String(s) => Some(s.as_str()),
            _ => None,
        })
        .collect();
    assert!(strings.contains(&"ABCD"));
}

#[test]
fn test_vm_sub_call_evaluates_zero_arg_function_argument_by_value() {
    let source = "\
DECLARE FUNCTION NEXTVAL!()
DECLARE SUB CAPTURE(X!)
CALL CAPTURE(NEXTVAL!)
FUNCTION NEXTVAL!
    NEXTVAL! = 7
END FUNCTION
SUB CAPTURE(X!)
    R = X!
END SUB";

    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    assert!(bytecode.iter().any(
        |op| matches!(op, OpCode::CallSub { name, by_ref } if name == "CAPTURE" && by_ref.iter().all(|slot| matches!(slot, ByRefTarget::None)))
    ));

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    assert!(vm.runtime.globals.iter().any(|value| match value {
        QType::Integer(v) => *v == 7,
        QType::Long(v) => *v == 7,
        QType::Single(v) => (*v - 7.0).abs() < f32::EPSILON,
        QType::Double(v) => (*v - 7.0).abs() < f64::EPSILON,
        _ => false,
    }));
}

#[test]
fn test_vm_sub_call_evaluates_builtin_argument_by_value() {
    let source = "\
DECLARE SUB CAPTURE(X!)
CALL CAPTURE(ABS(-3))
SUB CAPTURE(X!)
    R = X!
END SUB";

    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    assert!(bytecode.iter().any(
        |op| matches!(op, OpCode::CallSub { name, by_ref } if name == "CAPTURE" && by_ref.iter().all(|slot| matches!(slot, ByRefTarget::None)))
    ));

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    assert!(vm.runtime.globals.iter().any(|value| match value {
        QType::Integer(v) => *v == 3,
        QType::Long(v) => *v == 3,
        QType::Single(v) => (*v - 3.0).abs() < f32::EPSILON,
        QType::Double(v) => (*v - 3.0).abs() < f64::EPSILON,
        _ => false,
    }));
}
