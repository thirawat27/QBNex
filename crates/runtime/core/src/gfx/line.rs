//! LINE: `lineclip`, `fast_line`, `qb32_line`, `qb32_boxfill`,
//! `fast_boxfill` and `sub_line` of libqb.cpp.

use super::blend::{blend_onto, pset};
use super::{cvt_i32, map_point_f2l, qbr_f2l, ImgStruct, ERR_ILLEGAL_FUNCTION_CALL};

/// Result of the legacy `lineclip` (it used globals `lineclip_*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clip {
    pub draw: bool,
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
    /// Pixels from (x1, y1) that are not drawn; rotates the LINE style.
    pub skip: i32,
}

/// Clips a line to a rectangle the way libqb.cpp did (including its rounding
/// through `float`).
pub fn lineclip(
    mut x1: i32,
    mut y1: i32,
    x2: i32,
    y2: i32,
    xmin: i32,
    ymin: i32,
    xmax: i32,
    ymax: i32,
) -> Clip {
    let mut skip = 0;
    const NONE: Clip = Clip { draw: false, x1: 0, y1: 0, x2: 0, y2: 0, skip: 0 };
    let inside = |x: i32, y: i32| x >= xmin && x <= xmax && y >= ymin && y <= ymax;
    let skip_of = |ox: i32, oy: i32, nx: i32, ny: i32| {
        let xd = ox.wrapping_sub(nx).wrapping_abs();
        let yd = oy.wrapping_sub(ny).wrapping_abs();
        if xd >= yd {
            xd
        } else {
            yd
        }
    };

    let first_ok = 'first: {
        if inside(x1, y1) {
            if x1 == x2 && y1 == y2 {
                // single point
                return Clip { draw: true, x1, y1, x2, y2, skip: 0 };
            }
            break 'first true;
        }
        if x1 == x2 && y1 == y2 {
            return NONE;
        }
        if (x1 < xmin && x2 < xmin)
            || (x1 > xmax && x2 > xmax)
            || (y1 < ymin && y2 < ymin)
            || (y1 > ymax && y2 > ymax)
        {
            return NONE;
        }
        let mx = f64::from(x2.wrapping_sub(x1)) / f64::from(y2.wrapping_sub(y1)).abs();
        let my = f64::from(y2.wrapping_sub(y1)) / f64::from(x2.wrapping_sub(x1)).abs();
        if x1 > xmax && mx < 0.0 {
            let y = f64::from(y1) + (f64::from(x1) - f64::from(xmax)) * my;
            if y >= f64::from(ymin) && y <= f64::from(ymax) {
                let (ox, oy) = (x1, y1);
                x1 = xmax;
                y1 = qbr_f2l(y as f32);
                skip = skip_of(ox, oy, x1, y1);
                break 'first true;
            }
        }
        if x1 < xmin && mx > 0.0 {
            let y = f64::from(y1) + (f64::from(xmin) - f64::from(x1)) * my;
            if y >= f64::from(ymin) && y <= f64::from(ymax) {
                let (ox, oy) = (x1, y1);
                x1 = xmin;
                y1 = qbr_f2l(y as f32);
                skip = skip_of(ox, oy, x1, y1);
                break 'first true;
            }
        }
        if y1 < ymin && my > 0.0 {
            let x = f64::from(x1) + (f64::from(ymin) - f64::from(y1)) * mx;
            if x >= f64::from(xmin) && x <= f64::from(xmax) {
                let (ox, oy) = (x1, y1);
                x1 = qbr_f2l(x as f32);
                y1 = ymin;
                skip = skip_of(ox, oy, x1, y1);
                break 'first true;
            }
        }
        if y1 > ymax && my < 0.0 {
            let x = f64::from(x1) + (f64::from(y1) - f64::from(ymax)) * mx;
            if x >= f64::from(xmin) && x <= f64::from(xmax) {
                let (ox, oy) = (x1, y1);
                x1 = qbr_f2l(x as f32);
                y1 = ymax;
                skip = skip_of(ox, oy, x1, y1);
                break 'first true;
            }
        }
        false
    };
    if !first_ok {
        return NONE;
    }

    if inside(x2, y2) {
        return Clip { draw: true, x1, y1, x2, y2, skip };
    }
    let mx = f64::from(x1.wrapping_sub(x2)) / f64::from(y1.wrapping_sub(y2)).abs();
    let my = f64::from(y1.wrapping_sub(y2)) / f64::from(x1.wrapping_sub(x2)).abs();
    if x2 > xmax && mx < 0.0 {
        let y = f64::from(y2) + (f64::from(x2) - f64::from(xmax)) * my;
        if y >= f64::from(ymin) && y <= f64::from(ymax) {
            return Clip { draw: true, x1, y1, x2: xmax, y2: qbr_f2l(y as f32), skip };
        }
    }
    if x2 < xmin && mx > 0.0 {
        let y = f64::from(y2) + (f64::from(xmin) - f64::from(x2)) * my;
        if y >= f64::from(ymin) && y <= f64::from(ymax) {
            return Clip { draw: true, x1, y1, x2: xmin, y2: qbr_f2l(y as f32), skip };
        }
    }
    if y2 < ymin && my > 0.0 {
        let x = f64::from(x2) + (f64::from(ymin) - f64::from(y2)) * mx;
        if x >= f64::from(xmin) && x <= f64::from(xmax) {
            return Clip { draw: true, x1, y1, x2: qbr_f2l(x as f32), y2: ymin, skip };
        }
    }
    if y2 > ymax && my < 0.0 {
        let x = f64::from(x2) + (f64::from(y2) - f64::from(ymax)) * mx;
        if x >= f64::from(xmin) && x <= f64::from(xmax) {
            return Clip { draw: true, x1, y1, x2: qbr_f2l(x as f32), y2: ymax, skip };
        }
    }
    NONE
}

/// The DDA shared by `fast_line` and `qb32_line`; `plot` decides per pixel.
fn walk(c: Clip, mut plot: impl FnMut(i32, i32)) {
    let (mut x1, mut y1) = (c.x1, c.y1);
    let mut l = c.x1.wrapping_sub(c.x2).wrapping_abs();
    let mut l2 = c.y1.wrapping_sub(c.y2).wrapping_abs();
    let mut mi = 0;
    let mut m = 0f32;
    if l > l2 {
        let mut y1f = c.y1 as f32;
        if l != 0 {
            m = (c.y2 as f32 - c.y1 as f32) / l as f32;
            mi = if c.x2 >= c.x1 { 1 } else { -1 };
        }
        l += 1;
        while l > 0 {
            l -= 1;
            y1 = if y1f < 0.0 { cvt_i32(f64::from(y1f - 0.5)) } else { cvt_i32(f64::from(y1f + 0.5)) };
            plot(x1, y1);
            x1 += mi;
            y1f += m;
        }
    } else {
        let mut x1f = c.x1 as f32;
        if l2 != 0 {
            m = (c.x2 as f32 - c.x1 as f32) / l2 as f32;
            mi = if c.y2 >= c.y1 { 1 } else { -1 };
        }
        l2 += 1;
        while l2 > 0 {
            l2 -= 1;
            x1 = if x1f < 0.0 { cvt_i32(f64::from(x1f - 0.5)) } else { cvt_i32(f64::from(x1f + 0.5)) };
            plot(x1, y1);
            y1 += mi;
            x1f += m;
        }
    }
}

/// Legacy `fast_line`: already mapped integer coordinates, no style.
///
/// # Safety
/// `im` must be a valid graphics surface whose VIEW lies inside it.
pub unsafe fn fast_line(im: &ImgStruct, x1: i32, y1: i32, x2: i32, y2: i32, col: u32) {
    let c = lineclip(x1, y1, x2, y2, im.view_x1, im.view_y1, im.view_x2, im.view_y2);
    if c.draw {
        // SAFETY: lineclip keeps every plotted point inside the view.
        walk(c, |x, y| unsafe { pset(im, x, y, col) });
    }
}

/// Legacy `qb32_line`: WINDOW/VIEW-mapped float coordinates with a style mask.
///
/// # Safety
/// `im` must be a valid graphics surface whose VIEW lies inside it.
pub unsafe fn qb32_line(im: &ImgStruct, x1f: f32, y1f: f32, x2f: f32, y2f: f32, col: u32, style: u32) {
    let (x1, y1) = map_point_f2l(im, x1f, y1f);
    let (x2, y2) = map_point_f2l(im, x2f, y2f);
    let c = lineclip(x1, y1, x2, y2, im.view_x1, im.view_y1, im.view_x2, im.view_y2);
    let mut style = (style & 65535).wrapping_add(style << 16);
    style = style.rotate_left((c.skip & 15) as u32);
    if c.draw {
        walk(c, |x, y| {
            style = style.rotate_left(1);
            if style & 1 != 0 {
                // SAFETY: lineclip keeps every plotted point inside the view.
                unsafe { pset(im, x, y, col) };
            }
        });
    }
}

/// Legacy `fast_boxfill`: on-screen, ordered integer coordinates.
///
/// # Safety
/// The rectangle must lie inside `im`.
pub unsafe fn fast_boxfill(im: &ImgStruct, x1: i32, y1: i32, x2: i32, y2: i32, col: u32) {
    let w = i32::from(im.width);
    // SAFETY: guaranteed by the caller.
    unsafe {
        if im.bytes_per_pixel == 1 {
            let c = (col & im.mask) as u8;
            let width = (x2 - x1 + 1) as usize;
            for y in y1..=y2 {
                core::ptr::write_bytes(im.offset.offset((y * w + x1) as isize), c, width);
            }
            return;
        }
        let a = col >> 24;
        if im.alpha_disabled != 0 || a == 255 {
            for y in y1..=y2 {
                let row = im.offset32().offset((y * w) as isize);
                for x in x1..=x2 {
                    *row.offset(x as isize) = col;
                }
            }
            return;
        }
        if a == 0 {
            return;
        }
        for y in y1..=y2 {
            let row = im.offset32().offset((y * w) as isize);
            for x in x1..=x2 {
                blend_onto(row.offset(x as isize), col);
            }
        }
    }
}

/// Legacy `qb32_boxfill`: maps, orders, clips to the VIEW, then fills.
///
/// # Safety
/// `im` must be a valid graphics surface whose VIEW lies inside it.
pub unsafe fn qb32_boxfill(im: &ImgStruct, x1f: f32, y1f: f32, x2f: f32, y2f: f32, col: u32) {
    let (mut x1, mut y1) = map_point_f2l(im, x1f, y1f);
    let (mut x2, mut y2) = map_point_f2l(im, x2f, y2f);
    if x1 > x2 {
        core::mem::swap(&mut x1, &mut x2);
    }
    if y1 > y2 {
        core::mem::swap(&mut y1, &mut y2);
    }
    if x2 < im.view_x1 || x1 > im.view_x2 || y2 < im.view_y1 || y1 > im.view_y2 {
        return;
    }
    x1 = x1.max(im.view_x1);
    y1 = y1.max(im.view_y1);
    x2 = x2.min(im.view_x2);
    y2 = y2.min(im.view_y2);
    // SAFETY: clipped to the view.
    unsafe { fast_boxfill(im, x1, y1, x2, y2, col) }
}

/// `LINE` statement (`sub_line`). `passed`: 1 = first point, 2 = STEP on the
/// second point, 4 = STEP on the first point, 8 = color, 16 = style.
/// `bf`: 0 line, 1 B, 2 BF. Returns a QBasic error code.
///
/// # Safety
/// `im` must be a valid surface whose VIEW lies inside it.
pub unsafe fn sub_line(
    im: &mut ImgStruct,
    mut x1: f32,
    mut y1: f32,
    mut x2: f32,
    mut y2: f32,
    mut col: u32,
    bf: i32,
    mut style: u32,
    passed: i32,
) -> i32 {
    if im.text != 0 {
        return ERR_ILLEGAL_FUNCTION_CALL;
    }
    if passed & 1 != 0 {
        if passed & 4 != 0 {
            x1 += im.x;
            y1 += im.y;
        }
        im.x = x1;
        im.y = y1;
    } else {
        x1 = im.x;
        y1 = im.y;
    }
    if passed & 2 != 0 {
        x2 += im.x;
        y2 += im.y;
    }
    im.x = x2;
    im.y = y2;
    if passed & 16 == 0 {
        style = 0xFFFF;
    }
    if passed & 8 == 0 {
        col = im.color;
    }
    // SAFETY (all calls below): guaranteed by the caller.
    unsafe {
        match bf {
            0 => {
                im.draw_color = col;
                qb32_line(im, x1, y1, x2, y2, col, style);
            }
            1 => {
                im.draw_color = col;
                qb32_line(im, x1, y1, x2, y1, col, style);
                qb32_line(im, x2, y1, x2, y2, col, style);
                qb32_line(im, x2, y2, x1, y2, col, style);
                qb32_line(im, x1, y2, x1, y1, col, style);
            }
            2 => {
                im.draw_color = col;
                qb32_boxfill(im, x1, y1, x2, y2, col);
            }
            _ => {}
        }
    }
    0
}

/// C ABI: `sub_line` on `write_page`.
///
/// # Safety
/// `im` must be `write_page`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_gfx_line(
    im: *mut ImgStruct,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    col: u32,
    bf: i32,
    style: u32,
    passed: i32,
) -> i32 {
    // SAFETY: guaranteed by the caller.
    unsafe { sub_line(&mut *im, x1, y1, x2, y2, col, bf, style, passed) }
}

/// C ABI for C++ callers of `fast_boxfill`/`qb32_boxfill`/`fast_line`.
///
/// # Safety
/// See the Rust functions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_gfx_boxfill(im: *const ImgStruct, x1: f32, y1: f32, x2: f32, y2: f32, col: u32) {
    // SAFETY: guaranteed by the caller.
    unsafe { qb32_boxfill(&*im, x1, y1, x2, y2, col) }
}

/// # Safety
/// The rectangle must lie inside `im`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_gfx_fast_boxfill(im: *const ImgStruct, x1: i32, y1: i32, x2: i32, y2: i32, col: u32) {
    // SAFETY: guaranteed by the caller.
    unsafe { fast_boxfill(&*im, x1, y1, x2, y2, col) }
}

/// # Safety
/// `im` must be a valid surface whose VIEW lies inside it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_gfx_fast_line(im: *const ImgStruct, x1: i32, y1: i32, x2: i32, y2: i32, col: u32) {
    // SAFETY: guaranteed by the caller.
    unsafe { fast_line(&*im, x1, y1, x2, y2, col) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::testutil::TestImage;

    #[test]
    fn clip_inside_and_outside() {
        let c = lineclip(1, 1, 5, 5, 0, 0, 9, 9);
        assert_eq!((c.draw, c.x1, c.y1, c.x2, c.y2, c.skip), (true, 1, 1, 5, 5, 0));
        assert!(!lineclip(-5, -5, -1, -1, 0, 0, 9, 9).draw);
        assert!(!lineclip(20, 20, 20, 20, 0, 0, 9, 9).draw);
        let c = lineclip(-4, 2, 4, 2, 0, 0, 9, 9);
        assert_eq!((c.draw, c.x1, c.y1, c.x2, c.y2, c.skip), (true, 0, 2, 4, 2, 4));
        let c = lineclip(2, 3, 30, 3, 0, 0, 9, 9);
        assert_eq!((c.x2, c.y2), (9, 3));
    }

    #[test]
    fn line_horizontal_vertical_diagonal() {
        let mut t = TestImage::new(8, 8, 13);
        unsafe { sub_line(&mut t.img, 1.0, 1.0, 4.0, 1.0, 7, 0, 0, 1 | 8) };
        assert_eq!((1..=4).map(|x| t.p8(x, 1)).collect::<Vec<_>>(), vec![7; 4]);
        assert_eq!(t.p8(5, 1), 0);
        unsafe { sub_line(&mut t.img, 0.0, 0.0, 7.0, 7.0, 3, 0, 0, 1 | 8) };
        assert!((0..8).all(|i| t.p8(i, i) == 3));
        assert_eq!((t.img.x, t.img.y), (7.0, 7.0));
        assert_eq!(t.img.draw_color, 3);
    }

    #[test]
    fn line_defaults_color_and_step() {
        let mut t = TestImage::new(8, 8, 13);
        t.img.color = 9;
        t.img.x = 2.0;
        t.img.y = 2.0;
        // LINE -STEP(2, 0)
        unsafe { sub_line(&mut t.img, 0.0, 0.0, 2.0, 0.0, 0, 0, 0, 2) };
        assert_eq!((t.p8(2, 2), t.p8(3, 2), t.p8(4, 2)), (9, 9, 9));
        assert_eq!(t.img.x, 4.0);
    }

    #[test]
    fn line_style_and_boxes() {
        let mut t = TestImage::new(16, 8, 13);
        // style &HAAAA: every other pixel, starting with the second
        unsafe { sub_line(&mut t.img, 0.0, 0.0, 7.0, 0.0, 5, 0, 0xAAAA, 1 | 8 | 16) };
        assert_eq!((0..8).map(|x| t.p8(x, 0)).collect::<Vec<_>>(), vec![5, 0, 5, 0, 5, 0, 5, 0]);
        unsafe { sub_line(&mut t.img, 2.0, 2.0, 5.0, 5.0, 4, 1, 0, 1 | 8) };
        assert_eq!((t.p8(2, 2), t.p8(5, 5), t.p8(3, 3)), (4, 4, 0));
        unsafe { sub_line(&mut t.img, 10.0, 6.0, 8.0, 2.0, 6, 2, 0, 1 | 8) };
        assert!((8..=10).all(|x| (2..=6).all(|y| t.p8(x, y) == 6)));
    }

    #[test]
    fn line_in_text_mode_is_an_error() {
        let mut t = TestImage::new(8, 8, 0);
        assert_eq!(unsafe { sub_line(&mut t.img, 0.0, 0.0, 1.0, 1.0, 1, 0, 0, 1) }, 5);
    }

    #[test]
    fn boxfill_clips_to_view_and_blends() {
        let mut t = TestImage::new(4, 4, 32);
        t.img.view_x1 = 1;
        t.img.view_y1 = 1;
        unsafe { qb32_boxfill(&t.img, -5.0, -5.0, 10.0, 10.0, 0xFF11_2233) };
        assert_eq!(t.p32(0, 0), 0);
        assert_eq!(t.p32(3, 3), 0xFF11_2233);
        unsafe { qb32_boxfill(&t.img, 0.0, 0.0, 3.0, 3.0, 0x0011_2233) };
        assert_eq!(t.p32(3, 3), 0xFF11_2233);
    }
}
