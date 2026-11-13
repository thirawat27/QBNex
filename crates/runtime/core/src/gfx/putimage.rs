//! `_PUTIMAGE` (`sub__putimage`): software copy between surfaces with
//! clipping, mirroring, flipping, stretching, clear color and alpha blending.
//!
//! Handle resolution, the hardware-image command queue and the "source is
//! the destination" detour stay in the C++ shim; it passes the two surfaces
//! here. For hardware images the shim passes emulated surfaces and asks for
//! the resolved coordinates only (`hw_out`).

use super::blend::blend_onto;
use std::sync::atomic::{AtomicU64, Ordering};
use super::{qbr_d2l, qbr_f2l, ImgStruct, ERR_ILLEGAL_FUNCTION_CALL};

/// `_PUTIMAGE` coordinates as passed by generated code.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PutCoords {
    pub dx1: f64,
    pub dy1: f64,
    pub dx2: f64,
    pub dy2: f64,
    pub sx1: f64,
    pub sy1: f64,
    pub sx2: f64,
    pub sy2: f64,
}

/// Maps a double coordinate through a surface's WINDOW/VIEW; the legacy code
/// passes `double*float+float` to `qbr_float_to_long(float)`.
fn map(im: &ImgStruct, x: f64, y: f64) -> (i32, i32) {
    if im.clipping_or_scaling != 0 {
        if im.clipping_or_scaling == 2 {
            (
                qbr_f2l((x * f64::from(im.scaling_x) + f64::from(im.scaling_offset_x)) as f32)
                    .wrapping_add(im.view_offset_x),
                qbr_f2l((y * f64::from(im.scaling_y) + f64::from(im.scaling_offset_y)) as f32)
                    .wrapping_add(im.view_offset_y),
            )
        } else {
            (qbr_f2l(x as f32).wrapping_add(im.view_offset_x), qbr_f2l(y as f32).wrapping_add(im.view_offset_y))
        }
    } else {
        (qbr_f2l(x as f32), qbr_f2l(y as f32))
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Blend32,
    Copy32,
    Copy8,
    Clear8,
    Pal8To32,
    ClearPal8To32,
}

fn kind(s: &ImgStruct, d: &ImgStruct) -> Kind {
    if s.bytes_per_pixel == 4 {
        if s.alpha_disabled != 0 || d.alpha_disabled != 0 {
            Kind::Copy32
        } else {
            Kind::Blend32
        }
    } else if d.bytes_per_pixel == 1 {
        if s.transparent_color == -1 {
            Kind::Copy8
        } else {
            Kind::Clear8
        }
    } else if s.transparent_color == -1 {
        Kind::Pal8To32
    } else {
        Kind::ClearPal8To32
    }
}

/// Writes source pixel (sx, sy) of `s` to destination pixel `doff` of `d`.
///
/// # Safety
/// Both offsets must lie inside their surfaces.
#[inline]
unsafe fn plot(k: Kind, s: &ImgStruct, d: &ImgStruct, soff: isize, doff: isize) {
    // SAFETY: guaranteed by the caller.
    unsafe {
        match k {
            Kind::Blend32 => blend_onto(d.offset32().offset(doff), *s.offset32().offset(soff)),
            Kind::Copy32 => *d.offset32().offset(doff) = *s.offset32().offset(soff),
            Kind::Copy8 => *d.offset.offset(doff) = *s.offset.offset(soff),
            Kind::Clear8 => {
                let col = u32::from(*s.offset.offset(soff));
                if col != s.transparent_color as u32 {
                    *d.offset.offset(doff) = col as u8;
                }
            }
            Kind::Pal8To32 => *d.offset32().offset(doff) = *s.pal.add(*s.offset.offset(soff) as usize),
            Kind::ClearPal8To32 => {
                let col = u32::from(*s.offset.offset(soff));
                if col != s.transparent_color as u32 {
                    *d.offset32().offset(doff) = *s.pal.add(col as usize);
                }
            }
        }
    }
}

/// `_PUTIMAGE` on resolved surfaces. `passed` uses the legacy bits
/// (1 dx1, 2 STEP dx1, 4 dx2, 16 STEP dx2, 64 sx1, 256 STEP sx1, 512 sx2,
/// 1024 STEP sx2, 128 _SMOOTH). With `hw_out`, only the coordinates are
/// resolved and written as (sx1, sy1, sx2, sy2, dx1, dy1, dx2, dy2).
///
/// # Safety
/// `s` and `d` must be valid, distinct graphics surfaces of compatible depth
/// (checked by the shim), or emulated surfaces when `hw_out` is given.
pub unsafe fn putimage(s: &mut ImgStruct, d: &mut ImgStruct, c: PutCoords, passed: i32, hw_out: Option<&mut [i32; 8]>) -> i32 {
    let PutCoords { dx1: mut f_dx1, dy1: mut f_dy1, dx2: mut f_dx2, dy2: mut f_dy2, sx1: mut f_sx1, sy1: mut f_sy1, sx2: mut f_sx2, sy2: mut f_sy2 } = c;
    let (sw, sh, dw, dh) = (i32::from(s.width), i32::from(s.height), i32::from(d.width), i32::from(d.height));
    if passed & 2 != 0 {
        f_dx1 += f64::from(d.x);
        f_dy1 += f64::from(d.y);
    }
    if passed & 16 != 0 {
        f_dx2 += f_dx1;
        f_dy2 += f_dy1;
    }
    if passed & 256 != 0 {
        f_sx1 += f64::from(s.x);
        f_sy1 += f64::from(s.y);
    }
    if passed & 1024 != 0 {
        f_sx2 += f_sx1;
        f_sy2 += f_sy1;
    }
    if passed & 4 != 0 {
        d.x = f_dx2 as f32;
        d.y = f_dy2 as f32;
    } else if passed & 1 != 0 {
        if passed & 512 != 0 {
            d.x = (f_dx1 + (f_sx2 - f_sx1).abs()) as f32;
            d.y = (f_dy1 + (f_sy2 - f_sy1).abs()) as f32;
        } else {
            d.x = (f_dx1 + (f64::from(sw) - f_sx1).abs()) as f32;
            d.y = (f_dy1 + (f64::from(sh) - f_sy1).abs()) as f32;
        }
    } else {
        d.x = dw as f32;
        d.y = dh as f32;
    }
    if passed & 512 != 0 {
        s.x = f_sx2 as f32;
        s.y = f_sy2 as f32;
    } else {
        s.x = sw as f32;
        s.y = sh as f32;
    }
    let (mut dx1, mut dy1, mut dx2, mut dy2) = (0, 0, 0, 0);
    if passed & 1 != 0 {
        (dx1, dy1) = map(d, f_dx1, f_dy1);
        if passed & 4 != 0 {
            (dx2, dy2) = map(d, f_dx2, f_dy2);
        }
    }
    let (mut sx1, mut sy1, mut sx2, mut sy2) = (0, 0, 0, 0);
    if passed & 64 != 0 {
        (sx1, sy1) = map(s, f_sx1, f_sy1);
        if passed & 512 != 0 {
            (sx2, sy2) = map(s, f_sx2, f_sy2);
        }
    }
    let s: &ImgStruct = s;
    let d: &ImgStruct = d;

    if let Some(out) = hw_out {
        // hardware images: resolve the missing corners and hand them back
        if passed & 4 != 0 && passed & 512 != 0 {
        } else if passed & 4 != 0 {
            if passed & 64 != 0 {
                sx2 = sx1 + (dx2 - dx1).abs();
                sy2 = sy1 + (dy2 - dy1).abs();
            } else {
                sx2 = sw - 1;
                sy2 = sh - 1;
            }
        } else if passed & 512 != 0 {
            if passed & 1 != 0 {
                dx2 = dx1 + (sx2 - sx1).abs();
                dy2 = dy1 + (sy2 - sy1).abs();
            } else {
                dx2 = dw - 1;
                dy2 = dh - 1;
            }
        } else if passed & 64 != 0 {
            return ERR_ILLEGAL_FUNCTION_CALL;
        } else if passed & 1 != 0 {
            sx2 = sw - 1;
            sy2 = sh - 1;
            dx2 = dx1 + sx2;
            dy2 = dy1 + sy2;
        } else {
            sx2 = sw - 1;
            sy2 = sh - 1;
            dx2 = dw - 1;
            dy2 = dh - 1;
        }
        *out = [sx1, sy1, sx2, sy2, dx1, dy1, dx2, dy2];
        return 0;
    }

    // The software path works in i64: a coordinate of i32::MIN (out of range
    // or NaN after `qbr_f2l`) must not wrap in the corner and clip arithmetic
    // below. Values only narrow back once clipped into both surfaces.
    let (sw, sh, dw, dh) = (i64::from(sw), i64::from(sh), i64::from(dw), i64::from(dh));
    let (mut dx1, mut dy1, mut dx2, mut dy2) = (i64::from(dx1), i64::from(dy1), i64::from(dx2), i64::from(dy2));
    let (mut sx1, mut sy1, mut sx2, mut sy2) = (i64::from(sx1), i64::from(sy1), i64::from(sx2), i64::from(sy2));

    enum Next {
        Reverse,
        Stretch,
        Clip,
        NoFlip,
        StretchNoClip(f64, f64, f64, f64),
    }
    let next = if passed & 4 != 0 && passed & 512 != 0 {
        if (dx2 - dx1).abs() == (sx2 - sx1).abs() && (dy2 - dy1).abs() == (sy2 - sy1).abs() {
            Next::Reverse
        } else {
            Next::Stretch
        }
    } else if passed & 4 != 0 {
        if passed & 64 != 0 {
            sx2 = sx1 + (dx2 - dx1).abs();
            sy2 = sy1 + (dy2 - dy1).abs();
            Next::Reverse
        } else {
            sx2 = sw - 1;
            sy2 = sh - 1;
            if (dx2 - dx1).abs() + 1 == sw && (dy2 - dy1).abs() + 1 == sh {
                Next::Reverse
            } else {
                Next::Stretch
            }
        }
    } else if passed & 512 != 0 {
        if passed & 1 != 0 {
            dx2 = dx1 + (sx2 - sx1).abs();
            dy2 = dy1 + (sy2 - sy1).abs();
            Next::Reverse
        } else {
            dx2 = dw - 1;
            dy2 = dh - 1;
            if (sx2 - sx1).abs() + 1 == dw && (sy2 - sy1).abs() + 1 == dh {
                Next::Reverse
            } else {
                Next::Stretch
            }
        }
    } else if passed & 64 != 0 {
        return ERR_ILLEGAL_FUNCTION_CALL;
    } else if passed & 1 != 0 {
        sx2 = sw - 1;
        sy2 = sh - 1;
        dx2 = dx1 + sx2;
        dy2 = dy1 + sy2;
        Next::Clip
    } else {
        sx2 = sw - 1;
        sy2 = sh - 1;
        dx2 = dw - 1;
        dy2 = dh - 1;
        if sx2 == dx2 && sy2 == dy2 {
            Next::NoFlip
        } else {
            let (w, h) = (dx2 - dx1, dy2 - dy1);
            let (fsx1, fsx2, fsy1, fsy2) = half_pixel(sx1, sx2, sy1, sy2);
            let mx = if w != 0 { (fsx2 - fsx1) / w as f64 } else { 0.0 };
            let my = if h != 0 { (fsy2 - fsy1) / h as f64 } else { 0.0 };
            let _ = (fsx2, fsy2);
            Next::StretchNoClip(fsx1, fsy1, mx, my)
        }
    };

    let (mut mirror, mut flip) = (false, false);
    let mut orient = |dx1: &mut i64, dx2: &mut i64, sx1: &mut i64, sx2: &mut i64, dy1: &mut i64, dy2: &mut i64, sy1: &mut i64, sy2: &mut i64| {
        if (*dx2 < *dx1 && *sx2 > *sx1) || (*sx2 < *sx1 && *dx2 > *dx1) {
            mirror = true;
        }
        if *dx2 < *dx1 {
            core::mem::swap(dx1, dx2);
        }
        if *sx2 < *sx1 {
            core::mem::swap(sx1, sx2);
        }
        if (*dy2 < *dy1 && *sy2 > *sy1) || (*sy2 < *sy1 && *dy2 > *dy1) {
            flip = true;
        }
        if *dy2 < *dy1 {
            core::mem::swap(dy1, dy2);
        }
        if *sy2 < *sy1 {
            core::mem::swap(sy1, sy2);
        }
    };

    match next {
        Next::StretchNoClip(fsx1, fsy1, mx, my) => {
            // SAFETY: the full destination maps onto the full source; the
            // corners are 0 and the surface size - 1, so they fit in i32.
            unsafe { stretch(s, d, dx1 as i32, dy1 as i32, dx2 as i32, dy2 as i32, fsx1, fsy1, mx, my, false, false) };
            0
        }
        Next::Stretch => {
            orient(&mut dx1, &mut dx2, &mut sx1, &mut sx2, &mut dy1, &mut dy2, &mut sy1, &mut sy2);
            let (w, h) = (dx2 - dx1, dy2 - dy1);
            let (mut fsx1, mut fsx2, mut fsy1, mut fsy2) = half_pixel(sx1, sx2, sy1, sy2);
            let mx = if w != 0 { (fsx2 - fsx1) / w as f64 } else { 0.0 };
            let my = if h != 0 { (fsy2 - fsy1) / h as f64 } else { 0.0 };
            if dx1 < 0 {
                if mirror {
                    fsx2 += dx1 as f64 * mx;
                } else {
                    fsx1 -= dx1 as f64 * mx;
                }
                dx1 = 0;
            }
            if dy1 < 0 {
                if flip {
                    fsy2 += dy1 as f64 * my;
                } else {
                    fsy1 -= dy1 as f64 * my;
                }
                dy1 = 0;
            }
            if dx2 >= dw {
                if mirror {
                    fsx1 += (dx2 - dw + 1) as f64 * mx;
                } else {
                    fsx2 -= (dx2 - dw + 1) as f64 * mx;
                }
                dx2 = dw - 1;
            }
            if dy2 >= dh {
                if flip {
                    fsy1 += (dy2 - dh + 1) as f64 * my;
                } else {
                    fsy2 -= (dy2 - dh + 1) as f64 * my;
                }
                dy2 = dh - 1;
            }
            if w != 0 {
                if fsx1 < -0.4999999 {
                    let x = super::cvt_i32((-fsx1 - 0.499999) / mx + 1.0);
                    if mirror {
                        dx2 -= i64::from(x);
                    } else {
                        dx1 += i64::from(x);
                    }
                    fsx1 += f64::from(x) * mx;
                }
                if fsx2 > sw as f64 - 0.5000001 {
                    let x = super::cvt_i32((fsx2 - (sw as f64 - 0.500001)) / mx + 1.0);
                    if mirror {
                        dx1 += i64::from(x);
                    } else {
                        dx2 -= i64::from(x);
                    }
                    fsx2 -= f64::from(x) * mx;
                }
            }
            if h != 0 {
                if fsy1 < -0.4999999 {
                    let y = super::cvt_i32((-fsy1 - 0.499999) / my + 1.0);
                    if flip {
                        dy2 -= i64::from(y);
                    } else {
                        dy1 += i64::from(y);
                    }
                    fsy1 += f64::from(y) * my;
                }
                if fsy2 > sh as f64 - 0.5000001 {
                    let y = super::cvt_i32((fsy2 - (sh as f64 - 0.500001)) / my + 1.0);
                    if flip {
                        dy1 += i64::from(y);
                    } else {
                        dy2 -= i64::from(y);
                    }
                    fsy2 -= f64::from(y) * my;
                }
            }
            let _ = (fsx2, fsy2);
            // The source adjustments above only shrink the destination for
            // sane input; an INDEFINITE `cvt_i32` result must not move it
            // outside the surface.
            dx1 = dx1.max(0);
            dy1 = dy1.max(0);
            dx2 = dx2.min(dw - 1);
            dy2 = dy2.min(dh - 1);
            if dx1 > dx2 || dy1 > dy2 {
                return 0;
            }
            // SAFETY: the destination is clipped to `d` (so it fits in i32);
            // `stretch` skips samples outside the source.
            unsafe { stretch(s, d, dx1 as i32, dy1 as i32, dx2 as i32, dy2 as i32, fsx1, fsy1, mx, my, mirror, flip) };
            0
        }
        Next::Reverse | Next::Clip | Next::NoFlip => {
            if let Next::Reverse = next {
                orient(&mut dx1, &mut dx2, &mut sx1, &mut sx2, &mut dy1, &mut dy2, &mut sy1, &mut sy2);
            }
            if !matches!(next, Next::NoFlip) {
                // clip
                if dx1 < 0 {
                    if mirror {
                        sx2 += dx1;
                    } else {
                        sx1 -= dx1;
                    }
                    dx1 = 0;
                }
                if dy1 < 0 {
                    if flip {
                        sy2 += dy1;
                    } else {
                        sy1 -= dy1;
                    }
                    dy1 = 0;
                }
                if dx2 >= dw {
                    if mirror {
                        sx1 += dx2 - dw + 1;
                    } else {
                        sx2 -= dx2 - dw + 1;
                    }
                    dx2 = dw - 1;
                }
                if dy2 >= dh {
                    if flip {
                        sy1 += dy2 - dh + 1;
                    } else {
                        sy2 -= dy2 - dh + 1;
                    }
                    dy2 = dh - 1;
                }
                if sx1 < 0 {
                    if mirror {
                        dx2 += sx1;
                    } else {
                        dx1 -= sx1;
                    }
                    sx1 = 0;
                }
                if sy1 < 0 {
                    if flip {
                        dy2 += sy1;
                    } else {
                        dy1 -= sy1;
                    }
                    sy1 = 0;
                }
                if sx2 >= sw {
                    if mirror {
                        dx1 += sx2 - sw + 1;
                    } else {
                        dx2 -= sx2 - sw + 1;
                    }
                    sx2 = sw - 1;
                }
                if sy2 >= sh {
                    if flip {
                        dy1 += sy2 - sh + 1;
                    } else {
                        dy2 -= sy2 - sh + 1;
                    }
                    sy2 = sh - 1;
                }
                if dx1 > dx2 || dy1 > dy2 {
                    return 0;
                }
            }
            // Both rectangles now lie inside their surfaces (16-bit sizes), so
            // every value below fits in i32 / isize.
            let k = kind(s, d);
            let (sw, dw) = (sw as isize, dw as isize);
            for row in 0..=(dy2 - dy1) {
                let sy = if flip { sy2 - row } else { sy1 + row } as isize;
                let dy = (dy1 + row) as isize;
                for col in 0..=(dx2 - dx1) {
                    let sx = if mirror { sx2 - col } else { sx1 + col } as isize;
                    // SAFETY: clipped to both surfaces.
                    unsafe { plot(k, s, d, sy * sw + sx, dy * dw + (dx1 + col) as isize) };
                }
            }
            0
        }
    }
}

/// Source rectangle widened by just under half a pixel on each side.
/// The inputs come from i32 coordinates, so the conversion is exact.
fn half_pixel(sx1: i64, sx2: i64, sy1: i64, sy2: i64) -> (f64, f64, f64, f64) {
    let (mut fsx1, mut fsx2, mut fsy1, mut fsy2) = (sx1 as f64, sx2 as f64, sy1 as f64, sy2 as f64);
    if fsx1 <= fsx2 {
        fsx1 -= 0.499999;
        fsx2 += 0.499999;
    } else {
        fsx1 += 0.499999;
        fsx2 -= 0.499999;
    }
    if fsy1 <= fsy2 {
        fsy1 -= 0.499999;
        fsy2 += 0.499999;
    } else {
        fsy1 += 0.499999;
        fsy2 -= 0.499999;
    }
    (fsx1, fsx2, fsy1, fsy2)
}

/// The legacy stretch loops: destination rows/columns are walked from the
/// flipped/mirrored corner while the source position accumulates `mx`/`my`.
/// Samples outside the source are skipped (documented deviation).
///
/// # Safety
/// The destination rectangle must be inside `d`.
unsafe fn stretch(
    s: &ImgStruct,
    d: &ImgStruct,
    dx1: i32,
    dy1: i32,
    dx2: i32,
    dy2: i32,
    fsx1: f64,
    fsy1: f64,
    mx: f64,
    my: f64,
    mirror: bool,
    flip: bool,
) {
    let k = kind(s, d);
    let (w, h) = (dx2 - dx1 + 1, dy2 - dy1 + 1);
    let (sw, dw) = (s.width as isize, d.width as isize);
    let start_x = if mirror { dx2 } else { dx1 };
    let start_y = if flip { dy2 } else { dy1 };
    let xdir: i32 = if mirror { -1 } else { 1 };
    let ydir: i32 = if flip { -1 } else { 1 };
    let fsx1 = fsx1 - mx; // "prev value is moved on from"
    let mut fy = fsy1;
    let sh = s.height as isize;
    for row in 0..h {
        let sy = qbr_d2l(fy) as isize;
        let dy = (start_y + row * ydir) as isize;
        let mut fx = fsx1;
        for col in 0..w {
            fx += mx;
            let sx = qbr_d2l(fx) as isize;
            let dx = (start_x + col * xdir) as isize;
            if sx < 0 || sx >= sw || sy < 0 || sy >= sh {
                // Legacy read outside the source buffer here (a one pixel
                // wide or high destination skips the source clipping).
                SOURCE_OUT_OF_RANGE.fetch_add(1, Ordering::Relaxed);
                continue;
            }
            // SAFETY: destination clipped by the caller, source checked above.
            unsafe { plot(k, s, d, sy * sw + sx, dy * dw + dx) };
        }
        fy += my;
    }
}

/// Number of stretched source samples that fell outside the source.
pub static SOURCE_OUT_OF_RANGE: AtomicU64 = AtomicU64::new(0);

/// C ABI. `hw_out` may be null (software copy) or point to 8 int32 that
/// receive (sx1, sy1, sx2, sy2, dx1, dy1, dx2, dy2) for a hardware command.
///
/// # Safety
/// See [`putimage`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_gfx_putimage(
    s: *mut ImgStruct,
    d: *mut ImgStruct,
    c: *const PutCoords,
    passed: i32,
    hw_out: *mut i32,
) -> i32 {
    // SAFETY: guaranteed by the caller.
    unsafe {
        let out = if hw_out.is_null() { None } else { Some(&mut *(hw_out as *mut [i32; 8])) };
        putimage(&mut *s, &mut *d, *c, passed, out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::testutil::TestImage;

    fn put(s: &mut TestImage, d: &mut TestImage, c: PutCoords, passed: i32) -> i32 {
        unsafe { putimage(&mut s.img, &mut d.img, c, passed, None) }
    }

    fn src8() -> Box<TestImage> {
        let s = TestImage::new(4, 2, 13);
        // row 0: 1 2 3 4, row 1: 5 6 7 8
        for i in 0..8u8 {
            unsafe { *s.img.offset.add(i as usize) = i + 1 };
        }
        s
    }

    #[test]
    fn plain_copy_and_cursor() {
        let mut s = src8();
        let mut d = TestImage::new(8, 4, 13);
        let c = PutCoords { dx1: 2.0, dy1: 1.0, ..Default::default() };
        assert_eq!(put(&mut s, &mut d, c, 1), 0);
        assert_eq!((d.p8(2, 1), d.p8(5, 1), d.p8(2, 2), d.p8(5, 2)), (1, 4, 5, 8));
        assert_eq!(d.p8(1, 1), 0);
        assert_eq!((d.img.x, d.img.y), (6.0, 3.0));
        assert_eq!((s.img.x, s.img.y), (4.0, 2.0));
    }

    #[test]
    fn clipping_at_edges() {
        let mut s = src8();
        let mut d = TestImage::new(8, 4, 13);
        let c = PutCoords { dx1: -2.0, dy1: 3.0, ..Default::default() };
        put(&mut s, &mut d, c, 1);
        assert_eq!((d.p8(0, 3), d.p8(1, 3), d.p8(2, 3)), (3, 4, 0));
    }

    #[test]
    fn mirror_and_flip() {
        let mut s = src8();
        let mut d = TestImage::new(8, 4, 13);
        // destination given right-to-left and bottom-to-top: mirrored and flipped
        let c = PutCoords { dx1: 3.0, dy1: 1.0, dx2: 0.0, dy2: 0.0, ..Default::default() };
        put(&mut s, &mut d, c, 1 | 4);
        assert_eq!((d.p8(0, 0), d.p8(3, 0), d.p8(0, 1), d.p8(3, 1)), (8, 5, 4, 1));
    }

    #[test]
    fn stretch_doubles() {
        let mut s = src8();
        let mut d = TestImage::new(8, 4, 13);
        let c = PutCoords { dx1: 0.0, dy1: 0.0, dx2: 7.0, dy2: 3.0, ..Default::default() };
        put(&mut s, &mut d, c, 1 | 4);
        assert_eq!((d.p8(0, 0), d.p8(1, 0), d.p8(2, 0), d.p8(7, 3)), (1, 1, 2, 8));
    }

    #[test]
    fn clear_color_and_palette_to_32() {
        let mut s = src8();
        s.img.transparent_color = 2;
        let mut d = TestImage::new(4, 2, 13);
        unsafe { *d.img.offset.add(1) = 9 };
        put(&mut s, &mut d, PutCoords::default(), 1);
        assert_eq!((d.p8(0, 0), d.p8(1, 0)), (1, 9));
        let mut d = TestImage::new(4, 2, 32);
        put(&mut s, &mut d, PutCoords::default(), 1);
        assert_eq!(d.p32(0, 0), s.palette[1]);
        assert_eq!(d.p32(1, 0), 0);
    }

    #[test]
    fn blend_32() {
        let mut s = TestImage::new(1, 1, 32);
        s.pixels[0] = 0x80FE_0000;
        let mut d = TestImage::new(1, 1, 32);
        d.pixels[0] = 0xFF00_00FE;
        put(&mut s, &mut d, PutCoords::default(), 1);
        assert_eq!(d.p32(0, 0) & 0xFFFFFF, 0x7F_007F);
        s.img.alpha_disabled = 1;
        put(&mut s, &mut d, PutCoords::default(), 1);
        assert_eq!(d.p32(0, 0), 0x80FE_0000);
    }

    #[test]
    fn out_of_range_and_nan_coordinates_stay_inside_the_destination() {
        // `qbr_f2l` turns these into INDEFINITE (i32::MIN); the clip used to
        // wrap and copy far outside the destination buffer
        let mut s = src8();
        let mut d = TestImage::new(8, 4, 13);
        // canary: the 32-byte surface sits in the middle of a larger buffer
        let mut buf = vec![0xAAu8; 96];
        d.img.offset = unsafe { buf.as_mut_ptr().add(32) };
        unsafe { core::ptr::write_bytes(d.img.offset, 0, 32) };
        for v in [-3e9, 3e9, f64::NAN, -2147483648.0, 2147483647.0] {
            let cases = [
                (PutCoords { dx1: v, dy1: 0.0, ..Default::default() }, 1),
                (PutCoords { dx1: 0.0, dy1: v, ..Default::default() }, 1),
                (PutCoords { dx1: v, dy1: 0.0, dx2: 7.0, dy2: v, ..Default::default() }, 1 | 4),
                (PutCoords { dx1: 7.0, dy1: 3.0, dx2: v, dy2: 0.0, ..Default::default() }, 1 | 4),
                (PutCoords { dx1: v, dy1: v, sx2: 3.0, sy2: 1.0, ..Default::default() }, 1 | 64 | 512),
                (PutCoords { dx1: 0.0, dy1: 0.0, dx2: 7.0, dy2: 3.0, sx1: v, sy1: 0.0, sx2: 3.0, sy2: v }, 1 | 4 | 64 | 512),
                (PutCoords { sx1: v, sy1: v, sx2: 3.0, sy2: 1.0, ..Default::default() }, 64 | 512),
            ];
            for (c, passed) in cases {
                assert_eq!(put(&mut s, &mut d, c, passed), 0, "{v} {passed}");
            }
        }
        assert!(buf[..32].iter().all(|b| *b == 0xAA));
        assert!(buf[64..].iter().all(|b| *b == 0xAA));
    }

    #[test]
    fn source_rect_without_destination_corner_is_an_error() {
        let mut s = src8();
        let mut d = TestImage::new(8, 4, 13);
        assert_eq!(put(&mut s, &mut d, PutCoords::default(), 64), 5);
    }
}
