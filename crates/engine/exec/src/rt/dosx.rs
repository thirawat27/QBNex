//! DOS-era statements as QB64 has them (`sub_out`, `func_inp`, `sub_wait`,
//! `sub_bload`, `sub_bsave`, `sub_run`, `sub_chain`), `VARPTR` of numeric
//! variables and the
//! RUN / CLEAR requests the VM carries out (`Vm::restart`).
//!
//! QIR variables do not live in conventional memory. `VARPTR` of a numeric
//! variable or array element mirrors its bytes in DBLOCK (the segment
//! `VARSEG` returns): PEEK, BSAVE and CALL ABSOLUTE see the value at the
//! time they run, and POKE / BLOAD into an array mirror are written back to
//! the array (a scalar mirror is a copy only: a SUB's local may be gone).
//!
//! CHAIN passes the COMMON variables by position, as QBasic does: CHAINPUT
//! collects them, CHAIN writes them to a file named by `QBNEX_CHAIN` in the
//! environment of the next program, whose COMMON statement reads them back
//! (CHAINGET) in the same order. A value whose kind or size does not match
//! the receiving variable is skipped. COMMON arrays go in the same sequence
//! with their bounds (CHAINPUTA / CHAINGETA): a dynamic array arrives
//! redimensioned to them, a static one of the same size gets the values.
//! A program lowered in register mode does the same through `Ins::Chain`
//! (vm.rs), with the same bytes, so neither side needs memory mode for it.
//!
//! RUN file$ / CHAIN replace the program, as in QBasic: on Unix the process
//! becomes the next program (`exec`); on Windows, which has no `exec`, this
//! program closes its files, stops its audio and hides its window, waits for
//! the next program and ends with that program's exit code.

use super::builtins::{Host, DBLOCK};
use super::{e, R, V};
use std::time::Instant;

/// One VARPTR mirror: DBLOCK offset `off` holds `len` bytes from `addr`.
struct Mirror {
    off: usize,
    addr: u64,
    len: usize,
    /// array descriptor (0 = a scalar) and the array's (first element, bytes)
    /// when mirrored: a REDIM/ERASE since then makes the mirror stale
    desc: u64,
    extent: (u64, u64),
}

#[derive(Default)]
pub(crate) struct Dos {
    /// VGA DAC: &H3C8 write index and component, &H3C7 read index and component
    dac_w: u8,
    dac_wn: u8,
    dac_r: u8,
    dac_rn: u8,
    /// emulated 60 Hz vertical retrace: program start and the last frame reported
    retrace_t0: Option<Instant>,
    retrace_frame: u64,
    mirrors: Vec<Mirror>,
    /// RUN / CLEAR asked for: (RUNFROM target, true = RUN)
    pub(crate) restart: Option<(i32, bool)>,
    /// what RUNFROM returns once (set by `Vm::restart`)
    pub(crate) run_from: i32,
    /// RUN file$ / CHAIN ran the other program: this one ends with its exit code
    pub(crate) end_code: Option<i32>,
    /// COMMON values for the program CHAIN runs next
    chain_out: Vec<u8>,
    /// COMMON values from the program that CHAINed here (read at the first CHAINGET)
    chain_in: Option<std::io::Cursor<Vec<u8>>>,
}

/// One COMMON value: kind (1 = bytes, 2 = STRING, 3 = array of fixed-size
/// elements, 4 = STRING array), length, bytes.
fn chain_item(out: &mut Vec<u8>, kind: u8, b: &[u8]) {
    out.push(kind);
    out.extend_from_slice(&(b.len() as u32).to_le_bytes());
    out.extend_from_slice(b);
}

/// A COMMON value of `kind` for the program CHAIN runs next.
pub(crate) fn chain_put(d: &mut Dos, kind: u8, b: &[u8]) {
    chain_item(&mut d.chain_out, kind, b);
}

/// The file of COMMON values this program was started with, when no COMMON
/// statement read it (the next program has no COMMON): removed at the end.
pub(crate) fn chain_cleanup(d: &mut Dos) {
    if d.chain_in.is_none()
        && let Some(p) = std::env::var_os("QBNEX_CHAIN")
    {
        let _ = std::fs::remove_file(p);
    }
}

/// The next COMMON value passed by CHAIN, when it is of `kind`.
pub(crate) fn chain_next(d: &mut Dos, kind: u8) -> Option<Vec<u8>> {
    use std::io::Read;
    let c = d.chain_in.get_or_insert_with(|| {
        let data = std::env::var_os("QBNEX_CHAIN").and_then(|p| {
            let b = std::fs::read(&p).ok();
            let _ = std::fs::remove_file(&p);
            b
        });
        std::io::Cursor::new(data.unwrap_or_default())
    });
    let mut head = [0u8; 5];
    c.read_exact(&mut head).ok()?;
    let mut b = vec![0u8; u32::from_le_bytes([head[1], head[2], head[3], head[4]]) as usize];
    c.read_exact(&mut b).ok()?;
    (head[0] == kind).then_some(b)
}

/// Bounds of a COMMON array item: their count, then (lower, upper) pairs; none = not dimensioned.
pub(crate) fn put_bounds(out: &mut Vec<u8>, dims: &[(i64, i64)]) {
    out.extend_from_slice(&(dims.len() as u32).to_le_bytes());
    for &(lb, ub) in dims {
        out.extend_from_slice(&lb.to_le_bytes());
        out.extend_from_slice(&ub.to_le_bytes());
    }
}

pub(crate) fn take<'b>(b: &'b [u8], at: &mut usize, n: usize) -> Option<&'b [u8]> {
    let s = b.get(*at..at.checked_add(n)?)?;
    *at += n;
    Some(s)
}

pub(crate) fn take_u32(b: &[u8], at: &mut usize) -> Option<u32> {
    take(b, at, 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

pub(crate) fn take_bounds(b: &[u8], at: &mut usize) -> Option<Vec<(i64, i64)>> {
    let n = take_u32(b, at)?;
    let i64_at =|at: &mut usize| take(b, at, 8).and_then(|s| s.try_into().ok()).map(i64::from_le_bytes);
    (0..n).map(|_| Some((i64_at(at)?, i64_at(at)?))).collect()
}

/// Number of elements of an array with these bounds (0 = not dimensioned).
pub(crate) fn elements(dims: &[(i64, i64)]) -> u64 {
    if dims.is_empty() {
        return 0;
    }
    dims.iter().fold(1u64, |n, &(lb, ub)| n.saturating_mul(ub.saturating_sub(lb).saturating_add(1).max(0) as u64))
}

/// CHAINPUTA / CHAINPUTAS: the COMMON array at `p` for the next program. Element
/// size `esize` (an item of kind 3): its bounds and element bytes; None (kind 4,
/// variable-length STRINGs): its bounds and each element as length + bytes.
fn chain_put_array(vm: &mut dyn Host<'_>, p: u64, esize: Option<u32>) -> R<()> {
    let (dims, _) = vm.array_dims(p)?;
    let mut b = Vec::new();
    if let Some(es) = esize {
        b.extend_from_slice(&es.to_le_bytes());
    }
    put_bounds(&mut b, &dims);
    let n = elements(&dims);
    if n > 0 {
        let (addr, bytes) = vm.array_extent(p)?;
        if esize.is_some() {
            b.extend_from_slice(vm.bytes(addr, bytes as usize)?);
        } else {
            let stride = bytes.checked_div(n).unwrap_or(0);
            for k in 0..n {
                let s = vm.load_str(addr + k * stride)?;
                b.extend_from_slice(&(s.len() as u32).to_le_bytes());
                b.extend_from_slice(&s);
            }
        }
    }
    chain_item(&mut vm.rt().dos.chain_out, if esize.is_some() { 3 } else { 4 }, &b);
    Ok(())
}

/// CHAINGETA / CHAINGETAS: the COMMON array at `p` takes the array passed at the
/// same position. A dynamic array is redimensioned to the passed bounds; a static
/// one keeps its bounds and takes the values when it has as many elements.
/// An array of another kind or element size is skipped.
fn chain_get_array(vm: &mut dyn Host<'_>, p: u64, esize: Option<u32>) -> R<()> {
    let Some(b) = chain_next(&mut vm.rt().dos, if esize.is_some() { 3 } else { 4 }) else { return Ok(()) };
    let mut at = 0;
    if esize.is_some() && take_u32(&b, &mut at) != esize {
        return Ok(());
    }
    let Some(dims) = take_bounds(&b, &mut at) else { return Ok(()) };
    let n = elements(&dims);
    if n == 0 {
        return Ok(()); // not dimensioned where it came from
    }
    let (cur, fixed) = vm.array_dims(p)?;
    if fixed {
        if elements(&cur) != n {
            return Ok(());
        }
    } else {
        vm.array_redim(p, dims)?;
    }
    let (addr, bytes) = vm.array_extent(p)?;
    if esize.is_some() {
        let data = &b[at..];
        let k = (bytes as usize).min(data.len());
        if k > 0 {
            vm.bytes(addr, k)?.copy_from_slice(&data[..k]);
        }
    } else {
        let stride = bytes.checked_div(n).unwrap_or(0);
        for k in 0..n {
            let Some(len) = take_u32(&b, &mut at) else { break };
            let Some(s) = take(&b, &mut at, len as usize) else { break };
            vm.store_str(addr + k * stride, s)?;
        }
    }
    Ok(())
}

impl Dos {
    /// The statement just executed ends this run of the program.
    pub(crate) fn stops(&self) -> bool {
        self.restart.is_some() || self.end_code.is_some()
    }
}

fn overlap(m: &Mirror, lo: usize, hi: usize) -> Option<(usize, usize)> {
    let (a, b) = (DBLOCK + m.off, DBLOCK + m.off + m.len);
    let (s, t) = (lo.max(a), hi.min(b));
    (s < t).then_some((s, t))
}

/// Drops mirrors of arrays that were redimensioned or erased.
fn prune(vm: &mut dyn Host<'_>) {
    let mut ms = std::mem::take(&mut vm.rt().dos.mirrors);
    ms.retain(|m| m.desc == 0 || vm.array_extent(m.desc).ok() == Some(m.extent));
    vm.rt().dos.mirrors = ms;
}

/// Copies the variables mirrored in cmem[lo..hi] into cmem.
fn sync_in(vm: &mut dyn Host<'_>, lo: usize, hi: usize) -> R<()> {
    prune(vm);
    let spans: Vec<(usize, u64, usize)> = vm
        .rt()
        .dos
        .mirrors
        .iter()
        .filter_map(|m| overlap(m, lo, hi).map(|(s, t)| (s, m.addr + (s - DBLOCK - m.off) as u64, t - s)))
        .collect();
    for (at, addr, n) in spans {
        let b = vm.bytes(addr, n)?.to_vec();
        vm.rt().cmem[at..at + n].copy_from_slice(&b);
    }
    Ok(())
}

/// Writes cmem[lo..hi] back into the array elements mirrored there.
fn sync_out(vm: &mut dyn Host<'_>, lo: usize, hi: usize) -> R<()> {
    prune(vm);
    let spans: Vec<(usize, u64, usize)> = vm
        .rt()
        .dos
        .mirrors
        .iter()
        .filter(|m| m.desc != 0)
        .filter_map(|m| overlap(m, lo, hi).map(|(s, t)| (s, m.addr + (s - DBLOCK - m.off) as u64, t - s)))
        .collect();
    for (at, addr, n) in spans {
        let b = vm.rt().cmem[at..at + n].to_vec();
        vm.bytes(addr, n)?.copy_from_slice(&b);
    }
    Ok(())
}

/// VARPTR of a numeric variable (`desc` = 0) or of an element of the array `desc`.
fn varptr(vm: &mut dyn Host<'_>, addr: u64, desc: u64, size: usize) -> R<i64> {
    prune(vm);
    let (base, len, extent) = if desc != 0 {
        let x = vm.array_extent(desc)?;
        (x.0, x.1 as usize, x)
    } else {
        (addr, size, (0, 0))
    };
    let found = vm.rt().dos.mirrors.iter().position(|m| m.addr == base && m.desc == desc);
    let off = match found {
        Some(i) => vm.rt().dos.mirrors[i].off,
        None => {
            let rt = vm.rt();
            let off = rt.cmem_next;
            // the mirror stays inside the 64 KB segment
            let len = len.min(0x10000usize.saturating_sub(off));
            if len == 0 {
                return e(7);
            }
            rt.cmem_next += len;
            rt.dos.mirrors.push(Mirror { off, addr: base, len, desc, extent });
            off
        }
    };
    let at = off + (addr - base) as usize;
    if at >= 0x10000 {
        return e(7);
    }
    sync_in(vm, DBLOCK + at, DBLOCK + at + size.max(1))?;
    Ok(at as i64)
}

/// Absolute cmem index of DEF SEG:offset (offset wraps at 64 KB, as PEEK/POKE).
fn seg_at(vm: &mut dyn Host<'_>, off: i64) -> usize {
    let rt = vm.rt();
    (rt.def_seg.unwrap_or(DBLOCK) + (off as u16 as usize)) % rt.cmem.len()
}

/// The write page's palette entry `i` (`None` before the screen started).
fn pal_entry(vm: &mut dyn Host<'_>, i: u8) -> Option<*mut u32> {
    let d = &vm.rt().display;
    if !d.started {
        return None;
    }
    // SAFETY: the write index is a live registry entry once the screen started.
    let pal = unsafe { (*qb_runtime::gfx::registry::img.add(d.write_index as usize)).pal };
    // SAFETY: palettes have 256 entries.
    (!pal.is_null()).then(|| unsafe { pal.add(usize::from(i)) })
}

/// INP; `None` = a port the runtime does not emulate.
fn inp(vm: &mut dyn Host<'_>, port: i64) -> R<Option<i64>> {
    if !(-65536..=65535).contains(&port) {
        return e(6);
    }
    match port & 0xFFFF {
        0x3C9 => {
            let (i, n) = (vm.rt().dos.dac_r, vm.rt().dos.dac_rn);
            let v = match pal_entry(vm, i) {
                // SAFETY: valid palette entry.
                Some(p) => {
                    let c = unsafe { *p };
                    let comp = (c >> (16 - 8 * u32::from(n))) & 255;
                    i64::from(qb_runtime::gfx::qbr_d2l(f64::from(comp) / 3.984376 - f64::from(0.4999999f32)))
                }
                None => return Ok(Some(0)),
            };
            let d = &mut vm.rt().dos;
            d.dac_rn += 1;
            if d.dac_rn == 3 {
                d.dac_rn = 0;
                d.dac_r = d.dac_r.wrapping_add(1);
            }
            Ok(Some(v))
        }
        0x3DA => {
            // bit 3: a vertical retrace happened since the last read (60 Hz)
            let d = &mut vm.rt().dos;
            let t0 = *d.retrace_t0.get_or_insert_with(Instant::now);
            let frame = (t0.elapsed().as_micros() * 60 / 1_000_000) as u64;
            if frame != d.retrace_frame {
                d.retrace_frame = frame;
                Ok(Some(8))
            } else {
                Ok(Some(0))
            }
        }
        0x60 => {
            let rt = vm.rt();
            rt.input.pump(&mut rt.cmem);
            Ok(Some(i64::from(rt.input.port60())))
        }
        _ => Ok(None),
    }
}

fn out(vm: &mut dyn Host<'_>, port: i64, data: i64) {
    let data = (data & 255) as u8;
    let d = &mut vm.rt().dos;
    match port & 0xFFFF {
        0x3C7 => {
            d.dac_r = data;
            d.dac_rn = 0;
        }
        0x3C8 => {
            d.dac_w = data;
            d.dac_wn = 0;
        }
        0x3C9 => {
            let (i, n) = (d.dac_w, d.dac_wn);
            d.dac_wn += 1;
            if d.dac_wn == 3 {
                d.dac_wn = 0;
                d.dac_w = d.dac_w.wrapping_add(1);
            }
            if let Some(p) = pal_entry(vm, i) {
                let v = qb_runtime::gfx::qbr_i64(f64::from(data & 63) * f64::from(4.063492f32) - f64::from(0.4999999f32)) as u32;
                let shift = 16 - 8 * u32::from(n);
                // SAFETY: valid palette entry.
                unsafe { *p = (*p & !(0xFF << shift)) | ((v & 0xFF) << shift) };
            }
        }
        // attribute controller: bit 3 switches blinking (see `_BLINK`)
        0x3C0 => super::display::BLINK.store(data & 8 != 0, std::sync::atomic::Ordering::Relaxed),
        _ => {}
    }
}

fn bload(vm: &mut dyn Host<'_>, file: &[u8], offset: i64, passed: bool) -> R<()> {
    let mut offset = offset;
    if passed {
        if !(-65536..=65535).contains(&offset) {
            return e(6);
        }
        offset &= 0xFFFF;
    }
    let path = String::from_utf8_lossy(file).into_owned();
    let data = std::fs::read(&path).or_else(|_| e(53))?;
    if data.len() < 7 || data[0] != 253 {
        return e(54);
    }
    let le = |i: usize| usize::from(u16::from_le_bytes([data[i], data[i + 1]]));
    let (seg, off) = (le(1), le(3));
    let mut size = le(5);
    if size == 0 {
        size = data.len() - 7;
        if size < 65536 {
            size = 0;
        }
    }
    if data.len() - 7 < size {
        return e(54);
    }
    let at = if passed { seg_at(vm, offset) } else { seg * 16 + off };
    let rt = vm.rt();
    if at + size > rt.cmem.len() {
        return e(54);
    }
    rt.cmem[at..at + size].copy_from_slice(&data[7..7 + size]);
    sync_out(vm, at, at + size)
}

fn bsave(vm: &mut dyn Host<'_>, file: &[u8], offset: i64, size: i64) -> R<()> {
    if !(-65536..=65535).contains(&offset) || !(-65536..=65536).contains(&size) {
        return e(6);
    }
    let size = if size == 65536 { 65536 } else { (size & 0xFFFF) as usize };
    let at = seg_at(vm, offset & 0xFFFF);
    sync_in(vm, at, at + size)?;
    let rt = vm.rt();
    let seg = (rt.def_seg.unwrap_or(DBLOCK) / 16) as u16;
    let mut b = vec![253u8];
    b.extend_from_slice(&seg.to_le_bytes());
    b.extend_from_slice(&((offset & 0xFFFF) as u16).to_le_bytes());
    b.extend_from_slice(&(if size > 65535 { 0 } else { size as u16 }).to_le_bytes());
    let end = (at + size).min(rt.cmem.len());
    b.extend_from_slice(&rt.cmem[at..end]);
    let path = String::from_utf8_lossy(file).into_owned();
    std::fs::write(&path, b).or_else(|_| e(64))
}

/// What this program holds that the next one may need, released before
/// RUN file$ / CHAIN hands over: open files (flushed and closed) and the
/// audio device; a program window is hidden (the window thread owns it and
/// closes it when this process ends).
fn release_for_next(vm: &mut dyn Host<'_>) -> R<()> {
    vm.rt().flush();
    if let Some(r) = super::files::call(vm, "CLOSEALL", &[]) {
        r?;
    }
    let rt = vm.rt();
    if rt.snd_started {
        qb_audio::ffi::qb_snd_un_init();
        rt.snd_started = false;
    }
    #[cfg(not(target_arch = "wasm32"))]
    if rt.window {
        qb_runtime::window::qb_window_show(0);
    }
    Ok(())
}

/// RUN file$ / CHAIN: the program `cmd` replaces this one. Unix: the process
/// becomes it (`exec`, through the shell as SHELL does). Windows: this program
/// releases its files, audio and window, waits for it and ends with its exit
/// code (the old C++ runtime started it and ended at once; waiting keeps the console
/// output in order and the exit code for whoever started this program).
fn run_program(vm: &mut dyn Host<'_>, cmd: &str, exe: &str) -> R<()> {
    let found = std::path::Path::new(exe).is_file() || (cfg!(windows) && std::path::Path::new(&format!("{exe}.exe")).is_file());
    if !found {
        return e(53);
    }
    let mut c = super::builtins::shell_command(cmd);
    let common = std::mem::take(&mut vm.rt().dos.chain_out);
    let file = (!common.is_empty()).then(|| std::env::temp_dir().join(format!("qbnex-chain-{}.bin", std::process::id())));
    match &file {
        Some(f) => {
            if std::fs::write(f, &common).is_err() {
                return e(75);
            }
            c.env("QBNEX_CHAIN", f);
        }
        None => {
            c.env_remove("QBNEX_CHAIN");
        }
    }
    // Every way back from here removes the file of COMMON values this program
    // wrote: the next program has ended (Windows) or could not be started. When
    // `exec` works this process is gone, and the next program removes the file
    // when it reads it.
    let _file = std::mem::DropGuard::new(file, |f| {
        if let Some(f) = f {
            let _ = std::fs::remove_file(f);
        }
    });
    // the file of COMMON values this program got is no longer needed
    chain_cleanup(&mut vm.rt().dos);
    release_for_next(vm)?;
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // returns only when the shell could not be started
        let _ = c.exec();
        e(53)
    }
    #[cfg(not(unix))]
    {
        let Ok(st) = c.status() else { return e(53) };
        vm.rt().dos.end_code = Some(st.code().unwrap_or(1));
        Ok(())
    }
}

/// CHAIN name: `.bas` runs the `.exe` of the same name; without an extension
/// the first word gets `.exe` and the rest are its arguments (as QB64's `sub_chain`).
fn chain_command(f: &str) -> (String, String) {
    let up = f.to_ascii_uppercase();
    if let Some(p) = up.find('.') {
        let ext = &up[p + 1..];
        if ext.starts_with("BAS") || ext.starts_with("EXE") {
            let exe = format!("{}{}", &f[..p], if cfg!(windows) { ".exe" } else { "" });
            return (format!("\"{}\"{}", in_dir(&exe), &f[p + 4..]), exe);
        }
    }
    let (name, args) = f.split_once(' ').map_or((f, ""), |(a, b)| (a, b));
    let exe = format!("{name}{}", if cfg!(windows) { ".exe" } else { "" });
    let cmd = if args.is_empty() { format!("\"{}\"", in_dir(&exe)) } else { format!("\"{}\" {args}", in_dir(&exe)) };
    (cmd, exe)
}

/// A program name without a directory runs from the current directory, as DOS did:
/// `sh` searches only the PATH, and so does `cmd` with NoDefaultCurrentDirectoryInExePath.
fn in_dir(exe: &str) -> String {
    if exe.contains(['/', '\\', ':']) {
        exe.to_string()
    } else if cfg!(windows) {
        format!(".\\{exe}")
    } else {
        format!("./{exe}")
    }
}

/// RUN / CLEAR: what CLEAR (and for RUN also the run-time init) resets that
/// lives in the runtime; the variables are the VM's (`Vm::restart`).
fn clear(vm: &mut dyn Host<'_>, run: bool) -> R<()> {
    if let Some(r) = super::files::call(vm, "CLOSEALL", &[]) {
        r?;
    }
    let rt = vm.rt();
    rt.cmem_strs.clear();
    rt.cmem_sdata.clear();
    rt.field_vars.clear();
    rt.dos.mirrors.clear();
    if run {
        rt.rnd = Default::default();
        if rt.display.started {
            // SCREEN 0, COLOR 7, 0 (sub_run_init)
            for (name, a) in [("qbg_screen", vec![0, 0, 0, 0, 0, 1 | 4 | 8]), ("qbg_sub_color", vec![7, 0, 0, 3])] {
                let a: Vec<V> = a.into_iter().map(V::I).collect();
                if let Some(r) = super::display::call(vm, name, &a) {
                    r?;
                }
            }
        }
    }
    Ok(())
}

/// FILES [spec$]: the directory line, entries of 18 columns
/// (a new line when one does not fit in 80), then "n Bytes free".
fn files(vm: &mut dyn Host<'_>, spec: &[u8], passed: bool) -> R<()> {
    let (err, lines) = qb_runtime::fsops::files(passed.then_some(spec));
    if err == 5 {
        return e(5);
    }
    let Some((dir, rest)) = lines.split_first() else { return Ok(()) };
    let p = |vm: &mut dyn Host<'_>, s: &[u8], nl: bool| -> R<()> {
        super::builtins::call(vm, "PRINTS", vec![V::I(0), V::S(s.to_vec())])?;
        if nl {
            super::builtins::call(vm, "PRINTNL", vec![V::I(0)])?;
        }
        Ok(())
    };
    p(vm, dir, true)?;
    if err != 0 {
        return e(err);
    }
    let Some((last, entries)) = rest.split_last() else { return Ok(()) };
    for x in entries {
        let pos = super::builtins::call(vm, "POS", vec![V::I(0)])?.map_or(1, |v| v.i());
        if pos > 1 && pos as usize - 1 + x.len() > 80 {
            p(vm, b"", true)?;
        }
        p(vm, x, false)?;
    }
    let pos = super::builtins::call(vm, "POS", vec![V::I(0)])?.map_or(1, |v| v.i());
    if pos > 1 {
        p(vm, b"", true)?;
    }
    p(vm, last, true)
}

pub(crate) fn call(vm: &mut dyn Host<'_>, name: &str, a: &[V]) -> Option<R<Option<V>>> {
    let i = |k: usize| a[k].i();
    let none = Ok(None);
    Some(match name {
        "sub_out" => {
            out(vm, i(0), i(1));
            none
        }
        "func_inp" => inp(vm, i(0)).map(|v| Some(V::I(v.unwrap_or(0)))),
        "sub_wait" => {
            let (port, and, xor, passed) = (i(0), i(1), i(2), i(3) != 0);
            if !(-65536..=65535).contains(&port) || !(-32768..=65535).contains(&and) || (passed && !(-32768..=65535).contains(&xor)) {
                return Some(e(6));
            }
            loop {
                match inp(vm, port) {
                    Err(x) => return Some(Err(x)),
                    // an unsupported port does not block the program
                    Ok(None) => break none,
                    Ok(Some(v)) => {
                        let v = if passed { v ^ (xor & 255) } else { v };
                        if v & and & 255 != 0 {
                            break none;
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        "sub_bload" => bload(vm, a[0].s(), i(1), i(2) != 0).map(|_| None),
        "sub_bsave" => bsave(vm, a[0].s(), i(1), i(2)).map(|_| None),
        "VARPTR.n" => varptr(vm, a[0].p(), a[1].p(), i(2).max(0) as usize).map(|v| Some(V::I(v))),
        "PEEK" | "POKE" if !vm.rt().dos.mirrors.is_empty() => {
            let at = seg_at(vm, i(0));
            if name == "POKE" {
                vm.rt().cmem[at] = i(1) as u8;
                sync_out(vm, at, at + 1).map(|_| None)
            } else {
                sync_in(vm, at, at + 1).map(|_| Some(V::I(vm.rt().cmem[at].into())))
            }
        }
        "RUN" | "CLEAR" => {
            let run = name == "RUN";
            clear(vm, run).map(|_| {
                vm.rt().dos.restart = Some((i(0) as i32, run));
                None
            })
        }
        "RUNFROM" => {
            let d = &mut vm.rt().dos;
            let k = std::mem::take(&mut d.run_from);
            Ok(Some(V::I(i64::from(k))))
        }
        "RUNFILE" => {
            let f = String::from_utf8_lossy(a[0].s()).into_owned();
            // as CHAIN: a name without a directory is the program in the current directory
            let (exe, args) = f.split_once(' ').map_or((f.as_str(), ""), |(x, y)| (x, y));
            let exe = exe.to_string();
            let cmd = if args.is_empty() { format!("\"{}\"", in_dir(&exe)) } else { format!("\"{}\" {args}", in_dir(&exe)) };
            // RUN passes no COMMON variables
            vm.rt().dos.chain_out.clear();
            run_program(vm, &cmd, &exe).map(|_| None)
        }
        "CHAINPUT" => {
            let b = match vm.bytes(a[0].p(), i(1).max(0) as usize) {
                Ok(b) => b.to_vec(),
                Err(x) => return Some(Err(x)),
            };
            chain_item(&mut vm.rt().dos.chain_out, 1, &b);
            none
        }
        "CHAINPUTS" => {
            let s = match vm.load_str(a[0].p()) {
                Ok(s) => s,
                Err(x) => return Some(Err(x)),
            };
            chain_item(&mut vm.rt().dos.chain_out, 2, &s);
            none
        }
        "CHAINGET" => {
            let n = i(1).max(0) as usize;
            match chain_next(&mut vm.rt().dos, 1) {
                Some(b) if b.len() == n => vm.bytes(a[0].p(), n).map(|m| {
                    m.copy_from_slice(&b);
                    None
                }),
                _ => none,
            }
        }
        "CHAINGETS" => match chain_next(&mut vm.rt().dos, 2) {
            Some(s) => vm.store_str(a[0].p(), &s).map(|_| None),
            None => none,
        },
        "CHAINPUTA" => chain_put_array(vm, a[0].p(), Some(i(1).max(0) as u32)).map(|_| None),
        "CHAINPUTAS" => chain_put_array(vm, a[0].p(), None).map(|_| None),
        "CHAINGETA" => chain_get_array(vm, a[0].p(), Some(i(1).max(0) as u32)).map(|_| None),
        "CHAINGETAS" => chain_get_array(vm, a[0].p(), None).map(|_| None),
        "CHAIN" => {
            let f = String::from_utf8_lossy(a[0].s()).into_owned();
            if f.is_empty() {
                return Some(e(53));
            }
            let (cmd, exe) = chain_command(&f);
            run_program(vm, &cmd, &exe).map(|_| None)
        }
        // STRIG[(n[, controller])] ON|OFF|STOP (+ passed)
        "sub_strig" => {
            let ev = qb_runtime::events::onstrigs();
            qb_runtime::events::strig_statement(ev, i(0) as i32, i(1) as i32, i(2) as i32, i(3) as i32).map_or_else(e, |_| Ok(None))
        }
        "sub_files" => files(vm, a[0].s(), i(1) != 0).map(|_| None),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::chain_command;

    #[test]
    fn common_values_come_back_in_order_and_mismatches_are_skipped() {
        use super::{chain_item, chain_next, Dos};
        let mut out = Vec::new();
        chain_item(&mut out, 1, &7i16.to_le_bytes());
        chain_item(&mut out, 2, b"text");
        chain_item(&mut out, 1, &[1, 2, 3, 4]);
        let mut d = Dos { chain_in: Some(std::io::Cursor::new(out)), ..Dos::default() };
        assert_eq!(chain_next(&mut d, 1), Some(7i16.to_le_bytes().to_vec()));
        assert_eq!(chain_next(&mut d, 2), Some(b"text".to_vec()));
        // a STRING asked for where bytes were passed: skipped
        assert_eq!(chain_next(&mut d, 2), None);
        assert_eq!(chain_next(&mut d, 1), None);
    }

    #[test]
    fn common_array_bounds_round_trip() {
        use super::{elements, put_bounds, take_bounds, take_u32};
        let mut b = 6u32.to_le_bytes().to_vec();
        put_bounds(&mut b, &[(-1, 1), (0, 2)]);
        b.extend_from_slice(b"rest");
        let mut at = 0;
        assert_eq!(take_u32(&b, &mut at), Some(6));
        let dims = take_bounds(&b, &mut at).unwrap();
        assert_eq!(dims, vec![(-1, 1), (0, 2)]);
        assert_eq!(elements(&dims), 9);
        assert_eq!(&b[at..], b"rest");
        // not dimensioned: no bounds, no elements
        let mut e = Vec::new();
        put_bounds(&mut e, &[]);
        assert_eq!(elements(&take_bounds(&e, &mut 0).unwrap()), 0);
        // a cut item gives no bounds instead of reading past its end
        assert_eq!(take_bounds(&b[4..20], &mut 0), None);
    }

    #[test]
    fn chain_names() {
        let x = if cfg!(windows) { ".exe" } else { "" };
        assert_eq!(chain_command("next.bas").1, format!("next{x}"));
        assert_eq!(chain_command("NEXT.EXE").1, format!("NEXT{x}"));
        let dir = if cfg!(windows) { ".\\" } else { "./" };
        assert_eq!(chain_command("menu a b"), (format!("\"{dir}menu{x}\" a b"), format!("menu{x}")));
    }
}
