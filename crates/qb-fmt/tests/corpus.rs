//! Formats every `.bas` under tests/conformance/cases and tests/corpus-local
//! and checks: the output has the same tokens as the input, formatting the
//! output again changes nothing, and the output parses with no more
//! diagnostics than the input.
//!
//! `QB_CORPUS_DIR` adds another directory (for example the main checkout's
//! tests/corpus-local, whose programs are not committed).

use std::path::{Path, PathBuf};

use proptest::prelude::*;
use qb_fmt::{KeywordCase, Options, equivalent, format};

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("bas") || x.eq_ignore_ascii_case("bi") || x.eq_ignore_ascii_case("bm")) {
            out.push(p);
        }
    }
}

fn check_text(name: &str, text: &str, opts: &Options) -> Result<(), String> {
    let once = format(text, opts);
    equivalent(text, &once).map_err(|e| format!("{name}: tokens changed: {e}"))?;
    let twice = format(&once, opts);
    if twice != once {
        let line = once.lines().zip(twice.lines()).position(|(a, b)| a != b).unwrap_or(0);
        return Err(format!(
            "{name}: not idempotent at line {}: {:?} vs {:?}",
            line + 1,
            once.lines().nth(line),
            twice.lines().nth(line)
        ));
    }
    let before = qb_syntax::parse(text).diagnostics.len();
    let after = qb_syntax::parse(&once).diagnostics.len();
    if after != before {
        return Err(format!("{name}: diagnostics {before} -> {after}"));
    }
    Ok(())
}

#[test]
fn corpus_round_trips() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    collect(&root.join("tests/conformance/cases"), &mut files);
    collect(&root.join("tests/corpus-local"), &mut files);
    if let Ok(extra) = std::env::var("QB_CORPUS_DIR") {
        collect(Path::new(&extra), &mut files);
    }
    assert!(!files.is_empty(), "no .bas files found");
    let variants = [
        Options::default(),
        Options { keyword_case: KeywordCase::Lower, indent: "\t".into(), ..Options::default() },
    ];
    let mut failures = Vec::new();
    for f in &files {
        let bytes = std::fs::read(f).unwrap();
        let (text, _) = qb_syntax::encoding::decode(&bytes);
        for opts in &variants {
            if let Err(e) = check_text(&f.display().to_string(), &text, opts) {
                failures.push(e);
            }
        }
    }
    eprintln!("checked {} files", files.len());
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}

fn fragment() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec![
        "IF ", " THEN", "ELSE", "END IF", "FOR i = 1 TO 3", "NEXT", "DO", "LOOP", "SELECT CASE x", "CASE 1",
        "END SELECT", "SUB s", "END SUB", "TYPE t", "END TYPE", "DATA 1, 2 ", "REM x", "' c", "10 ", "lbl:",
        "\"s\"", "\"open", " _", ":", "\n", "\r\n", "-", "+", "*", "(", ")", ",", ";", "#", "x", "a$", "1",
        "=", "<>", "PRINT", " ", "  ", "\t", ".", "&", "@", "$", "?", "é", "{", "}",
    ])
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 3000, ..ProptestConfig::default() })]

    #[test]
    fn random_programs(parts in proptest::collection::vec(fragment(), 0..40)) {
        let text = parts.concat();
        if let Err(e) = check_text("random", &text, &Options::default()) {
            prop_assert!(false, "{e}\ninput: {text:?}");
        }
    }
}
