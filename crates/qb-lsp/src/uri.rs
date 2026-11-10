//! `file:` URI <-> path conversion (percent-encoding, Windows drive letters).

use std::path::{Path, PathBuf};

/// Path of a `file:` URI, or `None` for other schemes.
pub fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    // Drop an authority (`file://host/...`); `localhost` and empty are local.
    let slash = rest.find('/')?;
    let (host, path) = rest.split_at(slash);
    if !host.is_empty() && !host.eq_ignore_ascii_case("localhost") {
        return None;
    }
    let decoded = percent_decode(path)?;
    let b = decoded.as_bytes();
    // `/C:/dir` -> `C:/dir`
    let p = if b.len() >= 3 && b[0] == b'/' && b[1].is_ascii_alphabetic() && b[2] == b':' { &decoded[1..] } else { &decoded[..] };
    Some(PathBuf::from(p))
}

/// Canonical form used to compare URIs: re-encoded, drive letter upper-case.
pub fn uri_key(uri: &str) -> String {
    match uri_to_path(uri) {
        Some(p) => {
            let mut s = path_to_uri(&p);
            if s.as_bytes().get(9) == Some(&b':') {
                let d = s[8..9].to_ascii_uppercase();
                s.replace_range(8..9, &d);
            }
            s
        }
        None => uri.to_string(),
    }
}

/// `file:` URI of a path.
pub fn path_to_uri(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    let s = s.strip_prefix("//?/").unwrap_or(&s).to_string();
    let mut out = String::from("file://");
    if !s.starts_with('/') {
        out.push('/');
    }
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/:".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn percent_decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let h = std::str::from_utf8(&b[i + 1..i + 3]).ok()?;
            out.push(u8::from_str_radix(h, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_and_unix() {
        assert_eq!(uri_to_path("file:///d%3A/My%20Dir/a.bas").unwrap(), PathBuf::from("d:/My Dir/a.bas"));
        assert_eq!(uri_to_path("file:///home/x/a.bas").unwrap(), PathBuf::from("/home/x/a.bas"));
        assert!(uri_to_path("untitled:Untitled-1").is_none());
        assert_eq!(path_to_uri(Path::new("D:\\My Dir\\a.bas")), "file:///D:/My%20Dir/a.bas");
        assert_eq!(path_to_uri(Path::new("/home/x/a#.bas")), "file:///home/x/a%23.bas");
        assert_eq!(uri_key("file:///d%3A/My%20Dir/a.bas"), "file:///D:/My%20Dir/a.bas");
    }
}
