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
fn udt_array_get_put_runs() {
    let source = r#"
TYPE ADDRESS
STREET AS STRING * 4
ZIP AS INTEGER
END TYPE
TYPE PERSON
ADDR AS ADDRESS
AGE AS INTEGER
END TYPE
DIM recs(2) AS PERSON
recs(1).addr.street = "AB"
recs(1).addr.zip = 42
recs(1).age = 7
OPEN "udt_array_test.bin" FOR BINARY AS #1
PUT #1, 1, recs(1)
recs(2).addr.street = "ZZZZ"
recs(2).addr.zip = 0
recs(2).age = 0
GET #1, 1, recs(2)
CLOSE #1
PRINT recs(2).addr.street
PRINT recs(2).addr.zip
PRINT recs(2).age
"#;

    let code = generate(source);
    let temp_dir = std::env::temp_dir().join(format!("qbnex_udt_array_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let rust_path = temp_dir.join("udt_array_test.rs");
    let exe_path = temp_dir.join("udt_array_test.exe");
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
    assert_eq!(stdout, "AB  \n42\n7\n");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn udt_array_subrecord_and_lset_rset_runs() {
    let source = r#"
TYPE ADDRESS
STREET AS STRING * 4
ZIP AS INTEGER
END TYPE
TYPE PERSON
ADDR AS ADDRESS
AGE AS INTEGER
END TYPE
DIM recs(2) AS PERSON
LSET recs(1).addr.street = "AB"
recs(1).addr.zip = 42
RSET recs(1).addr.street = "Z"
OPEN "udt_array_subrecord_test.bin" FOR BINARY AS #1
PUT #1, 1, recs(1).addr
recs(2).addr.street = ""
recs(2).addr.zip = 0
GET #1, 1, recs(2).addr
CLOSE #1
PRINT recs(2).addr.street
PRINT recs(2).addr.zip
"#;

    let code = generate(source);
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_udt_array_subrecord_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let rust_path = temp_dir.join("udt_array_subrecord_test.rs");
    let exe_path = temp_dir.join("udt_array_subrecord_test.exe");
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
    assert_eq!(stdout, "   Z\n42\n");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn udt_array_record_get_put_evaluates_index_once() {
    let source = r#"
TYPE ADDRESS
STREET AS STRING * 4
ZIP AS INTEGER
END TYPE
TYPE PERSON
ADDR AS ADDRESS
AGE AS INTEGER
END TYPE
DECLARE FUNCTION IDX%()
DIM recs(3) AS PERSON
recs(1).addr.street = "AB"
recs(1).addr.zip = 42
recs(1).age = 7
recs(2).addr.street = "CD"
recs(2).addr.zip = 99
recs(2).age = 8
OPEN "udt_array_index_once.bin" FOR BINARY AS #1
PUT #1, 1, recs(IDX%)
GET #1, 1, recs(IDX%)
CLOSE #1
PRINT recs(2).addr.street
PRINT recs(2).addr.zip
PRINT recs(2).age
PRINT recs(3).addr.street
PRINT recs(3).addr.zip
PRINT recs(3).age
FUNCTION IDX% STATIC
c% = c% + 1
IDX% = c%
END FUNCTION
"#;

    let stdout = compile_and_run(source, "udt_array_index_once");
    assert_eq!(stdout, "AB  \n42\n7\n\n0\n0\n");
}

#[test]
fn udt_array_leaf_get_put_evaluates_index_once() {
    let source = r#"
TYPE ADDRESS
STREET AS STRING * 4
ZIP AS INTEGER
END TYPE
TYPE PERSON
ADDR AS ADDRESS
AGE AS INTEGER
END TYPE
DECLARE FUNCTION IDX%()
DIM recs(3) AS PERSON
recs(1).addr.street = "AB"
recs(1).addr.zip = 42
OPEN "udt_array_leaf_index_once.bin" FOR BINARY AS #1
PUT #1, 1, recs(IDX%).addr.street
GET #1, 1, recs(IDX%).addr.street
CLOSE #1
PRINT recs(2).addr.street
PRINT recs(3).addr.street
FUNCTION IDX% STATIC
c% = c% + 1
IDX% = c%
END FUNCTION
"#;

    let stdout = compile_and_run(source, "udt_array_leaf_index_once");
    assert_eq!(stdout, "AB  \n\n");
}

#[test]
fn native_sub_call_copies_back_udt_array_numeric_leaf_argument() {
    let source = r#"
TYPE PERSON
AGE AS INTEGER
END TYPE
DECLARE SUB INC(X!)
DIM recs(2) AS PERSON
recs(1).age = 10
CALL INC(recs(1).age)
PRINT recs(1).age
SUB INC(X!)
X! = X! + 5
END SUB
"#;

    let stdout = compile_and_run(source, "udt_array_byref_numeric_leaf");
    assert_eq!(stdout, "15\n");
}

#[test]
fn native_function_call_copies_back_udt_array_string_leaf_argument() {
    let source = r#"
TYPE PERSON
NAME AS STRING * 4
END TYPE
DECLARE FUNCTION PAD$(A$ AS STRING * 4)
DIM recs(2) AS PERSON
recs(1).name = "Z"
PRINT PAD$(recs(1).name)
PRINT recs(1).name
FUNCTION PAD$(A$ AS STRING * 4) AS STRING * 4
A$ = "ABCDE"
PAD$ = A$
END FUNCTION
"#;

    let stdout = compile_and_run(source, "udt_array_byref_string_leaf");
    assert_eq!(stdout, "ABCD\nABCD\n");
}
