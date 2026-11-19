//! DOS file name patterns (`*`, `?`) of FILES, KILL and NAME.


/// Files matching a DOS pattern (`*`, `?` in the last component), as Windows and DOS
/// match them: `?` is one letter (a Thai letter is one, not three bytes), `*.*` and
/// `name.*` also take a name without an extension, and letter case counts where the
/// file system tells cases apart (Linux): there `KILL "notes.*"` also deleted NOTES.TXT.
pub(crate) fn glob_files(spec: &std::path::Path) -> Vec<std::path::PathBuf> {
    if !spec.as_os_str().as_encoded_bytes().iter().any(|c| matches!(c, b'*' | b'?')) {
        return if spec.is_file() { vec![spec.to_path_buf()] } else { vec![] };
    }
    let fold = |s: &str| -> Vec<char> { if cfg!(any(windows, target_os = "macos")) { s.to_lowercase().chars().collect() } else { s.chars().collect() } };
    let path = spec;
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(std::path::Path::new("."));
    let pat = fold(&path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
    // "name.*": the name alone too
    let bare = pat.strip_suffix(&['.', '*']).map(<[char]>::to_vec);
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let name = fold(&e.file_name().to_string_lossy());
            let hit = dos_match(&pat, &name) || bare.as_ref().is_some_and(|b| !name.contains(&'.') && dos_match(b, &name));
            if hit && e.path().is_file() {
                out.push(e.path());
            }
        }
    }
    out
}

/// `*` any run of letters, `?` one letter, anything else itself.
fn dos_match(p: &[char], t: &[char]) -> bool {
    match (p.first(), t.first()) {
        (None, None) => true,
        (Some('*'), _) => dos_match(&p[1..], t) || (!t.is_empty() && dos_match(p, &t[1..])),
        (Some('?'), Some(_)) => dos_match(&p[1..], &t[1..]),
        (Some(a), Some(b)) if a == b => dos_match(&p[1..], &t[1..]),
        _ => false,
    }
}

#[cfg(test)]
mod glob_tests {
    #[test]
    fn dos_patterns() {
        let d = std::env::temp_dir().join(format!("qb_test_{}_glob", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        for f in ["README", "notes.txt", "ก.txt", "ab.txt"] {
            std::fs::write(d.join(f), b"").unwrap();
        }
        let names = |pat: &str| {
            let mut v: Vec<String> = super::glob_files(&d.join(pat)).iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
            v.sort();
            v
        };
        // *.* takes a name without an extension too
        assert_eq!(names("*.*").len(), 4);
        assert_eq!(names("README.*"), ["README"]);
        // ? is one letter, also a Thai one
        assert_eq!(names("?.txt"), ["ก.txt"]);
        assert_eq!(names("??.txt"), ["ab.txt"]);
        if cfg!(target_os = "linux") {
            std::fs::write(d.join("NOTES.TXT"), b"").unwrap();
            assert_eq!(names("notes.*"), ["notes.txt"]);
        }
        let _ = std::fs::remove_dir_all(&d);
    }
}
