//! The image registry: `img`, `nimg`, `nextimg`, the free list and the pixel
//! and palette buffers owned by surfaces (QB64's `newimg`, `freeimg`,
//! `grow_img_registry`, `imgframe`, `imgnew`, `imgrevert`).
//!
//! `img`, `nimg` and `nextimg` keep QB64's names; callers index `img[i]`
//! directly. The array is reallocated when the registry grows, so callers
//! must refresh cached page pointers afterwards (compare `nimg` before and
//! after each call).
//!
//! The registry is only touched from the program thread (as in legacy; the
//! display thread reads `img`/pixels without locking, unchanged).

#![allow(non_upper_case_globals)]

use super::image::{imgrevert, setup_new_image};
use super::{ImgStruct, ERR_ILLEGAL_FUNCTION_CALL, IMG_FREEMEM, IMG_FREEPAL};
use std::cell::UnsafeCell;

/// Initial number of slots (legacy `IMG_BUFFERSIZE`).
pub const IMG_BUFFERSIZE: i32 = 4096;
/// QBasic "Out of memory".
pub const ERR_OUT_OF_MEMORY: i32 = 502;
/// Largest surface side: `ImgStruct.width`/`height` are 16-bit.
pub const IMG_MAX_SIDE: i32 = 0xFFFF;

/// Writes `e` through `err` when it is not null.
///
/// # Safety
/// `err` must be writable or null.
unsafe fn set_err(err: *mut i32, e: i32) {
    if !err.is_null() {
        // SAFETY: guaranteed by the caller.
        unsafe { *err = e };
    }
}

/// `img_struct *img`: base of the registry array.
pub static mut img: *mut ImgStruct = core::ptr::null_mut();
/// `int32 nimg`: capacity of the registry.
pub static mut nimg: i32 = 0;
/// `int32 nextimg`: first never-used index.
pub static mut nextimg: i32 = 0;

struct Storage {
    structs: Vec<ImgStruct>,
    pixels: Vec<Vec<u8>>,
    palettes: Vec<Vec<u32>>,
    /// Freed indexes, reused last-in first-out (legacy `fimg`).
    free: Vec<u32>,
}

struct Global(UnsafeCell<Option<Storage>>);
// SAFETY: single-threaded use by contract (see module docs).
unsafe impl Sync for Global {}
static STORAGE: Global = Global(UnsafeCell::new(None));

fn storage() -> &'static mut Storage {
    // SAFETY: single-threaded use by contract; never re-entered while borrowed.
    let s = unsafe { &mut *STORAGE.0.get() };
    s.get_or_insert_with(|| {
        let n = IMG_BUFFERSIZE as usize;
        let mut st = Storage {
            structs: vec![ImgStruct::default(); n],
            pixels: Vec::from_fn(n, |_| Vec::new()),
            palettes: Vec::from_fn(n, |_| Vec::new()),
            free: Vec::new(),
        };
        // SAFETY: publishing the registry base for direct indexing.
        unsafe {
            img = st.structs.as_mut_ptr();
            nimg = IMG_BUFFERSIZE;
        }
        st
    })
}

#[inline]
fn slot(i: i32) -> &'static mut ImgStruct {
    let st = storage();
    &mut st.structs[i as usize]
}

/// Growth policy of `next_img_capacity`.
pub fn next_capacity(current: i32, required: i32) -> i32 {
    let mut capacity = current.max(IMG_BUFFERSIZE);
    while capacity < required {
        if capacity > 1_073_741_823 {
            capacity = required;
            break;
        }
        capacity *= 2;
    }
    capacity
}

/// `qb_img_init`: allocates the initial registry (idempotent).
pub fn qb_img_init() {
    storage();
}

/// `grow_img_registry`: 0 on success, 502 when out of memory.
pub fn qb_img_grow(required: i32) -> i32 {
    let st = storage();
    let current = st.structs.len() as i32;
    if required <= current {
        return 0;
    }
    let cap = next_capacity(current, required) as usize;
    let extra = cap - st.structs.len();
    if st.structs.try_reserve_exact(extra).is_err()
        || st.pixels.try_reserve_exact(extra).is_err()
        || st.palettes.try_reserve_exact(extra).is_err()
    {
        return ERR_OUT_OF_MEMORY;
    }
    st.structs.resize(cap, ImgStruct::default());
    st.pixels.resize_with(cap, Vec::new);
    st.palettes.resize_with(cap, Vec::new);
    // SAFETY: publishing the (possibly moved) registry base for direct indexing.
    unsafe {
        img = st.structs.as_mut_ptr();
        nimg = cap as i32;
    }
    0
}

/// `newimg`: index of a free slot (marked valid), 0 with `*err = 502` when
/// the registry cannot grow.
///
/// # Safety
/// `err` must be writable or null.
pub unsafe fn qb_img_new(err: *mut i32) -> u32 {
    let st = storage();
    let i = if let Some(i) = st.free.pop() {
        i
    } else {
        // SAFETY: single-threaded globals.
        let next = unsafe { nextimg };
        if next >= st.structs.len() as i32 {
            let e = qb_img_grow(next + 1);
            if e != 0 {
                if !err.is_null() {
                    // SAFETY: guaranteed by the caller.
                    unsafe { *err = e };
                }
                return 0;
            }
        }
        // SAFETY: single-threaded globals.
        unsafe { nextimg += 1 };
        next as u32
    };
    slot(i as i32).valid = 1;
    i
}

/// Releases a slot: frees its buffers, zeroes the struct and recycles the
/// index. The caller has already checked `valid` and handled its side
/// tables (memory lock, text cells, page list).
pub fn qb_img_free(i: u32) {
    let st = storage();
    let k = i as usize;
    st.pixels[k] = Vec::new();
    st.palettes[k] = Vec::new();
    st.structs[k] = ImgStruct::default();
    st.free.push(i);
}

/// Gives slot `i` a zero-filled pixel buffer of `bytes` bytes. When the
/// allocation fails the slot is left without pixels (`offset` null).
pub fn qb_img_alloc_pixels(i: i32, bytes: isize) {
    let st = storage();
    let k = i as usize;
    let n = bytes.max(0) as usize;
    // drop the old buffer first; a failed reservation must not abort
    st.pixels[k] = Vec::new();
    let mut v = Vec::<u8>::new();
    if v.try_reserve_exact(n).is_ok() {
        v.resize(n, 0u8);
    }
    st.pixels[k] = v;
    st.structs[k].offset = if st.pixels[k].is_empty() { core::ptr::null_mut() } else { st.pixels[k].as_mut_ptr() };
}

/// Gives slot `i` a zero-filled 256-entry palette.
pub fn qb_img_alloc_palette(i: i32) {
    let st = storage();
    let k = i as usize;
    st.palettes[k] = vec![0u32; 256];
    st.structs[k].pal = st.palettes[k].as_mut_ptr();
}

/// Frees slot `i`'s pixel buffer and clears `offset`.
pub fn qb_img_release_pixels(i: i32) {
    let st = storage();
    st.pixels[i as usize] = Vec::new();
    st.structs[i as usize].offset = core::ptr::null_mut();
}

/// Frees slot `i`'s palette and clears `pal`.
pub fn qb_img_release_palette(i: i32) {
    let st = storage();
    st.palettes[i as usize] = Vec::new();
    st.structs[i as usize].pal = core::ptr::null_mut();
}

/// Points slot `i` at pixels it does not own (video memory, other pages).
pub fn qb_img_set_external_offset(i: i32, offset: *mut u8) {
    let st = storage();
    st.pixels[i as usize] = Vec::new();
    st.structs[i as usize].offset = offset;
}

/// Points slot `i` at a palette it does not own (shared with page 0).
pub fn qb_img_set_external_palette(i: i32, pal: *mut u32) {
    let st = storage();
    st.palettes[i as usize] = Vec::new();
    st.structs[i as usize].pal = pal;
}

/// Frees what IMG_FREEMEM / IMG_FREEPAL say the slot owns.
pub fn qb_img_release_owned(i: i32) {
    let flags = slot(i).flags;
    if flags & IMG_FREEMEM != 0 {
        qb_img_release_pixels(i);
    }
    if flags & IMG_FREEPAL != 0 {
        qb_img_release_palette(i);
    }
}

/// `imgframe(o, x, y, bpp)`: new slot set up for mode `bpp` over pixels `o`;
/// 0 when the size or mode is invalid (or the registry cannot grow: `*err`).
/// A side over [`IMG_MAX_SIDE`] (`*err` = 5) or a surface over `i32::MAX`
/// bytes (`*err` = 502) is rejected before a slot is taken, so the 16-bit
/// sizes never truncate and `y * width + x` pixel offsets fit in `i32`.
///
/// # Safety
/// `o` must stay valid while the slot uses it; `err` writable or null.
pub unsafe fn qb_imgframe(o: *mut u8, x: i32, y: i32, bpp: i32, err: *mut i32) -> i32 {
    if x <= 0 || y <= 0 {
        return 0;
    }
    if x > IMG_MAX_SIDE || y > IMG_MAX_SIDE {
        // SAFETY: guaranteed by the caller.
        unsafe { set_err(err, ERR_ILLEGAL_FUNCTION_CALL) };
        return 0;
    }
    let bytes_per_pixel: i64 = match bpp {
        32 => 4,
        0 => 2,
        _ => 1,
    };
    if i64::from(x) * i64::from(y) * bytes_per_pixel > i64::from(i32::MAX) {
        // SAFETY: guaranteed by the caller.
        unsafe { set_err(err, ERR_OUT_OF_MEMORY) };
        return 0;
    }
    // SAFETY: guaranteed by the caller.
    let i = unsafe { qb_img_new(err) } as i32;
    if i == 0 {
        return 0;
    }
    qb_img_set_external_offset(i, o);
    let valid_mode = matches!(bpp, 0 | 1 | 2 | 7..=13 | 256 | 32);
    if valid_mode && bpp != 32 {
        qb_img_alloc_palette(i);
        slot(i).flags |= IMG_FREEPAL;
    }
    // An unknown mode returns 0 after the size and defaults were stored; the
    // slot stays allocated (marked valid), as in legacy.
    if !setup_new_image(slot(i), x as u16, y as u16, bpp as u16) {
        return 0;
    }
    i
}

/// `imgnew(x, y, bpp)`: `imgframe` with an owned, cleared buffer (text
/// surfaces are filled with spaces in attribute 7).
///
/// # Safety
/// `err` writable or null.
pub unsafe fn qb_imgnew(x: i32, y: i32, bpp: i32, err: *mut i32) -> i32 {
    // SAFETY: guaranteed by the caller.
    let i = unsafe { qb_imgframe(core::ptr::null_mut(), x, y, bpp, err) };
    if i == 0 {
        return 0;
    }
    let bytes_per_pixel = slot(i).bytes_per_pixel as isize;
    qb_img_alloc_pixels(i, x as isize * y as isize * bytes_per_pixel);
    if slot(i).offset.is_null() {
        qb_img_free(i as u32);
        // SAFETY: guaranteed by the caller.
        unsafe { set_err(err, ERR_OUT_OF_MEMORY) };
        return 0;
    }
    let im = slot(i);
    if bpp == 0 {
        let n = x as usize * y as usize;
        // SAFETY: the buffer holds n 16-bit cells.
        let cells = unsafe { core::slice::from_raw_parts_mut(im.offset as *mut u16, n) };
        cells.fill(0x0720);
    }
    im.flags |= IMG_FREEMEM;
    i
}

/// `imgrevert(i)`.
///
/// # Safety
/// `i` must be a valid slot.
pub unsafe fn qb_imgrevert(i: i32) {
    // SAFETY: guaranteed by the caller.
    unsafe { imgrevert(slot(i)) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // The registry is process-global; serialise the tests that use it.
    static LOCK: Mutex<()> = Mutex::new(());

    /// Start-up as in QB64: indexes 0 and 1 are reserved (handles 0/-1).
    fn init() {
        qb_img_init();
        if unsafe { nextimg } == 0 {
            let mut e = 0;
            for _ in 0..2 {
                let x = unsafe { qb_img_new(&mut e) };
                unsafe { (*img.add(x as usize)).valid = 0 };
            }
        }
    }

    fn slot(i: u32) -> ImgStruct {
        unsafe { *img.add(i as usize) }
    }

    #[test]
    fn capacity_doubles_from_4096() {
        assert_eq!(next_capacity(4096, 4097), 8192);
        assert_eq!(next_capacity(4096, 20000), 32768);
        assert_eq!(next_capacity(100, 10), 4096);
        assert_eq!(next_capacity(1 << 30, (1 << 30) + 5), (1 << 30) + 5);
    }

    #[test]
    fn new_free_reuse_and_grow() {
        let _g = LOCK.lock().unwrap();
        init();
        assert!(unsafe { nimg } >= IMG_BUFFERSIZE);
        let mut e = 0;
        let a = unsafe { qb_img_new(&mut e) };
        let b = unsafe { qb_img_new(&mut e) };
        assert_eq!(e, 0);
        assert_ne!(a, b);
        assert_eq!(slot(a).valid, 1);
        qb_img_alloc_pixels(a as i32, 16);
        assert!(!slot(a).offset.is_null());
        qb_img_free(a);
        assert_eq!(slot(a).valid, 0);
        assert!(slot(a).offset.is_null());
        // most recently freed index is reused first
        assert_eq!(unsafe { qb_img_new(&mut e) }, a);
        // fill the registry: it grows and keeps the old contents
        let before = unsafe { nimg };
        unsafe { (*img.add(b as usize)).width = 77 };
        while unsafe { nextimg } < before {
            unsafe { qb_img_new(&mut e) };
        }
        let c = unsafe { qb_img_new(&mut e) };
        assert_eq!(c as i32, before);
        assert_eq!(unsafe { nimg }, before * 2);
        assert_eq!(slot(b).width, 77);
    }

    #[test]
    fn imgnew_graphics_text_and_errors() {
        let _g = LOCK.lock().unwrap();
        init();
        let mut e = 0;
        let i = unsafe { qb_imgnew(10, 5, 13, &mut e) };
        assert!(i > 0);
        let s = slot(i as u32);
        assert_eq!((s.width, s.height, s.compatible_mode, s.flags), (10, 5, 13, 1 | 4));
        assert!(!s.pal.is_null());
        assert_eq!(unsafe { *s.pal.add(15) }, crate::gfx::data::palette_256[15]);
        let t = unsafe { qb_imgnew(4, 2, 0, &mut e) };
        let s = slot(t as u32);
        let cells = unsafe { core::slice::from_raw_parts(s.offset as *const u16, 8) };
        assert!(cells.iter().all(|c| *c == 0x0720));
        let p = unsafe { qb_imgnew(3, 3, 32, &mut e) };
        assert!(slot(p as u32).pal.is_null());
        assert_eq!(slot(p as u32).flags, 4);
        assert_eq!(unsafe { qb_imgnew(0, 3, 32, &mut e) }, 0);
        assert_eq!(unsafe { qb_imgnew(3, 3, 5, &mut e) }, 0);
    }

    #[test]
    fn oversized_images_are_rejected_without_taking_a_slot() {
        let _g = LOCK.lock().unwrap();
        init();
        let next = unsafe { nextimg };
        let free = storage().free.len();
        // a side over 16 bits would truncate the u16 width/height
        let mut e = 0;
        assert_eq!(unsafe { qb_imgnew(65536, 1, 32, &mut e) }, 0);
        assert_eq!(e, ERR_ILLEGAL_FUNCTION_CALL);
        let mut e = 0;
        assert_eq!(unsafe { qb_imgnew(1, 70000, 13, &mut e) }, 0);
        assert_eq!(e, ERR_ILLEGAL_FUNCTION_CALL);
        // over i32::MAX bytes would overflow the i32 pixel offsets
        let mut e = 0;
        assert_eq!(unsafe { qb_imgnew(65535, 65535, 32, &mut e) }, 0);
        assert_eq!(e, ERR_OUT_OF_MEMORY);
        let mut e = 0;
        assert_eq!(unsafe { qb_imgframe(core::ptr::null_mut(), 65535, 32769, 0, &mut e) }, 0);
        assert_eq!(e, ERR_OUT_OF_MEMORY);
        assert_eq!(unsafe { qb_imgnew(65536, 1, 32, core::ptr::null_mut()) }, 0);
        assert_eq!((unsafe { nextimg }, storage().free.len()), (next, free));
        // the largest side is still accepted
        let mut e = 0;
        let i = unsafe { qb_imgnew(65535, 1, 13, &mut e) };
        assert!(i > 0);
        assert_eq!((e, slot(i as u32).width), (0, 65535));
    }

    #[test]
    fn external_buffers_are_not_freed() {
        let _g = LOCK.lock().unwrap();
        init();
        let mut mem = vec![0u8; 64];
        let mut e = 0;
        let i = unsafe { qb_imgframe(mem.as_mut_ptr(), 8, 8, 13, &mut e) };
        assert_eq!(slot(i as u32).offset, mem.as_mut_ptr());
        assert_eq!(slot(i as u32).flags, 1); // owns only its palette
        qb_img_release_owned(i);
        assert!(slot(i as u32).pal.is_null());
        assert_eq!(slot(i as u32).offset, mem.as_mut_ptr());
    }

    #[test]
    fn revert_restores_defaults_and_clears() {
        let _g = LOCK.lock().unwrap();
        init();
        let mut e = 0;
        let i = unsafe { qb_imgnew(4, 4, 12, &mut e) };
        unsafe {
            let s = &mut *img.add(i as usize);
            s.color = 3;
            *s.offset = 9;
            *s.pal.add(1) = 0;
            qb_imgrevert(i);
        }
        let s = slot(i as u32);
        assert_eq!((s.color, unsafe { *s.offset }), (15, 0));
        assert_eq!(unsafe { *s.pal.add(1) }, crate::gfx::data::palette_256[1]);
    }
}
