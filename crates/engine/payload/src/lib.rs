//! Layout of a compiled QBNex program: a front part, then the program.
//!
//! The front part is one of
//! - the runner `qb-exec` itself;
//! - the launcher `qb-stub` followed by the packed runner (see [`PackedRunner`]):
//!   `<brotli stream><u64 its length><u64 runner length><u64 runner id><u64 filter>QBNEXRB1`.
//!   The launcher unpacks the runner once into a cache folder and starts it from there.
//!
//! The program is one of
//! - `<zlib stream><u64 its length><u64 QIR length>QBNEXIZ0`: what `qb` writes
//!   (the QIR text compressed);
//! - `<QIR text><u64 length>QBNEXIR0`: the same uncompressed (setup's stage-0
//!   compiler, `scripts/qir_pack.ps1`);
//! - `<QIR text><native image><u64 QIR length><u64 image length>QBNEXAO1`:
//!   `qb-exec build` with a native image.
//!
//! Every number is little endian. Both parts are found from the end of the file,
//! so a reader needs only the last few dozen bytes to know where each one starts.

pub const MAGIC_IR: &[u8; 8] = b"QBNEXIR0";
pub const MAGIC_IR_ZLIB: &[u8; 8] = b"QBNEXIZ0";
pub const MAGIC_NATIVE: &[u8; 8] = b"QBNEXAO1";
pub const MAGIC_RUNNER: &[u8; 8] = b"QBNEXRB1";

/// Bytes at the end of a file that hold the longest program trailer.
pub const PROGRAM_TRAILER_MAX: usize = 24;
/// Bytes of the packed runner's trailer.
pub const RUNNER_TRAILER: usize = 40;

/// The program a compiled executable carries.
#[derive(Debug, PartialEq, Eq)]
pub enum Program<'a> {
    /// QIR text.
    Ir(&'a [u8]),
    /// QIR text as a zlib stream, and the length of the text.
    IrZlib { stream: &'a [u8], len: usize },
    /// QIR text and the native image built from it.
    Native { qir: &'a [u8], image: &'a [u8] },
}

/// Where the program's parts lie in a file of `file_len` bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Span {
    Ir { at: u64, len: u64 },
    IrZlib { at: u64, stream_len: u64, len: u64 },
    Native { at: u64, qir_len: u64, image_len: u64 },
}

impl Span {
    /// Offset of the program: everything before it is the front part.
    pub fn start(self) -> u64 {
        match self {
            Span::Ir { at, .. } | Span::IrZlib { at, .. } | Span::Native { at, .. } => at,
        }
    }
}

fn u64_at(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

/// The program's span from `tail`, the last bytes of a file of `file_len` bytes
/// (at least [`PROGRAM_TRAILER_MAX`] of them when the file is that long). `None`
/// when the file carries no program or lengths that do not fit inside it.
pub fn program_span(tail: &[u8], file_len: u64) -> Option<Span> {
    let t = tail.len();
    let magic = tail.get(t.checked_sub(8)?..)?;
    let field = |k: usize| u64_at(tail, t.checked_sub(8 + 8 * k)?);
    if magic == MAGIC_NATIVE || magic == MAGIC_IR_ZLIB {
        let (a, b) = (field(2)?, field(1)?);
        let end = file_len.checked_sub(24)?;
        if magic == MAGIC_NATIVE {
            let at = end.checked_sub(b)?.checked_sub(a)?;
            return Some(Span::Native { at, qir_len: a, image_len: b });
        }
        let at = end.checked_sub(a)?;
        return Some(Span::IrZlib { at, stream_len: a, len: b });
    }
    if magic == MAGIC_IR {
        let len = field(1)?;
        let at = file_len.checked_sub(16)?.checked_sub(len)?;
        return Some(Span::Ir { at, len });
    }
    None
}

/// The program in `exe` and the offset where it starts.
pub fn program(exe: &[u8]) -> Option<(Program<'_>, usize)> {
    let n = exe.len();
    let span = program_span(&exe[n.saturating_sub(PROGRAM_TRAILER_MAX)..], n as u64)?;
    let at = usize::try_from(span.start()).ok()?;
    let len = |v: u64| usize::try_from(v).ok();
    let p = match span {
        Span::Ir { len: l, .. } => Program::Ir(exe.get(at..at + len(l)?)?),
        Span::IrZlib { stream_len, len: l, .. } => {
            Program::IrZlib { stream: exe.get(at..at + len(stream_len)?)?, len: len(l)? }
        }
        Span::Native { qir_len, image_len, .. } => {
            let q = at + len(qir_len)?;
            Program::Native { qir: exe.get(at..q)?, image: exe.get(q..q + len(image_len)?)? }
        }
    };
    Some((p, at))
}

/// Reversible transform applied to a runner before it is compressed: branch
/// targets become absolute, so the many calls to one function look alike.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    None = 0,
    /// x86-64 `CALL rel32` / `JMP rel32`.
    X86 = 1,
    /// AArch64 `BL imm26`.
    Arm64 = 2,
}

impl Filter {
    pub fn from_u64(v: u64) -> Option<Filter> {
        match v {
            0 => Some(Filter::None),
            1 => Some(Filter::X86),
            2 => Some(Filter::Arm64),
            _ => None,
        }
    }

    /// The filter for the machine an executable (PE, ELF or Mach-O) was built for.
    pub fn for_executable(exe: &[u8]) -> Filter {
        let u16_at = |at: usize| exe.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
        let u32_at = |at: usize| exe.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
        let machine = if exe.starts_with(b"MZ") {
            // PE: the machine field follows the "PE\0\0" signature
            let pe = u32_at(0x3C).unwrap_or(0) as usize;
            match u16_at(pe + 4) {
                Some(0x8664) => Some(Filter::X86),
                Some(0xAA64) => Some(Filter::Arm64),
                _ => None,
            }
        } else if exe.starts_with(b"\x7fELF") {
            match u16_at(18) {
                Some(62) => Some(Filter::X86),
                Some(183) => Some(Filter::Arm64),
                _ => None,
            }
        } else if u32_at(0) == Some(0xFEED_FACF) {
            match u32_at(4) {
                Some(0x0100_0007) => Some(Filter::X86),
                Some(0x0100_000C) => Some(Filter::Arm64),
                _ => None,
            }
        } else {
            None
        };
        machine.unwrap_or(Filter::None)
    }

    /// Applies the transform (`encode`) or takes it back. Which bytes change
    /// depends only on bytes the transform leaves alone, so decoding finds the
    /// same places.
    pub fn apply(self, buf: &mut [u8], encode: bool) {
        match self {
            Filter::None => {}
            Filter::X86 => {
                let mut i = 0usize;
                while i + 5 <= buf.len() {
                    if buf[i] == 0xE8 || buf[i] == 0xE9 {
                        let v = u32::from_le_bytes([buf[i + 1], buf[i + 2], buf[i + 3], buf[i + 4]]);
                        let pos = (i + 5) as u32;
                        let n = if encode { v.wrapping_add(pos) } else { v.wrapping_sub(pos) };
                        buf[i + 1..i + 5].copy_from_slice(&n.to_le_bytes());
                        i += 5;
                    } else {
                        i += 1;
                    }
                }
            }
            Filter::Arm64 => {
                for (k, w) in buf.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                    let v = u32::from_le_bytes(*w);
                    if v & 0xFC00_0000 == 0x9400_0000 {
                        let pos = k as u32;
                        let imm = if encode { v.wrapping_add(pos) } else { v.wrapping_sub(pos) };
                        let n = 0x9400_0000 | (imm & 0x03FF_FFFF);
                        w.copy_from_slice(&n.to_le_bytes());
                    }
                }
            }
        }
    }
}

/// The packed runner at the end of a launcher's front part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackedRunner {
    /// Offset of the brotli stream.
    pub at: u64,
    pub packed_len: u64,
    pub runner_len: u64,
    /// Names the unpacked runner in the cache: one id per runner build.
    pub id: u64,
    pub filter: Filter,
}

/// The packed runner whose trailer is `tail` (the last [`RUNNER_TRAILER`] bytes of
/// a front part `front_len` bytes long).
pub fn packed_runner(tail: &[u8], front_len: u64) -> Option<PackedRunner> {
    if tail.len() != RUNNER_TRAILER || &tail[32..] != MAGIC_RUNNER {
        return None;
    }
    let packed_len = u64_at(tail, 0)?;
    let at = front_len.checked_sub(RUNNER_TRAILER as u64)?.checked_sub(packed_len)?;
    Some(PackedRunner {
        at,
        packed_len,
        runner_len: u64_at(tail, 8)?,
        id: u64_at(tail, 16)?,
        filter: Filter::from_u64(u64_at(tail, 24)?)?,
    })
}

/// The trailer written after a packed runner's brotli stream.
pub fn runner_trailer(packed_len: u64, runner_len: u64, id: u64, filter: Filter) -> [u8; RUNNER_TRAILER] {
    let mut t = [0u8; RUNNER_TRAILER];
    t[0..8].copy_from_slice(&packed_len.to_le_bytes());
    t[8..16].copy_from_slice(&runner_len.to_le_bytes());
    t[16..24].copy_from_slice(&id.to_le_bytes());
    t[24..32].copy_from_slice(&(filter as u64).to_le_bytes());
    t[32..].copy_from_slice(MAGIC_RUNNER);
    t
}

/// FNV-1a over the runner's bytes: the cache name of one runner build.
pub fn runner_id(runner: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in runner {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Windows subsystem values of the PE optional header.
pub const SUBSYSTEM_GUI: u16 = 2;
pub const SUBSYSTEM_CONSOLE: u16 = 3;

/// Offset of the subsystem field of a PE file, `None` for other formats.
fn pe_subsystem_at(exe: &[u8]) -> Option<usize> {
    if exe.len() < 0x40 || &exe[..2] != b"MZ" {
        return None;
    }
    let pe = u32::from_le_bytes(exe[0x3C..0x40].try_into().ok()?) as usize;
    // signature (4), file header (20), then the subsystem 68 bytes into the optional header
    let at = pe.checked_add(4 + 20 + 68)?;
    (exe.get(pe..pe + 4)? == b"PE\0\0" && at + 2 <= exe.len()).then_some(at)
}

/// The subsystem of a Windows executable (`None` for other formats).
pub fn pe_subsystem(exe: &[u8]) -> Option<u16> {
    let at = pe_subsystem_at(exe)?;
    Some(u16::from_le_bytes([exe[at], exe[at + 1]]))
}

/// Sets the subsystem of a Windows executable; other formats are left alone.
pub fn set_pe_subsystem(exe: &mut [u8], subsystem: u16) {
    if let Some(at) = pe_subsystem_at(exe) {
        exe[at..at + 2].copy_from_slice(&subsystem.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(front: &[u8], body: &[&[u8]], fields: &[u64], magic: &[u8; 8]) -> Vec<u8> {
        let mut v = front.to_vec();
        for b in body {
            v.extend_from_slice(b);
        }
        for f in fields {
            v.extend_from_slice(&f.to_le_bytes());
        }
        v.extend_from_slice(magic);
        v
    }

    #[test]
    fn finds_every_program_form() {
        let ir = with(b"RUNNER", &[b"QIR 1\n"], &[6], MAGIC_IR);
        assert_eq!(program(&ir), Some((Program::Ir(b"QIR 1\n"), 6)));
        let z = with(b"RUN", &[b"zzzz"], &[4, 100], MAGIC_IR_ZLIB);
        assert_eq!(program(&z), Some((Program::IrZlib { stream: b"zzzz", len: 100 }, 3)));
        let n = with(b"R", &[b"QIR", b"IMAGE"], &[3, 5], MAGIC_NATIVE);
        assert_eq!(program(&n), Some((Program::Native { qir: b"QIR", image: b"IMAGE" }, 1)));
    }

    #[test]
    fn rejects_lengths_outside_the_file() {
        assert_eq!(program(&with(b"", &[b"ab"], &[3], MAGIC_IR)), None);
        assert_eq!(program(&with(b"", &[b"ab"], &[u64::MAX, 1], MAGIC_IR_ZLIB)), None);
        assert_eq!(program(&with(b"", &[b"ab"], &[1, u64::MAX], MAGIC_NATIVE)), None);
        assert_eq!(program(b"QBNEXIR0"), None);
        assert_eq!(program(b"MZ plain runner"), None);
    }

    #[test]
    fn span_from_the_tail_alone_matches() {
        let z = with(&[7u8; 1000], &[&[1u8; 50]], &[50, 400], MAGIC_IR_ZLIB);
        let tail = &z[z.len() - PROGRAM_TRAILER_MAX..];
        assert_eq!(program_span(tail, z.len() as u64).map(Span::start), Some(1000));
    }

    #[test]
    fn runner_trailer_round_trip() {
        let mut front = vec![9u8; 300];
        front.extend_from_slice(&[5u8; 77]);
        front.extend_from_slice(&runner_trailer(77, 4000, 0x1234, Filter::X86));
        let n = front.len() as u64;
        let r = packed_runner(&front[front.len() - RUNNER_TRAILER..], n).unwrap();
        assert_eq!(r, PackedRunner { at: 300, packed_len: 77, runner_len: 4000, id: 0x1234, filter: Filter::X86 });
        assert_eq!(packed_runner(&front[..RUNNER_TRAILER], n), None);
    }

    #[test]
    fn filters_round_trip() {
        let mut seed = 0x9E37_79B9u32;
        let data: Vec<u8> = (0..100_003)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                // plenty of E8/E9 and BL patterns among random bytes
                match seed % 7 {
                    0 => 0xE8,
                    1 => 0x94,
                    _ => seed as u8,
                }
            })
            .collect();
        for f in [Filter::X86, Filter::Arm64] {
            let mut b = data.clone();
            f.apply(&mut b, true);
            assert_ne!(b, data, "{f:?} changed nothing");
            f.apply(&mut b, false);
            assert_eq!(b, data, "{f:?} is not reversible");
        }
    }

    #[test]
    fn machine_of_executable_headers() {
        let mut pe = vec![0u8; 0x100];
        pe[..2].copy_from_slice(b"MZ");
        pe[0x3C] = 0x80;
        pe[0x80..0x84].copy_from_slice(b"PE\0\0");
        pe[0x84..0x86].copy_from_slice(&0x8664u16.to_le_bytes());
        assert_eq!(Filter::for_executable(&pe), Filter::X86);
        let mut elf = vec![0u8; 64];
        elf[..4].copy_from_slice(b"\x7fELF");
        elf[18..20].copy_from_slice(&183u16.to_le_bytes());
        assert_eq!(Filter::for_executable(&elf), Filter::Arm64);
        let mut macho = vec![0u8; 32];
        macho[..4].copy_from_slice(&0xFEED_FACFu32.to_le_bytes());
        macho[4..8].copy_from_slice(&0x0100_000Cu32.to_le_bytes());
        assert_eq!(Filter::for_executable(&macho), Filter::Arm64);
        assert_eq!(Filter::for_executable(b"text"), Filter::None);
    }

    #[test]
    fn pe_subsystem_is_read_and_set() {
        let mut pe = vec![0u8; 0x200];
        pe[..2].copy_from_slice(b"MZ");
        pe[0x3C] = 0x80;
        pe[0x80..0x84].copy_from_slice(b"PE\0\0");
        set_pe_subsystem(&mut pe, SUBSYSTEM_CONSOLE);
        assert_eq!(pe_subsystem(&pe), Some(SUBSYSTEM_CONSOLE));
        set_pe_subsystem(&mut pe, SUBSYSTEM_GUI);
        assert_eq!(pe_subsystem(&pe), Some(SUBSYSTEM_GUI));
        let mut elf = b"\x7fELF and more bytes".to_vec();
        set_pe_subsystem(&mut elf, SUBSYSTEM_GUI);
        assert_eq!(elf, b"\x7fELF and more bytes");
    }
}
