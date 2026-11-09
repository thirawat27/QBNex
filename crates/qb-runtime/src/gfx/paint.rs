//! PAINT: color fill (`sub_paint` 8-bit, `sub_paint32`, `sub_paint32x`) and
//! tiled fill (`sub_paint` with a pattern string).
//!
//! The legacy fills keep two node lists (65536 entries each, `uint16`
//! coordinates) and a `done` byte map. Every pixel is tested exactly once,
//! before it is painted, so the filled set is the 4-connected region of
//! fillable pixels around the start point inside the VIEW, independent of
//! traversal order. The Rust code keeps the same breadth-first traversal but
//! grows the lists instead of overrunning them (legacy wrote past the end of
//! its 65536-entry lists, undefined behaviour, on very large frontiers).

use super::blend::blend_onto;
use super::{map_point_f2l, ImgStruct, ERR_ILLEGAL_FUNCTION_CALL};

/// Breadth-first fill shared by every PAINT variant. `test(offset)` decides
/// whether a not-yet-visited pixel is filled; `plot(offset)` paints it. The
/// start pixel is painted by the caller.
#[allow(clippy::int_plus_one)] // same comparisons as legacy
fn flood(
    im: &ImgStruct,
    ix: i32,
    iy: i32,
    mut test: impl FnMut(usize) -> bool,
    mut plot: impl FnMut(usize, i32, i32),
) {
    let width = i32::from(im.width);
    let mut done = vec![false; usize::from(im.width) * usize::from(im.height)];
    done[(iy * width + ix) as usize] = true;
    // node: (x, y, type) where type bits are 1 left, 2 right, 4 above, 8 below
    let mut a: Vec<(i32, i32, u8)> = vec![(ix, iy, 15)];
    let mut b: Vec<(i32, i32, u8)> = Vec::new();
    loop {
        b.clear();
        for &(x, y, t) in &a {
            let mut visit = |x2: i32, y2: i32, nt: u8| {
                let off = (y2 * width + x2) as usize;
                if !done[off] {
                    done[off] = true;
                    if test(off) {
                        plot(off, x2, y2);
                        b.push((x2, y2, nt));
                    }
                }
            };
            if t & 1 != 0 && x - 1 >= im.view_x1 {
                visit(x - 1, y, 13);
            }
            if t & 2 != 0 && x + 1 <= im.view_x2 {
                visit(x + 1, y, 14);
            }
            if t & 4 != 0 && y - 1 >= im.view_y1 {
                visit(x, y - 1, 7);
            }
            if t & 8 != 0 && y + 1 <= im.view_y2 {
                visit(x, y + 1, 11);
            }
        }
        if b.is_empty() {
            return;
        }
        core::mem::swap(&mut a, &mut b);
    }
}

/// Moves the graphics cursor (STEP or absolute) and maps it to pixels;
/// `None` when the point is outside the VIEW.
fn start_point(im: &mut ImgStruct, x: f32, y: f32, passed: i32) -> Option<(i32, i32)> {
    if passed & 1 != 0 {
        im.x += x;
        im.y += y;
    } else {
        im.x = x;
        im.y = y;
    }
    let (ix, iy) = map_point_f2l(im, im.x, im.y);
    if ix < im.view_x1 || iy < im.view_y1 || ix > im.view_x2 || iy > im.view_y2 {
        return None;
    }
    Some((ix, iy))
}

/// `PAINT [STEP](x, y)[, fill[, border]]`; passed: 1 = STEP, 2 = fill,
/// 4 = border, 8 = background string (illegal with a color fill).
///
/// # Safety
/// `im` must be a valid surface whose VIEW lies inside it.
pub unsafe fn paint_color(im: &mut ImgStruct, x: f32, y: f32, mut fillcol: u32, mut bordercol: u32, passed: i32) -> i32 {
    if im.text != 0 || passed & 8 != 0 {
        return ERR_ILLEGAL_FUNCTION_CALL;
    }
    if passed & 2 == 0 {
        fillcol = im.color;
    }
    if passed & 4 == 0 {
        bordercol = fillcol;
    }
    if im.bytes_per_pixel != 4 {
        fillcol &= im.mask;
        bordercol &= im.mask;
    }
    im.draw_color = fillcol;
    let Some((ix, iy)) = start_point(im, x, y, passed) else { return 0 };
    let start = (iy * i32::from(im.width) + ix) as usize;
    let im: &ImgStruct = im;
    if im.bytes_per_pixel == 4 {
        let p = im.offset32();
        // SAFETY (all accesses): offsets come from points inside the view.
        let get = |o: usize| unsafe { *p.add(o) };
        if get(start) == bordercol {
            return 0;
        }
        if im.alpha_disabled != 0 {
            let set = |o: usize| unsafe { *p.add(o) = fillcol };
            set(start);
            flood(im, ix, iy, |o| get(o) != bordercol, |o, _, _| set(o));
        } else {
            let set = |o: usize| unsafe { blend_onto(p.add(o), fillcol) };
            set(start);
            flood(im, ix, iy, |o| get(o) != bordercol, |o, _, _| set(o));
        }
    } else {
        let p = im.offset;
        let get = |o: usize| u32::from(unsafe { *p.add(o) });
        if get(start) == bordercol {
            return 0;
        }
        let set = |o: usize| unsafe { *p.add(o) = fillcol as u8 };
        set(start);
        flood(im, ix, iy, |o| get(o) != bordercol, |o, _, _| set(o));
    }
    0
}

/// Pattern decoding of the legacy `getptsize_*`/`getptcol_*` helpers.
#[derive(Clone, Copy)]
enum Pattern {
    Bpp1,
    Bpp2,
    Bpp4,
    Bpp8,
}

impl Pattern {
    fn for_mode(mode: u16) -> Pattern {
        match mode {
            1 => Pattern::Bpp2,
            2 => Pattern::Bpp1,
            13 => Pattern::Bpp8,
            _ => Pattern::Bpp4, // 7, 8, 9, 12 and every other mode
        }
    }
    fn size(self, len: usize) -> (usize, usize) {
        match self {
            Pattern::Bpp1 => (8, len),
            Pattern::Bpp2 => (4, len),
            Pattern::Bpp4 => (8, len.div_ceil(4)),
            Pattern::Bpp8 => (1, len),
        }
    }
    fn color(self, s: &[u8], x: usize, y: usize) -> u8 {
        match self {
            Pattern::Bpp1 => (s[y] >> (7 - x)) & 1,
            Pattern::Bpp2 => (s[y] >> ((3 - x) << 1)) & 3,
            Pattern::Bpp4 => {
                let mut c = 0u8;
                for i in 0..4 {
                    let byte = s.get(y * 4 + i).copied().unwrap_or(0);
                    if byte & (1 << (7 - x)) != 0 {
                        c |= 1 << i;
                    }
                }
                c
            }
            Pattern::Bpp8 => s[y],
        }
    }
}

/// `PAINT [STEP](x, y), tile$[, border]`; passed as for [`paint_color`]
/// (2 is required). A border of 0 counts as "not given": the fill then
/// spreads over pixels equal to the start pixel. The surface is always
/// accessed as bytes, as in legacy.
///
/// # Safety
/// `im` must be a valid surface whose VIEW lies inside it.
pub unsafe fn paint_tile(im: &mut ImgStruct, x: f32, y: f32, tile: &[u8], mut bordercol: u32, passed: i32) -> i32 {
    if passed & 2 == 0 || tile.is_empty() {
        return ERR_ILLEGAL_FUNCTION_CALL;
    }
    let pat = Pattern::for_mode(im.compatible_mode);
    let (sx, sy) = pat.size(tile.len());
    // Legacy `static uint8 tile[8][64]`, indexed [x][y] and filled column by
    // column; rows >= 64 alias the next column exactly as in that flat array.
    let mut t = vec![0u8; 8 * 64 + sy];
    for xx in 0..sx {
        for yy in 0..sy {
            t[xx * 64 + yy] = pat.color(tile, xx, yy);
        }
    }
    let tile_at = |x: i32, y: i32| t[(x as usize % sx) * 64 + (y as usize % sy)];
    bordercol &= im.mask;
    let Some((ix, iy)) = start_point(im, x, y, passed) else { return 0 };
    let im: &ImgStruct = im;
    let p = im.offset;
    let start = (iy * i32::from(im.width) + ix) as usize;
    // SAFETY (all accesses): offsets come from points inside the view.
    let get = |o: usize| u32::from(unsafe { *p.add(o) });
    let first = get(start) & im.mask;
    if bordercol != 0 && get(start) == bordercol {
        return 0;
    }
    unsafe { *p.add(start) = tile_at(ix, iy) };
    let test = |o: usize| if bordercol != 0 { get(o) != bordercol } else { get(o) == first };
    flood(im, ix, iy, test, |o, x2, y2| unsafe { *p.add(o) = tile_at(x2, y2) });
    0
}

// --- C ABI ---------------------------------------------------------------------

/// # Safety
/// `im` must be `write_page`.
#[no_mangle]
pub unsafe extern "C" fn qb_gfx_paint(im: *mut ImgStruct, x: f32, y: f32, fillcol: u32, bordercol: u32, passed: i32) -> i32 {
    // SAFETY: guaranteed by the caller.
    unsafe { paint_color(&mut *im, x, y, fillcol, bordercol, passed) }
}

/// # Safety
/// `im` must be `write_page`; `tile` readable for `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn qb_gfx_paint_tile(
    im: *mut ImgStruct,
    x: f32,
    y: f32,
    tile: *const u8,
    len: i32,
    bordercol: u32,
    passed: i32,
) -> i32 {
    // SAFETY: guaranteed by the caller.
    unsafe {
        let s = if tile.is_null() || len <= 0 { &[][..] } else { core::slice::from_raw_parts(tile, len as usize) };
        paint_tile(&mut *im, x, y, s, bordercol, passed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::line::sub_line;
    use crate::gfx::testutil::TestImage;

    fn boxed(mode: u16) -> Box<TestImage> {
        let mut t = TestImage::new(16, 16, mode);
        let c = if mode == 32 { 0xFFFF_FFFF } else { 15 };
        unsafe { sub_line(&mut t.img, 2.0, 2.0, 10.0, 10.0, c, 1, 0, 1 | 8) };
        t
    }

    #[test]
    fn fills_inside_border_only() {
        let mut t = boxed(13);
        assert_eq!(unsafe { paint_color(&mut t.img, 5.0, 5.0, 4, 15, 1 << 1 | 1 << 2) }, 0);
        assert_eq!(t.p8(5, 5), 4);
        assert_eq!(t.p8(3, 9), 4);
        assert_eq!(t.p8(2, 2), 15);
        assert_eq!(t.p8(1, 1), 0);
        assert_eq!(t.p8(12, 12), 0);
        assert_eq!(t.img.draw_color, 4);
        assert_eq!((t.img.x, t.img.y), (5.0, 5.0));
    }

    #[test]
    fn border_defaults_to_fill_color() {
        let mut t = boxed(13);
        t.img.color = 15;
        unsafe { paint_color(&mut t.img, 0.0, 0.0, 0, 0, 0) }; // fill outside with 15
        assert_eq!(t.p8(0, 0), 15);
        assert_eq!(t.p8(15, 15), 15);
        assert_eq!(t.p8(5, 5), 0);
    }

    #[test]
    fn start_on_border_does_nothing() {
        let mut t = boxed(13);
        unsafe { paint_color(&mut t.img, 2.0, 2.0, 4, 15, 6) };
        assert_eq!(t.p8(5, 5), 0);
    }

    #[test]
    fn fill_32bit_with_and_without_blending() {
        let mut t = boxed(32);
        unsafe { paint_color(&mut t.img, 5.0, 5.0, 0xFF00_FF00, 0xFFFF_FFFF, 6) };
        assert_eq!(t.p32(5, 5), 0xFF00_FF00);
        assert_eq!(t.p32(0, 0), 0);
        let mut t = boxed(32);
        unsafe { paint_color(&mut t.img, 5.0, 5.0, 0x8000_FE00, 0xFFFF_FFFF, 6) };
        assert_eq!(t.p32(5, 5) & 0xFFFFFF, 0x007F00);
        let mut t = boxed(32);
        t.img.alpha_disabled = 1;
        unsafe { paint_color(&mut t.img, 5.0, 5.0, 0x1200_FE00, 0xFFFF_FFFF, 6) };
        assert_eq!(t.p32(5, 5), 0x1200_FE00);
    }

    #[test]
    fn color_paint_errors() {
        let mut t = TestImage::new(8, 8, 0);
        assert_eq!(unsafe { paint_color(&mut t.img, 0.0, 0.0, 1, 1, 6) }, 5);
        let mut t = boxed(13);
        assert_eq!(unsafe { paint_color(&mut t.img, 0.0, 0.0, 1, 1, 8) }, 5);
    }

    #[test]
    fn tile_screen13_uses_one_byte_per_pixel() {
        let mut t = boxed(13);
        // tile rows: 2 rows of single-byte colors 7 and 9
        assert_eq!(unsafe { paint_tile(&mut t.img, 5.0, 5.0, &[7, 9], 15, 2 | 4) }, 0);
        assert_eq!(t.p8(5, 4), 7); // row 4 even -> 7
        assert_eq!(t.p8(5, 5), 9);
        assert_eq!(t.p8(1, 1), 0);
    }

    #[test]
    fn tile_screen12_planar_bits() {
        let mut t = boxed(12);
        // 4 planes, one row: plane0 = &H80 -> leftmost pixel gets bit 0
        unsafe { paint_tile(&mut t.img, 5.0, 5.0, &[0x80, 0x80, 0, 0], 15, 2 | 4) };
        assert_eq!(t.p8(8, 5), 3); // x % 8 == 0 -> colour 1+2
        assert_eq!(t.p8(5, 5), 0);
    }

    #[test]
    fn tile_errors() {
        let mut t = boxed(13);
        assert_eq!(unsafe { paint_tile(&mut t.img, 5.0, 5.0, &[], 15, 2) }, 5);
        assert_eq!(unsafe { paint_tile(&mut t.img, 5.0, 5.0, &[1], 15, 0) }, 5);
    }

    #[test]
    fn tile_without_border_fills_first_color_region() {
        let mut t = boxed(13);
        // border 0 means "not given": fill pixels equal to the start color
        unsafe { paint_tile(&mut t.img, 5.0, 5.0, &[3], 0, 2) };
        assert_eq!(t.p8(5, 5), 3);
        assert_eq!(t.p8(1, 1), 0);
    }
}
