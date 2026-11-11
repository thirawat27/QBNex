//! Per-image color operations: `_SETALPHA`, `_CLEARCOLOR`, `_PALETTECOLOR`,
//! `_RGB`, `_RGBA`, `_RED`/`_GREEN`/`_BLUE`/`_ALPHA` and `matchcol`.
//! Handles are resolved by the C++ shims; these functions get the surface.

use super::{ImgStruct, ERR_ILLEGAL_FUNCTION_CALL};

/// Nearest palette index to (r, g, b) by Manhattan distance (`matchcol`);
/// the first index wins ties.
///
/// # Safety
/// `im.pal` must hold at least `mask + 1` (or 16 for text) entries.
pub unsafe fn matchcol(im: &ImgStruct, r: i32, g: i32, b: i32) -> u32 {
    let n2 = if im.text != 0 { 16 } else { im.mask.wrapping_add(1) as usize };
    let mut v = 1000;
    let mut best = 0u32;
    for n in 0..n2 {
        // SAFETY: guaranteed by the caller.
        let c = unsafe { *im.pal.add(n) } as i32;
        let v2 = (b - (c & 0xFF)).abs() + (g - (c >> 8 & 0xFF)).abs() + (r - (c >> 16 & 0xFF)).abs();
        if v2 < v {
            if v2 == 0 {
                return n as u32;
            }
            v = v2;
            best = n as u32;
        }
    }
    best
}

/// `_RGB(r, g, b)` for surface `im`.
///
/// # Safety
/// As [`matchcol`].
pub unsafe fn rgb(im: &ImgStruct, r: i32, g: i32, b: i32) -> u32 {
    let (r, g, b) = (r.clamp(0, 255), g.clamp(0, 255), b.clamp(0, 255));
    if im.bytes_per_pixel == 4 {
        ((r << 16) + (g << 8) + b) as u32 | 0xFF00_0000
    } else {
        // SAFETY: guaranteed by the caller.
        unsafe { matchcol(im, r, g, b) }
    }
}

/// `_RGBA(r, g, b, a)` for surface `im`.
///
/// # Safety
/// As [`matchcol`].
pub unsafe fn rgba(im: &ImgStruct, r: i32, g: i32, b: i32, a: i32) -> u32 {
    let (r, g, b, a) = (r.clamp(0, 255), g.clamp(0, 255), b.clamp(0, 255), a.clamp(0, 255));
    if im.bytes_per_pixel == 4 {
        ((a as u32) << 24) + ((r << 16) + (g << 8) + b) as u32
    } else {
        if a == 0 && im.transparent_color != -1 {
            return im.transparent_color as u32;
        }
        // SAFETY: guaranteed by the caller.
        unsafe { matchcol(im, r, g, b) }
    }
}

/// Component selector for [`component`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(i32)]
pub enum Component {
    Red = 0,
    Green = 1,
    Blue = 2,
    Alpha = 3,
}

/// `_RED/_GREEN/_BLUE/_ALPHA(col)`: (value, error code).
///
/// # Safety
/// Palette surfaces must own a 256-entry palette.
pub unsafe fn component(im: &ImgStruct, col: u32, which: Component) -> (i32, i32) {
    if im.bytes_per_pixel == 4 {
        let v = match which {
            Component::Red => col >> 16 & 0xFF,
            Component::Green => col >> 8 & 0xFF,
            Component::Blue => col & 0xFF,
            Component::Alpha => col >> 24,
        };
        return (v as i32, 0);
    }
    if col > im.mask {
        return (0, ERR_ILLEGAL_FUNCTION_CALL);
    }
    if which == Component::Alpha {
        return (if im.transparent_color as u32 == col { 0 } else { 255 }, 0);
    }
    // SAFETY: col <= mask < 256.
    let p = unsafe { *im.pal.add(col as usize) };
    let v = match which {
        Component::Red => p >> 16 & 0xFF,
        Component::Green => p >> 8 & 0xFF,
        _ => p & 0xFF,
    };
    (v as i32, 0)
}

/// `_SETALPHA a[, c[ TO c2]]`; passed: 1 = c, 4 = range. Returns an error code.
///
/// # Safety
/// `im` must be a valid surface.
pub unsafe fn setalpha(im: &mut ImgStruct, a: i32, c: u32, c2: u32, passed: i32) -> i32 {
    if !im.pal.is_null() || !(0..=255).contains(&a) {
        return ERR_ILLEGAL_FUNCTION_CALL;
    }
    let n = usize::from(im.width) * usize::from(im.height);
    // SAFETY: a 32-bit surface owns n pixels.
    let px = unsafe { core::slice::from_raw_parts_mut(im.offset32(), n) };
    let exact = |px: &mut [u32], c: u32| {
        let alpha = (a as u32) << 24;
        for p in px.iter_mut() {
            if *p == c {
                *p = (*p & 0xFFFFFF) | alpha;
            }
        }
    };
    if passed & 4 != 0 {
        if c == c2 {
            exact(px, c);
            return 0;
        }
        let range = |shift: u32| {
            let (x, y) = ((c >> shift & 0xFF) as u8, (c2 >> shift & 0xFF) as u8);
            (x.min(y), x.max(y))
        };
        let (b, g, r, al) = (range(0), range(8), range(16), range(24));
        let inr = |v: u8, (lo, hi): (u8, u8)| v >= lo && v <= hi;
        for p in px.iter_mut() {
            let bytes = p.to_le_bytes();
            if inr(bytes[0], b) && inr(bytes[1], g) && inr(bytes[2], r) && inr(bytes[3], al) {
                *p = (*p & 0xFFFFFF) | ((a as u32) << 24);
            }
        }
        return 0;
    }
    if passed & 1 != 0 {
        exact(px, c);
        return 0;
    }
    for p in px.iter_mut() {
        *p = (*p & 0xFFFFFF) | ((a as u32) << 24);
    }
    0
}

/// `_CLEARCOLOR {color | _NONE}`; passed: 1 = _NONE, 2 = color. Returns an error code.
///
/// # Safety
/// `im` must be a valid surface.
pub unsafe fn clearcolor(im: &mut ImgStruct, c: u32, passed: i32) -> i32 {
    const ERR: i32 = ERR_ILLEGAL_FUNCTION_CALL;
    let none = passed & 1 != 0;
    let color = passed & 2 != 0;
    if im.text != 0 {
        // _CLEARCOLOR _NONE is accepted (and ignored) on text surfaces
        return if none && !color { 0 } else { ERR };
    }
    if !im.pal.is_null() {
        if none {
            if color {
                return ERR;
            }
            im.transparent_color = -1;
            return 0;
        }
        if !color || c > 255 {
            return ERR;
        }
        im.transparent_color = c as i32;
        return 0;
    }
    // 32-bit: alpha is cleared on matching pixels; _NONE does nothing
    if none {
        return if color { ERR } else { 0 };
    }
    if !color {
        return ERR;
    }
    let c = c & 0xFFFFFF;
    let n = usize::from(im.width) * usize::from(im.height);
    // SAFETY: a 32-bit surface owns n pixels.
    let px = unsafe { core::slice::from_raw_parts_mut(im.offset32(), n) };
    for p in px.iter_mut() {
        if *p & 0xFFFFFF == c {
            *p = c;
        }
    }
    0
}

/// `_PALETTECOLOR(n)`: (value, error code).
///
/// # Safety
/// `im` must be a valid surface.
pub unsafe fn palettecolor_get(im: &ImgStruct, n: i32) -> (u32, i32) {
    if im.pal.is_null() || !(0..=255).contains(&n) {
        return (0, ERR_ILLEGAL_FUNCTION_CALL);
    }
    // SAFETY: palettes hold 256 entries.
    (unsafe { *im.pal.add(n as usize) } | 0xFF00_0000, 0)
}

/// `_PALETTECOLOR n, c`: error code.
///
/// # Safety
/// `im` must be a valid surface.
pub unsafe fn palettecolor_set(im: &mut ImgStruct, n: i32, c: u32) -> i32 {
    if im.pal.is_null() || !(0..=255).contains(&n) {
        return ERR_ILLEGAL_FUNCTION_CALL;
    }
    // SAFETY: palettes hold 256 entries.
    unsafe { *im.pal.add(n as usize) = c };
    0
}

// --- C ABI (the shims resolve handles to surfaces) ----------------------------

/// # Safety
/// `im` must be a valid surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_gfx_rgb(im: *const ImgStruct, r: i32, g: i32, b: i32) -> u32 {
    // SAFETY: guaranteed by the caller.
    unsafe { rgb(&*im, r, g, b) }
}

/// # Safety
/// `im` must be a valid surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_gfx_rgba(im: *const ImgStruct, r: i32, g: i32, b: i32, a: i32) -> u32 {
    // SAFETY: guaranteed by the caller.
    unsafe { rgba(&*im, r, g, b, a) }
}

/// # Safety
/// `im` must be a palette surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_gfx_matchcol(im: *const ImgStruct, r: i32, g: i32, b: i32) -> u32 {
    // SAFETY: guaranteed by the caller.
    unsafe { matchcol(&*im, r, g, b) }
}

/// `which`: 0 red, 1 green, 2 blue, 3 alpha.
///
/// # Safety
/// `im` must be a valid surface; `err` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_gfx_component(im: *const ImgStruct, col: u32, which: i32, err: *mut i32) -> i32 {
    let which = match which {
        0 => Component::Red,
        1 => Component::Green,
        2 => Component::Blue,
        _ => Component::Alpha,
    };
    // SAFETY: guaranteed by the caller.
    unsafe {
        let (v, e) = component(&*im, col, which);
        *err = e;
        v
    }
}

/// # Safety
/// `im` must be a valid surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_gfx_setalpha(im: *mut ImgStruct, a: i32, c: u32, c2: u32, passed: i32) -> i32 {
    // SAFETY: guaranteed by the caller.
    unsafe { setalpha(&mut *im, a, c, c2, passed) }
}

/// # Safety
/// `im` must be a valid surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_gfx_clearcolor(im: *mut ImgStruct, c: u32, passed: i32) -> i32 {
    // SAFETY: guaranteed by the caller.
    unsafe { clearcolor(&mut *im, c, passed) }
}

/// # Safety
/// `im` must be a valid surface; `err` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_gfx_palettecolor_get(im: *const ImgStruct, n: i32, err: *mut i32) -> u32 {
    // SAFETY: guaranteed by the caller.
    unsafe {
        let (v, e) = palettecolor_get(&*im, n);
        *err = e;
        v
    }
}

/// # Safety
/// `im` must be a valid surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_gfx_palettecolor_set(im: *mut ImgStruct, n: i32, c: u32) -> i32 {
    // SAFETY: guaranteed by the caller.
    unsafe { palettecolor_set(&mut *im, n, c) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::data::palette_256;
    use crate::gfx::testutil::TestImage;

    #[test]
    fn rgb_on_32bit_and_palettes() {
        let t = TestImage::new(1, 1, 32);
        assert_eq!(unsafe { rgb(&t.img, 300, -5, 16) }, 0xFFFF_0010);
        assert_eq!(unsafe { rgba(&t.img, 1, 2, 3, 999) }, 0xFF01_0203);
        let t = TestImage::new(1, 1, 13);
        assert_eq!(unsafe { rgb(&t.img, 0xAA, 0, 0) }, 4); // exact VGA red
        assert_eq!(unsafe { rgb(&t.img, 0xFF, 0xFF, 0xFF) }, 15); // first exact match
        let t = TestImage::new(1, 1, 12);
        assert_eq!(unsafe { matchcol(&t.img, 0x55, 0x55, 0xFF) }, 9);
        let mut t = TestImage::new(1, 1, 13);
        t.img.transparent_color = 7;
        assert_eq!(unsafe { rgba(&t.img, 0xAA, 0, 0, 0) }, 7);
        assert_eq!(unsafe { rgba(&t.img, 0xAA, 0, 0, 1) }, 4);
    }

    #[test]
    fn components() {
        let t = TestImage::new(1, 1, 32);
        assert_eq!(unsafe { component(&t.img, 0x8012_3456, Component::Red) }, (0x12, 0));
        assert_eq!(unsafe { component(&t.img, 0x8012_3456, Component::Alpha) }, (0x80, 0));
        let mut t = TestImage::new(1, 1, 13);
        assert_eq!(unsafe { component(&t.img, 4, Component::Red) }, (0xAA, 0));
        assert_eq!(unsafe { component(&t.img, 1, Component::Blue) }, (0xAA, 0));
        assert_eq!(unsafe { component(&t.img, 256, Component::Green) }, (0, 5));
        assert_eq!(unsafe { component(&t.img, 3, Component::Alpha) }, (255, 0));
        t.img.transparent_color = 3;
        assert_eq!(unsafe { component(&t.img, 3, Component::Alpha) }, (0, 0));
    }

    #[test]
    fn setalpha_all_exact_and_range() {
        let mut t = TestImage::new(3, 1, 32);
        t.pixels[..3].copy_from_slice(&[0xFF10_2030, 0xFF40_5060, 0x0010_2030]);
        assert_eq!(unsafe { setalpha(&mut t.img, 128, 0xFF10_2030, 0, 1) }, 0);
        assert_eq!(t.pixels[..3], [0x8010_2030, 0xFF40_5060, 0x0010_2030]);
        assert_eq!(unsafe { setalpha(&mut t.img, 7, 0x0000_0000, 0xFF30_3030, 1 | 4) }, 0);
        assert_eq!(t.pixels[..3], [0x0710_2030, 0xFF40_5060, 0x0710_2030]);
        assert_eq!(unsafe { setalpha(&mut t.img, 9, 0, 0, 0) }, 0);
        assert!(t.pixels[..3].iter().all(|p| p >> 24 == 9));
        assert_eq!(unsafe { setalpha(&mut t.img, 256, 0, 0, 0) }, 5);
        let mut t = TestImage::new(3, 1, 13);
        assert_eq!(unsafe { setalpha(&mut t.img, 1, 0, 0, 0) }, 5);
    }

    #[test]
    fn clearcolor_modes() {
        let mut t = TestImage::new(2, 1, 13);
        assert_eq!(unsafe { clearcolor(&mut t.img, 5, 2) }, 0);
        assert_eq!(t.img.transparent_color, 5);
        assert_eq!(unsafe { clearcolor(&mut t.img, 0, 1) }, 0);
        assert_eq!(t.img.transparent_color, -1);
        assert_eq!(unsafe { clearcolor(&mut t.img, 256, 2) }, 5);
        assert_eq!(unsafe { clearcolor(&mut t.img, 1, 3) }, 5);
        let mut t = TestImage::new(2, 1, 32);
        t.pixels[..2].copy_from_slice(&[0xFF11_2233, 0x8011_2234]);
        assert_eq!(unsafe { clearcolor(&mut t.img, 0xAA11_2233, 2) }, 0);
        assert_eq!(t.pixels[..2], [0x0011_2233, 0x8011_2234]);
        let mut t = TestImage::new(2, 1, 0);
        assert_eq!(unsafe { clearcolor(&mut t.img, 0, 1) }, 0);
        assert_eq!(unsafe { clearcolor(&mut t.img, 0, 2) }, 5);
    }

    #[test]
    fn palettecolor() {
        let mut t = TestImage::new(1, 1, 13);
        assert_eq!(unsafe { palettecolor_get(&t.img, 1) }, (palette_256[1], 0));
        assert_eq!(unsafe { palettecolor_set(&mut t.img, 1, 0x12_3456) }, 0);
        assert_eq!(unsafe { palettecolor_get(&t.img, 1) }, (0xFF12_3456, 0));
        assert_eq!(t.palette[1], 0x12_3456);
        assert_eq!(unsafe { palettecolor_get(&t.img, 256) }.1, 5);
        let mut t = TestImage::new(1, 1, 32);
        assert_eq!(unsafe { palettecolor_set(&mut t.img, 1, 0) }, 5);
    }
}
