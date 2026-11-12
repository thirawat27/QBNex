//! `qb-link` — QBNex's own linker/loader for native program code (D15).
//!
//! Cranelift (`cranelift-object`) turns a program into an object file
//! (COFF on Windows, ELF on Linux, Mach-O on macOS). [`link`] lays its
//! allocated sections out into one block and records every relocation:
//! those into the runtime by *name*, those inside the program by offset. The result, an [`Image`], is a
//! small byte format ([`Image::to_bytes`]) that a QBNex executable carries
//! next to its program. At start-up [`load`] copies the image into
//! executable memory and patches the runtime addresses of that very process.
//!
//! So no external linker, no C toolchain and no code generation at run
//! time: the program's machine code was produced when it was built.
//! Supported relocations: 64-bit absolute, 32-bit PC-relative (x86-64) and
//! the AArch64 26-bit call.

use object::{Object, ObjectSection, ObjectSymbol, RelocationEncoding, RelocationKind, RelocationTarget, SectionKind};
use std::collections::HashMap;

/// Standard-linker path (rust-lld + native kit), see [`toolchain`].
#[cfg(not(target_arch = "wasm32"))]
pub mod toolchain;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `S + A` as 64 bits.
    Abs64,
    /// `S + A - P` as 32 bits.
    Rel32,
    /// AArch64 `BL`/`B`: `(S + A - P) >> 2` in the low 26 bits.
    Call26,
}

impl Kind {
    /// Bytes of code a relocation of this kind patches.
    fn size(self) -> u64 {
        match self {
            Kind::Abs64 => 8,
            Kind::Rel32 | Kind::Call26 => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A function of the runtime, resolved at load time.
    External(String),
    /// An offset inside the image.
    Local(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reloc {
    pub offset: u64,
    pub kind: Kind,
    pub target: Target,
    pub addend: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Image {
    /// Machine code and read-only data of the program, laid out.
    pub code: Vec<u8>,
    pub relocs: Vec<Reloc>,
    /// Exported symbols (functions) and their offsets.
    pub symbols: Vec<(String, u64)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkError(pub String);

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "qb-link: {}", self.0)
    }
}

impl std::error::Error for LinkError {}

fn err<T>(m: impl Into<String>) -> Result<T, LinkError> {
    Err(LinkError(m.into()))
}

/// Lays out the object file `obj` into an [`Image`].
pub fn link(obj: &[u8]) -> Result<Image, LinkError> {
    let file = object::File::parse(obj).map_err(|e| LinkError(e.to_string()))?;
    let mut img = Image::default();
    // Section placement (only sections loaded at run time).
    let mut place: HashMap<object::SectionIndex, u64> = HashMap::new();
    for sec in file.sections() {
        let keep = matches!(
            sec.kind(),
            SectionKind::Text | SectionKind::ReadOnlyData | SectionKind::ReadOnlyString | SectionKind::Data
        );
        if !keep || sec.size() == 0 {
            continue;
        }
        let align = sec.align().max(16);
        let at = (img.code.len() as u64).div_ceil(align) * align;
        img.code.resize(at as usize, 0);
        let data = sec.data().map_err(|e| LinkError(e.to_string()))?;
        img.code.extend_from_slice(data);
        img.code.resize((at + sec.size()) as usize, 0);
        place.insert(sec.index(), at);
    }
    for sec in file.sections() {
        let Some(&base) = place.get(&sec.index()) else { continue };
        for (off, r) in sec.relocations() {
            let kind = match (r.kind(), r.encoding(), r.size()) {
                (RelocationKind::Absolute, _, 64) => Kind::Abs64,
                (RelocationKind::Relative, RelocationEncoding::Generic, 32)
                | (RelocationKind::Relative, RelocationEncoding::X86Branch, 32)
                | (RelocationKind::PltRelative, _, 32) => Kind::Rel32,
                (RelocationKind::Relative, RelocationEncoding::AArch64Call, 26)
                | (RelocationKind::PltRelative, RelocationEncoding::AArch64Call, 26) => Kind::Call26,
                other => return err(format!("unsupported relocation {other:?} at {off:#x}")),
            };
            let at = base + off;
            let mut addend = r.addend();
            if r.has_implicit_addend() {
                let p = at as usize;
                addend += match kind {
                    Kind::Abs64 => i64::from_le_bytes(img.code[p..p + 8].try_into().expect("8 bytes")),
                    Kind::Rel32 => i64::from(i32::from_le_bytes(img.code[p..p + 4].try_into().expect("4 bytes"))),
                    Kind::Call26 => {
                        let w = u32::from_le_bytes(img.code[p..p + 4].try_into().expect("4 bytes"));
                        (i64::from((w << 6) as i32) >> 6) << 2
                    }
                };
            }
            let target = match r.target() {
                RelocationTarget::Symbol(si) => {
                    let sym = file.symbol_by_index(si).map_err(|e| LinkError(e.to_string()))?;
                    match sym.section_index().and_then(|s| place.get(&s)) {
                        Some(&sb) => Target::Local(sb + sym.address() - section_address(&file, sym.section_index())),
                        None if sym.is_undefined() => {
                            Target::External(plain_name(&file, sym.name().map_err(|e| LinkError(e.to_string()))?).to_string())
                        }
                        None => return err(format!("symbol {:?} in a section that is not loaded", sym.name())),
                    }
                }
                // Mach-O section relocations carry the target's address in the
                // object's address space as the addend: rebase it on the section.
                RelocationTarget::Section(si) => match place.get(&si) {
                    Some(&sb) => Target::Local(sb.wrapping_sub(section_address(&file, Some(si)))),
                    None => return err("relocation against a section that is not loaded"),
                },
                _ => return err("unsupported relocation target"),
            };
            img.relocs.push(Reloc { offset: at, kind, target, addend });
        }
    }
    for sym in file.symbols() {
        if sym.is_definition()
            && sym.kind() == object::SymbolKind::Text
            && let Some(&sb) = sym.section_index().and_then(|s| place.get(&s))
        {
            let name = sym.name().map_err(|e| LinkError(e.to_string()))?;
            img.symbols.push((plain_name(&file, name).to_string(), sb + sym.address() - section_address(&file, sym.section_index())));
        }
    }
    Ok(img)
}

/// Symbol name without the object format's C prefix: Mach-O stores every C
/// name with a leading `_` (`qb_func_0` is `_qb_func_0` in the file), while
/// the image and the runtime use the plain name.
fn plain_name<'a>(file: &object::File, name: &'a str) -> &'a str {
    if file.format() == object::BinaryFormat::MachO {
        name.strip_prefix('_').unwrap_or(name)
    } else {
        name
    }
}

fn section_address(file: &object::File, s: Option<object::SectionIndex>) -> u64 {
    s.and_then(|i| file.section_by_index(i).ok()).map_or(0, |s| s.address())
}

/// Writes relocation `r` given the target value `s` and the image load address `base`.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn patch(code: &mut [u8], r: &Reloc, s: i64, base: i64) -> Result<(), LinkError> {
    let p = r.offset;
    let at = base.wrapping_add(r.offset as i64);
    // the bytes this relocation patches, inside the code
    let f = match usize::try_from(p).ok().and_then(|q| Some(q..q.checked_add(r.kind.size() as usize)?)) {
        Some(range) if range.end <= code.len() => &mut code[range],
        _ => return err(format!("relocation outside the image at {p:#x}")),
    };
    match r.kind {
        Kind::Abs64 => f.copy_from_slice(&s.wrapping_add(r.addend).to_le_bytes()),
        Kind::Rel32 => {
            let v = s.wrapping_add(r.addend).wrapping_sub(at);
            let v = i32::try_from(v).map_err(|_| LinkError(format!("PC-relative target out of range at {p:#x}")))?;
            f.copy_from_slice(&v.to_le_bytes());
        }
        Kind::Call26 => {
            let v = s.wrapping_add(r.addend).wrapping_sub(at);
            if v & 3 != 0 || !(-(1 << 27)..(1 << 27)).contains(&v) {
                return err(format!("AArch64 call out of range at {p:#x}"));
            }
            let w = u32::from_le_bytes(f[..4].try_into().expect("4 bytes"));
            let w = (w & 0xFC00_0000) | (((v >> 2) as u32) & 0x03FF_FFFF);
            f.copy_from_slice(&w.to_le_bytes());
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- byte format

const MAGIC: &[u8; 8] = b"QBNXIMG1";

impl Image {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut o = Vec::new();
        o.extend_from_slice(MAGIC);
        let put = |o: &mut Vec<u8>, v: u64| o.extend_from_slice(&v.to_le_bytes());
        let put_s = |o: &mut Vec<u8>, s: &str| {
            o.extend_from_slice(&(s.len() as u64).to_le_bytes());
            o.extend_from_slice(s.as_bytes());
        };
        put(&mut o, self.code.len() as u64);
        o.extend_from_slice(&self.code);
        put(&mut o, self.relocs.len() as u64);
        for r in &self.relocs {
            put(&mut o, r.offset);
            o.push(match r.kind {
                Kind::Abs64 => 0,
                Kind::Rel32 => 1,
                Kind::Call26 => 2,
            });
            put(&mut o, r.addend as u64);
            match &r.target {
                Target::External(n) => {
                    o.push(0);
                    put_s(&mut o, n)
                }
                Target::Local(t) => {
                    o.push(1);
                    put(&mut o, *t)
                }
            }
        }
        put(&mut o, self.symbols.len() as u64);
        for (n, off) in &self.symbols {
            put_s(&mut o, n);
            put(&mut o, *off);
        }
        o
    }

    pub fn from_bytes(b: &[u8]) -> Result<Image, LinkError> {
        let mut r = Rd { b, p: 0 };
        if r.take(8)? != MAGIC {
            return err("not a QBNex image");
        }
        let n = r.u64()? as usize;
        let code = r.take(n)?.to_vec();
        let mut relocs = Vec::new();
        for _ in 0..r.u64()? {
            let offset = r.u64()?;
            let kind = match r.take(1)?[0] {
                0 => Kind::Abs64,
                1 => Kind::Rel32,
                2 => Kind::Call26,
                k => return err(format!("bad relocation kind {k}")),
            };
            let addend = r.u64()? as i64;
            let target = match r.take(1)?[0] {
                0 => Target::External(r.str()?),
                1 => Target::Local(r.u64()?),
                t => return err(format!("bad relocation target {t}")),
            };
            // the patched bytes (8 for Abs64) must lie inside the code
            if offset.checked_add(kind.size()).is_none_or(|e| e > code.len() as u64) {
                return err("relocation outside the image");
            }
            relocs.push(Reloc { offset, kind, target, addend });
        }
        let mut symbols = Vec::new();
        for _ in 0..r.u64()? {
            let n = r.str()?;
            let off = r.u64()?;
            if off >= code.len() as u64 {
                return err(format!("symbol {n} outside the image"));
            }
            symbols.push((n, off));
        }
        Ok(Image { code, relocs, symbols })
    }
}

struct Rd<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Rd<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], LinkError> {
        let s = self.b.get(self.p..self.p.checked_add(n).ok_or(LinkError("truncated image".into()))?);
        let s = s.ok_or(LinkError("truncated image".into()))?;
        self.p += n;
        Ok(s)
    }
    fn u64(&mut self) -> Result<u64, LinkError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().expect("8 bytes")))
    }
    fn str(&mut self) -> Result<String, LinkError> {
        let n = self.u64()? as usize;
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| LinkError("bad name".into()))
    }
}

// ---------------------------------------------------------------- loading

/// An image in executable memory.
#[cfg(not(target_arch = "wasm32"))]
pub struct Loaded {
    mem: region::Allocation,
    symbols: HashMap<String, u64>,
}

/// Browser build: wasm cannot run machine code, so nothing loads.
#[cfg(target_arch = "wasm32")]
pub struct Loaded {
    _never: core::convert::Infallible,
}

#[cfg(target_arch = "wasm32")]
impl Loaded {
    /// Address of an exported function (never: nothing loads on wasm).
    pub fn symbol(&self, _name: &str) -> Option<*const u8> {
        match self._never {}
    }
}

/// Native code cannot be loaded into a browser.
#[cfg(target_arch = "wasm32")]
pub fn load(_img: &Image, _resolve: &dyn Fn(&str) -> Option<usize>) -> Result<Loaded, LinkError> {
    Err(LinkError("native code images cannot run in a browser".into()))
}

// SAFETY: the code block is immutable after loading.
#[cfg(not(target_arch = "wasm32"))]
unsafe impl Send for Loaded {}
#[cfg(not(target_arch = "wasm32"))]
unsafe impl Sync for Loaded {}

#[cfg(not(target_arch = "wasm32"))]
impl Loaded {
    /// Address of an exported function.
    pub fn symbol(&self, name: &str) -> Option<*const u8> {
        let off = *self.symbols.get(name)?;
        // SAFETY: offsets come from the image and lie inside the allocation.
        Some(unsafe { self.mem.as_ptr::<u8>().add(off as usize) })
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// Copies `img` into executable memory, resolving every external name with `resolve`.
pub fn load(img: &Image, resolve: &dyn Fn(&str) -> Option<usize>) -> Result<Loaded, LinkError> {
    if let Some((n, _)) = img.symbols.iter().find(|(_, off)| *off >= img.code.len() as u64) {
        return err(format!("symbol {n} outside the image"));
    }
    let mut code = img.code.clone();
    let size = code.len().max(1);
    let mut mem =
        region::alloc(size, region::Protection::READ_WRITE).map_err(|e| LinkError(format!("allocating {size} bytes for the image: {e}")))?;
    let base = mem.as_mut_ptr::<u8>() as i64;
    for r in &img.relocs {
        let s = match &r.target {
            Target::External(name) => {
                resolve(name).ok_or_else(|| LinkError(format!("unresolved runtime symbol {name}")))? as i64
            }
            Target::Local(t) => base + *t as i64,
        };
        patch(&mut code, r, s, base)?;
    }
    // SAFETY: the allocation has `size` bytes and nothing else refers to it yet.
    unsafe {
        core::ptr::copy_nonoverlapping(code.as_ptr(), mem.as_mut_ptr::<u8>(), code.len());
        region::protect(mem.as_ptr::<u8>(), size, region::Protection::READ_EXECUTE)
            .map_err(|e| LinkError(format!("making the image executable: {e}")))?;
    }
    flush_icache(mem.as_ptr::<u8>(), size);
    Ok(Loaded { mem, symbols: img.symbols.iter().cloned().collect() })
}

#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
fn flush_icache(p: *const u8, n: usize) {
    unsafe extern "C" {
        fn sys_icache_invalidate(start: *mut core::ffi::c_void, len: usize);
    }
    // SAFETY: range inside the allocation.
    unsafe { sys_icache_invalidate(p as *mut _, n) };
}

#[cfg(all(target_arch = "aarch64", not(target_os = "macos")))]
fn flush_icache(p: *const u8, n: usize) {
    // AArch64 needs the instruction cache synchronised with the data writes.
    unsafe extern "C" {
        fn __clear_cache(start: *mut core::ffi::c_char, end: *mut core::ffi::c_char);
    }
    // SAFETY: range inside the allocation.
    unsafe { __clear_cache(p as *mut _, p.add(n) as *mut _) };
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "wasm32")))]
fn flush_icache(_p: *const u8, _n: usize) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn image() -> Image {
        Image {
            code: vec![0; 16],
            relocs: vec![Reloc { offset: 8, kind: Kind::Abs64, target: Target::Local(0), addend: 0 }],
            symbols: vec![("f".into(), 0)],
        }
    }

    #[test]
    fn bytes_round_trip() {
        assert_eq!(Image::from_bytes(&image().to_bytes()), Ok(image()));
    }

    #[test]
    fn truncated_or_inconsistent_images_are_errors_not_panics() {
        let b = image().to_bytes();
        for n in 0..b.len() {
            assert!(Image::from_bytes(&b[..n]).is_err(), "prefix of {n} bytes");
        }
        // an 8-byte relocation ending past the code (12 + 8 > 16), and one whose end overflows
        for offset in [12, u64::MAX - 2] {
            let mut i = image();
            i.relocs[0].offset = offset;
            assert!(Image::from_bytes(&i.to_bytes()).is_err(), "relocation at {offset}");
            #[cfg(not(target_arch = "wasm32"))]
            assert!(load(&i, &|_| None).is_err(), "load, relocation at {offset}");
        }
        // a symbol outside the code
        let mut i = image();
        i.symbols[0].1 = 16;
        assert!(Image::from_bytes(&i.to_bytes()).is_err());
        #[cfg(not(target_arch = "wasm32"))]
        assert!(load(&i, &|_| None).is_err());
    }
}
