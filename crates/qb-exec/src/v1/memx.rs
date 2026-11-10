//! `_MEM` builtins of the v1 interpreter, routed to `qb_runtime::mem` (the
//! Rust port the C++ backend links). A `_MEM` variable is the `mem_block`
//! UDT (`MemBlock`, 56 bytes); QIR passes its address.

use super::rt::Host;
use super::{e, Stop, R, V};
use qb_runtime::mem::{self, MemBlock};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Lock shared by `_MEM(variable)` blocks (the C++ backend uses the scope's `sf_mem_lock`).
static VAR_LOCK: AtomicUsize = AtomicUsize::new(0);

fn var_lock() -> *mut mem::MemLock {
    let p = VAR_LOCK.load(Ordering::Relaxed);
    if p != 0 {
        return p as *mut mem::MemLock;
    }
    let l = mem::new_mem_lock();
    VAR_LOCK.store(l as usize, Ordering::Relaxed);
    l
}

fn blk<'b>(vm: &mut dyn Host<'_>, p: u64) -> R<&'b mut MemBlock> {
    let b = vm.bytes(p, std::mem::size_of::<MemBlock>())?;
    // SAFETY: `p` addresses a _MEM UDT, whose layout is `MemBlock` (repr(C)).
    Ok(unsafe { &mut *(b.as_mut_ptr() as *mut MemBlock) })
}

fn chk(err: i32) -> R<()> {
    if err != 0 {
        return Err(Stop::Err(err));
    }
    Ok(())
}

/// Handles `name` if it is a `_MEM` builtin; `None` otherwise.
pub(crate) fn call<'a>(vm: &mut dyn Host<'a>, name: &str, a: &[V]) -> Option<R<Option<V>>> {
    let i = |k: usize| a[k].i();
    let p = |k: usize| a[k].p();
    let r = (|| -> R<Option<V>> {
        match name {
            "_MEMNEW" => {
                let (b, err) = mem::memnew(i(1) as isize, false);
                *blk(vm, p(0))? = b;
                chk(err)?;
            }
            "_MEMAT" => {
                let (b, err) = mem::mem_at_offset(i(1) as isize, i(2) as isize, false);
                *blk(vm, p(0))? = b;
                chk(err)?;
            }
            "_MEMVAR" => {
                // SAFETY: the address is live VM storage of `size` bytes.
                let b = unsafe { mem::mem(p(1) as isize, i(2) as isize, i(3) as i32, i(4) as isize, var_lock()) };
                *blk(vm, p(0))? = b;
            }
            "_MEMEXISTS" => {
                let b = blk(vm, p(0))?;
                // SAFETY: b is a mem_block.
                return Ok(Some(V::I(unsafe { mem::memexists(b) }.into())));
            }
            "_MEMFREE" => {
                let b = blk(vm, p(0))?;
                // SAFETY: b is a mem_block.
                chk(unsafe { mem::memfree(b) })?;
            }
            "_MEMGETP" => {
                let b = blk(vm, p(0))?;
                // SAFETY: b is a mem_block; memget checks the range and the lock.
                let (q, err) = unsafe { mem::memget(b, i(1) as isize, i(2) as isize) };
                chk(err)?;
                return Ok(Some(V::P(q as u64)));
            }
            "_MEMFILLP" | "_MEMFILLS" => {
                let src: Vec<u8> = if name == "_MEMFILLS" {
                    a[3].s().to_vec()
                } else {
                    vm.bytes(p(3), i(4) as usize)?.to_vec()
                };
                let b = blk(vm, p(0))?;
                // SAFETY: memfill checks the destination range.
                chk(unsafe { mem::memfill(b, i(1) as isize, i(2) as isize, src.as_ptr(), src.len() as isize) })?;
            }
            "_MEMPUTS" => {
                let s = a[2].s().to_vec();
                let b = blk(vm, p(0))?;
                // SAFETY: memget checks the range.
                let (q, err) = unsafe { mem::memget(b, i(1) as isize, s.len() as isize) };
                chk(err)?;
                if !s.is_empty() {
                    vm.bytes(q as u64, s.len())?.copy_from_slice(&s);
                }
            }
            "_MEMCOPY" => {
                let s = *blk(vm, p(0))?;
                let d = blk(vm, p(3))?;
                // SAFETY: memcopy checks both ranges.
                chk(unsafe { mem::memcopy(&s, i(1) as isize, i(2) as isize, d, i(4) as isize) })?;
            }
            "_ARRADDR" | "_ARRBYTES" => {
                let (addr, bytes) = vm.array_extent(p(0))?;
                return Ok(Some(if name == "_ARRADDR" { V::P(addr) } else { V::I(bytes as i64) }));
            }
            _ => return e(-1),
        }
        Ok(None)
    })();
    match r {
        Err(Stop::Err(-1)) => None,
        other => Some(other),
    }
}
