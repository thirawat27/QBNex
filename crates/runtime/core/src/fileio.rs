//! File statements on top of [`crate::gfs`]: OPEN, CLOSE, PRINT #, INPUT #,
//! LINE INPUT #, INPUT$(n, #f), GET/PUT (incl. variable-length strings in
//! RANDOM files), FIELD/LSET/RSET buffers, LOF, EOF, SEEK, LOC, FREEFILE,
//! LOCK/UNLOCK and the internal `generic_put/get`.
//!
//! File numbers are QBasic file numbers; negative numbers are TCP/IP handles,
//! which the runner handles. Each function returns 0 or the QBasic error code
//! to raise. When QB64 raised an error and carried on, the first error is the
//! one returned (QB64's `error()` keeps only the first one, too).

use crate::gfs::{table, Table};

/// `sub_file_print` result: the file is "SCRN:", print to the screen instead.
pub const PRINT_TO_SCREEN: i32 = -1;

/// Usual mapping of a GFS failure to a QBasic error.
fn map(e: i32) -> i32 {
    match e {
        -2 => 258,
        -3 => 54,
        -4 => 5,
        -7 => 70,
        _ => 75,
    }
}

/// Error collector: keeps the first error, like `error()`.
#[derive(Default)]
struct Ctx {
    err: i32,
}

impl Ctx {
    fn raise(&mut self, e: i32) {
        if self.err == 0 {
            self.err = e;
        }
    }
}

/// Validates file number `i` and returns its table index (error 52 otherwise).
fn index(t: &mut Table, i: i32) -> Result<i32, i32> {
    if t.fileno_valid(i) != 1 {
        return Err(52);
    }
    Ok(t.fileno_index(i))
}

// --- OPEN / CLOSE --------------------------------------------------------------

/// `sub_open`. `type_` 0/1 RANDOM, 2 BINARY, 3 INPUT, 4 OUTPUT, 5 APPEND;
/// `access` 1 READ WRITE, 2 READ, 3 WRITE; `sharing` 1 SHARED, 2 LOCK READ
/// WRITE, 3 LOCK READ, 4 LOCK WRITE.
pub fn open(name: &[u8], mut type_: i32, access: i32, sharing: i32, i: i32, record_length: i64, passed: bool) -> i32 {
    let mut t = table();
    if type_ == 0 {
        type_ = 1;
    }
    if passed && (record_length == 0 || record_length < -1) {
        return 5;
    }
    match t.fileno_valid(i) {
        -2 => return 52,
        1 => return 55,
        _ => {}
    }
    // RANDOM: a record length that cannot be allocated is "Out of memory" (7)
    // before the file is opened, as QBasic reports it
    if type_ == 1 {
        let rl = if passed && record_length != -1 { record_length } else { 128 };
        if rl > i64::from(i32::MAX) {
            return 5;
        }
        if Vec::<u8>::new().try_reserve_exact(rl as usize).is_err() {
            return 7;
        }
    }
    let (mut g_access, mut g_restrictions, mut g_how) = match type_ {
        3 => (1, 0, 0),
        4 => (2, 0, 2),
        5 => (2, 0, 1),
        _ => (3, 0, 3),
    };
    match access {
        1 => g_access = 3,
        2 => g_access = 1,
        3 => g_access = 2,
        _ => {}
    }
    if access != 0 && g_how == 3 {
        g_how = 1;
    }
    match sharing {
        1 => g_restrictions = 0,
        2 => g_restrictions = 3,
        3 => g_restrictions = 1,
        4 => g_restrictions = 2,
        _ => {}
    }
    let x = t.open(name, g_access, g_restrictions, g_how);
    if x < 0 {
        return match x {
            -5 => 53,
            -6 => 76,
            -7 => 70,
            -8 => 68,
            -11 => 64,
            -12 => 54,
            _ => 53,
        };
    }
    t.fileno_use(i, x);
    {
        let f = t.f(x);
        f.type_ = if type_ == 5 { 4 } else { type_ as u8 };
        f.append = u8::from(type_ == 5);
        f.column = 1;
    }
    if type_ == 1 {
        let rl = if passed && record_length != -1 { record_length } else { 128 };
        t.f(x).record_length = rl;
    }
    if type_ == 5 {
        let n = t.lof(x);
        if n > 0 {
            t.setpos(x, n);
        }
    }
    if type_ == 3 {
        let n = t.lof(x);
        if n != 0 {
            let mut c = [0u8; 1];
            let e = t.read(x, -1, &mut c);
            if e != 0 {
                return if e == -7 { 70 } else { 75 };
            }
            if c[0] == 26 {
                t.f(x).eof_passed = 1;
            }
            t.setpos(x, 0);
        }
    }
    0
}

/// `sub_open_gwbasic`: `OPEN mode$, #n, name$[, reclen]`.
pub fn open_gwbasic(typestr: &[u8], i: i32, name: &[u8], record_length: i64, passed: bool) -> i32 {
    let Some(&first) = typestr.first() else { return 54 };
    let type_ = match first & 223 {
        b'R' => 1,
        b'B' => 2,
        b'I' => 3,
        b'O' => 4,
        b'A' => 5,
        _ => return 54,
    };
    if passed {
        open(name, type_, 0, 0, i, record_length, true)
    } else {
        open(name, type_, 0, 0, i, 0, false)
    }
}

/// `CLOSE #i` for a file number (not a TCP/IP handle).
pub fn close(i: i32) {
    let mut t = table();
    if t.fileno_valid(i) == 1 {
        let x = t.fileno_index(i);
        t.close(x);
    }
}

/// `CLOSE` without arguments: every open file number.
pub fn close_all() {
    let mut t = table();
    for i in 1..=t.max_fileno() {
        if t.fileno_valid(i) == 1 {
            let x = t.fileno_index(i);
            t.close(x);
        }
    }
}

/// FREEFILE.
pub fn freefile() -> i32 {
    table().freefile()
}

// --- PRINT # ---------------------------------------------------------------------

/// `sub_file_print`: writes `data`, then a space (`extraspace`), a comma tab
/// to the next 14-column zone (`tab`) and CR LF (`newline`).
pub fn file_print(i: i32, data: &[u8], extraspace: bool, tab: bool, newline: bool) -> i32 {
    let mut t = table();
    let x = match index(&mut t, i) {
        Ok(x) => x,
        Err(e) => return e,
    };
    if t.f(x).scrn == 1 {
        return PRINT_TO_SCREEN;
    }
    if t.f(x).type_ != 4 {
        return 54;
    }
    if t.f(x).write == 0 {
        return 75;
    }
    let e = t.write(x, -1, data);
    if e != 0 {
        return map(e);
    }
    let f = t.f(x);
    for &c in data {
        if c != 13 && c != 10 {
            f.column += 1;
        } else if c == 13 {
            f.column = 1;
        }
    }
    let mut nspaces = 0;
    if extraspace {
        nspaces += 1;
        f.column += 1;
    }
    if tab {
        nspaces += 1;
        f.column += 1;
        let r = (f.column - 1) % 14;
        if r != 0 {
            nspaces += 14 - r;
            f.column += 14 - r;
        }
    }
    if nspaces != 0 {
        let e = t.write(x, -1, &[b' '; 32][..nspaces as usize]);
        if e != 0 {
            return map(e);
        }
    }
    if newline {
        let e = t.write(x, -1, b"\r\n");
        if e != 0 {
            return map(e);
        }
        t.f(x).column = 1;
    }
    0
}

// --- INPUT # -------------------------------------------------------------------

/// `file_input_chr`: next byte, -1 at end of file (or CHR$(26), which is
/// then re-read by the next call), -2 after raising an error.
fn input_chr(t: &mut Table, i: i32, cx: &mut Ctx) -> i32 {
    let mut c = [0u8; 1];
    let e = t.read(i, -1, &mut c);
    if e != 0 {
        if e == -10 {
            return -1;
        }
        cx.raise(map(e));
        return -2;
    }
    if c[0] == 26 {
        let p = t.getpos(i);
        t.setpos(i, p - 1);
        t.f(i).eof_passed = 1;
        return -1;
    }
    i32::from(c[0])
}

fn back_one(t: &mut Table, i: i32) {
    let p = t.getpos(i);
    t.setpos(i, p - 1);
}

/// `file_input_skip1310`: after a CR or LF, also skips its LF/CR partner.
fn skip1310(t: &mut Table, i: i32, c: i32, cx: &mut Ctx) {
    let nextc = input_chr(t, i, cx);
    if nextc < 0 {
        return;
    }
    if (c == 10 && nextc != 13) || (c == 13 && nextc != 10) {
        back_one(t, i);
    } else {
        let nextc = input_chr(t, i, cx);
        if nextc < 0 {
            return;
        }
        back_one(t, i);
    }
}

/// `file_input_nextitem`: skips to the start of the next INPUT # item.
fn nextitem(t: &mut Table, i: i32, lastc: i32, cx: &mut Ctx) {
    if i < 0 {
        return;
    }
    let mut c = lastc;
    loop {
        if c == -1 {
            return;
        }
        if c == 32 {
            let nextc = input_chr(t, i, cx);
            if nextc < 0 {
                return;
            }
            if nextc != 32 && nextc != 44 && nextc != 10 && nextc != 13 {
                back_one(t, i);
                return;
            }
            c = nextc;
            continue;
        }
        if c == 44 {
            return;
        }
        if c == 10 || c == 13 {
            skip1310(t, i, c, cx);
            return;
        }
        c = input_chr(t, i, cx);
        if c == -2 {
            return;
        }
    }
}

/// Validation shared by the INPUT # readers (sequential INPUT mode only).
fn input_index(t: &mut Table, fileno: i32) -> Result<i32, i32> {
    let x = index(t, fileno)?;
    if t.f(x).type_ != 3 {
        return Err(54);
    }
    if t.f(x).read == 0 {
        return Err(75);
    }
    Ok(x)
}

/// Universal number representation filled by INPUT # (QB64's `n_digits`,
/// `n_digit`, `n_exp`, `n_neg`, `n_hex`).
pub struct NumRepr {
    pub digits: u16,
    pub digit: [u8; 256],
    pub exp: i64,
    pub neg: u8,
    pub hex: u8,
}

impl Default for NumRepr {
    fn default() -> Self {
        NumRepr { digits: 0, digit: [0; 256], exp: 0, neg: 0, hex: 0 }
    }
}

/// `n_inputnumberfromfile`: reads one number item. Returns 0 success,
/// 1 overflow, 2 end of file, 3 failed (an error was raised, see `cx`).
fn input_number(t: &mut Table, fileno: i32, n: &mut NumRepr, cx: &mut Ctx) -> i32 {
    let mut i = fileno;
    if fileno >= 0 {
        match input_index(t, fileno) {
            Ok(x) => i = x,
            Err(e) => {
                cx.raise(e);
                return 3;
            }
        }
    }
    let mut return_value = 1;
    let mut step = 0;
    let mut negate_exponent = false;
    let mut exponent_value: i64 = 0;
    let mut exponent_digits = 0;
    n.digits = 0;
    n.exp = 0;
    n.neg = 0;
    n.hex = 0;

    macro_rules! chr {
        () => {{
            let c = input_chr(t, i, cx);
            if c == -2 {
                return 3;
            }
            c
        }};
    }

    let mut c;
    loop {
        c = chr!();
        if c == -1 {
            return_value = 2;
            nextitem(t, i, c, cx);
            return return_value;
        }
        if c != 32 {
            break;
        }
    }

    let overflow = 'parse: {
        if c == 38 {
            // &H / &O / &B
            c = chr!();
            if c == -1 {
                break 'parse false;
            }
            let (kind, ok): (u8, fn(i32) -> bool) = match c {
                72 | 104 => (1, |c| (48..=57).contains(&c) || (65..=70).contains(&c) || (97..=102).contains(&c)),
                79 | 111 => (2, |c| (48..=55).contains(&c)),
                66 | 98 => (3, |c| (48..=49).contains(&c)),
                _ => break 'parse false,
            };
            loop {
                c = chr!();
                if !ok(c) {
                    break 'parse false;
                }
                if n.digits == 256 {
                    break 'parse true;
                }
                n.digit[n.digits as usize] = c as u8;
                n.digits += 1;
                n.hex = kind;
            }
        }
        loop {
            if c == -1 {
                break 'parse false;
            }
            match c {
                45 | 43 => {
                    // - +
                    if step == 0 {
                        if c == 45 {
                            n.neg = 1;
                        }
                        step = 1;
                    } else if step == 3 {
                        if c == 45 {
                            negate_exponent = true;
                        }
                        step = 4;
                    } else {
                        break 'parse false;
                    }
                }
                48..=57 => {
                    if step <= 1 {
                        step = 1;
                        if n.digits != 0 || c > 48 {
                            if n.digits != 0 {
                                n.exp += 1;
                            }
                            if n.digits == 256 {
                                break 'parse true;
                            }
                            n.digit[n.digits as usize] = c as u8;
                            n.digits += 1;
                        }
                    }
                    if step == 2 {
                        if n.digits == 0 && c == 48 {
                            n.exp -= 1;
                        }
                        if n.digits != 0 || c > 48 {
                            if n.digits == 256 {
                                break 'parse true;
                            }
                            n.digit[n.digits as usize] = c as u8;
                            n.digits += 1;
                        }
                    }
                    if step >= 3 {
                        step = 4;
                        if exponent_digits != 0 || c > 48 {
                            if exponent_digits == 18 {
                                break 'parse true;
                            }
                            exponent_value = exponent_value * 10 + i64::from(c - 48);
                            exponent_digits += 1;
                        }
                    }
                }
                46 => {
                    // .
                    if step > 1 {
                        break 'parse false;
                    }
                    if n.digits == 0 {
                        n.exp = -1;
                    }
                    step = 2;
                }
                68 | 69 | 100 | 101 => {
                    // D E d e
                    if step > 2 {
                        break 'parse false;
                    }
                    step = 3;
                }
                _ => break 'parse false,
            }
            c = chr!();
        }
    };
    if overflow {
        nextitem(t, i, c, cx);
        return return_value;
    }
    if negate_exponent {
        n.exp -= exponent_value;
    } else {
        n.exp += exponent_value;
    }
    if n.digits == 0 {
        n.exp = 0;
        n.neg = 0;
    }
    nextitem(t, i, c, cx);
    0
}

/// `sub_file_input_string`. Returns the error and, when legacy assigned the
/// destination, the new string.
fn input_string(t: &mut Table, fileno: i32, cx: &mut Ctx) -> Option<Vec<u8>> {
    let i = match input_index(t, fileno) {
        Ok(x) => x,
        Err(e) => {
            cx.raise(e);
            return None;
        }
    };
    let mut s = Vec::new();
    let mut c;
    loop {
        c = input_chr(t, i, cx);
        if c == -2 {
            return None;
        }
        if c == -1 {
            cx.raise(62);
            return Some(s);
        }
        if c != 32 && c != 9 {
            break;
        }
    }
    let inspeechmarks = c == 34;
    if inspeechmarks {
        c = input_chr(t, i, cx);
    }
    loop {
        if c == -2 {
            return None;
        }
        if c == -1 {
            break;
        }
        if inspeechmarks {
            if c == 34 {
                break;
            }
        } else if c == 44 || c == 10 || c == 13 {
            break;
        }
        s.push(c as u8);
        c = input_chr(t, i, cx);
    }
    if !inspeechmarks {
        while matches!(s.last(), Some(&b' ') | Some(&9)) {
            s.pop();
        }
    }
    loop {
        if c == -2 {
            return None;
        }
        if c == -1 || c == 44 {
            return Some(s);
        }
        if c == 10 || c == 13 {
            skip1310(t, i, c, cx);
            return Some(s);
        }
        c = input_chr(t, i, cx);
    }
}

/// `INPUT$(n, #fileno)` for INPUT and BINARY files. Fills `out` (n bytes)
/// and returns the length of the result.
fn input_n(t: &mut Table, n: i32, fileno: i32, out: &mut [u8], cx: &mut Ctx) -> usize {
    let len = out.len();
    let i = match index(t, fileno) {
        Ok(x) => x,
        Err(e) => {
            cx.raise(e);
            return len;
        }
    };
    let ty = t.f(i).type_;
    if !(2..=3).contains(&ty) {
        cx.raise(62);
        return len;
    }
    if t.f(i).read == 0 {
        cx.raise(75);
        return len;
    }
    if n < 0 {
        cx.raise(52);
        return len;
    }
    if n == 0 {
        return len;
    }
    if ty == 3 {
        for slot in out.iter_mut().take(n as usize) {
            let c = input_chr(t, i, cx);
            if c == -1 {
                cx.raise(62);
                return len;
            }
            if c == -2 {
                cx.raise(75);
                return len;
            }
            *slot = c as u8;
            if t.f(i).eof_passed != 1 {
                input_chr(t, i, cx);
                if t.f(i).eof_passed != 1 {
                    back_one(t, i);
                }
            }
        }
        return len;
    }
    let e = t.read(i, -1, &mut out[..n as usize]);
    if e != 0 && e != -10 {
        cx.raise(map(e));
        return 0;
    }
    t.read_bytes() as usize
}

/// `LINE INPUT #` on an INPUT file.
fn line_input_character(t: &mut Table, i: i32, cx: &mut Ctx) -> Option<Vec<u8>> {
    let mut s = Vec::new();
    let mut c = input_chr(t, i, cx);
    if c == -2 {
        return None;
    }
    if c == -1 {
        cx.raise(62);
        return Some(s);
    }
    loop {
        if c == -1 || c == 10 || c == 13 {
            break;
        }
        s.push(c as u8);
        c = input_chr(t, i, cx);
        if c == -2 {
            return None;
        }
    }
    if c == 10 || c == 13 {
        skip1310(t, i, c, cx);
    }
    Some(s)
}

/// `LINE INPUT #` on a BINARY file (reads 512-byte chunks via GET).
fn line_input_binary(t: &mut Table, fileno: i32, i: i32, cx: &mut Ctx) -> Option<Vec<u8>> {
    let start_byte = t.getpos(i) + 1; // SEEK(fileno) of a BINARY file
    let filelength = t.lof(i);
    if start_byte > filelength {
        cx.raise(62);
        return None;
    }
    let mut fbs: i64 = 512;
    let mut dest: Vec<u8> = Vec::new();
    loop {
        if start_byte + fbs > filelength {
            fbs = filelength - start_byte + 1;
        }
        let mut buffer = vec![b' '; fbs.max(0) as usize];
        get_bytes(t, fileno, start_byte, &mut buffer, true, cx);
        match buffer.iter().position(|&b| b == b'\n') {
            None => {
                if start_byte + fbs >= filelength {
                    t.setpos(i, filelength);
                    t.f(i).eof_passed = 1;
                    return Some(buffer);
                }
                fbs += 512;
            }
            Some(p) => {
                dest.extend_from_slice(&buffer[..p]);
                break;
            }
        }
        if t.f(i).eof_passed == 1 {
            break;
        }
    }
    let strip_cr = |d: &mut Vec<u8>| {
        if d.last() == Some(&b'\r') {
            d.pop();
        }
    };
    // `dest` ends before the LF at byte start_byte + len; the next line starts after
    // it. Legacy tested `+ 2` here and dropped a final line of 1 or 2 bytes.
    if start_byte + dest.len() as i64 >= filelength {
        t.setpos(i, filelength);
        t.f(i).eof_passed = 1;
        strip_cr(&mut dest);
        return Some(dest);
    }
    t.setpos(i, start_byte + dest.len() as i64);
    strip_cr(&mut dest);
    Some(dest)
}

/// `sub_file_line_input_string`.
fn line_input(t: &mut Table, fileno: i32, cx: &mut Ctx) -> Option<Vec<u8>> {
    let i = match index(t, fileno) {
        Ok(x) => x,
        Err(e) => {
            cx.raise(e);
            return None;
        }
    };
    if t.f(i).read == 0 {
        cx.raise(75);
        return None;
    }
    match t.f(i).type_ {
        2 => line_input_binary(t, fileno, i, cx),
        3 => line_input_character(t, i, cx),
        _ => {
            cx.raise(54);
            None
        }
    }
}

// --- GET / PUT -------------------------------------------------------------------

/// Byte offset for GET/PUT: record `offset` (1-based) or -1 = current position.
fn record_offset(type_: u8, record_length: i64, offset: i64, passed: bool) -> Result<i64, i32> {
    if !passed {
        return Ok(-1);
    }
    if offset < 1 {
        return Err(63);
    }
    let o = offset - 1;
    if type_ != 1 {
        return Ok(o);
    }
    // (o + 1) * rl fitting also keeps the callers' `offset + rl` in range
    offset.checked_mul(record_length).map(|end| end - record_length).ok_or(63)
}

/// `sub_get` for a file number: reads `buf.len()` bytes.
fn get_bytes(t: &mut Table, fileno: i32, offset: i64, buf: &mut [u8], passed: bool, cx: &mut Ctx) {
    let i = match index(t, fileno) {
        Ok(x) => x,
        Err(e) => return cx.raise(e),
    };
    let (ty, rl) = (t.f(i).type_, t.f(i).record_length);
    if ty > 2 {
        return cx.raise(54);
    }
    if t.f(i).read == 0 {
        return cx.raise(75);
    }
    let len = buf.len() as i64;
    if ty == 1 && len > rl {
        return cx.raise(59);
    }
    let offset = match record_offset(ty, rl, offset, passed) {
        Ok(o) => o,
        Err(e) => return cx.raise(e),
    };
    let e = t.read(i, offset, buf);
    if e != 0 && e != -10 {
        return cx.raise(map(e));
    }
    if ty == 1 && e != -10 && len < rl {
        let p = if offset != -1 { offset + rl } else { t.getpos(i) - len + rl };
        if t.setpos(i, p) != 0 {
            cx.raise(54);
        }
    }
}

/// `sub_put` for a file number.
fn put_bytes(t: &mut Table, fileno: i32, offset: i64, data: &[u8], passed: bool) -> i32 {
    let i = match index(t, fileno) {
        Ok(x) => x,
        Err(e) => return e,
    };
    let (ty, rl) = (t.f(i).type_, t.f(i).record_length);
    if ty > 2 {
        return 54;
    }
    if t.f(i).write == 0 {
        return 75;
    }
    let len = data.len() as i64;
    if ty == 1 && len > rl {
        return 59;
    }
    let offset = match record_offset(ty, rl, offset, passed) {
        Ok(o) => o,
        Err(e) => return e,
    };
    let e = t.write(i, offset, data);
    if e != 0 {
        return map(e);
    }
    if ty == 1 && len < rl {
        let p = if offset != -1 { offset + rl } else { t.getpos(i) - len + rl };
        if t.setpos(i, p) != 0 {
            return 54;
        }
    }
    0
}

/// `sub_get2` for a file number: GET of a variable-length string. BINARY
/// files fill the string in place (`fixed`); RANDOM files read the 2- or
/// 8-byte length header written by PUT and return the new string.
fn get_string(t: &mut Table, fileno: i32, offset: i64, fixed: &mut [u8], passed: bool, cx: &mut Ctx) -> Option<Vec<u8>> {
    let i = match index(t, fileno) {
        Ok(x) => x,
        Err(e) => {
            cx.raise(e);
            return None;
        }
    };
    let (ty, rl) = (t.f(i).type_, t.f(i).record_length);
    if ty > 2 {
        cx.raise(54);
        return None;
    }
    if t.f(i).read == 0 {
        cx.raise(75);
        return None;
    }
    if ty == 2 {
        let f = t.f(i).fileno;
        get_bytes(t, f, offset, fixed, passed, cx);
        return None;
    }
    if rl < 2 {
        cx.raise(59);
        return None;
    }
    let offset = match record_offset(ty, rl, offset, passed) {
        Ok(o) => o,
        Err(e) => {
            cx.raise(e);
            return None;
        }
    };
    // Legacy reads the record into a scratch vector and may copy up to 8 bytes
    // past it for corrupt headers (undefined behaviour); here those read as 0.
    let Some(n) = usize::try_from(rl).ok().and_then(|r| r.checked_add(8)) else {
        cx.raise(7);
        return None;
    };
    let mut data = Vec::new();
    if data.try_reserve_exact(n).is_err() {
        cx.raise(7);
        return None;
    }
    data.resize(n, 0);
    let e = t.read(i, offset, &mut data[..rl as usize]);
    if e != 0 && e != -10 {
        cx.raise(map(e));
        return None;
    }
    let bytes = t.read_bytes();
    if bytes == 0 {
        return Some(Vec::new());
    }
    if bytes < rl {
        let p = if offset != -1 { offset + rl } else { t.getpos(i) - bytes + rl };
        if t.setpos(i, p) != 0 {
            cx.raise(54);
            return None;
        }
    }
    let restore = |t: &mut Table| {
        let p = if offset != -1 { offset } else { t.getpos(i) - rl };
        t.setpos(i, p);
    };
    let mut x = 2usize;
    let mut l = u64::from(u16::from_le_bytes([data[0], data[1]]));
    if l & 32768 != 0 {
        if rl < 8 {
            restore(t);
            cx.raise(59);
            return None;
        }
        x = 8;
        let head = u64::from_le_bytes(data[..8].try_into().expect("8 bytes"));
        l = (l & 0x7FFF) + ((head >> 16) << 15);
    }
    // legacy compares against `record_length - x2` where x2 is an unset static (0)
    if (rl as u64) < l {
        restore(t);
        cx.raise(59);
        return None;
    }
    let l = l as usize;
    data.resize(x + l, 0);
    Some(data[x..x + l].to_vec())
}

/// `sub_put2` for a file number: PUT of a variable-length string; RANDOM
/// files get a 2-byte (or, above 32767 bytes, 8-byte) length header.
fn put_string(t: &mut Table, fileno: i32, offset: i64, data: &[u8], passed: bool) -> i32 {
    let i = match index(t, fileno) {
        Ok(x) => x,
        Err(e) => return e,
    };
    if t.f(i).type_ > 2 {
        return 54;
    }
    if t.f(i).write == 0 {
        return 75;
    }
    let f = t.f(i).fileno;
    if t.f(i).type_ == 2 {
        return put_bytes(t, f, offset, data, passed);
    }
    let l = data.len() as u64;
    let mut rec;
    if l > 32767 {
        // {15 bits of length | 0x8000}{48 bits: length >> 15}. Legacy ORs a
        // 64-bit word at byte 2, so its top 16 bits land on the first two data
        // bytes; they are zero for any length below 2^63.
        rec = vec![0u8; 8];
        rec[..2].copy_from_slice(&(((l & 32767) + 32768) as u16).to_le_bytes());
        rec[2..8].copy_from_slice(&(l >> 15).to_le_bytes()[..6]);
        rec.extend_from_slice(data);
    } else {
        rec = (l as u16).to_le_bytes().to_vec();
        rec.extend_from_slice(data);
    }
    put_bytes(t, f, offset, &rec, passed)
}

// --- LOF / EOF / SEEK / LOC / LOCK ---------------------------------------------------

/// LOF.
pub fn lof(i: i32) -> (i64, i32) {
    let mut t = table();
    let x = match index(&mut t, i) {
        Ok(x) => x,
        Err(e) => return (0, e),
    };
    let size = t.lof(x);
    if size < 0 {
        return (0, map(size as i32));
    }
    (size, 0)
}

/// EOF: -1 or 0.
pub fn eof(i: i32) -> (i32, i32) {
    let mut t = table();
    let x = match index(&mut t, i) {
        Ok(x) => x,
        Err(e) => return (0, e),
    };
    if t.f(x).scrn != 0 {
        return (0, 5);
    }
    if t.f(x).type_ != 3 {
        if t.eof_passed(x) == 1 {
            return (-1, 0);
        }
    } else if t.eof_reached(x) == 1 || t.eof_passed(x) == 1 {
        return (-1, 0);
    }
    (0, 0)
}

/// `SEEK #i, pos`.
pub fn seek_set(i: i32, mut pos: i64) -> i32 {
    let mut t = table();
    let x = match index(&mut t, i) {
        Ok(x) => x,
        Err(e) => return e,
    };
    if t.f(x).type_ == 1 {
        if pos < 1 {
            return 63;
        }
        pos -= 1;
        pos = match pos.checked_mul(t.f(x).record_length).and_then(|p| p.checked_add(1)) {
            Some(p) => p,
            None => return 63,
        };
    }
    if pos < 1 {
        return 63;
    }
    pos -= 1;
    let e = t.setpos(x, pos);
    if e < 0 {
        return map(e);
    }
    0
}

/// `SEEK(i)`.
pub fn seek_get(i: i32) -> (i64, i32) {
    let mut t = table();
    let x = match index(&mut t, i) {
        Ok(x) => x,
        Err(e) => return (0, e),
    };
    if t.f(x).scrn != 0 {
        return (0, 0);
    }
    let p = t.getpos(x);
    if t.f(x).type_ == 1 {
        return (p / t.f(x).record_length + 1, 0);
    }
    (p + 1, 0)
}

/// `FILEATTR(i, attribute)`: attribute 1 = how the file was opened (1 INPUT,
/// 2 OUTPUT, 4 RANDOM, 8 APPEND, 32 BINARY), 2 = a number that stands for the
/// open file (QuickBASIC returned the DOS handle there). Any other attribute is error 5.
pub fn fileattr(i: i32, attribute: i32) -> (i32, i32) {
    const INPUT: i32 = 1;
    const OUTPUT: i32 = 2;
    const RANDOM: i32 = 4;
    const APPEND: i32 = 8;
    const BINARY: i32 = 32;
    let mut t = table();
    let x = match index(&mut t, i) {
        Ok(x) => x,
        Err(e) => return (0, e),
    };
    let (kind, append) = (t.f(x).type_, t.f(x).append != 0);
    match attribute {
        1 => (
            match kind {
                1 => RANDOM,
                2 => BINARY,
                3 => INPUT,
                _ if append => APPEND,
                _ => OUTPUT,
            },
            0,
        ),
        2 => (x + 1, 0),
        _ => (0, 5),
    }
}

/// `LOC(i)`.
pub fn loc(i: i32) -> (i64, i32) {
    let mut t = table();
    let x = match index(&mut t, i) {
        Ok(x) => x,
        Err(e) => return (0, e),
    };
    if t.f(x).scrn != 0 {
        return (0, 5);
    }
    if cfg!(windows) && t.f(x).com_port != 0 {
        return (t.com_in_queue(x), 0);
    }
    let p = t.getpos(x);
    match t.f(x).type_ {
        1 => (p / t.f(x).record_length, 0),
        2 => (p, 0),
        _ => (p / 128, 0),
    }
}

/// `LOCK` / `UNLOCK` statements; `passed` bit 1 = start, bit 2 = end given.
pub fn lock(i: i32, mut start: i64, mut end: i64, mut passed: i32, lock: bool) -> i32 {
    let mut t = table();
    let x = match index(&mut t, i) {
        Ok(x) => x,
        Err(e) => return e,
    };
    let (ty, rl) = (t.f(x).type_, t.f(x).record_length);
    if ty > 2 {
        passed = 0;
    }
    if passed & 1 != 0 {
        if start < 1 {
            return 5;
        }
        start -= 1;
        if ty == 1 {
            start = match start.checked_mul(rl) {
                Some(s) => s,
                None => return 5,
            };
        }
    } else {
        start = -1;
    }
    if passed & 2 != 0 {
        if end < 1 {
            return 5;
        }
        end -= 1;
        if ty == 1 {
            // end * rl + rl - 1 == (end + 1) * rl - 1
            end = match (end + 1).checked_mul(rl) {
                Some(e) => e - 1,
                None => return 5,
            };
        }
    } else {
        end = start;
        if ty == 1 {
            end = match start.checked_add(rl) {
                Some(e) => e - 1,
                None => return 5,
            };
        }
        if passed & 1 == 0 {
            end = -1;
        }
    }
    let e = t.lock(x, start, end, lock);
    match e {
        0 => 0,
        -2 => 258,
        -4 => 5,
        -7 => 70,
        _ => 75,
    }
}

// --- LSET / RSET ----------------------------------------------------------------

/// LSET: left-aligns `src` in `dest`, padding with spaces.
pub fn lset(dest: &mut [u8], src: &[u8]) {
    if src.len() >= dest.len() {
        let n = dest.len();
        dest.copy_from_slice(&src[..n]);
        return;
    }
    dest[..src.len()].copy_from_slice(src);
    dest[src.len()..].fill(b' ');
}

/// RSET: right-aligns `src` in `dest`, padding with spaces.
pub fn rset(dest: &mut [u8], src: &[u8]) {
    if src.len() >= dest.len() {
        let n = dest.len();
        dest.copy_from_slice(&src[..n]);
        return;
    }
    let pad = dest.len() - src.len();
    dest[pad..].copy_from_slice(src);
    dest[..pad].fill(b' ');
}

// --- entry points of the runner: each also returns the error to raise (0 = none) ---

/// INPUT # of a number into `n`: (QB64's result code 0-3, error).
pub fn qb_file_input_number(fileno: i32, n: &mut NumRepr) -> (i32, i32) {
    let mut cx = Ctx::default();
    let r = input_number(&mut table(), fileno, n, &mut cx);
    (r, cx.err)
}

/// INPUT # of a string: (the value to assign, if any; error).
pub fn qb_file_input_string(fileno: i32) -> (Option<Vec<u8>>, i32) {
    let mut cx = Ctx::default();
    let v = input_string(&mut table(), fileno, &mut cx);
    (v, cx.err)
}

/// LINE INPUT #; same conventions as [`qb_file_input_string`].
pub fn qb_file_line_input(fileno: i32) -> (Option<Vec<u8>>, i32) {
    let mut cx = Ctx::default();
    let v = line_input(&mut table(), fileno, &mut cx);
    (v, cx.err)
}

/// `INPUT$(n, #fileno)` into `buf`: (bytes read, error).
pub fn qb_file_input_n(n: i32, fileno: i32, buf: &mut [u8]) -> (usize, i32) {
    let mut cx = Ctx::default();
    let len = input_n(&mut table(), n, fileno, buf, &mut cx);
    (len, cx.err)
}

/// GET of `data.len()` bytes.
pub fn qb_file_get(i: i32, offset: i64, data: &mut [u8], passed: bool) -> i32 {
    let mut cx = Ctx::default();
    get_bytes(&mut table(), i, offset, data, passed, &mut cx);
    cx.err
}

/// PUT of `data`.
pub fn qb_file_put(i: i32, offset: i64, data: &[u8], passed: bool) -> i32 {
    put_bytes(&mut table(), i, offset, data, passed)
}

/// GET of a variable-length string (`fixed` is the current string, filled in
/// place for BINARY files): (the new value to assign, if any; error).
pub fn qb_file_get_string(i: i32, offset: i64, fixed: &mut [u8], passed: bool) -> (Option<Vec<u8>>, i32) {
    let mut cx = Ctx::default();
    let v = get_string(&mut table(), i, offset, fixed, passed, &mut cx);
    (v, cx.err)
}

/// PUT of a variable-length string.
pub fn qb_file_put_string(i: i32, offset: i64, data: &[u8], passed: bool) -> i32 {
    put_string(&mut table(), i, offset, data, passed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfs::tests::{p, tmp};

    /// File numbers are global; each test uses its own range.
    fn fresh(base: i32) -> i32 {
        for f in base..base + 10 {
            close(f);
        }
        base
    }

    fn input_items(f: i32) -> Vec<Result<Vec<u8>, i32>> {
        let mut v = Vec::new();
        for _ in 0..20 {
            let mut cx = Ctx::default();
            let s = input_string(&mut table(), f, &mut cx);
            if cx.err != 0 {
                v.push(Err(cx.err));
                break;
            }
            v.push(Ok(s.unwrap()));
        }
        v
    }

    #[test]
    fn open_errors_and_modes() {
        let d = tmp("open");
        let f = fresh(100);
        assert_eq!(open(&p(&d, "none.txt"), 3, 0, 0, f, 0, false), 53);
        // Path not found (76) on Windows; legacy GFS_C reports bad file name (64).
        assert_eq!(open(&p(&d, "x/none.txt"), 4, 0, 0, f, 0, false), if cfg!(windows) { 76 } else { 64 });
        assert_eq!(open(&p(&d, "a.txt"), 4, 0, 0, 0, 0, false), 52);
        assert_eq!(open(&p(&d, "a.txt"), 1, 0, 0, f, 0, true), 5);
        assert_eq!(open(&p(&d, "a.txt"), 4, 0, 0, f, 0, false), 0);
        assert_eq!(open(&p(&d, "a.txt"), 4, 0, 0, f, 0, false), 55);
        assert_eq!(freefile(), freefile());
        close(f);
        assert_eq!(open_gwbasic(b"", f, &p(&d, "a.txt"), 0, false), 54);
        assert_eq!(open_gwbasic(b"x", f, &p(&d, "a.txt"), 0, false), 54);
        assert_eq!(open_gwbasic(b"r", f, &p(&d, "a.txt"), 64, true), 0);
        let x = table().fileno_index(f);
        let (ty, rl) = { let mut t = table(); (t.f(x).type_, t.f(x).record_length) };
        assert_eq!((ty, rl), (1, 64));
        close(f);
    }

    #[test]
    fn print_columns_tabs_and_input_quoting() {
        let d = tmp("print");
        let f = fresh(110);
        let name = p(&d, "s.txt");
        assert_eq!(open(&name, 4, 0, 0, f, 0, false), 0);
        assert_eq!(file_print(f, b"ab", false, true, false), 0); // comma zone
        assert_eq!(file_print(f, b"c", true, false, true), 0);
        assert_eq!(file_print(f, b"\"x, y\",  plain text  ,last", false, false, true), 0);
        assert_eq!(file_print(f, b"1", false, false, false), 0);
        close(f);
        assert_eq!(std::fs::read(d.join("s.txt")).unwrap(), b"ab            c \r\n\"x, y\",  plain text  ,last\r\n1");
        assert_eq!(open(&name, 3, 0, 0, f, 0, false), 0);
        assert_eq!(file_print(f, b"no", false, false, false), 54);
        let items = input_items(f);
        assert_eq!(
            items,
            vec![
                Ok(b"ab            c".to_vec()),
                Ok(b"x, y".to_vec()),
                Ok(b"plain text".to_vec()),
                Ok(b"last".to_vec()),
                Ok(b"1".to_vec()),
                Err(62)
            ]
        );
        close(f);
    }

    #[test]
    fn input_numbers_and_eof() {
        let d = tmp("num");
        std::fs::write(d.join("n.txt"), b"  12.5e1, &HFF\r\n.05 , 7\x1a99").unwrap();
        let f = fresh(120);
        assert_eq!(open(&p(&d, "n.txt"), 3, 0, 0, f, 0, false), 0);
        let mut n = NumRepr::default();
        let read = |n: &mut NumRepr| {
            let mut cx = Ctx::default();
            let r = input_number(&mut table(), f, n, &mut cx);
            (r, cx.err)
        };
        assert_eq!(read(&mut n), (0, 0));
        assert_eq!((&n.digit[..n.digits as usize], n.exp, n.neg), (&b"125"[..], 2, 0));
        assert_eq!(read(&mut n), (0, 0));
        assert_eq!((&n.digit[..n.digits as usize], n.hex), (&b"FF"[..], 1));
        assert_eq!(read(&mut n), (0, 0));
        assert_eq!((&n.digit[..n.digits as usize], n.exp), (&b"5"[..], -2));
        assert_eq!(read(&mut n), (0, 0));
        assert_eq!(eof(f), (-1, 0)); // CHR$(26) ends the file
        assert_eq!(read(&mut n).0, 2);
        close(f);
    }

    #[test]
    fn binary_get_put_seek_loc_lof() {
        let d = tmp("bin");
        let f = fresh(130);
        assert_eq!(open(&p(&d, "b.bin"), 2, 0, 0, f, 0, false), 0);
        let mut t = table();
        assert_eq!(put_bytes(&mut t, f, 3, b"XYZ", true), 0);
        drop(t);
        assert_eq!(lof(f), (5, 0));
        assert_eq!(seek_get(f), (6, 0));
        assert_eq!(loc(f), (5, 0));
        assert_eq!(seek_set(f, 0), 63);
        assert_eq!(seek_set(f, 1), 0);
        let mut buf = [9u8; 7];
        let mut cx = Ctx::default();
        get_bytes(&mut table(), f, 0, &mut buf, false, &mut cx);
        assert_eq!((cx.err, buf), (0, [0, 0, b'X', b'Y', b'Z', 0, 0]));
        assert_eq!(eof(f), (-1, 0));
        get_bytes(&mut table(), f, 0, &mut buf, true, &mut cx);
        assert_eq!(cx.err, 63);
        let mut out = [0u8; 4];
        seek_set(f, 3);
        let mut cx = Ctx::default();
        assert_eq!(input_n(&mut table(), 4, f, &mut out, &mut cx), 3);
        assert_eq!(&out[..3], b"XYZ");
        close(f);
        assert_eq!(lof(f), (0, 52));
        assert_eq!(eof(f), (0, 52));
    }

    #[test]
    fn random_records_and_strings() {
        let d = tmp("rnd");
        let f = fresh(140);
        assert_eq!(open(&p(&d, "r.dat"), 1, 0, 0, f, 10, true), 0);
        let mut t = table();
        assert_eq!(put_string(&mut t, f, 2, b"hello", true), 0);
        assert_eq!(put_bytes(&mut t, f, 1, b"12345678901", true), 59);
        assert_eq!(put_bytes(&mut t, f, 1, b"abc", true), 0);
        let ix = t.fileno_index(f);
        assert_eq!(t.getpos(ix), 10);
        let mut cx = Ctx::default();
        let s = get_string(&mut t, f, 2, &mut [], true, &mut cx);
        assert_eq!((cx.err, s), (0, Some(b"hello".to_vec())));
        let s = get_string(&mut t, f, 9, &mut [], true, &mut cx);
        assert_eq!((cx.err, s), (0, Some(Vec::new())));
        drop(t);
        assert_eq!(lof(f), (17, 0));
        assert_eq!(loc(f), (8, 0));
        assert_eq!(seek_set(f, 2), 0);
        assert_eq!(seek_get(f), (2, 0));
        close(f);
        let data = std::fs::read(d.join("r.dat")).unwrap();
        assert_eq!(&data[..3], b"abc");
    }

    #[test]
    fn line_input_modes() {
        let d = tmp("line");
        std::fs::write(d.join("l.txt"), b"one\r\ntwo\nthree\r\n").unwrap();
        let f = fresh(150);
        let li = || {
            let mut cx = Ctx::default();
            let v = line_input(&mut table(), f, &mut cx);
            (v, cx.err)
        };
        assert_eq!(open(&p(&d, "l.txt"), 3, 0, 0, f, 0, false), 0);
        assert_eq!(li(), (Some(b"one".to_vec()), 0));
        assert_eq!(li(), (Some(b"two".to_vec()), 0));
        assert_eq!(li(), (Some(b"three".to_vec()), 0));
        assert_eq!(li(), (Some(Vec::new()), 62));
        close(f);
        assert_eq!(open(&p(&d, "l.txt"), 2, 0, 0, f, 0, false), 0);
        assert_eq!(li(), (Some(b"one".to_vec()), 0));
        assert_eq!(li(), (Some(b"two".to_vec()), 0));
        assert_eq!(li(), (Some(b"three".to_vec()), 0));
        assert_eq!(eof(f), (-1, 0));
        close(f);
        // A final line of 1 or 2 bytes after a CRLF is read, not dropped
        for last in [&b"x"[..], b"xy", b"xyz"] {
            let mut data = b"two\r\n".to_vec();
            data.extend_from_slice(last);
            std::fs::write(d.join("s.txt"), &data).unwrap();
            assert_eq!(open(&p(&d, "s.txt"), 2, 0, 0, f, 0, false), 0);
            assert_eq!(li(), (Some(b"two".to_vec()), 0));
            assert_eq!(eof(f), (0, 0));
            assert_eq!(li(), (Some(last.to_vec()), 0));
            assert_eq!(eof(f), (-1, 0));
            close(f);
        }
    }

    #[test]
    fn lset_rset_and_lock_args() {
        let mut b = *b"xxxxx";
        lset(&mut b, b"ab");
        assert_eq!(&b, b"ab   ");
        rset(&mut b, b"ab");
        assert_eq!(&b, b"   ab");
        rset(&mut b, b"abcdefg");
        assert_eq!(&b, b"abcde");
        let d = tmp("lock");
        let f = fresh(160);
        assert_eq!(open(&p(&d, "k.bin"), 2, 0, 0, f, 0, false), 0);
        assert_eq!(lock(f, 0, 0, 1, true), 5);
        assert_eq!(lock(f, 1, 10, 3, true), 0);
        assert_eq!(lock(f, 1, 10, 3, false), 0);
        assert_eq!(lock(f + 5, 1, 10, 3, true), 52);
        close(f);
    }

    #[test]
    fn huge_numbers_lengths_and_records_fail_cleanly() {
        let d = tmp("bounds");
        let f = fresh(170);
        // file numbers past the table: error 52, nothing allocated
        assert_eq!(open(&p(&d, "h.txt"), 4, 0, 0, 2_000_000_000, 0, false), 52);
        assert_eq!(eof(2_000_000_000), (0, 52));
        // a record length no buffer can hold: illegal function call, no abort
        assert_eq!(open(&p(&d, "big.dat"), 1, 0, 0, f, 1_000_000_000_000, true), 5);
        assert_eq!(lof(f), (0, 52));
        // record numbers whose byte offset overflows i64
        assert_eq!(open(&p(&d, "r.dat"), 1, 0, 0, f, 10, true), 0);
        assert_eq!(put_bytes(&mut table(), f, i64::MAX, b"x", true), 63);
        let mut cx = Ctx::default();
        get_bytes(&mut table(), f, i64::MAX, &mut [0u8; 1], true, &mut cx);
        assert_eq!(cx.err, 63);
        assert_eq!(seek_set(f, i64::MAX), 63);
        assert_eq!(seek_set(f, i64::MIN), 63);
        assert_eq!(lock(f, 1, i64::MAX, 3, true), 5);
        assert_eq!(lock(f, i64::MAX, 0, 1, true), 5);
        close(f);
    }
}
