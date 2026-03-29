use native_codegen::CodeGenerator;
use std::collections::HashSet;
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
        .current_dir(repo_root())
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

fn read_fixture(path: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(path)).unwrap()
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("cli directory should live under the workspace root")
        .to_path_buf()
}

fn qb64_corpus_root() -> PathBuf {
    let external_source_root = repo_root().join("qb64").join("source");
    if external_source_root.exists() {
        external_source_root
    } else {
        repo_root().join("tests").join("corpora").join("qb64")
    }
}

fn has_external_qb64_source_tree() -> bool {
    repo_root().join("qb64").join("source").exists()
}

fn parse_include_path(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let directive = trimmed.strip_prefix('\'').unwrap_or(trimmed).trim_start();

    if !directive
        .get(..8)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("$INCLUDE"))
    {
        return None;
    }

    let rest = directive
        .get(8..)?
        .trim_start()
        .strip_prefix(':')?
        .trim_start();
    let quote = rest.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }

    let after_quote = rest.get(quote.len_utf8()..)?;
    let end = after_quote.find(quote)?;
    let include_path = after_quote[..end].trim();
    if include_path.is_empty() {
        None
    } else {
        Some(include_path.to_string())
    }
}

fn normalized_include_path(include_path: &str) -> PathBuf {
    include_path
        .split(['\\', '/'])
        .filter(|segment| !segment.is_empty())
        .fold(PathBuf::new(), |mut path, segment| {
            path.push(segment);
            path
        })
}

fn collect_project_source_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", dir.display());
    });
    for entry in entries {
        let entry = entry.unwrap_or_else(|err| {
            panic!("failed to read entry under {}: {err}", dir.display());
        });
        let path = entry.path();
        if path.is_dir() {
            collect_project_source_files(&path, out);
        } else if path.extension().is_some_and(|ext| {
            ext.eq_ignore_ascii_case("bas")
                || ext.eq_ignore_ascii_case("bi")
                || ext.eq_ignore_ascii_case("bm")
        }) {
            out.push(path);
        }
    }
}

fn discover_qb64_source_roots() -> Vec<PathBuf> {
    let source_root = qb64_corpus_root();
    let canonical_source_root =
        fs::canonicalize(&source_root).unwrap_or_else(|_| source_root.clone());
    let mut files = Vec::new();
    collect_project_source_files(&source_root, &mut files);
    files.sort();

    let mut included = HashSet::new();
    for file in &files {
        let parent = file
            .parent()
            .unwrap_or_else(|| panic!("{} should have a parent directory", file.display()));
        let contents = fs::read_to_string(file).unwrap_or_else(|err| {
            panic!("failed to read {}: {err}", file.display());
        });
        for include in contents.lines().filter_map(parse_include_path) {
            let resolved = parent.join(normalized_include_path(&include));
            let Ok(canonical) = fs::canonicalize(&resolved) else {
                continue;
            };
            if canonical.starts_with(&canonical_source_root) {
                included.insert(canonical);
            }
        }
    }

    let mut roots = files
        .into_iter()
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("bas"))
                && fs::canonicalize(path)
                    .map(|canonical| !included.contains(&canonical))
                    .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    roots.sort();
    roots
}

fn discover_qb64_source_bas_files() -> Vec<PathBuf> {
    let source_root = qb64_corpus_root();
    let mut files = Vec::new();
    collect_project_source_files(&source_root, &mut files);
    files.retain(|path| {
        path.extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("bas"))
    });
    files.sort();
    files
}

fn compile_source_path_with_qb(source_path: &Path) {
    let output_path = unique_path(
        source_path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("qb64_source"),
        "exe",
    );

    let status = Command::new(env!("CARGO_BIN_EXE_qb"))
        .args([
            "-c",
            source_path.to_str().unwrap(),
            "-o",
            output_path.to_str().unwrap(),
        ])
        .current_dir(repo_root())
        .status()
        .unwrap();

    let _ = fs::remove_file(&output_path);
    assert!(
        status.success(),
        "qb failed to compile {}",
        source_path.display()
    );
}

fn compile_source_file_with_qb(relative_source_path: &str) {
    compile_source_path_with_qb(&repo_root().join(relative_source_path));
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
PRINT _CV(INTEGER, MKI$(99))
PRINT _CV(_UNSIGNED INTEGER, CHR$(255) + CHR$(255))
PRINT _CV(_INTEGER64, CHR$(0) + CHR$(0) + CHR$(0) + CHR$(0) + CHR$(1) + CHR$(0) + CHR$(0) + CHR$(0))
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
PRINT SCREEN(1, 1)
PRINT SCREEN(1, 1, 1)
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
#[ignore = "full QB64 source compile regression"]
fn qb_compiles_supported_qb64_source_files() {
    if !has_external_qb64_source_tree() {
        eprintln!("skipping QB64 compile sweep because qb64/source is not present");
        return;
    }

    let files = discover_qb64_source_bas_files();
    assert!(
        !files.is_empty(),
        "expected at least one QB64 corpus file under {}",
        qb64_corpus_root().display()
    );

    let labels = files
        .iter()
        .map(|path| {
            path.strip_prefix(repo_root())
                .unwrap_or(path)
                .display()
                .to_string()
        })
        .collect::<Vec<_>>();
    assert!(
        labels.iter().any(|path| {
            path.ends_with("qb64\\source\\qb64.bas")
                || path.ends_with("qb64/source/qb64.bas")
                || path.ends_with("tests\\corpora\\qb64\\qb64.bas")
                || path.ends_with("tests/corpora/qb64/qb64.bas")
        }),
        "expected the shipped or external QB64 root source in the full source sweep, found: {labels:?}"
    );

    if has_external_qb64_source_tree() {
        assert!(
            labels
                .iter()
                .any(|path| path == "qb64\\source\\ide\\ide_global.bas"
                    || path == "qb64/source/ide/ide_global.bas"),
            "expected qb64/source/ide/ide_global.bas in the full source sweep, found: {labels:?}"
        );
        assert!(
            labels
                .iter()
                .any(|path| path == "qb64\\source\\ide\\wiki\\wiki_global.bas"
                    || path == "qb64/source/ide/wiki/wiki_global.bas"),
            "expected qb64/source/ide/wiki/wiki_global.bas in the full source sweep, found: {labels:?}"
        );
    }

    for path in files {
        compile_source_path_with_qb(&path);
    }
}

#[test]
#[ignore = "full QB64 root compile regression derived from the include graph"]
fn qb_compiles_qb64_source_roots_discovered_from_include_graph() {
    if !has_external_qb64_source_tree() {
        eprintln!("skipping QB64 root compile sweep because qb64/source is not present");
        return;
    }

    let roots = discover_qb64_source_roots();
    assert!(
        !roots.is_empty(),
        "expected at least one QB64 corpus root under {}",
        qb64_corpus_root().display()
    );

    let root_labels = roots
        .iter()
        .map(|path| {
            path.strip_prefix(repo_root())
                .unwrap_or(path)
                .display()
                .to_string()
        })
        .collect::<Vec<_>>();
    assert!(
        root_labels.iter().any(|path| {
            path.ends_with("qb64\\source\\qb64.bas")
                || path.ends_with("qb64/source/qb64.bas")
                || path.ends_with("tests\\corpora\\qb64\\qb64.bas")
                || path.ends_with("tests/corpora/qb64/qb64.bas")
        }),
        "expected the shipped or external QB64 root source to be discovered as a root, found: {root_labels:?}"
    );

    for root in roots {
        compile_source_path_with_qb(&root);
    }
}

#[test]
#[ignore = "full QB64 fragment promotion regression"]
fn qb_promotes_supported_qb64_source_fragments_through_their_unique_root() {
    if !has_external_qb64_source_tree() {
        eprintln!("skipping fragment promotion sweep because qb64/source is not present");
        return;
    }

    for path in [
        "qb64/source/global/IDEsettings.bas",
        "qb64/source/ide/ide_global.bas",
        "qb64/source/ide/wiki/wiki_global.bas",
        "qb64/source/subs_functions/subs_functions.bas",
        "qb64/source/utilities/config.bas",
    ] {
        compile_source_file_with_qb(path);
    }
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
        read_fixture("tests/fixtures/basic/test_graphics_advanced.bas"),
        read_fixture("tests/fixtures/basic/test_graphics_getput.bas"),
        read_fixture("tests/fixtures/basic/test_graphics_modules.bas"),
    ] {
        compile_pipeline(&example, true);
    }
}
