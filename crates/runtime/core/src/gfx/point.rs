//! PSET, PRESET and POINT (`sub_pset`, `sub_preset`, `point`, `func_point`).

use super::blend::pset;
use super::{map_point_f2l, qbr_f2l, qbr_i32, ImgStruct, ERR_ILLEGAL_FUNCTION_CALL};

/// `PSET [STEP](x, y)[, color]`; passed: 1 = STEP, 2 = color.
///
/// # Safety
/// `im` must be a valid surface whose VIEW lies inside it.
pub unsafe fn sub_pset(im: &mut ImgStruct, x: f32, y: f32, mut col: u32, passed: i32) -> i32 {
    if im.compatible_mode == 0 {
        return ERR_ILLEGAL_FUNCTION_CALL;
    }
    if passed & 1 != 0 {
        im.x += x;
        im.y += y;
    } else {
        im.x = x;
        im.y = y;
    }
    if passed & 2 == 0 {
        col = im.color;
    }
    im.draw_color = col;
    // Unlike LINE/PAINT, PSET rounds with qbr (64-bit), truncated to int32.
    if im.clipping_or_scaling != 0 {
        let (x2, y2) = if im.clipping_or_scaling == 2 {
            (
                qbr_i32(f64::from(im.x * im.scaling_x + im.scaling_offset_x)).wrapping_add(im.view_offset_x),
                qbr_i32(f64::from(im.y * im.scaling_y + im.scaling_offset_y)).wrapping_add(im.view_offset_y),
            )
        } else {
            (
                qbr_i32(f64::from(im.x)).wrapping_add(im.view_offset_x),
                qbr_i32(f64::from(im.y)).wrapping_add(im.view_offset_y),
            )
        };
        if x2 >= im.view_x1 && x2 <= im.view_x2 && y2 >= im.view_y1 && y2 <= im.view_y2 {
            // SAFETY: inside the view.
            unsafe { pset(im, x2, y2, col) };
        }
    } else {
        let x2 = qbr_i32(f64::from(im.x));
        if x2 >= 0 && x2 < i32::from(im.width) {
            let y2 = qbr_i32(f64::from(im.y));
            if y2 >= 0 && y2 < i32::from(im.height) {
                // SAFETY: inside the image.
                unsafe { pset(im, x2, y2, col) };
            }
        }
    }
    0
}

/// `PRESET`: PSET with the background color as default.
///
/// # Safety
/// As [`sub_pset`].
pub unsafe fn sub_preset(im: &mut ImgStruct, x: f32, y: f32, mut col: u32, mut passed: i32) -> i32 {
    if passed & 2 == 0 {
        col = im.background_color;
        passed |= 2;
    }
    // SAFETY: guaranteed by the caller.
    unsafe { sub_pset(im, x, y, col, passed) }
}

/// Legacy `point(x, y)`: unclipped pixel read.
///
/// # Safety
/// `(x, y)` must lie inside `im`.
pub unsafe fn point(im: &ImgStruct, x: i32, y: i32) -> u32 {
    let idx = (y * i32::from(im.width) + x) as isize;
    // SAFETY: guaranteed by the caller.
    unsafe {
        if im.bytes_per_pixel == 1 {
            u32::from(*im.offset.offset(idx)) & im.mask
        } else {
            *im.offset32().offset(idx)
        }
    }
}

/// `POINT(x, y)` (passed != 0, reads `read`) or `POINT(n)` (passed == 0,
/// graphics cursor of `write`). Returns (value, error code).
///
/// # Safety
/// Both surfaces must be valid.
pub unsafe fn func_point(write: &ImgStruct, read: &ImgStruct, x: f32, y: f32, passed: i32) -> (f64, i32) {
    const ERR: (f64, i32) = (0.0, ERR_ILLEGAL_FUNCTION_CALL);
    if passed == 0 {
        if write.text != 0 {
            return ERR;
        }
        let v = match qbr_f2l(x) {
            0 if write.clipping_or_scaling == 2 => qbr_f2l(write.x * write.scaling_x + write.scaling_offset_x) as f64,
            0 => qbr_f2l(write.x) as f64,
            1 if write.clipping_or_scaling == 2 => qbr_f2l(write.y * write.scaling_y + write.scaling_offset_y) as f64,
            1 => qbr_f2l(write.y) as f64,
            2 => f64::from(write.x),
            3 => f64::from(write.y),
            _ => return ERR,
        };
        return (v, 0);
    }
    if read.text != 0 {
        return ERR;
    }
    let (x2, y2) = map_point_f2l(read, x, y);
    if x2 >= read.view_x1 && x2 <= read.view_x2 && y2 >= read.view_y1 && y2 <= read.view_y2 {
        // SAFETY: inside the view.
        return (f64::from(unsafe { point(read, x2, y2) }), 0);
    }
    (-1.0, 0)
}

// --- entry points ----------------------------------------------------------------

/// # Safety
/// `im` must be `write_page`.
pub unsafe fn qb_gfx_pset_stmt(im: *mut ImgStruct, x: f32, y: f32, col: u32, passed: i32) -> i32 {
    // SAFETY: guaranteed by the caller.
    unsafe { sub_pset(&mut *im, x, y, col, passed) }
}

/// # Safety
/// `im` must be `write_page`.
pub unsafe fn qb_gfx_preset_stmt(im: *mut ImgStruct, x: f32, y: f32, col: u32, passed: i32) -> i32 {
    // SAFETY: guaranteed by the caller.
    unsafe { sub_preset(&mut *im, x, y, col, passed) }
}

/// # Safety
/// `(x, y)` must lie inside `im`.
pub unsafe fn qb_gfx_point_raw(im: *const ImgStruct, x: i32, y: i32) -> u32 {
    // SAFETY: guaranteed by the caller.
    unsafe { point(&*im, x, y) }
}

/// # Safety
/// `write`/`read` must be `write_page`/`read_page`; `err` writable.
pub unsafe fn qb_gfx_point(
    write: *const ImgStruct,
    read: *const ImgStruct,
    x: f32,
    y: f32,
    passed: i32,
    err: *mut i32,
) -> f64 {
    // SAFETY: guaranteed by the caller.
    unsafe {
        let (v, e) = func_point(&*write, &*read, x, y, passed);
        *err = e;
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::testutil::TestImage;

    #[test]
    fn pset_step_color_and_cursor() {
        let mut t = TestImage::new(8, 8, 13);
        assert_eq!(unsafe { sub_pset(&mut t.img, 2.0, 3.0, 9, 2) }, 0);
        assert_eq!(t.p8(2, 3), 9);
        assert_eq!((t.img.x, t.img.y, t.img.draw_color), (2.0, 3.0, 9));
        t.img.color = 4;
        unsafe { sub_pset(&mut t.img, 1.0, 1.0, 0, 1) }; // STEP, default color
        assert_eq!(t.p8(3, 4), 4);
        unsafe { sub_pset(&mut t.img, 2.5, 3.5, 7, 2) }; // banker's rounding: (2, 4)
        assert_eq!(t.p8(2, 4), 7);
        unsafe { sub_pset(&mut t.img, -1.0, 100.0, 7, 2) }; // off-screen: ignored
    }

    #[test]
    fn pset_text_mode_error_and_preset() {
        let mut t = TestImage::new(8, 8, 0);
        assert_eq!(unsafe { sub_pset(&mut t.img, 0.0, 0.0, 1, 2) }, 5);
        let mut t = TestImage::new(8, 8, 13);
        t.img.background_color = 6;
        unsafe { sub_preset(&mut t.img, 1.0, 1.0, 0, 0) };
        assert_eq!(t.p8(1, 1), 6);
        unsafe { sub_preset(&mut t.img, 1.0, 1.0, 3, 2) };
        assert_eq!(t.p8(1, 1), 3);
    }

    #[test]
    fn pset_respects_view() {
        let mut t = TestImage::new(8, 8, 13);
        t.img.clipping_or_scaling = 1;
        t.img.view_x1 = 2;
        t.img.view_offset_x = 2;
        unsafe { sub_pset(&mut t.img, 0.0, 0.0, 5, 2) };
        assert_eq!(t.p8(2, 0), 5);
        unsafe { sub_pset(&mut t.img, -1.0, 0.0, 5, 2) };
        assert_eq!(t.p8(1, 0), 0);
    }

    #[test]
    fn point_reads_and_clips() {
        let mut t = TestImage::new(8, 8, 13);
        unsafe { sub_pset(&mut t.img, 4.0, 5.0, 77, 2) };
        let r = TestImage::new(1, 1, 13);
        let (v, e) = unsafe { func_point(&r.img, &t.img, 4.0, 5.0, 1) };
        assert_eq!((v, e), (77.0, 0));
        assert_eq!(unsafe { func_point(&r.img, &t.img, 9.0, 5.0, 1) }, (-1.0, 0));
        let mut t32 = TestImage::new(2, 2, 32);
        t32.pixels[3] = 0xFF12_3456;
        assert_eq!(unsafe { func_point(&r.img, &t32.img, 1.0, 1.0, 1) }, (f64::from(0xFF12_3456u32), 0));
    }

    #[test]
    fn point_cursor_forms() {
        let mut w = TestImage::new(8, 8, 13);
        w.img.x = 2.5;
        w.img.y = 3.25;
        let r = TestImage::new(1, 1, 13);
        assert_eq!(unsafe { func_point(&w.img, &r.img, 0.0, 0.0, 0) }, (2.0, 0));
        assert_eq!(unsafe { func_point(&w.img, &r.img, 1.0, 0.0, 0) }, (3.0, 0));
        assert_eq!(unsafe { func_point(&w.img, &r.img, 2.0, 0.0, 0) }, (2.5, 0));
        assert_eq!(unsafe { func_point(&w.img, &r.img, 3.0, 0.0, 0) }, (3.25, 0));
        assert_eq!(unsafe { func_point(&w.img, &r.img, 4.0, 0.0, 0) }.1, 5);
    }
}
