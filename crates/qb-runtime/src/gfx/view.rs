//! WINDOW, VIEW and PMAP (`qbg_sub_window`, `qbg_sub_view`, `func_pmap`).

use super::blend::pset;
use super::line::qb32_boxfill;
use super::{qbr_f2l, ImgStruct, ERR_ILLEGAL_FUNCTION_CALL};

/// `WINDOW [[SCREEN] (x1, y1)-(x2, y2)]`; passed: 1 = coordinates, 2 = SCREEN.
///
/// # Safety
/// `im` must be a valid surface.
pub unsafe fn window(im: &mut ImgStruct, mut x1: f32, mut y1: f32, mut x2: f32, mut y2: f32, passed: i32) -> i32 {
    const ERR: i32 = ERR_ILLEGAL_FUNCTION_CALL;
    if im.text != 0 {
        return ERR;
    }
    if passed & 1 == 0 && passed & 2 != 0 {
        return ERR;
    }
    // graphics cursor relative to the viewport, not the window
    let (old_x, old_y) = if im.clipping_or_scaling == 2 {
        (im.x * im.scaling_x + im.scaling_offset_x, im.y * im.scaling_y + im.scaling_offset_y)
    } else {
        (im.x, im.y)
    };
    let mut restore = passed & 1 == 0;
    if !restore {
        if x1 == x2 || y1 == y2 {
            return ERR;
        }
        if x1 > x2 {
            core::mem::swap(&mut x1, &mut x2);
        }
        if y1 > y2 {
            core::mem::swap(&mut y1, &mut y2);
        }
        if passed & 2 == 0 {
            core::mem::swap(&mut y1, &mut y2);
        }
        im.clipping_or_scaling = 2;
        im.scaling_x = (im.view_x2 - im.view_x1) as f32 / (x2 - x1);
        im.scaling_y = (im.view_y2 - im.view_y1) as f32 / (y2 - y1);
        im.scaling_offset_x = -x1 * im.scaling_x;
        im.scaling_offset_y = -y1 * im.scaling_y;
        if passed & 2 == 0 {
            im.scaling_offset_y = -y2 * im.scaling_y + (im.view_y2 - im.view_y1) as f32;
        }
        im.window_x1 = x1;
        im.window_x2 = x2;
        im.window_y1 = y1;
        im.window_y2 = y2;
        if x1 == 0.0
            && y1 == 0.0
            && x2 == (i32::from(im.width) - 1) as f32
            && y2 == (i32::from(im.height) - 1) as f32
            && im.scaling_x == 1.0
            && im.scaling_y == 1.0
            && im.scaling_offset_x == 0.0
            && im.scaling_offset_y == 0.0
        {
            restore = true;
        } else {
            im.x = (old_x - im.scaling_offset_x) / im.scaling_x;
            im.y = (old_y - im.scaling_offset_y) / im.scaling_y;
            return 0;
        }
    }
    if restore {
        im.clipping_or_scaling = 1;
        im.scaling_x = 1.0;
        im.scaling_y = 1.0;
        im.scaling_offset_x = 0.0;
        im.scaling_offset_y = 0.0;
        im.window_x1 = 0.0;
        im.window_x2 = (i32::from(im.width) - 1) as f32;
        im.window_y1 = 0.0;
        im.window_y2 = (i32::from(im.height) - 1) as f32;
        if im.view_x1 == 0
            && im.view_y1 == 0
            && im.view_x2 == i32::from(im.width) - 1
            && im.view_y2 == i32::from(im.height) - 1
            && im.view_offset_x == 0
            && im.view_offset_y == 0
        {
            im.clipping_or_scaling = 0;
        }
        im.x = old_x;
        im.y = old_y;
    }
    0
}

/// `VIEW [[SCREEN] (x1, y1)-(x2, y2)[, fill][, border]]`;
/// passed: 1 = coordinates, 2 = SCREEN, 4 = fill, 8 = border.
///
/// # Safety
/// `im` must be a valid surface.
pub unsafe fn view(
    im: &mut ImgStruct,
    mut x1: i32,
    mut y1: i32,
    mut x2: i32,
    mut y2: i32,
    fillcolor: i32,
    bordercolor: i32,
    passed: i32,
) -> i32 {
    const ERR: i32 = ERR_ILLEGAL_FUNCTION_CALL;
    let w = i32::from(im.width);
    let h = i32::from(im.height);
    if passed & 1 != 0 {
        if x1 < 0 || x1 >= w || y1 < 0 || y1 >= h || x2 < 0 || x2 >= w || y2 < 0 || y2 >= h {
            return ERR;
        }
    } else if passed & (2 | 4 | 8) != 0 {
        return ERR;
    }
    im.draw_ta = 0.0;
    im.draw_scale = 1.0;
    if passed & 1 != 0 {
        if x2 < x1 {
            core::mem::swap(&mut x1, &mut x2);
        }
        if y2 < y1 {
            core::mem::swap(&mut y1, &mut y2);
        }
        im.view_x1 = x1;
        im.view_y1 = y1;
        im.view_x2 = x2;
        im.view_y2 = y2;
        if passed & 2 == 0 {
            im.view_offset_x = x1;
            im.view_offset_y = y1;
        } else {
            im.view_offset_x = 0;
            im.view_offset_y = 0;
        }
        if im.clipping_or_scaling == 0 {
            im.clipping_or_scaling = 1;
        }
    } else {
        im.view_x1 = 0;
        im.view_y1 = 0;
        im.view_x2 = w - 1;
        im.view_y2 = h - 1;
        im.view_offset_x = 0;
        im.view_offset_y = 0;
        if im.clipping_or_scaling == 1 {
            im.clipping_or_scaling = 0;
        }
    }
    if im.clipping_or_scaling == 2 {
        im.scaling_x = (im.view_x2 - im.view_x1) as f32 / (im.window_x2 - im.window_x1);
        im.scaling_y = (im.view_y2 - im.view_y1) as f32 / (im.window_y2 - im.window_y1);
        im.scaling_offset_x = -im.window_x1 * im.scaling_x;
        im.scaling_offset_y = -im.window_y1 * im.scaling_y;
        if im.window_y2 < im.window_y1 {
            im.scaling_offset_y = -im.window_y2 * im.scaling_y + im.view_y2 as f32;
        }
    }
    if passed & 4 != 0 {
        // SAFETY: the view lies inside the surface.
        unsafe { qb32_boxfill(im, im.window_x1, im.window_y1, im.window_x2, im.window_y2, fillcolor as u32) };
    }
    if passed & 8 != 0 {
        let c = bordercolor as u32;
        let inside = |x: i32, y: i32| x >= 0 && x < w && y >= 0 && y < h;
        // SAFETY (all psets): `inside` checked.
        for (by, x_end) in [(im.view_y1 - 1, im.view_x2), (im.view_y2 + 1, im.view_x2)] {
            if by >= 0 && by < h {
                for bx in im.view_x1 - 1..=x_end {
                    if inside(bx, by) {
                        unsafe { pset(im, bx, by, c) };
                    }
                }
            }
        }
        for (bx, y_end) in [(im.view_x1 - 1, im.view_y2), (im.view_x2 + 1, im.view_y2 + 1)] {
            if bx >= 0 && bx < w {
                for by in im.view_y1 - 1..=y_end {
                    if inside(bx, by) {
                        unsafe { pset(im, bx, by, c) };
                    }
                }
            }
        }
    }
    0
}

/// `PMAP(value, option)`. Returns (value, error code).
pub fn pmap(im: &ImgStruct, val: f32, option: i32) -> (f32, i32) {
    if im.text == 0 {
        // QBasic compatibility: clipping_or_scaling is deliberately ignored.
        match option {
            0 => return (qbr_f2l(val * im.scaling_x + im.scaling_offset_x) as f32, 0),
            1 => return (qbr_f2l(val * im.scaling_y + im.scaling_offset_y) as f32, 0),
            2 => {
                return (
                    ((f64::from(qbr_f2l(val)) - f64::from(im.scaling_offset_x)) / f64::from(im.scaling_x)) as f32,
                    0,
                )
            }
            3 => {
                return (
                    ((f64::from(qbr_f2l(val)) - f64::from(im.scaling_offset_y)) / f64::from(im.scaling_y)) as f32,
                    0,
                )
            }
            _ => {}
        }
    }
    (0.0, ERR_ILLEGAL_FUNCTION_CALL)
}

// --- C ABI ---------------------------------------------------------------------

/// # Safety
/// `im` must be `write_page`.
#[no_mangle]
pub unsafe extern "C" fn qb_gfx_window(im: *mut ImgStruct, x1: f32, y1: f32, x2: f32, y2: f32, passed: i32) -> i32 {
    // SAFETY: guaranteed by the caller.
    unsafe { window(&mut *im, x1, y1, x2, y2, passed) }
}

/// # Safety
/// `im` must be `write_page`.
#[no_mangle]
pub unsafe extern "C" fn qb_gfx_view(
    im: *mut ImgStruct,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    fillcolor: i32,
    bordercolor: i32,
    passed: i32,
) -> i32 {
    // SAFETY: guaranteed by the caller.
    unsafe { view(&mut *im, x1, y1, x2, y2, fillcolor, bordercolor, passed) }
}

/// # Safety
/// `im` must be `write_page`; `err` writable.
#[no_mangle]
pub unsafe extern "C" fn qb_gfx_pmap(im: *const ImgStruct, val: f32, option: i32, err: *mut i32) -> f32 {
    // SAFETY: guaranteed by the caller.
    unsafe {
        let (v, e) = pmap(&*im, val, option);
        *err = e;
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::testutil::TestImage;

    #[test]
    fn window_maps_and_restores() {
        let mut t = TestImage::new(320, 200, 13);
        // WINDOW (0,0)-(1,1): y axis flipped
        assert_eq!(unsafe { window(&mut t.img, 0.0, 0.0, 1.0, 1.0, 1) }, 0);
        assert_eq!(t.img.clipping_or_scaling, 2);
        assert_eq!((t.img.scaling_x, t.img.scaling_y), (319.0, -199.0));
        assert_eq!((t.img.scaling_offset_x, t.img.scaling_offset_y), (0.0, 199.0));
        assert_eq!((t.img.window_y1, t.img.window_y2), (1.0, 0.0));
        assert_eq!(pmap(&t.img, 1.0, 1).0, 0.0);
        assert_eq!(pmap(&t.img, 0.0, 1).0, 199.0);
        // WINDOW SCREEN (0,0)-(1,1): y down
        unsafe { window(&mut t.img, 0.0, 0.0, 1.0, 1.0, 3) };
        assert_eq!(pmap(&t.img, 1.0, 1).0, 199.0);
        // WINDOW: back to pixels
        unsafe { window(&mut t.img, 0.0, 0.0, 0.0, 0.0, 0) };
        assert_eq!(t.img.clipping_or_scaling, 0);
        assert_eq!(unsafe { window(&mut t.img, 1.0, 0.0, 1.0, 5.0, 1) }, 5);
        assert_eq!(unsafe { window(&mut t.img, 0.0, 0.0, 0.0, 0.0, 2) }, 5);
    }

    #[test]
    fn view_sets_clip_and_offset() {
        let mut t = TestImage::new(320, 200, 13);
        assert_eq!(unsafe { view(&mut t.img, 50, 40, 10, 20, 3, 9, 1 | 4 | 8) }, 0);
        assert_eq!((t.img.view_x1, t.img.view_y1, t.img.view_x2, t.img.view_y2), (10, 20, 50, 40));
        assert_eq!((t.img.view_offset_x, t.img.view_offset_y, t.img.clipping_or_scaling), (10, 20, 1));
        assert_eq!(t.p8(10, 20), 3); // filled
        assert_eq!(t.p8(9, 19), 9); // border outside the view
        assert_eq!(t.p8(51, 41), 9);
        unsafe { view(&mut t.img, 0, 0, 0, 0, 0, 0, 0) };
        assert_eq!((t.img.view_x2, t.img.clipping_or_scaling), (319, 0));
        assert_eq!(unsafe { view(&mut t.img, 0, 0, 320, 0, 0, 0, 1) }, 5);
        assert_eq!(unsafe { view(&mut t.img, 0, 0, 0, 0, 0, 0, 4) }, 5);
    }

    #[test]
    fn view_screen_keeps_absolute_coordinates() {
        let mut t = TestImage::new(320, 200, 13);
        unsafe { view(&mut t.img, 10, 10, 20, 20, 0, 0, 1 | 2) };
        assert_eq!((t.img.view_offset_x, t.img.view_offset_y), (0, 0));
    }

    #[test]
    fn pmap_options() {
        let mut t = TestImage::new(320, 200, 13);
        assert_eq!(pmap(&t.img, 5.0, 0), (5.0, 0));
        assert_eq!(pmap(&t.img, 5.0, 2), (5.0, 0));
        assert_eq!(pmap(&t.img, 5.0, 4).1, 5);
        unsafe { window(&mut t.img, 0.0, 0.0, 319.0 * 2.0, 199.0 * 2.0, 3) };
        assert_eq!(pmap(&t.img, 100.0, 0).0, 50.0);
        assert_eq!(pmap(&t.img, 50.0, 2).0, 100.0);
    }
}
