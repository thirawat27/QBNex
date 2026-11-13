//! Software presentation of the SCREEN in the window (what QB64 did with OpenGL).
//!
//! * [`layout`] is a port of `prepare_environment_2d()`: where the SCREEN goes
//!   inside the window (stretch, letterbox, top-left).
//! * [`put`], [`tri2d`], [`tri3d`] render "hardware" images (`_COPYIMAGE(,33)`,
//!   `_PUTIMAGE`/`_MAPTRIANGLE` onto `_DISPLAY`) in software. Legacy drew them
//!   with OpenGL textures, so results can differ from legacy in edge pixels and
//!   filtering (documented deviation).
//! * [`scale_to_window`] draws the composed frame into the window buffer.
//!
//! Pixels are `0xAARRGGBB` (`uint32` BGRA in memory), like `img_struct`.

/// Result of [`layout`] (the `environment_2d__*` globals).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
    pub scaled_w: i32,
    pub scaled_h: i32,
    pub x_scale: f32,
    pub y_scale: f32,
    pub smooth: bool,
    /// 0 none, 1 vertical stripes, 2 horizontal stripes.
    pub letterbox: i32,
}

/// QB64's `prepare_environment_2d()`. `full_screen`: 0 windowed,
/// 1 stretched, 2 square pixels. `resize_auto`: 0 off, 1 `_STRETCH`, 2 `_SMOOTH`.
pub fn layout(
    screen_w: i32,
    screen_h: i32,
    window_w: i32,
    window_h: i32,
    full_screen: i32,
    fullscreen_smooth: bool,
    resize_auto: i32,
) -> Layout {
    let mut can_scale = false;
    let mut need_square_pixels = false;
    let mut smooth = false;
    let mut letterbox = 0;
    if full_screen > 0 {
        can_scale = true;
        if full_screen == 2 {
            need_square_pixels = true;
        }
        smooth = fullscreen_smooth;
    } else if resize_auto > 0 {
        can_scale = true;
        if resize_auto == 2 {
            smooth = true;
        }
    }
    if screen_w == window_w && screen_h == window_h {
        return Layout {
            x1: 0,
            y1: 0,
            x2: screen_w - 1,
            y2: screen_h - 1,
            scaled_w: screen_w,
            scaled_h: screen_h,
            x_scale: 1.0,
            y_scale: 1.0,
            smooth: false,
            letterbox,
        };
    }
    let window_ratio = window_w as f32 / window_h as f32;
    let screen_ratio = screen_w as f32 / screen_h as f32;
    let (x1, y1, x2, y2);
    if !can_scale {
        // top-left of the window, blank space on the bottom & right
        (x1, y1, x2, y2) = (0, 0, screen_w - 1, screen_h - 1);
    } else if !need_square_pixels || window_ratio == screen_ratio {
        (x1, y1, x2, y2) = (0, 0, window_w - 1, window_h - 1);
    } else if window_ratio > screen_ratio {
        let z = (window_h as f32 * screen_ratio) as i32; // new width
        x1 = window_w / 2 - z / 2;
        x2 = x1 + z - 1;
        (y1, y2) = (0, window_h - 1);
        letterbox = 1;
    } else {
        let z = (window_w as f32 / screen_ratio) as i32; // new height
        y1 = window_h / 2 - z / 2;
        y2 = y1 + z - 1;
        (x1, x2) = (0, window_w - 1);
        letterbox = 2;
    }
    let scaled_w = x2 - x1 + 1;
    let scaled_h = y2 - y1 + 1;
    Layout {
        x1,
        y1,
        x2,
        y2,
        scaled_w,
        scaled_h,
        x_scale: scaled_w as f32 / screen_w as f32,
        y_scale: scaled_h as f32 / screen_h as f32,
        smooth,
        letterbox,
    }
}

/// A mutable 32-bit image.
pub struct Target<'a> {
    pub px: &'a mut [u32],
    pub w: usize,
    pub h: usize,
}

/// A read-only 32-bit image.
#[derive(Clone, Copy)]
pub struct Source<'a> {
    pub px: &'a [u32],
    pub w: usize,
    pub h: usize,
}

/// QB64's `DEPTHBUFFER_MODE__*` / `CULL_MODE__*` values.
pub const DEPTH_OFF: i32 = 0;
pub const DEPTH_ON: i32 = 1;
pub const DEPTH_LOCKED: i32 = 2;
pub const CULL_NONE: i32 = 0;
pub const CULL_CLOCKWISE_ONLY: i32 = 1;
pub const CULL_ANTICLOCKWISE_ONLY: i32 = 2;

/// Blends `s` over `d` like `glBlendFuncSeparate(SRC_ALPHA, ONE_MINUS_SRC_ALPHA, ONE, ONE)`.
#[inline]
pub fn blend(d: u32, s: u32) -> u32 {
    let sa = s >> 24;
    if sa == 255 {
        return s;
    }
    let da = d >> 24;
    let ch = |shift: u32| {
        let sc = (s >> shift) & 255;
        let dc = (d >> shift) & 255;
        ((sc * sa + dc * (255 - sa) + 127) / 255) << shift
    };
    ((sa + da).min(255) << 24) | ch(16) | ch(8) | ch(0)
}

#[inline]
fn wrap_or_clamp(i: i64, n: usize, wrap: bool) -> usize {
    if wrap {
        i.rem_euclid(n as i64) as usize
    } else {
        i.clamp(0, n as i64 - 1) as usize
    }
}

/// Samples `src` at texel coordinates (`u`,`v`), texel centres at +0.5.
fn sample(src: &Source, u: f32, v: f32, smooth: bool, wrap: bool) -> u32 {
    if src.w == 0 || src.h == 0 {
        return 0;
    }
    if !smooth {
        let x = wrap_or_clamp(u.floor() as i64, src.w, wrap);
        let y = wrap_or_clamp(v.floor() as i64, src.h, wrap);
        return src.px[y * src.w + x];
    }
    let fu = u - 0.5;
    let fv = v - 0.5;
    let x0 = fu.floor();
    let y0 = fv.floor();
    let ax = fu - x0;
    let ay = fv - y0;
    let xa = wrap_or_clamp(x0 as i64, src.w, wrap);
    let xb = wrap_or_clamp(x0 as i64 + 1, src.w, wrap);
    let ya = wrap_or_clamp(y0 as i64, src.h, wrap);
    let yb = wrap_or_clamp(y0 as i64 + 1, src.h, wrap);
    let p = [
        src.px[ya * src.w + xa],
        src.px[ya * src.w + xb],
        src.px[yb * src.w + xa],
        src.px[yb * src.w + xb],
    ];
    let mut out = 0u32;
    for shift in [0u32, 8, 16, 24] {
        let c = |q: u32| ((q >> shift) & 255) as f32;
        let top = c(p[0]) + (c(p[1]) - c(p[0])) * ax;
        let bot = c(p[2]) + (c(p[3]) - c(p[2])) * ax;
        let val = (top + (bot - top) * ay).round().clamp(0.0, 255.0) as u32;
        out |= val << shift;
    }
    out
}

#[inline]
fn plot(dst: &mut Target, x: usize, y: usize, c: u32, use_alpha: bool) {
    let i = y * dst.w + x;
    dst.px[i] = if use_alpha { blend(dst.px[i], c) } else { c };
}

/// Screen-space vertex: position, texel coordinates, 1/w (1 for 2D).
#[derive(Clone, Copy)]
struct Vtx {
    x: f32,
    y: f32,
    u: f32,
    v: f32,
    q: f32,
}

/// Fills a triangle with pixel-centre sampling and the top-left rule.
/// Texture coordinates are interpolated perspective-correctly through `q`.
fn raster_tri(
    dst: &mut Target,
    mut depth: Option<(&mut [f32], i32)>,
    src: &Source,
    v: [Vtx; 3],
    use_alpha: bool,
    smooth: bool,
    wrap: bool,
) {
    let [mut a, mut b, c] = v;
    let mut area = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    if area == 0.0 || !area.is_finite() {
        return;
    }
    if area < 0.0 {
        std::mem::swap(&mut a, &mut b);
        area = -area;
    }
    let edge = |p: &Vtx, q: &Vtx, x: f32, y: f32| (q.x - p.x) * (y - p.y) - (q.y - p.y) * (x - p.x);
    let top_left = |p: &Vtx, q: &Vtx| {
        let (dx, dy) = (q.x - p.x, q.y - p.y);
        dy < 0.0 || (dy == 0.0 && dx > 0.0)
    };
    let tl = [top_left(&b, &c), top_left(&c, &a), top_left(&a, &b)];
    let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as i64;
    let max_x = (a.x.max(b.x).max(c.x).ceil() as i64).min(dst.w as i64 - 1);
    let min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as i64;
    let max_y = (a.y.max(b.y).max(c.y).ceil() as i64).min(dst.h as i64 - 1);
    let inside = |w: f32, t: bool| w > 0.0 || (w == 0.0 && t);
    for y in min_y..=max_y {
        let py = y as f32 + 0.5;
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let w0 = edge(&b, &c, px, py);
            let w1 = edge(&c, &a, px, py);
            let w2 = edge(&a, &b, px, py);
            if !(inside(w0, tl[0]) && inside(w1, tl[1]) && inside(w2, tl[2])) {
                continue;
            }
            let (l0, l1, l2) = (w0 / area, w1 / area, w2 / area);
            let q = l0 * a.q + l1 * b.q + l2 * c.q;
            if q <= 0.0 {
                continue;
            }
            let i = y as usize * dst.w + x as usize;
            if let Some((ref mut zb, mode)) = depth {
                let z = 1.0 / q;
                if mode != DEPTH_OFF {
                    if z > zb[i] {
                        continue;
                    }
                    if mode == DEPTH_ON {
                        zb[i] = z;
                    }
                }
            }
            let u = (l0 * a.u * a.q + l1 * b.u * b.q + l2 * c.u * c.q) / q;
            let vv = (l0 * a.v * a.q + l1 * b.v * b.q + l2 * c.v * c.q) / q;
            let col = sample(src, u, vv, smooth, wrap);
            plot(dst, x as usize, y as usize, col, use_alpha);
        }
    }
}

/// `hardware_img_put`: copies the inclusive source rectangle onto the
/// inclusive destination rectangle (either may be mirrored), scaling as needed.
#[allow(clippy::too_many_arguments)]
pub fn put(dst: &mut Target, src: Source, d: [i32; 4], s: [i32; 4], use_alpha: bool, smooth: bool) {
    let [mut dx1, mut dy1, mut dx2, mut dy2] = d;
    let [mut sx1, mut sy1, mut sx2, mut sy2] = s;
    // ensure dst_x1/y1 represent the top-left co-ordinate of the destination
    if dx2 < dx1 {
        std::mem::swap(&mut dx1, &mut dx2);
        std::mem::swap(&mut sx1, &mut sx2);
    }
    if dy2 < dy1 {
        std::mem::swap(&mut dy1, &mut dy2);
        std::mem::swap(&mut sy1, &mut sy2);
    }
    // texture co-ordinates as legacy computed them (in texel units)
    let (u1, u2) = if sx1 <= sx2 {
        (sx1 as f32 + 0.01, sx2 as f32 + 0.99)
    } else {
        (sx1 as f32 + 0.99, sx2 as f32 + 0.01)
    };
    let (v1, v2) = if sy1 <= sy2 {
        (sy1 as f32 + 0.01, sy2 as f32 + 0.99)
    } else {
        (sy1 as f32 + 0.99, sy2 as f32 + 0.01)
    };
    let qw = (dx2 + 1 - dx1) as f32;
    let qh = (dy2 + 1 - dy1) as f32;
    let x_lo = dx1.max(0);
    let x_hi = dx2.min(dst.w as i32 - 1);
    let y_lo = dy1.max(0);
    let y_hi = dy2.min(dst.h as i32 - 1);
    for y in y_lo..=y_hi {
        let v = v1 + ((y - dy1) as f32 + 0.5) / qh * (v2 - v1);
        for x in x_lo..=x_hi {
            let u = u1 + ((x - dx1) as f32 + 0.5) / qw * (u2 - u1);
            let c = sample(&src, u, v, smooth, false);
            plot(dst, x as usize, y as usize, c, use_alpha);
        }
    }
}

/// `hardware_img_tri2d`: textured triangle in destination pixel space.
pub fn tri2d(
    dst: &mut Target,
    src: Source,
    d: [(f32, f32); 3],
    s: [(f32, f32); 3],
    use_alpha: bool,
    smooth: bool,
) {
    let mk = |i: usize| Vtx {
        x: d[i].0,
        y: d[i].1,
        u: s[i].0 + 0.5,
        v: s[i].1 + 0.5,
        q: 1.0,
    };
    raster_tri(
        dst,
        None,
        &src,
        [mk(0), mk(1), mk(2)],
        use_alpha,
        smooth,
        false,
    );
}

const NEAR: f32 = 0.1;

/// `hardware_img_tri3d`: perspective textured triangle (eye space, camera at
/// the origin looking down -Z, 90 degree field of view on the shorter side).
#[allow(clippy::too_many_arguments)]
pub fn tri3d(
    dst: &mut Target,
    depth: &mut [f32],
    src: Source,
    d: [(f32, f32, f32); 3],
    s: [(f32, f32); 3],
    use_alpha: bool,
    smooth: bool,
    cull: i32,
    depth_mode: i32,
) {
    let (w, h) = (dst.w as f32, dst.h as f32);
    if dst.w == 0 || dst.h == 0 || depth.len() < dst.px.len() {
        return;
    }
    // gluPerspective(fov, w/h, 0.1, 10000) as set_view(VIEW_MODE__3D) did
    let fov = if dst.w > dst.h { 90.0 * h / w } else { 90.0f32 };
    let f = 1.0 / (fov.to_radians() / 2.0).tan();
    let aspect = w / h;
    // clip against the near plane (z = -NEAR) in eye space
    let mut poly: Vec<([f32; 3], [f32; 2])> = Vec::with_capacity(4);
    for i in 0..3 {
        let (p, pt) = (d[i], s[i]);
        let (n, nt) = (d[(i + 1) % 3], s[(i + 1) % 3]);
        let p_in = -p.2 >= NEAR;
        let n_in = -n.2 >= NEAR;
        if p_in {
            poly.push(([p.0, p.1, p.2], [pt.0, pt.1]));
        }
        if p_in != n_in {
            let t = (-NEAR - p.2) / (n.2 - p.2);
            let lerp = |a: f32, b: f32| a + (b - a) * t;
            poly.push((
                [lerp(p.0, n.0), lerp(p.1, n.1), -NEAR],
                [lerp(pt.0, nt.0), lerp(pt.1, nt.1)],
            ));
        }
    }
    if poly.len() < 3 {
        return;
    }
    let proj = |e: &([f32; 3], [f32; 2])| {
        let wc = -e.0[2];
        Vtx {
            x: (f / aspect * e.0[0] / wc + 1.0) * 0.5 * w,
            y: (1.0 - f * e.0[1] / wc) * 0.5 * h,
            u: e.1[0] + 0.5,
            v: e.1[1] + 0.5,
            q: 1.0 / wc,
        }
    };
    let pv: Vec<Vtx> = poly.iter().map(proj).collect();
    // winding on screen (y down): positive area = clockwise
    let area = {
        let (a, b, c) = (pv[0], pv[1], pv[2]);
        (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
    };
    if (cull == CULL_CLOCKWISE_ONLY && area < 0.0)
        || (cull == CULL_ANTICLOCKWISE_ONLY && area > 0.0)
    {
        return;
    }
    for k in 1..pv.len() - 1 {
        raster_tri(
            dst,
            Some((&mut *depth, depth_mode)),
            &src,
            [pv[0], pv[k], pv[k + 1]],
            use_alpha,
            smooth,
            true,
        );
    }
}

/// Draws the composed `frame` into the window buffer `win` (0x00RRGGBB) using
/// `l`; everything outside the SCREEN rectangle is black.
pub fn scale_to_window(win: &mut Target, frame: Source, l: &Layout) {
    win.px.fill(0);
    if frame.w == 0 || frame.h == 0 || l.scaled_w <= 0 || l.scaled_h <= 0 {
        return;
    }
    let same = l.scaled_w as usize == frame.w && l.scaled_h as usize == frame.h;
    let x_lo = l.x1.max(0);
    let x_hi = l.x2.min(win.w as i32 - 1);
    let y_lo = l.y1.max(0);
    let y_hi = l.y2.min(win.h as i32 - 1);
    let sx = frame.w as f32 / l.scaled_w as f32;
    let sy = frame.h as f32 / l.scaled_h as f32;
    for y in y_lo..=y_hi {
        let row = y as usize * win.w;
        let v = ((y - l.y1) as f32 + 0.5) * sy;
        for x in x_lo..=x_hi {
            let c = if same {
                frame.px[(y - l.y1) as usize * frame.w + (x - l.x1) as usize]
            } else {
                let u = ((x - l.x1) as f32 + 0.5) * sx;
                sample(&frame, u, v, l.smooth, false)
            };
            win.px[row + x as usize] = c & 0x00FF_FFFF;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: u32 = 0xFFFF0000;
    const G: u32 = 0xFF00FF00;
    const B: u32 = 0xFF0000FF;
    const W: u32 = 0xFFFFFFFF;

    #[test]
    fn layout_identity_when_sizes_match() {
        let l = layout(640, 480, 640, 480, 0, false, 0);
        assert_eq!((l.x1, l.y1, l.x2, l.y2), (0, 0, 639, 479));
        assert_eq!(
            (l.scaled_w, l.scaled_h, l.x_scale, l.y_scale),
            (640, 480, 1.0, 1.0)
        );
        assert!(!l.smooth);
        assert_eq!(l.letterbox, 0);
    }

    #[test]
    fn layout_cannot_scale_in_plain_window() {
        let l = layout(320, 200, 640, 400, 0, false, 0);
        assert_eq!((l.x1, l.y1, l.x2, l.y2), (0, 0, 319, 199));
        assert_eq!((l.scaled_w, l.scaled_h), (320, 200));
        assert_eq!(l.x_scale, 1.0);
    }

    #[test]
    fn layout_stretch_and_letterbox() {
        let l = layout(320, 200, 640, 400, 0, false, 2);
        assert_eq!((l.x1, l.y1, l.x2, l.y2), (0, 0, 639, 399));
        assert!(l.smooth);
        assert_eq!((l.x_scale, l.y_scale), (2.0, 2.0));
        // full screen, square pixels, wide monitor: vertical stripes
        let l = layout(640, 480, 1920, 1080, 2, false, 0);
        assert_eq!(l.letterbox, 1);
        assert_eq!((l.x1, l.x2, l.y1, l.y2), (240, 1679, 0, 1079));
        assert_eq!(l.scaled_w, 1440);
        // tall window: horizontal stripes
        let l = layout(640, 400, 800, 800, 2, true, 0);
        assert_eq!(l.letterbox, 2);
        assert_eq!((l.y1, l.y2), (150, 649));
        assert!(l.smooth);
    }

    #[test]
    fn blend_alpha() {
        assert_eq!(blend(R, G), G);
        assert_eq!(blend(R, 0x0000FF00), 0xFFFF0000);
        let half = blend(0xFF000000, 0x80FFFFFF);
        assert_eq!(half & 0x00FFFFFF, 0x00808080);
        assert_eq!(half >> 24, 0xFF);
        assert_eq!(blend(0x10000000, 0x20000000) >> 24, 0x30);
    }

    #[test]
    fn put_copies_scales_and_mirrors() {
        let src = [R, G, B, W];
        let s = Source {
            px: &src,
            w: 2,
            h: 2,
        };
        let mut buf = vec![0u32; 16];
        let mut t = Target {
            px: &mut buf,
            w: 4,
            h: 4,
        };
        put(&mut t, s, [0, 0, 1, 1], [0, 0, 1, 1], false, false);
        assert_eq!(&buf[0..2], &[R, G]);
        assert_eq!(&buf[4..6], &[B, W]);
        let mut buf = vec![0u32; 16];
        let mut t = Target {
            px: &mut buf,
            w: 4,
            h: 4,
        };
        put(&mut t, s, [0, 0, 3, 3], [0, 0, 1, 1], false, false);
        assert_eq!(&buf[0..4], &[R, R, G, G]);
        assert_eq!(&buf[12..16], &[B, B, W, W]);
        let mut buf = vec![0u32; 16];
        let mut t = Target {
            px: &mut buf,
            w: 4,
            h: 4,
        };
        put(&mut t, s, [1, 0, 0, 0], [0, 0, 1, 0], false, false); // mirrored
        assert_eq!(&buf[0..2], &[G, R]);
        // clipped, no panic
        let mut t = Target {
            px: &mut buf,
            w: 4,
            h: 4,
        };
        put(&mut t, s, [-5, -5, 10, 10], [0, 0, 1, 1], true, true);
    }

    #[test]
    fn put_alpha_keeps_background_for_transparent_texels() {
        let src = [0x00FF0000u32];
        let mut buf = vec![B; 4];
        let mut t = Target {
            px: &mut buf,
            w: 2,
            h: 2,
        };
        put(
            &mut t,
            Source {
                px: &src,
                w: 1,
                h: 1,
            },
            [0, 0, 1, 1],
            [0, 0, 0, 0],
            true,
            false,
        );
        assert_eq!(buf, vec![B; 4]);
        let mut t = Target {
            px: &mut buf,
            w: 2,
            h: 2,
        };
        put(
            &mut t,
            Source {
                px: &src,
                w: 1,
                h: 1,
            },
            [0, 0, 1, 1],
            [0, 0, 0, 0],
            false,
            false,
        );
        assert_eq!(buf, vec![0x00FF0000; 4]);
    }

    #[test]
    fn tri2d_fills_half_square() {
        let src = [G];
        let mut buf = vec![0u32; 16];
        let mut t = Target {
            px: &mut buf,
            w: 4,
            h: 4,
        };
        let s = Source {
            px: &src,
            w: 1,
            h: 1,
        };
        tri2d(
            &mut t,
            s,
            [(0.0, 0.0), (4.0, 0.0), (0.0, 4.0)],
            [(0.0, 0.0); 3],
            false,
            false,
        );
        let filled = buf.iter().filter(|&&p| p == G).count();
        // pixel centres strictly inside or on top/left edges (GL fill rule)
        assert_eq!(filled, 6, "{buf:x?}");
        assert_eq!(buf[0], G);
        assert_eq!(buf[15], 0);
    }

    #[test]
    fn tri3d_projects_and_depth_tests() {
        let red = [R];
        let green = [G];
        let mut buf = vec![0u32; 64 * 64];
        let mut depth = vec![f32::INFINITY; 64 * 64];
        let mut t = Target {
            px: &mut buf,
            w: 64,
            h: 64,
        };
        // big triangle at z=-10 covering the centre
        let near = [
            (-10.0, -10.0, -10.0),
            (10.0, -10.0, -10.0),
            (0.0, 10.0, -10.0),
        ];
        let far = [
            (-100.0, -100.0, -100.0),
            (100.0, -100.0, -100.0),
            (0.0, 100.0, -100.0),
        ];
        tri3d(
            &mut t,
            &mut depth,
            Source {
                px: &red,
                w: 1,
                h: 1,
            },
            near,
            [(0.0, 0.0); 3],
            false,
            false,
            CULL_NONE,
            DEPTH_ON,
        );
        tri3d(
            &mut t,
            &mut depth,
            Source {
                px: &green,
                w: 1,
                h: 1,
            },
            far,
            [(0.0, 0.0); 3],
            false,
            false,
            CULL_NONE,
            DEPTH_ON,
        );
        assert_eq!(buf[32 * 64 + 32], R, "nearer triangle wins");
        // same triangle without depth test overwrites
        let mut t = Target {
            px: &mut buf,
            w: 64,
            h: 64,
        };
        tri3d(
            &mut t,
            &mut depth,
            Source {
                px: &green,
                w: 1,
                h: 1,
            },
            far,
            [(0.0, 0.0); 3],
            false,
            false,
            CULL_NONE,
            DEPTH_OFF,
        );
        assert_eq!(buf[32 * 64 + 32], G);
        // behind the camera: nothing drawn, no panic
        let mut buf2 = vec![0u32; 64 * 64];
        let mut t = Target {
            px: &mut buf2,
            w: 64,
            h: 64,
        };
        let behind = [(-1.0, -1.0, 5.0), (1.0, -1.0, 5.0), (0.0, 1.0, 5.0)];
        tri3d(
            &mut t,
            &mut depth,
            Source {
                px: &green,
                w: 1,
                h: 1,
            },
            behind,
            [(0.0, 0.0); 3],
            false,
            false,
            CULL_NONE,
            DEPTH_OFF,
        );
        assert!(buf2.iter().all(|&p| p == 0));
        // culling: anticlockwise-only drops this (screen clockwise) triangle
        let mut t = Target {
            px: &mut buf2,
            w: 64,
            h: 64,
        };
        let cw = [
            (-10.0, 10.0, -10.0),
            (10.0, 10.0, -10.0),
            (0.0, -10.0, -10.0),
        ];
        tri3d(
            &mut t,
            &mut depth,
            Source {
                px: &green,
                w: 1,
                h: 1,
            },
            cw,
            [(0.0, 0.0); 3],
            false,
            false,
            CULL_ANTICLOCKWISE_ONLY,
            DEPTH_OFF,
        );
        assert!(buf2.iter().all(|&p| p == 0));
        let mut t = Target {
            px: &mut buf2,
            w: 64,
            h: 64,
        };
        tri3d(
            &mut t,
            &mut depth,
            Source {
                px: &green,
                w: 1,
                h: 1,
            },
            cw,
            [(0.0, 0.0); 3],
            false,
            false,
            CULL_CLOCKWISE_ONLY,
            DEPTH_OFF,
        );
        assert!(buf2.contains(&G));
    }

    #[test]
    fn scale_to_window_nearest_and_black_border() {
        let frame = [R, G, B, W];
        let mut win = vec![0x123456u32; 6 * 4];
        let l = layout(2, 2, 6, 4, 0, false, 0); // can't scale: top-left
        scale_to_window(
            &mut Target {
                px: &mut win,
                w: 6,
                h: 4,
            },
            Source {
                px: &frame,
                w: 2,
                h: 2,
            },
            &l,
        );
        assert_eq!(&win[0..3], &[0xFF0000, 0x00FF00, 0]);
        assert_eq!(win[23], 0);
        let l = layout(2, 2, 4, 4, 0, false, 1); // stretch
        let mut win = vec![0u32; 16];
        scale_to_window(
            &mut Target {
                px: &mut win,
                w: 4,
                h: 4,
            },
            Source {
                px: &frame,
                w: 2,
                h: 2,
            },
            &l,
        );
        assert_eq!(&win[0..4], &[0xFF0000, 0xFF0000, 0x00FF00, 0x00FF00]);
        assert_eq!(win[15], 0xFFFFFF);
    }
}
