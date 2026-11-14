//! File builtins of the runtime, routed to `qb_runtime::fileio`: file numbers,
//! INPUT # parsing, PRINT # zones, GET/PUT records and EOF/LOC/SEEK behave as
//! in QB64.

use super::builtins::Host;
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


/// Value of an INPUT # number (the digits `qb_runtime::fileio` parsed).
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

/// The 64-bit integer an INPUT item stands for, when it is a whole number in the
/// range of `_INTEGER64` / `_UNSIGNED _INTEGER64` (the bits of a value above
/// `i64::MAX`); None: take it as a DOUBLE and round.
pub(crate) fn exact_i64(text: &[u8]) -> Option<i64> {
    let t = std::str::from_utf8(text).ok()?.trim();
    let digits = t.strip_prefix(['-', '+']).unwrap_or(t);
    if digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let v: i128 = t.strip_prefix('+').unwrap_or(t).parse().ok()?;
    if v < i128::from(i64::MIN) || v > i128::from(u64::MAX) {
        return None;
    }
    Some(v as u64 as i64)
}

/// 2^63: the first DOUBLE past the largest `_INTEGER64`.
pub(crate) const I64_LIMIT: f64 = 9_223_372_036_854_775_808.0;

fn int_value(n: &fio::NumRepr) -> R<i64> {
    let d = &n.digit[..n.digits as usize];
    // d0.d1d2... x 10^exp is whole when the exponent covers every digit
    if n.hex == 0 && !d.is_empty() && n.exp >= d.len() as i64 - 1 && n.exp < 20 {
        let mut text: Vec<u8> = if n.neg != 0 { vec![b'-'] } else { Vec::new() };
        text.extend_from_slice(d);
        text.resize(text.len() + (n.exp as usize + 1 - d.len()), b'0');
        if let Some(v) = exact_i64(&text) {
            return Ok(v);
        }
    }
    let v = num_value(n)?.round_ties_even();
    if v < -I64_LIMIT || v >= (2.0 * I64_LIMIT) {
        return e(6);
    }
    Ok(if v >= I64_LIMIT { v as u64 as i64 } else { v as i64 })
}

/// Handles `name` if it is a file builtin; `None` otherwise.
pub(crate) fn call<'a>(vm: &mut dyn Host<'a>, name: &str, a: &[V]) -> Option<R<Option<V>>> {
    let i = |k: usize| a[k].i();
    let s = |k: usize| a[k].s();
    let some = |v: V| Some(Ok(Some(v)));
    let ch0 = a.first().map_or(0, |v| v.i()) as i32;
    match name {
        "FIELDNEW" => {
            if !vm.rt().reclen.contains_key(&ch0) {
                return Some(e(if fio::eof(ch0).1 == 52 { 52 } else { 54 }));
            }
            vm.rt().field_cur = (ch0, 0);
            Some(Ok(None))
        }
        "FIELD" => {
            let (size, var) = (i(1), a[2].p());
            let rl = vm.rt().reclen.get(&ch0).copied().unwrap_or(0);
            if size < 0 {
                return Some(e(5));
            }
            let off = vm.rt().field_cur.1;
            if off + size as usize > rl {
                return Some(e(50));
            }
            vm.rt().field_cur.1 += size as usize;
            let buf = vm.rt().field_buf.entry(ch0).or_insert_with(|| vec![0u8; rl]);
            let val = buf[off..off + size as usize].to_vec();
            vm.rt().field_vars.retain(|f| f.0 != var);
            vm.rt().field_vars.push((var, ch0, off, size as usize));
            vm.rt().field_store = true;
            let r = vm.store_str(var, &val);
            vm.rt().field_store = false;
            Some(r.map(|_| None))
        }
        "GETFIELD" | "PUTFIELD" => {
            let pos = i(1);
            let rl = vm.rt().reclen.get(&ch0).copied().unwrap_or(0);
            let passed = pos != -1;
            let mut buf = vm.rt().field_buf.remove(&ch0).unwrap_or_else(|| vec![0u8; rl]);
            buf.resize(rl, 0);
            let err = if name == "GETFIELD" {
                fio::qb_file_get(ch0, pos, &mut buf, passed)
            } else {
                fio::qb_file_put(ch0, pos, &buf, passed)
            };
            let links: Vec<_> = vm.rt().field_vars.iter().filter(|f| f.1 == ch0).copied().collect();
            vm.rt().field_store = true;
            for (var, _, off, size) in links {
                let _ = vm.store_str(var, &buf[off..off + size]);
            }
            vm.rt().field_store = false;
            vm.rt().field_buf.insert(ch0, buf);
            Some(chk(err).map(|_| None))
        }
        "LSETVAR" => {
            let var = a[0].p();
            let val = a[1].s().to_vec();
            if let Some(&(_, ch, off, size)) = vm.rt().field_vars.iter().find(|f| f.0 == var)
                && let Some(b) = vm.rt().field_buf.get_mut(&ch) {
                    let n = size.min(val.len());
                    b[off..off + n].copy_from_slice(&val[..n]);
                }
            // the other variables FIELDed over the same bytes show the new contents too
            let links: Vec<_> = match vm.rt().field_vars.iter().find(|f| f.0 == var).copied() {
                Some((_, ch, off, size)) => vm.rt().field_vars.iter().filter(|f| f.1 == ch && f.0 != var && f.2 < off + size && off < f.2 + f.3).copied().collect(),
                None => Vec::new(),
            };
            vm.rt().field_store = true;
            let r = vm.store_str(var, &val);
            for (v, ch, off, size) in links {
                if let Some(bytes) = vm.rt().field_buf.get(&ch).map(|b| b[off..off + size].to_vec()) {
                    let _ = vm.store_str(v, &bytes);
                }
            }
            vm.rt().field_store = false;
            Some(r.map(|_| None))
        }
        // OPENL: OPENX with LEN = written in the program (0 is then an error, not "no LEN")
        "OPEN" | "OPENX" | "OPENL" => {
            let ty = match i(1) {
                1 => 3,
                2 => 4,
                3 => 5,
                4 => 2,
                _ => 1,
            };
            let reclen = i(3);
            let (access, sharing) = if name == "OPEN" { (0, 0) } else { (i(4) as i32, i(5) as i32) };
            let r = chk(fio::open(s(0), ty, access, sharing, i(2) as i32, reclen, reclen > 0 || name == "OPENL"));
            if r.is_ok() && ty == 1 {
                vm.rt().reclen.insert(i(2) as i32, if reclen > 0 { reclen as usize } else { 128 });
            }
            Some(r.map(|_| None))
        }
        // OPEN mode$, #n, file$[, reclen]: -1 = no record length (also QIR without the argument)
        "OPENOLD" | "OPENOLDL" => {
            let rl = a.get(3).map_or(-1, |v| v.i());
            let r = chk(fio::open_gwbasic(s(2), i(1) as i32, s(0), rl.max(0), rl >= 0));
            if r.is_ok() && s(2).first().is_some_and(|c| c & 223 == b'R') {
                vm.rt().reclen.insert(i(1) as i32, if rl > 0 { rl as usize } else { 128 });
            }
            Some(r.map(|_| None))
        }
        "CLOSE" => {
            fio::close(ch0);
            vm.rt().reclen.remove(&ch0);
            vm.rt().field_buf.remove(&ch0);
            vm.rt().field_vars.retain(|f| f.1 != ch0);
            Some(Ok(None))
        }
        "CLOSEALL" => {
            fio::close_all();
            vm.rt().net.close_all();
            vm.rt().reclen.clear();
            vm.rt().field_buf.clear();
            vm.rt().field_vars.clear();
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
            let (v, err) = if name == "LINEINPUT" { fio::qb_file_line_input(ch0) } else { fio::qb_file_input_string(ch0) };
            Some(chk(err).map(|_| Some(V::S(v.unwrap_or_default()))))
        }
        "INPUTITEM.x" if ch0 != 0 => {
            let mut n = fio::NumRepr::default();
            let (r, err) = fio::qb_file_input_number(ch0, &mut n);
            if err != 0 {
                return Some(e(err));
            }
            let v = match r {
                0 if n.hex != 0 => num_value(&n).map(qb_float80::F80::from_f64),
                0 => {
                    // d0.d1d2... x 10^exp as text, read with the precision of VAL
                    let d = &n.digit[..n.digits as usize];
                    let mut text: Vec<u8> = if n.neg != 0 { vec![b'-'] } else { Vec::new() };
                    text.push(d.first().copied().unwrap_or(b'0'));
                    text.push(b'.');
                    text.extend_from_slice(d.get(1..).unwrap_or(&[]));
                    text.extend_from_slice(format!("E{}", n.exp).as_bytes());
                    Ok(qb_runtime::val::val(&text).map_or(qb_float80::F80::from_f64(0.0), qb_float80::F80::from_bytes))
                }
                1 => e(6),
                2 => e(62),
                _ => Ok(qb_float80::F80::from_f64(0.0)),
            };
            Some(v.map(|v| Some(V::X(v))))
        }
        "INPUTITEM.i" if ch0 != 0 => {
            let mut n = fio::NumRepr::default();
            let (r, err) = fio::qb_file_input_number(ch0, &mut n);
            if err != 0 {
                return Some(e(err));
            }
            let v = match r {
                0 => int_value(&n),
                1 => e(6),
                2 => e(62),
                _ => Ok(0),
            };
            Some(v.map(|v| Some(V::I(v))))
        }
        "INPUTITEM.n" if ch0 != 0 => {
            let mut n = fio::NumRepr::default();
            let (r, err) = fio::qb_file_input_number(ch0, &mut n);
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
        // INPUT$(n) without a file number reads the keyboard (builtins.rs)
        "INPUT$" if i(1) != 0 => {
            let n = i(0);
            // the runtime reads at most a LONG count
            if n < 0 || n > i64::from(i32::MAX) {
                return Some(e(5));
            }
            let mut buf = vec![0u8; n as usize];
            let (len, err) = fio::qb_file_input_n(n as i32, i(1) as i32, &mut buf);
            buf.truncate(len);
            Some(chk(err).map(|_| Some(V::S(buf))))
        }
        "GETARR" | "PUTARR" => {
            // every element of the array, as one block of bytes
            let pos = i(1);
            let passed = pos != -1;
            let (base, total) = match vm.array_extent(a[2].p()) {
                Ok(x) => x,
                Err(x) => return Some(Err(x)),
            };
            if total == 0 {
                return Some(e(9)); // not dimensioned
            }
            let r = match vm.bytes(base, total as usize) {
                Ok(b) => b,
                Err(x) => return Some(Err(x)),
            };
            let err = if name == "GETARR" { fio::qb_file_get(ch0, pos, r, passed) } else { fio::qb_file_put(ch0, pos, r, passed) };
            Some(chk(err).map(|_| None))
        }
        "GETREC" | "PUTREC" => {
            let (pos, size) = (i(1), i(3) as usize);
            let passed = pos != -1;
            let ptr = a[2].p();
            let r = match vm.bytes(ptr, size) {
                Ok(b) => b,
                Err(x) => return Some(Err(x)),
            };
            let err = if name == "GETREC" { fio::qb_file_get(ch0, pos, r, passed) } else { fio::qb_file_put(ch0, pos, r, passed) };
            Some(chk(err).map(|_| None))
        }
        "PUTSTR" => {
            let pos = i(1);
            let d = s(2);
            let err = fio::qb_file_put_string(ch0, pos, d, pos != -1);
            Some(chk(err).map(|_| None))
        }
        "GETSTR" => {
            let pos = i(1);
            let mut fixed = vec![0u8; i(2).max(0) as usize];
            let (v, err) = fio::qb_file_get_string(ch0, pos, &mut fixed, pos != -1);
            let v = v.unwrap_or(fixed);
            Some(chk(err).map(|_| Some(V::S(v))))
        }
        _ => None,
    }
}
