use native_codegen::CodeGenerator;
use syntax_tree::Parser;

fn generate(source: &str) -> String {
    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();
    let mut codegen = CodeGenerator::new();
    codegen.generate(&program).unwrap()
}

#[test]
fn loop_local_gosub_codegen_compiles() {
    let code = generate("WHILE -1\nGOSUB work\nWEND\nwork:\nRETURN");
    assert_generated_rust_compiles(&code, "loop_local_gosub_codegen_compiles");
}

#[test]
fn loop_local_on_gosub_codegen_compiles() {
    let code = generate("FOR I = 1 TO 1\nON I GOSUB work\nNEXT\nEND\nwork:\nRETURN");
    assert_generated_rust_compiles(&code, "loop_local_on_gosub_codegen_compiles");
}

#[test]
fn loop_local_on_error_codegen_compiles() {
    let code = generate("DO\nON ERROR GOTO handler\nERROR 5\nLOOP\nhandler:\nRESUME NEXT");
    assert_generated_rust_compiles(&code, "loop_local_on_error_codegen_compiles");
}

#[test]
fn loop_local_timer_codegen_compiles() {
    let code = generate(
        "FOR I = 1 TO 2\nON TIMER(0) GOSUB tick\nTIMER ON\nNEXT\nEND\ntick:\nTIMER OFF\nRETURN",
    );
    assert_generated_rust_compiles(&code, "loop_local_timer_codegen_compiles");
}

fn assert_generated_rust_compiles(code: &str, test_name: &str) {
    let temp_dir = std::env::temp_dir();
    let rust_path = temp_dir.join(format!("{}_{}.rs", test_name, std::process::id()));
    let exe_path = temp_dir.join(format!("{}_{}.exe", test_name, std::process::id()));

    std::fs::write(&rust_path, code).unwrap();

    let output = std::process::Command::new("rustc")
        .arg("--edition=2024")
        .arg(&rust_path)
        .arg("-o")
        .arg(&exe_path)
        .output()
        .unwrap();

    if !output.status.success() {
        panic!(
            "generated Rust failed to compile\nstdout:\n{}\nstderr:\n{}\nsource:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
            code
        );
    }

    let _ = std::fs::remove_file(&rust_path);
    let _ = std::fs::remove_file(&exe_path);
}
