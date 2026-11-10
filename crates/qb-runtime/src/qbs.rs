//! The BASIC string engine (`qbs`): descriptor pool, tracking lists, the
//! `qbs_data` arena, conventional-memory (DBLOCK) strings and the string
//! functions built on them.
//!
//! Replaces libqb.cpp (`qbs_new_descriptor` … `qbs_len`) and the body of
//! `qbs_cleanup` in qbx.cpp. Ported bug-for-bug: generated C++ reads and writes
//! `struct qbs` fields directly, keeps `qbs_tmp_list_nexti` as its scope marker
//! and relies on the exact reuse rules (tmp strings acquired by `qbs_set`,
//! tmp strings cut in place by LEFT$/RIGHT$/MID$/trims, 32 spare bytes after
//! every arena string). The differential harness `tests/diff/diff_qbs.cpp` replays
//! random operation sequences against a verbatim copy of the C++ engine and
//! compares every descriptor, tracking list, arena offset and DBLOCK byte.
//!
//! The engine is a process-wide, single-threaded allocator like the legacy one:
//! all state lives in `ENGINE` and the two globals the generated code touches
//! (`qbs_tmp_list_nexti`, `qbs_cmem_sp`). Conventional memory (`cmem`,
//! `dblock`, `cmem_sp`) stays owned by libqb/qbx.cpp; errors are raised through
//! `qb_host_error` and FIELD buffers are released through
//! `qb_host_field_free`.

#![allow(non_upper_case_globals, clippy::missing_safety_doc)]

use core::ffi::{c_char, c_void};
use core::ptr::{self, addr_of_mut, null_mut};

/// `struct qbs` from internal/c/common.h (layout checked by the unit tests and
/// by diff_qbs.cpp with `offsetof`).
#[repr(C)]
pub struct Qbs {
    pub chr: *mut u8,
    pub len: i32,
    pub in_cmem: u8,
    pub cmem_descriptor: *mut u16,
    pub cmem_descriptor_offset: u16,
    pub listi: u32,
    pub tmp: u8,
    pub tmplisti: u32,
    pub fixed: u8,
    pub readonly: u8,
    pub field: *mut c_void,
}

const DESCRIPTOR_BLOCK: u32 = 65_536;
const TRACK_BLOCK: usize = 65_536;
const DATA_INITIAL: usize = 1_048_576;

/// Next free index of the tmp tracking list; generated code saves it as the
/// scope base (`uint32 qbs_tmp_base=qbs_tmp_list_nexti;`). Index 0 is a sentinel.
#[unsafe(no_mangle)]
pub static mut qbs_tmp_list_nexti: u32 = 1;
/// Top of the string area in DBLOCK (strings grow up from here).
#[unsafe(no_mangle)]
pub static mut qbs_cmem_sp: u32 = 256;
/// Start of the DBLOCK string area (room for 64 string descriptors).
#[unsafe(no_mangle)]
pub static mut qbs_cmem_descriptor_space: u32 = 256;

#[cfg(not(test))]
mod host {
    use super::Qbs;
    unsafe extern "C" {
        pub static mut cmem_sp: u32;
        pub static mut dblock: isize;
        pub static mut cmem: [u8; 0];
        pub fn qb_host_error(n: i32);
        pub fn qb_host_field_free(s: *mut Qbs);
    }
    #[inline]
    pub unsafe fn error(n: i32) {
        unsafe { qb_host_error(n) }
    }
    #[inline]
    pub unsafe fn field_free(s: *mut Qbs) {
        unsafe { qb_host_field_free(s) }
    }
    #[inline]
    pub unsafe fn cmem_base() -> *mut u8 {
        core::ptr::addr_of_mut!(cmem).cast()
    }
}

#[cfg(test)]
pub(crate) mod host {
    use super::Qbs;
    use core::ptr::{addr_of, addr_of_mut};
    pub static mut cmem_sp: u32 = 65_536;
    pub static mut dblock: isize = 0;
    static mut CMEM: [u8; 1_114_099] = [0; 1_114_099];
    static mut LAST_ERROR: i32 = 0;
    pub unsafe fn error(n: i32) {
        unsafe { LAST_ERROR = n }
    }
    pub unsafe fn field_free(_s: *mut Qbs) {}
    pub unsafe fn cmem_base() -> *mut u8 {
        addr_of_mut!(CMEM).cast()
    }
    pub unsafe fn reset() {
        unsafe {
            cmem_sp = 65_536;
            dblock = cmem_base() as isize + 1280;
        }
    }
    pub fn take_error() -> i32 {
        unsafe {
            let e = LAST_ERROR;
            LAST_ERROR = 0;
            e
        }
    }
    pub fn cmem_sp_value() -> u32 {
        unsafe { *addr_of!(cmem_sp) }
    }
    pub fn dblock_value() -> isize {
        unsafe { *addr_of!(dblock) }
    }
}

use host::{cmem_sp, dblock};

/// A block of 65536 string descriptors, owned through a raw pointer: generated
/// code keeps `qbs*` into it, and moving a `Box<[Qbs]>` into `blocks` retagged
/// it and invalidated those pointers under Rust's aliasing rules (found by
/// miri, R14). Freed when the engine is dropped (`qb_qbs_reset`).
pub(crate) struct DescBlock(*mut [Qbs]);

impl Drop for DescBlock {
    fn drop(&mut self) {
        // SAFETY: created by Box::into_raw and dropped once.
        drop(unsafe { Box::from_raw(self.0) });
    }
}

struct Engine {
    ready: bool,
    blocks: Vec<DescBlock>,
    malloc: *mut Qbs,
    malloc_next: u32,
    freed: Vec<*mut Qbs>,
    cmem_list: Vec<isize>,
    cmem_lasti: u32,
    cmem_nexti: u32,
    list: Vec<isize>,
    lasti: u32,
    nexti: u32,
    tmp_list: Vec<isize>,
    tmp_lasti: u32,
    data: Vec<u8>,
    data_ptr: *mut u8,
    data_size: u32,
    sp: u32,
}

static mut ENGINE: Engine = Engine {
    ready: false,
    blocks: Vec::new(),
    malloc: null_mut(),
    malloc_next: 0,
    freed: Vec::new(),
    cmem_list: Vec::new(),
    cmem_lasti: 65_535,
    cmem_nexti: 0,
    list: Vec::new(),
    lasti: 65_535,
    nexti: 0,
    tmp_list: Vec::new(),
    tmp_lasti: 65_535,
    data: Vec::new(),
    data_ptr: null_mut(),
    data_size: 0,
    sp: 0,
};

#[inline(always)]
unsafe fn eng() -> &'static mut Engine {
    let e = unsafe { &mut *addr_of_mut!(ENGINE) };
    if !e.ready {
        e.init();
    }
    e
}

#[inline(always)]
unsafe fn tmp_nexti() -> *mut u32 {
    addr_of_mut!(qbs_tmp_list_nexti)
}

#[inline(always)]
unsafe fn cmem_sp_ref() -> *mut u32 {
    addr_of_mut!(qbs_cmem_sp)
}

#[inline(always)]
unsafe fn dblock_ptr() -> *mut u8 {
    unsafe { *addr_of_mut!(dblock) as *mut u8 }
}

#[inline(always)]
unsafe fn copy(src: *const u8, dst: *mut u8, n: i32) {
    // memcpy in the legacy code; memmove here because a few call sites can
    // overlap (acquired tmp regions), where memmove is the defined result.
    if n > 0 {
        unsafe { ptr::copy(src, dst, n as usize) }
    }
}

#[inline(always)]
unsafe fn update_cmem_descriptor(s: *mut Qbs) {
    unsafe {
        let d = (*s).cmem_descriptor;
        if !d.is_null() {
            d.write_unaligned((*s).len as u16);
            d.add(1).write_unaligned(((*s).chr as isize).wrapping_sub(*addr_of_mut!(dblock)) as u16);
        }
    }
}

fn expand(list: &mut Vec<isize>, lasti: &mut u32) {
    let new_size = (list.len() * 2).max(TRACK_BLOCK);
    list.resize(new_size, 0);
    *lasti = (new_size - 1) as u32;
}

impl Engine {
    fn init(&mut self) {
        self.cmem_list = vec![0; TRACK_BLOCK];
        self.list = vec![0; TRACK_BLOCK];
        self.tmp_list = vec![0; TRACK_BLOCK];
        self.cmem_lasti = 65_535;
        self.lasti = 65_535;
        self.tmp_lasti = 65_535;
        self.data = vec![0; DATA_INITIAL];
        self.data_ptr = self.data.as_mut_ptr();
        self.data_size = DATA_INITIAL as u32;
        self.ready = true;
    }

    unsafe fn new_descriptor(&mut self) -> *mut Qbs {
        if let Some(d) = self.freed.pop() {
            unsafe { ptr::write_bytes(d, 0, 1) };
            return d;
        }
        if self.malloc.is_null() || self.malloc_next == DESCRIPTOR_BLOCK {
            let mut block: Vec<Qbs> = Vec::with_capacity(DESCRIPTOR_BLOCK as usize);
            unsafe {
                ptr::write_bytes(block.as_mut_ptr(), 0, DESCRIPTOR_BLOCK as usize);
                block.set_len(DESCRIPTOR_BLOCK as usize);
            }
            let block = DescBlock(Box::into_raw(block.into_boxed_slice()));
            self.malloc = block.0.cast();
            self.blocks.push(block);
            self.malloc_next = 0;
        }
        let d = unsafe { self.malloc.add(self.malloc_next as usize) };
        self.malloc_next += 1;
        d
    }

    #[inline(always)]
    fn free_descriptor(&mut self, s: *mut Qbs) {
        self.freed.push(s);
    }

    #[inline(always)]
    unsafe fn tmp_add(&mut self, s: *mut Qbs) {
        unsafe {
            let n = tmp_nexti();
            if *n > self.tmp_lasti {
                self.tmp_concat_list();
            }
            (*s).tmplisti = *n;
            self.tmp_list[*n as usize] = s as isize;
            *n += 1;
            (*s).tmp = 1;
        }
    }

    unsafe fn free(&mut self, s: *mut Qbs) {
        unsafe {
            if !(*s).field.is_null() {
                host::field_free(s);
            }
            if (*s).tmplisti != 0 {
                self.tmp_list[(*s).tmplisti as usize] = -1;
                let n = tmp_nexti();
                while self.tmp_list[(*n).wrapping_sub(1) as usize] == -1 {
                    *n -= 1;
                }
            }
            if (*s).fixed != 0 || (*s).readonly != 0 {
                self.free_descriptor(s);
                return;
            }
            if (*s).in_cmem != 0 {
                self.cmem_list[(*s).listi as usize] = -1;
                if self.cmem_nexti.wrapping_sub(1) == (*s).listi {
                    self.cmem_nexti -= 1;
                }
            } else {
                self.list[(*s).listi as usize] = -1;
                while self.list[self.nexti.wrapping_sub(1) as usize] == -1 {
                    self.nexti -= 1;
                    if self.nexti == 0 {
                        break;
                    }
                }
                if self.nexti != 0 {
                    let t = self.list[self.nexti as usize - 1] as *mut Qbs;
                    let off = ((*t).chr as isize).wrapping_sub(self.data_ptr as isize);
                    self.sp = off.wrapping_add((*t).len as isize).wrapping_add(32) as u32;
                    if self.sp > self.data_size {
                        self.sp = self.data_size;
                    }
                } else {
                    self.sp = 0;
                }
            }
            self.free_descriptor(s);
        }
    }

    unsafe fn cmem_concat_list(&mut self) {
        let mut d = 0u32;
        for i in 0..self.cmem_nexti {
            let v = self.cmem_list[i as usize];
            if v != -1 {
                if i != d {
                    unsafe { (*(v as *mut Qbs)).listi = d };
                    self.cmem_list[d as usize] = v;
                }
                d += 1;
            }
        }
        self.cmem_nexti = d;
        if self.cmem_nexti >= self.cmem_lasti / 2 {
            expand(&mut self.cmem_list, &mut self.cmem_lasti);
        }
    }

    unsafe fn concat_list(&mut self) {
        let mut d = 0u32;
        for i in 0..self.nexti {
            let v = self.list[i as usize];
            if v != -1 {
                if i != d {
                    unsafe { (*(v as *mut Qbs)).listi = d };
                    self.list[d as usize] = v;
                }
                d += 1;
            }
        }
        self.nexti = d;
        if self.nexti >= self.lasti / 2 {
            expand(&mut self.list, &mut self.lasti);
        }
    }

    unsafe fn tmp_concat_list(&mut self) {
        if unsafe { *tmp_nexti() } >= self.tmp_lasti / 2 {
            expand(&mut self.tmp_list, &mut self.tmp_lasti);
        }
    }

    /// Compacts the arena (moves strings down when a gap above 32 bytes opens)
    /// and doubles it when less than half would be free.
    unsafe fn concat(&mut self, bytesrequired: u32) {
        unsafe {
            let mut dest = self.data_ptr;
            if self.nexti != 0 {
                self.sp = 0;
                for i in 0..self.nexti {
                    let v = self.list[i as usize];
                    if v != -1 {
                        let t = v as *mut Qbs;
                        if ((*t).chr as isize).wrapping_sub(dest as isize) > 32 {
                            copy((*t).chr, dest, (*t).len);
                            (*t).chr = dest;
                        }
                        dest = (*t).chr.wrapping_offset((*t).len as isize);
                        self.sp = (dest as isize).wrapping_sub(self.data_ptr as isize) as u32;
                    }
                }
            }
            if self.sp.wrapping_mul(2).wrapping_add(bytesrequired.wrapping_add(32)) >= self.data_size {
                let old = self.data_ptr as usize;
                self.data_size = self.data_size.wrapping_mul(2).wrapping_add(bytesrequired);
                self.data.resize(self.data_size as usize, 0);
                self.data_ptr = self.data.as_mut_ptr();
                for i in 0..self.nexti {
                    let v = self.list[i as usize];
                    if v != -1 {
                        let t = v as *mut Qbs;
                        let off = ((*t).chr as usize).wrapping_sub(old);
                        (*t).chr = self.data_ptr.wrapping_add(off);
                    }
                }
            }
        }
    }

    /// Compacts DBLOCK strings; the area cannot grow (error 513 when full).
    unsafe fn concat_cmem(&mut self, bytesrequired: u32) {
        unsafe {
            let mut dest = dblock_ptr();
            let csp = cmem_sp_ref();
            *csp = *addr_of_mut!(qbs_cmem_descriptor_space);
            for i in 0..self.cmem_nexti {
                let v = self.cmem_list[i as usize];
                if v != -1 {
                    let t = v as *mut Qbs;
                    if (*t).chr != dest {
                        copy((*t).chr, dest, (*t).len);
                        (*t).chr = dest;
                        update_cmem_descriptor(t);
                    }
                    dest = dest.wrapping_offset((*t).len as isize);
                    *csp = (*csp).wrapping_add((*t).len as u32);
                }
            }
            if (*csp).wrapping_add(bytesrequired) > *addr_of_mut!(cmem_sp) {
                host::error(513);
            }
        }
    }

    unsafe fn new_cmem(&mut self, size: i32, tmp: u8) -> *mut Qbs {
        unsafe {
            let csp = cmem_sp_ref();
            if (*csp).wrapping_add(size as u32) > *addr_of_mut!(cmem_sp) {
                self.concat_cmem(size as u32);
            }
            let n = self.new_descriptor();
            (*n).len = size;
            if (*csp).wrapping_add(size as u32) > *addr_of_mut!(cmem_sp) {
                self.concat_cmem(size as u32);
            }
            (*n).chr = dblock_ptr().wrapping_add(*csp as usize);
            *csp = (*csp).wrapping_add(size as u32);
            (*n).in_cmem = 1;
            if self.cmem_nexti > self.cmem_lasti {
                self.cmem_concat_list();
            }
            (*n).listi = self.cmem_nexti;
            self.cmem_list[self.cmem_nexti as usize] = n as isize;
            self.cmem_nexti += 1;
            if tmp != 0 {
                self.tmp_add(n);
            } else {
                self.alloc_cmem_descriptor(n, 514);
            }
            n
        }
    }

    /// Reserves the 4-byte [length][offset] descriptor below `cmem_sp`.
    unsafe fn alloc_cmem_descriptor(&mut self, n: *mut Qbs, err: i32) {
        unsafe {
            let sp = &mut *addr_of_mut!(cmem_sp);
            *sp = sp.wrapping_sub(4);
            (*n).cmem_descriptor = dblock_ptr().wrapping_add(*sp as usize).cast();
            if *sp < *cmem_sp_ref() {
                host::error(err);
            }
            (*n).cmem_descriptor_offset = *sp as u16;
            update_cmem_descriptor(n);
        }
    }

    unsafe fn new_txt_len(&mut self, txt: *const u8, len: i32) -> *mut Qbs {
        unsafe {
            let n = self.new_descriptor();
            (*n).len = len;
            (*n).chr = txt as *mut u8;
            self.tmp_add(n);
            (*n).readonly = 1;
            n
        }
    }

    unsafe fn new_fixed(&mut self, offset: *mut u8, size: u32, tmp: u8) -> *mut Qbs {
        unsafe {
            let n = self.new_descriptor();
            (*n).len = size as i32;
            (*n).chr = offset;
            (*n).fixed = 1;
            if tmp != 0 {
                self.tmp_add(n);
            } else {
                let cm = host::cmem_base();
                if offset > cm.wrapping_add(1280) && offset < cm.wrapping_add(66_816) {
                    self.alloc_cmem_descriptor(n, 515);
                }
            }
            n
        }
    }

    #[inline]
    unsafe fn new_str(&mut self, size: i32, tmp: u8) -> *mut Qbs {
        unsafe {
            if self.sp.wrapping_add(size as u32).wrapping_add(32) > self.data_size {
                self.concat((size as u32).wrapping_add(32));
            }
            let n = self.new_descriptor();
            (*n).len = size;
            (*n).chr = self.data_ptr.wrapping_add(self.sp as usize);
            self.sp = self.sp.wrapping_add(size as u32).wrapping_add(32);
            if self.nexti > self.lasti {
                self.concat_list();
            }
            (*n).listi = self.nexti;
            self.list[self.nexti as usize] = n as isize;
            self.nexti += 1;
            if tmp != 0 {
                self.tmp_add(n);
            }
            n
        }
    }

    unsafe fn set(&mut self, dest: *mut Qbs, src: *mut Qbs) -> *mut Qbs {
        unsafe {
            if (*dest).fixed != 0 {
                if (*src).len >= (*dest).len {
                    copy((*src).chr, (*dest).chr, (*dest).len);
                } else {
                    copy((*src).chr, (*dest).chr, (*src).len);
                    ptr::write_bytes((*dest).chr.add((*src).len as usize), 32, ((*dest).len - (*src).len) as usize);
                }
                return self.set_return(dest, src);
            }
            // can deststr acquire srcstr?
            if (*src).tmp != 0 && (*src).fixed == 0 && (*src).readonly == 0 && (*src).in_cmem == (*dest).in_cmem {
                if (*dest).in_cmem != 0 {
                    self.cmem_list[(*dest).listi as usize] = -1;
                    self.cmem_list[(*src).listi as usize] = dest as isize;
                } else {
                    self.list[(*dest).listi as usize] = -1;
                    self.list[(*src).listi as usize] = dest as isize;
                }
                (*dest).listi = (*src).listi;
                self.tmp_list[(*src).tmplisti as usize] = -1;
                let n = tmp_nexti();
                if (*src).tmplisti == (*n).wrapping_sub(1) {
                    *n -= 1;
                }
                (*dest).chr = (*src).chr;
                (*dest).len = (*src).len;
                self.free_descriptor(src);
                update_cmem_descriptor(dest);
                return dest;
            }
            if (*src).len <= (*dest).len {
                copy((*src).chr, (*dest).chr, (*src).len);
                (*dest).len = (*src).len;
                return self.set_return(dest, src);
            }
            if (*dest).in_cmem != 0 {
                let limit = (*addr_of_mut!(dblock)).wrapping_add(*addr_of_mut!(cmem_sp) as isize);
                if self.set_in_place(dest, src, true, limit) {
                    return self.set_return(dest, src);
                }
                // "realloc" deststr
                self.cmem_list[(*dest).listi as usize] = -1;
                let csp = cmem_sp_ref();
                if (*csp).wrapping_add((*src).len as u32) > *addr_of_mut!(cmem_sp) {
                    self.concat_cmem((*src).len as u32);
                }
                if self.cmem_nexti > self.cmem_lasti {
                    self.cmem_concat_list();
                }
                (*dest).listi = self.cmem_nexti;
                self.cmem_list[self.cmem_nexti as usize] = dest as isize;
                self.cmem_nexti += 1;
                (*dest).chr = dblock_ptr().wrapping_add(*csp as usize);
                (*dest).len = (*src).len;
                *csp = (*csp).wrapping_add((*dest).len as u32);
                copy((*src).chr, (*dest).chr, (*src).len);
                return self.set_return(dest, src);
            }
            let limit = (self.data_ptr as isize).wrapping_add(self.data_size as isize);
            if self.set_in_place(dest, src, false, limit) {
                return self.set_return(dest, src);
            }
            self.list[(*dest).listi as usize] = -1;
            if self.sp.wrapping_add((*src).len as u32) > self.data_size {
                self.concat((*src).len as u32);
            }
            if self.nexti > self.lasti {
                self.concat_list();
            }
            (*dest).listi = self.nexti;
            self.list[self.nexti as usize] = dest as isize;
            self.nexti += 1;
            (*dest).chr = self.data_ptr.wrapping_add(self.sp as usize);
            (*dest).len = (*src).len;
            self.sp = self.sp.wrapping_add((*dest).len as u32);
            copy((*src).chr, (*dest).chr, (*src).len);
            self.set_return(dest, src)
        }
    }

    /// The "srcstr is longer" cases of qbs_set that grow deststr where it is.
    /// Returns false when deststr has to be reallocated.
    #[inline]
    unsafe fn set_in_place(&mut self, dest: *mut Qbs, src: *mut Qbs, cmem: bool, limit: isize) -> bool {
        unsafe {
            let (list, nexti) = if cmem {
                (self.cmem_list.as_mut_ptr(), &mut self.cmem_nexti)
            } else {
                (self.list.as_mut_ptr(), &mut self.nexti)
            };
            let fits = |d: *mut Qbs| ((*d).chr as isize).wrapping_add((*src).len as isize) <= limit;
            if (*dest).listi == nexti.wrapping_sub(1) {
                // last index
                if !fits(dest) {
                    return false;
                }
                copy((*src).chr, (*dest).chr, (*src).len);
                (*dest).len = (*src).len;
            } else {
                // locate the next valid index
                let mut i = (*dest).listi + 1;
                loop {
                    let v = *list.add(i as usize);
                    if v != -1 {
                        let t = v as *mut Qbs;
                        if !(t == src && (*src).tmp == 1) {
                            if (*dest).chr.wrapping_add((*src).len as usize) > (*t).chr {
                                return false;
                            }
                            copy((*src).chr, (*dest).chr, (*src).len);
                            (*dest).len = (*src).len;
                            return true;
                        }
                    }
                    i += 1;
                    if i == *nexti {
                        break;
                    }
                }
                // all next indexes invalid
                *nexti = (*dest).listi + 1;
                if !fits(dest) {
                    return false;
                }
                copy((*src).chr, (*dest).chr, (*src).len);
                (*dest).len = (*src).len;
            }
            let end = ((*dest).chr as isize).wrapping_add((*dest).len as isize);
            if cmem {
                *cmem_sp_ref() = end.wrapping_sub(*addr_of_mut!(dblock)) as u32;
            } else {
                self.sp = end.wrapping_sub(self.data_ptr as isize) as u32;
            }
            true
        }
    }

    #[inline]
    unsafe fn set_return(&mut self, dest: *mut Qbs, src: *mut Qbs) -> *mut Qbs {
        unsafe {
            if (*src).tmp != 0 {
                self.free(src);
            }
            update_cmem_descriptor(dest);
            dest
        }
    }

    #[inline]
    unsafe fn free_if_tmp(&mut self, s: *mut Qbs) {
        unsafe {
            if (*s).tmp != 0 {
                self.free(s);
            }
        }
    }
}

/// tmp, variable-length, writable and in the arena: may be modified in place.
#[inline(always)]
unsafe fn acquirable(s: *mut Qbs) -> bool {
    unsafe { (*s).tmp != 0 && (*s).fixed == 0 && (*s).readonly == 0 && (*s).in_cmem == 0 }
}

#[inline(always)]
unsafe fn bytes<'a>(s: *mut Qbs) -> &'a [u8] {
    unsafe {
        let n = (*s).len;
        if n <= 0 {
            &[]
        } else {
            core::slice::from_raw_parts((*s).chr, n as usize)
        }
    }
}

unsafe extern "C" {
    fn memchr(s: *const c_void, c: i32, n: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
}

// --- C ABI (include/qb_runtime.h); libqb.cpp keeps one-line shims with the old names ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_new_descriptor() -> *mut Qbs {
    unsafe { eng().new_descriptor() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_free_descriptor(s: *mut Qbs) {
    unsafe { eng().free_descriptor(s) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_free(s: *mut Qbs) {
    unsafe { eng().free(s) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_cmem_concat_list() {
    unsafe { eng().cmem_concat_list() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_concat_list() {
    unsafe { eng().concat_list() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_tmp_concat_list() {
    unsafe { eng().tmp_concat_list() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_concat(bytesrequired: u32) {
    unsafe { eng().concat(bytesrequired) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_concat_cmem(bytesrequired: u32) {
    unsafe { eng().concat_cmem(bytesrequired) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_new_cmem(size: i32, tmp: u8) -> *mut Qbs {
    unsafe { eng().new_cmem(size, tmp) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_new_txt(txt: *const c_char) -> *mut Qbs {
    unsafe {
        let len = if txt.is_null() { 0 } else { strlen(txt) as i32 };
        eng().new_txt_len(txt.cast(), len)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_new_txt_len(txt: *const c_char, len: i32) -> *mut Qbs {
    unsafe { eng().new_txt_len(txt.cast(), len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_new_fixed(offset: *mut u8, size: u32, tmp: u8) -> *mut Qbs {
    unsafe { eng().new_fixed(offset, size, tmp) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_new(size: i32, tmp: u8) -> *mut Qbs {
    unsafe { eng().new_str(size, tmp) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_maketmp(s: *mut Qbs) {
    unsafe { eng().tmp_add(s) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_set(dest: *mut Qbs, src: *mut Qbs) -> *mut Qbs {
    unsafe { eng().set(dest, src) }
}

/// Body of qbx.cpp's `qbs_cleanup`: frees the tmp strings created since `base`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_cleanup(base: u32) {
    unsafe {
        let e = eng();
        let n = tmp_nexti();
        while *n > base {
            *n -= 1;
            let v = e.tmp_list[*n as usize];
            if v != -1 {
                e.free(v as *mut Qbs);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_add(a: *mut Qbs, b: *mut Qbs) -> *mut Qbs {
    unsafe {
        if (*b).len == 0 {
            return a;
        }
        if (*a).len == 0 {
            return b;
        }
        let e = eng();
        let t = e.new_str((*a).len + (*b).len, 1);
        copy((*a).chr, (*t).chr, (*a).len);
        copy((*b).chr, (*t).chr.add((*a).len as usize), (*b).len);
        e.free_if_tmp(a);
        e.free_if_tmp(b);
        t
    }
}

unsafe fn change_case(s: *mut Qbs, upper: bool) -> *mut Qbs {
    unsafe {
        if (*s).len == 0 {
            return s;
        }
        let e = eng();
        let t = if acquirable(s) {
            s
        } else {
            let t = e.new_str((*s).len, 1);
            copy((*s).chr, (*t).chr, (*s).len);
            t
        };
        let c = core::slice::from_raw_parts_mut((*t).chr, (*s).len as usize);
        if upper {
            c.make_ascii_uppercase();
        } else {
            c.make_ascii_lowercase();
        }
        if t != s && (*s).tmp != 0 {
            e.free(s);
        }
        t
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_ucase(s: *mut Qbs) -> *mut Qbs {
    unsafe { change_case(s, true) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_lcase(s: *mut Qbs) -> *mut Qbs {
    unsafe { change_case(s, false) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_func_chr(value: i32) -> *mut Qbs {
    unsafe {
        if !(0..=255).contains(&value) {
            let t = eng().new_str(0, 1);
            host::error(5);
            t
        } else {
            let t = eng().new_str(1, 1);
            *(*t).chr = value as u8;
            t
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_func_varptr_helper(ty: u8, offset: u16) -> *mut Qbs {
    unsafe {
        let t = eng().new_str(3, 1);
        let c = (*t).chr;
        *c = ty;
        *c.add(1) = offset as u8;
        *c.add(2) = (offset >> 8) as u8;
        t
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_left(s: *mut Qbs, l: i32) -> *mut Qbs {
    unsafe {
        let l = l.min((*s).len).max(0);
        if l == (*s).len {
            return s;
        }
        if acquirable(s) {
            (*s).len = l;
            return s;
        }
        let e = eng();
        let t = e.new_str(l, 1);
        copy((*s).chr, (*t).chr, l);
        e.free_if_tmp(s);
        t
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_right(s: *mut Qbs, l: i32) -> *mut Qbs {
    unsafe {
        let l = l.min((*s).len).max(0);
        if l == (*s).len {
            return s;
        }
        if acquirable(s) {
            (*s).chr = (*s).chr.add(((*s).len - l) as usize);
            (*s).len = l;
            return s;
        }
        let e = eng();
        let t = e.new_str(l, 1);
        copy((*s).chr.add(((*s).len - l) as usize), (*t).chr, l);
        e.free_if_tmp(s);
        t
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_func_space(spaces: i32) -> *mut Qbs {
    unsafe { qb_func_string(spaces, 32) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_func_string(characters: i32, asciivalue: i32) -> *mut Qbs {
    unsafe {
        let n = characters.max(0);
        let t = eng().new_str(n, 1);
        if n != 0 {
            ptr::write_bytes((*t).chr, asciivalue as u8, n as usize);
        }
        t
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_set_qbs_size(target: *mut isize, newlength: i32) {
    unsafe {
        qb_qbs_set(*target as *mut Qbs, qb_func_space(newlength));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_func_instr(start: i32, s: *mut Qbs, sub: *mut Qbs, passed: i32) -> i32 {
    // QBNex difference: start can be 0 or negative
    unsafe {
        let mut start = if passed == 0 { 1 } else { start };
        let (slen, sublen) = ((*s).len, (*sub).len);
        if slen == 0 {
            return 0;
        }
        if start < 1 {
            start = 1;
            if sublen == 0 {
                return 0;
            }
        }
        if start > slen {
            return 0;
        }
        if sublen == 0 {
            return start;
        }
        if start.wrapping_add(sublen).wrapping_sub(1) > slen {
            return 0;
        }
        let limit = (*s).chr.add(slen as usize);
        let first = *(*sub).chr as i32;
        let needle = bytes(sub);
        let mut base = (*s).chr.add(start as usize - 1);
        loop {
            base = memchr(base.cast(), first, limit.offset_from(base) as usize).cast();
            if base.is_null() {
                return 0;
            }
            if base.add(sublen as usize) > limit {
                return 0;
            }
            if core::slice::from_raw_parts(base, sublen as usize) == needle {
                return base.offset_from((*s).chr) as i32 + 1;
            }
            base = base.add(1);
            if base.add(sublen as usize) > limit {
                return 0;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_func__instrrev(start: i32, s: *mut Qbs, sub: *mut Qbs, passed: i32) -> i32 {
    unsafe {
        let (slen, sublen) = ((*s).len, (*sub).len);
        if slen == 0 || sublen > slen {
            return 0;
        }
        let mut start = start;
        if passed == 0 {
            if sublen == slen && bytes(s) == bytes(sub) {
                return 1;
            }
            start = slen - sublen + 1;
        }
        if start < 1 {
            start = slen - sublen + 1;
        }
        if start > slen {
            start = slen - sublen + 1;
        }
        if sublen == 0 {
            return start - 1;
        }
        if start.wrapping_add(sublen).wrapping_sub(1) > slen {
            start = slen - sublen + 1;
        }
        let (mut forward, mut result) = (0, 0);
        loop {
            forward = qb_func_instr(forward + 1, s, sub, 1);
            if forward > 0 {
                if forward <= start {
                    result = forward;
                }
                if forward > start {
                    break;
                }
            } else {
                break;
            }
        }
        result
    }
}

/// MID$ statement after the `new_error` / `nothingstring` checks the C++ shim does.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_sub_mid(dest: *mut Qbs, start: i32, l: i32, src: *mut Qbs, passed: i32) {
    unsafe {
        // i64 so that `start + l` cannot wrap (R14: MID$(d$, 2, 2147483647) = s$
        // wrote past d$).
        let (dlen, slen) = ((*dest).len as i64, (*src).len as i64);
        let mut l = if passed == 0 { slen } else { l as i64 };
        let mut start = start as i64;
        let mut src_offset = 0i64;
        if start < 1 {
            l = l + start - 1;
            src_offset = 1 - start;
            start = 1;
        }
        if l <= 0 || start > dlen {
            return;
        }
        if start + l - 1 > dlen {
            l = dlen - start + 1;
        }
        if src_offset >= slen {
            return;
        }
        if l > slen - src_offset {
            l = slen - src_offset;
        }
        if dest == src && start - 1 == src_offset {
            return;
        }
        copy((*src).chr.add(src_offset as usize), (*dest).chr.add(start as usize - 1), l as i32);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_func_mid(s: *mut Qbs, start: i32, l: i32, passed: i32) -> *mut Qbs {
    unsafe {
        // i64 so that `start + l` cannot wrap (R14: MID$(a$, 2, 2147483647)
        // read past a$).
        let (mut start, mut l) = (start as i64, l as i64);
        let len64 = (*s).len as i64;
        if passed != 0 {
            if start < 1 {
                l = l - 1 + start;
                start = 1;
            }
            if l >= 1 && start <= len64 {
                if start + l > len64 {
                    l = len64 - start + 1;
                }
            } else {
                l = 0;
                start = 1;
            }
        } else {
            if start < 1 {
                start = 1;
            }
            l = len64 - start + 1;
            if l < 1 {
                l = 0;
                start = 1;
            }
        }
        let (start, l) = (start as i32, l as i32);
        let len = (*s).len;
        if start == 1 && l == len {
            return s;
        }
        if acquirable(s) {
            (*s).chr = (*s).chr.add(start as usize - 1);
            (*s).len = l;
            return s;
        }
        let e = eng();
        let t = e.new_str(l, 1);
        copy((*s).chr.add(start as usize - 1), (*t).chr, l);
        e.free_if_tmp(s);
        t
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_ltrim(s: *mut Qbs) -> *mut Qbs {
    unsafe {
        if (*s).len == 0 || *(*s).chr != 32 {
            return s;
        }
        if acquirable(s) {
            while *(*s).chr == 32 {
                (*s).chr = (*s).chr.add(1);
                (*s).len -= 1;
                if (*s).len == 0 {
                    break;
                }
            }
            return s;
        }
        let b = bytes(s);
        let i = b.iter().position(|&c| c != 32).unwrap_or(b.len()) as i32;
        let e = eng();
        let t = e.new_str((*s).len - i, 1);
        copy((*s).chr.add(i as usize), (*t).chr, (*t).len);
        e.free_if_tmp(s);
        t
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_rtrim(s: *mut Qbs) -> *mut Qbs {
    unsafe {
        if (*s).len == 0 || *(*s).chr.add((*s).len as usize - 1) != 32 {
            return s;
        }
        if acquirable(s) {
            while *(*s).chr.add((*s).len as usize - 1) == 32 {
                (*s).len -= 1;
                if (*s).len == 0 {
                    break;
                }
            }
            return s;
        }
        let b = bytes(s);
        let i = b.iter().rposition(|&c| c != 32).map_or(0, |p| p + 1) as i32;
        let e = eng();
        let t = e.new_str(i, 1);
        copy((*s).chr, (*t).chr, i);
        e.free_if_tmp(s);
        t
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs__trim(s: *mut Qbs) -> *mut Qbs {
    unsafe { qb_qbs_rtrim(qb_qbs_ltrim(s)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_func__str_nc_compare(a: *mut Qbs, b: *mut Qbs) -> i32 {
    unsafe {
        let (x, y) = (bytes(a), bytes(b));
        if x.is_empty() {
            return -i32::from(!y.is_empty());
        }
        if y.is_empty() {
            return 1;
        }
        for (&c1, &c2) in x.iter().zip(y) {
            let (v1, v2) = (c1.to_ascii_lowercase(), c2.to_ascii_lowercase());
            if v1 != v2 {
                return if v1 < v2 { -1 } else { 1 };
            }
        }
        (x.len() as i32 - y.len() as i32).signum()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_func__str_compare(a: *mut Qbs, b: *mut Qbs) -> i32 {
    unsafe {
        let (x, y) = (bytes(a), bytes(b));
        if x.is_empty() {
            return -i32::from(!y.is_empty());
        }
        if y.is_empty() {
            return 1;
        }
        x.cmp(y) as i32
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_equal(a: *mut Qbs, b: *mut Qbs) -> i32 {
    unsafe { -i32::from((*a).len == (*b).len && bytes(a) == bytes(b)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_notequal(a: *mut Qbs, b: *mut Qbs) -> i32 {
    unsafe { -i32::from(!((*a).len == (*b).len && bytes(a) == bytes(b))) }
}

/// Shared body of `<` / `<=` (legacy argument order: `less(str1, str2)`).
#[inline]
unsafe fn less(s1: *mut Qbs, s2: *mut Qbs, or_equal: bool) -> i32 {
    unsafe {
        let (l1, l2) = ((*s1).len, (*s2).len);
        if l1 == 0 {
            return if or_equal || l2 != 0 { -1 } else { 0 };
        }
        let limit = l1.min(l2).max(0) as usize;
        let i = core::slice::from_raw_parts((*s1).chr, limit).cmp(core::slice::from_raw_parts((*s2).chr, limit));
        match i {
            core::cmp::Ordering::Less => -1,
            core::cmp::Ordering::Greater => 0,
            core::cmp::Ordering::Equal => -i32::from(if or_equal { l1 <= l2 } else { l1 < l2 }),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_lessthan(a: *mut Qbs, b: *mut Qbs) -> i32 {
    unsafe { less(a, b, false) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_greaterthan(a: *mut Qbs, b: *mut Qbs) -> i32 {
    unsafe { less(b, a, false) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_lessorequal(a: *mut Qbs, b: *mut Qbs) -> i32 {
    unsafe { less(a, b, true) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_greaterorequal(a: *mut Qbs, b: *mut Qbs) -> i32 {
    unsafe { less(b, a, true) }
}

/// `ASC(s$, i)`; `i` is unsigned so 0 and negatives fail like out-of-range indexes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_asc(s: *mut Qbs, i: u32) -> i32 {
    unsafe {
        let i = i.wrapping_sub(1);
        if i < (*s).len as u32 {
            return *(*s).chr.add(i as usize) as i32;
        }
        host::error(5);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_asc1(s: *mut Qbs) -> i32 {
    unsafe {
        if (*s).len != 0 {
            return *(*s).chr as i32;
        }
        host::error(5);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_len(s: *mut Qbs) -> i32 {
    unsafe { (*s).len }
}

// --- introspection for tests and the differential harness ----------------------

/// Snapshot of the engine's internals (diff_qbs.cpp compares it with the legacy globals).
#[repr(C)]
pub struct QbsDebugState {
    pub list: *const isize,
    pub list_nexti: u32,
    pub list_lasti: u32,
    pub cmem_list: *const isize,
    pub cmem_list_nexti: u32,
    pub cmem_list_lasti: u32,
    pub tmp_list: *const isize,
    pub tmp_list_lasti: u32,
    pub data: *const u8,
    pub data_size: u32,
    pub sp: u32,
    pub freed: *const *mut Qbs,
    pub freed_count: u32,
    pub blocks: u32,
    pub malloc_next: u32,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_debug_state(out: *mut QbsDebugState) {
    unsafe {
        let e = eng();
        out.write(QbsDebugState {
            list: e.list.as_ptr(),
            list_nexti: e.nexti,
            list_lasti: e.lasti,
            cmem_list: e.cmem_list.as_ptr(),
            cmem_list_nexti: e.cmem_nexti,
            cmem_list_lasti: e.cmem_lasti,
            tmp_list: e.tmp_list.as_ptr(),
            tmp_list_lasti: e.tmp_lasti,
            data: e.data_ptr,
            data_size: e.data_size,
            sp: e.sp,
            freed: e.freed.as_ptr(),
            freed_count: e.freed.len() as u32,
            blocks: e.blocks.len() as u32,
            malloc_next: e.malloc_next,
        });
    }
}

/// Position of a descriptor in the pool (block * 65536 + slot), or -1.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_debug_descriptor_index(s: *const Qbs) -> i64 {
    unsafe {
        let e = eng();
        for (b, block) in e.blocks.iter().enumerate() {
            let base = block.0.cast::<Qbs>().cast_const();
            if s >= base && s < base.add(block.0.len()) {
                return (b as i64) * i64::from(DESCRIPTOR_BLOCK) + s.offset_from(base) as i64;
            }
        }
        -1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_debug_sp() -> u32 {
    unsafe { eng().sp }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_debug_data_size() -> u32 {
    unsafe { eng().data_size }
}

/// Drops all strings and returns the engine to its start-up state (tests only;
/// every outstanding `qbs*` becomes dangling).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_qbs_reset() {
    unsafe {
        let e = &mut *addr_of_mut!(ENGINE);
        *e = Engine {
            ready: false,
            blocks: Vec::new(),
            malloc: null_mut(),
            malloc_next: 0,
            freed: Vec::new(),
            cmem_list: Vec::new(),
            cmem_lasti: 65_535,
            cmem_nexti: 0,
            list: Vec::new(),
            lasti: 65_535,
            nexti: 0,
            tmp_list: Vec::new(),
            tmp_lasti: 65_535,
            data: Vec::new(),
            data_ptr: null_mut(),
            data_size: 0,
            sp: 0,
        };
        *tmp_nexti() = 1;
        *cmem_sp_ref() = 256;
        *addr_of_mut!(qbs_cmem_descriptor_space) = 256;
        #[cfg(test)]
        host::reset();
    }
}

#[cfg(test)]
#[path = "qbs_tests.rs"]
pub(crate) mod tests;
