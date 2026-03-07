use native_codegen::CodeGenerator;
use syntax_tree::Parser;

fn generate(source: &str) -> String {
    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();
    let mut codegen = CodeGenerator::new();
    codegen.generate(&program).unwrap()
}

fn compile_and_run(source: &str, test_name: &str) -> String {
    let code = generate(source);
    let temp_dir = std::env::temp_dir().join(format!("{test_name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let rust_path = temp_dir.join(format!("{test_name}.rs"));
    let exe_path = temp_dir.join(format!("{test_name}.exe"));
    std::fs::write(&rust_path, code).unwrap();

    let compile = std::process::Command::new("rustc")
        .arg("--edition=2024")
        .arg(&rust_path)
        .arg("-o")
        .arg(&exe_path)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "generated Rust failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr)
    );

    let run = std::process::Command::new(&exe_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "generated program failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
    String::from_utf8_lossy(&run.stdout).replace('\r', "")
}

#[test]
fn field_string_variable_get_put_uses_declared_width() {
    let source = r#"
OPEN "field_var_test.dat" FOR RANDOM AS #1 LEN = 4
FIELD #1, 4 AS A$
LSET A$ = "AB"
PUT #1, 1, A$
A$ = ""
GET #1, 1, A$
CLOSE #1
PRINT A$
"#;

    let stdout = compile_and_run(source, "field_string_var_width");
    assert_eq!(stdout, "AB  \n");
}
