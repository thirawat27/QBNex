//! UTF-8 helpers for BASIC byte strings: counting and slicing by code point,
//! case mapping, Unicode normalization and CP437 <-> UTF-8 conversion.
//!
//! Invalid UTF-8 is never rejected: every byte of an invalid sequence counts
//! as one "character" and slicing keeps those bytes unchanged, so text that
//! is not valid UTF-8 survives a round trip.

use unicode_normalization::UnicodeNormalization;

/// Byte ranges of the characters of `s` (see the module docs).
pub fn char_ranges(s: &[u8]) -> Vec<(usize, usize)> {
    let mut out = Vec::with_capacity(s.len());
    let mut pos = 0;
    for chunk in s.utf8_chunks() {
        for (i, c) in chunk.valid().char_indices() {
            out.push((pos + i, pos + i + c.len_utf8()));
        }
        pos += chunk.valid().len();
        for _ in chunk.invalid() {
            out.push((pos, pos + 1));
            pos += 1;
        }
    }
    out
}

/// Number of characters.
pub fn len(s: &[u8]) -> usize {
    char_ranges(s).len()
}

/// `MID$` by character: `start` is 1-based, `count` < 0 means "to the end".
pub fn mid(s: &[u8], start: i64, count: i64) -> Vec<u8> {
    let r = char_ranges(s);
    let first = (start.max(1) - 1) as usize;
    if first >= r.len() || count == 0 {
        return Vec::new();
    }
    let last = if count < 0 { r.len() } else { (first + count as usize).min(r.len()) };
    s[r[first].0..r[last - 1].1].to_vec()
}

/// `LEFT$` by character.
pub fn left(s: &[u8], count: i64) -> Vec<u8> {
    if count <= 0 { Vec::new() } else { mid(s, 1, count) }
}

/// `RIGHT$` by character.
pub fn right(s: &[u8], count: i64) -> Vec<u8> {
    let n = len(s) as i64;
    if count <= 0 {
        Vec::new()
    } else if count >= n {
        s.to_vec()
    } else {
        mid(s, n - count + 1, -1)
    }
}

/// Applies `f` to every valid UTF-8 run, keeping invalid bytes.
fn map_valid(s: &[u8], f: impl Fn(&str) -> String) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    for chunk in s.utf8_chunks() {
        out.extend_from_slice(f(chunk.valid()).as_bytes());
        out.extend_from_slice(chunk.invalid());
    }
    out
}

/// Unicode upper case (e.g. "straße" -> "STRASSE").
pub fn upper(s: &[u8]) -> Vec<u8> {
    map_valid(s, str::to_uppercase)
}

/// Unicode lower case.
pub fn lower(s: &[u8]) -> Vec<u8> {
    map_valid(s, str::to_lowercase)
}

/// Unicode normalization forms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    Nfc = 1,
    Nfd = 2,
    Nfkc = 3,
    Nfkd = 4,
}

impl Form {
    pub fn from_code(c: i32) -> Option<Self> {
        Some(match c {
            1 => Self::Nfc,
            2 => Self::Nfd,
            3 => Self::Nfkc,
            4 => Self::Nfkd,
            _ => return None,
        })
    }
}

/// Normalizes every valid UTF-8 run.
pub fn normalize(s: &[u8], form: Form) -> Vec<u8> {
    map_valid(s, |t| match form {
        Form::Nfc => t.nfc().collect(),
        Form::Nfd => t.nfd().collect(),
        Form::Nfkc => t.nfkc().collect(),
        Form::Nfkd => t.nfkd().collect(),
    })
}

/// True when `s` is valid UTF-8.
pub fn is_valid(s: &[u8]) -> bool {
    std::str::from_utf8(s).is_ok()
}

/// Unicode code point of the `index`-th (1-based) character; -1 if there is
/// none, the byte value for a byte of an invalid sequence.
pub fn codepoint(s: &[u8], index: i64) -> i64 {
    let r = char_ranges(s);
    let Some(&(a, b)) = usize::try_from(index - 1).ok().and_then(|i| r.get(i)) else { return -1 };
    match std::str::from_utf8(&s[a..b]) {
        Ok(t) => t.chars().next().map_or(-1, |c| i64::from(u32::from(c))),
        Err(_) => i64::from(s[a]),
    }
}

/// UTF-8 encoding of a code point ("" for surrogates and values > 0x10FFFF).
pub fn from_codepoint(cp: i64) -> Vec<u8> {
    u32::try_from(cp).ok().and_then(char::from_u32).map(|c| c.to_string().into_bytes()).unwrap_or_default()
}

/// Code page 437 as mapped by QBasic/QB64 `_MAPUNICODE` (0x01-0x1F and 0x7F
/// are the PC graphic symbols).
pub const CP437: [u16; 256] = [
    0x0020, 0x263A, 0x263B, 0x2665, 0x2666, 0x2663, 0x2660, 0x2022, 0x25D8, 0x25CB, 0x25D9, 0x2642, 0x2640, 0x266A,
    0x266B, 0x263C, 0x25BA, 0x25C4, 0x2195, 0x203C, 0x00B6, 0x00A7, 0x25AC, 0x21A8, 0x2191, 0x2193, 0x2192, 0x2190,
    0x221F, 0x2194, 0x25B2, 0x25BC, 0x0020, 0x0021, 0x0022, 0x0023, 0x0024, 0x0025, 0x0026, 0x0027, 0x0028, 0x0029,
    0x002A, 0x002B, 0x002C, 0x002D, 0x002E, 0x002F, 0x0030, 0x0031, 0x0032, 0x0033, 0x0034, 0x0035, 0x0036, 0x0037,
    0x0038, 0x0039, 0x003A, 0x003B, 0x003C, 0x003D, 0x003E, 0x003F, 0x0040, 0x0041, 0x0042, 0x0043, 0x0044, 0x0045,
    0x0046, 0x0047, 0x0048, 0x0049, 0x004A, 0x004B, 0x004C, 0x004D, 0x004E, 0x004F, 0x0050, 0x0051, 0x0052, 0x0053,
    0x0054, 0x0055, 0x0056, 0x0057, 0x0058, 0x0059, 0x005A, 0x005B, 0x005C, 0x005D, 0x005E, 0x005F, 0x0060, 0x0061,
    0x0062, 0x0063, 0x0064, 0x0065, 0x0066, 0x0067, 0x0068, 0x0069, 0x006A, 0x006B, 0x006C, 0x006D, 0x006E, 0x006F,
    0x0070, 0x0071, 0x0072, 0x0073, 0x0074, 0x0075, 0x0076, 0x0077, 0x0078, 0x0079, 0x007A, 0x007B, 0x007C, 0x007D,
    0x007E, 0x2302, 0x00C7, 0x00FC, 0x00E9, 0x00E2, 0x00E4, 0x00E0, 0x00E5, 0x00E7, 0x00EA, 0x00EB, 0x00E8, 0x00EF,
    0x00EE, 0x00EC, 0x00C4, 0x00C5, 0x00C9, 0x00E6, 0x00C6, 0x00F4, 0x00F6, 0x00F2, 0x00FB, 0x00F9, 0x00FF, 0x00D6,
    0x00DC, 0x00A2, 0x00A3, 0x00A5, 0x20A7, 0x0192, 0x00E1, 0x00ED, 0x00F3, 0x00FA, 0x00F1, 0x00D1, 0x00AA, 0x00BA,
    0x00BF, 0x2310, 0x00AC, 0x00BD, 0x00BC, 0x00A1, 0x00AB, 0x00BB, 0x2591, 0x2592, 0x2593, 0x2502, 0x2524, 0x2561,
    0x2562, 0x2556, 0x2555, 0x2563, 0x2551, 0x2557, 0x255D, 0x255C, 0x255B, 0x2510, 0x2514, 0x2534, 0x252C, 0x251C,
    0x2500, 0x253C, 0x255E, 0x255F, 0x255A, 0x2554, 0x2569, 0x2566, 0x2560, 0x2550, 0x256C, 0x2567, 0x2568, 0x2564,
    0x2565, 0x2559, 0x2558, 0x2552, 0x2553, 0x256B, 0x256A, 0x2518, 0x250C, 0x2588, 0x2584, 0x258C, 0x2590, 0x2580,
    0x03B1, 0x00DF, 0x0393, 0x03C0, 0x03A3, 0x03C3, 0x00B5, 0x03C4, 0x03A6, 0x0398, 0x03A9, 0x03B4, 0x221E, 0x03C6,
    0x03B5, 0x2229, 0x2261, 0x00B1, 0x2265, 0x2264, 0x2320, 0x2321, 0x00F7, 0x2248, 0x00B0, 0x2219, 0x00B7, 0x221A,
    0x207F, 0x00B2, 0x25A0, 0x00A0,
];

/// CP437 bytes to UTF-8. With `graphics`, bytes 0x01-0x1F and 0x7F become
/// the PC symbols (☺ ♥ ...); otherwise bytes 0x00-0x7F stay ASCII (text with
/// tabs and line breaks). Byte 0xFF is U+00A0 (no-break space).
pub fn cp437_to_utf8(s: &[u8], graphics: bool) -> Vec<u8> {
    let mut out = String::with_capacity(s.len());
    for &b in s {
        let cp = if b < 0x80 && !(graphics && (b != 0 && (b < 0x20 || b == 0x7F))) { u32::from(b) } else { u32::from(CP437[b as usize]) };
        out.push(char::from_u32(cp).unwrap_or('?'));
    }
    out.into_bytes()
}

/// UTF-8 to CP437. ASCII stays as is (also control characters); the PC
/// symbols map back to 0x01-0x1F/0x7F; anything without a CP437 form
/// becomes `replacement` (`?` by default). Invalid UTF-8 bytes too.
pub fn utf8_to_cp437(s: &[u8], replacement: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    for chunk in s.utf8_chunks() {
        for c in chunk.valid().chars() {
            let cp = u32::from(c);
            if cp < 0x80 {
                out.push(cp as u8);
            } else if cp == 0xA0 {
                out.push(0xFF);
            } else if let Some(i) = CP437.iter().position(|&u| u32::from(u) == cp && u != 0x20) {
                out.push(i as u8);
            } else {
                out.push(replacement);
            }
        }
        out.extend(std::iter::repeat_n(replacement, chunk.invalid().len()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counting_and_slicing() {
        let s = "aé€😀b".as_bytes();
        assert_eq!(len(s), 5);
        assert_eq!(mid(s, 2, 2), "é€".as_bytes());
        assert_eq!(mid(s, 4, -1), "😀b".as_bytes());
        assert_eq!(mid(s, 9, 1), b"");
        assert_eq!(left(s, 3), "aé€".as_bytes());
        assert_eq!(right(s, 2), "😀b".as_bytes());
        assert_eq!(right(s, 99), s);
        assert_eq!(codepoint(s, 4), 0x1F600);
        assert_eq!(codepoint(s, 6), -1);
        assert_eq!(from_codepoint(0x20AC), "€".as_bytes());
        assert_eq!(from_codepoint(0xD800), b"");
        // Invalid bytes count one each and are kept.
        let bad = [b'x', 0xFF, 0xC3, b'y'];
        assert_eq!(len(&bad), 4);
        assert_eq!(mid(&bad, 2, 2), [0xFF, 0xC3]);
        assert!(!is_valid(&bad));
        assert_eq!(codepoint(&bad, 2), 0xFF);
    }

    #[test]
    fn case_and_normalization() {
        assert_eq!(upper("straße ǆ".as_bytes()), "STRASSE Ǆ".as_bytes());
        assert_eq!(lower("ÀÉÎ İ".as_bytes()), "àéî i̇".as_bytes());
        assert_eq!(upper(&[b'a', 0xFF]), [b'A', 0xFF]);
        let decomposed = "e\u{301}";
        assert_eq!(normalize(decomposed.as_bytes(), Form::Nfc), "é".as_bytes());
        assert_eq!(normalize("é".as_bytes(), Form::Nfd), decomposed.as_bytes());
        assert_eq!(normalize("ﬁ①".as_bytes(), Form::Nfkc), b"fi1");
        assert_eq!(Form::from_code(5), None);
    }

    #[test]
    fn cp437_round_trip() {
        let all: Vec<u8> = (0u8..=255).collect();
        let u = cp437_to_utf8(&all, true);
        assert_eq!(utf8_to_cp437(&u, b'?'), all);
        assert_eq!(cp437_to_utf8(&[0x01, 0x0D, 0x0A, 0x82, 0xDB], false), "\u{1}\r\né█".as_bytes());
        assert_eq!(cp437_to_utf8(&[0x01, 0x7F], true), "☺⌂".as_bytes());
        assert_eq!(utf8_to_cp437("Grüße €".as_bytes(), b'?'), [b'G', b'r', 0x81, 0xE1, b'e', b' ', b'?']);
        assert_eq!(utf8_to_cp437(&[0xFF], b'#'), b"#");
    }
}
