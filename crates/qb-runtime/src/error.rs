//! Runtime error machinery (R12b).
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
#[no_mangle]
pub static mut new_error: u32 = 0;
/// Backup of [`new_error`] used by `$DEBUG` (vWATCH) code.
#[no_mangle]
pub static mut bkp_new_error: u32 = 0;
/// `ERR`.
#[no_mangle]
pub static mut error_err: u32 = 0;
/// `ERL`.
#[no_mangle]
pub static mut error_erl: f64 = 0.0;
/// Set when the error handler is entered; `RESUME` checks it.
#[no_mangle]
pub static mut error_occurred: u32 = 0;
/// Label number of the active `ON ERROR GOTO` (0 = none).
#[no_mangle]
pub static mut error_goto_line: u32 = 0;
/// 1 while an `ON ERROR` handler runs.
#[no_mangle]
pub static mut error_handling: u32 = 0;
/// Set by `RESUME` (retry the failing line).
#[no_mangle]
pub static mut error_retry: u32 = 0;
/// `_ERRORLINE`: source line of the last error.
#[no_mangle]
pub static mut ercl: u32 = 0;
/// `_INCLERRORLINE`: line inside an `$INCLUDE` file (0 = main module).
#[no_mangle]
pub static mut inclercl: u32 = 0;
/// `_INCLERRORFILE$` (NUL-terminated, owned by generated code).
#[no_mangle]
pub static mut includedfilename: *mut c_char = core::ptr::null_mut();
/// Last numbered line passed (`ERL`).
#[no_mangle]
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
        11 => ("Division by zero\0", CRIT),
        256 => ("Out of stack space\0", CRIT),
        259 => ("Cannot find dynamic library file\0", CRIT),
        260 | 261 => ("Sub/Function does not exist in dynamic library\0", CRIT),
        270 => ("_GL command called outside of SUB _GL's scope\0", CRIT),
        271 => ("END/SYSTEM called within SUB _GL's scope\0", CRIT),
        _ => return None,
    })
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
    extern "C" {
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
}


/// `error(n)`: critical errors show a message box and end the process;
/// otherwise the first error of the statement is latched into `new_error`
/// and the program thread is told to check events.
pub fn raise(code: i32) {
    if let Some((msg, title)) = critical_error(code) {
        host::message_box(msg.trim_end_matches('\0').as_bytes(), title.trim_end_matches('\0').as_bytes(), MB_OK | MB_SYSTEMMODAL);
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

/// `error(n)`.
#[no_mangle]
pub extern "C" fn qb_error(code: i32) {
    raise(code);
}

/// `fix_error`: returns 1 when C++ must enter `QBMAIN` for the `ON ERROR`
/// handler, 0 otherwise. `command0` is `COMMAND$(0)`.
///
/// # Safety
/// `command0` readable for `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn qb_fix_error(command0: *const u8, len: i32) -> i32 {
    let c = unsafe { crate::ffi_bytes(command0, len.max(0) as usize) };
    match fix_error(c) {
        FixError::Handle => 1,
        FixError::Continue => 0,
    }
}

/// `human_error`: NUL-terminated static message text.
#[no_mangle]
pub extern "C" fn qb_human_error(code: i32) -> *const c_char {
    human_error_z(code).as_ptr().cast()
}

// ------------------------------------------- test hooks (tests/diff only)

/// `critical_error`: 1 and the NUL-terminated texts for a critical error.
///
/// # Safety
/// `msg` and `title` valid.
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn qb_test_critical_error(code: i32, msg: *mut *const c_char, title: *mut *const c_char) -> i32 {
    match critical_error(code) {
        Some((m, t)) => {
            unsafe {
                *msg = m.as_ptr().cast();
                *title = t.as_ptr().cast();
            }
            1
        }
        None => 0,
    }
}

/// [`error_dialog`] into caller buffers (NUL-terminated, truncated to `cap`).
/// Returns 1 for a critical (OK-only) dialog.
///
/// # Safety
/// `file` null or NUL-terminated; `command0` readable for `len`; `title`
/// and `body` writable for `cap` bytes.
#[doc(hidden)]
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn qb_test_error_dialog(
    code: u32,
    ercl_: u32,
    inclercl_: u32,
    file: *const c_char,
    command0: *const u8,
    len: i32,
    title: *mut u8,
    body: *mut u8,
    cap: i32,
) -> i32 {
    let f = if file.is_null() { None } else { Some(unsafe { std::ffi::CStr::from_ptr(file) }.to_bytes()) };
    let c = unsafe { crate::ffi_bytes(command0, len.max(0) as usize) };
    let d = error_dialog(code, ercl_, inclercl_, f, c);
    for (src, dst) in [(&d.title, title), (&d.body, body)] {
        let n = src.len().min(cap.max(1) as usize - 1);
        unsafe {
            core::ptr::copy_nonoverlapping(src.as_ptr(), dst, n);
            *dst.add(n) = 0;
        }
    }
    i32::from(d.critical)
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
        assert_eq!(human_error(315), "_ASSERT failed (check console for description)");
        assert_eq!(human_error(-1), "Unprintable error");
        let p = qb_human_error(76);
        assert_eq!(unsafe { std::ffi::CStr::from_ptr(p) }.to_bytes(), b"Path not found");
    }

    #[test]
    fn critical_table() {
        assert_eq!(critical_error(257).map(|c| c.1), Some("Critical Error #1\0"));
        assert_eq!(critical_error(518).map(|c| c.1), Some("Critical Error #18\0"));
        assert_eq!(critical_error(11).map(|c| c.0), Some("Division by zero\0"));
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
}
