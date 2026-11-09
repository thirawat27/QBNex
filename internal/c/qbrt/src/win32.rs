//! Minimal Win32 declarations used by the file, file-system and SHELL layers.
//!
//! The legacy runtime was compiled without `UNICODE`, so every call goes to the
//! ANSI (`...A`) entry point, exactly as libqb.cpp did. Declared by hand to keep
//! the crate free of dependencies.
#![allow(non_snake_case, non_camel_case_types, clippy::upper_case_acronyms)]

use std::ffi::c_void;

pub type HANDLE = *mut c_void;
pub type BOOL = i32;
pub type DWORD = u32;

pub const INVALID_HANDLE_VALUE: HANDLE = usize::MAX as HANDLE;
pub const INVALID_FILE_ATTRIBUTES: DWORD = 0xFFFF_FFFF;
pub const FILE_ATTRIBUTE_DIRECTORY: DWORD = 0x10;
pub const FILE_ATTRIBUTE_NORMAL: DWORD = 0x80;
pub const GENERIC_READ: DWORD = 0x8000_0000;
pub const GENERIC_WRITE: DWORD = 0x4000_0000;
pub const FILE_SHARE_READ: DWORD = 1;
pub const FILE_SHARE_WRITE: DWORD = 2;
pub const OPEN_EXISTING: DWORD = 3;
pub const OPEN_ALWAYS: DWORD = 4;
pub const TRUNCATE_EXISTING: DWORD = 5;
pub const FILE_BEGIN: DWORD = 0;
pub const NO_ERROR: DWORD = 0;
pub const INFINITE: DWORD = 0xFFFF_FFFF;
pub const MAXDWORD: DWORD = 0xFFFF_FFFF;
pub const MAX_PATH: usize = 260;

/// DCB with its bit-field word kept as a plain `u32` (fBinary = bit 0,
/// fParity = bit 1, fRtsControl = bits 12-13).
#[repr(C)]
#[derive(Default)]
pub struct DCB {
    pub DCBlength: DWORD,
    pub BaudRate: DWORD,
    pub bits: DWORD,
    pub wReserved: u16,
    pub XonLim: u16,
    pub XoffLim: u16,
    pub ByteSize: u8,
    pub Parity: u8,
    pub StopBits: u8,
    pub XonChar: i8,
    pub XoffChar: i8,
    pub ErrorChar: i8,
    pub EofChar: i8,
    pub EvtChar: i8,
    pub wReserved1: u16,
}

#[repr(C)]
#[derive(Default)]
pub struct COMMTIMEOUTS {
    pub ReadIntervalTimeout: DWORD,
    pub ReadTotalTimeoutMultiplier: DWORD,
    pub ReadTotalTimeoutConstant: DWORD,
    pub WriteTotalTimeoutMultiplier: DWORD,
    pub WriteTotalTimeoutConstant: DWORD,
}

#[repr(C)]
#[derive(Default)]
pub struct COMSTAT {
    pub bits: DWORD,
    pub cbInQue: DWORD,
    pub cbOutQue: DWORD,
}

#[repr(C)]
pub struct FILETIME {
    pub lo: DWORD,
    pub hi: DWORD,
}

#[repr(C)]
pub struct WIN32_FIND_DATAA {
    pub dwFileAttributes: DWORD,
    pub ftCreationTime: FILETIME,
    pub ftLastAccessTime: FILETIME,
    pub ftLastWriteTime: FILETIME,
    pub nFileSizeHigh: DWORD,
    pub nFileSizeLow: DWORD,
    pub dwReserved0: DWORD,
    pub dwReserved1: DWORD,
    pub cFileName: [u8; MAX_PATH],
    pub cAlternateFileName: [u8; 14],
}

impl WIN32_FIND_DATAA {
    pub fn zeroed() -> Self {
        // SAFETY: plain-old-data struct; all-zero is a valid value.
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
pub struct STARTUPINFOA {
    pub cb: DWORD,
    pub lpReserved: *mut u8,
    pub lpDesktop: *mut u8,
    pub lpTitle: *mut u8,
    pub dwX: DWORD,
    pub dwY: DWORD,
    pub dwXSize: DWORD,
    pub dwYSize: DWORD,
    pub dwXCountChars: DWORD,
    pub dwYCountChars: DWORD,
    pub dwFillAttribute: DWORD,
    pub dwFlags: DWORD,
    pub wShowWindow: u16,
    pub cbReserved2: u16,
    pub lpReserved2: *mut u8,
    pub hStdInput: HANDLE,
    pub hStdOutput: HANDLE,
    pub hStdError: HANDLE,
}

#[repr(C)]
pub struct PROCESS_INFORMATION {
    pub hProcess: HANDLE,
    pub hThread: HANDLE,
    pub dwProcessId: DWORD,
    pub dwThreadId: DWORD,
}

#[repr(C)]
pub struct SHELLEXECUTEINFOA {
    pub cbSize: DWORD,
    pub fMask: u32,
    pub hwnd: HANDLE,
    pub lpVerb: *const u8,
    pub lpFile: *const u8,
    pub lpParameters: *const u8,
    pub lpDirectory: *const u8,
    pub nShow: i32,
    pub hInstApp: HANDLE,
    pub lpIDList: *mut c_void,
    pub lpClass: *const u8,
    pub hkeyClass: HANDLE,
    pub dwHotKey: DWORD,
    pub hIconOrMonitor: HANDLE,
    pub hProcess: HANDLE,
}

#[link(name = "kernel32")]
extern "system" {
    pub fn CreateFileA(
        name: *const u8,
        access: DWORD,
        share: DWORD,
        sa: *mut c_void,
        disposition: DWORD,
        flags: DWORD,
        template: HANDLE,
    ) -> HANDLE;
    pub fn CloseHandle(h: HANDLE) -> BOOL;
    pub fn ReadFile(h: HANDLE, buf: *mut u8, n: DWORD, read: *mut DWORD, ov: *mut c_void) -> BOOL;
    pub fn WriteFile(h: HANDLE, buf: *const u8, n: DWORD, written: *mut DWORD, ov: *mut c_void) -> BOOL;
    pub fn SetFilePointer(h: HANDLE, lo: i32, hi: *mut i32, method: DWORD) -> DWORD;
    pub fn GetFileSize(h: HANDLE, hi: *mut DWORD) -> DWORD;
    pub fn GetLastError() -> DWORD;
    pub fn LockFile(h: HANDLE, off_lo: DWORD, off_hi: DWORD, n_lo: DWORD, n_hi: DWORD) -> BOOL;
    pub fn UnlockFile(h: HANDLE, off_lo: DWORD, off_hi: DWORD, n_lo: DWORD, n_hi: DWORD) -> BOOL;
    pub fn GetCommState(h: HANDLE, dcb: *mut DCB) -> BOOL;
    pub fn SetCommState(h: HANDLE, dcb: *mut DCB) -> BOOL;
    pub fn SetCommTimeouts(h: HANDLE, t: *mut COMMTIMEOUTS) -> BOOL;
    pub fn ClearCommError(h: HANDLE, errors: *mut DWORD, stat: *mut COMSTAT) -> BOOL;
    pub fn FindFirstFileA(name: *const u8, data: *mut WIN32_FIND_DATAA) -> HANDLE;
    pub fn FindNextFileA(h: HANDLE, data: *mut WIN32_FIND_DATAA) -> BOOL;
    pub fn FindClose(h: HANDLE) -> BOOL;
    pub fn DeleteFileA(name: *const u8) -> BOOL;
    pub fn GetFileAttributesA(name: *const u8) -> DWORD;
    pub fn GetCurrentDirectoryA(n: DWORD, buf: *mut u8) -> DWORD;
    pub fn GetShortPathNameA(long: *const u8, short: *mut u8, n: DWORD) -> DWORD;
    pub fn GetDiskFreeSpaceExA(dir: *const u8, avail: *mut u64, total: *mut u64, free: *mut u64) -> BOOL;
    pub fn CreateProcessA(
        app: *const u8,
        cmd: *mut u8,
        pa: *mut c_void,
        ta: *mut c_void,
        inherit: BOOL,
        flags: DWORD,
        env: *mut c_void,
        dir: *const u8,
        si: *mut STARTUPINFOA,
        pi: *mut PROCESS_INFORMATION,
    ) -> BOOL;
    pub fn WaitForSingleObject(h: HANDLE, ms: DWORD) -> DWORD;
    pub fn GetExitCodeProcess(h: HANDLE, code: *mut DWORD) -> BOOL;
}

#[link(name = "shell32")]
extern "system" {
    pub fn ShellExecuteExA(info: *mut SHELLEXECUTEINFOA) -> BOOL;
}

/// Copies `bytes` and appends a NUL terminator.
pub fn cstr(bytes: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(bytes.len() + 1);
    v.extend_from_slice(bytes);
    v.push(0);
    v
}
