//! Addressable memory for QIR v1 programs that need it (TYPE, `STRING * n`,
//! pointer arithmetic, GET/PUT of variables). Same model as the reference
//! interpreter (`v1.rs`), so both give identical results:
//!
//! - every memory-backed variable, array descriptor and array data area is a
//!   block; a pointer is `(block << 32) | offset`
//! - a `str` slot in memory is an 8-byte handle into `strs` (0 = empty string)
//! - element layout: first subscript varies fastest (QB64)

use crate::bytecode::{DimMode, MTy, MVar};

pub(crate) enum Block {
    Bytes { data: Vec<u8>, strs: bool },
    Arr(MArr),
    Free,
}

pub(crate) struct MArr {
    esize: u32,
    fill: u8,
    strs: bool,
    dims: Vec<(i64, i64)>,
    data: Option<u32>,
}

#[derive(Default)]
pub(crate) struct Mem {
    blocks: Vec<Block>,
    free_blocks: Vec<u32>,
    strs: Vec<Vec<u8>>,
    free_strs: Vec<u64>,
}

pub(crate) fn ptr(b: u32, off: u32) -> u64 {
    (u64::from(b) << 32) | u64::from(off)
}

fn split(p: u64) -> (usize, usize) {
    ((p >> 32) as usize, (p & 0xFFFF_FFFF) as usize)
}

type R<T> = Result<T, i32>;

impl Mem {
    pub(crate) fn new() -> Mem {
        Mem { strs: vec![Vec::new()], ..Mem::default() }
    }

    fn alloc(&mut self, b: Block) -> u32 {
        if let Some(i) = self.free_blocks.pop() {
            self.blocks[i as usize] = b;
            i
        } else {
            self.blocks.push(b);
            (self.blocks.len() - 1) as u32
        }
    }

    /// Frees the block a pointer of a variable points at.
    pub(crate) fn free_var(&mut self, p: u64) {
        self.free(split(p).0 as u32);
    }

    fn free(&mut self, i: u32) {
        let Some(slot) = self.blocks.get_mut(i as usize) else { return };
        let b = std::mem::replace(slot, Block::Free);
        match b {
            Block::Bytes { data, strs: true } => {
                for c in data.chunks_exact(8) {
                    self.free_str(u64::from_le_bytes(c.try_into().unwrap()));
                }
            }
            Block::Arr(a) => {
                if let Some(d) = a.data {
                    self.free(d);
                }
            }
            Block::Free => return,
            _ => {}
        }
        self.free_blocks.push(i);
    }

    fn free_str(&mut self, h: u64) {
        if h != 0 && (h as usize) < self.strs.len() {
            self.strs[h as usize] = Vec::new();
            self.free_strs.push(h);
        }
    }

    fn new_str(&mut self, s: &[u8]) -> u64 {
        if s.is_empty() {
            return 0;
        }
        if let Some(h) = self.free_strs.pop() {
            self.strs[h as usize] = s.to_vec();
            h
        } else {
            self.strs.push(s.to_vec());
            (self.strs.len() - 1) as u64
        }
    }

    /// New zero-initialized storage for a variable; returns its pointer.
    pub(crate) fn new_var(&mut self, v: &MVar) -> u64 {
        let b = if v.array {
            Block::Arr(MArr { esize: v.size, fill: v.fill, strs: v.strs, dims: vec![], data: None })
        } else {
            Block::Bytes { data: vec![v.fill; v.size as usize], strs: v.strs }
        };
        ptr(self.alloc(b), 0)
    }

    pub(crate) fn bytes(&mut self, p: u64, len: usize) -> R<&mut [u8]> {
        let (b, o) = split(p);
        match self.blocks.get_mut(b) {
            Some(Block::Bytes { data, .. }) if o + len <= data.len() => Ok(&mut data[o..o + len]),
            _ => Err(7),
        }
    }

    /// Numeric load: register bits (integers sign/zero-extended, floats as f64).
    pub(crate) fn ld(&mut self, t: MTy, p: u64) -> R<u64> {
        let n = t.size();
        let b = self.bytes(p, n)?;
        let mut w = [0u8; 8];
        w[..n].copy_from_slice(b);
        let u = u64::from_le_bytes(w);
        Ok(match t {
            MTy::I8 => u as i8 as i64 as u64,
            MTy::U8 => u as u8 as u64,
            MTy::I16 => u as i16 as i64 as u64,
            MTy::U16 => u as u16 as u64,
            MTy::I32 => u as i32 as i64 as u64,
            MTy::U32 => u as u32 as u64,
            MTy::I64 | MTy::U64 | MTy::Ptr | MTy::Str => u,
            MTy::F32 => f64::from(f32::from_bits(u as u32)).to_bits(),
            MTy::F64 => u,
            MTy::F80 => u, // the VM reads _FLOAT through bytes()
        })
    }

    pub(crate) fn st(&mut self, t: MTy, p: u64, bits: u64) -> R<()> {
        let u = match t {
            MTy::F32 => u64::from((f64::from_bits(bits) as f32).to_bits()),
            _ => bits,
        };
        let n = t.size();
        self.bytes(p, n)?.copy_from_slice(&u.to_le_bytes()[..n]);
        Ok(())
    }

    pub(crate) fn ld_str(&mut self, p: u64) -> R<&[u8]> {
        let h = self.ld(MTy::U64, p)?;
        Ok(self.strs.get(h as usize).map_or(&[][..], |v| v.as_slice()))
    }

    pub(crate) fn st_str(&mut self, p: u64, s: &[u8]) -> R<()> {
        let old = self.ld(MTy::U64, p)?;
        self.free_str(old);
        let h = self.new_str(s);
        self.st(MTy::U64, p, h)
    }

    pub(crate) fn ldfs(&mut self, p: u64, len: u32) -> R<Vec<u8>> {
        Ok(self.bytes(p, len as usize)?.to_vec())
    }

    pub(crate) fn stfs(&mut self, p: u64, len: u32, s: &[u8]) -> R<()> {
        let b = self.bytes(p, len as usize)?;
        for (i, x) in b.iter_mut().enumerate() {
            *x = s.get(i).copied().unwrap_or(b' ');
        }
        Ok(())
    }

    pub(crate) fn copy(&mut self, dst: u64, src: u64, n: u32) -> R<()> {
        let v = self.bytes(src, n as usize)?.to_vec();
        self.bytes(dst, n as usize)?.copy_from_slice(&v);
        Ok(())
    }

    fn arr(&mut self, desc: u64) -> R<&mut MArr> {
        match self.blocks.get_mut(split(desc).0) {
            Some(Block::Arr(a)) => Ok(a),
            _ => Err(9),
        }
    }

    pub(crate) fn dim(&mut self, desc: u64, kind: DimMode, bounds: Vec<(i64, i64)>) -> R<()> {
        let mut n: u64 = 1;
        for (lo, hi) in &bounds {
            if hi < lo {
                return Err(9);
            }
            n = n.saturating_mul((hi - lo + 1) as u64);
        }
        let (esize, fill, strs, old, olddims) = {
            let a = self.arr(desc)?;
            if kind == DimMode::Dim && a.data.is_some() {
                return Err(10);
            }
            (a.esize, a.fill, a.strs, a.data.take(), std::mem::take(&mut a.dims))
        };
        let total = n.saturating_mul(u64::from(esize));
        if total > (1 << 31) {
            if let Ok(a) = self.arr(desc) {
                a.data = old;
                a.dims = olddims;
            }
            return Err(7);
        }
        let mut data = vec![fill; total as usize];
        if kind == DimMode::RedimPreserve {
            if let Some(ob) = old {
                let od = match std::mem::replace(&mut self.blocks[ob as usize], Block::Bytes { data: vec![], strs: false }) {
                    Block::Bytes { data, .. } => data,
                    _ => vec![],
                };
                let k = od.len().min(data.len());
                data[..k].copy_from_slice(&od[..k]);
                if strs {
                    for c in od[k..].chunks_exact(8) {
                        self.free_str(u64::from_le_bytes(c.try_into().unwrap()));
                    }
                }
                self.free(ob);
            }
        } else if let Some(ob) = old {
            self.free(ob);
        }
        let d = self.alloc(Block::Bytes { data, strs });
        let a = self.arr(desc)?;
        a.data = Some(d);
        a.dims = bounds;
        Ok(())
    }

    pub(crate) fn erase(&mut self, desc: u64) -> R<()> {
        let a = self.arr(desc)?;
        a.dims.clear();
        if let Some(d) = a.data.take() {
            self.free(d);
        }
        Ok(())
    }

    pub(crate) fn aaddr(&mut self, desc: u64, idx: &[i64]) -> R<u64> {
        let a = self.arr(desc)?;
        let Some(d) = a.data else { return Err(9) };
        if idx.len() != a.dims.len() {
            return Err(9);
        }
        let mut off: i64 = 0;
        let mut mul: i64 = 1;
        for (i, (lo, hi)) in idx.iter().zip(&a.dims) {
            if i < lo || i > hi {
                return Err(9);
            }
            off += (i - lo) * mul;
            mul *= hi - lo + 1;
        }
        Ok(ptr(d, (off * i64::from(a.esize)) as u32))
    }

    pub(crate) fn bound(&mut self, desc: u64, k: i64, upper: bool) -> R<i64> {
        let a = self.arr(desc)?;
        if a.data.is_none() || k < 1 || k as usize > a.dims.len() {
            return Err(9);
        }
        let (lo, hi) = a.dims[k as usize - 1];
        Ok(if upper { hi } else { lo })
    }
}
