//! Static and conventional-memory allocators used by generated code (R12b).
//!
//! * `mem_static`: a bump allocator for `STATIC`/`SHARED` data and array
//!   descriptors. Generated code allocates inline
//!   (`if ((mem_static_pointer+=n)<mem_static_limit) ...`) and only calls
//!   [`qb_mem_static_malloc`] when the block is full. A full block is never
//!   reallocated (pointers into it must stay valid): a new block at least
//!   twice as large replaces it and the old one is kept forever.
//! * `cmem_dynamic`: the "links" allocator for `$DYNAMIC` arrays in the
//!   emulated 640 KB conventional memory (`cmem`), growing down from &HA000
//!   towards `cmem_static_pointer`. Blocks are multiples of 16 bytes; a free
//!   gap between existing blocks is reused first-fit, scanning from the top.
//!
//! Replaces `allocate_mem_static_block`, `mem_static_malloc`,
//! `mem_static_restore`, `cmem_dynamic_malloc` and `cmem_dynamic_free` from
//! libqb.cpp.

#![allow(non_upper_case_globals)]

use core::ptr::{addr_of, addr_of_mut, null_mut};

/// Start of the current `mem_static` block.
#[unsafe(no_mangle)]
pub static mut mem_static: *mut u8 = null_mut();
/// Next free byte of the current block.
#[unsafe(no_mangle)]
pub static mut mem_static_pointer: *mut u8 = null_mut();
/// End of the current block.
#[unsafe(no_mangle)]
pub static mut mem_static_limit: *mut u8 = null_mut();
/// Size of the current block.
#[unsafe(no_mangle)]
pub static mut mem_static_size: u32 = 0;

/// Every block ever allocated (old blocks stay alive, see module docs).
static mut MEM_STATIC_BLOCKS: Vec<Box<[u8]>> = Vec::new();

/// `allocate_mem_static_block`: makes a new current block of `size` bytes.
/// Returns 504 (critical "Out of memory") when the allocation fails.
pub fn mem_static_new_block(size: u32) -> i32 {
    let mut v: Vec<u8> = Vec::new();
    if v.try_reserve_exact(size as usize).is_err() {
        return 504;
    }
    v.resize(size as usize, 0);
    let mut b = v.into_boxed_slice();
    // SAFETY: the allocator globals are only used by the program thread.
    unsafe {
        mem_static = b.as_mut_ptr();
        (*addr_of_mut!(MEM_STATIC_BLOCKS)).push(b);
        mem_static_size = size;
        mem_static_pointer = mem_static;
        mem_static_limit = mem_static.add(size as usize);
    }
    0
}

/// `mem_static_malloc`: `size` rounded up to 8 bytes. `Err(504)` when a new
/// block cannot be allocated.
pub fn mem_static_malloc(size: u32) -> Result<*mut u8, i32> {
    let size = size.wrapping_add(7);
    let size = size - (size & 7);
    // SAFETY: as above; pointer arithmetic mirrors the legacy C (the result
    // is only dereferenced when it lies inside the block).
    unsafe {
        let p = mem_static_pointer.wrapping_add(size as usize);
        mem_static_pointer = p;
        if (p as usize) < (mem_static_limit as usize) {
            return Ok(p.wrapping_sub(size as usize));
        }
        let new_size = (mem_static_size << 1).wrapping_add(size);
        let e = mem_static_new_block(new_size);
        if e != 0 {
            return Err(e);
        }
        mem_static_pointer = mem_static.add(size as usize);
        Ok(mem_static_pointer.sub(size as usize))
    }
}

/// `mem_static_restore`: frees everything allocated after `restore_point`;
/// a point outside the current block rewinds to the block start.
pub fn mem_static_restore(restore_point: *mut u8) {
    // SAFETY: as above.
    unsafe {
        let r = restore_point as usize;
        if r >= mem_static as usize && r <= mem_static_limit as usize {
            mem_static_pointer = restore_point;
        } else {
            mem_static_pointer = mem_static;
        }
    }
}

// ------------------------------------------------------------ cmem_dynamic

/// Offset of &HA000:0000 (top of the conventional-memory heap) in `cmem`.
pub const CMEM_TOP: u32 = 655_360;
const MAX_LINKS: usize = 147_136;

#[derive(Clone, Copy, Default)]
struct Link {
    offset: u32,
    top: u32,
    /// Index of the next (lower) block, `NONE` at the end.
    next: u32,
}

const NONE: u32 = u32::MAX;

/// The `cmem_dynamic` block list. Addresses are offsets into `cmem`.
pub struct CmemDynamic {
    links: Vec<Link>,
    first: u32,
    next_link: u32,
    free_list: Vec<u32>,
    /// `cmem_dynamic_base` after the last allocation below every block.
    pub base: u32,
}

impl Default for CmemDynamic {
    fn default() -> Self {
        Self::new()
    }
}

impl CmemDynamic {
    pub const fn new() -> Self {
        CmemDynamic { links: Vec::new(), first: NONE, next_link: 0, free_list: Vec::new(), base: CMEM_TOP }
    }

    /// `cmem_dynamic_malloc`: returns the block offset, or the critical error
    /// (505 block over 64 KB, 506 no room above `static_top`, 507 no link).
    pub fn malloc(&mut self, size: u32, static_top: u32) -> Result<u32, i32> {
        if size > 65_536 {
            return Err(505);
        }
        if size == 0 {
            return Ok(CMEM_TOP);
        }
        let size = if size & 15 != 0 { size - (size & 15) + 16 } else { size };
        let mut top = CMEM_TOP;
        let mut prev = NONE;
        let mut link = self.first;
        let mut found = false;
        while link != NONE {
            let l = self.links[link as usize];
            if i64::from(top) - i64::from(l.top) >= i64::from(size) {
                found = true;
                break;
            }
            prev = link;
            top = l.offset;
            link = l.next;
        }
        if !found {
            if (i64::from(top) - i64::from(static_top)) < i64::from(size) {
                return Err(506);
            }
            self.base = top - size;
        }
        let i = if let Some(i) = self.free_list.pop() {
            i
        } else {
            let i = self.next_link;
            self.next_link += 1;
            if i as usize >= MAX_LINKS {
                return Err(507);
            }
            i
        };
        if self.links.len() <= i as usize {
            self.links.resize(i as usize + 1, Link::default());
        }
        let new_next = if prev != NONE { self.links[prev as usize].next } else { self.first };
        self.links[i as usize] = Link { offset: top - size, top, next: new_next };
        if prev != NONE {
            self.links[prev as usize].next = i;
        } else {
            self.first = i;
        }
        Ok(top - size)
    }

    /// `cmem_dynamic_free`: unknown offsets are ignored.
    pub fn free(&mut self, offset: u32) {
        if self.first == NONE || offset == CMEM_TOP {
            return;
        }
        let mut prev = NONE;
        let mut link = self.first;
        while link != NONE {
            let l = self.links[link as usize];
            if l.offset == offset {
                if prev != NONE {
                    self.links[prev as usize].next = l.next;
                } else {
                    self.first = l.next;
                }
                self.free_list.push(link);
                return;
            }
            prev = link;
            link = l.next;
        }
    }
}

static mut CMEM_DYNAMIC: CmemDynamic = CmemDynamic::new();

#[cfg(not(test))]
mod host {
    unsafe extern "C" {
        pub static mut cmem: [u8; 0];
        pub static mut cmem_static_pointer: *mut u8;
        pub static mut cmem_dynamic_base: *mut u8;
    }
}

#[cfg(test)]
mod host {
    pub static mut cmem: [u8; 1_114_099] = [0; 1_114_099];
    pub static mut cmem_static_pointer: *mut u8 = core::ptr::null_mut();
    pub static mut cmem_dynamic_base: *mut u8 = core::ptr::null_mut();
}

#[allow(unused_unsafe)]
fn cmem_base() -> *mut u8 {
    // SAFETY: only the address is taken.
    unsafe { addr_of_mut!(host::cmem).cast() }
}

// ---------------------------------------------------------------- C ABI

/// `allocate_mem_static_block` (program start uses 1 MB).
#[unsafe(no_mangle)]
pub extern "C" fn qb_mem_static_new_block(size: u32) -> i32 {
    mem_static_new_block(size)
}

/// `mem_static_malloc`. Null with `*err` set when allocation failed.
///
/// # Safety
/// `err` valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_mem_static_malloc(size: u32, err: *mut i32) -> *mut u8 {
    match mem_static_malloc(size) {
        Ok(p) => p,
        Err(e) => {
            unsafe { *err = e };
            null_mut()
        }
    }
}

/// `mem_static_restore`.
#[unsafe(no_mangle)]
pub extern "C" fn qb_mem_static_restore(restore_point: *mut u8) {
    mem_static_restore(restore_point);
}

/// `cmem_dynamic_malloc`. On failure returns null and sets `*err`.
///
/// # Safety
/// `err` valid; `cmem_static_pointer` points into `cmem`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_cmem_dynamic_malloc(size: u32, err: *mut i32) -> *mut u8 {
    let base = cmem_base();
    unsafe {
        let static_top = (host::cmem_static_pointer as usize).wrapping_sub(base as usize) as u32;
        let d = &mut *addr_of_mut!(CMEM_DYNAMIC);
        match d.malloc(size, static_top) {
            Ok(o) => {
                host::cmem_dynamic_base = base.add(d.base as usize);
                base.add(o as usize)
            }
            Err(e) => {
                *err = e;
                null_mut()
            }
        }
    }
}

/// `cmem_dynamic_free`.
#[unsafe(no_mangle)]
pub extern "C" fn qb_cmem_dynamic_free(block: *mut u8) {
    if block.is_null() {
        return;
    }
    let o = (block as usize).wrapping_sub(cmem_base() as usize);
    // SAFETY: only the program thread uses the allocator.
    let d = unsafe { &mut *addr_of_mut!(CMEM_DYNAMIC) };
    if let Ok(o) = u32::try_from(o) {
        d.free(o);
    }
}

/// Empties the `cmem_dynamic` block list (tests/diff only).
#[doc(hidden)]
#[unsafe(no_mangle)]
pub extern "C" fn qb_test_cmem_dynamic_reset() {
    // SAFETY: test hook, single-threaded.
    unsafe { *addr_of_mut!(CMEM_DYNAMIC) = CmemDynamic::new() };
}

#[doc(hidden)]
pub fn mem_static_state() -> (*mut u8, *mut u8, *mut u8, u32) {
    // SAFETY: plain reads.
    unsafe { (*addr_of!(mem_static), *addr_of!(mem_static_pointer), *addr_of!(mem_static_limit), mem_static_size) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mem_static_bumps_aligned_and_starts_new_blocks() {
        let _g = crate::program::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(mem_static_new_block(64), 0);
        let (base, ..) = mem_static_state();
        let a = mem_static_malloc(5).unwrap();
        assert_eq!(a, base);
        let b = mem_static_malloc(8).unwrap();
        assert_eq!(b as usize - base as usize, 8);
        // 48 more bytes would end exactly at the limit: legacy uses '<', so a new block.
        let c = mem_static_malloc(48).unwrap();
        let (base2, ptr2, limit2, size2) = mem_static_state();
        assert_ne!(base2, base);
        assert_eq!(c, base2);
        assert_eq!(size2, 64 * 2 + 48);
        assert_eq!(ptr2 as usize - base2 as usize, 48);
        assert_eq!(limit2 as usize - base2 as usize, size2 as usize);
        // Restore inside the block, then outside (rewinds to the block start).
        mem_static_restore(base2.wrapping_add(8));
        assert_eq!(mem_static_state().1 as usize - base2 as usize, 8);
        mem_static_restore(base);
        assert_eq!(mem_static_state().1, base2);
        // The old block is still alive: writing to it is fine.
        unsafe { *a = 1 };
    }

    #[test]
    fn cmem_dynamic_first_fit_from_top() {
        let mut d = CmemDynamic::new();
        let st = 1280 + 65_536;
        assert_eq!(d.malloc(0, st), Ok(CMEM_TOP));
        let a = d.malloc(1, st).unwrap();
        assert_eq!(a, CMEM_TOP - 16);
        let b = d.malloc(32, st).unwrap();
        assert_eq!(b, CMEM_TOP - 48);
        let c = d.malloc(17, st).unwrap();
        assert_eq!(c, CMEM_TOP - 80);
        assert_eq!(d.base, CMEM_TOP - 80);
        d.free(b);
        // A 16-byte block fits into the 32-byte gap (top of the gap first).
        assert_eq!(d.malloc(16, st), Ok(CMEM_TOP - 32));
        assert_eq!(d.malloc(16, st), Ok(CMEM_TOP - 48));
        assert_eq!(d.malloc(16, st), Ok(CMEM_TOP - 96));
        d.free(12345);
        assert_eq!(d.malloc(65_537, st), Err(505));
        assert_eq!(d.malloc(65_536, CMEM_TOP - 100), Err(506));
    }

    #[test]
    fn cmem_dynamic_ffi_uses_cmem_addresses() {
        let _g = crate::program::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            host::cmem_static_pointer = cmem_base().add(1280 + 65_536);
            let mut e = 0;
            let p = qb_cmem_dynamic_malloc(100, &mut e);
            assert_eq!(e, 0);
            assert_eq!(p as usize - cmem_base() as usize, CMEM_TOP as usize - 112);
            assert_eq!({ host::cmem_dynamic_base }, p);
            qb_cmem_dynamic_free(p);
            let q = qb_cmem_dynamic_malloc(100, &mut e);
            assert_eq!(q, p);
            qb_cmem_dynamic_free(q);
        }
    }
}
