//! Regular expressions (Rust `regex` syntax, byte-oriented: patterns and
//! texts are BASIC byte strings; `(?-u)` turns off Unicode classes).
//!
//! Positions are 1-based byte offsets like `INSTR`; 0 means "no match".

use regex::bytes::Regex;
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    /// Compiled patterns, so loops that use the same pattern compile it once.
    static CACHE: RefCell<HashMap<Vec<u8>, Regex>> = RefCell::new(HashMap::new());
}

/// Compiles `pattern` (cached). `Err` holds the regex error message.
pub fn compile(pattern: &[u8]) -> Result<Regex, String> {
    CACHE.with(|c| {
        if let Some(r) = c.borrow().get(pattern) {
            return Ok(r.clone());
        }
        let text = std::str::from_utf8(pattern).map_err(|_| "pattern is not valid UTF-8".to_string())?;
        let r = Regex::new(text).map_err(|e| e.to_string())?;
        let mut c = c.borrow_mut();
        if c.len() > 256 {
            c.clear();
        }
        c.insert(pattern.to_vec(), r.clone());
        Ok(r)
    })
}

/// True when `pattern` matches anywhere in `text`.
pub fn is_match(pattern: &[u8], text: &[u8]) -> Result<bool, String> {
    Ok(compile(pattern)?.is_match(text))
}

/// A match: 1-based start, the matched bytes and its capture groups
/// (`None` for a group that did not take part).
#[derive(Debug, PartialEq, Eq)]
pub struct Found {
    pub start: usize,
    pub groups: Vec<Option<Vec<u8>>>,
}

/// First match at or after the 1-based byte position `start`.
pub fn find(pattern: &[u8], text: &[u8], start: usize) -> Result<Option<Found>, String> {
    let r = compile(pattern)?;
    let from = start.max(1) - 1;
    if from > text.len() {
        return Ok(None);
    }
    Ok(r.captures_at(text, from).map(|c| Found {
        start: c.get(0).map_or(0, |m| m.start() + 1),
        groups: c.iter().map(|g| g.map(|m| m.as_bytes().to_vec())).collect(),
    }))
}

/// Replaces the first (or every) match. In `replacement`, `$1`, `${name}`
/// insert groups and `$$` is a dollar sign.
pub fn replace(pattern: &[u8], text: &[u8], replacement: &[u8], all: bool) -> Result<Vec<u8>, String> {
    let r = compile(pattern)?;
    Ok(if all { r.replace_all(text, replacement).into_owned() } else { r.replace(text, replacement).into_owned() })
}

/// Splits `text` at every match.
pub fn split(pattern: &[u8], text: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    let r = compile(pattern)?;
    Ok(r.split(text).map(<[u8]>::to_vec).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_find_groups() {
        assert_eq!(is_match(br"^\d{3}-\d{4}$", b"555-1234"), Ok(true));
        assert_eq!(is_match(br"^\d+$", b"12a"), Ok(false));
        let f = find(br"(\w+)@(\w+)\.com", b"mail bob@example.com now", 1).unwrap().unwrap();
        assert_eq!(f.start, 6);
        assert_eq!(f.groups[0].as_deref(), Some(&b"bob@example.com"[..]));
        assert_eq!(f.groups[2].as_deref(), Some(&b"example"[..]));
        assert_eq!(find(br"a", b"banana", 3).unwrap().unwrap().start, 4);
        assert_eq!(find(br"a", b"banana", 7).unwrap(), None);
        assert_eq!(find(br"a", b"banana", 99).unwrap(), None);
        let opt = find(br"(x)?y", b"y", 1).unwrap().unwrap();
        assert_eq!(opt.groups[1], None);
        assert!(is_match(b"(", b"").is_err());
    }

    #[test]
    fn replace_and_split() {
        assert_eq!(replace(br"(\d+)", b"a1b22", b"<$1>", true).unwrap(), b"a<1>b<22>");
        assert_eq!(replace(br"\d", b"a1b2", b"#", false).unwrap(), b"a#b2");
        assert_eq!(replace(br"x", b"x", b"$$", true).unwrap(), b"$");
        assert_eq!(split(br"\s*,\s*", b"a , b,c").unwrap(), vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
        assert_eq!(split(br",", b"").unwrap(), vec![Vec::<u8>::new()]);
        // Non-UTF-8 bytes in the text are fine.
        assert_eq!(is_match(br"(?-u)\xff", &[0x41, 0xff]), Ok(true));
    }
}
