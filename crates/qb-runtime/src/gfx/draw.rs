//! DRAW (`sub_draw`, `draw_num`): the graphics macro language.
//!
//! `VARPTR$` arguments (`=` numbers and `X` sub-strings) point into the
//! conventional memory block `cmem` (offset 1280 = DBLOCK), so the caller
//! passes `cmem`.

use super::line::fast_line;
use super::paint::paint_color;
use super::{qbr_i32, ImgStruct, ERR_ILLEGAL_FUNCTION_CALL};

/// QBasic "Overflow" (X-stack exhausted).
const ERR_OVERFLOW: i32 = 6;
const DBLOCK: usize = 1280;
const STACK_MAX: usize = 8192;

/// C/C++ `int64 = double` (`cvttsd2si`): truncation, `INT64_MIN` when out of range.
fn cvt_i64(v: f64) -> i64 {
    if v.is_nan() || v >= -(i64::MIN as f64) || v < i64::MIN as f64 {
        i64::MIN
    } else {
        v as i64
    }
}

/// `double d = *(long double*)p` on x87: unsupported encodings (unnormals,
/// pseudo-NaN/infinity) load as the "indefinite" QNaN and NaNs keep their
/// payload, quietened; finite values round to nearest.
fn f80_load_store(b: [u8; 10]) -> f64 {
    qb_float80::F80::from_bytes(b).to_f64()
}

struct Parser {
    cp: *const u8,
    len: i32,
    i: i32,
    cmem: *const u8,
    invalid: bool,
    undefined: bool,
}

impl Parser {
    fn at(&self, k: i32) -> u8 {
        // SAFETY: callers check k < len; the string or cmem block holds len bytes.
        unsafe { *self.cp.add(k as usize) }
    }

    fn cmem_at<T: Copy>(&self, offset: usize) -> T {
        // SAFETY: VARPTR$ offsets address variables inside cmem.
        unsafe { core::ptr::read_unaligned(self.cmem.add(DBLOCK + offset) as *const T) }
    }

    /// Legacy `draw_num`.
    fn num(&mut self) -> f64 {
        self.invalid = false;
        self.undefined = true;
        let mut d = 0f64;
        let mut dp = false;
        let mut dp_mult = 0f64;
        let mut sgn = 1f64;
        let mut vptr = false;
        loop {
            if self.i >= self.len {
                return d * sgn;
            }
            let c = self.at(self.i);
            if vptr {
                if self.i + 2 >= self.len {
                    self.invalid = true;
                    return 0.0;
                }
                let offset = self.at(self.i + 2) as usize * 256 + self.at(self.i + 1) as usize;
                self.i += 3;
                d = match c {
                    1 => f64::from(self.cmem_at::<i8>(offset)),
                    129 => f64::from(self.cmem_at::<u8>(offset)),
                    2 => f64::from(self.cmem_at::<i16>(offset)),
                    130 => f64::from(self.cmem_at::<u16>(offset)),
                    4 => f64::from(self.cmem_at::<f32>(offset)),
                    5 => self.cmem_at::<i64>(offset) as f64,
                    133 => self.cmem_at::<u64>(offset) as f64,
                    6 => f80_load_store(self.cmem_at(offset)),
                    8 => self.cmem_at::<f64>(offset),
                    20 => f64::from(self.cmem_at::<i32>(offset)),
                    148 => f64::from(self.cmem_at::<u32>(offset)),
                    _ => {
                        self.invalid = true;
                        return 0.0;
                    }
                };
                self.invalid = false;
                self.undefined = false;
                return d;
            }
            match c {
                32 | 9 => self.i += 1,
                48..=57 => {
                    let v = f64::from(c - 48);
                    if dp {
                        d += v * dp_mult;
                        dp_mult /= 10.0;
                    } else {
                        d = d * 10.0 + v;
                    }
                    self.undefined = false;
                    self.invalid = false;
                    self.i += 1;
                }
                45 => {
                    if dp || !self.undefined {
                        return d * sgn;
                    }
                    sgn = -sgn;
                    self.invalid = true;
                    self.i += 1;
                }
                43 => {
                    if dp || !self.undefined {
                        return d * sgn;
                    }
                    self.invalid = true;
                    self.i += 1;
                }
                46 => {
                    if dp {
                        return d * sgn;
                    }
                    dp = true;
                    dp_mult = 0.1;
                    if !self.undefined {
                        self.invalid = true;
                    }
                    self.i += 1;
                }
                61 => {
                    if self.invalid || dp || !self.undefined {
                        self.invalid = true;
                        return 0.0;
                    }
                    vptr = true;
                    self.i += 1;
                }
                _ => return d * sgn,
            }
        }
    }

    fn bad(&self) -> bool {
        self.invalid || self.undefined
    }
}

/// Unit vectors (up, right, up-right "E", down-right "F") after rotating by `ta` degrees.
fn vectors(r: f64, ta: f64) -> [f64; 8] {
    let (mut vx, mut vy, mut ex, mut ey, mut hx, mut hy, mut fx, mut fy) = (0.0, -1.0, r, -1.0, r, 0.0, r, 1.0);
    let d = ta * 0.0174532925199433;
    let (sin_ta, cos_ta) = (d.sin(), d.cos());
    let rot = |x: f64, y: f64| (x * cos_ta + y * sin_ta, y * cos_ta - x * sin_ta);
    (vx, vy) = rot(vx, vy);
    (hx, hy) = rot(hx, hy);
    (ex, ey) = rot(ex, ey);
    (fx, fy) = rot(fx, fy);
    [vx, vy, ex, ey, hx, hy, fx, fy]
}

/// `DRAW s$`. Returns a QBasic error code (state changes made before the
/// error are kept, as in legacy).
///
/// # Safety
/// `im` must be a valid surface whose VIEW lies inside it; `s` readable for
/// `len` bytes; `cmem` must be the conventional memory block when the string
/// uses `VARPTR$` (it may be null otherwise). The byte after the end of each
/// (sub-)string must be readable: `M` and `P` peek at it like legacy did.
#[allow(clippy::eq_op)] // aspect expressions copy the legacy arithmetic
pub unsafe fn draw(im: &mut ImgStruct, s: *const u8, len: i32, cmem: *const u8) -> i32 {
    const ERR: i32 = ERR_ILLEGAL_FUNCTION_CALL;
    if im.text != 0 {
        return ERR;
    }
    let mut draw_ta = im.draw_ta;
    let mut draw_scale = im.draw_scale;
    let r = match im.compatible_mode {
        1 | 7 | 13 => 4.0 / ((3.0 / 200.0) * 320.0),
        2 | 8 => 4.0 / ((3.0 / 200.0) * 640.0),
        9 | 10 => 4.0 / ((3.0 / 350.0) * 640.0),
        11 | 12 => 4.0 / ((3.0 / 480.0) * 640.0),
        m if m <= 13 => 1.0, // unreachable modes; legacy kept a stale static
        _ => 1.0,
    };
    let ir = 1.0 / r;
    let [mut vx, mut vy, mut ex, mut ey, mut hx, mut hy, mut fx, mut fy] = [0.0, -1.0, r, -1.0, r, 0.0, r, 1.0];
    if draw_ta != 0.0 {
        [vx, vy, ex, ey, hx, hy, fx, fy] = vectors(r, draw_ta);
    }
    // the cursor is mapped with float arithmetic, then kept as double
    let (mut px, mut py): (f64, f64) = if im.clipping_or_scaling != 0 {
        if im.clipping_or_scaling == 2 {
            (
                f64::from(im.x * im.scaling_x + im.scaling_offset_x + im.view_offset_x as f32),
                f64::from(im.y * im.scaling_y + im.scaling_offset_y + im.view_offset_y as f32),
            )
        } else {
            (f64::from(im.x + im.view_offset_x as f32), f64::from(im.y + im.view_offset_y as f32))
        }
    } else {
        (f64::from(im.x), f64::from(im.y))
    };
    let unmap = |im: &ImgStruct, px: f64, py: f64| -> (f64, f64) {
        if im.clipping_or_scaling != 0 {
            if im.clipping_or_scaling == 2 {
                (
                    (px - f64::from(im.view_offset_x) - f64::from(im.scaling_offset_x)) / f64::from(im.scaling_x),
                    (py - f64::from(im.view_offset_y) - f64::from(im.scaling_offset_y)) / f64::from(im.scaling_y),
                )
            } else {
                (px - f64::from(im.view_offset_x), py - f64::from(im.view_offset_y))
            }
        } else {
            (px, py)
        }
    };
    let mut col = im.draw_color;
    let (mut prefix_b, mut prefix_n) = (false, false);
    let mut stack: Vec<(*const u8, u16, u16)> = Vec::new();
    let mut p = Parser { cp: s, len, i: 0, cmem, invalid: false, undefined: true };

    loop {
        if p.i >= p.len {
            if let Some((cp, l, i)) = stack.pop() {
                p.cp = cp;
                p.len = i32::from(l);
                p.i = i32::from(i);
                continue;
            }
            let (x, y) = unmap(im, px, py);
            im.x = x as f32;
            im.y = y as f32;
            return 0;
        }
        let mut c = p.at(p.i);
        if c.is_ascii_lowercase() {
            c -= 32;
        }
        let step: (f64, f64) = match c {
            b'M' => {
                loop {
                    p.i += 1;
                    if p.i >= p.len {
                        return ERR;
                    }
                    let c = p.at(p.i);
                    if c != 32 && c != 9 {
                        break;
                    }
                }
                let c = p.at(p.i);
                let relative = c == 43 || c == 45;
                let mut px2 = p.num();
                if p.bad() {
                    return ERR;
                }
                // legacy reads the byte at p.i even when it is one past the end
                // of the string (usually a terminator, sometimes a comma)
                if p.at(p.i) != 44 {
                    return ERR;
                }
                p.i += 1;
                let mut py2 = p.num();
                if p.bad() {
                    return ERR;
                }
                if relative {
                    let xx = (px2 * ir) * hx - (py2 * ir) * vx;
                    let yy = px2 * hy - py2 * vy;
                    px2 = px + xx * draw_scale;
                    py2 = py + yy * draw_scale;
                }
                if !prefix_b {
                    // SAFETY: fast_line clips to the view.
                    unsafe { fast_line(im, qbr_i32(px), qbr_i32(py), qbr_i32(px2), qbr_i32(py2), col) };
                }
                if !prefix_n {
                    px = px2;
                    py = py2;
                }
                prefix_b = false;
                prefix_n = false;
                continue;
            }
            b'T' => {
                loop {
                    p.i += 1;
                    if p.i >= p.len {
                        return ERR;
                    }
                    let c = p.at(p.i);
                    if c != 32 && c != 9 {
                        break;
                    }
                }
                let c = p.at(p.i);
                if c != 65 && c != 97 {
                    return ERR;
                }
                p.i += 1;
                let d = p.num();
                if p.bad() {
                    return ERR;
                }
                draw_ta = d;
                im.draw_ta = draw_ta;
                [vx, vy, ex, ey, hx, hy, fx, fy] = vectors(r, draw_ta);
                continue;
            }
            b'U' => (vx, vy),
            b'D' => (-vx, -vy),
            b'L' => (-hx, -hy),
            b'R' => (hx, hy),
            b'E' => (ex, ey),
            b'F' => (fx, fy),
            b'G' => (-ex, -ey),
            b'H' => (-fx, -fy),
            b'C' => {
                p.i += 1;
                let d = p.num();
                if p.bad() {
                    return ERR;
                }
                let c64 = cvt_i64(d);
                if c64 as f64 != d {
                    return ERR;
                }
                col = c64 as u32;
                im.draw_color = col;
                continue;
            }
            b'B' => {
                prefix_b = true;
                p.i += 1;
                continue;
            }
            b'N' => {
                prefix_n = true;
                p.i += 1;
                continue;
            }
            b'S' => {
                p.i += 1;
                let d = p.num();
                if p.bad() || d < 0.0 {
                    return ERR;
                }
                draw_scale = d / 4.0;
                im.draw_scale = draw_scale;
                continue;
            }
            b'P' => {
                p.i += 1;
                let d = p.num();
                if p.bad() {
                    return ERR;
                }
                let fill = cvt_i64(d);
                if fill as f64 != d {
                    return ERR;
                }
                if p.at(p.i) != 44 {
                    // (may read one byte past the end, as legacy)
                    return ERR;
                }
                p.i += 1;
                let d = p.num();
                if p.bad() {
                    return ERR;
                }
                let border = cvt_i64(d);
                if border as f64 != d {
                    return ERR;
                }
                let (xx, yy) = unmap(im, px, py);
                // SAFETY: guaranteed by the caller; cannot fail (not a text surface).
                unsafe { paint_color(im, xx as f32, yy as f32, fill as u32, border as u32, 2 + 4) };
                col = fill as u32;
                continue;
            }
            b'A' => {
                p.i += 1;
                let d = p.num();
                if p.bad() {
                    return ERR;
                }
                draw_ta = if d == 0.0 {
                    0.0
                } else if d == 1.0 {
                    90.0
                } else if d == 2.0 {
                    180.0
                } else if d == 3.0 {
                    270.0
                } else {
                    return ERR;
                };
                im.draw_ta = draw_ta;
                [vx, vy, ex, ey, hx, hy, fx, fy] = vectors(r, draw_ta);
                continue;
            }
            b'X' => {
                p.i += 1;
                if p.i + 2 >= p.len || p.at(p.i) != 3 {
                    return ERR;
                }
                let offset = p.at(p.i + 2) as usize * 256 + p.at(p.i + 1) as usize;
                p.i += 3;
                if stack.len() == STACK_MAX {
                    return ERR_OVERFLOW;
                }
                stack.push((p.cp, p.len as u16, p.i as u16));
                // string descriptor in DBLOCK: u16 length, u16 offset
                let at = |k: usize| -> usize { p.cmem_at::<u8>(offset + k) as usize };
                let (l, o) = (at(1) * 256 + at(0), at(3) * 256 + at(2));
                // SAFETY: the descriptor addresses a string inside cmem.
                p.cp = unsafe { cmem.add(DBLOCK + o) };
                p.len = l as i32;
                p.i = 0;
                continue;
            }
            32 | 9 | 59 => {
                p.i += 1;
                continue;
            }
            _ => return ERR,
        };
        // U D L R E F G H
        p.i += 1;
        let mut d = p.num();
        if p.invalid {
            return ERR;
        }
        if p.undefined {
            d = 1.0;
        }
        let xx = step.0 * d * ir;
        let yy = step.1 * d;
        let px2 = px + xx * draw_scale;
        let py2 = py + yy * draw_scale;
        if !prefix_b {
            // SAFETY: fast_line clips to the view.
            unsafe { fast_line(im, qbr_i32(px), qbr_i32(py), qbr_i32(px2), qbr_i32(py2), col) };
        }
        if !prefix_n {
            px = px2;
            py = py2;
        }
        prefix_b = false;
        prefix_n = false;
    }
}

/// Diagnostic for the differential test: [`f80_load_store`] on 10 bytes.
///
/// # Safety
/// `p` must be readable for 10 bytes.
#[no_mangle]
pub unsafe extern "C" fn qb_gfx_draw_f80(p: *const u8) -> f64 {
    let mut b = [0u8; 10];
    // SAFETY: guaranteed by the caller.
    unsafe { core::ptr::copy_nonoverlapping(p, b.as_mut_ptr(), 10) };
    f80_load_store(b)
}

/// C ABI: `DRAW` on `write_page`.
///
/// # Safety
/// See [`draw`].
#[no_mangle]
pub unsafe extern "C" fn qb_gfx_draw(im: *mut ImgStruct, s: *const u8, len: i32, cmem: *const u8) -> i32 {
    // SAFETY: guaranteed by the caller.
    unsafe { draw(&mut *im, s, len, cmem) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::testutil::TestImage;

    fn run(t: &mut TestImage, s: &str) -> i32 {
        let mut b = s.as_bytes().to_vec();
        b.push(0); // terminator, as qbs strings provide
        unsafe { draw(&mut t.img, b.as_ptr(), s.len() as i32, core::ptr::null()) }
    }

    #[test]
    fn moves_and_lines() {
        let mut t = TestImage::new(64, 64, 12); // aspect 1
        t.img.draw_color = 4;
        assert_eq!(run(&mut t, "BM10,10 R5 D5 L5 U5"), 0);
        assert_eq!((t.p8(10, 10), t.p8(15, 10), t.p8(15, 15), t.p8(10, 15)), (4, 4, 4, 4));
        assert_eq!(t.p8(9, 9), 0);
        assert_eq!((t.img.x, t.img.y), (10.0, 10.0));
        assert_eq!(run(&mut t, "c2 nE3 bm+0,10 f2"), 0);
        assert_eq!((t.p8(13, 7), t.p8(12, 22)), (2, 2));
        assert_eq!(t.img.draw_color, 2);
    }

    #[test]
    fn scale_angle_and_paint() {
        let mut t = TestImage::new(64, 64, 12);
        assert_eq!(run(&mut t, "S8 BM20,20 C3 R4 D4 L4 U4 BM+1,1 P5,3"), 0);
        assert_eq!((t.p8(28, 20), t.p8(24, 24)), (3, 5));
        assert_eq!(t.img.draw_scale, 2.0);
        let mut t = TestImage::new(64, 64, 12);
        assert_eq!(run(&mut t, "BM30,30 A1 C1 U5"), 0);
        assert_eq!(t.p8(25, 30), 1); // A1: up turns into left
        assert_eq!(t.img.draw_ta, 90.0);
        assert_eq!(run(&mut t, "TA0"), 0);
        assert_eq!(t.img.draw_ta, 0.0);
    }

    #[test]
    fn errors() {
        let mut t = TestImage::new(16, 16, 13);
        assert_eq!(run(&mut t, "Q"), 5);
        assert_eq!(run(&mut t, "M5"), 5);
        assert_eq!(run(&mut t, "C1.5"), 5);
        assert_eq!(run(&mut t, "A4"), 5);
        assert_eq!(run(&mut t, "S-1"), 5);
        assert_eq!(run(&mut t, "TB"), 5);
        let mut t = TestImage::new(16, 16, 0);
        assert_eq!(run(&mut t, "R1"), 5);
    }
}
