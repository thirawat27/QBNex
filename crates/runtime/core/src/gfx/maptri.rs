//! `_MAPTRIANGLE` on software surfaces: the setup part of
//! `sub__maptriangle` plus the scan converter that libqb.cpp included eight
//! times (`mtri1/1t/2/2t/3/3t/4/4t.cpp`: 32-bit copy, 32-bit blend, 8-bit
//! copy, 8-bit with clear color; `t` = tiled texture coordinates).
//!
//! All arithmetic is the legacy 16.16 fixed point in `int32` (with `int64`
//! intermediates), wrapping where C wrapped. Known legacy quirks kept: the
//! tiling test of the third source y uses `< 0` instead of `< 16`, and the
//! upward texture shift compares with the source width.

use super::blend::blend_onto;
use super::{cvt_i32, qbr_f2l, ImgStruct, ERR_ILLEGAL_FUNCTION_CALL};

#[derive(Clone, Copy, Default)]
struct Point {
    x: i32,
    y: i32,
    tx: i32,
    ty: i32,
}

#[derive(Clone, Copy, Default)]
struct Grad {
    x: i32,
    xi: i32,
    tx: i32,
    ty: i32,
    txi: i32,
    tyi: i32,
    y1: i32,
    y2: i32,
    p1: usize,
    p2: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
    Copy32,
    Blend32,
    Copy8,
    Clear8,
}

/// Software `_MAPTRIANGLE [_SEAMLESS] (s1)-(s2)-(s3), src TO (d1)-(d2)-(d3), dst`.
/// `seamless`: the `_SEAMLESS` flag (no edge overlap). Returns an error code.
///
/// # Safety
/// `src` and `dst` must be valid surfaces.
#[allow(clippy::needless_range_loop)] // 1-based vertex indices as in legacy
pub unsafe fn maptriangle(
    src: &ImgStruct,
    dst: &ImgStruct,
    s: [f32; 6],
    d: [f32; 6],
    seamless: bool,
) -> i32 {
    const ERR: i32 = ERR_ILLEGAL_FUNCTION_CALL;
    let dx = [qbr_f2l(d[0]), qbr_f2l(d[2]), qbr_f2l(d[4])];
    let dy = [qbr_f2l(d[1]), qbr_f2l(d[3]), qbr_f2l(d[5])];
    if src.text != 0 || dst.text != 0 {
        return ERR;
    }
    if src.bytes_per_pixel != dst.bytes_per_pixel {
        return ERR;
    }
    let no_edge_overlap = seamless;
    let (dwidth, dheight) = (i32::from(dst.width), i32::from(dst.height));
    let (swidth, sheight) = (i32::from(src.width), i32::from(src.height));
    let swidth2 = swidth.wrapping_shl(16);
    let sheight2 = sheight.wrapping_shl(16);
    let mut p = [Point::default(); 4];
    let mut g = [Grad::default(); 4];
    let mut tile = false;
    let limit: i32 = 16383;
    let limit2: i32 = (limit << 16) + 32678;
    let (nlimit, nlimit2) = (-limit, -limit2);
    let fixed = |v: f32| cvt_i32(f64::from(v) * 65536.0).wrapping_add(32768);
    // source x (vs width), then source y (vs height); the third y tests < 0
    for (i, &sv) in [s[0], s[2], s[4]].iter().enumerate() {
        let v = fixed(sv);
        if v < 16 || v >= swidth2.wrapping_sub(16) {
            tile = true;
        }
        if v < nlimit2 || v > limit2 {
            return ERR;
        }
        p[i + 1].tx = v;
    }
    for (i, &sv) in [s[1], s[3], s[5]].iter().enumerate() {
        let v = fixed(sv);
        let low = if i == 2 { 0 } else { 16 };
        if v < low || v >= sheight2.wrapping_sub(16) {
            tile = true;
        }
        if v < nlimit2 || v > limit2 {
            return ERR;
        }
        p[i + 1].ty = v;
    }
    if tile {
        let (mut lhs, mut rhs, mut top, mut bottom) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        for tp in &p[1..4] {
            bottom = bottom.max(tp.ty);
            top = top.min(tp.ty);
            rhs = rhs.max(tp.tx);
            lhs = lhs.min(tp.tx);
        }
        let _ = (rhs, bottom);
        let mut z = false;
        if lhs < 0 {
            let v = ((lhs + 1) / -swidth2 + 1).wrapping_mul(swidth2);
            for tp in &mut p[1..4] {
                tp.tx = tp.tx.wrapping_add(v);
            }
            z = true;
        } else if lhs >= swidth2 {
            let v = (lhs / swidth2).wrapping_mul(swidth2);
            for tp in &mut p[1..4] {
                tp.tx = tp.tx.wrapping_sub(v);
            }
            z = true;
        }
        if top < 0 {
            let v = ((top + 1) / -sheight2 + 1).wrapping_mul(sheight2);
            for tp in &mut p[1..4] {
                tp.ty = tp.ty.wrapping_add(v);
            }
            z = true;
        } else if top >= swidth2 {
            // legacy compares with the width here
            let v = (top / sheight2).wrapping_mul(sheight2);
            for tp in &mut p[1..4] {
                tp.ty = tp.ty.wrapping_sub(v);
            }
            z = true;
        }
        if z {
            let still = p[1..4].iter().any(|tp| {
                tp.tx < 16 || tp.tx >= swidth2.wrapping_sub(16) || tp.ty < 16 || tp.ty >= sheight2.wrapping_sub(16)
            });
            if !still {
                tile = false;
            }
        }
    }
    for i in 0..3 {
        if dx[i] < nlimit || dx[i] > limit || dy[i] < nlimit || dy[i] > limit {
            // legacy checks all x first, then all y; the result is the same error
            return ERR;
        }
    }
    for i in 0..3 {
        p[i + 1].x = (dx[i] << 16) + 32768;
        p[i + 1].y = dy[i];
    }
    let fix_floor = |x: i32| if x < 0 { (x - 65535) / 65536 } else { x / 65536 };
    let (mut lhs, mut rhs, mut top, mut bottom) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    for tp in &p[1..4] {
        bottom = bottom.max(tp.y);
        top = top.min(tp.y);
        let x = fix_floor(tp.x);
        rhs = rhs.max(x);
        lhs = lhs.min(x);
    }
    if bottom < 0 || top >= dheight || rhs < 0 || lhs >= dwidth {
        return 0;
    }
    let mut flats = 0;
    let mut flatg = 0usize;
    for i in 1..=3usize {
        let (mut p1, mut p2) = (i, if i == 3 { 1 } else { i + 1 });
        if p[p1].y > p[p2].y {
            core::mem::swap(&mut p1, &mut p2);
        }
        let tg = &mut g[i];
        tg.tx = p[p1].tx;
        tg.ty = p[p1].ty;
        tg.x = p[p1].x;
        tg.y1 = p[p1].y;
        tg.y2 = p[p2].y;
        let h = tg.y2 - tg.y1;
        if h == 0 {
            flats += 1;
            flatg = i;
        } else {
            tg.xi = (p[p2].x - p[p1].x) / h;
            tg.txi = p[p2].tx.wrapping_sub(p[p1].tx) / h;
            tg.tyi = p[p2].ty.wrapping_sub(p[p1].ty) / h;
        }
        tg.p2 = p2;
        tg.p1 = p1;
    }
    let (mut g1, mut g2, mut g3) = (1usize, 2usize, 3usize);
    let mut final_ = false;
    if flats == 0 {
        if g[g3].y1 < g[g1].y1 {
            core::mem::swap(&mut g1, &mut g3);
        }
        if g[g3].y1 < g[g2].y1 {
            core::mem::swap(&mut g2, &mut g3);
        }
    } else if flats == 1 {
        if flatg == 1 {
            core::mem::swap(&mut g1, &mut g3);
        }
        if flatg == 2 {
            core::mem::swap(&mut g2, &mut g3);
        }
        final_ = true;
    } else {
        let (mut lhs, mut rhs) = (i32::MAX, i32::MIN);
        let (mut lhsi, mut rhsi) = (0usize, 0usize);
        for ti in 1..=3usize {
            let x = p[ti].x;
            if x <= lhs {
                lhs = x;
                lhsi = ti;
            }
            if x >= rhs {
                rhs = x;
                rhsi = ti;
            }
        }
        g[1].x = lhs;
        g[2].x = rhs;
        g[1].tx = p[lhsi].tx;
        g[1].ty = p[lhsi].ty;
        g[2].tx = p[rhsi].tx;
        g[2].ty = p[rhsi].ty;
        final_ = true;
    }
    let y1 = g[g1].y1;
    let y2 = if g[g1].y2 > g[g2].y2 { g[g2].y2 } else { g[g1].y2 };
    let y = y2 - y1;
    let lhs1 = g[g1].x.wrapping_add(g[g1].xi.wrapping_mul(y) / 2);
    let lhs2 = g[g2].x.wrapping_add(g[g2].xi.wrapping_mul(y) / 2);
    if lhs1 > lhs2 {
        core::mem::swap(&mut g1, &mut g2);
    }
    let op = if src.bytes_per_pixel == 4 {
        if src.alpha_disabled != 0 || dst.alpha_disabled != 0 {
            Op::Copy32
        } else {
            Op::Blend32
        }
    } else if src.transparent_color == -1 {
        Op::Copy8
    } else {
        Op::Clear8
    };
    let ctx = Scan {
        src,
        dst,
        op,
        tile,
        swidth,
        sheight,
        dwidth,
        dheight,
        no_edge_overlap,
        transparent_color: src.transparent_color as u32,
    };
    // SAFETY: the scan converter clips to the destination; source texels come
    // from the (possibly tiled) texture coordinates, as in legacy.
    unsafe { ctx.run(&p, &mut g, g1, g2, g3, y1, y2, final_) };
    0
}

struct Scan<'a> {
    src: &'a ImgStruct,
    dst: &'a ImgStruct,
    op: Op,
    tile: bool,
    swidth: i32,
    sheight: i32,
    dwidth: i32,
    dheight: i32,
    no_edge_overlap: bool,
    transparent_color: u32,
}

impl Scan<'_> {
    #[inline]
    unsafe fn plot(&self, x: i32, y: i32, tx: i32, ty: i32) {
        let (sx, sy) = if self.tile { ((tx >> 16) % self.swidth, (ty >> 16) % self.sheight) } else { (tx >> 16, ty >> 16) };
        let soff = (sy.wrapping_mul(self.swidth).wrapping_add(sx)) as isize;
        let doff = (y.wrapping_mul(self.dwidth).wrapping_add(x)) as isize;
        // SAFETY: guaranteed by the caller of `run`.
        unsafe {
            match self.op {
                Op::Copy32 => *self.dst.offset32().offset(doff) = *self.src.offset32().offset(soff),
                Op::Blend32 => blend_onto(self.dst.offset32().offset(doff), *self.src.offset32().offset(soff)),
                Op::Copy8 => *self.dst.offset.offset(doff) = *self.src.offset.offset(soff),
                Op::Clear8 => {
                    let col = *self.src.offset.offset(soff);
                    if u32::from(col) != self.transparent_color {
                        *self.dst.offset.offset(doff) = col;
                    }
                }
            }
        }
    }

    /// Advances a gradient by `y` rows using the original edge endpoints.
    /// Legacy quirk: the endpoints of g2 are taken from g2 only when g1 had a
    /// non-zero height; otherwise g1's endpoints are reused.
    fn jump(p: &[Point; 4], g: &mut [Grad; 4], g1: usize, g2: usize, y: i32) {
        let (mut p1, mut p2) = (g[g1].p1, g[g1].p2);
        let d = g[g1].y2 - g[g1].y1;
        if d != 0 {
            let gg = &mut g[g1];
            gg.tx = gg.tx.wrapping_add((i64::from(p[p2].tx.wrapping_sub(p[p1].tx)) * i64::from(y) / i64::from(d)) as i32);
            gg.ty = gg.ty.wrapping_add((i64::from(p[p2].ty.wrapping_sub(p[p1].ty)) * i64::from(y) / i64::from(d)) as i32);
            gg.x = gg.x.wrapping_add((i64::from(p[p2].x.wrapping_sub(p[p1].x)) * i64::from(y) / i64::from(d)) as i32);
            p1 = g[g2].p1;
            p2 = g[g2].p2;
        }
        let d = g[g2].y2 - g[g2].y1;
        if d != 0 {
            let gg = &mut g[g2];
            gg.tx = gg.tx.wrapping_add((i64::from(p[p2].tx.wrapping_sub(p[p1].tx)) * i64::from(y) / i64::from(d)) as i32);
            gg.ty = gg.ty.wrapping_add((i64::from(p[p2].ty.wrapping_sub(p[p1].ty)) * i64::from(y) / i64::from(d)) as i32);
            gg.x = gg.x.wrapping_add((i64::from(p[p2].x.wrapping_sub(p[p1].x)) * i64::from(y) / i64::from(d)) as i32);
        }
    }

    unsafe fn run(
        &self,
        p: &[Point; 4],
        g: &mut [Grad; 4],
        mut g1: usize,
        mut g2: usize,
        g3: usize,
        mut y1: i32,
        mut y2: i32,
        mut final_: bool,
    ) {
        let (dwidth, dheight) = (self.dwidth, self.dheight);
        loop {
            // usegrad3:
            if final_ && self.no_edge_overlap {
                y2 -= 1;
            }
            if y1 >= dheight {
                return;
            }
            if y2 < 0 {
                if final_ {
                    return;
                }
                Self::jump(p, g, g1, g2, y2 - y1);
                // goto final
            } else {
                if y1 < 0 {
                    Self::jump(p, g, g1, g2, -y1);
                    y1 = 0;
                }
                if y2 >= dheight {
                    y2 = dheight - 1;
                }
                let (mut g1x, mut g2x) = (g[g1].x, g[g2].x);
                let (mut g1tx, mut g2tx) = (g[g1].tx, g[g2].tx);
                let (mut g1ty, mut g2ty) = (g[g1].ty, g[g2].ty);
                let (g1xi, g2xi) = (g[g1].xi, g[g2].xi);
                let (g1txi, g2txi) = (g[g1].txi, g[g2].txi);
                let (g1tyi, g2tyi) = (g[g1].tyi, g[g2].tyi);
                let floor16 = |v: i32| if v < 0 { v.wrapping_sub(65535) / 65536 } else { v / 65536 };
                let mut y = y1;
                while y <= y2 {
                    'row: {
                        let mut x1 = floor16(g1x);
                        let mut x2 = floor16(g2x);
                        if x1 >= dwidth || x2 < 0 {
                            break 'row;
                        }
                        let (mut tx, mut ty) = (g1tx, g1ty);
                        let (txi, tyi) = if x1 != x2 {
                            let d = i64::from(g2x.wrapping_sub(g1x));
                            (
                                ((i64::from(g2tx.wrapping_sub(g1tx)) << 16) / d) as i32,
                                ((i64::from(g2ty.wrapping_sub(g1ty)) << 16) / d) as i32,
                            )
                        } else {
                            (0, 0)
                        };
                        let mut loff = (g1x & 65535) - 32768;
                        let roff = (g2x & 65535) - 32768;
                        if roff < 0 {
                            if x2 < dwidth && !self.no_edge_overlap {
                                // SAFETY: 0 <= x2 < dwidth, 0 <= y < dheight.
                                unsafe { self.plot(x2, y, g2tx, g2ty) };
                            }
                            x2 -= 1;
                            if x1 > x2 || x2 < 0 {
                                break 'row;
                            }
                        } else if self.no_edge_overlap {
                            x2 -= 1;
                            if x1 > x2 || x2 < 0 {
                                break 'row;
                            }
                        }
                        if loff > 0 {
                            if x1 >= 0 {
                                // SAFETY: 0 <= x1 <= x2 < ... (x1 < dwidth checked).
                                unsafe { self.plot(x1, y, tx, ty) };
                            }
                            x1 += 1;
                            if x1 > x2 {
                                break 'row;
                            }
                            loff = -(65536 - loff);
                        }
                        let i = i64::from(-loff);
                        tx = tx.wrapping_add((i * i64::from(txi) / 65536) as i32);
                        ty = ty.wrapping_add((i * i64::from(tyi) / 65536) as i32);
                        if x1 < 0 {
                            let d = i64::from(g2x.wrapping_sub(g1x));
                            let n = i64::from(-x1);
                            tx = tx.wrapping_add(((i64::from(g2tx.wrapping_sub(g1tx)) << 16).wrapping_mul(n) / d) as i32);
                            ty = ty.wrapping_add(((i64::from(g2ty.wrapping_sub(g1ty)) << 16).wrapping_mul(n) / d) as i32);
                            x1 = 0;
                        }
                        if x2 >= dwidth {
                            x2 = dwidth - 1;
                        }
                        for x in x1..=x2 {
                            // SAFETY: clipped to the destination row.
                            unsafe { self.plot(x, y, tx, ty) };
                            tx = tx.wrapping_add(txi);
                            ty = ty.wrapping_add(tyi);
                        }
                    }
                    // donerow:
                    if y != y2 {
                        g1x = g1x.wrapping_add(g1xi);
                        g1tx = g1tx.wrapping_add(g1txi);
                        g1ty = g1ty.wrapping_add(g1tyi);
                        g2x = g2x.wrapping_add(g2xi);
                        g2tx = g2tx.wrapping_add(g2txi);
                        g2ty = g2ty.wrapping_add(g2tyi);
                    }
                    y += 1;
                }
                if final_ {
                    return;
                }
                g[g1].x = g1x;
                g[g2].x = g2x;
                g[g1].tx = g1tx;
                g[g2].tx = g2tx;
                g[g1].ty = g1ty;
                g[g2].ty = g2ty;
            }
            // final:
            if y2 < dheight - 1 {
                if g[g1].y2 < g[g2].y2 {
                    g1 = g3;
                } else {
                    g2 = g3;
                }
                y1 = g[g3].y1 + 1;
                y2 = g[g3].y2;
                for k in [g1, g2] {
                    let gg = &mut g[k];
                    gg.x = gg.x.wrapping_add(gg.xi);
                    gg.tx = gg.tx.wrapping_add(gg.txi);
                    gg.ty = gg.ty.wrapping_add(gg.tyi);
                }
                final_ = true;
                continue;
            }
            return;
        }
    }
}

/// C ABI: s = sx1,sy1,sx2,sy2,sx3,sy3; d = dx1,dy1,...; `seamless` 0/1.
///
/// # Safety
/// See [`maptriangle`]; `s` and `d` readable for 6 floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_gfx_maptriangle(
    src: *const ImgStruct,
    dst: *const ImgStruct,
    s: *const f32,
    d: *const f32,
    seamless: i32,
) -> i32 {
    // SAFETY: guaranteed by the caller.
    unsafe {
        let s = *(s as *const [f32; 6]);
        let d = *(d as *const [f32; 6]);
        maptriangle(&*src, &*dst, s, d, seamless != 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::testutil::TestImage;

    fn checker(mode: u16) -> Box<TestImage> {
        let t = TestImage::new(8, 8, mode);
        for y in 0..8 {
            for x in 0..8 {
                let c = if (x + y) % 2 == 0 { 1 } else { 2 };
                unsafe { crate::gfx::blend::pset(&t.img, x, y, if mode == 32 { 0xFF00_0000 | c } else { c }) };
            }
        }
        t
    }

    #[test]
    fn identity_mapping_copies_triangle() {
        let s = checker(13);
        let d = TestImage::new(16, 16, 13);
        let r = unsafe { maptriangle(&s.img, &d.img, [0.0, 0.0, 7.0, 0.0, 0.0, 7.0], [0.0, 0.0, 7.0, 0.0, 0.0, 7.0], false) };
        assert_eq!(r, 0);
        assert_eq!((d.p8(0, 0), d.p8(1, 0), d.p8(0, 1)), (1, 2, 2));
        assert_eq!(d.p8(7, 7), 0); // outside the triangle
        assert!(d.count_nonzero() > 20);
    }

    #[test]
    fn errors_and_offscreen() {
        let s = checker(13);
        let d = TestImage::new(16, 16, 32);
        assert_eq!(unsafe { maptriangle(&s.img, &d.img, [0.0; 6], [0.0; 6], false) }, 5); // depth mismatch
        let d = TestImage::new(16, 16, 13);
        assert_eq!(unsafe { maptriangle(&s.img, &d.img, [0.0, 0.0, 7.0, 0.0, 0.0, 7.0], [0.0, 0.0, 20000.0, 0.0, 0.0, 5.0], false) }, 5);
        assert_eq!(unsafe { maptriangle(&s.img, &d.img, [0.0, 0.0, 7.0, 0.0, 0.0, 7.0], [-50.0, -50.0, -40.0, -50.0, -50.0, -40.0], false) }, 0);
        assert_eq!(d.count_nonzero(), 0);
    }

    #[test]
    fn clear_color_and_32bit() {
        let mut s = checker(13);
        s.img.transparent_color = 2;
        let d = TestImage::new(16, 16, 13);
        unsafe { maptriangle(&s.img, &d.img, [0.0, 0.0, 7.0, 0.0, 0.0, 7.0], [0.0, 0.0, 7.0, 0.0, 0.0, 7.0], false) };
        assert_eq!((d.p8(0, 0), d.p8(1, 0)), (1, 0));
        let s = checker(32);
        let d = TestImage::new(16, 16, 32);
        unsafe { maptriangle(&s.img, &d.img, [0.0, 0.0, 7.0, 0.0, 0.0, 7.0], [0.0, 0.0, 14.0, 0.0, 0.0, 14.0], false) };
        assert_eq!(d.p32(0, 0), 0xFF00_0001);
    }
}
