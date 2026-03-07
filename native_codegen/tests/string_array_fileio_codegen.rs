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
fn string_array_get_put_runs() {
    let source = r#"
DIM B$(2)
B$(1) = "ABCD"
OPEN "string_array_test.bin" FOR BINARY AS #1
PUT #1, 1, B$(1)
B$(2) = "ZZZZ"
GET #1, 1, B$(2)
CLOSE #1
PRINT B$(2)
"#;

    let code = generate(source);
    let temp_dir = std::env::temp_dir().join(format!("qbnex_string_array_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let rust_path = temp_dir.join("string_array_test.rs");
    let exe_path = temp_dir.join("string_array_test.exe");
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

    let stdout = String::from_utf8_lossy(&run.stdout).replace('\r', "");
    assert_eq!(stdout, "ABCD\n");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn string_array_get_put_evaluates_index_once() {
    let source = r#"
DECLARE FUNCTION IDX%()
DIM B$(3)
B$(1) = "ABCD"
B$(2) = "WXYZ"
OPEN "string_array_index_once.bin" FOR BINARY AS #1
PUT #1, 1, B$(IDX%)
GET #1, 1, B$(IDX%)
CLOSE #1
PRINT B$(2)
PRINT B$(3)
FUNCTION IDX% STATIC
c% = c% + 1
IDX% = c%
END FUNCTION
"#;

    let stdout = compile_and_run(source, "string_array_index_once");
    assert_eq!(stdout, "ABCD\n\n");
}

#[test]
fn numeric_array_get_put_evaluates_index_once() {
    let source = r#"
DECLARE FUNCTION IDX%()
DIM A(3)
A(1) = 12
A(2) = 34
OPEN "numeric_array_index_once.bin" FOR BINARY AS #1
PUT #1, 1, A(IDX%)
GET #1, 1, A(IDX%)
CLOSE #1
PRINT A(2)
PRINT A(3)
FUNCTION IDX% STATIC
c% = c% + 1
IDX% = c%
END FUNCTION
"#;

    let stdout = compile_and_run(source, "numeric_array_index_once");
    assert_eq!(stdout, "12\n0\n");
}
