//! ENVIRON$, ENVIRON, _ENVIRONCOUNT and COMMAND$.
//!
//! Replaces `func_environ`, `sub_environ`, `func__environcount` and
//! `func_command` of libqb.cpp. The C runtime environment is used, as
//! before: `getenv` / `_putenv_s` (Windows) or `setenv` / `unsetenv`, and the
//! live `environ` table, which the C++ shim passes in (`envp`).

use std::ffi::{c_char, CStr};

unsafe extern "C" {
    fn getenv(name: *const c_char) -> *const c_char;
}
#[cfg(windows)]
unsafe extern "C" {
    fn _putenv(s: *const c_char) -> i32;
    fn _putenv_s(name: *const c_char, value: *const c_char) -> i32;
}
#[cfg(not(windows))]
extern "C" {
    fn setenv(name: *const c_char, value: *const c_char, overwrite: i32) -> i32;
    fn unsetenv(name: *const c_char) -> i32;
}

/// `ENVIRON$(name$)`; the name ends at its first NUL byte (legacy used
/// `c_str()`).
pub fn get(name: &[u8]) -> Vec<u8> {
    let mut q: Vec<u8> = name.iter().copied().take_while(|&c| c != 0).collect();
    q.push(0);
    // SAFETY: NUL-terminated query; the result is copied before any change.
    unsafe {
        let r = getenv(q.as_ptr().cast());
        if r.is_null() {
            Vec::new()
        } else {
            CStr::from_ptr(r).to_bytes().to_vec()
        }
    }
}

/// Number of entries in `envp` as legacy counted them (it starts looking for
/// the terminator at index 1).
///
/// # Safety
/// `envp` must be a NULL-terminated array with at least one entry.
pub unsafe fn count(envp: *const *const c_char) -> i32 {
    let mut p = envp;
    // SAFETY: caller contract.
    unsafe {
        loop {
            p = p.add(1);
            if (*p).is_null() {
                break;
            }
        }
        p.offset_from(envp) as i32
    }
}

/// `ENVIRON$(n)`: (error, entry). n <= 0 is error 5; n past the end is "".
///
/// # Safety
/// As [`count`].
pub unsafe fn by_index(envp: *const *const c_char, number: i32) -> (i32, Vec<u8>) {
    if number <= 0 {
        return (5, Vec::new());
    }
    // SAFETY: caller contract.
    unsafe {
        if number > count(envp) {
            return (0, Vec::new());
        }
        (0, CStr::from_ptr(*envp.add(number as usize - 1)).to_bytes().to_vec())
    }
}

/// `ENVIRON "name=value"` / `ENVIRON "name value"`: a space wins over '=';
/// a separator at the end removes the variable. Error 5 without separator.
pub fn set(s: &[u8]) -> i32 {
    let mut buf = s.to_vec();
    buf.push(0);
    let sep = buf[..buf.len() - 1]
        .iter()
        .position(|&c| c == b' ')
        .or_else(|| buf[..buf.len() - 1].iter().position(|&c| c == b'='));
    // legacy strchr stops at the first NUL of the string
    let nul = buf.iter().position(|&c| c == 0).expect("terminated");
    let Some(sep) = sep.filter(|&k| k < nul) else { return 5 };
    buf[sep] = 0;
    // SAFETY: NUL-terminated pieces of `buf`.
    unsafe {
        if sep == s.len() - 1 {
            #[cfg(windows)]
            {
                buf[sep] = b'=';
                _putenv(buf.as_ptr().cast());
            }
            #[cfg(not(windows))]
            unsetenv(buf.as_ptr().cast());
        } else {
            #[cfg(windows)]
            _putenv_s(buf.as_ptr().cast(), buf[sep + 1..].as_ptr().cast());
            #[cfg(not(windows))]
            setenv(buf.as_ptr().cast(), buf[sep + 1..].as_ptr().cast(), 1);
        }
    }
    0
}

/// `COMMAND$(index)`: argument `index` of `array` (length `count`), "" when
/// out of range.
///
/// # Safety
/// `array` null or valid for `count` NUL-terminated strings.
pub unsafe fn command_arg(array: *const *const c_char, count: i32, index: i32) -> Vec<u8> {
    if index >= count || index < 0 || array.is_null() {
        return Vec::new();
    }
    // SAFETY: caller contract.
    unsafe { CStr::from_ptr(*array.add(index as usize)).to_bytes().to_vec() }
}

// --- C ABI (include/qb_runtime.h) ---------------------------------------------------

static OUT: std::sync::Mutex<Vec<u8>> = std::sync::Mutex::new(Vec::new());

/// # Safety
/// Out pointers writable.
unsafe fn publish(v: Vec<u8>, out: *mut *const u8, out_len: *mut i32) {
    let mut o = OUT.lock().unwrap_or_else(|e| e.into_inner());
    *o = v;
    // SAFETY: caller contract.
    unsafe {
        *out = o.as_ptr();
        *out_len = o.len() as i32;
    }
}

/// # Safety
/// `name` readable for `len` bytes; out pointers writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_environ_get(name: *const u8, len: i32, out: *mut *const u8, out_len: *mut i32) {
    // SAFETY: caller contract.
    unsafe { publish(get(crate::ffi_bytes(name, len.max(0) as usize)), out, out_len) }
}

/// # Safety
/// See [`by_index`]; out pointers writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_environ_index(
    envp: *const *const c_char,
    number: i32,
    out: *mut *const u8,
    out_len: *mut i32,
) -> i32 {
    // SAFETY: caller contract.
    unsafe {
        let (e, v) = by_index(envp, number);
        publish(v, out, out_len);
        e
    }
}

/// # Safety
/// See [`count`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_environ_count(envp: *const *const c_char) -> i32 {
    // SAFETY: caller contract.
    unsafe { count(envp) }
}

/// # Safety
/// `s` readable for `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_environ_set(s: *const u8, len: i32) -> i32 {
    // SAFETY: caller contract.
    set(unsafe { crate::ffi_bytes(s, len.max(0) as usize) })
}

/// # Safety
/// See [`command_arg`]; out pointers writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_command_arg(
    array: *const *const c_char,
    count: i32,
    index: i32,
    out: *mut *const u8,
    out_len: *mut i32,
) {
    // SAFETY: caller contract.
    unsafe { publish(command_arg(array, count, index), out, out_len) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_remove() {
        assert_eq!(set(b"QB_T1=hello world"), 0);
        assert_eq!(set(b"QB_T2=a=b"), 0);
        assert_eq!(get(b"QB_T2"), b"a=b");
        assert_eq!(set(b"QB_T3 x"), 0);
        assert_eq!(get(b"QB_T3"), b"x");
        assert_eq!(get(b"QB_T3\0junk"), b"x");
        assert_eq!(set(b"QB_T3="), 0);
        assert_eq!(get(b"QB_T3"), b"");
        assert_eq!(set(b"QB_NOSEP"), 5);
    }

    #[test]
    fn index_count_and_command() {
        let a = c"A=1";
        let b = c"B=2";
        let envp = [a.as_ptr(), b.as_ptr(), std::ptr::null()];
        unsafe {
            assert_eq!(count(envp.as_ptr()), 2);
            assert_eq!(by_index(envp.as_ptr(), 2), (0, b"B=2".to_vec()));
            assert_eq!(by_index(envp.as_ptr(), 3), (0, Vec::new()));
            assert_eq!(by_index(envp.as_ptr(), 0), (5, Vec::new()));
            assert_eq!(command_arg(envp.as_ptr(), 2, 1), b"B=2");
            assert_eq!(command_arg(envp.as_ptr(), 2, 2), b"");
            assert_eq!(command_arg(std::ptr::null(), 2, 0), b"");
        }
    }
}
