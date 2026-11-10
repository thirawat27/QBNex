//! Display names: QIR names (`$__INTEGER_A`, `$_SUB_BUMP_INTEGER_X`,
//! `FUNC_FACT`) back to BASIC names (`a%`, `x%`, `Fact`), spelled as in the
//! source when the source is available.

use std::collections::HashMap;

/// Type tokens of the compiler's name mangling and their suffixes.
const TYPES: &[(&str, &str)] = &[
    ("UINTEGER64", "~&&"),
    ("INTEGER64", "&&"),
    ("UINTEGER", "~%"),
    ("INTEGER", "%"),
    ("ULONG", "~&"),
    ("LONG", "&"),
    ("SINGLE", "!"),
    ("DOUBLE", "#"),
    ("FLOAT", "##"),
    ("UBYTE", "~%%"),
    ("BYTE", "%%"),
    ("UOFFSET", "~%&"),
    ("OFFSET", "%&"),
    ("UDT", ""),
    ("STRING", "$"),
    ("UBIT", "~`"),
    ("BIT", "`"),
];

/// Upper-case base name -> spelling in the source.
#[derive(Default, Debug, Clone)]
pub struct Casing(HashMap<String, String>);

impl Casing {
    /// Learns the first spelling of every identifier of `source`.
    pub fn from_source(source: &str) -> Casing {
        let mut map = HashMap::new();
        for t in qb_syntax::lex(source) {
            if matches!(t.kind, qb_syntax::TokenKind::Ident | qb_syntax::TokenKind::Label) {
                for part in t.text(source).split('.') {
                    let base = part.trim_end_matches(['%', '&', '!', '#', '$', '~', '`']);
                    map.entry(base.to_ascii_uppercase()).or_insert_with(|| base.to_string());
                }
            }
        }
        Casing(map)
    }

    pub fn spell(&self, upper: &str) -> String {
        self.0.get(&upper.to_ascii_uppercase()).cloned().unwrap_or_else(|| upper.to_string())
    }
}

/// `SUB_BUMP` -> `Bump`, `FUNC_FACT` -> `Fact`, `main` -> `<module>`.
pub fn func_name(qir: &str, casing: &Casing) -> String {
    if qir == "main" {
        return "<module>".into();
    }
    let base = qir.strip_prefix("SUB_").or_else(|| qir.strip_prefix("FUNC_")).unwrap_or(qir);
    casing.spell(base)
}

/// BASIC name of a variable declared in function `func` (QIR names).
pub fn var_name(qir: &str, func: &str, casing: &Casing) -> String {
    let mut s = qir.strip_prefix('$').unwrap_or(qir);
    let own = format!("_{func}_");
    if func != "main"
        && let Some(r) = s.strip_prefix(&own)
    {
        s = r;
    }
    let s = s.trim_start_matches('_');
    let s = s.strip_prefix("ARRAY_").unwrap_or(s);
    for (tok, suffix) in TYPES {
        if let Some(rest) = s.strip_prefix(tok) {
            // STRINGn / BITn carry a length: `STRING6_NAME`.
            let rest = rest.trim_start_matches(|c: char| c.is_ascii_digit());
            if let Some(name) = rest.strip_prefix('_')
                && !name.is_empty()
            {
                let fixed = *tok == "STRING" && s.as_bytes().get(6).is_some_and(u8::is_ascii_digit);
                return format!("{}{}", casing.spell(name), if fixed { "" } else { suffix });
            }
        }
    }
    casing.spell(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demangles() {
        let c = Casing::from_source("DIM SHARED total AS LONG\na% = 5\nSUB Bump (x AS INTEGER)\nFUNCTION Fact& (n AS LONG)\nc = 1\n");
        assert_eq!(var_name("$__INTEGER_A", "main", &c), "a%");
        assert_eq!(var_name("$__LONG_TOTAL", "main", &c), "total&");
        assert_eq!(var_name("$_SUB_BUMP_INTEGER_X", "SUB_BUMP", &c), "x%");
        assert_eq!(var_name("$_FUNC_FACT_LONG_N", "FUNC_FACT", &c), "n&");
        assert_eq!(var_name("$_SUB_COUNTER_SINGLE_C", "SUB_COUNTER", &c), "c!");
        assert_eq!(var_name("$__ARRAY_UDT_PTS", "main", &c), "PTS");
        assert_eq!(var_name("$__STRING6_NAME", "main", &c), "NAME");
        assert_eq!(var_name("$__UINTEGER64_U", "main", &c), "U~&&");
        assert_eq!(func_name("SUB_BUMP", &c), "Bump");
        assert_eq!(func_name("FUNC_FACT", &c), "Fact");
        assert_eq!(func_name("main", &c), "<module>");
    }
}
