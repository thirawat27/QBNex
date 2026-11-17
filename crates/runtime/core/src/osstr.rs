//! BASIC strings are bytes; the system's names, arguments and environment are
//! `OsStr`. These two functions are the one place where one becomes the other, so
//! a name a program got from the system (`_CWD$`, `COMMAND$`, `ENVIRON$`) can be
//! given back to it unchanged, and every file statement reads a name the same way.
//!
//! - Unix: the bytes themselves (a Latin-1 or other non-UTF-8 name stays as it is).
//! - Windows: UTF-8 when the bytes are UTF-8, else the ANSI code page, as the "A"
//!   system calls read them (`win32::wide_name`); back from the system as UTF-8.

use std::ffi::{OsStr, OsString};

/// `bytes` as the system's string.
pub fn os_string(bytes: &[u8]) -> OsString {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        OsStr::from_bytes(bytes).to_os_string()
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        let mut w = crate::win32::wide_name(bytes);
        w.pop(); // the NUL
        OsString::from_wide(&w)
    }
    #[cfg(not(any(unix, windows)))]
    {
        OsString::from(String::from_utf8_lossy(bytes).into_owned())
    }
}

/// The system's string as BASIC bytes.
pub fn bytes_of(s: &OsStr) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        s.as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        s.to_string_lossy().into_owned().into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_text_goes_both_ways_unchanged() {
        for t in ["plain.txt", "โฟลเดอร์\\ไทย.bas", "a b%c"] {
            assert_eq!(bytes_of(&os_string(t.as_bytes())), t.as_bytes());
        }
    }

    #[cfg(unix)]
    #[test]
    fn unix_keeps_bytes_that_are_not_utf8() {
        let latin1 = b"caf\xe9.txt";
        assert_eq!(bytes_of(&os_string(latin1)), latin1);
    }
}
