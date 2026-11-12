//! Property tests: any input lexes and parses without panicking, and the tree
//! is lossless.

use proptest::prelude::*;
use qb_syntax::{encoding, parse, parse_bytes};

fn check(text: &str) {
    let p = parse(text);
    // Tokens are contiguous and cover the input.
    let mut pos = 0u32;
    for t in &p.tokens {
        assert_eq!(t.start, pos);
        assert!(t.end > t.start);
        pos = t.end;
    }
    assert_eq!(pos as usize, text.len());
    // The tree visits every token once, in order.
    let idx = p.root.token_indices();
    assert_eq!(idx, (0..p.tokens.len() as u32).collect::<Vec<_>>());
    assert_eq!(p.to_text(), text);
    for d in &p.diagnostics {
        assert!(d.range.start <= d.range.end && d.range.end <= text.len());
    }
}

/// BASIC-flavoured fragments, so random inputs hit the interesting paths.
fn fragment() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("IF ".to_string()),
        Just(" THEN".to_string()),
        Just("ELSE".to_string()),
        Just("ELSEIF".to_string()),
        Just("END IF".to_string()),
        Just("END".to_string()),
        Just("FOR i = 1 TO 3".to_string()),
        Just("NEXT".to_string()),
        Just("NEXT i, j".to_string()),
        Just("DO".to_string()),
        Just("LOOP".to_string()),
        Just("WHILE".to_string()),
        Just("WEND".to_string()),
        Just("SELECT CASE x".to_string()),
        Just("CASE ".to_string()),
        Just("END SELECT".to_string()),
        Just("SUB s".to_string()),
        Just("END SUB".to_string()),
        Just("FUNCTION f%".to_string()),
        Just("TYPE t".to_string()),
        Just("END TYPE".to_string()),
        Just("DEF FNa".to_string()),
        Just("DATA 1, \"a:b\" ".to_string()),
        Just("REM x".to_string()),
        Just("'$INCLUDE: 'a.bi'".to_string()),
        Just("$CONSOLE:ONLY".to_string()),
        Just("10 ".to_string()),
        Just("lbl:".to_string()),
        Just("\"str".to_string()),
        Just("\"".to_string()),
        Just(" _".to_string()),
        Just(":".to_string()),
        Just("\n".to_string()),
        Just("\r\n".to_string()),
        Just("\r".to_string()),
        Just("&H".to_string()),
        Just("1.5E+".to_string()),
        Just("#".to_string()),
        Just("$".to_string()),
        Just("é".to_string()),
        "[ -~]{0,6}".prop_map(|s| s),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 2000, ..ProptestConfig::default() })]

    #[test]
    fn arbitrary_strings(s in "\\PC{0,80}") {
        check(&s);
    }

    #[test]
    fn arbitrary_bytes(b in proptest::collection::vec(any::<u8>(), 0..120)) {
        let (p, enc) = parse_bytes(&b);
        check(&p.text);
        prop_assert_eq!(encoding::encode(&p.text, enc), b);
    }

    #[test]
    fn basic_like(parts in proptest::collection::vec(fragment(), 0..40)) {
        check(&parts.concat());
    }

    #[test]
    fn include_path_never_panics(prefix in prop_oneof![Just(""), Just("'"), Just("'$"), Just("REM "), Just("$"), Just("'$INCLUDE:'")], s in "\\PC{0,20}") {
        let _ = qb_syntax::include_path(&format!("{prefix}{s}"));
    }
}
