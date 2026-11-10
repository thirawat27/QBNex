//! Source decoding: UTF-8 (with or without BOM) or code page 437.
//!
//! QBasic sources are usually CP437 bytes; QB64/QBNex sources are often UTF-8.
//! Decoding is lossless: [`encode`] gives back the original bytes for every
//! input [`decode`] accepted.

/// How a source file was stored on disk.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Encoding {
    /// Valid UTF-8 without a byte order mark.
    Utf8,
    /// Valid UTF-8 that started with a byte order mark (`EF BB BF`).
    Utf8Bom,
    /// Not valid UTF-8: each byte is one CP437 character.
    Cp437,
}

/// Unicode characters for CP437 bytes 0x80..=0xFF.
const CP437_HIGH: [char; 128] = [
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å', //
    'É', 'æ', 'Æ', 'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', '¢', '£', '¥', '₧', 'ƒ', //
    'á', 'í', 'ó', 'ú', 'ñ', 'Ñ', 'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡', '«', '»', //
    '░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕', '╣', '║', '╗', '╝', '╜', '╛', '┐', //
    '└', '┴', '┬', '├', '─', '┼', '╞', '╟', '╚', '╔', '╩', '╦', '╠', '═', '╬', '╧', //
    '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘', '┌', '█', '▄', '▌', '▐', '▀', //
    'α', 'ß', 'Γ', 'π', 'Σ', 'σ', 'µ', 'τ', 'Φ', 'Θ', 'Ω', 'δ', '∞', 'φ', 'ε', '∩', //
    '≡', '±', '≥', '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√', 'ⁿ', '²', '■', '\u{a0}',
];

/// Decodes source bytes. Valid UTF-8 stays UTF-8 (a leading BOM is removed and
/// remembered); anything else is read as CP437, one character per byte.
pub fn decode(bytes: &[u8]) -> (String, Encoding) {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        if let Ok(s) = std::str::from_utf8(rest) {
            return (s.to_owned(), Encoding::Utf8Bom);
        }
    } else if let Ok(s) = std::str::from_utf8(bytes) {
        return (s.to_owned(), Encoding::Utf8);
    }
    (decode_cp437(bytes), Encoding::Cp437)
}

/// Reads every byte as one CP437 character (bytes below 0x80 are ASCII).
pub fn decode_cp437(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| if b < 0x80 { b as char } else { CP437_HIGH[(b - 0x80) as usize] })
        .collect()
}

/// Encodes text back to bytes. For [`Encoding::Cp437`], characters with no
/// CP437 byte become `?`.
pub fn encode(text: &str, encoding: Encoding) -> Vec<u8> {
    match encoding {
        Encoding::Utf8 => text.as_bytes().to_vec(),
        Encoding::Utf8Bom => {
            let mut v = vec![0xEF, 0xBB, 0xBF];
            v.extend_from_slice(text.as_bytes());
            v
        }
        Encoding::Cp437 => text.chars().map(cp437_byte).collect(),
    }
}

fn cp437_byte(c: char) -> u8 {
    if (c as u32) < 0x80 {
        return c as u8;
    }
    CP437_HIGH.iter().position(|&h| h == c).map_or(b'?', |i| 0x80 + i as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cp437_round_trips_every_byte() {
        let all: Vec<u8> = (0..=255u8).collect();
        let (text, enc) = decode(&all);
        assert_eq!(enc, Encoding::Cp437);
        assert_eq!(encode(&text, enc), all);
    }

    #[test]
    fn utf8_and_bom() {
        let (t, e) = decode("PRINT \"é\"".as_bytes());
        assert_eq!((t.as_str(), e), ("PRINT \"é\"", Encoding::Utf8));
        let bytes = b"\xEF\xBB\xBFCLS".to_vec();
        let (t, e) = decode(&bytes);
        assert_eq!((t.as_str(), e), ("CLS", Encoding::Utf8Bom));
        assert_eq!(encode(&t, e), bytes);
    }
}
