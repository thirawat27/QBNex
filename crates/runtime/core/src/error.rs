//! Runtime error texts and the error/END policies of a running program.
//!
//! The message texts (`_ERRORMESSAGE$`, the "Unhandled Error" dialog) match
//! QB64/QBasic. Whether an unhandled error shows a dialog or one stderr line,
//! and whether END waits for a key, depends on how the program was started
//! (see [`no_dialog_policy`] and [`end_prompt_policy`]).

/// Message for an error number (`_ERRORMESSAGE$`).
pub fn human_error(code: i32) -> &'static str {
    match code {
        0 => "No error",
        1 => "NEXT without FOR",
        2 => "Syntax error",
        3 => "RETURN without GOSUB",
        4 => "Out of DATA",
        5 => "Illegal function call",
        6 => "Overflow",
        7 => "Out of memory",
        8 => "Label not defined",
        9 => "Subscript out of range",
        10 => "Duplicate definition",
        11 => "Division by zero", // QB64: "Unprintable error"
        12 => "Illegal in direct mode",
        13 => "Type mismatch",
        14 => "Out of string space",
        16 => "String formula too complex",
        17 => "Cannot continue",
        18 => "Function not defined",
        19 => "No RESUME",
        20 => "RESUME without error",
        24 => "Device timeout",
        25 => "Device fault",
        26 => "FOR without NEXT",
        27 => "Out of paper",
        29 => "WHILE without WEND",
        30 => "WEND without WHILE",
        33 => "Duplicate label",
        35 => "Subprogram not defined",
        37 => "Argument-count mismatch",
        38 => "Array not defined",
        40 => "Variable required",
        50 => "FIELD overflow",
        51 => "Internal error",
        52 => "Bad file name or number",
        53 => "File not found",
        54 => "Bad file mode",
        55 => "File already open",
        56 => "FIELD statement active",
        57 => "Device I/O error",
        58 => "File already exists",
        59 => "Bad record length",
        61 => "Disk full",
        62 => "Input past end of file",
        63 => "Bad record number",
        64 => "Bad file name",
        67 => "Too many files",
        68 => "Device unavailable",
        69 => "Communication-buffer overflow",
        70 => "Permission denied",
        71 => "Disk not ready",
        72 => "Disk-media error",
        73 => "Feature unavailable",
        74 => "Rename across disks",
        75 => "Path/File access error",
        76 => "Path not found",
        258 => "Invalid handle",
        300 => "Memory region out of range",
        301 => "Invalid size",
        302 => "Source memory region out of range",
        303 => "Destination memory region out of range",
        304 => "Source and destination memory regions out of range",
        305 => "Source memory has been freed",
        306 => "Destination memory has been freed",
        307 => "Memory already freed",
        308 => "Memory has been freed",
        309 => "Memory not initialized",
        310 => "Source memory not initialized",
        311 => "Destination memory not initialized",
        312 => "Source and destination memory not initialized",
        313 => "Source and destination memory have been freed",
        314 => "_ASSERT failed",
        315 => "_ASSERT failed (check console for description)",
        _ => "Unprintable error",
    }
}

/// The program file name shown in the dialog title: `COMMAND$(0)` without
/// its folder (text up to the first NUL).
pub fn binary_name(command0: &[u8]) -> &[u8] {
    let s = c_text(command0);
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

/// Builds the dialog shown for an error that no `ON ERROR` handler takes.
/// `include_file` is `None` when the include file name is unknown.
pub fn error_dialog(code: u32, ercl: u32, inclercl: u32, include_file: Option<&[u8]>, command0: &[u8]) -> ErrorDialog {
    let critical = (300..=315).contains(&code);
    let mut body = Vec::new();
    let line = if inclercl != 0 { inclercl } else { ercl };
    body.extend_from_slice(format!("Line: {line} (in ").as_bytes());
    if inclercl != 0 {
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

/// Text up to the first NUL.
fn c_text(s: &[u8]) -> &[u8] {
    match s.iter().position(|&c| c == 0) {
        Some(p) => &s[..p],
        None => s,
    }
}

/// How a standard stream of the process is connected.
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

/// [`end_prompt_policy`] of a console program for this process: true = END
/// shows the prompt and waits for a key.
pub fn end_prompt_wanted() -> bool {
    let env = std::env::var("QBNEX_NO_ERROR_DIALOG").ok();
    end_prompt_policy(env.as_deref(), false, stream_state(0), stream_state(1))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages() {
        assert_eq!(human_error(0), "No error");
        assert_eq!(human_error(6), "Overflow");
        assert_eq!(human_error(15), "Unprintable error");
        assert_eq!(human_error(11), "Division by zero");
        assert_eq!(human_error(76), "Path not found");
        assert_eq!(human_error(315), "_ASSERT failed (check console for description)");
        assert_eq!(human_error(-1), "Unprintable error");
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
}
