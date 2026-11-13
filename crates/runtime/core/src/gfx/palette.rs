//! Palettes and colors: `restorepalette`, `qbg_palette` (PALETTE),
//! `qbg_sub_color` (COLOR, graphics surfaces), `defaultcolors`.

use super::data::{palette_256, palette_64};
use super::{qbr_i64, ImgStruct, ERR_ILLEGAL_FUNCTION_CALL};

/// Legacy `restorepalette`: default palette for the surface's mode.
///
/// # Safety
/// `im.pal` must hold 256 entries unless the surface is 32-bit.
pub unsafe fn restorepalette(im: &ImgStruct) {
    if im.bytes_per_pixel == 4 {
        return;
    }
    // SAFETY: guaranteed by the caller.
    let pal = unsafe { core::slice::from_raw_parts_mut(im.pal, 256) };
    match im.compatible_mode {
        1 => {
            pal[0] = palette_256[0];
            pal[1] = palette_256[3];
            pal[2] = palette_256[5];
            pal[3] = palette_256[7];
        }
        2 | 11 => {
            pal[0] = 0;
            pal[1] = 0xFFFFFF;
        }
        9 => {
            const MAP: [usize; 16] = [0, 1, 2, 3, 4, 5, 20, 7, 56, 57, 58, 59, 60, 61, 62, 63];
            for (i, m) in MAP.iter().enumerate() {
                pal[i] = palette_64[*m];
            }
        }
        10 => {
            pal[4] = 0;
            pal[5] = 4;
            pal[6] = 6;
            pal[7] = 8;
        }
        13 | 256 => pal.copy_from_slice(&palette_256),
        _ => pal[..16].copy_from_slice(&palette_256[..16]),
    }
}

/// VGA DAC conversion used by PALETTE: 6-bit components to 8-bit with the
/// legacy formula, which yields 256 for 63 and lets it carry into the next
/// byte (no alpha is stored either).
fn vga_color(col: u32) -> u32 {
    let scale = |c: u32| qbr_i64(f64::from(c) * f64::from(4.063492f32) - f64::from(0.4999999f32)) as i32;
    let r = scale(col & 63);
    let g = scale((col >> 8) & 63);
    let b = scale((col >> 16) & 63);
    b.wrapping_add(g.wrapping_mul(256)).wrapping_add(r.wrapping_mul(65536)) as u32
}

/// `PALETTE [attribute, color]`. Returns a QBasic error code.
///
/// # Safety
/// `im` must be a valid surface.
pub unsafe fn palette(im: &mut ImgStruct, attribute: u32, col: u32, passed: i32) -> i32 {
    const ERR: i32 = ERR_ILLEGAL_FUNCTION_CALL;
    if passed == 0 {
        // SAFETY: guaranteed by the caller.
        unsafe { restorepalette(im) };
        return 0;
    }
    if im.bytes_per_pixel == 4 {
        return ERR;
    }
    let attribute = attribute & 255;
    let a = attribute as usize;
    // SAFETY: non-32-bit surfaces own a 256-entry palette.
    let pal = unsafe { core::slice::from_raw_parts_mut(im.pal, 256) };
    // Legacy also tests `(col<0)` on an unsigned value: only the upper bound counts.
    let (max_attr, max_col, value): (u32, u32, fn(u32) -> u32) = match im.compatible_mode {
        13 | 256 | 12 | 11 => {
            let max_attr = match im.compatible_mode {
                12 => 15,
                11 => 1,
                _ => 255,
            };
            if attribute > max_attr || col & 0xFFC0_C0C0 != 0 {
                return ERR;
            }
            pal[a] = vga_color(col);
            return 0;
        }
        10 => {
            if attribute > 3 || col > 8 {
                return ERR;
            }
            pal[a + 4] = col;
            return 0;
        }
        9 | 0 => (15, 63, |c| palette_64[c as usize]),
        8 | 7 | 1 => (15, 15, |c| palette_256[c as usize]),
        2 => (1, 15, |c| palette_256[c as usize]),
        _ => return ERR,
    };
    if attribute > max_attr || col > max_col {
        return ERR;
    }
    pal[a] = value(col);
    0
}

/// `COLOR [fore][, back][, border]` on a non-console surface
/// (passed: 1 = fore, 2 = back, 4 = border). Returns a QBasic error code.
///
/// # Safety
/// `im` must be a valid surface.
pub unsafe fn color(im: &mut ImgStruct, col1: u32, col2: u32, passed: i32) -> i32 {
    const ERR: i32 = ERR_ILLEGAL_FUNCTION_CALL;
    if passed == 0 {
        return 0;
    }
    let p1 = passed & 1 != 0;
    let p2 = passed & 2 != 0;
    let mode = im.compatible_mode;
    if mode != 0 && passed & 4 != 0 {
        return ERR;
    }
    let pal = im.pal;
    // SAFETY: palette surfaces own 256 entries; only used for such modes.
    let setpal = |i: usize, v: u32| unsafe { *pal.add(i) = v };
    match mode {
        32 => {
            if p1 {
                im.color = col1;
            }
            if p2 {
                im.background_color = col2;
            }
        }
        256 | 13 | 12 | 11 => {
            let max = match mode {
                12 => 15,
                11 => 1,
                _ => 255,
            };
            if (p1 && col1 > max) || (p2 && col2 > max) {
                return ERR;
            }
            if p1 {
                im.color = col1;
            }
            if p2 {
                im.background_color = col2;
            }
        }
        10 => {
            if (p1 && col1 > 3) || (p2 && col2 > 8) {
                return ERR;
            }
            if p1 {
                im.color = col1;
            }
            if p2 {
                setpal(4, col2);
            }
        }
        9 => {
            if (p1 && col1 > 15) || (p2 && col2 > 63) {
                return ERR;
            }
            if p1 {
                im.color = col1;
            }
            if p2 {
                setpal(0, palette_64[col2 as usize]);
            }
        }
        8 | 7 | 2 => {
            let max1 = if mode == 2 { 1 } else { 15 };
            if (p1 && col1 > max1) || (p2 && col2 > 15) {
                return ERR;
            }
            if p1 {
                im.color = col1;
            }
            if p2 {
                setpal(0, palette_256[col2 as usize]);
            }
        }
        1 => {
            if p1 {
                if col1 > 15 {
                    return ERR;
                }
                setpal(0, palette_256[col1 as usize]);
            }
            if p2 {
                let set = if col2 & 1 != 0 { [3, 5, 7] } else { [2, 4, 6] };
                for (i, c) in set.iter().enumerate() {
                    setpal(i + 1, palette_256[*c]);
                }
            }
        }
        0 => {
            if (p1 && col1 > 31) || (p2 && col2 > 15) {
                return ERR;
            }
            if p1 {
                im.color = col1;
            }
            if p2 {
                im.background_color = col2 & 7;
            }
        }
        _ => return ERR,
    }
    0
}

/// Legacy `defaultcolors` (called on SCREEN page changes).
pub fn defaultcolors(im: &mut ImgStruct) {
    let (c, b) = match im.compatible_mode {
        0 => (7, 0),
        1 => (3, 0),
        2 => (1, 0),
        10 => (3, 0),
        11 => (1, 0),
        32 => (0xFFFF_FFFF, 0xFF00_0000),
        _ => (15, 0),
    };
    im.color = c;
    im.background_color = b;
    im.draw_color = im.color;
}

// --- entry points (the caller passes write_page) ------------------------------

/// # Safety
/// `im` must be a valid surface.
pub unsafe fn qb_gfx_palette(im: *mut ImgStruct, attribute: u32, col: u32, passed: i32) -> i32 {
    // SAFETY: guaranteed by the caller.
    unsafe { palette(&mut *im, attribute, col, passed) }
}

/// # Safety
/// `im` must be a valid, non-console surface.
pub unsafe fn qb_gfx_color(im: *mut ImgStruct, col1: u32, col2: u32, passed: i32) -> i32 {
    // SAFETY: guaranteed by the caller.
    unsafe { color(&mut *im, col1, col2, passed) }
}

/// # Safety
/// `im` must be a valid surface.
pub unsafe fn qb_gfx_defaultcolors(im: *mut ImgStruct) {
    // SAFETY: guaranteed by the caller.
    defaultcolors(unsafe { &mut *im })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::data::{palette_256, palette_64};
    use crate::gfx::testutil::TestImage;

    #[test]
    fn palette_vga_scales_6bit_components() {
        let mut t = TestImage::new(4, 4, 13);
        // red 63, green 32, blue 1: CINT(c*4.063492f-0.4999999f) gives 255, 130, 4.
        // Legacy stores no alpha.
        assert_eq!(unsafe { palette(&mut t.img, 3, 63 + 32 * 256 + 65536, 1) }, 0);
        assert_eq!(t.palette[3], 0x00FF_8204);
        assert_eq!(unsafe { palette(&mut t.img, 300, 0, 1) }, 0); // attribute & 255 = 44
        assert_eq!(t.palette[44], 0);
        assert_eq!(unsafe { palette(&mut t.img, 1, 64, 1) }, 5);
    }

    #[test]
    fn palette_ranges_per_mode() {
        let mut t = TestImage::new(4, 4, 12);
        assert_eq!(unsafe { palette(&mut t.img, 16, 0, 1) }, 5);
        let mut t = TestImage::new(4, 4, 9);
        assert_eq!(unsafe { palette(&mut t.img, 2, 63, 1) }, 0);
        assert_eq!(t.palette[2], palette_64[63]);
        assert_eq!(unsafe { palette(&mut t.img, 2, 64, 1) }, 5);
        let mut t = TestImage::new(4, 4, 7);
        assert_eq!(unsafe { palette(&mut t.img, 1, 14, 1) }, 0);
        assert_eq!(t.palette[1], palette_256[14]);
        let mut t = TestImage::new(4, 4, 10);
        assert_eq!(unsafe { palette(&mut t.img, 3, 8, 1) }, 0);
        assert_eq!(t.palette[7], 8);
        let mut t = TestImage::new(4, 4, 32);
        assert_eq!(unsafe { palette(&mut t.img, 1, 1, 1) }, 5);
    }

    #[test]
    fn palette_without_arguments_restores() {
        let mut t = TestImage::new(4, 4, 13);
        t.palette[5] = 1;
        assert_eq!(unsafe { palette(&mut t.img, 0, 0, 0) }, 0);
        assert_eq!(t.palette[5], palette_256[5]);
    }

    #[test]
    fn color_per_mode() {
        let mut t = TestImage::new(4, 4, 13);
        assert_eq!(unsafe { color(&mut t.img, 200, 3, 3) }, 0);
        assert_eq!((t.img.color, t.img.background_color), (200, 3));
        assert_eq!(unsafe { color(&mut t.img, 256, 0, 1) }, 5);
        assert_eq!(unsafe { color(&mut t.img, 1, 0, 4) }, 5);
        let mut t = TestImage::new(4, 4, 9);
        assert_eq!(unsafe { color(&mut t.img, 14, 63, 3) }, 0);
        assert_eq!((t.img.color, t.palette[0]), (14, palette_64[63]));
        let mut t = TestImage::new(4, 4, 1);
        assert_eq!(unsafe { color(&mut t.img, 4, 0, 3) }, 0);
        assert_eq!(t.palette[0], palette_256[4]);
        assert_eq!(t.palette[1], palette_256[2]);
        let mut t = TestImage::new(4, 4, 32);
        assert_eq!(unsafe { color(&mut t.img, 0x1234_5678, 7, 3) }, 0);
        assert_eq!((t.img.color, t.img.background_color), (0x1234_5678, 7));
        let mut t = TestImage::new(4, 4, 0);
        assert_eq!(unsafe { color(&mut t.img, 31, 15, 3) }, 0);
        assert_eq!((t.img.color, t.img.background_color), (31, 7));
        assert_eq!(unsafe { color(&mut t.img, 32, 0, 1) }, 5);
    }

    #[test]
    fn defaults() {
        let mut t = TestImage::new(4, 4, 32);
        t.img.color = 1;
        defaultcolors(&mut t.img);
        assert_eq!((t.img.color, t.img.background_color, t.img.draw_color), (0xFFFF_FFFF, 0xFF00_0000, 0xFFFF_FFFF));
        let mut t = TestImage::new(4, 4, 10);
        defaultcolors(&mut t.img);
        assert_eq!(t.img.color, 3);
    }
}
