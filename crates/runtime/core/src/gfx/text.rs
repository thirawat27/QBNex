//! Text in graphics modes with the built-in 8x8, 8x14 and 8x16 fonts
//! (the default-font part of `printchr`). Custom fonts (`_LOADFONT`) and text
//! surfaces are drawn elsewhere.

use super::blend::{pset, pset_and_clip};
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

/// `_PRINTSTRING` with font 8, 14 or 16: `text` with the top left corner of its
/// first character at pixel (`x`, `y`), 8 pixels a character, clipped at the
/// edges of `im`. Other font numbers draw nothing.
///
/// # Safety
/// `im` must be a valid graphics surface.
pub unsafe fn printstring_builtin(im: &ImgStruct, x: i32, y: i32, text: &[u8]) {
    let (rows, glyph, skip): (i32, &[u8], usize) = match im.font {
        8 => (8, &charset8x8[..], 0),
        14 => (14, &charset8x16[..], 8), // rows 1..=14 of the 8x16 glyph
        16 => (16, &charset8x16[..], 0),
        _ => return,
    };
    let cell = if im.font == 8 { 64 } else { 128 };
    let (color, background) = (im.color, im.background_color);
    for (n, &ch) in text.iter().enumerate() {
        let start = usize::from(ch) * cell + skip;
        let left = x.saturating_add((n as i32).saturating_mul(8));
        for y2 in 0..rows {
            for x2 in 0..8 {
                let ink = glyph[start + (y2 * 8 + x2) as usize] != 0;
                let (px, py) = (left.saturating_add(x2), y.saturating_add(y2));
                // SAFETY: the clipping variant checks the position.
                unsafe {
                    match im.print_mode {
                        3 => pset_and_clip(im, px, py, if ink { color } else { background }),
                        1 if ink => pset_and_clip(im, px, py, color),
                        2 if !ink => pset_and_clip(im, px, py, background),
                        _ => {}
                    }
                }
            }
        }
    }
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
    fn printstring_draws_at_a_pixel_position_and_clips() {
        let mut t = TestImage::new(20, 10, 13); // font 8
        t.img.color = 7;
        t.img.background_color = 2;
        t.img.print_mode = 3;
        // the third character starts at x 19 and runs past the right edge; rows 3..10 of 8 fit
        unsafe { printstring_builtin(&t.img, 3, 3, b"ABC") };
        assert_eq!(t.p8(2, 3), 0);
        assert_eq!(t.p8(3, 2), 0);
        for y in 3..10 {
            for x in 3..20 {
                let (ch, gx) = (b"ABC"[(x - 3) / 8], (x - 3) % 8);
                let ink = charset8x8[usize::from(ch) * 64 + (y - 3) * 8 + gx] != 0;
                assert_eq!(t.p8(x, y), if ink { 7 } else { 2 }, "pixel {x},{y}");
            }
        }
        // entirely outside: nothing is written
        let u = TestImage::new(8, 8, 13);
        unsafe { printstring_builtin(&u.img, -100, 50, b"A") };
        assert_eq!(u.count_nonzero(), 0);
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
