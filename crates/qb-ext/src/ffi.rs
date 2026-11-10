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

use crate::{codec, coll, datetime, hash, http, re, utf8, ziparc};
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

// ------------------------------------------------------------- UTF-8, CP437

/// Number of characters (invalid bytes count one each).
///
/// # Safety
/// Pointer/length pair valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_utf8_len(p: *const u8, l: i64) -> i64 {
    utf8::len(unsafe { bytes(p, l) }) as i64
}

/// Slices by character: `op` 1 = MID$(s, a, b) (b < 0: to the end),
/// 2 = LEFT$(s, a), 3 = RIGHT$(s, a). Returns the result length.
///
/// # Safety
/// Pointer/length pair valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_utf8_slice(op: i32, p: *const u8, l: i64, a: i64, b: i64) -> i64 {
    let s = unsafe { bytes(p, l) };
    match op {
        1 => ok_result(utf8::mid(s, a, b)),
        2 => ok_result(utf8::left(s, a)),
        3 => ok_result(utf8::right(s, a)),
        _ => fail(format!("unknown slice op {op}")),
    }
}

/// `op` 1 = upper case, 2 = lower case, 11..14 = normalize to NFC, NFD,
/// NFKC, NFKD. Returns the result length (-1 for an unknown op).
///
/// # Safety
/// Pointer/length pair valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_utf8_transform(op: i32, p: *const u8, l: i64) -> i64 {
    let s = unsafe { bytes(p, l) };
    match op {
        1 => ok_result(utf8::upper(s)),
        2 => ok_result(utf8::lower(s)),
        11..=14 => match utf8::Form::from_code(op - 10) {
            Some(f) => ok_result(utf8::normalize(s, f)),
            None => fail("bad form"),
        },
        _ => fail(format!("unknown transform {op}")),
    }
}

/// 1 when the bytes are valid UTF-8, else 0.
///
/// # Safety
/// Pointer/length pair valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_utf8_valid(p: *const u8, l: i64) -> i32 {
    i32::from(utf8::is_valid(unsafe { bytes(p, l) }))
}

/// Code point of character `index` (1-based); -1 if there is none.
///
/// # Safety
/// Pointer/length pair valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_utf8_codepoint(p: *const u8, l: i64, index: i64) -> i64 {
    utf8::codepoint(unsafe { bytes(p, l) }, index)
}

/// UTF-8 encoding of a code point; returns its length (0 if invalid).
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_utf8_chr(cp: i64) -> i64 {
    ok_result(utf8::from_codepoint(cp))
}

/// `op` 1 = CP437 to UTF-8 (control bytes stay ASCII), 2 = CP437 to UTF-8
/// with the PC symbols for 0x01-0x1F/0x7F, 3 = UTF-8 to CP437 (`?` for
/// characters without a CP437 form). Returns the result length.
///
/// # Safety
/// Pointer/length pair valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_cp437(op: i32, p: *const u8, l: i64) -> i64 {
    let s = unsafe { bytes(p, l) };
    match op {
        1 => ok_result(utf8::cp437_to_utf8(s, false)),
        2 => ok_result(utf8::cp437_to_utf8(s, true)),
        3 => ok_result(utf8::utf8_to_cp437(s, b'?')),
        _ => fail(format!("unknown CP437 op {op}")),
    }
}

// ------------------------------------------------------------- lists, dictionaries

thread_local! {
    static COLL: RefCell<coll::Store> = RefCell::new(coll::Store::default());
}

fn coll_with<R>(f: impl FnOnce(&mut coll::Store) -> R) -> R {
    COLL.with(|c| f(&mut c.borrow_mut()))
}

const BAD_HANDLE: &str = "not a list/dictionary handle";

/// New empty list; returns its handle.
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_list_new() -> i32 {
    coll_with(coll::Store::list_new)
}

/// New empty dictionary; returns its handle.
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_dict_new() -> i32 {
    coll_with(coll::Store::dict_new)
}

/// Frees a list or dictionary: 0, or -1 for an unknown handle.
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_coll_free(h: i32) -> i32 {
    if coll_with(|c| c.free(h)) { 0 } else { fail(BAD_HANDLE) as i32 }
}

fn list_op<R>(h: i32, bad: R, f: impl FnOnce(&mut Vec<coll::Value>) -> R) -> R {
    match coll_with(|c| c.list(h).map(f)) {
        Some(r) => {
            with(|s| s.error.clear());
            r
        }
        None => {
            fail(BAD_HANDLE);
            bad
        }
    }
}

fn dict_op<R>(h: i32, bad: R, f: impl FnOnce(&mut indexmap::IndexMap<Vec<u8>, coll::Value>) -> R) -> R {
    match coll_with(|c| c.dict(h).map(f)) {
        Some(r) => {
            with(|s| s.error.clear());
            r
        }
        None => {
            fail(BAD_HANDLE);
            bad
        }
    }
}

/// Number of items (-1 for a bad handle).
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_list_count(h: i32) -> i64 {
    list_op(h, -1, |l| l.len() as i64)
}

/// Appends a string; returns the new count.
///
/// # Safety
/// Pointer/length pair valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_list_add_str(h: i32, p: *const u8, l: i64) -> i64 {
    let v = unsafe { bytes(p, l) }.to_vec();
    list_op(h, -1, |list| {
        list.push(coll::Value::Str(v));
        list.len() as i64
    })
}

/// Appends a number; returns the new count.
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_list_add_num(h: i32, n: f64) -> i64 {
    list_op(h, -1, |list| {
        list.push(coll::Value::Num(n));
        list.len() as i64
    })
}

fn list_put(h: i32, i: i64, v: coll::Value, insert: bool) -> i32 {
    let r = list_op(h, Err(BAD_HANDLE), |list| {
        if insert {
            match usize::try_from(i).ok().filter(|&i| i <= list.len()) {
                Some(i) => {
                    list.insert(i, v);
                    Ok(())
                }
                None => Err("index out of range"),
            }
        } else {
            match coll::index(list.len(), i) {
                Some(i) => {
                    list[i] = v;
                    Ok(())
                }
                None => Err("index out of range"),
            }
        }
    });
    match r {
        Ok(()) => 0,
        Err(e) => fail(e) as i32,
    }
}

/// Replaces (`insert` = 0) or inserts before (`insert` <> 0) item `i` with a
/// string. 0 or -1.
///
/// # Safety
/// Pointer/length pair valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_list_put_str(h: i32, i: i64, p: *const u8, l: i64, insert: i32) -> i32 {
    list_put(h, i, coll::Value::Str(unsafe { bytes(p, l) }.to_vec()), insert != 0)
}

/// As [`qb_ext_list_put_str`] with a number.
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_list_put_num(h: i32, i: i64, n: f64, insert: i32) -> i32 {
    list_put(h, i, coll::Value::Num(n), insert != 0)
}

/// Type of item `i`: 0 none, 1 string, 2 number.
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_list_type(h: i32, i: i64) -> i32 {
    list_op(h, 0, |l| coll::index(l.len(), i).map_or(0, |i| l[i].kind()))
}

/// Item `i` as a string (numbers as text); returns its length, -1 if none.
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_list_get_str(h: i32, i: i64) -> i64 {
    match list_op(h, None, |l| coll::index(l.len(), i).map(|i| l[i].as_bytes())) {
        Some(v) => ok_result(v),
        None => -1,
    }
}

/// Item `i` as a number (strings read like VAL); 0 if none.
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_list_get_num(h: i32, i: i64) -> f64 {
    list_op(h, 0.0, |l| coll::index(l.len(), i).map_or(0.0, |i| l[i].as_num()))
}

/// Removes item `i`: 0, or -1.
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_list_remove(h: i32, i: i64) -> i32 {
    match list_op(h, None, |l| coll::index(l.len(), i).map(|i| l.remove(i))) {
        Some(_) => 0,
        None => fail("index out of range") as i32,
    }
}

/// Removes every item (lists and dictionaries): 0, or -1.
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_coll_clear(h: i32) -> i32 {
    let done = coll_with(|c| {
        if let Some(l) = c.list(h) {
            l.clear();
            true
        } else if let Some(d) = c.dict(h) {
            d.clear();
            true
        } else {
            false
        }
    });
    if done { 0 } else { fail(BAD_HANDLE) as i32 }
}

/// First index of a string item equal to the bytes (-1 if none).
///
/// # Safety
/// Pointer/length pair valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_list_find_str(h: i32, p: *const u8, l: i64) -> i64 {
    let needle = unsafe { bytes(p, l) };
    list_op(h, -1, |list| list.iter().position(|v| matches!(v, coll::Value::Str(s) if s.as_slice() == needle)).map_or(-1, |i| i as i64))
}

/// First index of a number item equal to `n` (-1 if none).
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_list_find_num(h: i32, n: f64) -> i64 {
    list_op(h, -1, |list| list.iter().position(|v| matches!(v, coll::Value::Num(x) if *x == n)).map_or(-1, |i| i as i64))
}

/// Joins the items as text with a separator; returns the length.
///
/// # Safety
/// Pointer/length pair valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_list_join(h: i32, p: *const u8, l: i64) -> i64 {
    let sep = unsafe { bytes(p, l) }.to_vec();
    match list_op(h, None, |list| Some(list.iter().map(coll::Value::as_bytes).collect::<Vec<_>>().join(sep.as_slice()))) {
        Some(v) => ok_result(v),
        None => -1,
    }
}

/// Sorts: numbers first (by value), then strings (by bytes, or ignoring
/// ASCII case with `ignore_case`). 0 or -1.
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_list_sort(h: i32, descending: i32, ignore_case: i32) -> i32 {
    list_op(h, -1, |l| {
        coll::sort(l, descending != 0, ignore_case != 0);
        0
    })
}

/// Number of keys (-1 for a bad handle).
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_dict_count(h: i32) -> i64 {
    dict_op(h, -1, |d| d.len() as i64)
}

/// Sets a key to a string. 0 or -1.
///
/// # Safety
/// Pointer/length pairs valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_dict_set_str(h: i32, k: *const u8, kl: i64, p: *const u8, l: i64) -> i32 {
    let (k, v) = (unsafe { bytes(k, kl) }.to_vec(), unsafe { bytes(p, l) }.to_vec());
    dict_op(h, -1, |d| {
        d.insert(k, coll::Value::Str(v));
        0
    })
}

/// Sets a key to a number. 0 or -1.
///
/// # Safety
/// Pointer/length pair valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_dict_set_num(h: i32, k: *const u8, kl: i64, n: f64) -> i32 {
    let k = unsafe { bytes(k, kl) }.to_vec();
    dict_op(h, -1, |d| {
        d.insert(k, coll::Value::Num(n));
        0
    })
}

/// Type of a key's value: 0 missing, 1 string, 2 number.
///
/// # Safety
/// Pointer/length pair valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_dict_type(h: i32, k: *const u8, kl: i64) -> i32 {
    let k = unsafe { bytes(k, kl) };
    dict_op(h, 0, |d| d.get(k).map_or(0, coll::Value::kind))
}

/// A key's value as a string; returns its length, -1 if missing.
///
/// # Safety
/// Pointer/length pair valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_dict_get_str(h: i32, k: *const u8, kl: i64) -> i64 {
    let k = unsafe { bytes(k, kl) };
    match dict_op(h, None, |d| d.get(k).map(coll::Value::as_bytes)) {
        Some(v) => ok_result(v),
        None => -1,
    }
}

/// A key's value as a number; 0 if missing.
///
/// # Safety
/// Pointer/length pair valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_dict_get_num(h: i32, k: *const u8, kl: i64) -> f64 {
    let k = unsafe { bytes(k, kl) };
    dict_op(h, 0.0, |d| d.get(k).map_or(0.0, coll::Value::as_num))
}

/// Removes a key: 1 removed, 0 missing, -1 bad handle.
///
/// # Safety
/// Pointer/length pair valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_dict_remove(h: i32, k: *const u8, kl: i64) -> i32 {
    let k = unsafe { bytes(k, kl) };
    dict_op(h, -1, |d| i32::from(d.shift_remove(k).is_some()))
}

/// Key number `i` (0-based, insertion order); returns its length, -1 if none.
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_dict_key(h: i32, i: i64) -> i64 {
    match dict_op(h, None, |d| usize::try_from(i).ok().and_then(|i| d.get_index(i)).map(|(k, _)| k.clone())) {
        Some(k) => ok_result(k),
        None => -1,
    }
}

// ------------------------------------------------------------- date and time

/// Current time, milliseconds since 1970-01-01 UTC.
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_dt_now() -> f64 {
    datetime::now_ms()
}

/// Local time zone offset at `ms`, in seconds east of UTC.
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_dt_offset(ms: f64) -> i32 {
    datetime::local_offset_seconds(ms).unwrap_or(0)
}

/// ISO 8601 text of `ms` (UTC with `Z`, or local with its offset).
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_dt_iso(ms: f64, local: i32) -> i64 {
    match datetime::iso(ms, local != 0) {
        Some(s) => ok_result(s.into_bytes()),
        None => fail("time out of range"),
    }
}

/// strftime-like formatting; returns the length (-1 on a bad format).
///
/// # Safety
/// Pointer/length pair valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_dt_format(ms: f64, f: *const u8, fl: i64, local: i32) -> i64 {
    match datetime::format(ms, &unsafe { text(f, fl) }, local != 0) {
        Ok(s) => ok_result(s.into_bytes()),
        Err(e) => fail(e),
    }
}

/// Parses text (empty format: ISO 8601); NaN on error.
///
/// # Safety
/// Pointer/length pairs valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_ext_dt_parse(t: *const u8, tl: i64, f: *const u8, fl: i64, local: i32) -> f64 {
    match datetime::parse(&unsafe { text(t, tl) }, &unsafe { text(f, fl) }, local != 0) {
        Ok(v) => {
            with(|s| s.error.clear());
            v
        }
        Err(e) => {
            fail(e);
            f64::NAN
        }
    }
}

/// Time from calendar parts (month 1-12); NaN for an invalid date.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn qb_ext_dt_make(year: i32, month: i32, day: i32, hour: i32, minute: i32, second: i32, millis: i32, local: i32) -> f64 {
    let u = |v: i32| u32::try_from(v).unwrap_or(u32::MAX);
    match datetime::make(year, u(month), u(day), u(hour), u(minute), u(second), u(millis), local != 0) {
        Ok(v) => {
            with(|s| s.error.clear());
            v
        }
        Err(e) => {
            fail(e);
            f64::NAN
        }
    }
}

/// A calendar part of `ms` (1 year, 2 month, 3 day, 4 hour, 5 minute,
/// 6 second, 7 millisecond, 8 weekday 0=Sunday, 9 day of year, 10 ISO
/// week). Returns 0 and sets the error for a bad part or time.
#[unsafe(no_mangle)]
pub extern "C" fn qb_ext_dt_part(ms: f64, part: i32, local: i32) -> i64 {
    match datetime::Part::from_code(part).and_then(|p| datetime::part(ms, p, local != 0)) {
        Some(v) => {
            with(|s| s.error.clear());
            v
        }
        None => {
            fail("bad part or time");
            0
        }
    }
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
