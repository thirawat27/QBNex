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
    let temp_dir = std::env::temp_dir().join(format!("{}_{}", test_name, std::process::id()));
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
fn static_sub_persists_scalar_udt_across_early_exit() {
    let source = r#"
TYPE PERSON
NAME AS STRING * 4
AGE AS INTEGER
END TYPE
DECLARE SUB Demo()
CALL Demo
CALL Demo
SUB Demo STATIC
DIM rec AS PERSON
IF rec.age <> 0 THEN GOTO skipinit
rec.name = "AB"
skipinit:
rec.age = rec.age + 1
PRINT rec.name
PRINT rec.age
EXIT SUB
END SUB
"#;

    let stdout = compile_and_run(source, "static_scalar_udt");
    assert_eq!(stdout, "AB  \n1\nAB  \n2\n");
}

#[test]
fn static_sub_persists_udt_array_fields() {
    let source = r#"
TYPE PERSON
NAME AS STRING * 4
AGE AS INTEGER
END TYPE
DECLARE SUB Demo()
CALL Demo
CALL Demo
SUB Demo STATIC
DIM recs(2) AS PERSON
recs(1).age = recs(1).age + 1
IF recs(1).name = "" THEN recs(1).name = "AB"
PRINT recs(1).name
PRINT recs(1).age
END SUB
"#;

    let stdout = compile_and_run(source, "static_udt_array");
    assert_eq!(stdout, "AB  \n1\nAB  \n2\n");
}
