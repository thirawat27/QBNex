//! `rt` builtins of the v1 reference interpreter (table: `qb_ir::builtins`).

use super::{cvt, e, Stop, R, V};
use qb_ir::Ty;
use qb_runtime::numfmt;
use std::io::{Read, Write};

/// Sequential text input (INPUT #, LINE INPUT #, stdin).
#[derive(Default)]
pub(crate) struct Reader {
    rbuf: Vec<u8>,
    rpos: usize,
    eof: bool,
}

/// What the builtins need from the executor that runs them (the reference
/// interpreter or the typed VM).
pub(crate) trait Host<'a> {
    fn rt(&mut self) -> &mut RtState<'a>;
    fn err(&self) -> i32;
    fn erl(&self) -> u32;
    /// `len` bytes of memory at `p` (GET/PUT of variables); error 5 when unsupported.
    fn bytes(&mut self, p: u64, len: usize) -> R<&mut [u8]>;
}

pub(crate) struct RtState<'a> {
    out: &'a mut dyn Write,
    col: usize,
    rnd: qb_runtime::rnd::RndState,
    using: Option<Using>,
    stdin: Reader,
}

struct Using {
    ch: i32,
    fmt: Vec<u8>,
    pos: usize,
    used: bool,
}

impl<'a> RtState<'a> {
    pub(crate) fn new(out: &'a mut dyn Write) -> Self {
        RtState { out, col: 1, rnd: Default::default(), using: None, stdin: Reader::default() }
    }
    pub(crate) fn flush(&mut self) {
        let _ = self.out.flush();
    }
    /// Console output, or file output through qb-runtime (SCRN: falls back to the console).
    pub(crate) fn write(&mut self, ch: i32, b: &[u8]) -> R<()> {
        if ch != 0 && super::files::print(ch, b, false, false)? {
            return Ok(());
        }
        let _ = self.out.write_all(b);
        for c in b {
            if *c == b'\n' {
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        Ok(())
    }
    fn nl(&mut self, ch: i32) -> R<()> {
        if ch != 0 && super::files::print(ch, b"", false, true)? {
            return Ok(());
        }
        self.write(0, b"\n")
    }
}

pub(crate) fn numeric_text(s: &[u8]) -> bool {
    let t = std::str::from_utf8(s).unwrap_or("").trim();
    if let Some(h) = t.strip_prefix('&') {
        return h.len() > 1;
    }
    let t = t.trim_end_matches(['#', '!', '%', '&', '~']);
    let t = t.replace(['D', 'd'], "E");
    !t.is_empty() && t.parse::<f64>().is_ok()
}

pub(crate) fn val(s: &[u8]) -> f64 {
    qb_runtime::val::val(s).map(qb_runtime::val::f64_from_f80).unwrap_or(0.0)
}

fn str_num(t: Ty, v: &V) -> Vec<u8> {
    match t {
        Ty::U64 => numfmt::str_u64(v.i() as u64),
        Ty::F32 => numfmt::str_f32(v.f() as f32),
        Ty::F64 | Ty::F80 => numfmt::str_f64(v.f()),
        _ => numfmt::str_i64(v.i()),
    }
}

fn suffix_ty(name: &str) -> Ty {
    name.rsplit('.').next().and_then(Ty::from_name).unwrap_or(Ty::I64)
}

fn count(v: &V) -> R<usize> {
    usize::try_from(v.i()).or_else(|_| e(5))
}

fn instr(start: usize, s: &[u8], t: &[u8]) -> i32 {
    if start == 0 || start > s.len() + usize::from(t.is_empty()) {
        return 0;
    }
    if t.is_empty() {
        return start as i32;
    }
    if start > s.len() {
        return 0;
    }
    s[start - 1..].windows(t.len()).position(|w| w == t).map_or(0, |p| (p + start) as i32)
}

fn instrrev(start: usize, s: &[u8], t: &[u8]) -> i32 {
    if s.is_empty() {
        return 0;
    }
    let st = if start == 0 || start > s.len() { s.len() } else { start };
    if t.is_empty() {
        return st as i32;
    }
    let mut i = st.min(s.len().saturating_sub(t.len()) + 1);
    while i >= 1 {
        if s[i - 1..].starts_with(t) {
            return i as i32;
        }
        i -= 1;
    }
    0
}

fn float1(a: &V, f: impl Fn(f64) -> f64) -> V {
    V::F(f(a.f()))
}

fn mid(s: &[u8], start: i64, len: Option<i64>) -> R<Vec<u8>> {
    if start < 1 {
        return e(5);
    }
    if let Some(l) = len {
        if l < 0 {
            return e(5);
        }
    }
    let st = (start - 1) as usize;
    if st >= s.len() {
        return Ok(vec![]);
    }
    let rest = &s[st..];
    let n = len.map_or(rest.len(), |l| (l as usize).min(rest.len()));
    Ok(rest[..n].to_vec())
}

fn midset(t: &[u8], start: i64, len: Option<i64>, r: &[u8]) -> R<Vec<u8>> {
    if start < 1 || start as usize > t.len() {
        return e(5);
    }
    let mut out = t.to_vec();
    let st = (start - 1) as usize;
    let mut n = r.len().min(t.len() - st);
    if let Some(l) = len {
        if l < 0 {
            return e(5);
        }
        n = n.min(l as usize);
    }
    out[st..st + n].copy_from_slice(&r[..n]);
    Ok(out)
}

fn trim_l(s: &[u8]) -> Vec<u8> {
    let i = s.iter().position(|c| *c != b' ').unwrap_or(s.len());
    s[i..].to_vec()
}

fn trim_r(s: &[u8]) -> Vec<u8> {
    let i = s.iter().rposition(|c| *c != b' ').map_or(0, |p| p + 1);
    s[..i].to_vec()
}

fn fixed<const N: usize>(s: &[u8]) -> R<[u8; N]> {
    match s.get(..N).and_then(|b| b.try_into().ok()) {
        Some(b) => Ok(b),
        None => e(5),
    }
}

fn write_num(t: Ty, v: &V) -> Vec<u8> {
    let s = str_num(t, v);
    if s.first() == Some(&b' ') {
        s[1..].to_vec()
    } else {
        s
    }
}

pub(crate) fn call<'a>(vm: &mut dyn Host<'a>, name: &'static str, a: Vec<V>) -> R<Option<V>> {
    let s = |i: usize| a[i].s();
    let i = |k: usize| a[k].i();
    let f = |k: usize| a[k].f();
    let ok = |v: V| Ok(Some(v));
    if let Some(r) = super::files::call(vm, name, &a) {
        return r;
    }
    let (err, erl) = (vm.err(), vm.erl());
    let rt = vm.rt();
    match name {
        "PRINTS" => rt.write(i(0) as i32, s(1))?,
        n if n.starts_with("PRINTN.") => {
            let mut b = str_num(suffix_ty(n), &a[1]);
            b.push(b' ');
            rt.write(i(0) as i32, &b)?
        }
        "PRINTZONE" => {
            let ch = i(0) as i32;
            if ch != 0 && super::files::print(ch, b"", true, false)? {
                return Ok(None);
            }
            let col = rt.col;
            let next = ((col - 1) / 14 + 1) * 14 + 1;
            rt.write(ch, &vec![b' '; next - col])?
        }
        "PRINTNL" => rt.nl(i(0) as i32)?,
        "PRINTTAB" => {
            let ch = i(0) as i32;
            let n = i(1).max(1) as usize;
            let col = if ch == 0 { rt.col } else { 1 };
            if col > n {
                rt.nl(ch)?;
                rt.write(ch, &vec![b' '; n - 1])?
            } else {
                rt.write(ch, &vec![b' '; n - col])?
            }
        }
        "PRINTSPC" => rt.write(i(0) as i32, &vec![b' '; i(1).max(0) as usize])?,
        "WRITES" => {
            let mut b = vec![b'"'];
            b.extend_from_slice(s(1));
            b.push(b'"');
            rt.write(i(0) as i32, &b)?
        }
        n if n.starts_with("WRITEN.") => rt.write(i(0) as i32, &write_num(suffix_ty(n), &a[1]))?,
        "WRITESEP" => rt.write(i(0) as i32, b",")?,
        "USINGBEGIN" => {
            if s(1).is_empty() {
                return e(5);
            }
            rt.using = Some(Using { ch: i(0) as i32, fmt: s(1).to_vec(), pos: 0, used: false })
        }
        "USINGS" | "USINGEND" => return using(vm, name, &a),
        n if n.starts_with("USINGN.") => return using(vm, name, &a),
        n if n.starts_with("STR$.") => return ok(V::S(str_num(suffix_ty(n), &a[0]))),
        "VAL" => return ok(V::F(val(s(0)))),
        "HEX$" | "OCT$" | "BIN$" => {
            let v = i(0) as u64;
            let t = match name {
                "HEX$" => format!("{v:X}"),
                "OCT$" => format!("{v:o}"),
                _ => format!("{v:b}"),
            };
            return ok(V::S(t.into_bytes()));
        }
        "MKI$" => return ok(V::S((i(0) as i16).to_le_bytes().to_vec())),
        "MKL$" => return ok(V::S((i(0) as i32).to_le_bytes().to_vec())),
        "MKS$" => return ok(V::S((f(0) as f32).to_le_bytes().to_vec())),
        "MKD$" => return ok(V::S(f(0).to_le_bytes().to_vec())),
        "_MK$.i64" => return ok(V::S(i(0).to_le_bytes().to_vec())),
        "CVI" => return ok(V::I(i16::from_le_bytes(fixed(s(0))?).into())),
        "CVL" => return ok(V::I(i32::from_le_bytes(fixed(s(0))?).into())),
        "CVS" => return ok(V::F(f32::from_le_bytes(fixed(s(0))?).into())),
        "CVD" => return ok(V::F(f64::from_le_bytes(fixed(s(0))?))),
        "_CV.i64" => return ok(V::I(i64::from_le_bytes(fixed(s(0))?))),
        "MKSMBF$" => {
            let b = qb_runtime::mbf::fieee_to_msbin((f(0) as f32).to_le_bytes()).map_err(|_| Stop::Err(5))?;
            return ok(V::S(b.to_vec()));
        }
        "MKDMBF$" => {
            let b = qb_runtime::mbf::dieee_to_msbin(f(0).to_le_bytes()).map_err(|_| Stop::Err(5))?;
            return ok(V::S(b.to_vec()));
        }
        "CVSMBF" => {
            let b = qb_runtime::mbf::fmsbin_to_ieee(fixed(s(0))?).map_err(|_| Stop::Err(5))?;
            return ok(V::F(f32::from_le_bytes(b).into()));
        }
        "CVDMBF" => {
            let b = qb_runtime::mbf::dmsbin_to_ieee(fixed(s(0))?).map_err(|_| Stop::Err(5))?;
            return ok(V::F(f64::from_le_bytes(b)));
        }
        "LEN" => return ok(V::I(s(0).len() as i64)),
        "LEFT$" => {
            let n = count(&a[1])?;
            return ok(V::S(s(0)[..n.min(s(0).len())].to_vec()));
        }
        "RIGHT$" => {
            let n = count(&a[1])?;
            let t = s(0);
            return ok(V::S(t[t.len() - n.min(t.len())..].to_vec()));
        }
        "MID$" => return ok(V::S(mid(s(0), i(1), None)?)),
        "MID$.n" => return ok(V::S(mid(s(0), i(1), Some(i(2)))?)),
        "MIDSET" => return ok(V::S(midset(s(0), i(1), None, s(2))?)),
        "MIDSET.n" => return ok(V::S(midset(s(0), i(1), Some(i(2)), s(3))?)),
        "CHR$" => {
            let c = i(0);
            if !(0..=255).contains(&c) {
                return e(5);
            }
            return ok(V::S(vec![c as u8]));
        }
        "ASC" => return ok(V::I(i64::from(*s(0).first().ok_or(Stop::Err(5))?))),
        "ASC.n" => {
            let k = i(1);
            if k < 1 {
                return e(5);
            }
            return ok(V::I(s(0).get(k as usize - 1).map_or(-1, |c| i64::from(*c))));
        }
        "INSTR" => return ok(V::I(instr(1, s(0), s(1)).into())),
        "INSTR.n" => {
            if i(0) < 1 {
                return e(5);
            }
            return ok(V::I(instr(i(0) as usize, s(1), s(2)).into()));
        }
        "_INSTRREV" => return ok(V::I(instrrev(0, s(0), s(1)).into())),
        "_INSTRREV.n" => return ok(V::I(instrrev(i(0).max(0) as usize, s(1), s(2)).into())),
        "UCASE$" => return ok(V::S(s(0).to_ascii_uppercase())),
        "LCASE$" => return ok(V::S(s(0).to_ascii_lowercase())),
        "LTRIM$" => return ok(V::S(trim_l(s(0)))),
        "RTRIM$" => return ok(V::S(trim_r(s(0)))),
        "_TRIM$" => return ok(V::S(trim_l(&trim_r(s(0))))),
        "SPACE$" => return ok(V::S(vec![b' '; count(&a[0])?])),
        "STRING$" => {
            let c = i(1);
            if !(0..=255).contains(&c) {
                return e(5);
            }
            return ok(V::S(vec![c as u8; count(&a[0])?]));
        }
        "STRING$.s" => {
            let c = *s(1).first().ok_or(Stop::Err(5))?;
            return ok(V::S(vec![c; count(&a[0])?]));
        }
        "LSET" => {
            let n = s(0).len();
            let mut r = s(1)[..s(1).len().min(n)].to_vec();
            r.resize(n, b' ');
            return ok(V::S(r));
        }
        "RSET" => {
            let n = s(0).len();
            let src = &s(1)[..s(1).len().min(n)];
            let mut r = vec![b' '; n - src.len()];
            r.extend_from_slice(src);
            return ok(V::S(r));
        }
        "ABS.i64" => return ok(V::I(i(0).wrapping_abs())),
        "ABS.f32" | "ABS.f64" | "ABS.f80" => return ok(V::F(f(0).abs())),
        "SGN.i64" => return ok(V::I(i(0).signum())),
        "SGN.f64" | "SGN.f80" => {
            let x = f(0);
            return ok(V::I(if x > 0.0 { 1 } else if x < 0.0 { -1 } else { 0 }));
        }
        "INT.f32" => return ok(V::F(f64::from((f(0) as f32).floor()))),
        "INT.f64" | "INT.f80" => return ok(float1(&a[0], f64::floor)),
        "FIX.f32" | "FIX.f64" | "FIX.f80" => return ok(float1(&a[0], f64::trunc)),
        "SQR" => {
            if f(0) < 0.0 {
                return e(5);
            }
            return ok(float1(&a[0], f64::sqrt));
        }
        "SIN" => return ok(float1(&a[0], f64::sin)),
        "COS" => return ok(float1(&a[0], f64::cos)),
        "TAN" => return ok(float1(&a[0], f64::tan)),
        "ATN" => return ok(float1(&a[0], f64::atan)),
        "LOG" => {
            if f(0) <= 0.0 {
                return e(5);
            }
            return ok(float1(&a[0], f64::ln));
        }
        "EXP" => {
            let r = f(0).exp();
            if r.is_infinite() {
                return e(6);
            }
            return ok(V::F(r));
        }
        "_ROUND.f64" => return ok(cvt(Ty::I64, Ty::F64, &a[0], true)),
        "RND" => return ok(V::F(f64::from(rt.rnd.rnd(None)))),
        "RND.n" => return ok(V::F(f64::from(rt.rnd.rnd(Some(f(0) as f32))))),
        "RANDOMIZE" => rt.rnd.randomize(f(0)),
        "RANDOMIZEUSING" => rt.rnd.randomize_using(f(0)),
        "TIMER.n" => {
            use std::time::{SystemTime, UNIX_EPOCH};
            let t = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0) % 86400.0;
            let acc = f(0);
            return ok(V::F(if acc > 0.0 { (t / acc).round() * acc } else { t }));
        }
        "TIMER" => {
            use std::time::{SystemTime, UNIX_EPOCH};
            let t = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
            return ok(V::F(t % 86400.0));
        }
        "ERR" => return ok(V::I(err.into())),
        "ERL" | "_ERRORLINE" => return ok(V::I(erl.into())),
        "LINEINPUT" => {
            let ch = i(0) as i32;
            let _ = ch;
            fill(&mut std::io::stdin(), &mut rt.stdin);
            let line = read_line(&mut rt.stdin)?;
            return ok(V::S(line));
        }
        "INPUTPROMPT" => {
            if i(1) == 0 {
                rt.write(0, s(0))?;
            }
        }
        "INPUTITEM.s" | "INPUTITEM.n" => {
            let ch = i(0) as i32;
            let _ = ch;
            fill(&mut std::io::stdin(), &mut rt.stdin);
            let item = read_item(&mut rt.stdin, name == "INPUTITEM.n")?;
            if name == "INPUTITEM.s" {
                return ok(V::S(item));
            }
            return ok(V::F(val(&item)));
        }
        "KILL" => {
            let spec = String::from_utf8_lossy(s(0)).into_owned();
            let files = glob_files(&spec);
            if files.is_empty() {
                return e(53);
            }
            for f in files {
                if std::fs::remove_file(&f).is_err() {
                    return e(75);
                }
            }
        }
        "NAME" => {
            if std::fs::rename(String::from_utf8_lossy(s(0)).as_ref(), String::from_utf8_lossy(s(1)).as_ref()).is_err() {
                return e(53);
            }
        }
        "MKDIR" => {
            if std::fs::create_dir(String::from_utf8_lossy(s(0)).as_ref()).is_err() {
                return e(75);
            }
        }
        "RMDIR" => {
            if std::fs::remove_dir(String::from_utf8_lossy(s(0)).as_ref()).is_err() {
                return e(75);
            }
        }
        "CHDIR" => {
            if std::env::set_current_dir(String::from_utf8_lossy(s(0)).as_ref()).is_err() {
                return e(76);
            }
        }
        "_FILEEXISTS" => {
            return ok(V::I(if std::path::Path::new(String::from_utf8_lossy(s(0)).as_ref()).is_file() { -1 } else { 0 }))
        }
        "_DIREXISTS" => {
            return ok(V::I(if std::path::Path::new(String::from_utf8_lossy(s(0)).as_ref()).is_dir() { -1 } else { 0 }))
        }
        "CLS" | "BEEP" | "_DEST" | "COLOR" | "LOCATE" => {}
        "POS" => return ok(V::I(rt.col as i64)),
        "CSRLIN" => return ok(V::I(1)),
        "INKEY$" => return ok(V::S(vec![])),
        "SLEEP" => {}
        "COMMAND$" => {
            let args: Vec<String> = std::env::args().skip(1).collect();
            return ok(V::S(args.join(" ").into_bytes()));
        }
        "ENVIRON$" => {
            return ok(V::S(std::env::var_os(String::from_utf8_lossy(s(0)).as_ref()).map(|v| v.to_string_lossy().into_owned().into_bytes()).unwrap_or_default()))
        }
        "ENVIRON$.n" => {
            let k = i(0);
            let v = std::env::vars().nth((k - 1).max(0) as usize).map(|(a, b)| format!("{a}={b}").into_bytes());
            return ok(V::S(v.unwrap_or_default()));
        }
        "ENVIRON" => {
            let t = String::from_utf8_lossy(s(0)).into_owned();
            let lead = t.len() - t.trim_start().len();
            let split = t.find('=').or_else(|| t[lead..].find(' ').map(|p| p + lead));
            if let Some(p) = split {
                let k = t[..p].trim().to_string();
                let v = if t.as_bytes()[p] == b' ' { t[p + 1..].trim_start() } else { &t[p + 1..] };
                if v.is_empty() {
                    // SAFETY: the program thread is the only one touching the environment.
                    unsafe { std::env::remove_var(&k) };
                } else {
                    // SAFETY: as above.
                    unsafe { std::env::set_var(&k, v) };
                }
            }
        }
        "SHELL" => {
            rt.flush();
            let cmd = String::from_utf8_lossy(s(0)).into_owned();
            let st = if cfg!(windows) {
                std::process::Command::new("cmd").args(["/c", &cmd]).status()
            } else {
                std::process::Command::new("sh").args(["-c", &cmd]).status()
            };
            return ok(V::I(st.ok().and_then(|s| s.code()).unwrap_or(-1).into()));
        }
        "DATE$" | "TIME$" => return ok(V::S(vec![])),
        "_STRCMP" | "_STRICMP" => {
            let (x, y) = if name == "_STRICMP" {
                (s(0).to_ascii_lowercase(), s(1).to_ascii_lowercase())
            } else {
                (s(0).to_vec(), s(1).to_vec())
            };
            return ok(V::I(match x.cmp(&y) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            }));
        }
        "_CWD$" | "_STARTDIR$" => {
            let d = std::env::current_dir().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
            return ok(V::S(d.into_bytes()));
        }
        "FIELD" => return e(73),
        _ => return e(73),
    }
    Ok(None)
}

/// Files matching a DOS pattern (`*`, `?` in the last component).
fn glob_files(spec: &str) -> Vec<std::path::PathBuf> {
    if !spec.contains(['*', '?']) {
        let p = std::path::PathBuf::from(spec);
        return if p.is_file() { vec![p] } else { vec![] };
    }
    let path = std::path::Path::new(spec);
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(std::path::Path::new("."));
    let pat: Vec<u8> = path.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase().into_bytes()).unwrap_or_default();
    fn m(p: &[u8], t: &[u8]) -> bool {
        match (p.first(), t.first()) {
            (None, None) => true,
            (Some(b'*'), _) => m(&p[1..], t) || (!t.is_empty() && m(p, &t[1..])),
            (Some(b'?'), Some(_)) => m(&p[1..], &t[1..]),
            (Some(a), Some(b)) if a == b => m(&p[1..], &t[1..]),
            _ => false,
        }
    }
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_ascii_lowercase();
            if e.path().is_file() && m(&pat, name.as_bytes()) {
                out.push(e.path());
            }
        }
    }
    out
}

fn fill(f: &mut dyn Read, rd: &mut Reader) {
    if !rd.eof {
        let mut b = Vec::new();
        let _ = f.read_to_end(&mut b);
        rd.rbuf.extend_from_slice(&b);
        rd.eof = true;
    }
}

fn read_line(f: &mut Reader) -> R<Vec<u8>> {
    let b = &f.rbuf;
    let mut p = f.rpos;
    if p >= b.len() || b[p] == 26 {
        return e(62);
    }
    let mut line = Vec::new();
    while p < b.len() && b[p] != b'\n' && b[p] != b'\r' && b[p] != 26 {
        line.push(b[p]);
        p += 1;
    }
    f.rpos = skip_eol(b, p);
    Ok(line)
}

/// Consumes one line terminator (CR, LF or CR LF) at `p`.
fn skip_eol(b: &[u8], mut p: usize) -> usize {
    if p < b.len() && b[p] == b'\r' {
        p += 1;
        if p < b.len() && b[p] == b'\n' {
            p += 1;
        }
    } else if p < b.len() && b[p] == b'\n' {
        p += 1;
    }
    p
}

/// One INPUT # item: quoted strings, comma/line separated; numbers also end at a space.
fn read_item(f: &mut Reader, numeric: bool) -> R<Vec<u8>> {
    let b = &f.rbuf;
    let mut p = f.rpos;
    let end = |p: usize| p >= b.len() || b[p] == 26;
    while !end(p) && (b[p] == b' ' || (numeric && (b[p] == b'\r' || b[p] == b'\n'))) {
        p += 1;
    }
    if end(p) {
        return e(62);
    }
    let mut item = Vec::new();
    if b[p] == b'"' {
        p += 1;
        while !end(p) && b[p] != b'"' {
            item.push(b[p]);
            p += 1;
        }
        if !end(p) {
            p += 1;
        }
        while !end(p) && b[p] != b',' && b[p] != b'\r' && b[p] != b'\n' {
            p += 1;
        }
    } else {
        while !end(p) && b[p] != b',' && b[p] != b'\r' && b[p] != b'\n' && !(numeric && b[p] == b' ') {
            item.push(b[p]);
            p += 1;
        }
        while item.last() == Some(&b' ') {
            item.pop();
        }
        if numeric {
            while !end(p) && b[p] == b' ' {
                p += 1;
            }
        }
    }
    if !end(p) && b[p] == b',' {
        p += 1;
    } else {
        p = skip_eol(b, p);
    }
    f.rpos = p;
    Ok(item)
}

/// Minimal PRINT USING: literal text, `&`, `!`, `\ \`, and numeric fields `#.#`, `+`, `-`, `$$`, `**`, `,`, `^^^^`.
fn using<'a>(vm: &mut dyn Host<'a>, name: &str, a: &[V]) -> R<Option<V>> {
    let Some(mut u) = vm.rt().using.take() else { return e(5) };
    let ch = u.ch;
    let mut out = Vec::new();
    if name == "USINGEND" {
        // literal text up to the next field (or the end)
        let (lit, _) = scan_literal(&u.fmt, u.pos);
        out.extend_from_slice(&lit);
        vm.rt().write(ch, &out)?;
        return Ok(None);
    }
    let mut wrapped = false;
    let field = loop {
        let (lit, next) = scan_literal(&u.fmt, u.pos);
        out.extend_from_slice(&lit);
        u.pos = next;
        if u.pos >= u.fmt.len() {
            if wrapped || !u.used && u.pos == 0 {
                return e(5);
            }
            wrapped = true;
            u.pos = 0;
            continue;
        }
        break field_at(&u.fmt, u.pos);
    };
    let (kind, len) = field;
    u.used = true;
    let fmt = u.fmt[u.pos..u.pos + len].to_vec();
    u.pos += len;
    match (kind, name) {
        (b'S', "USINGS") => {
            let s = a[0].s();
            match fmt[0] {
                b'!' => out.push(*s.first().unwrap_or(&b' ')),
                b'&' => out.extend_from_slice(s),
                _ => {
                    let n = fmt.len();
                    let mut t = s[..s.len().min(n)].to_vec();
                    t.resize(n, b' ');
                    out.extend_from_slice(&t);
                }
            }
        }
        (b'N', n) if n.starts_with("USINGN.") => {
            out.extend_from_slice(&format_num(&fmt, a[0].f(), suffix_ty(n)));
        }
        _ => return e(13),
    }
    vm.rt().write(ch, &out)?;
    vm.rt().using = Some(u);
    Ok(None)
}

fn is_num_start(f: &[u8], p: usize) -> bool {
    let c = f[p];
    let n = f.get(p + 1).copied().unwrap_or(0);
    let n2 = f.get(p + 2).copied().unwrap_or(0);
    c == b'#'
        || (c == b'.' && n == b'#')
        || (c == b'+' && (n == b'#' || (n == b'.' && n2 == b'#') || n == b'$' || n == b'*'))
        || (c == b'$' && n == b'$')
        || (c == b'*' && n == b'*')
}

/// Returns (literal bytes, position of next field or end).
fn scan_literal(f: &[u8], mut p: usize) -> (Vec<u8>, usize) {
    let mut lit = Vec::new();
    while p < f.len() {
        let c = f[p];
        if c == b'_' && p + 1 < f.len() {
            lit.push(f[p + 1]);
            p += 2;
            continue;
        }
        if c == b'!' || c == b'&' || is_num_start(f, p) {
            break;
        }
        if c == b'\\' {
            if let Some(k) = f[p + 1..].iter().position(|x| *x == b'\\') {
                if f[p + 1..p + 1 + k].iter().all(|x| *x == b' ') {
                    break;
                }
            }
        }
        lit.push(c);
        p += 1;
    }
    (lit, p)
}

fn field_at(f: &[u8], p: usize) -> (u8, usize) {
    match f[p] {
        b'!' | b'&' => (b'S', 1),
        b'\\' => (b'S', f[p + 1..].iter().position(|x| *x == b'\\').unwrap() + 2),
        _ => {
            let mut q = p;
            if f[q] == b'+' {
                q += 1;
            }
            if f[q..].starts_with(b"**$") {
                q += 3;
            } else if f[q..].starts_with(b"**") || f[q..].starts_with(b"$$") {
                q += 2;
            }
            while q < f.len() && (f[q] == b'#' || f[q] == b',') {
                q += 1;
            }
            if q < f.len() && f[q] == b'.' {
                q += 1;
                while q < f.len() && f[q] == b'#' {
                    q += 1;
                }
            }
            if f[q..].starts_with(b"^^^^^") {
                q += 5;
            } else if f[q..].starts_with(b"^^^^") {
                q += 4;
            }
            if q < f.len() && (f[q] == b'+' || f[q] == b'-') && f[p] != b'+' {
                q += 1;
            }
            (b'N', q - p)
        }
    }
}

fn format_num(fmt: &[u8], v: f64, t: Ty) -> Vec<u8> {
    let lead_plus = fmt[0] == b'+';
    let trail = match fmt.last() {
        Some(b'+') if !lead_plus => Some(b'+'),
        Some(b'-') => Some(b'-'),
        _ => None,
    };
    let body: &[u8] = &fmt[usize::from(lead_plus)..fmt.len() - usize::from(trail.is_some())];
    let stars = body.starts_with(b"**");
    let dollar = body.starts_with(b"$$") || body.starts_with(b"**$");
    let comma = body.contains(&b',');
    let exp = body.iter().filter(|c| **c == b'^').count();
    let dot = body.iter().position(|c| *c == b'.');
    let int_w = dot.unwrap_or(body.len() - exp);
    let int_w = body[..int_w].iter().filter(|c| matches!(c, b'#' | b'*' | b'$' | b',')).count();
    let dec = dot.map_or(0, |d| body[d + 1..].iter().filter(|c| **c == b'#').count());
    let neg = v < 0.0;
    let a = v.abs();
    let a = if t == Ty::F32 { f64::from(a as f32) } else { a };
    let mut digits;
    if exp > 0 {
        let p = int_w.saturating_sub(usize::from(!lead_plus && trail.is_none())).max(1);
        let s = format!("{:.*e}", p - 1 + dec, a);
        let (m, ex) = s.split_once('e').unwrap();
        let ex: i32 = ex.parse().unwrap_or(0) - (p as i32 - 1);
        let m: String = m.chars().filter(|c| *c != '.').collect();
        let (ip, fp) = m.split_at(p.min(m.len()));
        digits = ip.to_string();
        if dot.is_some() {
            digits.push('.');
            digits.push_str(fp);
        }
        let w = if exp >= 5 { 3 } else { 2 };
        let letter = match t {
            Ty::F32 => 'E',
            Ty::F80 => 'F',
            _ => 'D',
        };
        digits.push_str(&format!("{letter}{}{:0w$}", if ex < 0 { '-' } else { '+' }, ex.abs(), w = w));
    } else {
        digits = format!("{:.*}", dec, a);
        if dec == 0 && dot.is_some() {
            digits.push('.');
        }
        if digits.starts_with("0.") && int_w == 0 {
            digits.remove(0);
        }
        if comma {
            let (ip, rest) = digits.split_at(digits.find('.').unwrap_or(digits.len()));
            let mut g = String::new();
            for (k, c) in ip.chars().enumerate() {
                if k > 0 && (ip.len() - k) % 3 == 0 {
                    g.push(',');
                }
                g.push(c);
            }
            digits = g + rest;
        }
    }
    let mut s = String::new();
    if dollar {
        s.push('$');
    }
    s.push_str(&digits);
    if lead_plus {
        // the sign takes the first column of the field; the number is right-aligned after it
        let width = fmt.len() - 1;
        let body = s.into_bytes();
        let mut r = vec![if neg { b'-' } else { b'+' }];
        if body.len() > width {
            let mut o = vec![b'%'];
            o.extend_from_slice(&r);
            o.extend_from_slice(&body);
            return o;
        }
        r.extend(std::iter::repeat_n(if stars { b'*' } else { b' ' }, width - body.len()));
        r.extend_from_slice(&body);
        return r;
    } else if neg && trail.is_none() {
        s.insert(0, '-');
    }
    if let Some(tc) = trail {
        s.push(if neg { '-' } else if tc == b'+' { '+' } else { ' ' });
    }
    let width = fmt.len();
    let mut out = s.into_bytes();
    if out.len() > width {
        let mut o = vec![b'%'];
        o.extend_from_slice(&out);
        return o;
    }
    let fillc = if stars { b'*' } else { b' ' };
    let mut r = vec![fillc; width - out.len()];
    r.append(&mut out);
    r
}
