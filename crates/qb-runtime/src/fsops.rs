//! File-system statements: KILL, NAME, CHDIR, MKDIR, RMDIR, FILES,
//! _FILEEXISTS, _DIREXISTS and _CWD$.
//!
//! Replaces `sub_kill`, `sub_name`, `sub_chdir`, `sub_mkdir`, `sub_rmdir`,
//! `sub_files`, `func__fileexists`, `func__direxists` and `func__cwd` of
//! libqb.cpp. On Windows the same Win32 / C runtime calls are made as before
//! (ANSI entry points, CRT `errno` for NAME/CHDIR/MKDIR/RMDIR), so error codes
//! match bug-for-bug. Other systems use `std` (not verified yet).

use crate::gfs::fixdir;

/// Path with OS separators and a NUL terminator (legacy `fixdir(strz)`).
fn osname(name: &[u8]) -> Vec<u8> {
    let mut v = name.to_vec();
    v.push(0);
    fixdir(&mut v);
    v
}

#[cfg(windows)]
mod crt {
    extern "C" {
        pub fn rename(old: *const u8, new: *const u8) -> i32;
        pub fn _chdir(dir: *const u8) -> i32;
        pub fn _mkdir(dir: *const u8) -> i32;
        pub fn _rmdir(dir: *const u8) -> i32;
        pub fn _errno() -> *mut i32;
    }
    pub const ENOENT: i32 = 2;
    pub const EACCES: i32 = 13;
    pub const EEXIST: i32 = 17;
    pub const EINVAL: i32 = 22;
    pub const ENOTEMPTY: i32 = 41;
    pub fn errno() -> i32 {
        // SAFETY: CRT thread-local errno.
        unsafe { *_errno() }
    }
}

/// KILL: deletes every file matching the pattern. 53 nothing matched or a
/// delete failed, 55 file in use / access denied.
#[cfg(windows)]
pub fn kill(spec: &[u8]) -> i32 {
    use crate::win32::*;
    // directory part as written (before separator conversion)
    let path_len = spec.iter().rposition(|&c| c == b'/' || c == b'\\').map_or(0, |p| p + 1);
    let path = &spec[..path_len];
    let pattern = osname(spec);
    let mut fd = WIN32_FIND_DATAA::zeroed();
    // SAFETY: NUL-terminated pattern, owned find data.
    unsafe {
        let h = FindFirstFileA(pattern.as_ptr(), &mut fd);
        if h == INVALID_HANDLE_VALUE {
            return 53;
        }
        let mut count = 0;
        loop {
            if fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0 {
                let n = fd.cFileName.iter().position(|&c| c == 0).unwrap_or(MAX_PATH);
                let mut full = path.to_vec();
                full.extend_from_slice(&fd.cFileName[..n]);
                full.push(0);
                if DeleteFileA(full.as_ptr()) == 0 {
                    let e = GetLastError();
                    FindClose(h);
                    return if matches!(e, 5 | 19 | 33 | 32) { 55 } else { 53 };
                }
                count += 1;
            }
            if FindNextFileA(h, &mut fd) == 0 {
                break;
            }
        }
        FindClose(h);
        if count == 0 {
            return 53;
        }
    }
    0
}

#[cfg(not(windows))]
pub fn kill(spec: &[u8]) -> i32 {
    let n = osname(spec);
    match std::fs::remove_file(String::from_utf8_lossy(&n[..n.len() - 1]).as_ref()) {
        Ok(()) => 0,
        Err(e) => match e.raw_os_error() {
            Some(2) => 53,
            Some(13) => 75,
            _ => 64,
        },
    }
}

/// NAME old AS new.
pub fn name(old: &[u8], new: &[u8]) -> i32 {
    let (a, b) = (osname(old), osname(new));
    #[cfg(windows)]
    {
        // SAFETY: NUL-terminated names.
        if unsafe { crt::rename(a.as_ptr(), b.as_ptr()) } != 0 {
            return match crt::errno() {
                crt::ENOENT => 53,
                crt::EINVAL => 64,
                crt::EACCES => 75,
                _ => 5,
            };
        }
        0
    }
    #[cfg(not(windows))]
    {
        let s = |v: &Vec<u8>| String::from_utf8_lossy(&v[..v.len() - 1]).into_owned();
        match std::fs::rename(s(&a), s(&b)) {
            Ok(()) => 0,
            Err(e) => match e.raw_os_error() {
                Some(2) => 53,
                Some(22) => 64,
                Some(13) => 75,
                _ => 5,
            },
        }
    }
}

/// CHDIR: 76 on any failure.
pub fn chdir(dir: &[u8]) -> i32 {
    let d = osname(dir);
    #[cfg(windows)]
    // SAFETY: NUL-terminated name.
    let failed = unsafe { crt::_chdir(d.as_ptr()) } == -1;
    #[cfg(not(windows))]
    let failed = std::env::set_current_dir(String::from_utf8_lossy(&d[..d.len() - 1]).as_ref()).is_err();
    if failed {
        76
    } else {
        0
    }
}

/// MKDIR: 75 already exists, 76 otherwise.
pub fn mkdir(dir: &[u8]) -> i32 {
    let d = osname(dir);
    #[cfg(windows)]
    {
        // SAFETY: NUL-terminated name.
        if unsafe { crt::_mkdir(d.as_ptr()) } == -1 {
            return if crt::errno() == crt::EEXIST { 75 } else { 76 };
        }
        0
    }
    #[cfg(not(windows))]
    match std::fs::create_dir(String::from_utf8_lossy(&d[..d.len() - 1]).as_ref()) {
        Ok(()) => 0,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => 75,
        Err(_) => 76,
    }
}

/// RMDIR: 75 not empty, 76 otherwise.
pub fn rmdir(dir: &[u8]) -> i32 {
    let d = osname(dir);
    #[cfg(windows)]
    {
        // SAFETY: NUL-terminated name.
        if unsafe { crt::_rmdir(d.as_ptr()) } == -1 {
            return if crt::errno() == crt::ENOTEMPTY { 75 } else { 76 };
        }
        0
    }
    #[cfg(not(windows))]
    match std::fs::remove_dir(String::from_utf8_lossy(&d[..d.len() - 1]).as_ref()) {
        Ok(()) => 0,
        Err(e) if matches!(e.raw_os_error(), Some(39) | Some(66)) => 75,
        Err(_) => 76,
    }
}

/// `_FILEEXISTS` (`dir` false) / `_DIREXISTS` (`dir` true): -1 or 0.
pub fn exists(name: &[u8], dir: bool) -> i32 {
    let n = osname(name);
    #[cfg(windows)]
    {
        use crate::win32::*;
        // SAFETY: NUL-terminated name.
        let x = unsafe { GetFileAttributesA(n.as_ptr()) };
        if x == INVALID_FILE_ATTRIBUTES {
            return 0;
        }
        let is_dir = x & FILE_ATTRIBUTE_DIRECTORY != 0;
        if is_dir == dir {
            -1
        } else {
            0
        }
    }
    #[cfg(not(windows))]
    match std::fs::metadata(String::from_utf8_lossy(&n[..n.len() - 1]).as_ref()) {
        Ok(m) if (dir && m.is_dir()) || (!dir && m.is_file()) => -1,
        _ => 0,
    }
}

/// `_CWD$`: (error, path). Error 51 when the directory changes between the
/// two Win32 calls (legacy sanity check).
pub fn cwd() -> (i32, Vec<u8>) {
    #[cfg(windows)]
    {
        use crate::win32::*;
        // SAFETY: size query, then a buffer of that size.
        unsafe {
            let length = GetCurrentDirectoryA(0, std::ptr::null_mut());
            let mut buf = vec![0u8; length as usize];
            if buf.is_empty() {
                return (7, Vec::new());
            }
            if GetCurrentDirectoryA(length, buf.as_mut_ptr()) != length - 1 {
                return (51, Vec::new());
            }
            buf.truncate(length as usize - 1);
            (0, buf)
        }
    }
    #[cfg(not(windows))]
    match std::env::current_dir() {
        Ok(p) => (0, p.to_string_lossy().into_owned().into_bytes()),
        Err(_) => (51, Vec::new()),
    }
}

/// FILES listing. Returns the error and the lines to print: the current
/// directory (short path, upper case), one 18-column entry per match, and
/// the "Bytes free" line. With error 53 only the directory line is printed
/// (legacy printed it before searching); with error 5 nothing.
#[cfg(windows)]
pub fn files(spec: Option<&[u8]>) -> (i32, Vec<Vec<u8>>) {
    use crate::win32::*;
    let mut strz: Vec<u8> = spec.map(|s| s.to_vec()).unwrap_or_default();
    strz.push(0);
    let add_star = strz.len() < 2 || strz[strz.len() - 2] == b'\\';
    if add_star {
        let last = strz.len() - 1;
        strz[last] = b'*';
        strz.push(0);
    }
    // path part including the separator (searched on the original slashes)
    let body = &strz[..strz.len()];
    let path_len = body.iter().rposition(|&c| c == b'/' || c == b'\\').map_or(0, |p| p + 1);
    let strpath = strz[..path_len].to_vec();
    let mut lines = Vec::new();
    // SAFETY: Win32 calls with owned buffers.
    unsafe {
        let mut curdir = [0u8; 4096];
        let mut curdir2 = [0u8; 4096];
        let i2 = GetCurrentDirectoryA(4096, curdir.as_mut_ptr());
        if i2 == 0 {
            return (5, lines);
        }
        let i2 = GetShortPathNameA(curdir.as_ptr(), curdir2.as_mut_ptr(), 4096);
        if i2 == 0 {
            return (5, lines);
        }
        lines.push(curdir2[..i2 as usize].to_ascii_uppercase());
        let mut pattern = strz.clone();
        fixdir(&mut pattern);
        let mut fd = WIN32_FIND_DATAA::zeroed();
        let h = FindFirstFileA(pattern.as_ptr(), &mut fd);
        if h == INVALID_HANDLE_VALUE {
            return (53, lines);
        }
        loop {
            let cz = |b: &[u8]| b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())].to_vec();
            let alt = cz(&fd.cAlternateFileName);
            let mut s = if alt.is_empty() { cz(&fd.cFileName) } else { alt }.to_ascii_uppercase();
            if s.len() < 12 {
                s.resize(12, b' ');
                if let Some(i) = s[..12].iter().position(|&c| c == b'.') {
                    // memmove(&s[8], &s[i], 4) reads past the 12 bytes for i > 8
                    let mut src = [b' '; 4];
                    for (k, slot) in src.iter_mut().enumerate() {
                        *slot = s.get(i + k).copied().unwrap_or(0);
                    }
                    s[8..12].copy_from_slice(&src);
                    for c in &mut s[i..8.max(i)] {
                        *c = b' ';
                    }
                }
            }
            s.extend_from_slice(if fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0 { b"<DIR> " } else { b"      " });
            lines.push(s);
            if FindNextFileA(h, &mut fd) == 0 {
                break;
            }
        }
        FindClose(h);
        let mut p = strpath;
        p.push(0);
        let (mut avail, mut total, mut free) = (0u64, 0u64, 0u64);
        let dir = if p.len() == 1 { std::ptr::null() } else { p.as_ptr() };
        let bytes = if GetDiskFreeSpaceExA(dir, &mut avail, &mut total, &mut free) != 0 { avail as i64 } else { 0 };
        let mut last = crate::numfmt::str_i64(bytes);
        last.extend_from_slice(b" Bytes free");
        lines.push(last);
    }
    (0, lines)
}

#[cfg(not(windows))]
pub fn files(_spec: Option<&[u8]>) -> (i32, Vec<Vec<u8>>) {
    (0, Vec::new()) // legacy FILES did nothing outside Windows
}

// --- C ABI (include/qb_runtime.h) ---------------------------------------------------

use crate::ffi_bytes;

/// # Safety
/// `s` readable for `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn qb_kill(s: *const u8, len: i32) -> i32 {
    // SAFETY: caller contract.
    kill(unsafe { ffi_bytes(s, len.max(0) as usize) })
}

/// # Safety
/// Both names readable for their lengths.
#[no_mangle]
pub unsafe extern "C" fn qb_name(a: *const u8, alen: i32, b: *const u8, blen: i32) -> i32 {
    // SAFETY: caller contract.
    unsafe { name(ffi_bytes(a, alen.max(0) as usize), ffi_bytes(b, blen.max(0) as usize)) }
}

/// # Safety
/// `s` readable for `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn qb_chdir(s: *const u8, len: i32) -> i32 {
    // SAFETY: caller contract.
    chdir(unsafe { ffi_bytes(s, len.max(0) as usize) })
}

/// # Safety
/// `s` readable for `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn qb_mkdir(s: *const u8, len: i32) -> i32 {
    // SAFETY: caller contract.
    mkdir(unsafe { ffi_bytes(s, len.max(0) as usize) })
}

/// # Safety
/// `s` readable for `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn qb_rmdir(s: *const u8, len: i32) -> i32 {
    // SAFETY: caller contract.
    rmdir(unsafe { ffi_bytes(s, len.max(0) as usize) })
}

/// # Safety
/// `s` readable for `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn qb_exists(s: *const u8, len: i32, dir: i32) -> i32 {
    // SAFETY: caller contract.
    exists(unsafe { ffi_bytes(s, len.max(0) as usize) }, dir != 0)
}

static OUT: std::sync::Mutex<Vec<u8>> = std::sync::Mutex::new(Vec::new());

/// `_CWD$`: writes a pointer to the path (valid until the next call) and
/// returns the error code.
///
/// # Safety
/// Out pointers writable.
#[no_mangle]
pub unsafe extern "C" fn qb_cwd(out: *mut *const u8, out_len: *mut i32) -> i32 {
    let (e, v) = cwd();
    let mut o = OUT.lock().unwrap_or_else(|e| e.into_inner());
    *o = v;
    // SAFETY: caller contract.
    unsafe {
        *out = o.as_ptr();
        *out_len = o.len() as i32;
    }
    e
}

/// FILES: returns the error; the lines to print are joined with '\n' in
/// `*out` (valid until the next call). See [`files`].
///
/// # Safety
/// `s` readable for `len` bytes; out pointers writable.
#[no_mangle]
pub unsafe extern "C" fn qb_files(s: *const u8, len: i32, passed: i32, out: *mut *const u8, out_len: *mut i32) -> i32 {
    // SAFETY: caller contract.
    let spec = (passed != 0).then(|| unsafe { ffi_bytes(s, len.max(0) as usize) });
    let (e, lines) = files(spec);
    let mut o = OUT.lock().unwrap_or_else(|e| e.into_inner());
    *o = lines.join(&b'\n');
    // SAFETY: caller contract.
    unsafe {
        *out = o.as_ptr();
        *out_len = o.len() as i32;
    }
    e
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfs::tests::{p, tmp};

    #[test]
    fn mkdir_rmdir_name_kill_exists() {
        let d = tmp("fsops");
        let sub = p(&d, "dir1");
        assert_eq!(exists(&sub, true), 0);
        assert_eq!(mkdir(&sub), 0);
        assert_eq!(mkdir(&sub), 75);
        assert_eq!(exists(&sub, true), -1);
        assert_eq!(exists(&sub, false), 0);
        assert_eq!(mkdir(&p(&d, "nope/x")), 76);
        std::fs::write(d.join("dir1").join("a.txt"), b"1").unwrap();
        assert_eq!(rmdir(&sub), 75);
        assert_eq!(exists(&p(&d, "dir1/a.txt"), false), -1);
        assert_eq!(name(&p(&d, "dir1/a.txt"), &p(&d, "dir1/b.txt")), 0);
        assert_eq!(name(&p(&d, "dir1/a.txt"), &p(&d, "dir1/c.txt")), 53);
        std::fs::write(d.join("dir1").join("c.dat"), b"2").unwrap();
        assert_eq!(kill(&p(&d, "dir1/*.txt")), 0);
        assert_eq!(kill(&p(&d, "dir1/*.txt")), 53);
        assert_eq!(exists(&p(&d, "dir1/c.dat"), false), -1);
        assert_eq!(kill(&p(&d, "dir1/c.dat")), 0);
        assert_eq!(rmdir(&sub), 0);
        assert_eq!(rmdir(&sub), 76);
        assert_eq!(chdir(&p(&d, "missing")), 76);
    }

    #[test]
    fn cwd_matches_std() {
        let (e, v) = cwd();
        assert_eq!(e, 0);
        assert_eq!(String::from_utf8(v).unwrap(), std::env::current_dir().unwrap().to_string_lossy());
    }

    #[cfg(windows)]
    #[test]
    fn files_lists_short_names_padded() {
        let d = tmp("files");
        std::fs::write(d.join("ab.txt"), b"1").unwrap();
        std::fs::create_dir(d.join("sub")).unwrap();
        let mut spec = p(&d, "");
        spec.push(b'\\');
        let (e, lines) = files(Some(&spec));
        assert_eq!(e, 0);
        assert!(lines.contains(&b"AB      .TXT      ".to_vec()), "{:?}", lines);
        assert!(lines.contains(&b"SUB         <DIR> ".to_vec()));
        assert!(lines.last().unwrap().ends_with(b" Bytes free"));
        let (e, lines) = files(Some(&p(&d, "*.none")));
        assert_eq!((e, lines.len()), (53, 1));
    }
}
