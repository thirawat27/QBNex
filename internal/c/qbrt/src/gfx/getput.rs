//! Graphics `GET (x1,y1)-(x2,y2), array` and `PUT (x,y), array[, op]`
//! (`sub_graphics_get`, `sub_graphics_put`) with the QBasic array layouts:
//! 1 bit (SCREEN 2/11), 2 bit (SCREEN 1/10), 4 planes (SCREEN 7/8/9/12),
//! 8 bit (SCREEN 13/256) and 32-bit pixels, after a 4-byte width/height header.

use super::{map_point_f2l, ImgStruct, ERR_ILLEGAL_FUNCTION_CALL};

/// `GET [STEP](x1,y1)-[STEP](x2,y2), array`; passed: 1 STEP 1, 2 STEP 2,
/// 4 out-of-range pixels allowed (read as `mask`). `data`/`len` is the array
/// memory. Returns a QBasic error code.
///
/// # Safety
/// `im` must be a valid graphics surface; `data` writable for `len` bytes.
pub unsafe fn graphics_get(
    im: &mut ImgStruct,
    mut x1f: f32,
    mut y1f: f32,
    mut x2f: f32,
    mut y2f: f32,
    data: *mut u8,
    len: i32,
    mut mask: u32,
    passed: i32,
) -> i32 {
    const ERR: i32 = ERR_ILLEGAL_FUNCTION_CALL;
    if im.text != 0 {
        return ERR;
    }
    if passed & 1 != 0 {
        x1f += im.x;
        y1f += im.y;
    }
    im.x = x1f;
    im.y = y1f;
    if passed & 2 != 0 {
        x2f += im.x;
        y2f += im.y;
    }
    im.x = x2f;
    im.y = y2f;
    let (mut x1, mut y1) = map_point_f2l(im, x1f, y1f);
    let (mut x2, mut y2) = map_point_f2l(im, x2f, y2f);
    if x2 < x1 {
        core::mem::swap(&mut x1, &mut x2);
    }
    if y2 < y1 {
        core::mem::swap(&mut y1, &mut y2);
    }
    let (sx, sy) = (i32::from(im.width), i32::from(im.height));
    if passed & 4 == 0 && (x1 < 0 || y1 < 0 || x2 >= sx || y2 >= sy) {
        return ERR;
    }
    let (w, h) = (x2.wrapping_sub(x1).wrapping_add(1), y2.wrapping_sub(y1).wrapping_add(1));
    let inside = |x: i32, y: i32| x >= 0 && y >= 0 && x < sx && y < sy;
    let px8 = |x: i32, y: i32| -> u32 {
        // SAFETY: (x, y) checked by `inside`.
        unsafe { u32::from(*im.offset.add((y * sx + x) as usize)) }
    };
    // SAFETY (writes below): sizes are checked against `len` first, as in legacy.
    let dims = data as *mut u16;
    let mut cp = unsafe { data.add(4) };
    match im.bits_per_pixel {
        1 => {
            mask &= 1;
            let z = (w + 7) >> 3;
            if z.wrapping_mul(h).wrapping_add(4) > len {
                return ERR;
            }
            unsafe {
                *dims = w as u16;
                *dims.add(1) = h as u16;
            }
            for y in y1..=y2 {
                let mut z2 = 128u32;
                let mut col2 = 0u32;
                for x in x1..=x2 {
                    let col = if inside(x, y) { px8(x, y) } else { mask };
                    if col != 0 {
                        col2 |= z2;
                    }
                    z2 >>= 1;
                    if z2 == 0 {
                        z2 = 128;
                        unsafe {
                            *cp = col2 as u8;
                            cp = cp.add(1);
                        }
                        col2 = 0;
                    }
                }
                if z2 != 128 {
                    unsafe {
                        *cp = col2 as u8;
                        cp = cp.add(1);
                    }
                }
            }
        }
        2 => {
            mask &= 3;
            // D-11: legacy checked (w+7)>>3 bytes per row but writes (2w+7)>>3,
            // overrunning arrays sized by the check.
            let z = (w.wrapping_mul(2).wrapping_add(7)) >> 3;
            if z.wrapping_mul(h).wrapping_add(4) > len {
                return ERR;
            }
            unsafe {
                *dims = (w * 2) as u16;
                *dims.add(1) = h as u16;
            }
            for y in y1..=y2 {
                let mut byte = 0u32;
                let mut x3 = 0i32;
                for x in x1..=x2 {
                    let col = if inside(x, y) { px8(x, y) } else { mask };
                    byte = (byte << 2) | col;
                    if x3 & 3 == 3 {
                        unsafe {
                            *cp = byte as u8;
                            cp = cp.add(1);
                        }
                        byte = 0;
                    }
                    x3 += 1;
                }
                if x3 & 3 != 0 {
                    // D-11: the pending pixels, left-aligned as PUT reads them
                    // (legacy wrote a stale static `col2` here)
                    unsafe {
                        *cp = (byte << (2 * (4 - (x3 & 3)))) as u8;
                        cp = cp.add(1);
                    }
                }
            }
        }
        4 => {
            mask &= 15;
            let z = (w + 7) >> 3;
            if z.wrapping_mul(4).wrapping_mul(h).wrapping_add(4) > len {
                return ERR;
            }
            unsafe {
                *dims = w as u16;
                *dims.add(1) = h as u16;
            }
            let zu = z as usize;
            for (y3, y) in (y1..=y2).enumerate() {
                let base = y3 * zu * 4;
                let mut k = 0usize;
                let mut z2 = 128u32;
                let mut c = [0u32; 4];
                for x in x1..=x2 {
                    let col = if inside(x, y) { px8(x, y) } else { mask };
                    for (p, cv) in c.iter_mut().enumerate() {
                        if col & (1 << p) != 0 {
                            *cv |= z2;
                        }
                    }
                    z2 >>= 1;
                    if z2 == 0 {
                        z2 = 128;
                        for (p, cv) in c.iter_mut().enumerate() {
                            unsafe { *cp.add(base + zu * p + k) = *cv as u8 };
                            *cv = 0;
                        }
                        k += 1;
                    }
                }
                if z2 != 128 {
                    for (p, cv) in c.iter().enumerate() {
                        unsafe { *cp.add(base + zu * p + k) = *cv as u8 };
                    }
                }
            }
        }
        8 => {
            mask &= 255;
            if w.wrapping_mul(h).wrapping_add(4) > len {
                return ERR;
            }
            unsafe {
                *dims = (w * 8) as u16;
                *dims.add(1) = h as u16;
            }
            for y in y1..=y2 {
                for x in x1..=x2 {
                    let col = if inside(x, y) { px8(x, y) } else { mask };
                    unsafe {
                        *cp = col as u8;
                        cp = cp.add(1);
                    }
                }
            }
        }
        32 => {
            if w.wrapping_mul(h).wrapping_mul(4).wrapping_add(4) > len {
                return ERR;
            }
            unsafe {
                *dims = w as u16;
                *dims.add(1) = h as u16;
            }
            let mut lp = cp as *mut u32;
            for y in y1..=y2 {
                for x in x1..=x2 {
                    let col = if inside(x, y) {
                        unsafe { *im.offset32().add((y * sx + x) as usize) }
                    } else {
                        mask
                    };
                    unsafe {
                        lp.write_unaligned(col);
                        lp = lp.add(1);
                    }
                }
            }
        }
        _ => {}
    }
    0
}

/// `PUT [STEP](x,y), array[, XOR|PSET|PRESET|AND|OR]`; option 0 XOR (default),
/// 1 PSET, 2 PRESET, 3 AND, 4 OR, 5 XOR. passed: 1 STEP, 2 clip allowed,
/// 4 `_CLIP` mask color given (pixels equal to `mask` are skipped).
///
/// # Safety
/// `im` must be a valid graphics surface; `data` readable for `len` bytes.
pub unsafe fn graphics_put(
    im: &mut ImgStruct,
    mut x1f: f32,
    mut y1f: f32,
    data: *const u8,
    len: i32,
    option: i32,
    mut mask: u32,
    mut passed: i32,
) -> i32 {
    const ERR: i32 = ERR_ILLEGAL_FUNCTION_CALL;
    let step = passed & 1 != 0;
    let clip = passed & 2 != 0;
    passed &= !3;
    if im.text != 0 {
        return ERR;
    }
    if step {
        x1f += im.x;
        y1f += im.y;
        im.x = x1f;
        im.y = y1f;
    }
    let (x1, y1) = map_point_f2l(im, x1f, y1f);
    let (sx, sy) = (i32::from(im.width), i32::from(im.height));
    let bits = im.bits_per_pixel;
    if len < 4 {
        return ERR;
    }
    // SAFETY: len >= 4.
    let (mut w, h) = unsafe { (i32::from((data as *const u16).read_unaligned()), i32::from((data as *const u16).add(1).read_unaligned())) };
    let mut z = w;
    if bits == 2 {
        if w & 1 != 0 {
            return ERR;
        }
        w >>= 1;
    }
    if bits == 8 {
        if w & 7 != 0 {
            return ERR;
        }
        w >>= 3;
    }
    let (x2, y2) = (x1.wrapping_add(w).wrapping_sub(1), y1.wrapping_add(h).wrapping_sub(1));
    if !clip && (x1 < 0 || y1 < 0 || x2 >= sx || y2 >= sy) {
        return ERR;
    }
    if bits == 32 {
        z *= 32;
    }
    z = (z + 7) >> 3;
    let mut bytes = h * z;
    if bits == 4 {
        bytes *= 4;
    }
    if bytes + 4 > len {
        return ERR;
    }
    let pixelmask = im.mask;
    mask &= match bits {
        1 => 1,
        2 => 3,
        4 => 15,
        8 => 255,
        _ => 0xFFFF_FFFF,
    };
    let inside = |x: i32, y: i32| x >= 0 && y >= 0 && x < sx && y < sy;
    let apply = |dst: u32, col: u32| -> u32 {
        match option {
            0 | 5 => dst ^ col,
            1 => col,
            2 => !col & pixelmask,
            3 => dst & col,
            4 => dst | col,
            _ => dst,
        }
    };
    // SAFETY (all accesses): the size check above bounds every read of the
    // array; destination writes are checked by `inside`.
    let cp = unsafe { data.add(4) };
    let rd = |k: usize| -> u32 { u32::from(unsafe { *cp.add(k) }) };
    let plot8 = |x: i32, y: i32, col: u32| {
        if passed == 0 || col != mask {
            let p = unsafe { im.offset.add((y * sx + x) as usize) };
            unsafe { *p = apply(u32::from(*p), col) as u8 };
        }
    };
    match bits {
        1 | 2 => {
            let (per, b) = if bits == 1 { (8i32, 1u32) } else { (4, 2) };
            let mut k = 0usize;
            for y in y1..=y2 {
                let mut x3 = 0i32;
                let mut col2 = 0u32;
                for x in x1..=x2 {
                    if x3 == 0 {
                        x3 = per;
                        col2 = rd(k);
                        k += 1;
                    }
                    x3 -= 1;
                    if inside(x, y) {
                        plot8(x, y, (col2 >> (x3 as u32 * b)) & ((1 << b) - 1));
                    }
                }
            }
        }
        4 => {
            let zu = z as usize;
            for (y3, y) in (y1..=y2).enumerate() {
                let base = y3 * zu * 4;
                let mut k = 0usize;
                let mut x3 = 0i32;
                let mut c = [0u32; 4];
                for x in x1..=x2 {
                    if x3 == 0 {
                        x3 = 8;
                        for (p, cv) in c.iter_mut().enumerate() {
                            *cv = rd(base + zu * p + k) << p;
                        }
                        k += 1;
                    }
                    x3 -= 1;
                    if inside(x, y) {
                        let col = (0..4).fold(0, |acc, p| acc | ((c[p] >> x3) & (1 << p)));
                        plot8(x, y, col);
                    }
                }
            }
        }
        8 => {
            let mut k = 0usize;
            for y in y1..=y2 {
                for x in x1..=x2 {
                    if inside(x, y) {
                        plot8(x, y, rd(k));
                    }
                    k += 1;
                }
            }
        }
        32 => {
            let lp = cp as *const u32;
            let mut k = 0usize;
            for y in y1..=y2 {
                for x in x1..=x2 {
                    if inside(x, y) {
                        let col = unsafe { lp.add(k).read_unaligned() };
                        if passed == 0 || col != mask {
                            let p = unsafe { im.offset32().add((y * sx + x) as usize) };
                            unsafe { *p = apply(*p, col) };
                        }
                    }
                    k += 1;
                }
            }
        }
        _ => {}
    }
    0
}

/// `PCOPY` on resolved pages: same size and depth required; a palette page
/// cannot receive a page with more colors.
///
/// # Safety
/// Both surfaces must be valid.
pub unsafe fn pcopy(s: &ImgStruct, d: &ImgStruct) -> i32 {
    const ERR: i32 = ERR_ILLEGAL_FUNCTION_CALL;
    if core::ptr::eq(s, d) {
        return 0;
    }
    if s.bytes_per_pixel != d.bytes_per_pixel || s.height != d.height || s.width != d.width {
        return ERR;
    }
    if s.bytes_per_pixel == 1 && d.mask < s.mask {
        return ERR;
    }
    let n = usize::from(d.width) * usize::from(d.height) * usize::from(d.bytes_per_pixel);
    // SAFETY: equal sizes; distinct surfaces own distinct buffers.
    unsafe { core::ptr::copy(s.offset, d.offset, n) };
    0
}

// --- C ABI ---------------------------------------------------------------------

/// # Safety
/// See [`graphics_get`] (`im` = read_page).
#[no_mangle]
pub unsafe extern "C" fn qbrt_gfx_get(
    im: *mut ImgStruct,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    data: *mut u8,
    len: i32,
    mask: u32,
    passed: i32,
) -> i32 {
    // SAFETY: guaranteed by the caller.
    unsafe { graphics_get(&mut *im, x1, y1, x2, y2, data, len, mask, passed) }
}

/// # Safety
/// See [`graphics_put`] (`im` = write_page).
#[no_mangle]
pub unsafe extern "C" fn qbrt_gfx_put(
    im: *mut ImgStruct,
    x1: f32,
    y1: f32,
    data: *const u8,
    len: i32,
    option: i32,
    mask: u32,
    passed: i32,
) -> i32 {
    // SAFETY: guaranteed by the caller.
    unsafe { graphics_put(&mut *im, x1, y1, data, len, option, mask, passed) }
}

/// # Safety
/// See [`pcopy`].
#[no_mangle]
pub unsafe extern "C" fn qbrt_gfx_pcopy(s: *const ImgStruct, d: *const ImgStruct) -> i32 {
    // SAFETY: guaranteed by the caller.
    unsafe { pcopy(&*s, &*d) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::testutil::TestImage;

    #[test]
    fn get_put_roundtrip_all_depths() {
        for mode in [2u16, 1, 12, 13, 32] {
            let mut a = TestImage::new(20, 10, mode);
            for i in 0..(20 * 10) {
                let c = (i * 7 + 3) as u32 & a.img.mask;
                unsafe { crate::gfx::blend::pset(&a.img, i % 20, i / 20, c | if mode == 32 { 0xFF00_0000 } else { 0 }) };
            }
            let mut buf = vec![0u8; 2000];
            assert_eq!(unsafe { graphics_get(&mut a.img, 2.0, 1.0, 13.0, 7.0, buf.as_mut_ptr(), 2000, 0, 0) }, 0, "mode {mode}");
            let mut b = TestImage::new(20, 10, mode);
            assert_eq!(unsafe { graphics_put(&mut b.img, 3.0, 2.0, buf.as_ptr(), 2000, 1, 0, 0) }, 0);
            for y in 0..7 {
                for x in 0..12 {
                    let (p, q) = if mode == 32 { (a.p32(2 + x, 1 + y), b.p32(3 + x, 2 + y)) } else { (a.p8(2 + x, 1 + y) as u32, b.p8(3 + x, 2 + y) as u32) };
                    assert_eq!(p, q, "mode {mode} at {x},{y}");
                }
            }
            // XOR twice restores
            unsafe {
                graphics_put(&mut b.img, 3.0, 2.0, buf.as_ptr(), 2000, 0, 0, 0);
                graphics_put(&mut b.img, 3.0, 2.0, buf.as_ptr(), 2000, 0, 0, 0);
            }
            assert_eq!(b.p8(3, 2) as u32, if mode == 32 { b.p8(3, 2) as u32 } else { a.p8(2, 1) as u32 });
        }
    }

    // D-11: 2-bit GET checks ceil(w/4) bytes per row (it writes that many)
    #[test]
    fn get_2bit_size_check_covers_written_bytes() {
        let mut a = TestImage::new(16, 1, 1);
        let mut buf = vec![0xEEu8; 16];
        // 16 pixels = 4 bytes + 4 header: a 6-byte array is too small
        assert_eq!(unsafe { graphics_get(&mut a.img, 0.0, 0.0, 15.0, 0.0, buf.as_mut_ptr(), 6, 0, 0) }, 5);
        assert!(buf[6..].iter().all(|&b| b == 0xEE), "wrote past the array");
        assert_eq!(unsafe { graphics_get(&mut a.img, 0.0, 0.0, 15.0, 0.0, buf.as_mut_ptr(), 8, 0, 0) }, 0);
        assert!(buf[8..].iter().all(|&b| b == 0xEE));
    }

    // D-11: 2-bit GET stores the last partial byte of a row left-aligned, so
    // PUT gets back every pixel of a width that is not a multiple of 4
    #[test]
    fn get_put_2bit_odd_width_roundtrip() {
        let mut a = TestImage::new(20, 4, 1);
        for i in 0..80 {
            unsafe { crate::gfx::blend::pset(&a.img, i % 20, i / 20, (i * 5 + 1) as u32 & 3) };
        }
        let mut buf = vec![0u8; 64];
        // GET a 1-bit layout first so a stale value is around
        let mut m2 = TestImage::new(16, 4, 2);
        unsafe { *m2.img.offset = 1 };
        unsafe { graphics_get(&mut m2.img, 0.0, 0.0, 9.0, 0.0, buf.as_mut_ptr(), 64, 0, 0) };
        for w in 1..=7 {
            buf.fill(0);
            assert_eq!(unsafe { graphics_get(&mut a.img, 1.0, 0.0, w as f32, 3.0, buf.as_mut_ptr(), 64, 0, 0) }, 0);
            let mut b = TestImage::new(20, 4, 1);
            assert_eq!(unsafe { graphics_put(&mut b.img, 1.0, 0.0, buf.as_ptr(), 64, 1, 0, 0) }, 0);
            for y in 0..4 {
                for x in 1..=w {
                    assert_eq!(a.p8(x, y), b.p8(x, y), "width {w} at {x},{y}");
                }
            }
        }
    }

    #[test]
    fn errors() {
        let mut a = TestImage::new(8, 8, 13);
        let mut buf = vec![0u8; 8];
        assert_eq!(unsafe { graphics_get(&mut a.img, 0.0, 0.0, 9.0, 1.0, buf.as_mut_ptr(), 8, 0, 0) }, 5);
        assert_eq!(unsafe { graphics_get(&mut a.img, 0.0, 0.0, 3.0, 3.0, buf.as_mut_ptr(), 8, 0, 0) }, 5); // too small
        buf[0] = 3 * 8;
        buf[2] = 1;
        assert_eq!(unsafe { graphics_put(&mut a.img, 6.0, 0.0, buf.as_ptr(), 8, 1, 0, 0) }, 5); // off right edge
        assert_eq!(unsafe { graphics_put(&mut a.img, 6.0, 0.0, buf.as_ptr(), 8, 1, 0, 2) }, 0); // clipped
    }

    #[test]
    fn pcopy_checks() {
        let a = TestImage::new(4, 4, 13);
        let b = TestImage::new(4, 4, 13);
        unsafe { *a.img.offset = 9 };
        assert_eq!(unsafe { pcopy(&a.img, &b.img) }, 0);
        assert_eq!(b.p8(0, 0), 9);
        let c = TestImage::new(4, 4, 12);
        assert_eq!(unsafe { pcopy(&a.img, &c.img) }, 5);
        let d = TestImage::new(4, 5, 13);
        assert_eq!(unsafe { pcopy(&a.img, &d.img) }, 5);
    }
}
