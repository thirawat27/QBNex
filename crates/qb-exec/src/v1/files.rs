//! File builtins of the v1 interpreter, routed to `qb_runtime::fileio` (the
//! same Rust code the C++ backend links), so file numbers, INPUT # parsing,
//! PRINT # zones, GET/PUT records and EOF/LOC/SEEK follow legacy exactly.

use super::rt::Host;
use super::{e, Stop, R, V};
use qb_runtime::fileio as fio;

fn chk(err: i32) -> R<()> {
    if err != 0 {
        return Err(Stop::Err(err));
    }
    Ok(())
}

/// Writes to file `ch` (no newline, no zone).
pub(crate) fn print(ch: i32, data: &[u8], tab: bool, newline: bool) -> R<bool> {
    let r = fio::file_print(ch, data, false, tab, newline);
    if r == fio::PRINT_TO_SCREEN {
        return Ok(false);
    }
    chk(r)?;
    Ok(true)
}

unsafe fn taken(out: *const u8, len: i32) -> Vec<u8> {
    if out.is_null() || len <= 0 {
        return Vec::new();
    }
    // SAFETY: fileio publishes a buffer of `len` bytes that stays valid until the next call.
    unsafe { std::slice::from_raw_parts(out, len as usize).to_vec() }
}

/// Value of an INPUT # number (n_float of libqb.cpp).
fn num_value(n: &fio::NumRepr) -> R<f64> {
    let d = &n.digit[..n.digits as usize];
    if d.is_empty() {
        return Ok(0.0);
    }
    let radix = match n.hex {
        1 => 16,
        2 => 8,
        3 => 2,
        _ => 0,
    };
    if radix != 0 {
        let mut v: i64 = 0;
        for c in d {
            let x = (*c as char).to_digit(16).unwrap_or(0) as i64;
            v = v.wrapping_mul(radix).wrapping_add(x);
        }
        return Ok(v as f64);
    }
    if n.exp > 308 {
        return e(6);
    }
    if n.exp < -324 {
        return Ok(0.0);
    }
    let mut s = String::new();
    if n.neg != 0 {
        s.push('-');
    }
    s.push(d[0] as char);
    s.push('.');
    if d.len() == 1 {
        s.push('0');
    } else {
        for c in &d[1..d.len().min(17)] {
            s.push(*c as char);
        }
    }
    s.push_str(&format!("E{}", n.exp));
    match s.parse::<f64>() {
        Ok(v) if v.is_finite() => Ok(v),
        _ => e(6),
    }
}

/// Handles `name` if it is a file builtin; `None` otherwise.
pub(crate) fn call<'a>(vm: &mut dyn Host<'a>, name: &str, a: &[V]) -> Option<R<Option<V>>> {
    let i = |k: usize| a[k].i();
    let s = |k: usize| a[k].s();
    let some = |v: V| Some(Ok(Some(v)));
    let ch0 = a.first().map_or(0, |v| v.i()) as i32;
    match name {
        "OPEN" => {
            let ty = match i(1) {
                1 => 3,
                2 => 4,
                3 => 5,
                4 => 2,
                _ => 1,
            };
            let reclen = i(3);
            Some(chk(fio::open(s(0), ty, 0, 0, i(2) as i32, reclen, reclen > 0)).map(|_| None))
        }
        "OPENOLD" => Some(chk(fio::open_gwbasic(s(2), i(1) as i32, s(0), 0, false)).map(|_| None)),
        "CLOSE" => {
            fio::close(ch0);
            Some(Ok(None))
        }
        "CLOSEALL" => {
            fio::close_all();
            Some(Ok(None))
        }
        "FREEFILE" => some(V::I(fio::freefile().into())),
        "EOF" | "LOF" | "LOC" | "SEEK" => {
            let (v, err) = match name {
                "EOF" => {
                    let (v, err) = fio::eof(ch0);
                    (i64::from(v), err)
                }
                "LOF" => fio::lof(ch0),
                "LOC" => fio::loc(ch0),
                _ => fio::seek_get(ch0),
            };
            Some(chk(err).map(|_| Some(V::I(v))))
        }
        "SEEKSET" => Some(chk(fio::seek_set(ch0, i(1))).map(|_| None)),
        "LINEINPUT" | "INPUTITEM.s" if ch0 != 0 => {
            let (mut out, mut len, mut assign) = (std::ptr::null(), 0i32, 0i32);
            // SAFETY: valid out-pointers.
            let err = unsafe {
                if name == "LINEINPUT" {
                    fio::qb_file_line_input(ch0, &mut out, &mut len, &mut assign)
                } else {
                    fio::qb_file_input_string(ch0, &mut out, &mut len, &mut assign)
                }
            };
            // SAFETY: published buffer.
            let v = if assign != 0 { unsafe { taken(out, len) } } else { Vec::new() };
            Some(chk(err).map(|_| Some(V::S(v))))
        }
        "INPUTITEM.n" if ch0 != 0 => {
            let mut n = fio::NumRepr::default();
            let mut r = 3;
            // SAFETY: valid pointers.
            let err = unsafe { fio::qb_file_input_number(ch0, &mut n, &mut r) };
            if err != 0 {
                return Some(e(err));
            }
            let v = match r {
                0 => num_value(&n),
                1 => e(6),
                2 => e(62),
                _ => Ok(0.0),
            };
            Some(v.map(|v| Some(V::F(v))))
        }
        "INPUT$" => {
            let n = i(0);
            if n < 0 {
                return Some(e(5));
            }
            let mut buf = vec![0u8; n as usize];
            let mut len = 0;
            // SAFETY: buffer of n bytes.
            let err = unsafe { fio::qb_file_input_n(n as i32, i(1) as i32, buf.as_mut_ptr(), n as i32, &mut len) };
            buf.truncate(len.max(0) as usize);
            Some(chk(err).map(|_| Some(V::S(buf))))
        }
        "GETREC" | "PUTREC" => {
            let (pos, size) = (i(1), i(3) as usize);
            let passed = i32::from(pos != -1);
            let ptr = a[2].p();
            let r = match vm.bytes(ptr, size) {
                Ok(b) => b,
                Err(x) => return Some(Err(x)),
            };
            // SAFETY: `r` is `size` bytes of VM memory.
            let err = unsafe {
                if name == "GETREC" {
                    fio::qb_file_get(ch0, pos, r.as_mut_ptr(), size as i32, passed)
                } else {
                    fio::qb_file_put(ch0, pos, r.as_ptr(), size as i32, passed)
                }
            };
            Some(chk(err).map(|_| None))
        }
        "PUTSTR" => {
            let pos = i(1);
            let d = s(2);
            // SAFETY: d is a live slice.
            let err = unsafe { fio::qb_file_put_string(ch0, pos, d.as_ptr(), d.len() as i32, i32::from(pos != -1)) };
            Some(chk(err).map(|_| None))
        }
        "GETSTR" => {
            let pos = i(1);
            let mut fixed = vec![0u8; i(2).max(0) as usize];
            let (mut out, mut len, mut assign) = (std::ptr::null(), 0i32, 0i32);
            // SAFETY: valid buffers.
            let err = unsafe {
                fio::qb_file_get_string(
                    ch0,
                    pos,
                    fixed.as_mut_ptr(),
                    fixed.len() as i32,
                    i32::from(pos != -1),
                    &mut out,
                    &mut len,
                    &mut assign,
                )
            };
            // SAFETY: published buffer.
            let v = if assign != 0 { unsafe { taken(out, len) } } else { fixed };
            Some(chk(err).map(|_| Some(V::S(v))))
        }
        _ => None,
    }
}
