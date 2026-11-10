//! The `$CONSOLE` paths: PRINT/INPUT through stdout/stdin and the Windows
//! console API behind LOCATE, CSRLIN, POS, COLOR, CLS, WIDTH, SCREEN(),
//! `_CONSOLECURSOR`, `_CONSOLEINPUT` and `_CINP` when `_DEST`/`_SOURCE` is
//! `_CONSOLE`.
//!
//! stdout/stdin go through the C runtime's `FILE` streams, like legacy
//! (text-mode newline translation, buffering shared with the C++ code that
//! still prints).

use super::Host;
use core::ffi::c_void;

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" {
    fn fwrite(p: *const c_void, size: usize, n: usize, f: *mut c_void) -> usize;
    fn fflush(f: *mut c_void) -> i32;
    fn fgetc(f: *mut c_void) -> i32;
}

/// Browser build (R13-wasm): no C runtime. Console output collects in a
/// buffer the page drains ([`take_stdout`]); stdin is always at its end.
#[cfg(target_arch = "wasm32")]
static WASM_STDOUT: std::sync::Mutex<Vec<u8>> = std::sync::Mutex::new(Vec::new());

/// Console output written since the last call (browser build).
#[cfg(target_arch = "wasm32")]
pub fn take_stdout() -> Vec<u8> {
    std::mem::take(&mut *WASM_STDOUT.lock().unwrap_or_else(|e| e.into_inner()))
}

#[cfg(target_arch = "wasm32")]
unsafe fn fwrite(p: *const c_void, size: usize, n: usize, _f: *mut c_void) -> usize {
    // SAFETY: callers pass a buffer of `size * n` bytes.
    let s = unsafe { std::slice::from_raw_parts(p.cast::<u8>(), size * n) };
    WASM_STDOUT.lock().unwrap_or_else(|e| e.into_inner()).extend_from_slice(s);
    n
}
#[cfg(target_arch = "wasm32")]
unsafe fn fflush(_f: *mut c_void) -> i32 {
    0
}
#[cfg(target_arch = "wasm32")]
unsafe fn fgetc(_f: *mut c_void) -> i32 {
    -1
}
#[cfg(target_arch = "wasm32")]
unsafe fn c_stdout() -> *mut c_void {
    core::ptr::null_mut()
}
#[cfg(target_arch = "wasm32")]
unsafe fn c_stdin() -> *mut c_void {
    core::ptr::null_mut()
}

#[cfg(windows)]
unsafe extern "C" {
    fn __acrt_iob_func(index: u32) -> *mut c_void;
}
#[cfg(windows)]
unsafe fn c_stdout() -> *mut c_void {
    __acrt_iob_func(1)
}
#[cfg(windows)]
unsafe fn c_stdin() -> *mut c_void {
    __acrt_iob_func(0)
}
#[cfg(all(unix, not(target_os = "macos")))]
unsafe extern "C" {
    static stdout: *mut c_void;
    static stdin: *mut c_void;
}
#[cfg(all(unix, not(target_os = "macos")))]
unsafe fn c_stdout() -> *mut c_void {
    stdout
}
#[cfg(all(unix, not(target_os = "macos")))]
unsafe fn c_stdin() -> *mut c_void {
    stdin
}
#[cfg(target_os = "macos")]
unsafe extern "C" {
    static __stdoutp: *mut c_void;
    static __stdinp: *mut c_void;
}
#[cfg(target_os = "macos")]
unsafe fn c_stdout() -> *mut c_void {
    __stdoutp
}
#[cfg(target_os = "macos")]
unsafe fn c_stdin() -> *mut c_void {
    __stdinp
}

/// Column (1-based) of stdout as written by this program, for output that is
/// not a console (redirected to a file or pipe), where the console cursor
/// cannot be asked (L-02).
static COLUMN: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(1);

/// Column after writing `s` from column `col`: line breaks (LF, CR) return
/// to 1; UTF-8 continuation bytes do not move the column.
pub fn advance_column(mut col: i32, s: &[u8]) -> i32 {
    for &b in s {
        match b {
            b'\n' | b'\r' => col = 1,
            0x80..=0xBF => {}
            _ => col += 1,
        }
    }
    col
}

fn track(s: &[u8]) {
    use std::sync::atomic::Ordering;
    let c = COLUMN.load(Ordering::Relaxed);
    COLUMN.store(advance_column(c, s), Ordering::Relaxed);
}

/// Column tracked for non-console stdout (see [`advance_column`]).
pub fn tracked_column() -> i32 {
    COLUMN.load(std::sync::atomic::Ordering::Relaxed)
}

/// `printf`-style write to stdout without flushing.
pub fn write_raw(s: &[u8]) {
    if s.is_empty() {
        return;
    }
    track(s);
    // SAFETY: valid buffer; the CRT stream is process-global.
    unsafe {
        fwrite(s.as_ptr().cast(), 1, s.len(), c_stdout());
    }
}

/// One byte of stdin (`fgetc`), None at end of input.
pub fn getc() -> Option<u8> {
    // SAFETY: CRT stream.
    let c = unsafe { fgetc(c_stdin()) };
    if c < 0 {
        None
    } else {
        Some(c as u8)
    }
}

#[cfg(windows)]
mod win {
    use windows_sys::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Globalization::{GetACP, MultiByteToWideChar, CP_UTF8, MB_ERR_INVALID_CHARS};
    use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
    use windows_sys::Win32::Storage::FileSystem::{CreateFileA, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING};
    pub use windows_sys::Win32::System::Console::*;

    pub struct State {
        pub utf8_initialized: bool,
        pub original_output_cp: u32,
    }
    pub static mut STATE: State = State { utf8_initialized: false, original_output_cp: 0 };

    pub fn std(h: STD_HANDLE) -> HANDLE {
        // SAFETY: no preconditions.
        unsafe { GetStdHandle(h) }
    }

    pub fn is_console(h: HANDLE) -> bool {
        if h.is_null() || h == INVALID_HANDLE_VALUE {
            return false;
        }
        let mut mode = 0;
        // SAFETY: valid out pointer.
        unsafe { GetConsoleMode(h, &mut mode) != 0 }
    }

    /// `qb_init_windows_console_utf8`.
    pub unsafe fn init_utf8() {
        let st = &mut *core::ptr::addr_of_mut!(STATE);
        if st.utf8_initialized {
            return;
        }
        st.utf8_initialized = true;
        let mut out_cp = GetConsoleOutputCP();
        let in_cp = GetConsoleCP();
        let _ = in_cp;
        if out_cp == 0 {
            out_cp = GetACP();
        }
        st.original_output_cp = out_cp;
        let out = std(STD_OUTPUT_HANDLE);
        if is_console(out) {
            SetConsoleOutputCP(CP_UTF8);
            let mut mode = 0;
            if GetConsoleMode(out, &mut mode) != 0 {
                SetConsoleMode(out, mode | ENABLE_PROCESSED_OUTPUT);
            }
        }
        let inp = std(STD_INPUT_HANDLE);
        if is_console(inp) {
            SetConsoleCP(CP_UTF8);
        }
    }

    pub unsafe fn to_wide(text: &[u8], cp: u32, flags: u32) -> Option<Vec<u16>> {
        if text.is_empty() {
            return Some(Vec::new());
        }
        let n = MultiByteToWideChar(cp, flags, text.as_ptr(), text.len() as i32, core::ptr::null_mut(), 0);
        if n <= 0 {
            return None;
        }
        let mut v = vec![0u16; n as usize];
        if MultiByteToWideChar(cp, flags, text.as_ptr(), text.len() as i32, v.as_mut_ptr(), n) > 0 {
            Some(v)
        } else {
            None
        }
    }

    pub fn utf8_strict() -> u32 {
        MB_ERR_INVALID_CHARS
    }

    /// `CreateFileA("CONOUT$")` + `GetConsoleScreenBufferInfo`. Legacy left
    /// the structure uninitialised when there is no console; here it is zero.
    pub unsafe fn conout_info() -> CONSOLE_SCREEN_BUFFER_INFO {
        let sa = SECURITY_ATTRIBUTES {
            nLength: core::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: core::ptr::null_mut(),
            bInheritHandle: 1,
        };
        let h = CreateFileA(
            c"CONOUT$".as_ptr().cast(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            &sa,
            OPEN_EXISTING,
            0,
            core::ptr::null_mut(),
        );
        let mut info: CONSOLE_SCREEN_BUFFER_INFO = core::mem::zeroed();
        GetConsoleScreenBufferInfo(h, &mut info);
        if h != INVALID_HANDLE_VALUE {
            CloseHandle(h);
        }
        info
    }

    pub unsafe fn conout() -> HANDLE {
        let sa = SECURITY_ATTRIBUTES {
            nLength: core::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: core::ptr::null_mut(),
            bInheritHandle: 1,
        };
        CreateFileA(
            c"CONOUT$".as_ptr().cast(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            &sa,
            OPEN_EXISTING,
            0,
            core::ptr::null_mut(),
        )
    }
    pub unsafe fn close(h: HANDLE) {
        if h != INVALID_HANDLE_VALUE {
            CloseHandle(h);
        }
    }
}

/// `qb_write_utf8`: PRINT to the console. On a Windows console the UTF-8 text
/// is written as UTF-16 (falling back to the original code page for invalid
/// UTF-8); otherwise the bytes go to stdout.
pub fn write_utf8(text: &[u8], finish_on_new_line: bool) {
    #[cfg(windows)]
    // SAFETY: Win32 calls with valid buffers.
    unsafe {
        let out = win::std(win::STD_OUTPUT_HANDLE);
        if win::is_console(out) {
            win::init_utf8();
            if !text.is_empty() {
                let wide = win::to_wide(text, windows_sys::Win32::Globalization::CP_UTF8, win::utf8_strict())
                    .or_else(|| {
                        let st = &*core::ptr::addr_of!(win::STATE);
                        let cp = if st.original_output_cp != 0 {
                            st.original_output_cp
                        } else {
                            windows_sys::Win32::Globalization::GetACP()
                        };
                        win::to_wide(text, cp, 0)
                    })
                    .unwrap_or_default();
                if !wide.is_empty() {
                    let mut written = 0;
                    win::WriteConsoleW(out, wide.as_ptr().cast(), wide.len() as u32, &mut written, core::ptr::null());
                }
            }
            if finish_on_new_line {
                let nl = [13u16, 10];
                let mut written = 0;
                win::WriteConsoleW(out, nl.as_ptr().cast(), 2, &mut written, core::ptr::null());
            }
            return;
        }
    }
    track(text);
    if finish_on_new_line {
        track(b"\n");
    }
    // SAFETY: CRT stream.
    unsafe {
        let f = c_stdout();
        if !text.is_empty() {
            fwrite(text.as_ptr().cast(), 1, text.len(), f);
        }
        if finish_on_new_line {
            fwrite(b"\n".as_ptr().cast(), 1, 1, f);
        }
        fflush(f);
    }
}

/// POS(0) of the console (column of the console cursor).
#[cfg(windows)]
pub fn pos() -> i32 {
    // SAFETY: Win32 queries.
    unsafe {
        if !win::is_console(win::std(win::STD_OUTPUT_HANDLE)) {
            return tracked_column(); // redirected: no console cursor to ask (L-02)
        }
        win::conout_info().dwCursorPosition.X as i32 + 1
    }
}

/// CSRLIN of the console.
#[cfg(windows)]
pub fn csrlin() -> i32 {
    // SAFETY: Win32 query.
    unsafe { win::conout_info().dwCursorPosition.Y as i32 + 1 }
}

/// LOCATE on the console.
pub fn locate(mut row: i32, mut column: i32, passed: i32) {
    #[cfg(windows)]
    {
        let _ = passed;
        // SAFETY: Win32 calls.
        unsafe {
            let info = win::conout_info();
            if column == 0 {
                column = info.dwCursorPosition.X as i32 + 1;
            }
            if row == 0 {
                row = info.dwCursorPosition.Y as i32 + 1;
            }
            let pos = win::COORD { X: (column - 1) as i16, Y: (row - 1) as i16 };
            win::SetConsoleCursorPosition(win::std(win::STD_OUTPUT_HANDLE), pos);
        }
    }
    #[cfg(not(windows))]
    {
        // legacy printed "\033[%d;%dG" with one argument (undefined); this
        // writes the column it meant (unverified)
        if passed & 1 != 0 && passed & 2 != 0 {
            write_raw(format!("\x1b[{row};{column}H").as_bytes());
        } else if passed & 1 != 0 {
            write_raw(format!("\x1b[{row}d").as_bytes());
        } else if passed & 2 != 0 {
            write_raw(format!("\x1b[{column}G").as_bytes());
        }
        let _ = (&mut row, &mut column);
    }
}

/// COLOR on the console. Returns an error number (non-Windows range check).
pub fn color(col1: u32, col2: u32, passed: i32) -> i32 {
    #[cfg(windows)]
    {
        let _ = passed;
        // SAFETY: Win32 call.
        unsafe {
            win::SetConsoleTextAttribute(
                win::std(win::STD_OUTPUT_HANDLE),
                col2.wrapping_mul(16).wrapping_add(col1) as i32 as u16,
            );
        }
        0
    }
    #[cfg(not(windows))]
    {
        if (passed & 1 != 0 && col1 > 255) || (passed & 2 != 0 && col2 > 255) {
            return 5;
        }
        if passed & 1 != 0 {
            write_raw(format!("\x1b[38;5;{col1}m").as_bytes());
        }
        if passed & 2 != 0 {
            write_raw(format!("\x1b[48;5;{col2}m").as_bytes());
        }
        0
    }
}

/// CLS on the console.
pub fn cls(h: &mut dyn Host, use_color: u32, passed: i32) {
    #[cfg(windows)]
    {
        let _ = (use_color, passed, &h);
        // legacy: system("cls")
        // SAFETY: CRT flush before the child writes to the same console.
        unsafe { fflush(c_stdout()) };
        let _ = std::process::Command::new("cmd").args(["/c", "cls"]).status();
        locate(1, 1, 3);
    }
    #[cfg(not(windows))]
    {
        if passed & 2 != 0 {
            let e = color(0, use_color, 2);
            if e != 0 {
                h.error(e);
            }
        }
        write_raw(b"\x1b[2J");
        locate(1, 1, 3);
    }
}

/// WIDTH [w][, h][, buffer w][, buffer h] on the Windows console.
#[cfg(windows)]
pub fn width(value1: i32, value2: i32, mut value3: i32, mut value4: i32) {
    // SAFETY: Win32 calls.
    unsafe {
        let info = win::conout_info();
        let out = win::std(win::STD_OUTPUT_HANDLE);
        let mut w = value1;
        let mut hh = value2;
        if w <= 0 {
            w = info.srWindow.Right as i32 - info.srWindow.Left as i32 + 1;
        }
        if hh <= 0 {
            hh = info.srWindow.Bottom as i32 - info.srWindow.Top as i32 + 1;
        }
        if value3 <= 0 {
            value3 = info.dwSize.X as i32;
        }
        if value4 <= 0 {
            value4 = info.dwSize.Y as i32;
        }
        if value3 < value1 {
            value3 = value1;
        }
        if value4 < value2 {
            value4 = value2;
        }
        let rect = win::SMALL_RECT { Left: 0, Top: 0, Right: (w - 1) as i16, Bottom: (hh - 1) as i16 };
        let size = win::COORD { X: value3 as i16, Y: value4 as i16 };
        win::SetConsoleScreenBufferSize(out, size);
        win::SetConsoleWindowInfo(out, 1, &rect);
    }
}

/// SCREEN(y, x[, 1]) on the Windows console.
#[cfg(windows)]
pub fn screen_read(y: i32, x: i32, returncol: bool) -> u32 {
    // SAFETY: Win32 calls with valid buffers.
    unsafe {
        let h = win::conout();
        let at = win::COORD { X: (x - 1) as i16, Y: (y - 1) as i16 };
        let mut n = 0u32;
        let mut a: u16 = 0;
        let r = if returncol {
            win::ReadConsoleOutputAttribute(h, &mut a, 1, at, &mut n);
            a as u32
        } else {
            win::ReadConsoleOutputCharacterA(h, (&mut a as *mut u16).cast(), 1, at, &mut n);
            (a & 0xff) as u32
        };
        win::close(h);
        r
    }
}

/// `_KEYCLEAR`: the Windows console input buffer.
pub fn flush_input() {
    #[cfg(windows)]
    // SAFETY: Win32 call.
    unsafe {
        win::FlushConsoleInputBuffer(win::std(win::STD_INPUT_HANDLE));
    }
}

/// `_CONSOLECURSOR [_SHOW|_HIDE][, size]` (1 = show, 2 = hide).
pub fn console_cursor(visible: i32, size: i32, passed: i32) {
    #[cfg(windows)]
    // SAFETY: Win32 calls.
    unsafe {
        let h = win::std(win::STD_OUTPUT_HANDLE);
        let mut info: win::CONSOLE_CURSOR_INFO = core::mem::zeroed();
        win::GetConsoleCursorInfo(h, &mut info);
        if visible == 1 {
            info.bVisible = 1;
        }
        if visible == 2 {
            info.bVisible = 0;
        }
        if passed != 0 && (0..=100).contains(&size) {
            info.dwSize = size as u32;
        }
        win::SetConsoleCursorInfo(h, &info);
    }
    #[cfg(not(windows))]
    let _ = (visible, size, passed);
}

/// Last console key event (`_CINP`) and mouse state (`_MOUSEX` etc. on `_CONSOLE`).
#[unsafe(no_mangle)]
pub static mut consolekey: i32 = 0;
#[unsafe(no_mangle)]
pub static mut consolemousex: i32 = 0;
#[unsafe(no_mangle)]
pub static mut consolemousey: i32 = 0;
#[unsafe(no_mangle)]
pub static mut consolebutton: i32 = 0;

/// `_CONSOLEINPUT`: waits for one console input event; 1 = key, 2 = mouse, 0 = other.
pub fn console_input() -> i32 {
    #[cfg(windows)]
    // SAFETY: Win32 calls with valid buffers.
    unsafe {
        let h = win::std(win::STD_INPUT_HANDLE);
        let mut mode = 0u32;
        win::GetConsoleMode(h, &mut mode);
        win::SetConsoleMode(h, win::ENABLE_EXTENDED_FLAGS);
        win::SetConsoleMode(h, mode | win::ENABLE_WINDOW_INPUT | win::ENABLE_MOUSE_INPUT);
        let mut rec: win::INPUT_RECORD = core::mem::zeroed();
        let mut n = 0u32;
        win::ReadConsoleInputA(h, &mut rec, 1, &mut n);
        match rec.EventType as u32 {
            win::KEY_EVENT => {
                let k = rec.Event.KeyEvent;
                consolekey = k.wVirtualScanCode as i32;
                if k.bKeyDown == 0 {
                    consolekey = -consolekey;
                }
                return 1;
            }
            win::MOUSE_EVENT => {
                let m = rec.Event.MouseEvent;
                // legacy subtracted an uninitialised srWindow.Top; the window top is read here
                let top = win::conout_info().srWindow.Top as i32;
                consolemousex = m.dwMousePosition.X as i32 + 1;
                consolemousey = m.dwMousePosition.Y as i32 - top + 1;
                consolebutton = m.dwButtonState as i32;
                return 2;
            }
            _ => {}
        }
    }
    0
}

/// `_CINP[(toggle)]`: last console key scan code (negative = released), then 0.
pub fn cinp(mut toggle: i32, passed: i32) -> i32 {
    if !cfg!(windows) {
        return 0;
    }
    // SAFETY: program-thread global.
    unsafe {
        let temp = consolekey;
        consolekey = 0;
        if passed == 0 {
            toggle = 1;
        }
        if toggle != 0 || temp >= 0 {
            temp
        } else {
            -temp + 128
        }
    }
}
