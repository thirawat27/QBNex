use syntax_tree::Parser;
use vm_engine::{BytecodeCompiler, OpCode, VM};

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
    assert!(bytecode.iter().any(|op| matches!(op, OpCode::Window { .. })));
    assert!(bytecode.iter().any(|op| matches!(op, OpCode::GetImage { array, .. } if array == "sprite")));
    assert!(bytecode.iter().any(|op| matches!(op, OpCode::PutImage { array, action, .. } if array == "sprite" && action == "XOR")));
}
