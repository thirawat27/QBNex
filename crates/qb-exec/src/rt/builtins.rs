//! `rt` builtins (table: `qb_ir::builtins`), called by the VM through [`Host`].

use super::{cvt, e, Stop, R, V};
use qb_ir::Ty;
use std::collections::HashMap;
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
    /// ERL: last numeric line label executed before the error
    fn erl(&self) -> u32;
    /// _ERRORLINE: source line of the error
    fn errline(&self) -> u32;
    /// LASTLINE builtin: a numeric line label was passed
    fn set_last_line(&mut self, n: u32);
    /// `len` bytes of memory at the address `p` (GET/PUT, _MEM, TCP buffers).
    fn bytes(&mut self, p: u64, len: usize) -> R<&mut [u8]>;
    /// (address of the first element, total bytes) of the array whose descriptor `p` references.
    fn array_extent(&mut self, p: u64) -> R<(u64, u64)>;
    /// Value of the string variable whose storage is at `p`.
    fn load_str(&mut self, p: u64) -> R<Vec<u8>>;
    /// Assigns the string variable whose storage is at `p` (runs `str_store_hook`).
    fn store_str(&mut self, p: u64, s: &[u8]) -> R<()>;
}

/// Bookkeeping every executor runs before a string is stored into memory at
/// `p`: a plain assignment breaks a FIELD link, and a string whose descriptor
/// was exposed with VARPTR/SADD gets its length updated in DBLOCK.
pub(crate) fn str_store_hook(rt: &mut RtState<'_>, p: u64, len: usize) {
    if !rt.field_vars.is_empty() && !rt.field_store {
        rt.field_vars.retain(|f| f.0 != p);
    }
    if let Some(&d) = rt.cmem_strs.get(&p) {
        let l = (len as u16).to_le_bytes();
        rt.cmem[DBLOCK + d..DBLOCK + d + 2].copy_from_slice(&l);
    }
}

/// Byte offset of the DBLOCK segment (0x1000) in conventional memory.
pub(crate) const DBLOCK: usize = 0x1000 * 16;

pub(crate) struct RtState<'a> {
    out: &'a mut dyn Write,
    col: usize,
    rnd: qb_runtime::rnd::RndState,
    using: Option<Using>,
    stdin: Reader,
    /// the stdin line the console INPUT statement being run takes its values
    /// from (read by its first item; INPUTPROMPT starts a new statement)
    input_line: Option<Reader>,
    /// conventional memory (VARPTR/SADD/PEEK/POKE)
    pub(crate) cmem: Vec<u8>,
    pub(crate) cmem_next: usize,
    /// string variable address -> DBLOCK offset of its descriptor
    pub(crate) cmem_strs: HashMap<u64, usize>,
    /// DEF SEG base (bytes); None = DBLOCK
    pub(crate) def_seg: Option<usize>,
    /// set while LSETVAR/GETFIELD store into FIELD variables
    pub(crate) field_store: bool,
    /// RANDOM record length per open file number (FIELD buffers)
    pub(crate) reclen: HashMap<i32, usize>,
    /// FIELD buffer per file number and the variables linked to it (address, offset, size)
    pub(crate) field_buf: HashMap<i32, Vec<u8>>,
    pub(crate) field_vars: Vec<(u64, i32, usize, usize)>,
    /// FIELD statement in progress: (file number, bytes used)
    pub(crate) field_cur: (i32, usize),
    /// TCP/IP handles
    pub(crate) net: super::netx::Net,
    pub(crate) net_pending_error: i32,
    /// keyboard and mouse (INKEY$, _KEYHIT, _MOUSEINPUT, ...)
    pub(crate) input: super::input::Input,
    /// _LIMIT: time of the previous frame
    pub(crate) limit_last: Option<i64>,
    /// VARPTR$ value slots in DBLOCK (offset, next slot)
    pub(crate) varptr_slots: usize,
    pub(crate) varptr_next: usize,
    /// SADD character areas in DBLOCK: string variable -> (offset, capacity)
    pub(crate) cmem_sdata: HashMap<u64, (usize, usize)>,
    /// CALL ABSOLUTE argument offsets being collected (ABSARG.*)
    pub(crate) abs_args: Vec<u16>,
    /// error of the last numeric READ, raised by READCHK after the store
    pub(crate) read_err: i32,
    /// screen pages and images (SCREEN, graphics, `_NEWIMAGE`, ...)
    pub(crate) display: super::display::Display,
    /// `$CONSOLE` / `$CONSOLE:ONLY`
    pub(crate) console_on: bool,
    pub(crate) console_only: bool,
    /// qb-audio started (snd_init)
    pub(crate) snd_started: bool,
    /// browser runner (qb-web): INPUT prompts and the consumed stdin text are
    /// drawn on the screen page, as if typed; `_DELAY`, `_LIMIT` and `SLEEP n`
    /// show the frame drawn so far, and `SLEEP n` waits
    pub(crate) web: bool,
    /// GUI program with a window (rt/window.rs): PRINT draws on the screen page,
    /// INPUT reads typed keys (drawn as typed), SLEEP waits for a key
    pub(crate) window: bool,
    /// browser runner: ticks of the last frame shown while polling input
    pub(crate) web_frame_at: i64,
    /// browser runner: stdin bytes before this offset were typed (and drawn) live
    pub(crate) web_typed_end: usize,
}

struct Using {
    ch: i32,
    fmt: Vec<u8>,
    pos: usize,
    used: bool,
}

impl<'a> RtState<'a> {
    pub(crate) fn new(out: &'a mut dyn Write) -> Self {
        RtState { out, col: 1, rnd: Default::default(), using: None, stdin: Reader::default(),
            input_line: None,
            // 1 MB + 64 KB like libqb cmem (SS:SP of CALL ABSOLUTE is FFFF:FFFE)
            cmem: vec![0; 0x110000],
            cmem_next: 16,
            cmem_strs: HashMap::new(),
            def_seg: None,
            field_store: false,
            reclen: HashMap::new(),
            field_buf: HashMap::new(),
            field_vars: Vec::new(),
            field_cur: (0, 0),
            net: super::netx::Net::default(),
            net_pending_error: 0,
            snd_started: false,
            read_err: 0,
            cmem_sdata: HashMap::new(),
            abs_args: Vec::new(),
            input: super::input::Input::default(),
            limit_last: None,
            varptr_slots: 0,
            varptr_next: 0,
            display: super::display::Display::default(),
            console_on: false,
            console_only: false,
            web: false,
            window: false,
            web_frame_at: 0,
            web_typed_end: 0,
        }
    }
    /// PRINT and INPUT happen on the screen page (browser runner or window).
    pub(crate) fn screen_console(&self) -> bool {
        self.web || self.window
    }
    /// Text INPUT reads instead of the process stdin (browser runner).
    pub(crate) fn set_stdin(&mut self, b: &[u8]) {
        self.stdin = Reader { rbuf: b.to_vec(), rpos: 0, eof: true };
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

/// True when PRINT to the screen must draw on a page instead of writing stdout.
fn screen_out(vm: &mut dyn Host<'_>) -> bool {
    let rt = vm.rt();
    if !rt.display.started {
        // PRINT alone starts the screen only in a `$CONSOLE` program (its window page);
        // without the meta (hand-written QIR, no window here) it stays on stdout
        if rt.console_only || !rt.console_on {
            return false;
        }
        let cmem = rt.cmem.as_mut_ptr();
        let (on, only) = (rt.console_on, rt.console_only);
        rt.display.start(cmem, on, only);
    }
    rt.display.write_is_screen()
}

pub(crate) fn val(s: &[u8]) -> f64 {
    qb_runtime::val::val(s).map(qb_runtime::val::f64_from_f80).unwrap_or(0.0)
}

fn str_num(t: Ty, v: &V) -> Vec<u8> {
    match t {
        Ty::U64 => numfmt::str_u64(v.i() as u64),
        Ty::F32 => numfmt::str_f32(v.f() as f32),
        Ty::F64 => numfmt::str_f64(v.f()),
        Ty::F80 => numfmt::str_f80(v.x().to_bytes()),
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
    if let Some(r) = super::display::call(vm, name, &a) {
        return r;
    }
    let is_print = matches!(name, "PRINTS" | "PRINTZONE" | "PRINTNL" | "PRINTTAB" | "PRINTSPC") || name.starts_with("PRINTN.");
    if is_print && a.first().is_some_and(|c| c.i() == 0) && screen_out(vm) {
        // console output while a screen page is the destination: draw it
        let r = match name {
            "PRINTS" => Some(super::display::call(vm, "SCREENPRINT", &a[1..])),
            "PRINTZONE" => Some(super::display::call(vm, "SCREENZONE", &[])),
            "PRINTNL" => Some(super::display::call(vm, "SCREENNL", &[])),
            "PRINTTAB" => Some(super::display::call(vm, "SCREENTAB", &a[1..])),
            "PRINTSPC" => Some(super::display::call(vm, "SCREENSPC", &a[1..])),
            n if n.starts_with("PRINTN.") => {
                let mut b = str_num(suffix_ty(n), &a[1]);
                b.push(b' ');
                Some(super::display::call(vm, "SCREENPRINT", &[V::S(b)]))
            }
            _ => None,
        };
        if let Some(Some(r)) = r {
            return r;
        }
    }
    if let Some(r) = super::audiox::call(vm, name, &a) {
        return r;
    }
    if let Some(r) = super::scene3d::call(vm, name, &a) {
        return r;
    }
    if let Some(r) = super::netx::call(vm, name, &a) {
        return r;
    }
    if let Some(r) = super::files::call(vm, name, &a) {
        return r;
    }
    if let Some(r) = super::memx::call(vm, name, &a) {
        return r;
    }
    if (name == "POS" || name == "CSRLIN") && screen_out(vm) {
        return super::display::call(vm, if name == "POS" { "func_pos" } else { "func_csrlin" }, &[]).unwrap();
    }
    if let Some(r) = input_call(vm, name, &a) {
        return r;
    }
    if name == "VARPTR$" {
        // VARPTR$ of a numeric variable for DRAW/PLAY "=": the value goes to one of
        // 16 rotating DBLOCK slots (the program reads it in the same statement)
        let (ty, v) = (a[0].i() as i32, a[1].f());
        let rt = vm.rt();
        if rt.varptr_slots == 0 {
            rt.varptr_slots = rt.cmem_next;
            rt.cmem_next += 16 * 16;
        }
        let off = rt.varptr_slots + 16 * (rt.varptr_next % 16);
        rt.varptr_next += 1;
        let b = DBLOCK + off;
        let m = &mut rt.cmem[b..b + 16];
        match ty {
            1 | 129 => m[0] = v as i64 as u8,
            2 | 130 => m[..2].copy_from_slice(&(v as i64 as u16).to_le_bytes()),
            20 | 148 => m[..4].copy_from_slice(&(v as i64 as u32).to_le_bytes()),
            5 => m[..8].copy_from_slice(&(v as i64).to_le_bytes()),
            133 => m[..8].copy_from_slice(&(v as u64).to_le_bytes()),
            4 => m[..4].copy_from_slice(&(v as f32).to_le_bytes()),
            6 => m[..10].copy_from_slice(&qb_float80::F80::from_f64(v).to_bytes()),
            _ => m[..8].copy_from_slice(&v.to_le_bytes()),
        }
        return Ok(Some(V::S(vec![ty as u8, off as u8, (off >> 8) as u8])));
    }
    if name == "_COMMANDCOUNT" {
        return Ok(Some(V::I(std::env::args().count().saturating_sub(1) as i64)));
    }
    if name == "_OS$" {
        // func__os
        let bits = if cfg!(target_pointer_width = "32") { "[32BIT]" } else { "[64BIT]" };
        let os = if cfg!(windows) {
            format!("[WINDOWS]{bits}")
        } else if cfg!(target_os = "macos") {
            format!("[MACOSX]{bits}[LINUX]")
        } else if cfg!(target_os = "linux") {
            format!("[LINUX]{bits}")
        } else {
            bits.to_string()
        };
        return Ok(Some(V::S(os.into_bytes())));
    }    if name == "READCHK" {
        let c = std::mem::take(&mut vm.rt().read_err);
        return if c != 0 { e(c) } else { Ok(None) };
    }
    if name == "func__errormessage" {
        // _ERRORMESSAGE$[(code)]: without the argument, the current error
        let code = if a[1].i() != 0 { a[0].i() as i32 } else { vm.err() };
        return Ok(Some(V::S(qb_runtime::error::human_error(code).as_bytes().to_vec())));
    }
    if name == "LASTLINE" {
        vm.set_last_line(a[0].i() as u32);
        return Ok(None);
    }
    if name == "VARPTR.s" || name == "SADD" {
        let p = a[0].p();
        let known = vm.rt().cmem_strs.get(&p).copied();
        let d = match known {
            Some(d) => d,
            None => {
                let len = vm.load_str(p)?.len() as u16;
                let rt = vm.rt();
                let d = rt.cmem_next;
                rt.cmem_next += 4;
                rt.cmem_strs.insert(p, d);
                let base = DBLOCK + d;
                rt.cmem[base..base + 2].copy_from_slice(&len.to_le_bytes());
                d
            }
        };
        if name == "SADD" {
            // the characters are copied into DBLOCK too (CALL ABSOLUTE runs code from there)
            let s = vm.load_str(p)?;
            let rt = vm.rt();
            let (off, cap) = rt.cmem_sdata.get(&p).copied().unwrap_or((0, 0));
            let off = if s.len() <= cap && off != 0 {
                off
            } else if rt.cmem_next == d + 4 && cap == 0 {
                let o = d + 4;
                rt.cmem_next = o + s.len().max(1);
                rt.cmem_sdata.insert(p, (o, s.len().max(1)));
                o
            } else {
                let o = rt.cmem_next;
                rt.cmem_next = o + s.len().max(1);
                rt.cmem_sdata.insert(p, (o, s.len().max(1)));
                o
            };
            if DBLOCK + off + s.len() > rt.cmem.len() {
                return e(7);
            }
            rt.cmem[DBLOCK + off..DBLOCK + off + s.len()].copy_from_slice(&s);
            rt.cmem[DBLOCK + d + 2..DBLOCK + d + 4].copy_from_slice(&(off as u16).to_le_bytes());
            return Ok(Some(V::I(off as i64)));
        }
        return Ok(Some(V::I(d as i64)));
    }
    if let Some(ty) = name.strip_prefix("ABSARG.") {
        // CALL ABSOLUTE argument: the value in a DBLOCK slot (copied back by ABSGET)
        let v = a[0].clone();
        let rt = vm.rt();
        let size = if ty == "i16" { 2 } else if ty == "i32" || ty == "f32" { 4 } else { 8 };
        let off = rt.cmem_next;
        rt.cmem_next += size;
        let b = DBLOCK + off;
        match ty {
            "i16" => rt.cmem[b..b + 2].copy_from_slice(&(v.i() as i16).to_le_bytes()),
            "i32" => rt.cmem[b..b + 4].copy_from_slice(&(v.i() as i32).to_le_bytes()),
            "f32" => rt.cmem[b..b + 4].copy_from_slice(&(v.f() as f32).to_le_bytes()),
            "f64" => rt.cmem[b..b + 8].copy_from_slice(&v.f().to_le_bytes()),
            _ => rt.cmem[b..b + 8].copy_from_slice(&v.i().to_le_bytes()),
        }
        rt.abs_args.push(off as u16);
        return Ok(Some(V::I(off as i64)));
    }
    if let Some(ty) = name.strip_prefix("ABSGET.") {
        let off = a[0].i() as usize;
        let rt = vm.rt();
        let b = DBLOCK + off;
        let m = &rt.cmem;
        let v = match ty {
            "i16" => V::I(i64::from(i16::from_le_bytes([m[b], m[b + 1]]))),
            "i32" => V::I(i64::from(i32::from_le_bytes(m[b..b + 4].try_into().unwrap()))),
            "f32" => V::F(f64::from(f32::from_le_bytes(m[b..b + 4].try_into().unwrap()))),
            "f64" => V::F(f64::from_le_bytes(m[b..b + 8].try_into().unwrap())),
            _ => V::I(i64::from_le_bytes(m[b..b + 8].try_into().unwrap())),
        };
        return Ok(Some(v));
    }
    if name == "ABSCALL" {
        // call_absolute: code at DEF SEG:offset, data segment = DBLOCK
        let rt = vm.rt();
        let args = std::mem::take(&mut rt.abs_args);
        let seg = rt.def_seg.unwrap_or(DBLOCK);
        super::cpu::call_absolute(&mut rt.cmem, seg, (DBLOCK >> 4) as u16, a[0].i() as u16, &args).or_else(|c| e(c))?;
        return Ok(None);
    }
    let (err, erl, errline) = (vm.err(), vm.erl(), vm.errline());
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
            if ch == 0 {
                // screen/stdout: QBasic's 14-column zones in 80 columns, a new line
                // when no whole zone is left (same rule as qb-runtime's console)
                match qb_runtime::console::text::zone_spaces(col as i32, 80) {
                    Some(n) => rt.write(ch, &vec![b' '; n as usize])?,
                    None => rt.nl(ch)?,
                }
            } else {
                let next = ((col - 1) / 14 + 1) * 14 + 1;
                rt.write(ch, &vec![b' '; next - col])?
            }
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
        "VAL.f80" => {
            let b = qb_runtime::val::val(s(0)).map_err(Stop::Err)?;
            return ok(V::X(qb_float80::F80::from_bytes(b)));
        }
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
        "ABS.f32" | "ABS.f64" => return ok(V::F(f(0).abs())),
        "ABS.f80" => return ok(V::X(a[0].x().abs())),
        "SGN.i64" => return ok(V::I(i(0).signum())),
        "SGN.f80" => {
            use std::cmp::Ordering::*;
            let z = qb_float80::F80::from_i64(0);
            return ok(V::I(match a[0].x().compare(z) {
                Some(Greater) => 1,
                Some(Less) => -1,
                _ => 0,
            }));
        }
        "SGN.f64" => {
            let x = f(0);
            return ok(V::I(if x > 0.0 { 1 } else if x < 0.0 { -1 } else { 0 }));
        }
        "INT.f32" => return ok(V::F(f64::from((f(0) as f32).floor()))),
        "INT.f64" => return ok(float1(&a[0], f64::floor)),
        "FIX.f32" | "FIX.f64" => return ok(float1(&a[0], f64::trunc)),
        "INT.f80" => return ok(V::X(qb_float80::qb::int_float(a[0].x()))),
        "FIX.f80" => return ok(V::X(qb_float80::qb::fix_float(a[0].x()))),
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
        "EXP.s" => return match qb_runtime::mathfn::exp_single(f(0)) {
            Ok(v) => ok(V::F(v)),
            Err(c) => e(c),
        },
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
            let t = qb_runtime::systime::unix_time() % 86400.0;
            let acc = f(0);
            return ok(V::F(if acc > 0.0 { (t / acc).round() * acc } else { t }));
        }
        "TIMER" => {
            let t = qb_runtime::systime::unix_time();
            return ok(V::F(t % 86400.0));
        }
        "ERR" => return ok(V::I(err.into())),
        "ERL" => return ok(V::I(erl.into())),
        "_ERRORLINE" => return ok(V::I(errline.into())),
        "LINEINPUT" => {
            let ch = i(0) as i32;
            let _ = ch;
            if !rt.window {
                fill(&mut std::io::stdin(), &mut rt.stdin);
            }
            web_type_line(vm, false)?;
            let rt = vm.rt();
            let start = rt.stdin.rpos;
            let line = console_line(rt)?;
            echo_input(vm, start, true)?;
            return ok(V::S(line));
        }
        "INPUT$" => {
            // INPUT$(n) from the keyboard: keys of the window, or stdin bytes on the
            // console (none left at its end: what was read, like INPUT's empty line)
            let n = i(0);
            if n < 0 {
                return e(5);
            }
            let n = n as usize;
            if rt.display.started {
                let mut got = Vec::new();
                while got.len() < n {
                    let rt = vm.rt();
                    let cm = &mut rt.cmem[..];
                    let k = rt.input.inkey(cm);
                    if k.is_empty() {
                        // the window keeps showing; closing it ends the program
                        idle_frame(rt);
                        qb_runtime::systime::sleep_ms(10);
                    }
                    got.extend_from_slice(&k);
                }
                return ok(V::S(got));
            }
            fill(&mut std::io::stdin(), &mut rt.stdin);
            let s = &mut rt.stdin;
            let end = (s.rpos + n).min(s.rbuf.len());
            let got = s.rbuf[s.rpos..end].to_vec();
            s.rpos = end;
            return ok(V::S(got));
        }
        "INPUTPROMPT" => {
            if i(1) == 0 {
                rt.input_line = None;
                if rt.screen_console() && screen_out(vm) {
                    return super::display::call(vm, "SCREENPRINT", &a[..1]).unwrap();
                }
                vm.rt().write(0, s(0))?;
            }
        }
        "INPUTITEM.s" | "INPUTITEM.n" => {
            let ch = i(0) as i32;
            let _ = ch;
            let numeric = name == "INPUTITEM.n";
            if rt.input_line.is_none() {
                // first item: the statement reads one line; its values come from
                // that line only (missing ones are empty / 0, as in the C++ backend)
                if !rt.window {
                    fill(&mut std::io::stdin(), &mut rt.stdin);
                }
                web_type_line(vm, numeric)?;
                let rt = vm.rt();
                let start = rt.stdin.rpos;
                let line = console_line(rt)?;
                rt.input_line = Some(Reader { rbuf: line, rpos: 0, eof: true });
                echo_input(vm, start, true)?;
            }
            let rt = vm.rt();
            let line = rt.input_line.as_mut().expect("read above");
            // past the end of the line: an empty value
            let item = read_item(line, numeric).unwrap_or_default();
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
        "SLEEP" => {
            // with a window: n seconds or until a key is pressed (no time: until a key)
            #[cfg(not(target_arch = "wasm32"))]
            if rt.window {
                let ms = a.first().map(|v| v.f()).filter(|&s| s > 0.0).map(|s| (s * 1000.0).min(86_400_000.0) as i64);
                let until = ms.map(|m| qb_runtime::systime::ticks() + m);
                let seen = super::input::KEY_PRESSES.load(std::sync::atomic::Ordering::Relaxed);
                while until.is_none_or(|u| qb_runtime::systime::ticks() < u)
                    && super::input::KEY_PRESSES.load(std::sync::atomic::Ordering::Relaxed) == seen
                {
                    idle_frame(rt);
                    qb_runtime::systime::sleep_ms(10);
                }
                return Ok(None);
            }
            // waits only in the browser runner; SLEEP without a time (wait for a key) returns
            if rt.web && a.first().is_some_and(|v| v.f() > 0.0) {
                if rt.display.started && rt.display.autodisplay != 0 {
                    rt.display.capture(qb_runtime::gfx::capture::KIND_DISPLAY);
                }
                qb_runtime::systime::sleep_ms((a[0].f() * 1000.0).min(86_400_000.0) as u64);
            }
        }
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
        "_DEFLATE$" => {
            let src = s(0);
            let mut cap = qb_runtime::compress::qb_deflate_bound(src.len() as u32);
            let mut dst = vec![0u8; cap as usize];
            // SAFETY: buffers of the given sizes.
            unsafe { qb_runtime::compress::qb_deflate(src.as_ptr(), src.len() as u32, dst.as_mut_ptr(), &mut cap) };
            dst.truncate(cap as usize);
            return ok(V::S(dst));
        }
        "_INFLATE$" | "_INFLATE$.n" => {
            // legacy compress_bridge.cpp: a given size is used as is, otherwise 10 MB steps
            let src = s(0);
            let mut size: u64 = if name == "_INFLATE$.n" { i(1).max(0) as u64 } else { 0 };
            loop {
                if name == "_INFLATE$" {
                    size += 10_000_000;
                }
                let mut out_len = size as u32;
                let mut dst = vec![0u8; size as usize];
                // SAFETY: buffers of the given sizes.
                let r = unsafe { qb_runtime::compress::qb_inflate(src.as_ptr(), src.len() as u32, dst.as_mut_ptr(), &mut out_len) };
                if name == "_INFLATE$" && r == -5 {
                    continue;
                }
                dst.truncate(out_len as usize);
                return ok(V::S(dst));
            }
        }
        "VARSEG" => return ok(V::I(0x1000)),
        "DEFSEG" => rt.def_seg = Some((i(0) as u16 as usize) * 16),
        "DEFSEG.d" => rt.def_seg = None,
        "PEEK" | "POKE" => {
            let at = rt.def_seg.unwrap_or(DBLOCK) + (i(0) as u16 as usize);
            let at = at % rt.cmem.len();
            if name == "POKE" {
                rt.cmem[at] = i(1) as u8;
            } else {
                return ok(V::I(rt.cmem[at].into()));
            }
        }
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

/// Browser runner: draws the stdin bytes INPUT consumed since `start` on the
/// screen page (line breaks as newlines), like a typed answer.
fn echo_input(vm: &mut dyn Host<'_>, mut start: usize, mut line: bool) -> R<()> {
    let rt = vm.rt();
    if !rt.screen_console() {
        return Ok(());
    }
    if start < rt.web_typed_end {
        // typed live: already on the screen, with its newline
        start = rt.web_typed_end;
        line = false;
    }
    let mut text = rt.stdin.rbuf[start.min(rt.stdin.rpos)..rt.stdin.rpos].to_vec();
    if line && text.last() != Some(&b'\n') {
        text.push(b'\n');
    }
    if !screen_out(vm) {
        return Ok(());
    }
    for (k, seg) in text.split(|&c| c == b'\n').enumerate() {
        if k > 0 {
            if let Some(r) = super::display::call(vm, "SCREENNL", &[]) {
                r?;
            }
        }
        let seg: Vec<u8> = seg.iter().copied().filter(|&c| c != b'\r').collect();
        if !seg.is_empty() {
            if let Some(r) = super::display::call(vm, "SCREENPRINT", &[V::S(seg)]) {
                r?;
            }
        }
    }
    Ok(())
}

/// /// Browser runner / window: when INPUT has used up the given stdin text and
/// keys arrive live (the page, the window), waits for a typed line (drawn as
/// it is typed; Backspace edits, Enter ends) and appends it to the stdin text.
fn web_type_line(vm: &mut dyn Host<'_>, numeric: bool) -> R<()> {
    {
        let rt = vm.rt();
        if !typed_input(rt) || rt.stdin.rpos < rt.stdin.rbuf.len() {
            return Ok(());
        }
    }
    let mut line: Vec<u8> = Vec::new();
    let draw = |vm: &mut dyn Host<'_>, b: &[u8]| -> R<()> {
        if screen_out(vm) {
            if let Some(r) = super::display::call(vm, "SCREENPRINT", &[V::S(b.to_vec())]) {
                r?;
            }
        }
        Ok(())
    };
    loop {
        let k = {
            let rt = vm.rt();
            let cm = &mut rt.cmem[..];
            rt.input.inkey(cm)
        };
        match k.as_slice() {
            [] => {
                idle_frame(vm.rt());
                qb_runtime::systime::sleep_ms(15);
            }
            [13] => break,
            [8] => {
                if line.pop().is_some() {
                    // cursor left, blank, cursor left (PRINT control characters)
                    draw(vm, &[29, b' ', 29])?;
                }
            }
            [c] if *c >= 32 => {
                line.push(*c);
                draw(vm, &[*c])?;
            }
            _ => {}
        }
    }
    if screen_out(vm) {
        if let Some(r) = super::display::call(vm, "SCREENNL", &[]) {
            r?;
        }
    }
    if numeric && line.is_empty() {
        // an empty answer to a numeric INPUT is 0, as in QBasic
        line.push(b'0');
    }
    let rt = vm.rt();
    rt.stdin.rbuf.extend_from_slice(&line);
    rt.stdin.rbuf.push(b'\n');
    rt.web_typed_end = rt.stdin.rbuf.len();
    Ok(())
}

/// INPUT reads keys typed live (the page delivers them, or the window).
fn typed_input(rt: &RtState<'_>) -> bool {
    #[cfg(target_arch = "wasm32")]
    return rt.web && qb_runtime::systime::web::interactive();
    #[cfg(not(target_arch = "wasm32"))]
    return rt.window;
}

/// While the program waits (typed INPUT, SLEEP): the screen keeps showing,
/// and a closed window ends the program.
pub(crate) fn idle_frame(rt: &mut RtState<'_>) {
    if !rt.display.started || rt.display.autodisplay == 0 {
        #[cfg(not(target_arch = "wasm32"))]
        if rt.window && super::window::closed() {
            close_exit(rt);
        }
        return;
    }
    #[cfg(target_arch = "wasm32")]
    rt.display.capture(qb_runtime::gfx::capture::KIND_DISPLAY);
    #[cfg(not(target_arch = "wasm32"))]
    if rt.window {
        if super::window::closed() {
            close_exit(rt);
        }
        let now = qb_runtime::systime::ticks();
        if now - rt.web_frame_at >= 32 {
            rt.web_frame_at = now;
            super::window::present(&rt.display);
        }
    }
}

/// The window was closed while the program waits: end it (`end_program`).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn close_exit(rt: &mut RtState<'_>) -> ! {
    rt.flush();
    rt.display.capture(qb_runtime::gfx::capture::KIND_END);
    qb_runtime::fileio::close_all();
    std::process::exit(0)
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

/// One line of console input. At the end of stdin the console has no end of
/// file: the line is empty, as the C++ backend's INPUT never sees an end
/// either (the browser runner keeps error 62, it has no more keys to wait for).
fn console_line(rt: &mut RtState<'_>) -> R<Vec<u8>> {
    let s = &rt.stdin;
    if !rt.web && (s.rpos >= s.rbuf.len() || s.rbuf[s.rpos] == 26) {
        return Ok(Vec::new());
    }
    read_line(&mut rt.stdin)
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
            out.extend_from_slice(&format_num(&fmt, &a[0], suffix_ty(n)));
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

fn format_num(fmt: &[u8], val: &V, t: Ty) -> Vec<u8> {
    let v = val.f();
    let x80 = (t == Ty::F80).then(|| val.x().abs());
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
        let s = match x80 {
            Some(x) => qb_float80::dec::format_e(x, p - 1 + dec),
            None => format!("{:.*e}", p - 1 + dec, a),
        };
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
        digits = match x80 {
            Some(x) => qb_float80::dec::format_f(x, dec),
            None => format!("{:.*}", dec, a),
        };
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

/// Keyboard, mouse and frame-rate builtins (rt/input.rs).
fn input_call<'a>(vm: &mut dyn Host<'a>, name: &str, a: &[V]) -> Option<R<Option<V>>> {
    let rt = vm.rt();
    if rt.web && matches!(name, "INKEY$" | "qbs_inkey" | "func__keyhit" | "func__keydown" | "func__mouseinput") {
        // browser runner: a program polling the keyboard in a loop still shows its frames
        let now = qb_runtime::systime::ticks();
        if now - rt.web_frame_at >= 33 && rt.display.started && rt.display.autodisplay != 0 {
            rt.web_frame_at = now;
            rt.display.capture(qb_runtime::gfx::capture::KIND_DISPLAY);
        }
    }
    let cm = &mut rt.cmem[..];
    let v = match name {
        "INKEY$" | "qbs_inkey" => V::S(rt.input.inkey(cm)),
        "func__keyhit" => {
            rt.input.pump(cm);
            V::I(i64::from(qb_runtime::keybuf::qb_keyhit_pop()))
        }
        "func__keydown" => {
            rt.input.pump(cm);
            let x = a[0].i();
            if x <= 0 {
                return Some(e(5));
            }
            V::I(if qb_runtime::keybuf::qb_keyheld(x as u32) != 0 { -1 } else { 0 })
        }
        "func__mouseinput" => {
            rt.input.pump(cm);
            V::I(if rt.input.mouseinput() { -1 } else { 0 })
        }
        "func__mousebutton" => {
            rt.input.pump(cm);
            let i = a[0].i();
            if i < 1 {
                return Some(e(5));
            }
            V::I(if rt.input.mousebutton(i as i32) { -1 } else { 0 })
        }
        "func__mousewheel" => {
            rt.input.pump(cm);
            V::I(i64::from(rt.input.mousewheel()))
        }
        "func__mousex" | "func__mousey" => {
            rt.input.pump(cm);
            let (x, y) = rt.input.mouse_xy();
            // headless window = the display page at scale 1: clamp to the page
            let (w, h) = rt.display.display_size();
            let v = if name == "func__mousex" { x.clamp(0, (w - 1).max(0)) } else { y.clamp(0, (h - 1).max(0)) };
            V::F(f64::from(v))
        }
        "func__freetimer" => V::I(i64::from(qb_runtime::events::timers().alloc())),
        "sub_timer" => {
            // TIMER[(n)] ON|OFF|STOP|FREE: (n, option 1..4, passed)
            let i = if a[2].i() == 0 { 0 } else { a[0].i() as i32 };
            if let Err(c) = qb_runtime::events::timers().statement(i, a[1].i() as i32) {
                return Some(e(c));
            }
            return Some(Ok(None));
        }
        "sub_key" => {
            // KEY(n) ON|OFF|STOP: ON KEY trapping is not handled; the range is checked
            if !(0..=31).contains(&a[0].i()) {
                return Some(e(5));
            }
            return Some(Ok(None));
        }
        "DELAYBEGIN" => {
            // _DELAY s: the deadline in ms; the emitter loops on DELAYSTEP so events run
            let s = a[0].f();
            if !(0.0..=2147483.647).contains(&s) {
                return Some(e(5));
            }
            if rt.web && rt.display.started && rt.display.autodisplay != 0 {
                rt.display.capture(qb_runtime::gfx::capture::KIND_DISPLAY);
            }
            V::F(qb_runtime::systime::ticks() as f64 + s * 1000.0)
        }
        "DELAYSTEP" => {
            // sleeps at most 9 ms; 1 while the deadline is ahead
            rt.input.pump(cm);
            let left = a[0].f() - qb_runtime::systime::ticks() as f64;
            if left <= 0.0 {
                V::I(0)
            } else {
                qb_runtime::systime::sleep_ms(left.clamp(1.0, 9.0) as u64);
                V::I(1)
            }
        }
        "sub__limit" => {
            // _LIMIT fps: sleep until 1/fps after the previous frame
            let fps = a[0].f();
            rt.input.pump(cm);
            if fps <= 0.0 || 1000.0 / fps > 60000.0 {
                return Some(e(5));
            }
            if fps > 0.0 {
                // browser runner: the frame drawn so far is shown before the wait
                if rt.web && rt.display.started && rt.display.autodisplay != 0 {
                    rt.display.capture(qb_runtime::gfx::capture::KIND_DISPLAY);
                }
                // monotonic ms (qb_runtime::systime also serves the browser build)
                let frame = (1000.0 / fps) as i64;
                let now = qb_runtime::systime::ticks();
                if let Some(last) = rt.limit_last {
                    let due = last + frame;
                    if due > now {
                        qb_runtime::systime::sleep_ms((due - now) as u64);
                    }
                }
                rt.limit_last = Some(qb_runtime::systime::ticks());
            }
            return Some(Ok(None));
        }
        _ => return None,
    };
    Some(Ok(Some(v)))
}
