//! `qb_ext_*` C ABI used by the BASIC wrappers (`compiler/stdlib/ext/*.bas`,
//! prototypes in `include/qb_ext.h`).
//!
//! Conventions:
//! - Byte strings come in as (pointer, length); BASIC passes `_OFFSET(s$)`
//!   and `LEN(s$)`. A null pointer with length 0 is an empty string.
//! - A string result is kept in a per-thread result buffer; the function
//!   returns its length (or -1 on error) and BASIC copies it out with
//!   [`qb_ext_result_copy`]. List results (split parts, regex groups, zip
//!   member names) go to a per-thread parts list read with [`qb_ext_part`].
//! - On error the message is available through [`qb_ext_error_len`] /
//!   [`qb_ext_error_copy`]; a successful call clears it.
//! - Text arguments that must be UTF-8 (URLs, header lines, file paths,
//!   zip member names) are decoded lossily.

use crate::{codec, hash, http, re, ziparc};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Default)]
struct State {
    result: Vec<u8>,
    error: Vec<u8>,
    parts: Vec<Vec<u8>>,
    sizes: Vec<i64>,
    http_headers: Vec<u8>,
    zips: HashMap<i32, ziparc::Builder>,
    next_zip: i32,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

/// # Safety
/// `p` readable for `len` bytes when `len > 0`.
unsafe fn bytes<'a>(p: *const u8, len: i64) -> &'a [u8] {
    if len <= 0 || p.is_null() {
        &[]
    } else {
        // SAFETY: caller contract.
        unsafe { std::slice::from_raw_parts(p, len as usize) }
    }
}

unsafe fn text(p: *const u8, len: i64) -> String {
    String::from_utf8_lossy(unsafe { bytes(p, len) }).into_owned()
}

fn ok_result(v: Vec<u8>) -> i64 {
    with(|s| {
        s.error.clear();
        s.result = v;
        s.result.len() as i64
    })
}

fn fail(msg: impl Into<String>) -> i64 {
    let m = msg.into();
    with(|s| s.error = m.into_bytes());
    -1
}

fn ok_parts(parts: Vec<Vec<u8>>) -> i32 {
    with(|s| {
        s.error.clear();
        let n = parts.len() as i32;
        s.parts = parts;
        n
    })
}

fn copy_out(src: &[u8], dst: *mut u8, cap: i64) -> i64 {
    let n = src.len().min(cap.max(0) as usize);
    if n > 0 && !dst.is_null() {
        // SAFETY: caller passes a buffer of at least `cap` bytes.
        unsafe { std::ptr::copy_nonoverlapping(src.as_ptr(), dst, n) };
    }
    n as i64
}

// ------------------------------------------------------------- buffers

/// Length of the current result.
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_result_len() -> i64 {
    with(|s| s.result.len() as i64)
}

/// Copies up to `cap` bytes of the result to `dst`; returns the count.
///
/// # Safety
/// `dst` writable for `cap` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_result_copy(dst: *mut u8, cap: i64) -> i64 {
    with(|s| copy_out(&s.result, dst, cap))
}

/// Length of the last error message (0 = the last call succeeded).
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_error_len() -> i64 {
    with(|s| s.error.len() as i64)
}

/// Copies the last error message.
///
/// # Safety
/// `dst` writable for `cap` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_error_copy(dst: *mut u8, cap: i64) -> i64 {
    with(|s| copy_out(&s.error, dst, cap))
}

/// Number of entries in the parts list.
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_part_count() -> i32 {
    with(|s| s.parts.len() as i32)
}

/// Makes part `i` (0-based) the result; returns its length or -1.
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_part(i: i32) -> i64 {
    let p = with(|s| usize::try_from(i).ok().and_then(|i| s.parts.get(i).cloned()));
    match p {
        Some(v) => with(|s| {
            s.result = v;
            s.result.len() as i64
        }),
        None => -1,
    }
}

// ------------------------------------------------------------- regex

/// 1 match, 0 no match, -1 bad pattern.
///
/// # Safety
/// Pointer/length pairs valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_regex_match(p: *const u8, pl: i64, t: *const u8, tl: i64) -> i32 {
    match re::is_match(unsafe { bytes(p, pl) }, unsafe { bytes(t, tl) }) {
        Ok(m) => {
            with(|s| s.error.clear());
            i32::from(m)
        }
        Err(e) => fail(e) as i32,
    }
}

/// First match at or after the 1-based `start`: returns its 1-based
/// position (0 = none, -1 = bad pattern); result = the match, parts = the
/// groups (group 0 first; a group that did not match is empty).
///
/// # Safety
/// Pointer/length pairs valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_regex_find(p: *const u8, pl: i64, t: *const u8, tl: i64, start: i64) -> i64 {
    match re::find(unsafe { bytes(p, pl) }, unsafe { bytes(t, tl) }, start.max(1) as usize) {
        Ok(Some(f)) => {
            let groups: Vec<Vec<u8>> = f.groups.into_iter().map(Option::unwrap_or_default).collect();
            let whole = groups.first().cloned().unwrap_or_default();
            ok_parts(groups);
            ok_result(whole);
            f.start as i64
        }
        Ok(None) => {
            ok_parts(Vec::new());
            ok_result(Vec::new());
            0
        }
        Err(e) => fail(e),
    }
}

/// Replaces the first (`all` = 0) or every match; returns the result length.
///
/// # Safety
/// Pointer/length pairs valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_regex_replace(
    p: *const u8,
    pl: i64,
    t: *const u8,
    tl: i64,
    r: *const u8,
    rl: i64,
    all: i32,
) -> i64 {
    match re::replace(unsafe { bytes(p, pl) }, unsafe { bytes(t, tl) }, unsafe { bytes(r, rl) }, all != 0) {
        Ok(v) => ok_result(v),
        Err(e) => fail(e),
    }
}

/// Splits at every match; returns the number of parts (-1 = bad pattern).
///
/// # Safety
/// Pointer/length pairs valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_regex_split(p: *const u8, pl: i64, t: *const u8, tl: i64) -> i32 {
    match re::split(unsafe { bytes(p, pl) }, unsafe { bytes(t, tl) }) {
        Ok(v) => ok_parts(v),
        Err(e) => fail(e) as i32,
    }
}

// ------------------------------------------------------------- hashing, codecs

/// Lower-case hex digest (`kind`: 1 CRC32, 2 MD5, 3 SHA-1, 4 SHA-256,
/// 5 SHA-512); returns its length, -1 for an unknown kind.
///
/// # Safety
/// Pointer/length pair valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_hash(kind: i32, d: *const u8, dl: i64) -> i64 {
    match hash::Algorithm::from_code(kind) {
        Some(a) => ok_result(hash::hex_digest(a, unsafe { bytes(d, dl) }).into_bytes()),
        None => fail(format!("unknown hash kind {kind}")),
    }
}

/// CRC32 as a number.
///
/// # Safety
/// Pointer/length pair valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_crc32(d: *const u8, dl: i64) -> u32 {
    hash::crc32(unsafe { bytes(d, dl) })
}

/// Encodings (`op`: 1 Base64 encode, 2 Base64 decode (standard or URL-safe),
/// 3 Base64url encode, 4 hex encode, 5 hex decode). Returns the result
/// length, -1 on invalid input.
///
/// # Safety
/// Pointer/length pair valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_codec(op: i32, d: *const u8, dl: i64) -> i64 {
    let data = unsafe { bytes(d, dl) };
    let r = match op {
        1 => Some(codec::base64_encode(data).into_bytes()),
        2 => codec::base64_decode(data),
        3 => Some(codec::base64url_encode(data).into_bytes()),
        4 => Some(codec::hex_encode(data).into_bytes()),
        5 => codec::hex_decode(data),
        _ => return fail(format!("unknown codec {op}")),
    };
    match r {
        Some(v) => ok_result(v),
        None => fail("invalid encoded text"),
    }
}

// ------------------------------------------------------------- zip

/// Lists an archive: returns the member count (-1 on error); parts = names.
///
/// # Safety
/// Pointer/length pair valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_zip_list(path: *const u8, pl: i64) -> i32 {
    match ziparc::list(&PathBuf::from(unsafe { text(path, pl) })) {
        Ok(entries) => {
            let sizes = entries.iter().map(|e| e.size as i64).collect();
            with(|s| s.sizes = sizes);
            ok_parts(entries.into_iter().map(|e| e.name.into_bytes()).collect())
        }
        Err(e) => fail(e) as i32,
    }
}

/// Uncompressed size of member `i` of the last listed archive (-1 if none).
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_zip_entry_size(i: i32) -> i64 {
    with(|s| usize::try_from(i).ok().and_then(|i| s.sizes.get(i).copied()).unwrap_or(-1))
}

/// Reads one member; returns its length (-1 on error).
///
/// # Safety
/// Pointer/length pairs valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_zip_read(path: *const u8, pl: i64, name: *const u8, nl: i64) -> i64 {
    match ziparc::read(&PathBuf::from(unsafe { text(path, pl) }), &unsafe { text(name, nl) }) {
        Ok(v) => ok_result(v),
        Err(e) => fail(e),
    }
}

/// Extracts every member below `dest`; returns the number of files (-1 on error).
///
/// # Safety
/// Pointer/length pairs valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_zip_extract(path: *const u8, pl: i64, dest: *const u8, dl: i64) -> i32 {
    match ziparc::extract(&PathBuf::from(unsafe { text(path, pl) }), &PathBuf::from(unsafe { text(dest, dl) })) {
        Ok(n) => {
            with(|s| s.error.clear());
            n as i32
        }
        Err(e) => fail(e) as i32,
    }
}

/// Starts a new archive; returns a handle (> 0) or -1.
///
/// # Safety
/// Pointer/length pair valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_zip_create(path: *const u8, pl: i64) -> i32 {
    match ziparc::Builder::create(&PathBuf::from(unsafe { text(path, pl) })) {
        Ok(b) => with(|s| {
            s.error.clear();
            s.next_zip += 1;
            let h = s.next_zip;
            s.zips.insert(h, b);
            h
        }),
        Err(e) => fail(e) as i32,
    }
}

/// Adds a member to archive `h`: `kind` 0 = `data` holds the bytes,
/// 1 = `data` is the path of a file to add, 2 = a folder (data ignored).
/// Returns 0 or -1.
///
/// # Safety
/// Pointer/length pairs valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_zip_add(h: i32, name: *const u8, nl: i64, data: *const u8, dl: i64, kind: i32) -> i32 {
    let name = unsafe { text(name, nl) };
    let data = unsafe { bytes(data, dl) };
    let r = with(|s| match s.zips.get_mut(&h) {
        None => Err(format!("no open zip handle {h}")),
        Some(b) => match kind {
            0 => b.add_data(&name, data),
            1 => b.add_file(&name, &PathBuf::from(String::from_utf8_lossy(data).into_owned())),
            2 => b.add_dir(&name),
            _ => Err(format!("unknown member kind {kind}")),
        },
    });
    match r {
        Ok(()) => {
            with(|s| s.error.clear());
            0
        }
        Err(e) => fail(e) as i32,
    }
}

/// Finishes archive `h`; returns 0 or -1.
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_zip_close(h: i32) -> i32 {
    match with(|s| s.zips.remove(&h)) {
        None => fail(format!("no open zip handle {h}")) as i32,
        Some(b) => match b.finish() {
            Ok(()) => {
                with(|s| s.error.clear());
                0
            }
            Err(e) => fail(e) as i32,
        },
    }
}

// ------------------------------------------------------------- http

/// Sends an HTTP(S) request; returns the status code (-1 on a transport
/// error). Result = response body; [`qb_ext_http_headers`] gives the headers.
///
/// # Safety
/// Pointer/length pairs valid.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn qb_ext_http(
    method: *const u8,
    ml: i64,
    url: *const u8,
    ul: i64,
    headers: *const u8,
    hl: i64,
    body: *const u8,
    bl: i64,
    timeout_ms: i32,
) -> i32 {
    let m = unsafe { text(method, ml) };
    let m = if m.trim().is_empty() { "GET".to_string() } else { m };
    match http::request(&m, &unsafe { text(url, ul) }, &unsafe { text(headers, hl) }, unsafe { bytes(body, bl) }, i64::from(timeout_ms)) {
        Ok(r) => {
            with(|s| s.http_headers = r.headers.into_bytes());
            ok_result(r.body);
            i32::from(r.status)
        }
        Err(e) => {
            with(|s| s.http_headers.clear());
            fail(e) as i32
        }
    }
}

/// Makes the headers of the last response the result; returns their length.
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_http_headers() -> i64 {
    with(|s| {
        s.result = s.http_headers.clone();
        s.result.len() as i64
    })
}

/// Test hook: starts the in-process HTTP test server for `count` requests
/// on 127.0.0.1 and returns its port (conformance tests, no internet).
#[doc(hidden)]
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_test_http_server(count: i32) -> i32 {
    let base = http::testserver::start(count.max(1) as usize);
    base.rsplit(':').next().and_then(|p| p.parse().ok()).unwrap_or(-1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result() -> Vec<u8> {
        let n = qb_ext_result_len();
        let mut v = vec![0u8; n as usize];
        unsafe { qb_ext_result_copy(v.as_mut_ptr(), n) };
        v
    }
    fn error() -> String {
        let n = qb_ext_error_len();
        let mut v = vec![0u8; n as usize];
        unsafe { qb_ext_error_copy(v.as_mut_ptr(), n) };
        String::from_utf8(v).unwrap()
    }
    fn part(i: i32) -> Vec<u8> {
        assert!(qb_ext_part(i) >= 0);
        result()
    }

    #[test]
    fn regex_through_the_c_abi() {
        let (p, t) = (br"(\d+)-(\d+)", b"tel 12-345 x");
        unsafe {
            assert_eq!(qb_ext_regex_match(p.as_ptr(), p.len() as i64, t.as_ptr(), t.len() as i64), 1);
            assert_eq!(qb_ext_regex_find(p.as_ptr(), p.len() as i64, t.as_ptr(), t.len() as i64, 1), 5);
        }
        assert_eq!(result(), b"12-345");
        assert_eq!(qb_ext_part_count(), 3);
        assert_eq!(part(2), b"345");
        assert_eq!(qb_ext_part(3), -1);
        let bad = b"(";
        unsafe { assert_eq!(qb_ext_regex_match(bad.as_ptr(), 1, t.as_ptr(), 0), -1) };
        assert!(error().contains("unclosed group"), "{}", error());
        let sp = b",";
        unsafe { assert_eq!(qb_ext_regex_split(sp.as_ptr(), 1, b"a,,b".as_ptr(), 4), 3) };
        assert_eq!(qb_ext_error_len(), 0);
        assert_eq!(part(1), b"");
        let rp = b"<$1>";
        unsafe { assert_eq!(qb_ext_regex_replace(p.as_ptr(), p.len() as i64, t.as_ptr(), t.len() as i64, rp.as_ptr(), 4, 1), 10) };
        assert_eq!(result(), b"tel <12> x");
    }

    #[test]
    fn hash_and_codec_through_the_c_abi() {
        unsafe {
            assert_eq!(qb_ext_hash(2, b"abc".as_ptr(), 3), 32);
            assert_eq!(result(), b"900150983cd24fb0d6963f7d28e17f72");
            assert_eq!(qb_ext_hash(7, std::ptr::null(), 0), -1);
            assert_eq!(qb_ext_crc32(b"123456789".as_ptr(), 9), 0xCBF4_3926);
            assert_eq!(qb_ext_codec(1, b"foobar".as_ptr(), 6), 8);
            assert_eq!(result(), b"Zm9vYmFy");
            assert_eq!(qb_ext_codec(2, b"Zm9vYmFy".as_ptr(), 8), 6);
            assert_eq!(result(), b"foobar");
            assert_eq!(qb_ext_codec(5, b"xyz".as_ptr(), 3), -1);
            assert_eq!(qb_ext_codec(4, std::ptr::null(), 0), 0);
        }
    }

    #[test]
    fn zip_and_http_through_the_c_abi() {
        let dir = std::env::temp_dir().join(format!("qb_ext_ffi_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let zp = dir.join("t.zip").to_string_lossy().into_owned();
        unsafe {
            let h = qb_ext_zip_create(zp.as_ptr(), zp.len() as i64);
            assert!(h > 0);
            assert_eq!(qb_ext_zip_add(h, b"a.txt".as_ptr(), 5, b"AAA".as_ptr(), 3, 0), 0);
            assert_eq!(qb_ext_zip_add(h, b"d/".as_ptr(), 2, std::ptr::null(), 0, 2), 0);
            assert_eq!(qb_ext_zip_add(99, b"x".as_ptr(), 1, std::ptr::null(), 0, 0), -1);
            assert_eq!(qb_ext_zip_close(h), 0);
            assert_eq!(qb_ext_zip_close(h), -1);
            assert_eq!(qb_ext_zip_list(zp.as_ptr(), zp.len() as i64), 2);
            assert_eq!(part(0), b"a.txt");
            assert_eq!(qb_ext_zip_entry_size(0), 3);
            assert_eq!(qb_ext_zip_read(zp.as_ptr(), zp.len() as i64, b"a.txt".as_ptr(), 5), 3);
            assert_eq!(result(), b"AAA");
        }
        let _ = std::fs::remove_dir_all(&dir);

        let base = http::testserver::start(1);
        let url = format!("{base}/ffi");
        let hdr = b"X-Test: 1";
        unsafe {
            let st = qb_ext_http(b"POST".as_ptr(), 4, url.as_ptr(), url.len() as i64, hdr.as_ptr(), 9, b"hi".as_ptr(), 2, 5000);
            assert_eq!(st, 200);
        }
        assert_eq!(result(), b"POST /ffi x-test=1 body=hi");
        assert!(qb_ext_http_headers() > 0);
        assert!(String::from_utf8(result()).unwrap().contains("x-server: qb-test"));
    }
}
