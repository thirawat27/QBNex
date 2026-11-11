//! Executable writer (D15): links object files into a native executable
//! without any external linker.
//!
//! Formats: PE32+ (Windows x86-64), ELF64 (Linux x86-64 / AArch64, static or
//! dynamically linked against shared libraries through a GOT), Mach-O 64
//! (macOS x86-64 / arm64, bound to dylibs through dyld bind opcodes, ad-hoc
//! code signature).
//!
//! The pipeline is the same for every format:
//! 1. [`Linked::build`] reads the objects, resolves symbols (definitions,
//!    COMDAT/duplicate handling, imports from shared libraries), and lays the
//!    sections out into three segments: code, read-only data and writable
//!    data (+ zero-fill). Imports get a GOT slot (grouped per library, each
//!    group null-terminated: that is the PE import address table as is) and
//!    a jump stub in the code segment.
//! 2. A format writer chooses the segments' addresses, adds its own tables
//!    and calls [`Linked::apply`], which patches every relocation and reports
//!    the absolute pointers it wrote (base relocations / rebase info).

pub mod elf;
pub mod macho;
pub mod pe;

use crate::LinkError;
use object::{Object, ObjectSection, ObjectSymbol, RelocationFlags, RelocationTarget, SectionKind};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Pe,
    Elf,
    MachO,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86_64,
    Aarch64,
}

/// A function imported from a shared library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    /// Symbol name as the objects refer to it (Mach-O: with the leading `_`).
    pub symbol: String,
    /// DLL / shared object / dylib (`kernel32.dll`, `libc.so.6`, `/usr/lib/libSystem.B.dylib`).
    pub library: String,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub format: Format,
    pub arch: Arch,
    /// Entry symbol: PE/ELF process entry (`_start`-like: never returns), Mach-O `main`.
    pub entry: String,
    pub imports: Vec<Import>,
}

impl Options {
    pub fn new(format: Format, arch: Arch, entry: &str) -> Options {
        Options { format, arch, entry: entry.into(), imports: Vec::new() }
    }
    pub fn import(mut self, symbol: &str, library: &str) -> Options {
        self.imports.push(Import { symbol: symbol.into(), library: library.into() });
        self
    }
}

/// Links `objects` into an executable file.
pub fn link_exe(objects: &[&[u8]], opts: &Options) -> Result<Vec<u8>, LinkError> {
    let mut l = Linked::build(objects, opts)?;
    match opts.format {
        Format::Pe => pe::write(&mut l, opts),
        Format::Elf => elf::write(&mut l, opts),
        Format::MachO => macho::write(&mut l, opts),
    }
}

fn err<T>(m: impl Into<String>) -> Result<T, LinkError> {
    Err(LinkError(m.into()))
}

// ---------------------------------------------------------------- core

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Seg {
    Text,
    RoData,
    Data,
    /// zero-fill, placed right after `Data`
    Bss,
}

/// Where a relocation points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tgt {
    At(Seg, u64),
    /// GOT slot `i` (address of the slot)
    Got(usize),
    /// jump stub of import `i`
    Stub(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Abs64,
    Abs32,
    /// 32-bit image-relative address (PE `ADDR32NB`)
    Rva32,
    /// `S + A - P`, 32 bits
    Pc32,
    /// AArch64 B/BL: 26-bit word offset
    Call26,
    /// AArch64 ADRP: page difference
    Page21,
    /// AArch64 low 12 bits into ADD (shift 0) or a load/store (`shift` = log2 access size)
    Lo12 { shift: u8 },
}

#[derive(Debug, Clone, Copy)]
pub struct Fix {
    pub seg: Seg,
    pub off: u64,
    pub kind: Kind,
    pub tgt: Tgt,
    pub addend: i64,
}

#[derive(Debug, Clone)]
pub struct GotSlot {
    /// import index, or `None` for the terminator of a library group / a local slot
    pub import: Option<usize>,
    /// for local slots: the symbol address
    pub local: Option<(Seg, u64)>,
}

pub struct Linked {
    pub arch: Arch,
    pub text: Vec<u8>,
    pub rodata: Vec<u8>,
    /// starts with the GOT (`got.len()` slots of 8 bytes)
    pub data: Vec<u8>,
    pub bss: u64,
    pub fixes: Vec<Fix>,
    pub got: Vec<GotSlot>,
    /// imports that are referenced, in GOT order
    pub imports: Vec<Import>,
    /// offset of each import's stub in `text`
    pub stubs: Vec<u64>,
    pub entry: (Seg, u64),
    /// `.pdata` (PE exception table) placement, if an object has one
    pub pdata: Option<(u64, u64)>,
}

/// A defined symbol: object index, section index, offset in section.
type Def = (usize, object::SectionIndex, u64);

fn seg_of(kind: SectionKind, name: &str) -> Option<Seg> {
    if name == ".pdata" || name == ".xdata" {
        return Some(Seg::RoData);
    }
    match kind {
        SectionKind::Text => Some(Seg::Text),
        SectionKind::ReadOnlyData | SectionKind::ReadOnlyString | SectionKind::ReadOnlyDataWithRel => Some(Seg::RoData),
        SectionKind::Data => Some(Seg::Data),
        SectionKind::UninitializedData | SectionKind::Common => Some(Seg::Bss),
        SectionKind::Tls | SectionKind::UninitializedTls | SectionKind::TlsVariables => None,
        _ => None,
    }
}

const STUB_X86: [u8; 8] = [0xFF, 0x25, 0, 0, 0, 0, 0xCC, 0xCC]; // jmp [rip+slot]
const STUB_A64: [u8; 12] = [
    0x10, 0x00, 0x00, 0x90, // adrp x16, slot
    0x10, 0x02, 0x40, 0xF9, // ldr  x16, [x16, slot@lo12]
    0x00, 0x02, 0x1F, 0xD6, // br   x16
];

impl Linked {
    pub fn build(objects: &[&[u8]], opts: &Options) -> Result<Linked, LinkError> {
        let files: Vec<object::File> =
            objects.iter().map(|o| object::File::parse(*o).map_err(|e| LinkError(e.to_string()))).collect::<Result<_, _>>()?;
        for f in &files {
            let a = match f.architecture() {
                object::Architecture::X86_64 => Arch::X86_64,
                object::Architecture::Aarch64 => Arch::Aarch64,
                a => return err(format!("unsupported architecture {a:?}")),
            };
            if a != opts.arch {
                return err(format!("object for {a:?} linked for {:?}", opts.arch));
            }
        }
        let mut l = Linked {
            arch: opts.arch,
            text: Vec::new(),
            rodata: Vec::new(),
            data: Vec::new(),
            bss: 0,
            fixes: Vec::new(),
            got: Vec::new(),
            imports: Vec::new(),
            stubs: Vec::new(),
            entry: (Seg::Text, 0),
            pdata: None,
        };
        // 1. Which sections are kept (one copy per COMDAT group).
        let mut keep: HashMap<(usize, object::SectionIndex), Seg> = HashMap::new();
        let mut comdats: std::collections::HashSet<String> = Default::default();
        let mut dropped: std::collections::HashSet<(usize, object::SectionIndex)> = Default::default();
        for (fi, f) in files.iter().enumerate() {
            for c in f.comdats() {
                let name = c.name().map_err(|e| LinkError(e.to_string()))?.to_string();
                if !comdats.insert(name) {
                    for s in c.sections() {
                        dropped.insert((fi, s));
                    }
                }
            }
        }
        for (fi, f) in files.iter().enumerate() {
            for s in f.sections() {
                if dropped.contains(&(fi, s.index())) || s.size() == 0 && s.kind() != SectionKind::Text {
                    continue;
                }
                let name = s.name().unwrap_or("");
                if matches!(s.kind(), SectionKind::Tls | SectionKind::UninitializedTls | SectionKind::TlsVariables) {
                    return err(format!("thread-local storage is not supported (section {name})"));
                }
                if let Some(seg) = seg_of(s.kind(), name) {
                    keep.insert((fi, s.index()), seg);
                }
            }
        }
        // 2. Global definitions.
        let mut defs: HashMap<String, Def> = HashMap::new();
        let mut weak: std::collections::HashSet<String> = Default::default();
        for (fi, f) in files.iter().enumerate() {
            for sym in f.symbols() {
                if !sym.is_definition() || sym.is_local() {
                    continue;
                }
                let Some(si) = sym.section_index() else { continue };
                if !keep.contains_key(&(fi, si)) && !dropped.contains(&(fi, si)) {
                    continue;
                }
                if dropped.contains(&(fi, si)) {
                    continue;
                }
                let name = sym.name().map_err(|e| LinkError(e.to_string()))?.to_string();
                let sec = f.section_by_index(si).map_err(|e| LinkError(e.to_string()))?;
                let d = (fi, si, sym.address() - sec.address());
                match defs.get(&name) {
                    Some(_) if sym.is_weak() => {}
                    Some(_) if weak.contains(&name) => {
                        weak.remove(&name);
                        defs.insert(name, d);
                    }
                    Some(_) => return err(format!("duplicate symbol {name}")),
                    None => {
                        if sym.is_weak() {
                            weak.insert(name.clone());
                        }
                        defs.insert(name, d);
                    }
                }
            }
        }
        // 3. Imports actually referenced, grouped per library.
        let import_of: HashMap<&str, &Import> = opts.imports.iter().map(|i| (i.symbol.as_str(), i)).collect();
        let mut used: Vec<&Import> = Vec::new();
        for (fi, f) in files.iter().enumerate() {
            for s in f.sections() {
                if !keep.contains_key(&(fi, s.index())) {
                    continue;
                }
                for (_, r) in s.relocations() {
                    if let RelocationTarget::Symbol(si) = r.target() {
                        let sym = f.symbol_by_index(si).map_err(|e| LinkError(e.to_string()))?;
                        if sym.is_undefined() || sym.section_index().is_none() {
                            let n = sym.name().map_err(|e| LinkError(e.to_string()))?;
                            if defs.contains_key(n) {
                                continue;
                            }
                            match import_of.get(n) {
                                Some(i) if !used.iter().any(|u| u.symbol == i.symbol) => used.push(i),
                                Some(_) => {}
                                None => return err(format!("undefined symbol {n}")),
                            }
                        }
                    }
                }
            }
        }
        let mut libs: Vec<&str> = Vec::new();
        for u in &used {
            if !libs.contains(&u.library.as_str()) {
                libs.push(&u.library);
            }
        }
        for lib in &libs {
            for u in used.iter().filter(|u| u.library == *lib) {
                l.got.push(GotSlot { import: Some(l.imports.len()), local: None });
                l.imports.push((*u).clone());
            }
            l.got.push(GotSlot { import: None, local: None }); // group terminator (IAT null entry)
        }
        let import_idx: HashMap<String, usize> = l.imports.iter().enumerate().map(|(i, im)| (im.symbol.clone(), i)).collect();
        // 4. Place sections. Data starts after the GOT (local GOT slots are added later, so reserve lazily).
        let mut place: HashMap<(usize, object::SectionIndex), (Seg, u64)> = HashMap::new();
        let mut order: Vec<((usize, object::SectionIndex), Seg)> = keep.iter().map(|(k, v)| (*k, *v)).collect();
        order.sort_by_key(|((fi, si), _)| (*fi, si.0));
        for (k, seg) in &order {
            let f = &files[k.0];
            let s = f.section_by_index(k.1).map_err(|e| LinkError(e.to_string()))?;
            let align = s.align().max(1);
            let buf = match seg {
                Seg::Text => &mut l.text,
                Seg::RoData => &mut l.rodata,
                Seg::Data => &mut l.data,
                Seg::Bss => {
                    let at = l.bss.div_ceil(align) * align;
                    l.bss = at + s.size();
                    place.insert(*k, (Seg::Bss, at));
                    continue;
                }
            };
            let at = (buf.len() as u64).div_ceil(align) * align;
            let fill = if *seg == Seg::Text { 0xCC } else { 0 };
            buf.resize(at as usize, if l.arch == Arch::X86_64 { fill } else { 0 });
            buf.extend_from_slice(s.data().map_err(|e| LinkError(e.to_string()))?);
            buf.resize((at + s.size()) as usize, 0);
            place.insert(*k, (*seg, at));
            if s.name() == Ok(".pdata") {
                l.pdata = Some((at, s.size()));
            }
        }
        // 5. Relocations.
        let mut local_got: HashMap<(Seg, u64), usize> = HashMap::new();
        let mut fixes = Vec::new();
        for (k, seg) in &order {
            let f = &files[k.0];
            let s = f.section_by_index(k.1).map_err(|e| LinkError(e.to_string()))?;
            let (_, base) = place[k];
            if *seg == Seg::Bss {
                continue;
            }
            for (off, r) in s.relocations() {
                let at = base + off;
                let (kind, via) = decode(f.format(), l.arch, r.flags(), r.kind(), r.size())?;
                let Some(kind) = kind else { continue };
                // Target
                let sym_target: Result<Tgt, LinkError> = match r.target() {
                    RelocationTarget::Symbol(si) => {
                        let sym = f.symbol_by_index(si).map_err(|e| LinkError(e.to_string()))?;
                        let name = sym.name().unwrap_or("");
                        let local_def = sym.section_index().filter(|_| !sym.is_undefined());
                        if let (Some(sec), true) = (local_def, sym.is_local() || !defs.contains_key(name)) {
                            match place.get(&(k.0, sec)) {
                                Some(&(sg, sb)) => {
                                    let ss = f.section_by_index(sec).map_err(|e| LinkError(e.to_string()))?;
                                    Ok(Tgt::At(sg, sb + sym.address() - ss.address()))
                                }
                                None => err(format!("{name}: symbol in a section that is not linked")),
                            }
                        } else if let Some(&(dfi, dsi, doff)) = defs.get(name) {
                            let (sg, sb) = place[&(dfi, dsi)];
                            Ok(Tgt::At(sg, sb + doff))
                        } else if let Some(&ii) = import_idx.get(name) {
                            Ok(Tgt::Stub(ii))
                        } else {
                            err(format!("undefined symbol {name}"))
                        }
                    }
                    RelocationTarget::Section(si) => match place.get(&(k.0, si)) {
                        Some(&(sg, sb)) => {
                            let ss = f.section_by_index(si).map_err(|e| LinkError(e.to_string()))?;
                            // Section-relative (Mach-O r_extern = 0): the field holds an object address.
                            Ok(Tgt::At(sg, sb.wrapping_sub(ss.address())))
                        }
                        None => err("relocation against a section that is not linked"),
                    },
                    _ => err("unsupported relocation target"),
                };
                let mut tgt = sym_target?;
                let mut addend = r.addend();
                if r.has_implicit_addend() {
                    addend += implicit(seg_buf(&l, *seg), at, kind);
                }
                if via == Via::Got {
                    tgt = match tgt {
                        Tgt::Stub(ii) => Tgt::Got(l.got.iter().position(|g| g.import == Some(ii)).expect("import slot")),
                        Tgt::At(sg, o) => {
                            let n = l.got.len();
                            let slot = *local_got.entry((sg, o)).or_insert_with(|| {
                                l.got.push(GotSlot { import: None, local: Some((sg, o)) });
                                n
                            });
                            Tgt::Got(slot)
                        }
                        g => g,
                    };
                }
                fixes.push(Fix { seg: *seg, off: at, kind, tgt, addend });
            }
        }
        // GOT goes first in the data segment: shift everything placed in data.
        let got_bytes = (l.got.len() as u64) * 8;
        let shift = got_bytes.div_ceil(16) * 16;
        if shift > 0 {
            let mut d = vec![0u8; shift as usize];
            d.extend_from_slice(&l.data);
            l.data = d;
            let mv = |t: &mut Tgt| {
                if let Tgt::At(Seg::Data, o) = t {
                    *o += shift;
                }
            };
            for fx in &mut fixes {
                if fx.seg == Seg::Data {
                    fx.off += shift;
                }
                mv(&mut fx.tgt);
            }
            for g in &mut l.got {
                if let Some((Seg::Data, o)) = &mut g.local {
                    *o += shift;
                }
            }
            for v in place.values_mut() {
                if v.0 == Seg::Data {
                    v.1 += shift;
                }
            }
        }
        l.fixes = fixes;
        // Stubs at the end of the code segment.
        for i in 0..l.imports.len() {
            let at = (l.text.len() as u64).div_ceil(16) * 16;
            l.text.resize(at as usize, if l.arch == Arch::X86_64 { 0xCC } else { 0 });
            let slot = l.got.iter().position(|g| g.import == Some(i)).expect("slot");
            match l.arch {
                Arch::X86_64 => {
                    l.text.extend_from_slice(&STUB_X86);
                    l.fixes.push(Fix { seg: Seg::Text, off: at + 2, kind: Kind::Pc32, tgt: Tgt::Got(slot), addend: -4 });
                }
                Arch::Aarch64 => {
                    l.text.extend_from_slice(&STUB_A64);
                    l.fixes.push(Fix { seg: Seg::Text, off: at, kind: Kind::Page21, tgt: Tgt::Got(slot), addend: 0 });
                    l.fixes.push(Fix { seg: Seg::Text, off: at + 4, kind: Kind::Lo12 { shift: 3 }, tgt: Tgt::Got(slot), addend: 0 });
                }
            }
            l.stubs.push(at);
        }
        // Entry.
        let Some(&(efi, esi, eoff)) = defs.get(&opts.entry) else {
            return err(format!("entry symbol {} not defined", opts.entry));
        };
        let (es, eb) = place[&(efi, esi)];
        l.entry = (es, eb + eoff);
        Ok(l)
    }

    /// Patches every relocation for the segment addresses `va` (text,
    /// rodata, data, bss) and the image base (for `Rva32`). Calls `abs` with
    /// every absolute 64-bit pointer written (segment, offset): loaders that
    /// may move the image need them (PE base relocations, Mach-O rebase).
    pub fn apply(&mut self, va: &Vas, abs: &mut dyn FnMut(Seg, u64)) -> Result<(), LinkError> {
        let got_va = va.data;
        // Local GOT slots hold absolute addresses.
        for (i, g) in self.got.iter().enumerate() {
            if let Some((s, o)) = g.local {
                let v = va.of(s) + o;
                self.data[i * 8..i * 8 + 8].copy_from_slice(&v.to_le_bytes());
                abs(Seg::Data, (i * 8) as u64);
            }
        }
        let fixes = std::mem::take(&mut self.fixes);
        for fx in &fixes {
            let s = match fx.tgt {
                Tgt::At(sg, o) => va.of(sg).wrapping_add(o),
                Tgt::Got(i) => got_va + (i as u64) * 8,
                Tgt::Stub(i) => va.text + self.stubs[i],
            } as i64;
            let p = (va.of(fx.seg) + fx.off) as i64;
            let a = fx.addend;
            let buf = match fx.seg {
                Seg::Text => &mut self.text,
                Seg::RoData => &mut self.rodata,
                Seg::Data => &mut self.data,
                Seg::Bss => return err("relocation in zero-fill data"),
            };
            let o = fx.off as usize;
            match fx.kind {
                Kind::Abs64 => {
                    buf[o..o + 8].copy_from_slice(&(s + a).to_le_bytes());
                    abs(fx.seg, fx.off);
                }
                Kind::Abs32 => {
                    let v = u32::try_from(s + a).map_err(|_| LinkError(format!("32-bit absolute address out of range at {o:#x}")))?;
                    buf[o..o + 4].copy_from_slice(&v.to_le_bytes());
                }
                Kind::Rva32 => {
                    let v = u32::try_from(s + a - va.image as i64).map_err(|_| LinkError("RVA out of range".into()))?;
                    buf[o..o + 4].copy_from_slice(&v.to_le_bytes());
                }
                Kind::Pc32 => {
                    let v = i32::try_from(s + a - p).map_err(|_| LinkError(format!("PC-relative target out of range at {o:#x}")))?;
                    buf[o..o + 4].copy_from_slice(&v.to_le_bytes());
                }
                Kind::Call26 => {
                    let v = s + a - p;
                    if v & 3 != 0 || !(-(1 << 27)..(1 << 27)).contains(&v) {
                        return err(format!("branch out of range at {o:#x}"));
                    }
                    let w = u32::from_le_bytes(buf[o..o + 4].try_into().expect("4"));
                    let w = (w & 0xFC00_0000) | (((v >> 2) as u32) & 0x03FF_FFFF);
                    buf[o..o + 4].copy_from_slice(&w.to_le_bytes());
                }
                Kind::Page21 => {
                    let d = ((s + a) & !0xFFF) - (p & !0xFFF);
                    let pages = d >> 12;
                    if !(-(1 << 20)..(1 << 20)).contains(&pages) {
                        return err(format!("ADRP out of range at {o:#x}"));
                    }
                    let imm = pages as u32 & 0x1F_FFFF;
                    let w = u32::from_le_bytes(buf[o..o + 4].try_into().expect("4"));
                    let w = (w & 0x9F00_001F) | ((imm & 3) << 29) | ((imm >> 2) << 5);
                    buf[o..o + 4].copy_from_slice(&w.to_le_bytes());
                }
                Kind::Lo12 { shift } => {
                    let lo = ((s + a) & 0xFFF) as u32;
                    if lo & ((1 << shift) - 1) != 0 {
                        return err(format!("misaligned low-12 offset at {o:#x}"));
                    }
                    let imm = lo >> shift;
                    let w = u32::from_le_bytes(buf[o..o + 4].try_into().expect("4"));
                    let w = (w & !(0xFFF << 10)) | (imm << 10);
                    buf[o..o + 4].copy_from_slice(&w.to_le_bytes());
                }
            }
        }
        Ok(())
    }

    /// Bytes of the GOT (the first part of `data`).
    pub fn got_bytes(&self) -> u64 {
        self.got.len() as u64 * 8
    }
}

/// Addresses chosen by a writer.
#[derive(Debug, Clone, Copy, Default)]
pub struct Vas {
    pub image: u64,
    pub text: u64,
    pub rodata: u64,
    pub data: u64,
    pub bss: u64,
}

impl Vas {
    pub fn of(&self, s: Seg) -> u64 {
        match s {
            Seg::Text => self.text,
            Seg::RoData => self.rodata,
            Seg::Data => self.data,
            Seg::Bss => self.bss,
        }
    }
}

fn seg_buf(l: &Linked, s: Seg) -> &[u8] {
    match s {
        Seg::Text => &l.text,
        Seg::RoData => &l.rodata,
        Seg::Data => &l.data,
        Seg::Bss => &[],
    }
}

fn implicit(buf: &[u8], at: u64, kind: Kind) -> i64 {
    let o = at as usize;
    match kind {
        Kind::Abs64 if o + 8 <= buf.len() => i64::from_le_bytes(buf[o..o + 8].try_into().expect("8")),
        Kind::Abs32 | Kind::Rva32 | Kind::Pc32 if o + 4 <= buf.len() => {
            i64::from(i32::from_le_bytes(buf[o..o + 4].try_into().expect("4")))
        }
        _ => 0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Via {
    Direct,
    Got,
}

/// Maps a format-specific relocation to (kind, through the GOT?). `None` kind = ignore.
fn decode(
    fmt: object::BinaryFormat,
    arch: Arch,
    flags: RelocationFlags,
    kind: object::RelocationKind,
    size: u8,
) -> Result<(Option<Kind>, Via), LinkError> {
    use Via::*;
    let k = match flags {
        RelocationFlags::Coff { typ } => match (arch, typ) {
            (Arch::X86_64, object::pe::IMAGE_REL_AMD64_ADDR64) => (Kind::Abs64, Direct),
            (Arch::X86_64, object::pe::IMAGE_REL_AMD64_ADDR32) => (Kind::Abs32, Direct),
            (Arch::X86_64, object::pe::IMAGE_REL_AMD64_ADDR32NB) => (Kind::Rva32, Direct),
            (Arch::X86_64, t) if (object::pe::IMAGE_REL_AMD64_REL32..=object::pe::IMAGE_REL_AMD64_REL32_5).contains(&t) => {
                (Kind::Pc32, Direct)
            }
            (_, object::pe::IMAGE_REL_AMD64_ABSOLUTE) => return Ok((None, Direct)),
            (_, t) => return err(format!("unsupported COFF relocation type {t}")),
        },
        RelocationFlags::Elf { r_type } => match (arch, r_type) {
            (Arch::X86_64, object::elf::R_X86_64_64) => (Kind::Abs64, Direct),
            (Arch::X86_64, object::elf::R_X86_64_32 | object::elf::R_X86_64_32S) => (Kind::Abs32, Direct),
            (Arch::X86_64, object::elf::R_X86_64_PC32 | object::elf::R_X86_64_PLT32) => (Kind::Pc32, Direct),
            (
                Arch::X86_64,
                object::elf::R_X86_64_GOTPCREL | object::elf::R_X86_64_GOTPCRELX | object::elf::R_X86_64_REX_GOTPCRELX,
            ) => (Kind::Pc32, Got),
            (Arch::Aarch64, object::elf::R_AARCH64_ABS64) => (Kind::Abs64, Direct),
            (Arch::Aarch64, object::elf::R_AARCH64_PREL32) => (Kind::Pc32, Direct),
            (Arch::Aarch64, object::elf::R_AARCH64_CALL26 | object::elf::R_AARCH64_JUMP26) => (Kind::Call26, Direct),
            (Arch::Aarch64, object::elf::R_AARCH64_ADR_PREL_PG_HI21) => (Kind::Page21, Direct),
            (Arch::Aarch64, object::elf::R_AARCH64_ADD_ABS_LO12_NC | object::elf::R_AARCH64_LDST8_ABS_LO12_NC) => {
                (Kind::Lo12 { shift: 0 }, Direct)
            }
            (Arch::Aarch64, object::elf::R_AARCH64_LDST16_ABS_LO12_NC) => (Kind::Lo12 { shift: 1 }, Direct),
            (Arch::Aarch64, object::elf::R_AARCH64_LDST32_ABS_LO12_NC) => (Kind::Lo12 { shift: 2 }, Direct),
            (Arch::Aarch64, object::elf::R_AARCH64_LDST64_ABS_LO12_NC) => (Kind::Lo12 { shift: 3 }, Direct),
            (Arch::Aarch64, object::elf::R_AARCH64_LDST128_ABS_LO12_NC) => (Kind::Lo12 { shift: 4 }, Direct),
            (Arch::Aarch64, object::elf::R_AARCH64_ADR_GOT_PAGE) => (Kind::Page21, Got),
            (Arch::Aarch64, object::elf::R_AARCH64_LD64_GOT_LO12_NC) => (Kind::Lo12 { shift: 3 }, Got),
            (_, object::elf::R_X86_64_NONE) => return Ok((None, Direct)),
            (_, t) => return err(format!("unsupported ELF relocation type {t}")),
        },
        RelocationFlags::MachO { r_type, r_pcrel, r_length } => match (arch, r_type, r_pcrel, r_length) {
            (_, object::macho::X86_64_RELOC_UNSIGNED, false, 3) => (Kind::Abs64, Direct),
            (_, object::macho::X86_64_RELOC_UNSIGNED, false, 2) => (Kind::Abs32, Direct),
            (Arch::X86_64, t, true, 2)
                if [
                    object::macho::X86_64_RELOC_SIGNED,
                    object::macho::X86_64_RELOC_BRANCH,
                    object::macho::X86_64_RELOC_SIGNED_1,
                    object::macho::X86_64_RELOC_SIGNED_2,
                    object::macho::X86_64_RELOC_SIGNED_4,
                ]
                .contains(&t) =>
            {
                (Kind::Pc32, Direct)
            }
            (Arch::X86_64, object::macho::X86_64_RELOC_GOT_LOAD | object::macho::X86_64_RELOC_GOT, true, 2) => {
                (Kind::Pc32, Got)
            }
            (Arch::Aarch64, object::macho::ARM64_RELOC_BRANCH26, true, 2) => (Kind::Call26, Direct),
            (Arch::Aarch64, object::macho::ARM64_RELOC_PAGE21, true, 2) => (Kind::Page21, Direct),
            (Arch::Aarch64, object::macho::ARM64_RELOC_PAGEOFF12, false, 2) => (Kind::Lo12 { shift: 0xFF }, Direct),
            (Arch::Aarch64, object::macho::ARM64_RELOC_GOT_LOAD_PAGE21, true, 2) => (Kind::Page21, Got),
            (Arch::Aarch64, object::macho::ARM64_RELOC_GOT_LOAD_PAGEOFF12, false, 2) => (Kind::Lo12 { shift: 3 }, Got),
            (Arch::Aarch64, object::macho::ARM64_RELOC_POINTER_TO_GOT, true, 2) => (Kind::Pc32, Got),
            (_, t, pc, len) => return err(format!("unsupported Mach-O relocation type {t} (pcrel {pc}, length {len})")),
        },
        RelocationFlags::Generic { .. } => {
            return err(format!("unsupported relocation {kind:?}/{size} in {fmt:?}"));
        }
        _ => return err(format!("unsupported relocation in {fmt:?}")),
    };
    Ok((Some(k.0), k.1))
}

/// Mach-O PAGEOFF12 does not say the access size: read it from the instruction.
pub(crate) fn lo12_shift_of(insn: u32) -> u8 {
    // Load/store (unsigned immediate): size in bits 31:30, V bit 26, opc 23:22.
    if insn & 0x3B00_0000 == 0x3900_0000 {
        let size = (insn >> 30) as u8;
        let v = (insn >> 26) & 1;
        let opc = (insn >> 22) & 3;
        if v == 1 && size == 0 && opc & 2 != 0 {
            return 4; // 128-bit SIMD
        }
        return size;
    }
    0 // ADD (immediate)
}

impl Linked {
    /// Resolves Mach-O PAGEOFF12 access sizes (marked `shift: 0xFF` by `decode`).
    pub(crate) fn fix_pageoff_shifts(&mut self) {
        for fx in &mut self.fixes {
            if let Kind::Lo12 { shift: 0xFF } = fx.kind {
                let buf = match fx.seg {
                    Seg::Text => &self.text,
                    Seg::RoData => &self.rodata,
                    _ => &self.data,
                };
                let o = fx.off as usize;
                let w = u32::from_le_bytes(buf[o..o + 4].try_into().expect("4"));
                fx.kind = Kind::Lo12 { shift: lo12_shift_of(w) };
            }
        }
    }
}

pub(crate) fn align(v: u64, a: u64) -> u64 {
    v.div_ceil(a) * a
}
