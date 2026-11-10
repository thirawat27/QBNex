//! Base64 (RFC 4648, standard and URL-safe alphabets) and hex.

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};

/// Base64 with the standard alphabet and `=` padding.
pub fn base64_encode(data: &[u8]) -> String {
    STANDARD.encode(data)
}

/// URL-safe Base64 (`-`, `_`) without padding.
pub fn base64url_encode(data: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(data)
}

/// Decodes standard or URL-safe Base64, with or without padding; ASCII
/// whitespace (line breaks of MIME text) is ignored. `None` if invalid.
pub fn base64_decode(text: &[u8]) -> Option<Vec<u8>> {
    let mut clean: Vec<u8> = text.iter().copied().filter(|c| !c.is_ascii_whitespace()).collect();
    let url = clean.iter().any(|&c| c == b'-' || c == b'_');
    if url {
        for c in &mut clean {
            match *c {
                b'-' => *c = b'+',
                b'_' => *c = b'/',
                _ => {}
            }
        }
    }
    while clean.len() % 4 != 0 {
        clean.push(b'=');
    }
    STANDARD.decode(&clean).ok()
}

/// Lower-case hex.
pub fn hex_encode(data: &[u8]) -> String {
    hex::encode(data)
}

/// Decodes hex (either case, ASCII whitespace ignored). `None` if invalid.
pub fn hex_decode(text: &[u8]) -> Option<Vec<u8>> {
    let clean: Vec<u8> = text.iter().copied().filter(|c| !c.is_ascii_whitespace()).collect();
    hex::decode(clean).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_rfc4648_vectors() {
        let cases: [(&[u8], &str); 7] =
            [(b"", ""), (b"f", "Zg=="), (b"fo", "Zm8="), (b"foo", "Zm9v"), (b"foob", "Zm9vYg=="), (b"fooba", "Zm9vYmE="), (b"foobar", "Zm9vYmFy")];
        for (raw, enc) in cases {
            assert_eq!(base64_encode(raw), enc);
            assert_eq!(base64_decode(enc.as_bytes()).as_deref(), Some(raw));
        }
        assert_eq!(base64url_encode(&[0xfb, 0xff]), "-_8");
        assert_eq!(base64_decode(b"-_8").as_deref(), Some(&[0xfb, 0xff][..]));
        assert_eq!(base64_decode(b"Zm9v\r\nYmFy"), Some(b"foobar".to_vec()));
        assert_eq!(base64_decode(b"Zm9v!"), None);
    }

    #[test]
    fn hex_round_trip() {
        assert_eq!(hex_encode(&[0, 15, 255]), "000fff");
        assert_eq!(hex_decode(b"00 0F ff").as_deref(), Some(&[0, 15, 255][..]));
        assert_eq!(hex_decode(b"abc"), None);
        assert_eq!(hex_decode(b"zz"), None);
    }
}
