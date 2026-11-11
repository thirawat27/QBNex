//! Text in graphics modes with the built-in 8x8, 8x14 and 8x16 fonts
//! (the default-font part of `printchr`). Custom fonts (`_LOADFONT`) and text
//! surfaces stay in libqb.cpp.

use super::blend::pset;
use super::data::{charset8x16, charset8x8};
use super::ImgStruct;

/// Draws `character` at the text cursor of graphics surface `im` with font
/// 8, 14 or 16 (`_PRINTMODE` 1 keep background, 2 only background, 3 fill).
/// Other font numbers draw nothing.
///
/// # Safety
/// The character cell must lie inside `im` (as in legacy, no clipping).
pub unsafe fn printchr_builtin(im: &ImgStruct, character: i32) {
    let (rows, glyph): (u32, &[u8]) = match im.font {
        8 => (8, &charset8x8[..]),
        14 => (14, &charset8x16[..]),
        16 => (16, &charset8x16[..]),
        _ => return,
    };
    let ch = (character & 255) as usize; // built-in fonts are not unicode
    let start = match im.font {
        8 => ch * 64,
        14 => ch * 128 + 8, // rows 1..=14 of the 8x16 glyph
        _ => ch * 128,
    };
    let x = 8u32.wrapping_mul((im.cursor_x as i32 - 1) as u32);
    let y = ((im.cursor_y as i32 - 1) as u32).wrapping_mul(rows);
    let (color, background) = (im.color, im.background_color);
    for y2 in 0..rows {
        for x2 in 0..8u32 {
            let ink = glyph[start + (y2 * 8 + x2) as usize] != 0;
            let px = x.wrapping_add(x2) as i32;
            let py = y.wrapping_add(y2) as i32;
            // SAFETY: guaranteed by the caller.
            unsafe {
                match im.print_mode {
                    3 => pset(im, px, py, if ink { color } else { background }),
                    1 if ink => pset(im, px, py, color),
                    2 if !ink => pset(im, px, py, background),
                    _ => {}
                }
            }
        }
    }
}

/// C ABI: built-in font glyph on `write_page`.
///
/// # Safety
/// See [`printchr_builtin`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_gfx_printchr(im: *const ImgStruct, character: i32) {
    // SAFETY: guaranteed by the caller.
    unsafe { printchr_builtin(&*im, character) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::testutil::TestImage;

    #[test]
    fn glyph_cell_position_and_modes() {
        let mut t = TestImage::new(32, 32, 13); // font 8
        t.img.cursor_x = 2;
        t.img.cursor_y = 3;
        t.img.color = 5;
        t.img.background_color = 1;
        unsafe { printchr_builtin(&t.img, 'A' as i32) };
        // the cell (8..16, 16..24) is fully painted: ink 5, paper 1
        let cell: Vec<u8> = (16..24).flat_map(|y| (8..16).map(move |x| (x, y))).map(|(x, y)| t.p8(x, y)).collect();
        assert!(cell.iter().all(|c| *c == 5 || *c == 1));
        assert!(cell.contains(&5) && cell.contains(&1));
        assert_eq!(t.p8(7, 16), 0);
        // print mode 1 keeps the background
        let mut u = TestImage::new(32, 32, 13);
        u.img.print_mode = 1;
        u.img.color = 5;
        unsafe { printchr_builtin(&u.img, 'A' as i32 + 256) }; // only the low byte counts
        assert!((0..8).flat_map(|y| (0..8).map(move |x| (x, y))).all(|(x, y)| u.p8(x, y) == 5 || u.p8(x, y) == 0));
        assert_eq!(u.count_nonzero(), cell.iter().filter(|c| **c == 5).count());
    }

    #[test]
    fn font14_uses_rows_1_to_14_of_8x16() {
        let mut t = TestImage::new(8, 14, 9);
        t.img.color = 15;
        t.img.print_mode = 1;
        unsafe { printchr_builtin(&t.img, 'H' as i32) };
        for y in 0..14 {
            for x in 0..8 {
                let ink = charset8x16['H' as usize * 128 + (y + 1) * 8 + x] != 0;
                assert_eq!(t.p8(x, y) == 15, ink);
            }
        }
    }
}
