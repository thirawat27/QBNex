//! The console of a program without a window: keys typed there for INKEY$ and
//! INPUT$, and the text PRINT sends to a terminal.
//!
//! Keys come back as INKEY$ gives them: a character's bytes (UTF-8 past ASCII,
//! Enter is CHR$(13), Backspace CHR$(8)), or CHR$(0) + the scan code for a key
//! without a character (arrows, F1-F12, Home, End, Insert, Delete, Page Up/Down).

use super::utf8;

/// `b` with each byte from 128 up taken as code page 437 and written as UTF-8.
pub fn cp437_text(b: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(b.len());
    for &c in b {
        if c < 0x80 {
            out.push(c);
        } else {
            utf8::push_utf8(&mut out, utf8::cp437(c));
        }
    }
    out
}

/// Text for a terminal from the bytes one PRINT item wrote. It is UTF-8 when all of it
/// is, as on a window page; else its bytes are code page 437 (box drawing). A character
/// split over two writes (Thai printed a byte at a time with MID$) waits in `carry` for
/// its other bytes. Nothing invalid reaches the terminal: on a Windows console that is
/// not in code page 65001 one invalid byte made Rust's stdout drop all later output.
pub fn terminal_text(carry: &mut Vec<u8>, b: &[u8]) -> Vec<u8> {
    let mut data = std::mem::take(carry);
    data.extend_from_slice(b);
    match std::str::from_utf8(&data) {
        Ok(_) => data,
        Err(e) if e.error_len().is_none() => {
            // ends inside a character: the rest may come with the next write
            carry.extend_from_slice(&data[e.valid_up_to()..]);
            data.truncate(e.valid_up_to());
            data
        }
        Err(_) => cp437_text(&data),
    }
}

/// What [`terminal_text`] kept back, when no more bytes come for it (the program waits or ends).
pub fn terminal_text_end(carry: &mut Vec<u8>) -> Vec<u8> {
    let d = std::mem::take(carry);
    cp437_text(&d)
}

/// The INKEY$ value of a Windows key event: `ch` its character (0 for none), `vk` its
/// virtual key, `scan` its scan code. None for a key that gives nothing (Shift, Ctrl, Alt,
/// the lock keys, the Windows keys).
pub fn windows_key(ch: u32, vk: u16, scan: u16) -> Option<Vec<u8>> {
    if ch != 0 {
        let mut out = Vec::new();
        utf8::push_utf8(&mut out, ch);
        return Some(out);
    }
    match vk {
        0x10..=0x12 | 0x14 | 0x5B..=0x5D | 0x90 | 0x91 | 0xA0..=0xA5 => None,
        _ if scan == 0 || scan > 0xFF => None,
        _ => Some(vec![0, scan as u8]),
    }
}

/// One key from the bytes a terminal sent: (bytes used, INKEY$ value). The value is empty
/// for a sequence that is no key here; (0, empty) when `b` is empty.
pub fn terminal_key(b: &[u8]) -> (usize, Vec<u8>) {
    let ext = |n: usize, code: u8| (n, vec![0, code]);
    match b {
        [] => (0, Vec::new()),
        [27, b'[', rest @ ..] => {
            // CSI: parameters, then a final byte from @ to ~
            let Some(end) = rest.iter().position(|c| (0x40..=0x7E).contains(c)) else {
                return (b.len(), Vec::new());
            };
            let used = end + 3;
            let param: u32 = rest[..end]
                .split(|&c| c == b';')
                .next()
                .and_then(|p| std::str::from_utf8(p).ok())
                .and_then(|p| p.parse().ok())
                .unwrap_or(0);
            let code = match rest[end] {
                b'A' => 72,
                b'B' => 80,
                b'C' => 77,
                b'D' => 75,
                b'H' => 71,
                b'F' => 79,
                b'P' => 59,
                b'Q' => 60,
                b'R' => 61,
                b'S' => 62,
                b'~' => match param {
                    1 | 7 => 71,
                    2 => 82,
                    3 => 83,
                    4 | 8 => 79,
                    5 => 73,
                    6 => 81,
                    11..=15 => 48 + param as u8,
                    17..=21 => 47 + param as u8,
                    23 => 133,
                    24 => 134,
                    _ => return (used, Vec::new()),
                },
                _ => return (used, Vec::new()),
            };
            ext(used, code)
        }
        [27, b'O', c, ..] => match c {
            b'P'..=b'S' => ext(3, 59 + (c - b'P')),
            b'A' => ext(3, 72),
            b'B' => ext(3, 80),
            b'C' => ext(3, 77),
            b'D' => ext(3, 75),
            b'H' => ext(3, 71),
            b'F' => ext(3, 79),
            _ => (3, Vec::new()),
        },
        [0x7F, ..] => (1, vec![8]),
        [b'\n', ..] => (1, vec![13]),
        [c, ..] if *c < 0x80 => (1, vec![*c]),
        _ => {
            let (used, _) = utf8::decode(b);
            let used = used.max(1);
            (used, b[..used].to_vec())
        }
    }
}

/// The next key typed on the console, or None when none waits (`wait` false) or stdin is
/// no console or terminal (redirected from a file or pipe).
pub fn read_key(wait: bool) -> Option<Vec<u8>> {
    #[cfg(windows)]
    return win::read_key(wait);
    #[cfg(all(unix, not(target_arch = "wasm32")))]
    return unix::read_key(wait);
    #[cfg(not(any(windows, all(unix, not(target_arch = "wasm32")))))]
    {
        let _ = wait;
        None
    }
}

/// Puts the terminal back in line mode after [`read_key`] (before INPUT reads a line, before
/// another program starts, at the end). Nothing to do on Windows, where reading keys does not
/// change the console mode.
pub fn key_mode_off() {
    #[cfg(all(unix, not(target_arch = "wasm32")))]
    unix::off();
}

#[cfg(windows)]
mod win {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Console::*;
    use windows_sys::Win32::System::Threading::{WaitForSingleObject, INFINITE};

    /// The first half of a character outside the BMP, until its second half comes.
    static HIGH: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    pub fn read_key(wait: bool) -> Option<Vec<u8>> {
        use std::sync::atomic::Ordering;
        // SAFETY: Win32 calls on the console input handle with valid buffers.
        unsafe {
            let h = GetStdHandle(STD_INPUT_HANDLE);
            let mut mode = 0;
            if h.is_null() || h == INVALID_HANDLE_VALUE || GetConsoleMode(h, &mut mode) == 0 {
                return None;
            }
            loop {
                let mut n = 0u32;
                if GetNumberOfConsoleInputEvents(h, &mut n) == 0 {
                    return None;
                }
                if n == 0 {
                    if !wait {
                        return None;
                    }
                    WaitForSingleObject(h, INFINITE);
                    continue;
                }
                let mut rec: INPUT_RECORD = core::mem::zeroed();
                let mut got = 0u32;
                if ReadConsoleInputW(h, &mut rec, 1, &mut got) == 0 || got == 0 {
                    return None;
                }
                if rec.EventType as u32 != KEY_EVENT {
                    continue;
                }
                let k = rec.Event.KeyEvent;
                if k.bKeyDown == 0 {
                    continue;
                }
                let mut ch = u32::from(k.uChar.UnicodeChar);
                if (0xD800..0xDC00).contains(&ch) {
                    HIGH.store(ch, Ordering::Relaxed);
                    continue;
                }
                if (0xDC00..0xE000).contains(&ch) {
                    let high = HIGH.swap(0, Ordering::Relaxed);
                    if high == 0 {
                        continue;
                    }
                    ch = 0x10000 + ((high - 0xD800) << 10) + (ch - 0xDC00);
                }
                if let Some(key) = super::windows_key(ch, k.wVirtualKeyCode, k.wVirtualScanCode) {
                    return Some(key);
                }
            }
        }
    }
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
mod unix {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    /// The terminal's line-mode settings while keys are read one at a time.
    static mut SAVED: core::mem::MaybeUninit<libc::termios> = core::mem::MaybeUninit::uninit();
    static ACTIVE: AtomicBool = AtomicBool::new(false);
    /// Bytes read from the terminal and not yet given out as keys.
    static PENDING: Mutex<Vec<u8>> = Mutex::new(Vec::new());

    extern "C" fn at_exit() {
        off();
    }

    /// Ctrl+C or a kill while keys are read: the terminal gets its echo back first.
    extern "C" fn on_signal(sig: libc::c_int) {
        off();
        // SAFETY: async-signal-safe calls.
        unsafe {
            libc::signal(sig, libc::SIG_DFL);
            libc::raise(sig);
        }
    }

    /// Keys without Enter and without echo; output and Ctrl+C stay as they were.
    fn on() -> bool {
        if ACTIVE.load(Ordering::Acquire) {
            return true;
        }
        // SAFETY: termios calls on fd 0 with valid buffers; SAVED is written before ACTIVE is set.
        unsafe {
            if libc::isatty(0) == 0 {
                return false;
            }
            let mut t: libc::termios = core::mem::zeroed();
            if libc::tcgetattr(0, &mut t) != 0 {
                return false;
            }
            (*core::ptr::addr_of_mut!(SAVED)).write(t);
            t.c_lflag &= !(libc::ICANON | libc::ECHO);
            t.c_cc[libc::VMIN] = 0;
            t.c_cc[libc::VTIME] = 0;
            if libc::tcsetattr(0, libc::TCSANOW, &t) != 0 {
                return false;
            }
            ACTIVE.store(true, Ordering::Release);
            static HOOKS: std::sync::Once = std::sync::Once::new();
            HOOKS.call_once(|| {
                libc::atexit(at_exit);
                let handler = on_signal as extern "C" fn(libc::c_int) as libc::sighandler_t;
                for sig in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP, libc::SIGQUIT] {
                    let before = libc::signal(sig, handler);
                    if before != libc::SIG_DFL {
                        // ignored (nohup) or handled by someone else: left as it was
                        libc::signal(sig, before);
                    }
                }
            });
        }
        true
    }

    pub fn off() {
        if ACTIVE.swap(false, Ordering::AcqRel) {
            // SAFETY: SAVED was written before ACTIVE was set.
            unsafe { libc::tcsetattr(0, libc::TCSANOW, (*core::ptr::addr_of!(SAVED)).as_ptr()) };
        }
    }

    pub fn read_key(wait: bool) -> Option<Vec<u8>> {
        if !on() {
            return None;
        }
        let mut pending = PENDING.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            let (used, key) = super::terminal_key(&pending);
            if used > 0 {
                pending.drain(..used);
                if !key.is_empty() {
                    return Some(key);
                }
                continue;
            }
            if wait {
                let mut p = libc::pollfd { fd: 0, events: libc::POLLIN, revents: 0 };
                // SAFETY: one valid pollfd.
                if unsafe { libc::poll(&mut p, 1, -1) } < 0 {
                    if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                        continue;
                    }
                    return None;
                }
            }
            let mut buf = [0u8; 64];
            // SAFETY: `buf` holds 64 bytes.
            let n = unsafe { libc::read(0, buf.as_mut_ptr().cast(), buf.len()) };
            if n > 0 {
                pending.extend_from_slice(&buf[..n as usize]);
            } else if !wait || (n < 0 && std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted) {
                // nothing typed, or the terminal is gone
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_items_reach_a_terminal_as_utf8() {
        let mut carry = Vec::new();
        let mut out = Vec::new();
        // PRINT CHR$(201); STRING$(5, 205); CHR$(187): one byte waits, then the box
        for item in [&[201u8][..], &[205; 5], &[187], b"\n"] {
            out.extend(terminal_text(&mut carry, item));
        }
        out.extend(terminal_text_end(&mut carry));
        assert_eq!(String::from_utf8(out).unwrap(), "╔═════╗\n");
        // Thai a byte at a time: the character comes out whole
        let thai = "ไทย".as_bytes();
        let mut out = Vec::new();
        for b in thai {
            out.extend(terminal_text(&mut carry, &[*b]));
        }
        assert_eq!(out, thai);
        assert!(carry.is_empty());
        // a lone byte the program ends on
        assert_eq!(terminal_text(&mut carry, &[219]), b"");
        assert_eq!(String::from_utf8(terminal_text_end(&mut carry)).unwrap(), "█");
        // "x"; CHR$(224); "y"
        let mut out = Vec::new();
        for item in [&b"x"[..], &[224], b"y"] {
            out.extend(terminal_text(&mut carry, item));
        }
        assert_eq!(String::from_utf8(out).unwrap(), "xαy");
    }

    #[test]
    fn windows_keys() {
        assert_eq!(windows_key('a' as u32, 0x41, 0x1E), Some(b"a".to_vec()));
        assert_eq!(windows_key(13, 0x0D, 0x1C), Some(vec![13]));
        assert_eq!(windows_key('ก' as u32, 0x44, 0x20), Some("ก".as_bytes().to_vec()));
        assert_eq!(windows_key(0, 0x26, 72), Some(vec![0, 72]));
        assert_eq!(windows_key(0, 0x70, 59), Some(vec![0, 59]));
        assert_eq!(windows_key(0, 0x10, 0x2A), None);
        assert_eq!(windows_key(0, 0x14, 0x3A), None);
    }

    #[test]
    fn terminal_keys() {
        assert_eq!(terminal_key(b""), (0, vec![]));
        assert_eq!(terminal_key(b"ab"), (1, b"a".to_vec()));
        assert_eq!(terminal_key(b"\n"), (1, vec![13]));
        assert_eq!(terminal_key(&[0x7F]), (1, vec![8]));
        assert_eq!(terminal_key(b"\x1b[A"), (3, vec![0, 72]));
        assert_eq!(terminal_key(b"\x1b[1;5D"), (6, vec![0, 75]));
        assert_eq!(terminal_key(b"\x1b[3~x"), (4, vec![0, 83]));
        assert_eq!(terminal_key(b"\x1bOP"), (3, vec![0, 59]));
        assert_eq!(terminal_key(b"\x1b[15~"), (5, vec![0, 63]));
        assert_eq!(terminal_key(b"\x1b[21~"), (5, vec![0, 68]));
        assert_eq!(terminal_key(b"\x1b[24~"), (5, vec![0, 134]));
        assert_eq!(terminal_key(b"\x1b"), (1, vec![27]));
        assert_eq!(terminal_key("ไ!".as_bytes()), (3, "ไ".as_bytes().to_vec()));
    }
}
