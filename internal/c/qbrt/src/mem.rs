//! `_MEM` family: lock tags (`mem_lock`), `_MEMNEW`, `_MEMFREE`, `_MEMEXISTS`,
//! `_MEMGET`, `_MEMFILL`, `_MEMCOPY`, `_MEM(offset, size)`.
//!
//! Replaces `new_mem_lock`, `free_mem_lock`, `func__mem*` and `sub__mem*` of
//! libqb.cpp. Generated C++ code reads and writes `mem_block` / `mem_lock`
//! fields and the globals `mem_lock_id` / `mem_lock_tmp` directly, so their
//! layout and symbol names are part of the ABI (`internal/c/common.h`).
//!
//! Error codes are returned to the C++ shim, which raises them with `error()`.

use std::collections::BTreeMap;
use std::ffi::c_void;
use std::ptr::{addr_of_mut, null_mut};
use std::sync::Mutex;

/// `lock_id` written into a block by `_MEMFREE`; never a live lock id.
pub const MEM_LOCK_INVALID_ID: u64 = 1_073_741_821;
/// First lock id handed out is this + 1 (must never be 0 or 1).
pub const MEM_LOCK_INITIAL_ID: u64 = 1_073_741_823;
const MEM_LOCK_BLOCK_SIZE: usize = 10_000;
/// `mem_block.type` of `_MEMNEW` / `_MEM(offset, size)` blocks.
pub const MEMNEW_TYPE: isize = 16_384;

/// `mem_block` in common.h.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MemBlock {
    pub offset: isize,
    pub size: isize,
    pub lock_id: i64,
    pub lock_offset: isize,
    pub type_: isize,
    pub elementsize: isize,
    pub image: i32,
    pub sound: i32,
}

/// `mem_lock` in common.h. `type_`: 0 unsecured, 1 `_MEMNEW` allocation,
/// 2 image, 3 SUB/FUNCTION scope, 4 array, 5 sound.
#[repr(C)]
#[derive(Debug)]
pub struct MemLock {
    pub id: u64,
    pub type_: i32,
    pub offset: *mut c_void,
}

const EMPTY_LOCK: MemLock = MemLock { id: 0, type_: 0, offset: null_mut() };

/// Last lock id handed out. Generated code also increments it directly.
#[no_mangle]
#[allow(non_upper_case_globals)]
pub static mut mem_lock_id: u64 = MEM_LOCK_INITIAL_ID;

/// Lock created by the last `new_mem_lock()` call (read by generated code).
#[no_mangle]
#[allow(non_upper_case_globals)]
pub static mut mem_lock_tmp: *mut MemLock = null_mut();

/// First lock of the newest lock block (legacy global; `_MEMSOUND` uses it as
/// the lock of an invalid block).
#[no_mangle]
#[allow(non_upper_case_globals)]
pub static mut mem_lock_base: *mut MemLock = null_mut();

/// Lock referenced by invalid blocks (`_MEMIMAGE` failures); its id is 0.
static mut MEM_LOCK_INVALID: MemLock = EMPTY_LOCK;

struct Pool {
    /// Locks live in fixed blocks so their addresses never move.
    blocks: Vec<Box<[MemLock]>>,
    next: usize,
    freed: Vec<*mut MemLock>,
    /// `_MEMNEW` allocations keyed by the address of their lock.
    memnew: BTreeMap<usize, Box<[u8]>>,
    /// Zeroed buffer returned by `_MEMGET` when a check fails.
    fail_buffer: Vec<u8>,
}

// SAFETY: the raw pointers only refer to locks owned by `blocks`.
unsafe impl Send for Pool {}

static POOL: Mutex<Pool> = Mutex::new(Pool {
    blocks: Vec::new(),
    next: MEM_LOCK_BLOCK_SIZE,
    freed: Vec::new(),
    memnew: BTreeMap::new(),
    fail_buffer: Vec::new(),
});

fn pool() -> std::sync::MutexGuard<'static, Pool> {
    POOL.lock().unwrap_or_else(|e| e.into_inner())
}

/// `new_mem_lock()`: takes a fresh (or recycled) lock, gives it the next id,
/// stores it in `mem_lock_tmp` and returns it.
pub fn new_mem_lock() -> *mut MemLock {
    let lock = {
        let mut p = pool();
        if let Some(l) = p.freed.pop() {
            l
        } else {
            if p.next == MEM_LOCK_BLOCK_SIZE {
                let block: Box<[MemLock]> = (0..MEM_LOCK_BLOCK_SIZE).map(|_| EMPTY_LOCK).collect();
                p.blocks.push(block);
                p.next = 0;
                let base = p.blocks.last_mut().expect("just pushed").as_mut_ptr();
                // SAFETY: plain pointer store; the runtime is single-threaded here.
                unsafe { *addr_of_mut!(mem_lock_base) = base };
            }
            let i = p.next;
            p.next += 1;
            let block = p.blocks.last_mut().expect("block allocated above");
            &mut block[i] as *mut MemLock
        }
    };
    // SAFETY: single-threaded runtime contract of libqb; `lock` points into `blocks`.
    unsafe {
        let id = addr_of_mut!(mem_lock_id);
        *id += 1;
        (*lock).id = *id;
        (*lock).type_ = 0;
        (*lock).offset = null_mut();
        *addr_of_mut!(mem_lock_tmp) = lock;
    }
    lock
}

/// `free_mem_lock()`: invalidates the lock, releases a `_MEMNEW` allocation and
/// recycles the lock.
///
/// # Safety
/// `lock` must come from [`new_mem_lock`].
pub unsafe fn free_mem_lock(lock: *mut MemLock) {
    let mut p = pool();
    // SAFETY: caller contract.
    unsafe {
        (*lock).id = 0;
        if (*lock).type_ == 1 {
            p.memnew.remove(&(lock as usize));
        }
        (*lock).type_ = 0;
        (*lock).offset = null_mut();
    }
    p.freed.push(lock);
}

/// Lock id stored at `lock_offset`, or `None` for a null lock pointer.
///
/// # Safety
/// `lock_offset` must be null or point to a readable `MemLock`.
unsafe fn lock_of(lock_offset: isize) -> Option<u64> {
    if lock_offset == 0 {
        None
    } else {
        // SAFETY: caller contract.
        Some(unsafe { (*(lock_offset as *const MemLock)).id })
    }
}

/// True when `[off, off+bytes)` lies inside the block.
fn in_range(b: &MemBlock, off: isize, bytes: isize) -> bool {
    !(off < b.offset || off.wrapping_add(bytes) > b.offset.wrapping_add(b.size))
}

/// `_MEMFREE`. Returns 0 or the error code (309 no lock, 307 already freed).
///
/// Legacy freed a `_MEMNEW` buffer twice (vector erase + `free`), corrupting
/// the heap (L-08); here it is released once (D-06).
///
/// # Safety
/// `blk.lock_offset` must be null or point to a lock from [`new_mem_lock`].
pub unsafe fn memfree(blk: &mut MemBlock) -> i32 {
    // SAFETY: caller contract.
    let Some(id) = (unsafe { lock_of(blk.lock_offset) }) else { return 309 };
    if id != blk.lock_id as u64 {
        return 307;
    }
    let lock = blk.lock_offset as *mut MemLock;
    // SAFETY: caller contract; type 0 and 1 locks were created by new_mem_lock.
    unsafe {
        if (*lock).type_ == 0 || (*lock).type_ == 1 {
            free_mem_lock(lock);
        }
    }
    blk.lock_id = MEM_LOCK_INVALID_ID as i64;
    0
}

/// `_MEM(offset, size)`: an unsecured block over arbitrary memory.
pub fn mem_at_offset(offset: isize, size: isize, new_error: bool) -> (MemBlock, i32) {
    let lock = new_mem_lock();
    // SAFETY: fresh lock from the pool.
    let id = unsafe { (*lock).id };
    let mut b = MemBlock {
        offset,
        size,
        lock_id: id as i64,
        lock_offset: lock as isize,
        type_: MEMNEW_TYPE,
        elementsize: 1,
        image: -1,
        sound: 0,
    };
    let mut err = 0;
    if size < 0 || new_error {
        b.type_ = 0;
        b.size = 0;
        b.offset = 0;
        if size < 0 {
            err = 301;
        }
    }
    (b, err)
}

/// `_MEMNEW(bytes)`. A negative size still returns a (useless) block plus error 5.
pub fn memnew(bytes: isize, new_error: bool) -> (MemBlock, i32) {
    let lock = new_mem_lock();
    // SAFETY: fresh lock from the pool.
    let id = unsafe { (*lock).id };
    let mut b = MemBlock {
        offset: 0,
        size: 0,
        lock_id: id as i64,
        lock_offset: lock as isize,
        type_: MEMNEW_TYPE,
        elementsize: 1,
        image: -1,
        sound: 0,
    };
    if new_error {
        b.type_ = 0;
        return (b, 0);
    }
    if bytes < 0 {
        return (b, 5);
    }
    if bytes == 0 {
        b.offset = 1; // non-zero = success
        return (b, 0);
    }
    let mut v: Vec<u8> = Vec::new();
    if v.try_reserve_exact(bytes as usize).is_err() {
        return (b, 0);
    }
    v.resize(bytes as usize, 0);
    let mut buf = v.into_boxed_slice();
    let ptr = buf.as_mut_ptr();
    pool().memnew.insert(lock as usize, buf);
    b.offset = ptr as isize;
    b.size = bytes;
    // SAFETY: fresh lock from the pool.
    unsafe {
        (*lock).type_ = 1;
        (*lock).offset = ptr.cast();
    }
    (b, 0)
}

/// `_MEMEXISTS`: -1 while the block's lock is alive, else 0.
///
/// # Safety
/// `blk.lock_offset` must be null or point to a readable `MemLock`.
pub unsafe fn memexists(blk: &MemBlock) -> i32 {
    // SAFETY: caller contract.
    match unsafe { lock_of(blk.lock_offset) } {
        Some(id) if id == blk.lock_id as u64 => -1,
        _ => 0,
    }
}

/// `_MEMGET` address check. Returns the address to read from and an error
/// code; on error the address is a zeroed scratch buffer of `bytes` bytes.
///
/// # Safety
/// `blk.lock_offset` must be null or point to a readable `MemLock`.
pub unsafe fn memget(blk: &MemBlock, off: isize, bytes: isize) -> (*mut c_void, i32) {
    // SAFETY: caller contract.
    let err = match unsafe { lock_of(blk.lock_offset) } {
        None => 309,
        Some(id) if id != blk.lock_id as u64 => 308,
        Some(_) if !in_range(blk, off, bytes) => 300,
        Some(_) => return (off as *mut c_void, 0),
    };
    if bytes <= 0 {
        // Legacy: an empty scratch vector has no data(), reported as critical
        // "out of memory" (518) right after the check error.
        return (null_mut(), 518);
    }
    let mut p = pool();
    p.fail_buffer.clear();
    p.fail_buffer.resize(bytes as usize, 0);
    (p.fail_buffer.as_mut_ptr().cast(), err)
}

/// `_MEMFILL` without checks: repeats the `sbytes`-byte pattern at `src` over
/// `dbytes` bytes at `doff`.
///
/// # Safety
/// `doff` must be writable for `dbytes` bytes and `src` readable for the pattern.
pub unsafe fn memfill_nochecks(doff: isize, dbytes: isize, src: *const u8, sbytes: isize) {
    if dbytes <= 0 {
        return;
    }
    let dst = doff as *mut u8;
    // SAFETY: caller contract.
    unsafe {
        if sbytes == 1 {
            std::ptr::write_bytes(dst, *src, dbytes as usize);
            return;
        }
        // Legacy wraps the source index after each byte (`si>=sbytes`), so a
        // zero or negative pattern length repeats the first byte.
        let mut si: isize = 0;
        for d in 0..dbytes {
            *dst.offset(d) = *src.offset(si);
            si += 1;
            if si >= sbytes {
                si = 0;
            }
        }
    }
}

/// `_MEMFILL` with checks. Returns 0 or 309/308/301/300.
///
/// # Safety
/// As [`memfill_nochecks`] once the checks pass; `blk.lock_offset` null or valid.
pub unsafe fn memfill(blk: &MemBlock, doff: isize, dbytes: isize, src: *const u8, sbytes: isize) -> i32 {
    // SAFETY: caller contract.
    match unsafe { lock_of(blk.lock_offset) } {
        None => return 309,
        Some(id) if id != blk.lock_id as u64 => return 308,
        Some(_) => {}
    }
    if dbytes < 0 || sbytes == 0 {
        return 301;
    }
    if !in_range(blk, doff, dbytes) {
        return 300;
    }
    // SAFETY: range checked against the block.
    unsafe { memfill_nochecks(doff, dbytes, src, sbytes) };
    0
}

/// `_MEMCOPY`. Returns 0 or one of 300-313 as legacy did.
///
/// # Safety
/// Lock offsets null or valid; ranges inside their blocks are accessible.
pub unsafe fn memcopy(s: &MemBlock, soff: isize, bytes: isize, d: &MemBlock, doff: isize) -> i32 {
    // SAFETY: caller contract.
    let (sid, did) = unsafe { (lock_of(s.lock_offset), lock_of(d.lock_offset)) };
    let (sid, did) = match (sid, did) {
        (None, None) => return 312,
        (None, _) => return 310,
        (_, None) => return 311,
        (Some(a), Some(b)) => (a, b),
    };
    let sbad = sid != s.lock_id as u64;
    let dbad = did != d.lock_id as u64;
    let srange = !in_range(s, soff, bytes);
    let drange = !in_range(d, doff, bytes);
    if bytes < 0 || srange || drange || sbad || dbad {
        if sbad && dbad {
            return 313;
        }
        if sbad {
            return 305;
        }
        if dbad {
            return 306;
        }
        if bytes < 0 {
            return 301;
        }
        if srange {
            return if drange { 304 } else { 302 };
        }
        return 303;
    }
    // SAFETY: both ranges checked against live blocks.
    unsafe { std::ptr::copy(soff as *const u8, doff as *mut u8, bytes as usize) };
    0
}

/// `_MEM(variable)`: block over a variable guarded by an existing lock.
///
/// # Safety
/// `lock` must point to a readable `MemLock`.
pub unsafe fn mem(offset: isize, size: isize, type_: i32, elementsize: isize, lock: *mut MemLock) -> MemBlock {
    MemBlock {
        offset,
        size,
        // SAFETY: caller contract.
        lock_id: unsafe { (*lock).id } as i64,
        lock_offset: lock as isize,
        type_: type_ as isize,
        elementsize,
        image: -1,
        sound: 0,
    }
}

// --- C ABI (include/qbrt.h) ---------------------------------------------------

#[no_mangle]
pub extern "C" fn qbrt_new_mem_lock() -> *mut MemLock {
    new_mem_lock()
}

/// # Safety
/// `lock` must come from `qbrt_new_mem_lock`.
#[no_mangle]
pub unsafe extern "C" fn qbrt_free_mem_lock(lock: *mut MemLock) {
    // SAFETY: caller contract.
    unsafe { free_mem_lock(lock) }
}

#[no_mangle]
pub extern "C" fn qbrt_mem_lock_invalid() -> *mut MemLock {
    addr_of_mut!(MEM_LOCK_INVALID)
}

/// # Safety
/// `blk` must point to a `mem_block`.
#[no_mangle]
pub unsafe extern "C" fn qbrt_memfree(blk: *mut MemBlock) -> i32 {
    // SAFETY: caller contract.
    unsafe { memfree(&mut *blk) }
}

/// # Safety
/// `out` must be writable.
#[no_mangle]
pub unsafe extern "C" fn qbrt_mem_at_offset(offset: isize, size: isize, new_error: i32, out: *mut MemBlock) -> i32 {
    let (b, e) = mem_at_offset(offset, size, new_error != 0);
    // SAFETY: caller contract.
    unsafe { out.write(b) };
    e
}

/// # Safety
/// `out` must be writable.
#[no_mangle]
pub unsafe extern "C" fn qbrt_memnew(bytes: isize, new_error: i32, out: *mut MemBlock) -> i32 {
    let (b, e) = memnew(bytes, new_error != 0);
    // SAFETY: caller contract.
    unsafe { out.write(b) };
    e
}

/// # Safety
/// `blk` must point to a `mem_block`.
#[no_mangle]
pub unsafe extern "C" fn qbrt_memexists(blk: *const MemBlock) -> i32 {
    // SAFETY: caller contract.
    unsafe { memexists(&*blk) }
}

/// # Safety
/// `blk` must point to a `mem_block`; `err` writable.
#[no_mangle]
pub unsafe extern "C" fn qbrt_memget(blk: *const MemBlock, off: isize, bytes: isize, err: *mut i32) -> *mut c_void {
    // SAFETY: caller contract.
    let (p, e) = unsafe { memget(&*blk, off, bytes) };
    // SAFETY: caller contract.
    unsafe { *err = e };
    p
}

/// # Safety
/// See [`memfill`].
#[no_mangle]
pub unsafe extern "C" fn qbrt_memfill(
    blk: *const MemBlock,
    doff: isize,
    dbytes: isize,
    src: *const u8,
    sbytes: isize,
) -> i32 {
    // SAFETY: caller contract.
    unsafe { memfill(&*blk, doff, dbytes, src, sbytes) }
}

/// # Safety
/// See [`memfill_nochecks`].
#[no_mangle]
pub unsafe extern "C" fn qbrt_memfill_nochecks(doff: isize, dbytes: isize, src: *const u8, sbytes: isize) {
    // SAFETY: caller contract.
    unsafe { memfill_nochecks(doff, dbytes, src, sbytes) }
}

/// # Safety
/// See [`memcopy`].
#[no_mangle]
pub unsafe extern "C" fn qbrt_memcopy(
    sblk: *const MemBlock,
    soff: isize,
    bytes: isize,
    dblk: *const MemBlock,
    doff: isize,
) -> i32 {
    // SAFETY: caller contract.
    unsafe { memcopy(&*sblk, soff, bytes, &*dblk, doff) }
}

/// # Safety
/// See [`mem`]; `out` writable.
#[no_mangle]
pub unsafe extern "C" fn qbrt_mem(
    offset: isize,
    size: isize,
    type_: i32,
    elementsize: isize,
    lock: *mut MemLock,
    out: *mut MemBlock,
) {
    // SAFETY: caller contract.
    unsafe { out.write(mem(offset, size, type_, elementsize, lock)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes_of(b: &MemBlock) -> &'static [u8] {
        // SAFETY: tests only read blocks they just allocated.
        unsafe { std::slice::from_raw_parts(b.offset as *const u8, b.size as usize) }
    }

    #[test]
    fn layouts_match_common_h() {
        assert_eq!(std::mem::size_of::<MemBlock>(), 6 * std::mem::size_of::<isize>() + 8);
        assert_eq!(std::mem::size_of::<MemLock>(), 8 + 8 + std::mem::size_of::<usize>());
    }

    #[test]
    fn new_lock_gets_next_id_and_is_published() {
        let a = new_mem_lock();
        let b = new_mem_lock();
        unsafe {
            assert!((*a).id > MEM_LOCK_INITIAL_ID);
            assert!((*b).id > (*a).id);
            assert_eq!((*b).type_, 0);
            assert!((*b).offset.is_null());
            assert!(!a.is_null() && a != b);
            free_mem_lock(a);
            assert_eq!((*a).id, 0);
            free_mem_lock(b);
        }
    }

    #[test]
    fn memnew_allocates_zeroed_block() {
        let (b, e) = memnew(16, false);
        assert_eq!(e, 0);
        assert_eq!((b.size, b.type_, b.elementsize, b.image, b.sound), (16, MEMNEW_TYPE, 1, -1, 0));
        assert_eq!(bytes_of(&b), &[0u8; 16]);
        unsafe {
            let lock = b.lock_offset as *mut MemLock;
            assert_eq!((*lock).type_, 1);
            assert_eq!((*lock).id, b.lock_id as u64);
            assert_eq!(memexists(&b), -1);
        }
    }

    #[test]
    fn memnew_zero_negative_and_after_error() {
        let (z, e) = memnew(0, false);
        assert_eq!((z.offset, z.size, e), (1, 0, 0));
        let (n, e) = memnew(-1, false);
        assert_eq!((n.offset, n.size, n.type_, e), (0, 0, MEMNEW_TYPE, 5));
        let (x, e) = memnew(10, true);
        assert_eq!((x.offset, x.size, x.type_, e), (0, 0, 0, 0));
    }

    #[test]
    fn memfree_invalidates_and_reports_double_free() {
        let (mut b, _) = memnew(8, false);
        unsafe {
            assert_eq!(memfree(&mut b), 0);
            assert_eq!(b.lock_id as u64, MEM_LOCK_INVALID_ID);
            assert_eq!(memexists(&b), 0);
            assert_eq!(memfree(&mut b), 307);
            let mut null = MemBlock::default();
            assert_eq!(memfree(&mut null), 309);
        }
    }

    #[test]
    fn memget_checks() {
        let (b, _) = memnew(8, false);
        unsafe {
            assert_eq!(memget(&b, b.offset + 4, 4), ((b.offset + 4) as *mut c_void, 0));
            let (p, e) = memget(&b, b.offset + 5, 4);
            assert_eq!(e, 300);
            assert_ne!(p as isize, b.offset);
            assert_eq!(std::slice::from_raw_parts(p as *const u8, 4), &[0; 4]);
            assert_eq!(memget(&b, b.offset - 1, 1).1, 300);
            assert_eq!(memget(&MemBlock::default(), 0, 1).1, 309);
            assert_eq!(memget(&b, b.offset + 9, 0).1, 518);
            let mut stale = b;
            stale.lock_id += 1;
            assert_eq!(memget(&stale, b.offset, 1).1, 308);
        }
    }

    #[test]
    fn memfill_patterns_and_errors() {
        let (b, _) = memnew(10, false);
        unsafe {
            assert_eq!(memfill(&b, b.offset, 10, [7u8].as_ptr(), 1), 0);
            assert_eq!(bytes_of(&b), &[7; 10]);
            assert_eq!(memfill(&b, b.offset + 1, 7, [1u8, 2, 3].as_ptr(), 3), 0);
            assert_eq!(bytes_of(&b), &[7, 1, 2, 3, 1, 2, 3, 1, 7, 7]);
            assert_eq!(memfill(&b, b.offset, -1, [1u8].as_ptr(), 1), 301);
            assert_eq!(memfill(&b, b.offset, 1, [1u8].as_ptr(), 0), 301);
            assert_eq!(memfill(&b, b.offset, 11, [1u8].as_ptr(), 1), 300);
            assert_eq!(memfill(&MemBlock::default(), 0, 1, [1u8].as_ptr(), 1), 309);
            let mut stale = b;
            stale.lock_id -= 1;
            assert_eq!(memfill(&stale, b.offset, 1, [1u8].as_ptr(), 1), 308);
            memfill_nochecks(b.offset, 3, [9u8, 8].as_ptr(), 0);
            assert_eq!(&bytes_of(&b)[..3], &[9, 9, 9]);
        }
    }

    #[test]
    fn memcopy_moves_and_reports_each_error() {
        let (s, _) = memnew(8, false);
        let (d, _) = memnew(4, false);
        unsafe {
            memfill(&s, s.offset, 8, [1u8, 2, 3, 4, 5, 6, 7, 8].as_ptr(), 8);
            assert_eq!(memcopy(&s, s.offset + 2, 4, &d, d.offset), 0);
            assert_eq!(bytes_of(&d), &[3, 4, 5, 6]);
            assert_eq!(memcopy(&s, s.offset, 6, &s, s.offset + 2), 0); // overlap = memmove
            assert_eq!(bytes_of(&s), &[1, 2, 1, 2, 3, 4, 5, 6]);
            let null = MemBlock::default();
            assert_eq!(memcopy(&null, 0, 1, &null, 0), 312);
            assert_eq!(memcopy(&null, 0, 1, &d, d.offset), 310);
            assert_eq!(memcopy(&s, s.offset, 1, &null, 0), 311);
            let (mut ss, mut dd) = (s, d);
            ss.lock_id += 1;
            dd.lock_id += 1;
            assert_eq!(memcopy(&ss, s.offset, 1, &dd, d.offset), 313);
            assert_eq!(memcopy(&ss, s.offset, 1, &d, d.offset), 305);
            assert_eq!(memcopy(&s, s.offset, 1, &dd, d.offset), 306);
            assert_eq!(memcopy(&s, s.offset, -1, &d, d.offset), 301);
            assert_eq!(memcopy(&s, s.offset + 7, 2, &d, d.offset + 3), 304);
            assert_eq!(memcopy(&s, s.offset + 7, 2, &d, d.offset), 302);
            assert_eq!(memcopy(&s, s.offset, 5, &d, d.offset), 303);
        }
    }

    #[test]
    fn mem_at_offset_and_mem() {
        let (b, e) = mem_at_offset(1234, 10, false);
        assert_eq!((b.offset, b.size, b.type_, b.elementsize, b.image, e), (1234, 10, MEMNEW_TYPE, 1, -1, 0));
        let (b, e) = mem_at_offset(1234, -1, false);
        assert_eq!((b.offset, b.size, b.type_, e), (0, 0, 0, 301));
        let lock = new_mem_lock();
        let m = unsafe { mem(55, 4, 1 + 128, 4, lock) };
        assert_eq!((m.offset, m.size, m.type_, m.elementsize, m.lock_offset), (55, 4, 129, 4, lock as isize));
        assert_eq!(m.lock_id as u64, unsafe { (*lock).id });
    }
}
