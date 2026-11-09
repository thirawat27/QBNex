//! CIRCLE (`sub_circle`): circles, ellipses, arcs and pie slices.

use super::blend::pset_and_clip;
use super::line::fast_line;
use super::{cvt_i32, ImgStruct, ERR_ILLEGAL_FUNCTION_CALL};

/// `CIRCLE [STEP](x, y), r[, color[, start[, end[, aspect]]]]`;
/// passed: 1 = STEP, 2 = color, 4 = start, 8 = end, 16 = aspect.
///
/// # Safety
/// `im` must be a valid surface whose VIEW lies inside it.
// The aspect expressions copy the legacy arithmetic; the one-shot loop is
// the legacy `plot:` goto target.
#[allow(clippy::eq_op, clippy::never_loop)]
pub unsafe fn sub_circle(
    im: &mut ImgStruct,
    mut x: f64,
    mut y: f64,
    mut r: f64,
    mut col: u32,
    mut start: f64,
    mut end: f64,
    mut aspect: f64,
    passed: i32,
) -> i32 {
    // legacy literals 3.1415926535897932 / 6.2831853071795865: same bits
    const PI: f64 = std::f64::consts::PI;
    const PI2: f64 = std::f64::consts::TAU;
    if im.text != 0 {
        return ERR_ILLEGAL_FUNCTION_CALL;
    }
    if passed & 4 == 0 {
        start = 0.0;
    }
    if passed & 8 == 0 {
        end = PI2;
    }
    let mut line_to_start = false;
    if start < 0.0 {
        line_to_start = true;
        start = -start;
    }
    let mut line_from_end = false;
    if end < 0.0 {
        line_from_end = true;
        end = -end;
    }
    if start > PI2 || end > PI2 {
        return ERR_ILLEGAL_FUNCTION_CALL;
    }
    // end < start draws the part that start..end would not have drawn
    let mut exclusive = false;
    if end < start {
        core::mem::swap(&mut start, &mut end);
        core::mem::swap(&mut line_to_start, &mut line_from_end);
        exclusive = true;
    }
    if passed & 1 != 0 {
        x += f64::from(im.x);
        y += f64::from(im.y);
    }
    im.x = x as f32;
    im.y = y as f32;
    r += x; // radius measured in x pixels after mapping
    if im.clipping_or_scaling != 0 {
        if im.clipping_or_scaling == 2 {
            x = x * f64::from(im.scaling_x) + f64::from(im.scaling_offset_x) + f64::from(im.view_offset_x);
            y = y * f64::from(im.scaling_y) + f64::from(im.scaling_offset_y) + f64::from(im.view_offset_y);
            r = r * f64::from(im.scaling_x) + f64::from(im.scaling_offset_x) + f64::from(im.view_offset_x);
        } else {
            x += f64::from(im.view_offset_x);
            y += f64::from(im.view_offset_y);
            r += f64::from(im.view_offset_x);
        }
    }
    let ix = if x < 0.0 { cvt_i32(x - 0.5) } else { cvt_i32(x + 0.5) };
    let iy = if y < 0.0 { cvt_i32(y - 0.5) } else { cvt_i32(y + 0.5) };
    r = (r - x).abs();
    let mut xspan = r;
    let mut yspan = r;
    if passed & 16 == 0 {
        aspect = match im.compatible_mode {
            1 | 7 | 13 => 4.0 * (200.0 / 320.0) / 3.0,
            2 | 8 => 4.0 * (200.0 / 640.0) / 3.0,
            9 | 10 => 4.0 * (350.0 / 640.0) / 3.0,
            11 | 12 => 4.0 * (480.0 / 640.0) / 3.0,
            _ => 1.0,
        };
    }
    if aspect >= 0.0 {
        if aspect < 1.0 {
            yspan *= aspect;
        }
        if aspect > 1.0 {
            xspan /= aspect;
        }
    } else if aspect > -1.0 {
        yspan *= 1.0 + aspect;
    }
    if (x + xspan + 0.5) < f64::from(im.view_x1)
        || (y + yspan + 0.5) < f64::from(im.view_y1)
        || (x - xspan - 0.5) > f64::from(im.view_x2)
        || (y - yspan - 0.5) > f64::from(im.view_y2)
    {
        return 0;
    }
    if passed & 2 == 0 {
        col = im.color;
    }
    im.draw_color = col;

    let c = PI2 * r;
    let pixels = cvt_i32(c / 4.0 + 0.5);
    let (mut arc1, mut arc2, mut arc3, mut arc4) = (0.0, PI, PI, PI2);
    let arcinc = (PI / 2.0) / f64::from(pixels);
    let sinb = arcinc.sin();
    let cosb = arcinc.cos();
    let (mut lastplotted_x2, mut lastplotted_y2) = (-1i32, 0i32);
    let (mut lastchecked_x2, mut lastchecked_y2) = (-1i32, 0i32);
    let mut i = 0i32;
    let im: &ImgStruct = im;

    if line_to_start {
        let (px, py) = (start.cos(), start.sin());
        let x2 = cvt_i32(px * xspan + 0.5);
        let y2 = cvt_i32(py * yspan - 0.5);
        // SAFETY: fast_line clips to the view.
        unsafe { fast_line(im, ix, iy, ix.wrapping_add(x2), iy.wrapping_sub(y2), col) };
    }
    let (mut px, mut py) = (1.0f64, 0.0f64);
    let plot = |arc1: f64, arc2: f64, arc3: f64, arc4: f64, lx: i32, ly: i32| {
        let on = |a: f64| if exclusive { a <= start || a >= end } else { a >= start && a <= end };
        // SAFETY: pset_and_clip clips to the view.
        unsafe {
            if on(arc1) {
                pset_and_clip(im, ix.wrapping_add(lx), iy.wrapping_add(ly), col);
            }
            if on(arc2) {
                pset_and_clip(im, ix.wrapping_sub(lx), iy.wrapping_add(ly), col);
            }
            if on(arc3) {
                pset_and_clip(im, ix.wrapping_sub(lx), iy.wrapping_sub(ly), col);
            }
            if on(arc4) {
                pset_and_clip(im, ix.wrapping_add(lx), iy.wrapping_sub(ly), col);
            }
        }
    };
    // Literal translation of the legacy goto structure (drawcircle/plot/allplotted).
    'drawcircle: loop {
        let x2 = cvt_i32(px * xspan + 0.5);
        let y2 = cvt_i32(py * yspan - 0.5);
        let mut do_plot = false;
        if i == 0 {
            lastchecked_x2 = x2;
            lastchecked_y2 = y2;
            do_plot = true;
        } else if x2.wrapping_sub(lastplotted_x2).wrapping_abs() >= 2 || y2.wrapping_sub(lastplotted_y2).wrapping_abs() >= 2 {
            do_plot = true;
        }
        if do_plot {
            // "plot:" label
            loop {
                plot(arc1, arc2, arc3, arc4, lastchecked_x2, lastchecked_y2);
                if i > pixels {
                    break 'drawcircle;
                }
                lastplotted_x2 = lastchecked_x2;
                lastplotted_y2 = lastchecked_y2;
                break;
            }
        }
        lastchecked_x2 = x2;
        lastchecked_y2 = y2;
        if i <= pixels {
            i += 1;
            if i > pixels {
                // goto plot (with the values just checked)
                plot(arc1, arc2, arc3, arc4, lastchecked_x2, lastchecked_y2);
                break 'drawcircle; // i > pixels -> allplotted
            }
            let px2 = px * cosb + py * sinb;
            py = py * cosb - px * sinb;
            px = px2;
            if i != 0 {
                arc1 += arcinc;
                arc2 -= arcinc;
                arc3 += arcinc;
                arc4 -= arcinc;
            }
            continue 'drawcircle;
        }
        break;
    }
    if line_from_end {
        let (px, py) = (end.cos(), end.sin());
        let x2 = cvt_i32(px * xspan + 0.5);
        let y2 = cvt_i32(py * yspan - 0.5);
        // SAFETY: fast_line clips to the view.
        unsafe { fast_line(im, ix, iy, ix.wrapping_add(x2), iy.wrapping_sub(y2), col) };
    }
    0
}

/// C ABI: `sub_circle` on `write_page`.
///
/// # Safety
/// `im` must be `write_page`.
#[no_mangle]
pub unsafe extern "C" fn qbrt_gfx_circle(
    im: *mut ImgStruct,
    x: f64,
    y: f64,
    r: f64,
    col: u32,
    start: f64,
    end: f64,
    aspect: f64,
    passed: i32,
) -> i32 {
    // SAFETY: guaranteed by the caller.
    unsafe { sub_circle(&mut *im, x, y, r, col, start, end, aspect, passed) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::testutil::TestImage;

    fn lit(t: &TestImage) -> Vec<(usize, usize)> {
        let mut v = Vec::new();
        for y in 0..t.img.height as usize {
            for x in 0..t.img.width as usize {
                if t.p8(x, y) != 0 {
                    v.push((x, y));
                }
            }
        }
        v
    }

    #[test]
    fn full_circle_is_symmetric_and_on_radius() {
        let mut t = TestImage::new(64, 64, 12); // aspect 1 in SCREEN 12
        assert_eq!(unsafe { sub_circle(&mut t.img, 32.0, 32.0, 10.0, 3, 0.0, 0.0, 0.0, 2) }, 0);
        let pts = lit(&t);
        assert!(pts.len() > 40);
        for (x, y) in &pts {
            let d = ((*x as f64 - 32.0).powi(2) + (*y as f64 - 32.0).powi(2)).sqrt();
            assert!((d - 10.0).abs() < 1.0, "({x},{y}) d={d}");
            assert_eq!(t.p8(64 - x, *y), 3, "mirror of ({x},{y})");
        }
        assert_eq!((t.p8(42, 32), t.p8(22, 32), t.p8(32, 22), t.p8(32, 42)), (3, 3, 3, 3));
        assert_eq!((t.img.x, t.img.y, t.img.draw_color), (32.0, 32.0, 3));
    }

    #[test]
    fn default_aspect_squashes_screen13() {
        let mut t = TestImage::new(320, 200, 13);
        unsafe { sub_circle(&mut t.img, 160.0, 100.0, 50.0, 1, 0.0, 0.0, 0.0, 2) };
        // aspect 5/6: vertical radius ~41.7
        assert_eq!(t.p8(210, 100), 1);
        assert!(t.p8(160, 100 - 42) == 1 || t.p8(160, 100 - 41) == 1);
        assert_eq!(t.p8(160, 50), 0);
    }

    #[test]
    fn arc_and_pie() {
        let pi = std::f64::consts::PI;
        let mut t = TestImage::new(64, 64, 12);
        // quarter arc from 0 to pi/2: the upper right quadrant (the last step may
        // land one pixel left of the centre line: arc2 accumulates down to ~pi/2)
        unsafe { sub_circle(&mut t.img, 32.0, 32.0, 10.0, 2, 0.0, pi / 2.0, 0.0, 2 | 4 | 8) };
        let bad: Vec<_> = lit(&t).into_iter().filter(|(x, y)| *x < 31 || *y > 32).collect();
        assert!(bad.is_empty(), "{bad:?}");
        // negative angles draw radii to the centre
        let mut t = TestImage::new(64, 64, 12);
        unsafe { sub_circle(&mut t.img, 32.0, 32.0, 10.0, 2, -0.001, -pi / 2.0, 0.0, 2 | 4 | 8) };
        assert_eq!(t.p8(32, 32), 2);
        assert_eq!(t.p8(37, 32), 2);
        assert_eq!(t.p8(32, 27), 2);
    }

    #[test]
    fn errors_and_text_mode() {
        let mut t = TestImage::new(64, 64, 12);
        assert_eq!(unsafe { sub_circle(&mut t.img, 1.0, 1.0, 1.0, 1, 7.0, 0.0, 0.0, 4) }, 5);
        let mut t = TestImage::new(8, 8, 0);
        assert_eq!(unsafe { sub_circle(&mut t.img, 1.0, 1.0, 1.0, 1, 0.0, 0.0, 0.0, 0) }, 5);
    }

    #[test]
    fn offscreen_circle_draws_nothing() {
        let mut t = TestImage::new(64, 64, 12);
        unsafe { sub_circle(&mut t.img, 500.0, 500.0, 10.0, 2, 0.0, 0.0, 0.0, 2) };
        assert_eq!(t.count_nonzero(), 0);
        unsafe { sub_circle(&mut t.img, 0.0, 0.0, 10.0, 2, 0.0, 0.0, 2.0, 2 | 16) };
        assert!(t.count_nonzero() > 0); // partly visible, clipped per pixel
    }
}
