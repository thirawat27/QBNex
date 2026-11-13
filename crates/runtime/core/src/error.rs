//! Runtime error machinery.
//!
//! Replaces `human_error`, `error`, `fix_error` (message texts and the
//! decision what to do with an error), `get_error_err`/`get_error_erl` and
//! the error globals of `qbx.cpp`. The parts that need generated code stay
//! in C++: `fix_error` jumps into `QBMAIN` (the `ON ERROR` handler) when Rust
//! says so, and `end()` waits for the window thread.
//!
//! The error globals keep their C names because generated code reads and
//! writes them on every line (`if (new_error) goto skip...`).

#![allow(non_upper_case_globals)]

use crate::program;
use core::ptr::{addr_of, addr_of_mut};
use std::ffi::c_char;

/// Error raised by the current statement (0 = none). Generated code tests it.
#[unsafe(no_mangle)]
pub static mut new_error: u32 = 0;
/// Backup of [`new_error`] used by `$DEBUG` (vWATCH) code.
#[unsafe(no_mangle)]
pub static mut bkp_new_error: u32 = 0;
/// `ERR`.
#[unsafe(no_mangle)]
pub static mut error_err: u32 = 0;
/// `ERL`.
#[unsafe(no_mangle)]
pub static mut error_erl: f64 = 0.0;
/// Set when the error handler is entered; `RESUME` checks it.
#[unsafe(no_mangle)]
pub static mut error_occurred: u32 = 0;
/// Label number of the active `ON ERROR GOTO` (0 = none).
#[unsafe(no_mangle)]
pub static mut error_goto_line: u32 = 0;
/// 1 while an `ON ERROR` handler runs.
#[unsafe(no_mangle)]
pub static mut error_handling: u32 = 0;
/// Set by `RESUME` (retry the failing line).
#[unsafe(no_mangle)]
pub static mut error_retry: u32 = 0;
/// `_ERRORLINE`: source line of the last error.
#[unsafe(no_mangle)]
pub static mut ercl: u32 = 0;
/// `_INCLERRORLINE`: line inside an `$INCLUDE` file (0 = main module).
#[unsafe(no_mangle)]
pub static mut inclercl: u32 = 0;
/// `_INCLERRORFILE$` (NUL-terminated, owned by generated code).
#[unsafe(no_mangle)]
pub static mut includedfilename: *mut c_char = core::ptr::null_mut();
/// Last numbered line passed (`ERL`).
#[unsafe(no_mangle)]
pub static mut last_line: f64 = 0.0;

/// Win32 `MessageBox` flags/results the legacy dialogs used.
pub const MB_OK: i32 = 0;
pub const MB_YESNO: i32 = 4;
pub const MB_SYSTEMMODAL: i32 = 0x1000;
pub const IDOK: i32 = 1;
pub const IDNO: i32 = 7;

/// `human_error`: message for an error number (`_ERRORMESSAGE$`).
pub fn human_error(code: i32) -> &'static str {
    human_error_z(code).trim_end_matches('\0')
}

fn human_error_z(code: i32) -> &'static str {
    match code {
        0 => "No error\0",
        1 => "NEXT without FOR\0",
        2 => "Syntax error\0",
        3 => "RETURN without GOSUB\0",
        4 => "Out of DATA\0",
        5 => "Illegal function call\0",
        6 => "Overflow\0",
        7 => "Out of memory\0",
        8 => "Label not defined\0",
        9 => "Subscript out of range\0",
        10 => "Duplicate definition\0",
        11 => "Division by zero\0", // legacy: "Unprintable error"
        12 => "Illegal in direct mode\0",
        13 => "Type mismatch\0",
        14 => "Out of string space\0",
        16 => "String formula too complex\0",
        17 => "Cannot continue\0",
        18 => "Function not defined\0",
        19 => "No RESUME\0",
        20 => "RESUME without error\0",
        24 => "Device timeout\0",
        25 => "Device fault\0",
        26 => "FOR without NEXT\0",
        27 => "Out of paper\0",
        29 => "WHILE without WEND\0",
        30 => "WEND without WHILE\0",
        33 => "Duplicate label\0",
        35 => "Subprogram not defined\0",
        37 => "Argument-count mismatch\0",
        38 => "Array not defined\0",
        40 => "Variable required\0",
        50 => "FIELD overflow\0",
        51 => "Internal error\0",
        52 => "Bad file name or number\0",
        53 => "File not found\0",
        54 => "Bad file mode\0",
        55 => "File already open\0",
        56 => "FIELD statement active\0",
        57 => "Device I/O error\0",
        58 => "File already exists\0",
        59 => "Bad record length\0",
        61 => "Disk full\0",
        62 => "Input past end of file\0",
        63 => "Bad record number\0",
        64 => "Bad file name\0",
        67 => "Too many files\0",
        68 => "Device unavailable\0",
        69 => "Communication-buffer overflow\0",
        70 => "Permission denied\0",
        71 => "Disk not ready\0",
        72 => "Disk-media error\0",
        73 => "Feature unavailable\0",
        74 => "Rename across disks\0",
        75 => "Path/File access error\0",
        76 => "Path not found\0",
        258 => "Invalid handle\0",
        300 => "Memory region out of range\0",
        301 => "Invalid size\0",
        302 => "Source memory region out of range\0",
        303 => "Destination memory region out of range\0",
        304 => "Source and destination memory regions out of range\0",
        305 => "Source memory has been freed\0",
        306 => "Destination memory has been freed\0",
        307 => "Memory already freed\0",
        308 => "Memory has been freed\0",
        309 => "Memory not initialized\0",
        310 => "Source memory not initialized\0",
        311 => "Destination memory not initialized\0",
        312 => "Source and destination memory not initialized\0",
        313 => "Source and destination memory have been freed\0",
        314 => "_ASSERT failed\0",
        315 => "_ASSERT failed (check console for description)\0",
        _ => "Unprintable error\0",
    }
}

/// Errors that end the program at once with a message box:
/// `(message, title)`. `None` for ordinary (trappable) errors.
pub fn critical_error(code: i32) -> Option<(&'static str, &'static str)> {
    const OOM_TITLES: [&str; 17] = [
        "Critical Error #2\0",
        "Critical Error #3\0",
        "Critical Error #4\0",
        "Critical Error #5\0",
        "Critical Error #6\0",
        "Critical Error #7\0",
        "Critical Error #8\0",
        "Critical Error #9\0",
        "Critical Error #10\0",
        "Critical Error #11\0",
        "Critical Error #12\0",
        "Critical Error #13\0",
        "Critical Error #14\0",
        "Critical Error #15\0",
        "Critical Error #16\0",
        "Critical Error #17\0",
        "Critical Error #18\0",
    ];
    const CRIT: &str = "Critical Error\0";
    Some(match code {
        257 => ("Out of memory\0", "Critical Error #1\0"),
        502..=518 => ("Out of memory\0", OOM_TITLES[(code - 502) as usize]),
        256 => ("Out of stack space\0", CRIT),
        259 => ("Cannot find dynamic library file\0", CRIT),
        260 | 261 => ("Sub/Function does not exist in dynamic library\0", CRIT),
        270 => ("_GL command called outside of SUB _GL's scope\0", CRIT),
        271 => ("END/SYSTEM called within SUB _GL's scope\0", CRIT),
        _ => return None,
    })
}

/// Message box of a fatal runtime fault that cannot continue: the critical
/// errors, plus a hardware arithmetic fault in runtime code (11, "Division by
/// zero"). Integer `\` and `MOD` in BASIC code raise error 11 as an ordinary,
/// trappable error instead (checked in qbx.cpp, as in QBasic 4.5).
pub fn fatal_message(code: i32) -> (&'static str, &'static str) {
    if code == 11 {
        return ("Division by zero\0", "Critical Error\0");
    }
    critical_error(code).unwrap_or(("Internal error\0", "Critical Error\0"))
}

/// Ends the program after a fatal fault (see [`fatal_message`]): message
/// box (or one stderr line without dialogs) and exit.
pub fn fatal(code: i32) -> ! {
    let (msg, title) = fatal_message(code);
    let (msg, title) = (msg.trim_end_matches('\0').as_bytes(), title.trim_end_matches('\0').as_bytes());
    if host::no_dialog() {
        host::report(&report_line(title, msg));
        host::exit(NO_DIALOG_EXIT_CODE);
    }
    host::message_box(msg, title, MB_OK | MB_SYSTEMMODAL);
    host::exit(0);
}

/// C ABI of [`fatal`] (the SIGFPE handler in qbx.cpp).
#[unsafe(no_mangle)]
pub extern "C" fn qb_fatal_error(code: i32) {
    fatal(code);
}
/// What `error(n)` does to the error state when `n` is not critical:
/// the first error of a statement wins, numbers <= 0 become 5.
pub fn latch(current: u32, code: i32) -> Option<u32> {
    if current != 0 {
        return None;
    }
    Some(if code <= 0 { 5 } else { code as u32 })
}

/// The program file name shown in the dialog title: `COMMAND$(0)` without
/// its folder (text up to the first NUL, as printed with `%s`).
pub fn binary_name(command0: &[u8]) -> &[u8] {
    let s = match command0.iter().position(|&c| c == 0) {
        Some(p) => &command0[..p],
        None => command0,
    };
    match s.iter().rposition(|&c| c == b'/' || c == b'\\') {
        Some(p) => &s[p + 1..],
        None => s,
    }
}

/// Texts of the "Unhandled Error" / "Critical Error" dialog.
#[derive(Debug, PartialEq, Eq)]
pub struct ErrorDialog {
    pub title: Vec<u8>,
    pub body: Vec<u8>,
    /// Errors 300-315 cannot be handled: the dialog has only OK and the
    /// program ends.
    pub critical: bool,
}

/// Builds the dialog `fix_error` shows for an error that no `ON ERROR`
/// handler takes. `include_file` is `None` for a NULL pointer.
pub fn error_dialog(code: u32, ercl_: u32, inclercl_: u32, include_file: Option<&[u8]>, command0: &[u8]) -> ErrorDialog {
    let critical = (300..=315).contains(&code);
    let mut body = Vec::new();
    let line = if inclercl_ != 0 { inclercl_ } else { ercl_ };
    body.extend_from_slice(format!("Line: {line} (in ").as_bytes());
    if inclercl_ != 0 {
        match include_file {
            Some(f) => body.extend_from_slice(c_text(f)),
            None => body.extend_from_slice(b"(null)"),
        }
    } else {
        body.extend_from_slice(b"main module");
    }
    body.extend_from_slice(b")\n");
    body.extend_from_slice(human_error(code as i32).as_bytes());
    if !critical {
        body.extend_from_slice(b"\nContinue?");
    }
    let mut title = Vec::new();
    title.extend_from_slice(if critical { b"Critical Error #" as &[u8] } else { b"Unhandled Error #" });
    title.extend_from_slice(code.to_string().as_bytes());
    title.extend_from_slice(b" - ");
    title.extend_from_slice(binary_name(command0));
    ErrorDialog { title, body, critical }
}

/// Text up to the first NUL (a C string printed with `%s`).
fn c_text(s: &[u8]) -> &[u8] {
    match s.iter().position(|&c| c == 0) {
        Some(p) => &s[..p],
        None => s,
    }
}

/// Outcome of `fix_error` for the C++ caller.
#[derive(Debug, PartialEq, Eq)]
pub enum FixError {
    /// An `ON ERROR` handler takes the error: ERR/ERL are set, C++ must
    /// enter `QBMAIN` (which jumps to the handler).
    Handle,
    /// The user chose to continue after the dialog; the error is cleared.
    Continue,
}

// ------------------------------------------------------------- host calls

#[cfg(not(test))]
mod host {
    use std::ffi::c_char;
    unsafe extern "C" {
        /// `MessageBox2` (window owner, headless mode) in libqb.cpp.
        fn qb_host_message_box(message: *const c_char, title: *const c_char, kind: i32) -> i32;
        /// `end()` in libqb.cpp: waits for the window thread, never returns.
        fn qb_host_end();
    }
    fn cstr(s: &[u8]) -> Vec<u8> {
        let mut v = s.to_vec();
        v.push(0);
        v
    }
    pub fn message_box(message: &[u8], title: &[u8], kind: i32) -> i32 {
        let m = cstr(message);
        let t = cstr(title);
        // SAFETY: both buffers are NUL-terminated.
        unsafe { qb_host_message_box(m.as_ptr().cast(), t.as_ptr().cast(), kind) }
    }
    pub fn exit(code: i32) -> ! {
        std::process::exit(code)
    }
    pub fn end() {
        // SAFETY: plain call into libqb.cpp.
        unsafe { qb_host_end() }
    }
    /// True when error dialogs must not be shown (see [`super::no_dialog_policy`]).
    pub fn no_dialog() -> bool {
        static NO_DIALOG: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *NO_DIALOG.get_or_init(super::no_dialog_now)

    }
    pub fn report(line: &[u8]) {
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let mut e = std::io::stderr().lock();
        let _ = e.write_all(line);
        let _ = e.write_all(b"\n");
        let _ = e.flush();
    }
}

#[cfg(test)]
pub(crate) mod host {
    use std::cell::RefCell;
    type Boxes = Vec<(Vec<u8>, Vec<u8>, i32)>;
    thread_local! {
        pub static BOXES: RefCell<Boxes> = const { RefCell::new(Vec::new()) };
        pub static ANSWER: RefCell<i32> = const { RefCell::new(0) };
        pub static ENDED: RefCell<bool> = const { RefCell::new(false) };
    }
    pub fn message_box(message: &[u8], title: &[u8], kind: i32) -> i32 {
        BOXES.with(|b| b.borrow_mut().push((message.to_vec(), title.to_vec(), kind)));
        ANSWER.with(|a| *a.borrow())
    }
    pub fn exit(code: i32) -> ! {
        panic!("exit({code})")
    }
    pub fn end() {
        ENDED.with(|e| *e.borrow_mut() = true);
    }
    thread_local! {
        pub static NO_DIALOG: RefCell<bool> = const { RefCell::new(false) };
        pub static REPORTS: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
    }
    pub fn no_dialog() -> bool {
        NO_DIALOG.with(|n| *n.borrow())
    }
    pub fn report(line: &[u8]) {
        REPORTS.with(|r| r.borrow_mut().push(line.to_vec()));
    }
}

/// /// How a standard stream of the process is connected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stream {
    /// An interactive console / terminal.
    Terminal,
    /// A file, pipe or device such as NUL (`prog > out.txt`, test runners, CI).
    Redirected,
    /// Not there: a GUI program started without a console (Windows Explorer).
    Absent,
}

/// State of stdin (0), stdout (1) or stderr (2).
pub fn stream_state(fd: u32) -> Stream {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
        use windows_sys::Win32::System::Console::{GetConsoleMode, GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE};
        let which = match fd {
            0 => STD_INPUT_HANDLE,
            1 => STD_OUTPUT_HANDLE,
            _ => STD_ERROR_HANDLE,
        };
        // SAFETY: Win32 queries without preconditions.
        unsafe {
            let h = GetStdHandle(which);
            if h.is_null() || h == INVALID_HANDLE_VALUE {
                return Stream::Absent;
            }
            let mut mode = 0;
            if GetConsoleMode(h, &mut mode) != 0 { Stream::Terminal } else { Stream::Redirected }
        }
    }
    #[cfg(not(windows))]
    {
        use std::io::IsTerminal;
        let tty = match fd {
            0 => std::io::stdin().is_terminal(),
            1 => std::io::stdout().is_terminal(),
            _ => std::io::stderr().is_terminal(),
        };
        if tty { Stream::Terminal } else { Stream::Redirected }
    }
}

/// `QBNEX_NO_ERROR_DIALOG`: `Some(true)` set (not "" or "0"), `Some(false)`
/// set to "" or "0", `None` not set.
fn env_flag(env: Option<&str>) -> Option<bool> {
    env.map(|v| !v.is_empty() && v != "0")
}

/// Error dialogs are replaced by a line on stderr (and the program ends with
/// exit code 1) when `QBNEX_NO_ERROR_DIALOG` is set to anything but "" or "0",
/// or when stdout or stderr is redirected (test runners, pipes, CI). A
/// program without console streams (GUI) and interactive runs keep the
/// QBasic-style dialog.
pub fn no_dialog_policy(env: Option<&str>, stdout: Stream, stderr: Stream) -> bool {
    env_flag(env).unwrap_or(stdout == Stream::Redirected || stderr == Stream::Redirected)
}

/// Whether END shows "Press any key to continue" and waits. A console
/// program waits only when stdin and stdout are both the console; a window
/// program waits in its window unless `QBNEX_NO_ERROR_DIALOG` is set.
pub fn end_prompt_policy(env: Option<&str>, window: bool, stdin: Stream, stdout: Stream) -> bool {
    if env_flag(env) == Some(true) {
        return false;
    }
    window || (stdin == Stream::Terminal && stdout == Stream::Terminal)
}

/// [`no_dialog_policy`] for this process.
pub fn no_dialog_now() -> bool {
    no_dialog_policy(std::env::var("QBNEX_NO_ERROR_DIALOG").ok().as_deref(), stream_state(1), stream_state(2))
}

/// C ABI of [`end_prompt_policy`] for this process (`sub_end` in libqb.cpp):
/// 1 = show the prompt and wait, 0 = end at once.
#[unsafe(no_mangle)]
pub extern "C" fn qb_end_prompt_wanted(window: i32) -> i32 {
    let env = std::env::var("QBNEX_NO_ERROR_DIALOG").ok();
    i32::from(end_prompt_policy(env.as_deref(), window != 0, stream_state(0), stream_state(1)))
}
/// One-line form of a dialog: "<title>: <body lines joined with ': '>",
/// without the "Continue?" question.
pub fn report_line(title: &[u8], body: &[u8]) -> Vec<u8> {
    let body = body.trim_suffix(b"\nContinue?");
    let mut line = title.to_vec();
    for part in body.split(|&c| c == b'\n') {
        line.extend_from_slice(b": ");
        line.extend_from_slice(part);
    }
    line
}

/// Exit code of a program ended by an error without a dialog.
pub const NO_DIALOG_EXIT_CODE: i32 = 1;

/// `error(n)`: critical errors show a message box and end the process;
/// otherwise the first error of the statement is latched into `new_error`
/// and the program thread is told to check events.
pub fn raise(code: i32) {
    if let Some((msg, title)) = critical_error(code) {
        let (msg, title) = (msg.trim_end_matches('\0').as_bytes(), title.trim_end_matches('\0').as_bytes());
        if host::no_dialog() {
            host::report(&report_line(title, msg));
            host::exit(NO_DIALOG_EXIT_CODE);
        }
        host::message_box(msg, title, MB_OK | MB_SYSTEMMODAL);
        host::exit(0);
    }
    if let Some(n) = latch(program::load(addr_of!(new_error)), code) {
        program::store(addr_of_mut!(new_error), n);
        program::signal_event();
    }
}

/// `fix_error`: called by `evnt()` when `new_error` is set.
pub fn fix_error(command0: &[u8]) -> FixError {
    // SAFETY: the error globals are only used by the program thread here.
    unsafe {
        let code = new_error;
        let critical = (300..=315).contains(&code);
        if error_goto_line == 0 || error_handling != 0 || critical {
            let file = if includedfilename.is_null() {
                None
            } else {
                Some(std::ffi::CStr::from_ptr(includedfilename).to_bytes())
            };
            let d = error_dialog(code, ercl, inclercl, file, command0);
            if host::no_dialog() {
                // Same as answering "No": end the program, but with a failing exit code.
                host::report(&report_line(&d.title, &d.body));
                if d.critical {
                    host::exit(NO_DIALOG_EXIT_CODE);
                }
                program::store(addr_of_mut!(program::exit_code), i64::from(NO_DIALOG_EXIT_CODE));
                program::store(addr_of_mut!(program::close_program), 1);
                host::end();
                new_error = 0;
                return FixError::Continue;
            }
            let v = if d.critical {
                host::message_box(&d.body, &d.title, MB_OK);
                host::exit(0);
            } else {
                host::message_box(&d.body, &d.title, MB_YESNO | MB_SYSTEMMODAL)
            };
            if v == IDNO || v == IDOK {
                program::store(addr_of_mut!(program::close_program), 1);
                host::end();
            }
            new_error = 0;
            return FixError::Continue;
        }
        error_err = code;
        new_error = 0;
        error_erl = last_line;
        error_occurred = 1;
        FixError::Handle
    }
}

// ---------------------------------------------------------------- C ABI

/// `human_error`: NUL-terminated static message text.
#[unsafe(no_mangle)]
pub extern "C" fn qb_human_error(code: i32) -> *const c_char {
    human_error_z(code).as_ptr().cast()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset() {
        unsafe {
            new_error = 0;
            error_err = 0;
            error_erl = 0.0;
            error_occurred = 0;
            error_goto_line = 0;
            error_handling = 0;
            ercl = 0;
            inclercl = 0;
            includedfilename = core::ptr::null_mut();
            last_line = 0.0;
            program::qbevent = 0;
            program::close_program = 0;
        }
        host::BOXES.with(|b| b.borrow_mut().clear());
        host::ENDED.with(|e| *e.borrow_mut() = false);
    }

    #[test]
    fn messages() {
        assert_eq!(human_error(0), "No error");
        assert_eq!(human_error(6), "Overflow");
        assert_eq!(human_error(15), "Unprintable error");
        assert_eq!(human_error(11), "Division by zero");
        assert_eq!(human_error(315), "_ASSERT failed (check console for description)");
        assert_eq!(human_error(-1), "Unprintable error");
        let p = qb_human_error(76);
        assert_eq!(unsafe { std::ffi::CStr::from_ptr(p) }.to_bytes(), b"Path not found");
    }

    #[test]
    fn critical_table() {
        assert_eq!(critical_error(257).map(|c| c.1), Some("Critical Error #1\0"));
        assert_eq!(critical_error(518).map(|c| c.1), Some("Critical Error #18\0"));
        // Division by zero is an ordinary (trappable) error, as in QBasic 4.5.
        assert_eq!(critical_error(11), None);
        assert_eq!(fatal_message(11), ("Division by zero\0", "Critical Error\0"));
        assert_eq!(fatal_message(256).0, "Out of stack space\0");
        assert_eq!(critical_error(262), None);
        assert_eq!(critical_error(519), None);
        assert_eq!(critical_error(5), None);
    }

    #[test]
    fn first_error_wins_and_zero_becomes_illegal_call() {
        let _g = program::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        raise(0);
        assert_eq!(unsafe { new_error }, 5);
        assert_eq!(unsafe { program::qbevent }, 1);
        raise(9);
        assert_eq!(unsafe { new_error }, 5);
        unsafe { new_error = 0 };
        raise(-3);
        assert_eq!(unsafe { new_error }, 5);
        reset();
    }

    #[test]
    fn critical_error_exits() {
        let _g = program::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        let r = std::panic::catch_unwind(|| raise(505));
        assert!(r.is_err());
        host::BOXES.with(|b| {
            assert_eq!(b.borrow()[0], (b"Out of memory".to_vec(), b"Critical Error #5".to_vec(), MB_OK | MB_SYSTEMMODAL))
        });
        reset();
    }

    #[test]
    fn dialog_texts() {
        let d = error_dialog(53, 10, 0, None, b"C:\\dir/sub\\prog.exe\0");
        assert_eq!(d.title, b"Unhandled Error #53 - prog.exe");
        assert_eq!(d.body, b"Line: 10 (in main module)\nFile not found\nContinue?");
        assert!(!d.critical);
        let d = error_dialog(305, 10, 7, Some(b"inc.bi"), b"prog");
        assert_eq!(d.title, b"Critical Error #305 - prog");
        assert_eq!(d.body, b"Line: 7 (in inc.bi)\nSource memory has been freed");
        assert!(d.critical);
    }

    #[test]
    fn handled_error_sets_err_and_erl() {
        let _g = program::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        unsafe {
            new_error = 11 + 2;
            error_goto_line = 3;
            last_line = 120.0;
        }
        assert_eq!(fix_error(b"p"), FixError::Handle);
        unsafe {
            assert_eq!((new_error, error_err, error_erl, error_occurred), (0, 13, 120.0, 1));
        }
        reset();
    }

    #[test]
    fn unhandled_error_asks_and_closes_on_no() {
        let _g = program::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        unsafe {
            new_error = 9;
            ercl = 4;
        }
        host::ANSWER.with(|a| *a.borrow_mut() = 6); // IDYES
        assert_eq!(fix_error(b"x"), FixError::Continue);
        assert_eq!(unsafe { new_error }, 0);
        assert!(!host::ENDED.with(|e| *e.borrow()));
        unsafe { new_error = 9 };
        host::ANSWER.with(|a| *a.borrow_mut() = IDNO);
        assert_eq!(fix_error(b"x"), FixError::Continue);
        assert!(host::ENDED.with(|e| *e.borrow()));
        assert_eq!(unsafe { program::close_program }, 1);
        host::BOXES.with(|b| assert_eq!(b.borrow()[0].2, MB_YESNO | MB_SYSTEMMODAL));
        // A handler that is already running does not take a second error.
        unsafe {
            new_error = 5;
            error_goto_line = 2;
            error_handling = 1;
        }
        host::ANSWER.with(|a| *a.borrow_mut() = 6);
        assert_eq!(fix_error(b"x"), FixError::Continue);
        reset();
    }

    #[test]
    fn no_dialog_policy_env_and_redirection() {
        use Stream::*;
        assert!(no_dialog_policy(Some("1"), Terminal, Terminal));
        assert!(!no_dialog_policy(Some("0"), Redirected, Redirected));
        assert!(!no_dialog_policy(Some(""), Redirected, Redirected));
        assert!(!no_dialog_policy(None, Terminal, Terminal));
        assert!(no_dialog_policy(None, Redirected, Terminal));
        assert!(no_dialog_policy(None, Terminal, Redirected));
        // A GUI program started without a console has no standard streams at
        // all: it keeps its dialogs.
        assert!(!no_dialog_policy(None, Absent, Absent));
    }

    #[test]
    fn end_prompt_only_when_interactive() {
        use Stream::*;
        // console program: prompt only when both stdin and stdout are the console
        assert!(end_prompt_policy(None, false, Terminal, Terminal));
        assert!(!end_prompt_policy(None, false, Redirected, Terminal));
        assert!(!end_prompt_policy(None, false, Terminal, Redirected));
        assert!(!end_prompt_policy(Some("1"), false, Terminal, Terminal));
        assert!(end_prompt_policy(Some("0"), false, Terminal, Terminal));
        // window program: the prompt is in the window; only the variable turns it off
        assert!(end_prompt_policy(None, true, Absent, Absent));
        assert!(end_prompt_policy(None, true, Redirected, Redirected));
        assert!(!end_prompt_policy(Some("1"), true, Absent, Absent));
    }

    #[test]
    fn report_line_drops_the_question() {
        let d = error_dialog(9, 169, 0, None, b"C:\\x\\j1.exe");
        assert_eq!(
            report_line(&d.title, &d.body),
            b"Unhandled Error #9 - j1.exe: Line: 169 (in main module): Subscript out of range".to_vec()
        );
    }

    #[test]
    fn unhandled_error_without_dialog_ends_with_exit_code() {
        let _g = program::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        host::NO_DIALOG.with(|n| *n.borrow_mut() = true);
        host::REPORTS.with(|r| r.borrow_mut().clear());
        unsafe {
            new_error = 53;
            ercl = 12;
            program::exit_code = 0;
        }
        assert_eq!(fix_error(b"prog"), FixError::Continue);
        host::BOXES.with(|b| assert!(b.borrow().is_empty()));
        assert!(host::ENDED.with(|e| *e.borrow()));
        unsafe {
            assert_eq!((new_error, program::close_program, program::exit_code), (0, 1, 1));
        }
        host::REPORTS.with(|r| {
            assert_eq!(r.borrow()[0], b"Unhandled Error #53 - prog: Line: 12 (in main module): File not found".to_vec())
        });
        // A critical error reports and exits with code 1.
        let r = std::panic::catch_unwind(|| raise(257));
        assert!(r.is_err());
        host::REPORTS.with(|r| assert_eq!(r.borrow()[1], b"Critical Error #1: Out of memory".to_vec()));
        host::NO_DIALOG.with(|n| *n.borrow_mut() = false);
        unsafe { program::exit_code = 0 };
        reset();
    }

    #[test]
    fn division_by_zero_is_trappable() {
        let _g = program::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        raise(11);
        assert_eq!(unsafe { new_error }, 11);
        assert_eq!(unsafe { program::qbevent }, 1);
        host::BOXES.with(|b| assert!(b.borrow().is_empty()));
        reset();
    }
}