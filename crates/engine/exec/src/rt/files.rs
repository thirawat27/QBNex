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
            vm.rt().field_store = true;
            let r = vm.store_str(var, &val);
            vm.rt().field_store = false;
            Some(r.map(|_| None))
        }
        "OPEN" | "OPENX" => {
            let ty = match i(1) {
                1 => 3,
                2 => 4,
                3 => 5,
                4 => 2,
                _ => 1,
            };
            let reclen = i(3);
            let (access, sharing) = if name == "OPENX" { (i(4) as i32, i(5) as i32) } else { (0, 0) };
            let r = chk(fio::open(s(0), ty, access, sharing, i(2) as i32, reclen, reclen > 0));
            if r.is_ok() && ty == 1 {
                vm.rt().reclen.insert(i(2) as i32, if reclen > 0 { reclen as usize } else { 128 });
            }
            Some(r.map(|_| None))
        }
        "OPENOLD" => Some(chk(fio::open_gwbasic(s(2), i(1) as i32, s(0), 0, false)).map(|_| None)),
        "CLOSE" => {
            fio::close(ch0);
            vm.rt().reclen.remove(&ch0);
            vm.rt().field_buf.remove(&ch0);
            vm.rt().field_vars.retain(|f| f.1 != ch0);
            Some(Ok(None))
        }
        "CLOSEALL" => {
            fio::close_all();
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
