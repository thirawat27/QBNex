//! Addressable memory for QIR programs that need it (TYPE, `STRING * n`,
//! pointer arithmetic, GET/PUT of variables). Same layout as the C++ backend's storage:
//!
//! - every memory-backed variable, array descriptor and array data area is a
//!   block; a pointer to variable or element storage is its real address (so
//!   `_MEM`, `_OFFSET` arithmetic and qb-runtime routines work on it, like the
//!   C++ backend); an array descriptor reference is `ARR_TAG | block id`
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
    /// static array (`dims`): a second DIM keeps it, ERASE only clears it
    fixed: bool,
    /// ERASEd: no implicit `DIM name(10)` on its next use (subscript out of range)
    erased: bool,
}

#[derive(Default)]
pub(crate) struct Mem {
    blocks: Vec<Block>,
    free_blocks: Vec<u32>,
    strs: Vec<Vec<u8>>,
    free_strs: Vec<u64>,
}

/// Tag of an array-descriptor reference; every other pointer is a real address.
pub(crate) const ARR_TAG: u64 = 1 << 63;

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

    /// Frees a variable's block (id from `new_var`).
    pub(crate) fn free_var(&mut self, id: u32) {
        self.free(id);
    }

    fn block_addr(&self, id: u32) -> u64 {
        match &self.blocks[id as usize] {
            Block::Bytes { data, .. } => data.as_ptr() as u64,
            _ => 0,
        }
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

    /// New zero-initialized storage for a variable: (pointer, block id).
    pub(crate) fn new_var(&mut self, v: &MVar) -> (u64, u32) {
        if v.array {
            let id = self.alloc(Block::Arr(MArr { esize: v.size, fill: v.fill, strs: v.strs, dims: vec![], data: None, fixed: false, erased: false }));
            return (ARR_TAG | u64::from(id), id);
        }
        let id = self.alloc(Block::Bytes { data: vec![v.fill; v.size.max(1) as usize], strs: v.strs });
        (self.block_addr(id), id)
    }

    /// `len` bytes at the real address `p` (like the C++ backend, addresses are
    /// not checked beyond null and array references).
    pub(crate) fn bytes(&mut self, p: u64, len: usize) -> R<&mut [u8]> {
        if p == 0 || p & ARR_TAG != 0 {
            return Err(5);
        }
        // SAFETY: QIR pointers come from addr/aaddr/padd on live storage (or
        // _MEM blocks of qb-runtime); validated programs keep them in range.
        Ok(unsafe { std::slice::from_raw_parts_mut(p as *mut u8, len) })
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

    /// Address of the characters of the string in slot `p` (0 for "").
    pub(crate) fn str_data(&mut self, p: u64) -> R<u64> {
        let h = self.ld(MTy::U64, p)?;
        Ok(self.strs.get_mut(h as usize).filter(|_| h != 0).map_or(0, |v| v.as_mut_ptr() as u64))
    }

    pub(crate) fn st_str(&mut self, p: u64, s: &[u8]) -> R<()> {
        let old = self.ld(MTy::U64, p)?;
        // Reuse the slot's buffer: freeing and allocating a large string on
        // every store (`a$(i) = a$(i) + x$`) made repeated appends to a
        // memory string allocation-bound (R14).
        if old != 0 && !s.is_empty() {
            if let Some(v) = self.strs.get_mut(old as usize) {
                let r = v.as_ptr_range();
                let src = s.as_ptr();
                if !(r.start <= src && src < r.end) {
                    v.clear();
                    v.extend_from_slice(s);
                    return Ok(());
                }
            }
        }
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
        if desc & ARR_TAG == 0 {
            return Err(9);
        }
        match self.blocks.get_mut((desc & !ARR_TAG) as usize) {
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
            if kind == DimMode::Static && a.data.is_some() {
                return Ok(()); // a static array is dimensioned once
            }
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
        a.fixed = kind == DimMode::Static;
        a.erased = false;
        Ok(())
    }

    pub(crate) fn erase(&mut self, desc: u64) -> R<()> {
        let a = self.arr(desc)?;
        if a.fixed {
            // static array: cleared in place, dimensions kept (QBasic, C++ backend)
            let (fill, data) = (a.fill, a.data);
            if let Some(d) = data {
                let strs = match &mut self.blocks[d as usize] {
                    Block::Bytes { data, strs } => {
                        let old = std::mem::replace(data, vec![fill; data.len()]);
                        if *strs { old } else { Vec::new() }
                    }
                    _ => Vec::new(),
                };
                for c in strs.chunks_exact(8) {
                    self.free_str(u64::from_le_bytes(c.try_into().unwrap()));
                }
            }
            return Ok(());
        }
        a.dims.clear();
        a.erased = true;
        if let Some(d) = a.data.take() {
            self.free(d);
        }
        Ok(())
    }

    pub(crate) fn aaddr(&mut self, desc: u64, idx: &[i64]) -> R<u64> {
        let a = self.arr(desc)?;
        if a.data.is_none() && !a.erased && !idx.is_empty() {
            // never dimensioned: QBasic's implicit DIM name(10) (the register model's arr_at)
            self.dim(desc, DimMode::Dim, vec![(0, 10); idx.len()])?;
        }
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
        let esize = i64::from(a.esize);
        Ok(self.block_addr(d).wrapping_add((off * esize) as u64))
    }

    /// (address of the first element, total bytes) of an array.
    pub(crate) fn array_extent(&mut self, desc: u64) -> R<(u64, u64)> {
        let (data, n, esize) = {
            let a = self.arr(desc)?;
            let n: i64 = a.dims.iter().map(|(lo, hi)| hi - lo + 1).product();
            (a.data, n, a.esize)
        };
        Ok(match data {
            Some(d) => (self.block_addr(d), n as u64 * u64::from(esize)),
            None => (0, 0),
        })
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
