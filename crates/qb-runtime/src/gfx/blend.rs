//! Software alpha blending (`init_blend`, `cblend`/`ablend` tables) and the
//! `pset`/`pset_and_clip` pixel writers of libqb.cpp.

use super::{qbr_f2l, ImgStruct};
use std::sync::OnceLock;

/// The legacy lookup tables.
/// - `cblend[a<<16 | src<<8 | dst]` = CINT(a/255*src + (1-a/255)*dst)
/// - `ablend[a1<<8 | a2]` = CINT((1-(1-a1/255)*(1-a2/255))*255)
pub struct Tables {
    pub cblend: Vec<u8>,
    pub ablend: Vec<u8>,
}

fn build() -> Tables {
    let mut cblend = Vec::with_capacity(1 << 24);
    for i in 0..256 {
        let mut f = i as f32;
        f = (f64::from(f) / 255.0) as f32; // f/=255.0
        let inv = 1.0 - f64::from(f); // (1.0-f) is a double expression
        for x2 in 0..256 {
            let fs = f * x2 as f32; // (f*f2) stays float
            for x3 in 0..256 {
                let v = f64::from(fs) + inv * f64::from(x3 as f32);
                cblend.push(qbr_f2l(v as f32) as u8);
            }
        }
    }
    let mut ablend = Vec::with_capacity(1 << 16);
    for i in 0..256 {
        for i2 in 0..256 {
            let mut f = i as f32;
            let mut f2 = i2 as f32;
            f = (f64::from(f) / 255.0) as f32;
            f2 = (f64::from(f2) / 255.0) as f32;
            f = (1.0 - f64::from(f)) as f32;
            f2 = (1.0 - f64::from(f2)) as f32;
            let f3 = f * f2;
            let z = qbr_f2l(((1.0 - f64::from(f3)) * 255.0) as f32);
            ablend.push(z as u8);
        }
    }
    Tables { cblend, ablend }
}

static TABLES: OnceLock<Tables> = OnceLock::new();

/// The blend tables, built on first use (legacy built them when the first
/// 32-bit image was created; the content is the same).
pub fn tables() -> &'static Tables {
    TABLES.get_or_init(build)
}

/// Blends `col` (ARGB) onto `*p` with the legacy fast paths for alpha 0, 127,
/// 128 and 255.
///
/// # Safety
/// `p` must be valid for reads and writes.
#[inline]
pub unsafe fn blend_onto(p: *mut u32, col: u32) {
    // SAFETY: guaranteed by the caller.
    unsafe {
        match col & 0xFF00_0000 {
            0xFF00_0000 => *p = col,
            0 => {}
            0x8000_0000 => {
                let t = tables();
                let d = *p;
                *p = (((d & 0xFEFEFE) + (col & 0xFEFEFE)) >> 1)
                    .wrapping_add(u32::from(t.ablend[(128 << 8) + (d >> 24) as usize]) << 24);
            }
            0x7F00_0000 => {
                let t = tables();
                let d = *p;
                *p = (((d & 0xFEFEFE) + (col & 0xFEFEFE)) >> 1)
                    .wrapping_add(u32::from(t.ablend[(127 << 8) + (d >> 24) as usize]) << 24);
            }
            _ => *p = blend_ranged(*p, col),
        }
    }
}

/// The table blend used for every alpha other than 0/127/128/255.
#[inline]
pub fn blend_ranged(destcol: u32, col: u32) -> u32 {
    let t = tables();
    let cp = &t.cblend[((col >> 24) << 16) as usize..];
    u32::from(cp[((col << 8 & 0xFF00) + (destcol & 255)) as usize])
        + (u32::from(cp[((col & 0xFF00) + (destcol >> 8 & 255)) as usize]) << 8)
        + (u32::from(cp[((col >> 8 & 0xFF00) + (destcol >> 16 & 255)) as usize]) << 16)
        + (u32::from(t.ablend[((col >> 24) + (destcol >> 16 & 0xFF00)) as usize]) << 24)
}

/// Legacy `pset(x,y,col)`: writes one pixel of `im` without clipping.
///
/// # Safety
/// `(x, y)` must lie inside `im`, whose pixel buffer must be valid.
#[inline]
pub unsafe fn pset(im: &ImgStruct, x: i32, y: i32, col: u32) {
    let idx = (y * i32::from(im.width) + x) as isize;
    // SAFETY: guaranteed by the caller.
    unsafe {
        if im.bytes_per_pixel == 1 {
            *im.offset.offset(idx) = (col & im.mask) as u8;
        } else if im.alpha_disabled != 0 {
            *im.offset32().offset(idx) = col;
        } else {
            blend_onto(im.offset32().offset(idx), col);
        }
    }
}

/// Legacy `pset_and_clip`: `pset` limited to the VIEW rectangle.
///
/// # Safety
/// The VIEW rectangle must lie inside `im`.
#[inline]
pub unsafe fn pset_and_clip(im: &ImgStruct, x: i32, y: i32, col: u32) {
    if x >= im.view_x1 && x <= im.view_x2 && y >= im.view_y1 && y <= im.view_y2 {
        // SAFETY: inside the view, hence inside the image.
        unsafe { pset(im, x, y, col) };
    }
}

/// C ABI for the C++ code that still plots single pixels (custom fonts).
///
/// # Safety
/// `im` must be a valid surface and `(x, y)` inside it.
#[no_mangle]
pub unsafe extern "C" fn qb_gfx_pset(im: *const ImgStruct, x: i32, y: i32, col: u32) {
    // SAFETY: guaranteed by the caller.
    unsafe { pset(&*im, x, y, col) }
}

/// Exposes the blend tables to C++ (`cblend`, `ablend`), building them first.
///
/// # Safety
/// The out pointers must be writable.
#[no_mangle]
pub unsafe extern "C" fn qb_gfx_blend_tables(cblend: *mut *const u8, ablend: *mut *const u8) {
    let t = tables();
    // SAFETY: guaranteed by the caller.
    unsafe {
        *cblend = t.cblend.as_ptr();
        *ablend = t.ablend.as_ptr();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::testutil::TestImage;

    #[test]
    fn table_endpoints() {
        let t = tables();
        // alpha 0: destination wins; alpha 255: source wins.
        assert_eq!(t.cblend[0x00_C8_37], 0x37);
        assert_eq!(t.cblend[0xFF_C8_37], 0xC8);
        // alpha 0 over anything keeps the other alpha.
        assert_eq!(t.ablend[0x00_80], 0x80);
        assert_eq!(t.ablend[0xFF_10], 0xFF);
        // half: CINT(127.5*... ) of 0.5019...*200 + 0.498*0
        assert_eq!(t.cblend[(128 << 16) | (200 << 8)], 100);
    }

    #[test]
    fn pset_8bit_masks_color() {
        let t = TestImage::new(4, 4, 12);
        unsafe { pset(&t.img, 1, 2, 0x1F) };
        assert_eq!(t.p8(1, 2), 0x0F);
    }

    #[test]
    fn pset_32bit_alpha_paths() {
        let mut t = TestImage::new(4, 1, 32);
        t.pixels[0] = 0xFF00_0000;
        unsafe { pset(&t.img, 0, 0, 0x00FF_FFFF) }; // alpha 0: no change
        assert_eq!(t.p32(0, 0), 0xFF00_0000);
        unsafe { pset(&t.img, 0, 0, 0x80FE_FEFE) }; // half: average
        assert_eq!(t.p32(0, 0) & 0xFFFFFF, 0x7F7F7F);
        assert_eq!(t.p32(0, 0) >> 24, 0xFF);
        unsafe { pset(&t.img, 1, 0, 0x4000_00FF) };
        assert_eq!(t.p32(1, 0) & 0xFF, 64);
        t.img.alpha_disabled = 1;
        unsafe { pset(&t.img, 2, 0, 0x1234_5678) };
        assert_eq!(t.p32(2, 0), 0x1234_5678);
    }

    #[test]
    fn pset_and_clip_respects_view() {
        let mut t = TestImage::new(4, 4, 13);
        t.img.view_x1 = 1;
        t.img.view_y1 = 1;
        t.img.view_x2 = 2;
        t.img.view_y2 = 2;
        unsafe {
            pset_and_clip(&t.img, 0, 0, 9);
            pset_and_clip(&t.img, 2, 2, 9);
        }
        assert_eq!(t.p8(0, 0), 0);
        assert_eq!(t.p8(2, 2), 9);
    }
}
