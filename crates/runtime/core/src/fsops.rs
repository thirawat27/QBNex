//! File-system helpers: the FILES listing and `_DIR$`.
//!
//! On Windows the same Win32 calls are made as in QB64, through the Unicode ("W")
//! entry points: names are the runtime's UTF-8 bytes (the ANSI ones failed in a
//! folder with a Thai name, and gave ? for a Thai user name).
//! KILL, NAME, CHDIR, MKDIR, RMDIR, `_FILEEXISTS`, `_DIREXISTS` and `_CWD$`
//! are implemented by the runner (`engine/exec/src/rt/builtins.rs`).

#[cfg(windows)]
use crate::gfs::fixdir;

/// A folder `_DIR$` can name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UserFolder {
    Documents,
    Music,
    Pictures,
    Videos,
    Downloads,
    Desktop,
    AppData,
    LocalAppData,
    ProgramFiles,
    ProgramFilesX86,
    Temp,
}

/// The folder a `_DIR$` name stands for (compared without case); `None` for any other text.
fn user_folder(context: &[u8]) -> Option<UserFolder> {
    use UserFolder::*;
    Some(match context.to_ascii_uppercase().as_slice() {
        b"TEXT" | b"DOCUMENT" | b"DOCUMENTS" | b"MY DOCUMENTS" => Documents,
        b"MUSIC" | b"AUDIO" | b"SOUND" | b"SOUNDS" | b"MY MUSIC" => Music,
        b"PICTURE" | b"PICTURES" | b"IMAGE" | b"IMAGES" | b"MY PICTURES" => Pictures,
        b"DCIM" | b"CAMERA" | b"CAMERA ROLL" | b"PHOTO" | b"PHOTOS" => Pictures,
        b"MOVIE" | b"MOVIES" | b"VIDEO" | b"VIDEOS" | b"MY VIDEOS" => Videos,
        b"DOWNLOAD" | b"DOWNLOADS" => Downloads,
        b"DESKTOP" => Desktop,
        b"APPDATA" | b"APPLICATION DATA" | b"PROGRAM DATA" | b"DATA" => AppData,
        b"LOCALAPPDATA" | b"LOCAL APPLICATION DATA" | b"LOCAL PROGRAM DATA" | b"LOCAL DATA" => LocalAppData,
        b"PROGRAMFILES" | b"PROGRAM FILES" => ProgramFiles,
        b"PROGRAMFILESX86" | b"PROGRAMFILES X86" | b"PROGRAM FILES X86" | b"PROGRAM FILES 86" => ProgramFilesX86,
        b"PROGRAM FILES (X86)" | b"PROGRAMFILES (X86)" | b"PROGRAM FILES(X86)" => ProgramFilesX86,
        b"TEMP" | b"TEMP FILES" => Temp,
        _ => return None,
    })
}

/// `_DIR$(context)`: the user's folder of that name with a trailing `\`
/// (QB64's `func__dir`). A name that is not known, or a folder Windows
/// cannot report, gives the desktop folder, and `.\` when that fails too.
/// "DOWNLOADS" is the profile folder's `Downloads`, created when missing.
#[cfg(windows)]
pub fn special_dir(context: &[u8]) -> Vec<u8> {
    use crate::win32::{MAX_PATH, SHGetFolderPathW, utf8_name};
    const CSIDL_DESKTOP: i32 = 0x00;
    const CSIDL_PERSONAL: i32 = 0x05;
    const CSIDL_MYMUSIC: i32 = 0x0d;
    const CSIDL_MYVIDEO: i32 = 0x0e;
    const CSIDL_APPDATA: i32 = 0x1a;
    const CSIDL_LOCAL_APPDATA: i32 = 0x1c;
    const CSIDL_PROGRAM_FILES: i32 = 0x26;
    const CSIDL_MYPICTURES: i32 = 0x27;
    const CSIDL_PROFILE: i32 = 0x28;
    const CSIDL_PROGRAM_FILESX86: i32 = 0x2a;
    let shell = |csidl: i32| -> Option<Vec<u8>> {
        let mut buf = [0u16; MAX_PATH];
        // SAFETY: the API writes at most MAX_PATH units including the NUL.
        let hr = unsafe { SHGetFolderPathW(std::ptr::null_mut(), csidl, std::ptr::null_mut(), 0, buf.as_mut_ptr()) };
        (hr >= 0).then(|| utf8_name(&buf))
    };
    let with_sep = |mut p: Vec<u8>| {
        p.push(b'\\');
        p
    };
    use UserFolder::*;
    let found = match user_folder(context) {
        Some(Downloads) => shell(CSIDL_PROFILE).map(|mut p| {
            p.extend_from_slice(b"\\Downloads");
            let _ = std::fs::create_dir(String::from_utf8_lossy(&p).as_ref());
            with_sep(p)
        }),
        Some(Temp) => {
            let mut p = std::env::temp_dir().to_string_lossy().into_owned().into_bytes();
            if p.last().is_some_and(|&c| c != b'\\') {
                p.push(b'\\');
            }
            (!p.is_empty()).then_some(p)
        }
        Some(f) => shell(match f {
            Documents => CSIDL_PERSONAL,
            Music => CSIDL_MYMUSIC,
            Pictures => CSIDL_MYPICTURES,
            Videos => CSIDL_MYVIDEO,
            AppData => CSIDL_APPDATA,
            LocalAppData => CSIDL_LOCAL_APPDATA,
            ProgramFiles => CSIDL_PROGRAM_FILES,
            ProgramFilesX86 => CSIDL_PROGRAM_FILESX86,
            Desktop | Downloads | Temp => CSIDL_DESKTOP,
        })
        .map(with_sep),
        None => None,
    };
    found.or_else(|| shell(CSIDL_DESKTOP).map(with_sep)).unwrap_or_else(|| b".\\".to_vec())
}

/// `_DIR$(context)` off Windows: always the current folder, as in QB64.
#[cfg(not(windows))]
pub fn special_dir(context: &[u8]) -> Vec<u8> {
    let _ = user_folder(context);
    b"./".to_vec()
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
        let mut curdir = [0u16; 4096];
        let mut curdir2 = [0u16; 4096];
        let i2 = GetCurrentDirectoryW(4096, curdir.as_mut_ptr());
        if i2 == 0 {
            return (5, lines);
        }
        let i2 = GetShortPathNameW(curdir.as_ptr(), curdir2.as_mut_ptr(), 4096);
        if i2 == 0 {
            return (5, lines);
        }
        lines.push(utf8_name(&curdir2[..i2 as usize]).to_ascii_uppercase());
        let mut pattern = strz.clone();
        fixdir(&mut pattern);
        let mut fd = WIN32_FIND_DATAW::zeroed();
        let h = FindFirstFileW(wide_name(&pattern).as_ptr(), &mut fd);
        if h == INVALID_HANDLE_VALUE {
            return (53, lines);
        }
        loop {
            let alt = utf8_name(&fd.cAlternateFileName);
            let mut s = if alt.is_empty() { utf8_name(&fd.cFileName) } else { alt }.to_ascii_uppercase();
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
            if FindNextFileW(h, &mut fd) == 0 {
                break;
            }
        }
        FindClose(h);
        let p = wide_name(&strpath);
        let (mut avail, mut total, mut free) = (0u64, 0u64, 0u64);
        let dir = if strpath.is_empty() { std::ptr::null() } else { p.as_ptr() };
        let bytes = if GetDiskFreeSpaceExW(dir, &mut avail, &mut total, &mut free) != 0 { avail as i64 } else { 0 };
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

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    use crate::gfs::tests::{p, tmp};

    #[test]
    fn dir_names_are_matched_without_case_and_unknown_ones_fall_back() {
        assert_eq!(user_folder(b"my documents"), Some(UserFolder::Documents));
        assert_eq!(user_folder(b"Camera Roll"), Some(UserFolder::Pictures));
        assert_eq!(user_folder(b"program files (x86)"), Some(UserFolder::ProgramFilesX86));
        assert_eq!(user_folder(b"nowhere"), None);
        let sep: &[u8] = if cfg!(windows) { b"\\" } else { b"/" };
        for name in [&b"nowhere"[..], b"desktop", b"temp", b"documents"] {
            let d = special_dir(name);
            assert!(d.ends_with(sep), "{}", String::from_utf8_lossy(&d));
        }
        // an unknown name is the desktop folder (the current folder off Windows)
        assert_eq!(special_dir(b"nowhere"), special_dir(b"DESKTOP"));
        if cfg!(windows) {
            let t = special_dir(b"Temp Files");
            assert!(std::path::Path::new(String::from_utf8_lossy(&t).as_ref()).is_dir());
        }
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

    #[cfg(windows)]
    #[test]
    fn files_works_in_a_thai_folder() {
        // the ANSI calls failed here: FILES was "Illegal function call"
        let d = tmp("files_thai").join("โฟลเดอร์");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("ab.txt"), b"1").unwrap();
        let mut spec = p(&d, "");
        spec.push(b'\\');
        let (e, lines) = files(Some(&spec));
        assert_eq!(e, 0, "{:?}", lines);
        assert!(lines.contains(&b"AB      .TXT      ".to_vec()), "{:?}", lines);
        assert!(lines.last().unwrap().ends_with(b" Bytes free"));
    }
}
