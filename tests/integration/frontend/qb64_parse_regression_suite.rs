use std::fs;
use std::path::{Path, PathBuf};

use syntax_tree::Parser;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("frontend directory should live under the workspace root")
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

fn collect_basic_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", dir.display());
    });
    for entry in entries {
        let entry = entry.unwrap_or_else(|err| {
            panic!("failed to read entry under {}: {err}", dir.display());
        });
        let path = entry.path();
        if path.is_dir() {
            collect_basic_sources(&path, out);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("bas"))
        {
            out.push(path);
        }
    }
}

#[test]
fn all_qb64_source_files_parse() {
    let source_root = qb64_corpus_root();
    let mut files = Vec::new();
    collect_basic_sources(&source_root, &mut files);
    files.sort();

    assert!(
        !files.is_empty(),
        "expected at least one QB64 corpus file under {}",
        source_root.display()
    );

    let mut failures = Vec::new();
    for file in files {
        let input = fs::read_to_string(&file).unwrap_or_else(|err| {
            panic!("failed to read {}: {err}", file.display());
        });
        let mut parser = Parser::new(input).unwrap_or_else(|err| {
            panic!("failed to tokenize {}: {err}", file.display());
        });
        if let Err(err) = parser.parse() {
            failures.push(format!("{}: {}", file.display(), err));
        }
    }

    assert!(
        failures.is_empty(),
        "QB64 source parse regressions:\n{}",
        failures.join("\n")
    );
}
