use native_codegen::CodeGenerator;
use syntax_tree::Parser;

fn generate(source: &str) -> String {
    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();
    let mut codegen = CodeGenerator::new();
    codegen.generate(&program).unwrap()
}

#[test]
fn native_compat_helpers_codegen_compiles() {
    let code = generate(
        "A = 42\nA$ = \"HI\"\nP = VARPTR(A)\nS = VARSEG(A)\nDEF SEG = S\nPOKE 16, 123\nWAIT 16, 123\nBSAVE \"mem.bin\", 16, 1\nPOKE 16, 0\nBLOAD \"mem.bin\", 16\nOUT 100, 77\nZ = INP(100)\nX = PEEK(P)\nY = SADD(A$)\nQ$ = VARPTR$(A)\nK$ = INKEY$\nLPRINT \"X\"\nLOCATE 5, 10\nR = CSRLIN\nC = POS(0)\nD = LPOS(0)\nPRINT R\nPRINT C\nPRINT D",
    );
    assert_generated_rust_compiles(&code, "native_compat_helpers_codegen_compiles");
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
