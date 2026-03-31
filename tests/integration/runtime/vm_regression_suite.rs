use core_types::QType;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use syntax_tree::{parse_with_frontend, FrontendKind, Parser};
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

fn run_source(source: &str) -> VM {
    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();
    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();
    let mut vm = VM::new(bytecode);
    vm.run().unwrap();
    vm
}

fn run_source_with_captured_stdout(source: &str) -> String {
    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();
    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();
    let mut vm = VM::new(bytecode);
    vm.enable_stdout_capture();
    vm.run().unwrap();
    vm.take_captured_stdout()
        .replace("\r\n", "\n")
        .replace('\r', "")
}

fn run_checked_source_with_captured_stdout(source: &str) -> String {
    let program = parse_with_frontend(FrontendKind::Classic, source.to_string()).unwrap();
    let symbol_table = analyzer::scope::analyze_program(&program).unwrap();
    let mut type_checker = analyzer::TypeChecker::new(symbol_table);
    type_checker.check_program(&program).unwrap();
    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();
    let mut vm = VM::new(bytecode);
    vm.enable_stdout_capture();
    vm.run().unwrap();
    vm.take_captured_stdout()
        .replace("\r\n", "\n")
        .replace('\r', "")
}

fn unique_temp_file(stem: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("qbnex_{stem}_{nanos}.tmp"))
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
fn test_vm_stdout_capture_records_print_output() {
    let output = run_source_with_captured_stdout("PRINT \"QBNex\"\nPRINT 60\n");
    assert_eq!(output, "QBNex\n60\n");
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
fn test_vm_dynamic_graphics_and_audio_statements_execute_without_stack_leaks() {
    let vm = run_source(
        r#"
screenMode = 13
SCREEN screenMode
x = 10
y = 12
c = 6
PSET (x, y), c
LINE (x, y)-(x + 4, y), c
CIRCLE (x + 10, y + 8), 3, c
cmd$ = "BM40,20 C7 R3"
DRAW cmd$
melody$ = "R"
PLAY melody$
"#,
    );

    let gfx = vm.graphics.as_ref().expect("graphics runtime");
    assert_eq!(gfx.get_pixel(10, 12), 6);
    assert_eq!(gfx.get_pixel(14, 12), 6);
    assert_eq!(gfx.get_pixel(23, 20), 6);
    assert_eq!(gfx.get_pixel(43, 20), 7);
    assert!(!vm.sound.is_playing());
    assert!(vm.runtime.value_stack.is_empty());
}

#[test]
fn test_vm_cls_1_clears_active_graphics_viewport_only() {
    let vm = run_source(
        r#"
SCREEN 13
VIEW (10, 10)-(20, 20), 0, 0
PSET (15, 15), 12
CLS 1
"#,
    );

    let gfx = vm.graphics.as_ref().expect("graphics runtime");
    assert_eq!(gfx.get_pixel(15, 15), 0);
    assert!(gfx.viewport.active);
}

#[test]
fn test_vm_dynamic_graphics_functions_and_system_helpers() {
    let vm = run_source(
        r#"
screenMode = 13
SCREEN screenMode
vx1 = 10
vy1 = 10
vx2 = 110
vy2 = 110
VIEW (vx1, vy1)-(vx2, vy2), 0, 0
wx1 = -10
wy1 = -10
wx2 = 10
wy2 = 10
WINDOW (wx1, wy1)-(wx2, wy2)
x = 0
y = 0
shade = 12
PSET (x, y), shade
IF POINT(x, y) <> 12 THEN ERROR 61
IF PMAP(x, 0) <> 60 THEN ERROR 62
IF PMAP(y, 1) <> 60 THEN ERROR 63
IF PMAP(PMAP(x, 0), 2) <> 0 THEN ERROR 64
IF PMAP(PMAP(y, 1), 3) <> 0 THEN ERROR 65
row = 5
col = 9
LOCATE row, col
probe = 0
IF POS(probe) <> col THEN ERROR 66
probe$ = ""
IF FRE(probe$) <> 524288 THEN ERROR 67
segmentValue = 8192
addr = 16
DEF SEG = segmentValue
POKE addr, 77
IF PEEK(addr) <> 77 THEN ERROR 68
"#,
    );

    let gfx = vm.graphics.as_ref().expect("graphics runtime");
    assert_eq!(gfx.get_pixel(0, 0), 12);
    assert_eq!(vm.runtime.cursor_col, 9);
    assert_eq!(vm.runtime.current_segment, 8192);
    assert!(vm.runtime.value_stack.is_empty());
}

#[test]
fn test_vm_graphics_modes_write_bottom_right_pixels() {
    let cases = [
        (1, 319, 199, 1),
        (2, 639, 199, 1),
        (4, 319, 199, 2),
        (5, 319, 199, 3),
        (6, 639, 199, 1),
        (7, 319, 199, 4),
        (8, 639, 199, 5),
        (9, 639, 349, 6),
        (10, 639, 349, 3),
        (11, 639, 479, 1),
        (12, 639, 479, 2),
        (13, 319, 199, 7),
    ];

    for (mode, x, y, color) in cases {
        let source = format!("SCREEN {mode}\nPSET ({x}, {y}), {color}\n");
        let vm = run_source(&source);
        let gfx = vm.graphics.as_ref().expect("graphics runtime");
        assert_eq!(gfx.screen_mode, mode as u8, "mode {mode} screen selection");
        assert_eq!(gfx.get_pixel(x, y), color as u8, "mode {mode} pset result");
    }
}

#[test]
fn test_vm_graphics_modes_point_reads_bottom_right_pixels() {
    let cases = [
        (1, 319, 199, 1),
        (2, 639, 199, 1),
        (4, 319, 199, 2),
        (5, 319, 199, 3),
        (6, 639, 199, 1),
        (7, 319, 199, 4),
        (8, 639, 199, 5),
        (9, 639, 349, 6),
        (10, 639, 349, 3),
        (11, 639, 479, 1),
        (12, 639, 479, 2),
        (13, 319, 199, 7),
    ];

    for (mode, x, y, color) in cases {
        let source = format!("SCREEN {mode}\nPSET ({x}, {y}), {color}\nPRINT POINT({x}, {y})\n");
        let stdout = run_source_with_captured_stdout(&source);
        assert_eq!(stdout.trim(), color.to_string(), "mode {mode} point result");
    }
}

#[test]
fn test_checked_pipeline_graphics_modes_point_reads_bottom_right_pixels() {
    let cases = [
        (1, 319, 199, 1),
        (2, 639, 199, 1),
        (4, 319, 199, 2),
        (5, 319, 199, 3),
        (6, 639, 199, 1),
        (7, 319, 199, 4),
        (8, 639, 199, 5),
        (9, 639, 349, 6),
        (10, 639, 349, 3),
        (11, 639, 479, 1),
        (12, 639, 479, 2),
        (13, 319, 199, 7),
    ];

    for (mode, x, y, color) in cases {
        let source = format!("SCREEN {mode}\nPSET ({x}, {y}), {color}\nPRINT POINT({x}, {y})\n");
        let stdout = run_checked_source_with_captured_stdout(&source);
        assert_eq!(
            stdout.trim(),
            color.to_string(),
            "mode {mode} checked pipeline point result"
        );
    }
}

#[test]
fn test_vm_mid_without_length_and_instr_with_start_argument() {
    let vm = run_source(
        r#"
TAIL$ = MID$("ABCDE", 3)
POS1 = INSTR("ABCDE", "CD")
POS2 = INSTR(2, "ABCDE", "CD")
IF TAIL$ <> "CDE" THEN ERROR 81
IF POS1 <> 3 THEN ERROR 82
IF POS2 <> 3 THEN ERROR 83
"#,
    );

    assert!(vm.runtime.value_stack.is_empty());
}

#[test]
fn test_vm_randomize_and_rnd_argument_forms() {
    let vm = run_source(
        r#"
RANDOMIZE 1234
A = RND(1)
B = RND(0)
RANDOMIZE 1234
C = RND(1)
IF A <> B THEN ERROR 84
IF A <> C THEN ERROR 85
"#,
    );

    assert!(vm.runtime.value_stack.is_empty());
}

#[test]
fn test_vm_input_str_file_mode_reads_requested_bytes() {
    let path = unique_temp_file("input_chars");
    let escaped_path = path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        "\
OPEN \"{escaped_path}\" FOR OUTPUT AS #1
PRINT #1, \"ABCDE\"
CLOSE #1
OPEN \"{escaped_path}\" FOR INPUT AS #1
CHUNK$ = INPUT$(3, 1)
CLOSE #1
IF CHUNK$ <> \"ABC\" THEN ERROR 86
"
    );

    let vm = run_source(&source);
    let _ = fs::remove_file(&path);

    assert!(vm.runtime.value_stack.is_empty());
}

#[test]
fn test_vm_environ_supports_string_and_numeric_forms() {
    let key = format!("QBNEX_ENV_{}", std::process::id());
    let value = "VISIBLE";
    let pair = format!("{key}={value}");
    unsafe {
        std::env::set_var(&key, value);
    }

    let source = format!(
        "\
PROBE$ = \"{pair}\"
FOUND = 0
FOR I = 1 TO 4096
    IF ENVIRON$(I) = PROBE$ THEN FOUND = -1
NEXT I
IF FOUND = 0 THEN ERROR 87
IF ENVIRON$(\"{key}\") <> \"{value}\" THEN ERROR 88
"
    );

    let vm = run_source(&source);
    unsafe {
        std::env::remove_var(&key);
    }

    assert!(vm.runtime.value_stack.is_empty());
}

#[test]
fn test_vm_str_val_and_string_follow_qbasic_semantics() {
    let vm = run_source(
        r#"
A$ = STR$(123)
B = VAL("123ABC")
C = VAL("&H10")
D$ = STRING$(3, 65)
E$ = STRING$(2, "Z")
IF A$ <> " 123" THEN ERROR 89
IF B <> 123 THEN ERROR 90
IF C <> 16 THEN ERROR 91
IF D$ <> "AAA" THEN ERROR 92
IF E$ <> "ZZ" THEN ERROR 93
"#,
    );

    assert!(vm.runtime.value_stack.is_empty());
}

#[test]
fn test_vm_restore_label_and_line_number_reset_data_pointer() {
    let vm = run_source(
        r#"
DATA 1
firstLabel:
marker = 0
DATA 2, 3
READ a
IF a <> 1 THEN ERROR 71
RESTORE firstLabel
READ b
READ c
IF b <> 2 THEN ERROR 72
IF c <> 3 THEN ERROR 73
100 DATA 4
RESTORE 100
READ d
IF d <> 4 THEN ERROR 74
tailLabel:
marker = 1
RESTORE tailLabel
"#,
    );

    assert_eq!(
        vm.runtime.data_pointer.section_index,
        vm.runtime.data_section.len()
    );
    assert_eq!(vm.runtime.data_pointer.value_index, 0);
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
    assert!(bytecode
        .iter()
        .any(|op| matches!(op, OpCode::Width { columns, rows } if *columns == 80 && *rows == 25)));
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
        .any(|value| numeric_equals(value, 42.0)));

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
fn test_vm_compile_rejects_wrong_sub_argument_count() {
    let source = "\
DECLARE SUB INC(X!)
CALL INC(1, 2)
SUB INC(X!)
END SUB";
    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let result = compiler.compile();

    assert!(matches!(
        result,
        Err(core_types::QError::InvalidProcedure(message))
            if message.contains("SUB INC expects 1 argument(s), got 2")
    ));
}

#[test]
fn test_vm_compile_rejects_declared_function_without_definition() {
    let source = "\
DECLARE FUNCTION MISSING!(X!)
PRINT MISSING!(1)";
    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let result = compiler.compile();

    assert!(matches!(
        result,
        Err(core_types::QError::InvalidProcedure(message))
            if message.contains("FUNCTION MISSING! is declared but has no definition")
    ));
}

#[test]
fn test_vm_compile_rejects_declare_definition_signature_mismatch() {
    let source = "\
DECLARE SUB INC(BYVAL X!)
SUB INC(X!)
END SUB";
    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let result = compiler.compile();

    assert!(matches!(
        result,
        Err(core_types::QError::InvalidProcedure(message))
            if message.contains("SUB INC argument 1 declared as BYVAL, defined as BYREF")
    ));
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
fn test_vm_sub_call_respects_byval_parameter_declaration() {
    let source = "\
DECLARE SUB INC(BYVAL X!)
A = 10
CALL INC(A)
B = A
SUB INC(BYVAL X!)
    X! = X! + 1
END SUB";
    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    assert!(bytecode.iter().any(
        |op| matches!(op, OpCode::CallSub { name, by_ref } if name == "INC" && by_ref.iter().all(|slot| matches!(slot, ByRefTarget::None)))
    ));

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    let tens = vm
        .runtime
        .globals
        .iter()
        .filter(|value| numeric_equals(value, 10.0))
        .count();
    assert!(tens >= 2);
    assert!(!vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 11.0)));
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
fn test_vm_function_call_respects_byval_parameter_definition() {
    let source = "\
FUNCTION BUMP!(BYVAL X!)
    X! = X! + 1
    BUMP! = X!
END FUNCTION
A = 10
B = BUMP!(A)
C = A";
    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    assert!(bytecode.iter().any(
        |op| matches!(op, OpCode::CallFunction { name, by_ref } if name == "BUMP!" && by_ref.iter().all(|slot| matches!(slot, ByRefTarget::None)))
    ));

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    let tens = vm
        .runtime
        .globals
        .iter()
        .filter(|value| numeric_equals(value, 10.0))
        .count();
    let elevens = vm
        .runtime
        .globals
        .iter()
        .filter(|value| numeric_equals(value, 11.0))
        .count();
    assert!(tens >= 2);
    assert!(elevens >= 1);
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
    assert_eq!(get_builtin_arity("RND"), Some((0, 1)));
    assert_eq!(get_builtin_arity("INSTR"), Some((2, 3)));
    assert_eq!(get_builtin_arity("SPACE$"), Some((1, 1)));
}

#[test]
fn test_vm_executes_def_fn_calls_with_arguments() {
    let vm = run_source(
        r#"
DEF FNTWICE(X) = X * 2
RESULT = FNTWICE(5)
"#,
    );

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 10.0)));
}

#[test]
fn test_vm_def_fn_can_capture_outer_const_values() {
    let vm = run_source(
        r#"
CONST MY_PI = 3.141592653589793
CLEAR
DEF FNAREA(R) = MY_PI * (R ^ 2)
RESULT = FNAREA(5)
"#,
    );

    assert!(
        vm.runtime.globals.iter().any(|value| match value {
            QType::Integer(v) => (*v as f64 - 78.53981633974483).abs() < 1e-4,
            QType::Long(v) => (*v as f64 - 78.53981633974483).abs() < 1e-4,
            QType::Single(v) => (*v as f64 - 78.53981633974483).abs() < 1e-4,
            QType::Double(v) => (*v - 78.53981633974483).abs() < 1e-4,
            _ => false,
        }),
        "expected DEF FN result to use outer CONST, globals={:?}",
        vm.runtime.globals
    );
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
fn test_vm_inkey_noninteractive_fallback_is_one_shot() {
    let bytecode = vec![
        OpCode::InKeyFunc,
        OpCode::StoreVariable("K1$".to_string()),
        OpCode::InKeyFunc,
        OpCode::StoreVariable("K2$".to_string()),
        OpCode::End,
    ];

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    assert!(matches!(
        vm.runtime.variables.get("K1$"),
        Some(QType::String(value)) if !value.is_empty()
    ));
    assert!(matches!(
        vm.runtime.variables.get("K2$"),
        Some(QType::String(value)) if value.is_empty()
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

    assert!(bytecode.iter().any(|op| matches!(op, OpCode::LPosFunc(0))));

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 1.0)));
}

#[test]
fn test_vm_locate_zero_arguments_preserve_current_row_and_column() {
    let mut vm = VM::new(vec![
        OpCode::Locate(5, 7),
        OpCode::Locate(0, 9),
        OpCode::CsrLinFunc,
        OpCode::PosFunc(0),
        OpCode::Locate(6, 0),
        OpCode::CsrLinFunc,
        OpCode::PosFunc(0),
        OpCode::End,
    ]);

    vm.run().unwrap();

    assert_eq!(vm.runtime.cursor_row, 6);
    assert_eq!(vm.runtime.cursor_col, 9);
    assert_eq!(vm.runtime.value_stack.len(), 4);
    assert!(numeric_equals(&vm.runtime.value_stack[0], 5.0));
    assert!(numeric_equals(&vm.runtime.value_stack[1], 9.0));
    assert!(numeric_equals(&vm.runtime.value_stack[2], 6.0));
    assert!(numeric_equals(&vm.runtime.value_stack[3], 9.0));
}

#[test]
fn test_vm_locate_cursor_arguments_compile_and_update_runtime_state() {
    let source = "\
LOCATE , , 0, 5, 6
LOCATE , , 1";

    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    assert!(bytecode.iter().any(|op| matches!(
        op,
        OpCode::SetCursorState {
            visible: 0,
            start: 5,
            stop: 6
        }
    )));
    assert!(bytecode.iter().any(|op| matches!(
        op,
        OpCode::SetCursorState {
            visible: 1,
            start: -1,
            stop: -1
        }
    )));

    let mut vm = VM::new(bytecode);
    vm.run().unwrap();

    assert!(vm.runtime.cursor_visible);
    assert_eq!(vm.runtime.cursor_start, Some(5));
    assert_eq!(vm.runtime.cursor_stop, Some(6));
}

#[test]
fn test_vm_locate_does_not_emit_ansi_when_stdout_is_captured() {
    let stdout = run_source_with_captured_stdout(
        "\
LOCATE 5, 9
PRINT POS(0)
",
    );

    assert_eq!(stdout, "9\n");
}

#[test]
fn test_vm_view_print_updates_runtime_state_without_marker_output() {
    let mut vm = VM::new(vec![OpCode::ViewPrint { top: 2, bottom: 20 }, OpCode::End]);
    vm.run().unwrap();

    assert_eq!(vm.runtime.view_print_top, Some(2));
    assert_eq!(vm.runtime.view_print_bottom, Some(20));
    assert_eq!(vm.runtime.cursor_row, 2);
    assert_eq!(vm.runtime.cursor_col, 1);

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
fn test_vm_width_tracks_text_columns_and_wraps_exact_line_length() {
    let vm = run_source(
        r#"
WIDTH 10
PRINT STRING$(10, "A");
"#,
    );

    assert_eq!(vm.runtime.text_columns, 10);
    assert_eq!(vm.runtime.cursor_row, 2);
    assert_eq!(vm.runtime.cursor_col, 1);
}

#[test]
fn test_vm_screen_mode_resets_text_geometry_and_view_print_state() {
    let vm = run_source(
        r#"
WIDTH 10
VIEW PRINT 3 TO 4
SCREEN 0
"#,
    );

    assert_eq!(vm.runtime.text_columns, 80);
    assert_eq!(vm.runtime.text_rows, 25);
    assert_eq!(vm.runtime.view_print_top, None);
    assert_eq!(vm.runtime.view_print_bottom, None);
    assert_eq!(vm.runtime.cursor_row, 1);
    assert_eq!(vm.runtime.cursor_col, 1);

    let vm = run_source(
        r#"
SCREEN 13
"#,
    );
    assert_eq!(vm.runtime.text_columns, 40);
    assert_eq!(vm.runtime.text_rows, 25);

    let vm = run_source(
        r#"
SCREEN 12
"#,
    );
    assert_eq!(vm.runtime.text_columns, 80);
    assert_eq!(vm.runtime.text_rows, 30);
}

#[test]
fn test_vm_cls_respects_text_viewport_and_full_screen_modes() {
    let vm = run_source(
        r#"
VIEW PRINT 3 TO 4
LOCATE 4, 5
CLS 2
"#,
    );
    assert_eq!(vm.runtime.cursor_row, 3);
    assert_eq!(vm.runtime.cursor_col, 1);

    let vm = run_source(
        r#"
VIEW PRINT 3 TO 4
LOCATE 4, 5
CLS 0
"#,
    );
    assert_eq!(vm.runtime.cursor_row, 1);
    assert_eq!(vm.runtime.cursor_col, 1);
}

#[test]
fn test_vm_screen_function_reads_text_buffer_character_and_attribute() {
    let vm = run_source(
        r#"
PRINT "A";
CH = SCREEN(1, 1)
ATTR = SCREEN(1, 1, 1)
"#,
    );

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 65.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 7.0)));
}

#[test]
fn test_vm_view_print_defers_scroll_until_next_output() {
    let vm = run_source(
        r#"
VIEW PRINT 2 TO 3
PRINT "A"
PRINT "B"
TOPVAL = SCREEN(2, 1)
BOTVAL = SCREEN(3, 1)
ROWVAL = CSRLIN
"#,
    );

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 65.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 66.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 3.0)));
}

#[test]
fn test_vm_view_print_scrolls_when_next_output_arrives() {
    let vm = run_source(
        r#"
VIEW PRINT 2 TO 3
PRINT "A"
PRINT "B"
PRINT "C"
TOPVAL = SCREEN(2, 1)
BOTVAL = SCREEN(3, 1)
ROWVAL = CSRLIN
"#,
    );

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 66.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 67.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 3.0)));
}

#[test]
fn test_vm_view_print_blank_line_triggers_deferred_scroll() {
    let vm = run_source(
        r#"
VIEW PRINT 2 TO 3
PRINT "A"
PRINT "B"
PRINT
TOPVAL = SCREEN(2, 1)
BOTVAL = SCREEN(3, 1)
ROWVAL = CSRLIN
"#,
    );

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 66.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 32.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 3.0)));
}

#[test]
fn test_vm_locate_cancels_pending_view_print_scroll() {
    let vm = run_source(
        r#"
VIEW PRINT 2 TO 3
PRINT "A"
PRINT "B"
LOCATE 2, 5
PRINT "C"
TOPLEFT = SCREEN(2, 1)
TOPMID = SCREEN(2, 5)
BOTLEFT = SCREEN(3, 1)
ROWVAL = CSRLIN
"#,
    );

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 65.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 67.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 66.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 3.0)));
}

#[test]
fn test_vm_full_screen_scrolls_when_next_output_arrives() {
    let vm = run_source(
        r#"
WIDTH 10, 2
PRINT STRING$(10, "A");
PRINT STRING$(10, "B");
PRINT STRING$(10, "C");
TOPVAL = SCREEN(1, 1)
BOTVAL = SCREEN(2, 1)
ROWVAL = CSRLIN
"#,
    );

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 66.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 67.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 2.0)));
}

#[test]
fn test_vm_write_console_scrolls_in_view_print_region() {
    let vm = run_source(
        r#"
VIEW PRINT 2 TO 3
WRITE 1
WRITE 2
WRITE 3
TOPVAL = SCREEN(2, 1)
BOTVAL = SCREEN(3, 1)
ROWVAL = CSRLIN
"#,
    );

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 50.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 51.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 3.0)));
}

#[test]
fn test_vm_write_console_scrolls_in_full_screen_text_mode() {
    let vm = run_source(
        r#"
WIDTH 10, 2
WRITE 1
WRITE 2
WRITE 3
TOPVAL = SCREEN(1, 1)
BOTVAL = SCREEN(2, 1)
ROWVAL = CSRLIN
"#,
    );

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 50.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 51.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 2.0)));
}

#[test]
fn test_vm_print_comma_scrolls_when_zone_wrap_hits_view_print_bottom() {
    let vm = run_source(
        r#"
WIDTH 10, 3
VIEW PRINT 2 TO 3
PRINT "A"
PRINT 1, 2
TOPVAL = SCREEN(2, 1)
BOTVAL = SCREEN(3, 1)
ROWVAL = CSRLIN
"#,
    );

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 49.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 50.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 3.0)));
}

#[test]
fn test_vm_print_using_comma_scrolls_when_zone_wrap_hits_view_print_bottom() {
    let vm = run_source(
        r###"
WIDTH 10, 3
VIEW PRINT 2 TO 3
PRINT USING "##"; 1
PRINT USING "##"; 2, 3
TOPVAL = SCREEN(2, 2)
BOTVAL = SCREEN(3, 2)
ROWVAL = CSRLIN
"###,
    );

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 50.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 51.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 3.0)));
}

#[test]
fn test_vm_lprint_using_comma_updates_printer_column() {
    let vm = run_source(
        r###"
LPRINT USING "##"; 8, 9;
P = LPOS(0)
"###,
    );

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 17.0)));
}

#[test]
fn test_vm_print_using_cycles_across_multiple_fields() {
    let vm = run_source(
        r###"
PRINT USING "Item: ! Qty: ## "; "A"; 1; "B"; 2
FIRST = SCREEN(1, 7)
SECOND = SCREEN(1, 23)
NUM1 = SCREEN(1, 15)
NUM2 = SCREEN(1, 31)
"###,
    );

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 65.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 66.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 49.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 50.0)));
}

#[test]
fn test_vm_print_using_cycles_across_adjacent_mixed_fields() {
    let vm = run_source(
        r###"
PRINT USING "!##"; "A"; 1; "B"; 2
FIRST = SCREEN(1, 1)
SECOND = SCREEN(1, 2)
THIRD = SCREEN(1, 3)
FOURTH = SCREEN(1, 4)
FIFTH = SCREEN(1, 5)
SIXTH = SCREEN(1, 6)
"###,
    );

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 65.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 32.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 49.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 66.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 50.0)));
}

#[test]
fn test_vm_print_using_preserves_literal_commas_around_numeric_fields() {
    let vm = run_source(
        r###"
PRINT USING "Total, ###,"; 12
LEAD_COMMA = SCREEN(1, 6)
DIGIT1 = SCREEN(1, 9)
DIGIT2 = SCREEN(1, 10)
TAIL_COMMA = SCREEN(1, 11)
"###,
    );

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 44.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 49.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 50.0)));
}

#[test]
fn test_vm_print_using_literal_only_format_prints_literal_text() {
    let vm = run_source(
        r###"
PRINT USING "Header:";
PRINT USING "Total:"; 12
H = SCREEN(1, 1)
R = SCREEN(1, 7)
T = SCREEN(2, 1)
C = SCREEN(2, 6)
"###,
    );

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 72.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 58.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 84.0)));
}

#[test]
fn test_vm_on_timer_breaks_out_of_do_while_loop() {
    let vm = run_source(
        r###"
COUNT = 0
ON TIMER(0) GOSUB Tick
TIMER ON
DO WHILE COUNT = 0
LOOP
DONE = COUNT
END
Tick:
COUNT = COUNT + 1
TIMER OFF
RETURN
"###,
    );

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 1.0)));
}

#[test]
fn test_vm_print_tab_scrolls_when_wrap_hits_full_screen_bottom() {
    let vm = run_source(
        r#"
WIDTH 10, 2
PRINT STRING$(10, "A")
PRINT "1"; TAB(1); "2"
TOPVAL = SCREEN(1, 1)
BOTVAL = SCREEN(2, 1)
ROWVAL = CSRLIN
"#,
    );

    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 49.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 50.0)));
    assert!(vm
        .runtime
        .globals
        .iter()
        .any(|value| numeric_equals(value, 2.0)));
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

    assert!(bytecode
        .iter()
        .any(|op| matches!(op, OpCode::SetStringWidth { width, .. } if *width == 4)));

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

#[test]
fn test_vm_option_base_applies_to_declared_and_implicit_arrays() {
    let vm = run_source(
        "\
OPTION BASE 1
DIM A(3)
A(1) = 10
A(3) = 30
B(1) = 7",
    );

    assert_eq!(vm.runtime.array_dimensions.get("A"), Some(&vec![(1, 3)]));
    assert_eq!(vm.runtime.array_dimensions.get("B"), Some(&vec![(1, 6)]));

    let a = vm.runtime.arrays.get("A").expect("declared array A");
    assert!(numeric_equals(&a[0], 10.0));
    assert!(numeric_equals(&a[2], 30.0));

    let b = vm.runtime.arrays.get("B").expect("implicit array B");
    assert!(numeric_equals(&b[0], 7.0));
}

#[test]
fn test_vm_explicit_lower_bounds_and_multidimensional_arrays_work() {
    let vm = run_source(
        "\
DIM M(1 TO 2, 3 TO 4)
M(1, 3) = 11
M(2, 4) = 22",
    );

    assert_eq!(
        vm.runtime.array_dimensions.get("M"),
        Some(&vec![(1, 2), (3, 4)])
    );
    let m = vm.runtime.arrays.get("M").expect("array M");
    assert_eq!(m.len(), 4);
    assert!(numeric_equals(&m[0], 11.0));
    assert!(numeric_equals(&m[3], 22.0));
}

#[test]
fn test_vm_redim_preserve_string_arrays_default_to_empty_strings() {
    let vm = run_source(
        "\
OPTION BASE 1
REDIM A$(2)
A$(1) = \"ALPHA\"
A$(2) = \"BETA\"
REDIM PRESERVE A$(3)",
    );

    assert_eq!(vm.runtime.array_dimensions.get("A$"), Some(&vec![(1, 3)]));

    let values = vm.runtime.arrays.get("A$").expect("string array A$");
    assert_eq!(values.len(), 3);
    assert!(matches!(&values[0], QType::String(text) if text == "ALPHA"));
    assert!(matches!(&values[1], QType::String(text) if text == "BETA"));
    assert!(matches!(&values[2], QType::String(text) if text.is_empty()));
}

#[test]
fn test_vm_declared_string_arrays_without_suffix_use_string_defaults() {
    let vm = run_source(
        "\
DIM A(2) AS STRING
A(1) = \"ALPHA\"",
    );

    assert_eq!(vm.runtime.array_dimensions.get("A"), Some(&vec![(0, 2)]));

    let values = vm.runtime.arrays.get("A").expect("string array A");
    assert_eq!(values.len(), 3);
    assert!(matches!(&values[0], QType::String(text) if text.is_empty()));
    assert!(matches!(&values[1], QType::String(text) if text == "ALPHA"));
    assert!(matches!(&values[2], QType::String(text) if text.is_empty()));
}

#[test]
fn test_vm_compiler_emits_correct_dimension_indexes_for_lbound_and_ubound() {
    let source = "\
DIM M(1 TO 2, 3 TO 4)
A = LBOUND(M)
B = UBOUND(M)
C = LBOUND(M, 1)
D = UBOUND(M, 1)
E = LBOUND(M, 2)
F = UBOUND(M, 2)";

    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile().unwrap();

    assert!(bytecode
        .iter()
        .any(|op| matches!(op, OpCode::LBound(name, dim) if name == "M" && *dim == 0)));
    assert!(bytecode
        .iter()
        .any(|op| matches!(op, OpCode::UBound(name, dim) if name == "M" && *dim == 0)));
    assert_eq!(
        bytecode
            .iter()
            .filter(|op| matches!(op, OpCode::LBound(name, dim) if name == "M" && *dim == 1))
            .count(),
        1
    );
    assert_eq!(
        bytecode
            .iter()
            .filter(|op| matches!(op, OpCode::UBound(name, dim) if name == "M" && *dim == 1))
            .count(),
        1
    );
}
