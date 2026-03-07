use native_codegen::CodeGenerator;
use syntax_tree::Parser;

fn generate(source: &str) -> String {
    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();
    let mut codegen = CodeGenerator::new();
    codegen.generate(&program).unwrap()
}

#[test]
fn bload_bsave_round_trip_runs() {
    let source = r#"
DEF SEG = 8192
POKE 16, 123
BSAVE "mem.bin", 16, 1
POKE 16, 0
BLOAD "mem.bin", 16
PRINT PEEK(16)
"#;

    let code = generate(source);
    let temp_dir = std::env::temp_dir().join(format!("qbnex_bload_bsave_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let rust_path = temp_dir.join("bload_bsave_test.rs");
    let exe_path = temp_dir.join("bload_bsave_test.exe");
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
    assert_eq!(stdout, "123\n");

    let _ = std::fs::remove_dir_all(&temp_dir);
}
