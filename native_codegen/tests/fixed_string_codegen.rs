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
fn fixed_length_string_assignment_is_padded_and_truncated() {
    let source = r#"
DIM A$ AS STRING * 4
A$ = "ABCDE"
PRINT A$
A$ = "X"
PRINT A$
"#;

    let stdout = compile_and_run(source, "fixed_string_assignment");
    assert_eq!(stdout, "ABCD\nX   \n");
}

#[test]
fn fixed_length_string_array_get_put_uses_declared_width() {
    let source = r#"
DIM B$(2) AS STRING * 4
B$(1) = "AB"
OPEN "fixed_string_array.bin" FOR BINARY AS #1
PUT #1, 1, B$(1)
B$(2) = ""
GET #1, 1, B$(2)
CLOSE #1
PRINT B$(2)
"#;

    let stdout = compile_and_run(source, "fixed_string_array_get_put");
    assert_eq!(stdout, "AB  \n");
}

#[test]
fn fixed_length_string_sub_parameter_preserves_width_on_copy_back() {
    let source = r#"
DECLARE SUB Demo(A$ AS STRING * 4)
DIM S$
S$ = "Z"
CALL Demo(S$)
PRINT S$
SUB Demo(A$ AS STRING * 4)
A$ = "ABCDE"
END SUB
"#;

    let stdout = compile_and_run(source, "fixed_string_sub_param");
    assert_eq!(stdout, "ABCD\n");
}

#[test]
fn fixed_length_string_function_parameter_preserves_width_on_copy_back() {
    let source = r#"
DECLARE FUNCTION Demo$(A$ AS STRING * 4)
DIM S$
S$ = "Y"
PRINT Demo$(S$)
PRINT S$
FUNCTION Demo$(A$ AS STRING * 4)
A$ = "X"
Demo$ = A$
END FUNCTION
"#;

    let stdout = compile_and_run(source, "fixed_string_func_param");
    assert_eq!(stdout, "X   \nX   \n");
}

#[test]
fn fixed_length_string_function_return_preserves_declared_width() {
    let source = r#"
PRINT FIXED$
FUNCTION FIXED$() AS STRING * 4
FIXED$ = "ABCDE"
END FUNCTION
"#;

    let stdout = compile_and_run(source, "fixed_string_func_return");
    assert_eq!(stdout, "ABCD\n");
}

#[test]
fn native_sub_call_copies_back_numeric_array_element_argument() {
    let source = r#"
DECLARE SUB INC(X!)
DIM A(2)
A(1) = 10
CALL INC(A(1))
PRINT A(1)
SUB INC(X!)
X! = X! + 5
END SUB
"#;

    let stdout = compile_and_run(source, "native_sub_byref_array");
    assert_eq!(stdout, "15\n");
}

#[test]
fn native_function_call_copies_back_fixed_string_array_element_argument() {
    let source = r#"
DECLARE FUNCTION PAD$(A$ AS STRING * 4)
DIM B$(2) AS STRING * 4
B$(1) = "Z"
PRINT PAD$(B$(1))
PRINT B$(1)
FUNCTION PAD$(A$ AS STRING * 4) AS STRING * 4
A$ = "ABCDE"
PAD$ = A$
END FUNCTION
"#;

    let stdout = compile_and_run(source, "native_func_byref_string_array");
    assert_eq!(stdout, "ABCD\nABCD\n");
}
