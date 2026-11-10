//! Every conformance case is a valid program, so it must parse with no
//! diagnostics and round-trip exactly.

use std::path::Path;

fn collect(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("bas")) {
            out.push(p);
        }
    }
}

#[test]
fn conformance_cases_parse_cleanly() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/cases");
    let mut files = Vec::new();
    collect(&dir, &mut files);
    assert!(!files.is_empty());
    let mut problems = Vec::new();
    for f in &files {
        let bytes = std::fs::read(f).unwrap();
        let (p, enc) = qb_syntax::parse_bytes(&bytes);
        assert_eq!(qb_syntax::encoding::encode(&p.to_text(), enc), bytes, "{}", f.display());
        for d in &p.diagnostics {
            problems.push(format!("{}: {} at byte {}", f.display(), d.message, d.range.start));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
