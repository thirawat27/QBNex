//! SHELL family: `SHELL`, `SHELL(...)`, `_SHELLHIDE(...)`, `SHELL _HIDE`,
//! `SHELL _DONTWAIT`, `SHELL _DONTWAIT _HIDE`.
//!
//! Replaces the launching logic of `func_shell`, `func__shellhide`,
//! `sub_shell`, `sub_shell2`, `sub_shell3`, `sub_shell4`, `cmd_ok` and
//! `cmd_command` of libqb.cpp. The six legacy functions ran the same steps
//! and only differed in window visibility and waiting, so they become one
//! [`run`]. Leaving full screen, the `console`/`console_active` checks and
//! `shell_call_in_progress` stay in the C++ shims.
//!
//! Windows steps (unchanged): with an active console, `system(cmd)`.
//! Otherwise, if `cmd.exe` works: ShellExecuteEx the whole text (unless it
//! starts with a cmd built-in); then split at the first unquoted space and
//! ShellExecuteEx file + parameters; then `cmd` with `" /c " + text`.
//! Without `cmd.exe`: CreateProcess `command.com /c text`.

/// How to launch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mode {
    /// Window shown (`SW_SHOW`) or hidden (`SW_HIDE`).
    pub show: bool,
    /// Wait for the process and return its exit code.
    pub wait: bool,
    /// The program's console is active: run through the C `system()`.
    pub use_console: bool,
}

/// cmd.exe built-ins that cannot be started with ShellExecuteEx.
const CMD_BUILTINS: [&str; 46] = [
    "ASSOC", "BREAK", "BCDBOOT", "BCDEDIT", "CALL", "CD", "CHDIR", "CLS", "COLOR", "COPY", "DATE", "DEFRAG", "DEL",
    "DIR", "ECHO", "ENDLOCAL", "ERASE", "FOR", "FTYPE", "GOTO", "GRAFTABL", "IF", "MD", "MKDIR", "MKLINK", "MOVE",
    "PATH", "PAUSE", "POPD", "PROMPT", "PUSHD", "RD", "REM", "REN", "RENAME", "RMDIR", "SET", "SETLOCAL", "SHIFT",
    "START", "TIME", "TITLE", "TYPE", "VER", "VERIFY", "VOL",
];

/// `cmd_command`: is the whole (upper-cased) text a cmd built-in name?
pub fn cmd_command(s: &[u8]) -> bool {
    let u = s.to_ascii_uppercase();
    CMD_BUILTINS.iter().any(|b| b.as_bytes() == u.as_slice())
}

/// Splits at the first space outside double quotes: (file, parameters).
pub fn split(s: &[u8]) -> (&[u8], &[u8]) {
    let mut quotes = false;
    for (x, &c) in s.iter().enumerate() {
        if c == b'"' {
            quotes = !quotes;
        }
        if c == b' ' && !quotes {
            return (&s[..x], &s[x + 1..]);
        }
    }
    (s, &[])
}

unsafe extern "C" {
    fn system(cmd: *const std::ffi::c_char) -> i32;
}

/// C `system()` with a NUL-terminated copy of `cmd`.
fn c_system(cmd: &[u8]) -> i32 {
    let mut z = cmd.to_vec();
    z.push(0);
    // SAFETY: NUL-terminated command line.
    unsafe { system(z.as_ptr().cast()) }
}

/// Runs `cmd` (non-empty). Returns the exit code when waiting; 1 when every
/// launch attempt failed; 0 for started-but-not-waited processes.
#[cfg(windows)]
pub fn run(cmd: &[u8], mode: Mode) -> i64 {
    use crate::win32::*;
    use std::ptr::{null, null_mut};

    if mode.use_console {
        return i64::from(c_system(cmd));
    }
    const SEE_MASK_NOCLOSEPROCESS: u32 = 0x40;
    const SEE_MASK_FLAG_NO_UI: u32 = 0x400;
    let show = if mode.show { 5 } else { 0 }; // SW_SHOW / SW_HIDE
    let exec = |file: &[u8], params: Option<&[u8]>| -> Option<i64> {
        let f = cstr(file);
        let p = params.map(cstr);
        // SAFETY: zeroed POD, NUL-terminated strings that outlive the call.
        unsafe {
            let mut shi: SHELLEXECUTEINFOA = std::mem::zeroed();
            shi.cbSize = std::mem::size_of::<SHELLEXECUTEINFOA>() as u32;
            shi.lpFile = f.as_ptr();
            shi.lpParameters = p.as_ref().map_or(null(), |p| p.as_ptr());
            shi.fMask = if mode.wait { SEE_MASK_NOCLOSEPROCESS | SEE_MASK_FLAG_NO_UI } else { SEE_MASK_FLAG_NO_UI };
            shi.nShow = show;
            if ShellExecuteExA(&mut shi) == 0 {
                return None;
            }
            if !mode.wait {
                return Some(0);
            }
            WaitForSingleObject(shi.hProcess, INFINITE);
            let mut code: DWORD = 0;
            GetExitCodeProcess(shi.hProcess, &mut code);
            CloseHandle(shi.hProcess);
            Some(i64::from(code))
        }
    };

    if cmd_ok() {
        if !cmd_command(cmd)
            && let Some(rc) = exec(cmd, None) {
                return rc;
            }
        let (file, params) = split(cmd);
        if file.is_empty() {
            return 1;
        }
        if !params.is_empty() && !cmd_command(file)
            && let Some(rc) = exec(file, Some(params)) {
                return rc;
            }
        let mut line = b" /c ".to_vec();
        line.extend_from_slice(file);
        if !params.is_empty() {
            line.push(b' ');
            line.extend_from_slice(params);
        }
        return exec(b"cmd", Some(&line)).unwrap_or(1);
    }

    let mut line = cstr(&[b"command.com /c ".as_slice(), cmd].concat());
    // SAFETY: zeroed POD structs, writable command line buffer.
    unsafe {
        let mut si: STARTUPINFOA = std::mem::zeroed();
        si.cb = std::mem::size_of::<STARTUPINFOA>() as u32;
        let mut pi: PROCESS_INFORMATION = std::mem::zeroed();
        const CREATE_NEW_CONSOLE: u32 = 0x10;
        if CreateProcessA(null(), line.as_mut_ptr(), null_mut(), null_mut(), 0, CREATE_NEW_CONSOLE, null_mut(), null(), &mut si, &mut pi) != 0 {
            if mode.wait {
                WaitForSingleObject(pi.hProcess, INFINITE);
            }
            CloseHandle(pi.hProcess);
            CloseHandle(pi.hThread);
        }
    }
    0
}

/// Unix: `system()` (waiting: exit status) or a detached `sh -c`.
#[cfg(not(windows))]
pub fn run(cmd: &[u8], mode: Mode) -> i64 {
    if mode.wait {
        let r = c_system(cmd);
        if r == -1 {
            return -1;
        }
        return i64::from((r >> 8) & 0xFF); // WEXITSTATUS
    }
    let _ = std::process::Command::new("sh").arg("-c").arg(String::from_utf8_lossy(cmd).as_ref()).spawn();
    0
}

/// `cmd_ok`: can `cmd.exe /c ver` be started? Checked once.
#[cfg(windows)]
pub fn cmd_ok() -> bool {
    use std::sync::OnceLock;
    static OK: OnceLock<bool> = OnceLock::new();
    *OK.get_or_init(|| {
        use crate::win32::*;
        use std::ptr::{null, null_mut};
        let mut line = cstr(b"cmd.exe /c ver");
        // SAFETY: zeroed POD structs, writable command line buffer.
        unsafe {
            let mut si: STARTUPINFOA = std::mem::zeroed();
            si.cb = std::mem::size_of::<STARTUPINFOA>() as u32;
            let mut pi: PROCESS_INFORMATION = std::mem::zeroed();
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            if CreateProcessA(null(), line.as_mut_ptr(), null_mut(), null_mut(), 0, CREATE_NO_WINDOW, null_mut(), null(), &mut si, &mut pi) == 0 {
                return false;
            }
            WaitForSingleObject(pi.hProcess, INFINITE);
            CloseHandle(pi.hProcess);
            CloseHandle(pi.hThread);
            true
        }
    })
}

/// `SHELL` without a command: an interactive `cmd` in the program's console
/// (allocated for the call when the program has none). Windows only.
pub fn interactive(use_console: bool) -> i64 {
    #[cfg(windows)]
    {
        unsafe extern "system" {
            fn AllocConsole() -> i32;
            fn FreeConsole() -> i32;
        }
        // SAFETY: plain Win32 console calls.
        unsafe {
            if !use_console {
                AllocConsole();
            }
            let rc = c_system(b"cmd");
            if !use_console {
                FreeConsole();
            }
            i64::from(rc)
        }
    }
    #[cfg(not(windows))]
    {
        let _ = use_console;
        0
    }
}

// --- C ABI (include/qb_runtime.h) ---------------------------------------------------

/// Runs a non-empty command; see [`run`].
///
/// # Safety
/// `cmd` readable for `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_shell_run(cmd: *const u8, len: i32, show: i32, wait: i32, use_console: i32) -> i64 {
    // SAFETY: caller contract.
    let cmd = unsafe { crate::ffi_bytes(cmd, len.max(0) as usize) };
    run(cmd, Mode { show: show != 0, wait: wait != 0, use_console: use_console != 0 })
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_shell_interactive(use_console: i32) -> i64 {
    interactive(use_console != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_and_split() {
        assert!(cmd_command(b"dir"));
        assert!(cmd_command(b"Echo"));
        assert!(!cmd_command(b"echo hi"));
        assert!(!cmd_command(b"notepad"));
        assert_eq!(split(b"echo hi there"), (&b"echo"[..], &b"hi there"[..]));
        assert_eq!(split(b"\"a b\" c"), (&b"\"a b\""[..], &b"c"[..]));
        assert_eq!(split(b" lead"), (&b""[..], &b"lead"[..]));
        assert_eq!(split(b"single"), (&b"single"[..], &b""[..]));
    }

    #[test]
    fn exit_codes() {
        let wait_hidden = Mode { show: false, wait: true, use_console: false };
        assert_eq!(run(b"exit 3", wait_hidden), 3);
        let nested: &[u8] = if cfg!(windows) { b"cmd /c exit 7" } else { b"sh -c 'exit 7'" };
        assert_eq!(run(nested, wait_hidden), 7);
        let console = Mode { show: true, wait: true, use_console: true };
        assert_eq!(run(b"exit 5", console), 5);
        assert_eq!(run(b"exit 0", Mode { show: false, wait: false, use_console: false }), 0);
    }
}
