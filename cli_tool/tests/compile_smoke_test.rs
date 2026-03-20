use native_codegen::CodeGenerator;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use syntax_tree::Parser;
use vm_engine::BytecodeCompiler;

fn unique_path(stem: &str, ext: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("qbnex_{stem}_{nanos}.{ext}"))
}

fn compile_with_qb(source: &str) {
    let source_path = unique_path("smoke", "bas");
    let output_path = unique_path("smoke", "exe");
    fs::write(&source_path, source).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_qb"))
        .args([
            "-c",
            source_path.to_str().unwrap(),
            "-o",
            output_path.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    let _ = fs::remove_file(&source_path);
    let _ = fs::remove_file(&output_path);
    assert!(
        status.success(),
        "qb failed to compile {}",
        source_path.display()
    );
}

fn compile_pipeline(source: &str, enable_graphics: bool) {
    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new(program.clone());
    compiler.compile().unwrap();

    let mut codegen = CodeGenerator::new();
    if enable_graphics {
        codegen.enable_graphics();
    }
    let generated = codegen.generate(&program).unwrap();
    assert!(!generated.is_empty());
}

fn read_example(path: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(path)).unwrap()
}

#[test]
fn qb_compiles_basic_language_smoke_program() {
    compile_with_qb(
        r#"
CLS
x = 10
y = 20
PRINT x + y
IF x < y THEN PRINT "ok"
FOR i = 1 TO 3
PRINT i
NEXT i
DIM arr(5)
arr(1) = 42
PRINT arr(1)
s$ = "Hello World"
MID$(s$, 7, 5) = "QBas!"
PRINT s$
"#,
    );
}

#[test]
fn qb_compiles_select_case_smoke_program() {
    compile_with_qb(
        r#"
value = 2
SELECT CASE value
CASE 1
    PRINT "one"
CASE 2, 3
    PRINT "two-or-three"
CASE 4 TO 10
    PRINT "range"
CASE IS > 10
    PRINT "big"
CASE ELSE
    PRINT "other"
END SELECT
"#,
    );
}

#[test]
fn qb_compiles_file_io_smoke_program() {
    compile_with_qb(
        r#"
OPEN "temp.txt" FOR OUTPUT AS #1
PRINT #1, "Hello", 123
WRITE #1, "World", 456
CLOSE #1
OPEN "temp.txt" FOR INPUT AS #1
LINE INPUT #1, l$
PRINT INPUT$(3, 1)
CLOSE #1
"#,
    );
}

#[test]
fn qb_compiles_system_and_conversion_smoke_program() {
    compile_with_qb(
        r#"
s$ = MKS$(12.34)
PRINT CVS(s$)
PRINT CVI(MKI$(99))
PRINT VARPTR$(s$)
PRINT VARSEG(s$)
PRINT SADD(s$)
PRINT DATE$
PRINT TIME$
PRINT TIMER
RANDOMIZE 1234
PRINT RND(1)
PRINT CSTR(123)
PRINT COMMAND$
PRINT LEFT$(ENVIRON$("PATH"), 5)
PRINT ENVIRON$(1)
PRINT MID$("ABCDE", 3)
PRINT INSTR(2, "ABCDE", "CD")
PRINT TRIM$("  X  ")
PRINT STR$(123)
PRINT VAL("123ABC")
PRINT STRING$(3, 65)
PRINT "["; SPACE$(2); "]"
PRINT CSRLIN
PRINT POS(0)
"#,
    );
}

#[test]
fn qb_compiles_graphics_builtin_only_smoke_program() {
    compile_with_qb(
        r#"
PRINT POINT(1, 1)
PRINT PMAP(0, 0)
"#,
    );
}

#[test]
fn qb_compiles_graphics_smoke_program() {
    compile_with_qb(
        r#"
SCREEN 13
VIEW (10, 10)-(100, 80), 1, 15
WINDOW (0, 0)-(319, 199)
PSET (20, 20), 14
PRINT POINT(20, 20)
PRINT PMAP(0, 0)
LINE (10,10)-(30,30), 12
CIRCLE (60, 45), 15, 11
PAINT (60, 45), 9, 11
DRAW "BM120,60 C13 R20 D20 L20 U20"
GET (10,10)-(20,20), sprite
PUT (30,30), sprite, XOR
VIEW
WINDOW
"#,
    );
}

#[test]
fn compiler_and_native_codegen_cover_supported_text_and_data_features() {
    compile_pipeline(
        r#"
PRINT "start"
READ a, b
RESTORE
SWAP a, b
IF a <> b THEN
    PRINT "diff"
END IF
SELECT CASE a
CASE 1
    PRINT "one"
CASE 2 TO 4
    PRINT "range"
CASE ELSE
    PRINT "other"
END SELECT
DIM arr(5)
REDIM buf(10)
ERASE buf
DATA 1, 2, 3
"#,
        false,
    );
}

#[test]
fn compiler_and_native_codegen_cover_shipped_graphics_examples() {
    for example in [
        read_example("examples/test_graphics_advanced.bas"),
        read_example("examples/test_graphics_getput.bas"),
        read_example("examples/test_graphics_modules.bas"),
    ] {
        compile_pipeline(&example, true);
    }
}
