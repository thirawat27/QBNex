//! Addressable memory for QIR programs that need it (TYPE, `STRING * n`,
//! pointer arithmetic, GET/PUT of variables). Storage layout:
//!
//! - every memory-backed variable, array descriptor and array data area is a
//!   block; a pointer to variable or element storage is its real address (so
//!   `_MEM`, `_OFFSET` arithmetic and qb-runtime routines work on it, as in
//!   QB64); an array descriptor reference is `ARR_TAG | block id`
//! - a `str` slot in memory is an 8-byte handle into `strs` (0 = empty string)
//! - element layout: first subscript varies fastest (QB64)

use crate::bytecode::{DimMode, MTy, MVar};

pub(crate) enum Block {
    Bytes { data: Vec<u8>, strs: bool },
    /// the variables of `Module::vgroups[group]` (one call's `NewVA`): string handles
    /// in the byte ranges `Mem::group_strs[group]`
    Frame { data: Vec<u8>, group: u32 },
    Arr(MArr),
    Free,
}

pub(crate) struct MArr {
    esize: u32,
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
    /// Set by the VM while a SUB/FUNCTION is active: the bytes of an array that is
    /// redimensioned or erased go to `retired` and are not freed at once. A BYREF
    /// parameter is the address of an element, and the callee (or a SUB it calls)
    /// may REDIM that SHARED array: the parameter then points into these bytes.
    pub(crate) keep_freed: bool,
    /// `OPTION BASE`: lower bound of an array used without DIM
    pub(crate) option_base: i64,
    retired: Vec<Vec<u8>>,
    /// Storage of freed variables, by size (`VAR_POOL_MAX` bytes at most): `new_var`
    /// takes it zeroed instead of a new allocation. Every SUB/FUNCTION call creates
    /// the memory-backed variables of its frame (BYREF arguments, TYPE locals) and
    /// frees them at the return, so this is the hottest allocation of most programs.
    var_pool: Vec<Vec<Vec<u8>>>,
    /// per variable group (`Module::vgroups`): the byte ranges that hold string handles
    pub(crate) group_strs: Vec<Vec<(u32, u32)>>,
}

/// Largest variable whose storage `Mem::var_pool` keeps, and how many per size.
const VAR_POOL_MAX: usize = 1 << 16;
const VAR_POOL_PER_SIZE: usize = 64;

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

    /// (first address, end address) of a variable's storage, None for an array.
    pub(crate) fn var_range(&self, id: u32) -> Option<(u64, u64)> {
        match self.blocks.get(id as usize) {
            Some(Block::Bytes { data, .. } | Block::Frame { data, .. }) => Some((data.as_ptr() as u64, data.as_ptr() as u64 + data.len() as u64)),
            _ => None,
        }
    }

    /// Frees a variable's block (id from `new_var`).
    pub(crate) fn free_var(&mut self, id: u32) {
        self.free(id);
    }

    fn block_addr(&self, id: u32) -> u64 {
        match &self.blocks[id as usize] {
            Block::Bytes { data, .. } | Block::Frame { data, .. } => data.as_ptr() as u64,
            _ => 0,
        }
    }

    fn free(&mut self, i: u32) {
        let Some(slot) = self.blocks.get_mut(i as usize) else { return };
        let b = std::mem::replace(slot, Block::Free);
        match b {
            Block::Bytes { data, strs } => {
                if strs {
                    for c in data.as_chunks::<8>().0 {
                        self.free_str(u64::from_le_bytes(*c));
                    }
                }
                self.pool_var(data);
            }
            Block::Frame { data, group } => {
                let n = self.group_strs.get(group as usize).map_or(0, Vec::len);
                for k in 0..n {
                    let (a, b) = self.group_strs[group as usize][k];
                    for c in data[a as usize..b as usize].as_chunks::<8>().0 {
                        self.free_str(u64::from_le_bytes(*c));
                    }
                }
                self.pool_var(data);
            }
            Block::Arr(a) => {
                if let Some(d) = a.data {
                    self.free(d);
                }
            }
            Block::Free => return,
        }
        self.free_blocks.push(i);
    }

    /// Frees the data block of an array; see [`Mem::keep_freed`].
    fn free_data(&mut self, i: u32) {
        if self.keep_freed
            && let Some(Block::Bytes { data, strs }) = self.blocks.get_mut(i as usize)
        {
            let (bytes, strs) = (std::mem::take(data), *strs);
            if strs {
                for c in bytes.as_chunks::<8>().0 {
                    self.free_str(u64::from_le_bytes(*c));
                }
            }
            self.blocks[i as usize] = Block::Free;
            self.free_blocks.push(i);
            self.retire(bytes);
            return;
        }
        self.free(i);
    }

    fn retire(&mut self, bytes: Vec<u8>) {
        if self.keep_freed && !bytes.is_empty() {
            self.retired.push(bytes);
        }
    }

    pub(crate) fn has_retired(&self) -> bool {
        !self.retired.is_empty()
    }

    /// Frees the retired array bytes for which `live(first address, end address)`
    /// is false.
    pub(crate) fn prune_retired(&mut self, live: impl Fn(u64, u64) -> bool) {
        self.retired.retain(|b| {
            let lo = b.as_ptr() as u64;
            live(lo, lo + b.len() as u64)
        });
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

    /// Keeps the storage of a freed block for `new_var` (see `var_pool`).
    fn pool_var(&mut self, data: Vec<u8>) {
        let n = data.len();
        if n == 0 || n > VAR_POOL_MAX || data.capacity() != n {
            return;
        }
        if self.var_pool.len() <= n {
            self.var_pool.resize_with(n + 1, Vec::new);
        }
        let p = &mut self.var_pool[n];
        if p.len() < VAR_POOL_PER_SIZE {
            p.push(data);
        }
    }

    /// New zero-initialized storage for the variables of group `g` (`size` bytes):
    /// (pointer, block id).
    pub(crate) fn new_frame(&mut self, g: u32, size: u32) -> (u64, u32) {
        let n = size.max(1) as usize;
        let data = match self.var_pool.get_mut(n).and_then(Vec::pop) {
            Some(mut d) => {
                d.fill(0);
                d
            }
            None => vec![0; n],
        };
        let id = self.alloc(Block::Frame { data, group: g });
        (self.block_addr(id), id)
    }

    /// New zero-initialized storage for a variable: (pointer, block id).
    pub(crate) fn new_var(&mut self, v: &MVar) -> (u64, u32) {
        if v.array {
            let id = self.alloc(Block::Arr(MArr { esize: v.size, strs: v.strs, dims: vec![], data: None, fixed: false, erased: false }));
            return (ARR_TAG | u64::from(id), id);
        }
        let n = v.size.max(1) as usize;
        let data = match self.var_pool.get_mut(n).and_then(Vec::pop) {
            Some(mut d) => {
                d.fill(0);
                d
            }
            None => vec![0; n],
        };
        let id = self.alloc(Block::Bytes { data, strs: v.strs });
        (self.block_addr(id), id)
    }

    /// `len` bytes at the real address `p` (as in QB64, addresses are
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
    /// A `_FLOAT` comes as the f64 bits of its value (the VM's own `_FLOAT`
    /// instructions read all 10 bytes through `bytes()`; this is for inspection).
    pub(crate) fn ld(&mut self, t: MTy, p: u64) -> R<u64> {
        if t == MTy::F80 {
            let b = self.bytes(p, 10)?;
            let mant = u64::from_le_bytes(b[..8].try_into().expect("8 bytes"));
            let se = u16::from_le_bytes([b[8], b[9]]);
            return Ok(qb_float80::F80 { mant, se }.to_f64().to_bits());
        }
        if p == 0 || p & ARR_TAG != 0 {
            return Err(5);
        }
        // SAFETY: as `bytes`: `p` is `t.size()` bytes of live storage (unaligned reads,
        // little-endian like the former byte copy on the supported targets)
        Ok(unsafe {
            match t {
                MTy::I8 => (p as *const i8).read() as i64 as u64,
                MTy::U8 => u64::from((p as *const u8).read()),
                MTy::I16 => i16::from_le((p as *const i16).read_unaligned()) as i64 as u64,
                MTy::U16 => u64::from(u16::from_le((p as *const u16).read_unaligned())),
                MTy::I32 => i32::from_le((p as *const i32).read_unaligned()) as i64 as u64,
                MTy::U32 => u64::from(u32::from_le((p as *const u32).read_unaligned())),
                MTy::I64 | MTy::U64 | MTy::Ptr | MTy::Str | MTy::F64 => u64::from_le((p as *const u64).read_unaligned()),
                MTy::F32 => f64::from(f32::from_bits(u32::from_le((p as *const u32).read_unaligned()))).to_bits(),
                MTy::F80 => unreachable!("read above"),
            }
        })
    }

    pub(crate) fn st(&mut self, t: MTy, p: u64, bits: u64) -> R<()> {
        if t == MTy::F80 {
            // (as `ld`: the value as f64 bits)
            let x = qb_float80::F80::from_f64(f64::from_bits(bits));
            let b = self.bytes(p, 10)?;
            b[..8].copy_from_slice(&x.mant.to_le_bytes());
            b[8..].copy_from_slice(&x.se.to_le_bytes());
            return Ok(());
        }
        if p == 0 || p & ARR_TAG != 0 {
            return Err(5);
        }
        // SAFETY: as `ld`; the low `t.size()` bytes of the value, little-endian
        unsafe {
            match t.size() {
                1 => (p as *mut u8).write(bits as u8),
                2 => (p as *mut u16).write_unaligned((bits as u16).to_le()),
                4 => {
                    let u = if t == MTy::F32 { (f64::from_bits(bits) as f32).to_bits() } else { bits as u32 };
                    (p as *mut u32).write_unaligned(u.to_le())
                }
                _ => (p as *mut u64).write_unaligned(bits.to_le()),
            }
        }
        Ok(())
    }

    /// The string variable stored at `p` without copying it (a string operand with
    /// [`crate::bytecode::M`]); "" when `p` is not valid storage.
    pub(crate) fn peek_str(&self, p: u64) -> &[u8] {
        if p == 0 || p & ARR_TAG != 0 {
            return &[];
        }
        // SAFETY: as `bytes`: p is the storage of a string variable (8-byte handle).
        let h = unsafe { (p as *const u64).read_unaligned() };
        self.strs.get(h as usize).map_or(&[][..], |v| v.as_slice())
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
        // memory string allocation-bound.
        if old != 0 && !s.is_empty()
            && let Some(v) = self.strs.get_mut(old as usize) {
                let r = v.as_ptr_range();
                let src = s.as_ptr();
                if !(r.start <= src && src < r.end) {
                    v.clear();
                    v.extend_from_slice(s);
                    return Ok(());
                }
            }
        // `s` may be (part of) the string being replaced: the new one is made first
        let h = self.new_str(s);
        self.free_str(old);
        self.st(MTy::U64, p, h)
    }

    #[cfg(feature = "debug")]
    pub(crate) fn ldfs(&mut self, p: u64, len: u32) -> R<Vec<u8>> {
        Ok(self.bytes(p, len as usize)?.to_vec())
    }

    /// `ldfs` into `dst`, reusing its buffer (no allocation per load of a `STRING * n`).
    pub(crate) fn ldfs_into(&mut self, p: u64, len: u32, dst: &mut Vec<u8>) -> R<()> {
        let b = self.bytes(p, len as usize)?;
        dst.clear();
        dst.extend_from_slice(b);
        Ok(())
    }

    pub(crate) fn stfs(&mut self, p: u64, len: u32, s: &[u8]) -> R<()> {
        let b = self.bytes(p, len as usize)?;
        // the value, cut or padded with spaces to the length
        let k = s.len().min(b.len());
        b[..k].copy_from_slice(&s[..k]);
        b[k..].fill(b' ');
        Ok(())
    }

    pub(crate) fn copy(&mut self, dst: u64, src: u64, n: u32) -> R<()> {
        // both ranges are checked as before (null and array references fail)
        let s = self.bytes(src, n as usize)?.as_ptr();
        let d = self.bytes(dst, n as usize)?.as_mut_ptr();
        // SAFETY: both are `n` valid bytes (`bytes`); `copy` allows them to overlap,
        // as the former copy through a temporary did
        unsafe { std::ptr::copy(s, d, n as usize) };
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
            // checked: (i64::MIN TO i64::MAX) wraps to 0 elements with bounds that accept every subscript
            let k = hi.checked_sub(*lo).and_then(|d| d.checked_add(1)).ok_or(7)?;
            n = n.saturating_mul(k as u64);
        }
        let (esize, strs, old, olddims) = {
            let a = self.arr(desc)?;
            if kind == DimMode::Static && a.data.is_some() {
                return Ok(()); // a static array is dimensioned once
            }
            if kind == DimMode::Dim && a.data.is_some() {
                return Err(10);
            }
            (a.esize, a.strs, a.data.take(), std::mem::take(&mut a.dims))
        };
        let total = n.saturating_mul(u64::from(esize));
        if total > (1 << 31) {
            if let Ok(a) = self.arr(desc) {
                a.data = old;
                a.dims = olddims;
            }
            return Err(7);
        }
        // at least one byte: element addresses of an array of a 0-byte TYPE point
        // into a real block (an empty Vec has none, only a dangling address)
        let mut data = vec![0; (total as usize).max(1)];
        if kind == DimMode::RedimPreserve {
            if let Some(ob) = old {
                let od = match std::mem::replace(&mut self.blocks[ob as usize], Block::Bytes { data: vec![], strs: false }) {
                    Block::Bytes { data, .. } => data,
                    _ => vec![],
                };
                let k = od.len().min(data.len());
                data[..k].copy_from_slice(&od[..k]);
                if strs {
                    for c in od[k..].as_chunks::<8>().0 {
                        self.free_str(u64::from_le_bytes(*c));
                    }
                }
                self.free(ob);
                self.retire(od);
            }
        } else if let Some(ob) = old {
            self.free_data(ob);
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
            // static array: cleared in place, dimensions kept (as in QBasic)
            let data = a.data;
            if let Some(d) = data {
                // Cleared where it is: an element passed by reference, or its address
                // held across the statement, still points into these bytes.
                let strs = match &mut self.blocks[d as usize] {
                    Block::Bytes { data, strs } => {
                        let old = if *strs { data.clone() } else { Vec::new() };
                        data.fill(0);
                        old
                    }
                    _ => Vec::new(),
                };
                for c in strs.as_chunks::<8>().0 {
                    self.free_str(u64::from_le_bytes(*c));
                }
            }
            return Ok(());
        }
        a.dims.clear();
        a.erased = true;
        if let Some(d) = a.data.take() {
            self.free_data(d);
        }
        Ok(())
    }

    pub(crate) fn aaddr(&mut self, desc: u64, idx: &[i64]) -> R<u64> {
        let a = self.arr(desc)?;
        if a.data.is_none() && !a.erased && !idx.is_empty() {
            // never dimensioned: QBasic's implicit DIM name(10) (the register model's arr_at)
            self.dim(desc, DimMode::Dim, vec![(self.option_base, 10); idx.len()])?;
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

    /// Bounds of an array (empty: not dimensioned) and whether it is static.
    pub(crate) fn dims(&mut self, desc: u64) -> R<(Vec<(i64, i64)>, bool)> {
        let a = self.arr(desc)?;
        Ok((if a.data.is_some() { a.dims.clone() } else { Vec::new() }, a.fixed))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float80_load_and_store_do_not_panic() {
        let mut m = Mem::new();
        let (p, _) = m.new_var(&MVar { size: 10, strs: false, array: false });
        m.st(MTy::F80, p, 2.5f64.to_bits()).unwrap();
        assert_eq!(f64::from_bits(m.ld(MTy::F80, p).unwrap()), 2.5);
        let b = m.bytes(p, 10).unwrap();
        let x = qb_float80::F80 { mant: u64::from_le_bytes(b[..8].try_into().unwrap()), se: u16::from_le_bytes([b[8], b[9]]) };
        assert_eq!(x.to_f64(), 2.5);
    }

    #[test]
    fn element_of_a_zero_byte_type_points_into_a_block() {
        let mut m = Mem::new();
        let (desc, _) = m.new_var(&MVar { size: 0, strs: false, array: true });
        m.dim(desc, DimMode::Dim, vec![(0, 9)]).unwrap();
        let p = m.aaddr(desc, &[3]).unwrap();
        let (lo, _) = m.array_extent(desc).unwrap();
        assert_eq!(p, lo);
        // the data block has a real byte there (an empty Vec has only a dangling address)
        let d = m.arr(desc).unwrap().data.unwrap();
        assert!(matches!(&m.blocks[d as usize], Block::Bytes { data, .. } if !data.is_empty() && data.as_ptr() as u64 == p));
    }
}
