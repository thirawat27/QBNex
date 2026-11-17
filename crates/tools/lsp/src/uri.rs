//! `file:` URI <-> path conversion (percent-encoding, Windows drive letters,
//! UNC paths).

use std::path::{Path, PathBuf};

/// Path of a `file:` URI, or `None` for other schemes.
pub fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    // An authority (`file://host/...`): `localhost` and empty are local; any
    // other names a network share.
    let slash = rest.find('/')?;
    let (host, path) = rest.split_at(slash);
    if !host.is_empty() && !host.eq_ignore_ascii_case("localhost") {
        return unc_path(&percent_decode(host)?, &percent_decode(path)?);
    }
    let decoded = percent_decode(path)?;
    let b = decoded.as_bytes();
    // `/C:/dir` -> `C:/dir`
    let p = if b.len() >= 3 && b[0] == b'/' && b[1].is_ascii_alphabetic() && b[2] == b':' { &decoded[1..] } else { &decoded[..] };
    Some(PathBuf::from(p))
}

/// `file://server/share/x.bas` is `\\server\share\x.bas` on Windows (a WSL
/// folder too: `file://wsl$/Ubuntu/...`). Other systems have no such path.
fn unc_path(host: &str, path: &str) -> Option<PathBuf> {
    if !cfg!(windows) || host.contains(['/', '\\']) {
        return None;
    }
    // a share is needed: `\\server` alone is not a path
    let share = path.trim_start_matches('/').split('/').next().unwrap_or("");
    if share.is_empty() {
        return None;
    }
    Some(PathBuf::from(format!(r"\\{host}{}", path.replace('/', "\\"))))
}

/// Canonical form used to compare URIs: re-encoded; on Windows and macOS,
/// where file names do not tell letter case apart, the whole path in lower
/// case (else only the drive letter upper-case).
pub fn uri_key(uri: &str) -> String {
    match uri_to_path(uri) {
        Some(p) if cfg!(any(windows, target_os = "macos")) => path_to_uri(Path::new(&p.to_string_lossy().to_lowercase())),
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
    // `\\?\UNC\server\share` is `\\server\share`; `\\?\C:\x` is `C:\x`
    let s = match s.strip_prefix("//?/UNC/") {
        Some(unc) => format!("//{unc}"),
        None => s.trim_prefix("//?/").to_string(),
    };
    let mut out = String::from("file://");
    // A Windows UNC path `//server/share/x` names the server as the authority:
    // `file://server/share/x`.
    let body = match s.strip_prefix("//") {
        Some(unc) if cfg!(windows) && !unc.starts_with('/') => unc,
        _ => {
            if !s.starts_with('/') {
                out.push('/');
            }
            &s[..]
        }
    };
    for b in body.bytes() {
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
        assert_eq!(uri_key("file:///d%3A/My%20Dir/a.bas"), uri_key("file:///D:/My%20Dir/a.bas"));
    }

    /// The whole path compares without letter case where file names do.
    #[test]
    fn keys_follow_the_file_system_case_rule() {
        let (a, b) = ("file:///d%3A/My%20Dir/Inc.BI", "file:///D:/my%20dir/inc.bi");
        if cfg!(any(windows, target_os = "macos")) {
            assert_eq!(uri_key(a), uri_key(b));
            assert_eq!(uri_key("file:///home/%C3%89t%C3%A9/a.bas"), uri_key("file:///home/%C3%A9t%C3%A9/A.bas"));
        } else {
            assert_ne!(uri_key(a), uri_key(b));
            assert_eq!(uri_key("file:///d%3A/x.bas"), "file:///D:/x.bas");
        }
    }

    /// `file://server/share/...` (also WSL's `wsl$`) is a UNC path on Windows
    /// and nothing elsewhere.
    #[test]
    fn unc_uris() {
        let p = uri_to_path("file://server/share/dir/x.bas");
        let wsl = uri_to_path("file://wsl%24/Ubuntu/home/x.bas");
        if cfg!(windows) {
            assert_eq!(p, Some(PathBuf::from(r"\\server\share\dir\x.bas")));
            assert_eq!(wsl, Some(PathBuf::from(r"\\wsl$\Ubuntu\home\x.bas")));
            assert_eq!(path_to_uri(Path::new(r"\\server\share\dir\x.bas")), "file://server/share/dir/x.bas");
            assert_eq!(path_to_uri(Path::new(r"\\?\UNC\server\share\x.bas")), "file://server/share/x.bas");
            assert_eq!(path_to_uri(Path::new(r"\\wsl$\Ubuntu\x.bas")), "file://wsl%24/Ubuntu/x.bas");
            // round trip, and the same key as the server's own spelling
            assert_eq!(uri_key("file://SERVER/Share/dir/X.bas"), uri_key(&path_to_uri(&p.unwrap())));
            assert_eq!(uri_to_path("file://server/"), None);
        } else {
            assert_eq!(p, None);
            assert_eq!(wsl, None);
        }
        assert_eq!(uri_to_path("file://localhost/home/x.bas"), Some(PathBuf::from("/home/x.bas")));
    }
}
