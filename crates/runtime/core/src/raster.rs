//! Software presentation of the SCREEN in the window (what QB64 did with OpenGL).
//!
//! * [`layout`] is a port of `prepare_environment_2d()`: where the SCREEN goes
//!   inside the window (stretch, letterbox, top-left).
//! * [`tri3d`] renders "hardware" images (`_COPYIMAGE(,33)`, `_MAPTRIANGLE`
//!   onto `_DISPLAY`) in software. Legacy drew them with OpenGL textures, so
//!   results can differ from legacy in edge pixels and filtering (documented
//!   deviation).
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
    let n = n as i64;
    // Most lookups land inside the image; skip the 64-bit division then.
    if (0..n).contains(&i) {
        return i as usize;
    }
    if wrap {
        if n & (n - 1) == 0 {
            // two's complement masking equals rem_euclid for powers of two
            (i & (n - 1)) as usize
        } else {
            i.rem_euclid(n) as usize
        }
    } else {
        i.clamp(0, n - 1) as usize
    }
}

/// Samples `src` at texel coordinates (`u`,`v`), texel centres at +0.5.
#[inline]
fn sample(src: &Source, u: f32, v: f32, smooth: bool, wrap: bool) -> u32 {
    if src.w == 0 || src.h == 0 {
        return 0;
    }
    if smooth {
        sample_linear(src, u, v, wrap)
    } else {
        sample_nearest(src, u, v, wrap)
    }
}

/// [`sample`] without smoothing, for a non-empty `src`.
#[inline(always)]
fn sample_nearest(src: &Source, u: f32, v: f32, wrap: bool) -> u32 {
    let x = wrap_or_clamp(floor_i64(u), src.w, wrap);
    let y = wrap_or_clamp(floor_i64(v), src.h, wrap);
    src.px[y * src.w + x]
}

/// [`sample`] with bilinear smoothing, for a non-empty `src`.
#[inline(always)]
fn sample_linear(src: &Source, u: f32, v: f32, wrap: bool) -> u32 {
    let fu = u - 0.5;
    let fv = v - 0.5;
    let x0 = floor(fu);
    let y0 = floor(fv);
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
    // channels as lanes (blue, green, red, alpha), each with the same
    // operations as one channel at a time
    let ch = |q: u32| [q & 255, (q >> 8) & 255, (q >> 16) & 255, q >> 24].map(|c| c as f32);
    let [c0, c1, c2, c3] = p.map(ch);
    let mut out = 0u32;
    for k in 0..4 {
        let top = c0[k] + (c1[k] - c0[k]) * ax;
        let bot = c2[k] + (c3[k] - c2[k]) * ax;
        let val = round(top + (bot - top) * ay).clamp(0.0, 255.0) as u32;
        out |= val << (8 * k);
    }
    out
}

/// Floats from 2^23 up have no fraction bits.
const NO_FRACTION: f32 = 8_388_608.0;

/// `x.floor() as i64` without going through a float floor.
#[inline]
fn floor_i64(x: f32) -> i64 {
    if x.abs() < NO_FRACTION {
        let t = x as i32; // exact truncation
        (t - ((t as f32) > x) as i32) as i64
    } else {
        // already integral; NaN gives 0 and infinities saturate as before
        x as i64
    }
}

/// `f32::floor`, bit-identical (signed zero included). Baseline x86 has no
/// rounding instruction, so `floor` there is a math library call per texel.
#[inline]
fn floor(x: f32) -> f32 {
    if !cfg!(any(target_arch = "x86", target_arch = "x86_64")) {
        return x.floor();
    }
    if x.is_nan() || x.abs() >= NO_FRACTION {
        return x;
    }
    let t = x as i32 as f32; // exact truncation
    // branch-free: the fraction is unpredictable per texel
    (t - (t > x) as i32 as f32).copysign(x)
}

/// `f32::round` (halves away from zero), bit-identical, without the math library
/// call baseline x86 needs.
#[inline]
fn round(x: f32) -> f32 {
    if !cfg!(any(target_arch = "x86", target_arch = "x86_64")) {
        return x.round();
    }
    // branch-free, so the four channels of a bilinear sample can share SIMD
    // lanes; x itself is the answer when it has no fraction bits (or is NaN)
    let t = x as i32 as f32; // exact truncation whenever it is used
    let d = x - t; // exact: the fraction of x
    let r = (t + ((d >= 0.5) as i32 - (d <= -0.5) as i32) as f32).copysign(x);
    if x.abs() < NO_FRACTION { r } else { x }
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
    depth: Option<(&mut [f32], i32)>,
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
    // Edge function of p->q at (x, y) is dx * (y - p.y) - dy * (x - p.x). The
    // first product only depends on the row, so it is computed once per row;
    // the operands and their order stay the same as a per-pixel evaluation, so
    // every value is bit-identical.
    let edges = [Edge::new(&b, &c), Edge::new(&c, &a), Edge::new(&a, &b)];
    let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as i64;
    let max_x = (a.x.max(b.x).max(c.x).ceil() as i64).min(dst.w as i64 - 1);
    let min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as i64;
    let max_y = (a.y.max(b.y).max(c.y).ceil() as i64).min(dst.h as i64 - 1);
    if min_x > max_x || min_y > max_y {
        return;
    }
    // Along a row each edge value is monotonic in x (every step is a rounded
    // subtraction or a product with a fixed factor, all monotonic), so the
    // covered pixels form one span that can be found by searching for the
    // edges' crossings. That only holds while no product overflows to infinity
    // (inf - inf is NaN); with coordinates this large every pixel of the
    // bounding box is tested instead.
    const SPAN_LIMIT: f32 = 1e18;
    let span_ok = [a, b, c]
        .iter()
        .all(|p| p.x.abs() <= SPAN_LIMIT && p.y.abs() <= SPAN_LIMIT);
    let (zb, depth_test, depth_write): (&mut [f32], bool, bool) = match depth {
        Some((zb, mode)) if mode != DEPTH_OFF => (zb, true, mode == DEPTH_ON),
        _ => (&mut [], false, false),
    };
    let src_empty = src.w == 0 || src.h == 0;
    let mut pxs = [0f32; CHUNK];
    let mut ok = [false; CHUNK];
    let (mut zs, mut us, mut vs) = ([0f32; CHUNK], [0f32; CHUNK], [0f32; CHUNK]);
    for y in min_y..=max_y {
        let py = y as f32 + 0.5;
        let r = [edges[0].row(py), edges[1].row(py), edges[2].row(py)];
        let (mut lo, mut hi) = (min_x, max_x + 1);
        if span_ok {
            for (e, &r) in edges.iter().zip(&r) {
                let ins = |x: i64| e.inside(r, x as f32 + 0.5);
                if e.dy > 0.0 {
                    // inside on the left of the crossing
                    hi = first_false(lo, hi, e.crossing(r), ins);
                } else if e.dy < 0.0 {
                    // inside on the right of the crossing
                    lo = first_false(lo, hi, e.crossing(r), |x| !ins(x));
                } else if !ins(lo) {
                    hi = lo;
                }
                if lo >= hi {
                    break;
                }
            }
        }
        let row = y as usize * dst.w;
        let mut x0 = lo;
        while x0 < hi {
            let n = ((hi - x0) as usize).min(CHUNK);
            // First the arithmetic for a run of pixels, branch-free so the
            // compiler can use SIMD lanes (each lane does the same IEEE
            // operations in the same order as the scalar code would); then
            // the depth test, sampling and writes one pixel at a time.
            for (k, p) in pxs[..n].iter_mut().enumerate() {
                *p = (x0 + k as i64) as f32 + 0.5;
            }
            for k in 0..n {
                let px = pxs[k];
                let w0 = edges[0].at(r[0], px);
                let w1 = edges[1].at(r[1], px);
                let w2 = edges[2].at(r[2], px);
                let (l0, l1, l2) = (w0 / area, w1 / area, w2 / area);
                let q = l0 * a.q + l1 * b.q + l2 * c.q;
                // only q <= 0 is rejected; a NaN q is drawn
                ok[k] = inside(w0, edges[0].tl)
                    & inside(w1, edges[1].tl)
                    & inside(w2, edges[2].tl)
                    & ((q > 0.0) | q.is_nan());
                zs[k] = 1.0 / q;
                us[k] = (l0 * a.u * a.q + l1 * b.u * b.q + l2 * c.u * c.q) / q;
                vs[k] = (l0 * a.v * a.q + l1 * b.v * b.q + l2 * c.v * c.q) / q;
            }
            for k in 0..n {
                if !ok[k] {
                    continue;
                }
                let i = row + x0 as usize + k;
                if depth_test {
                    let z = zs[k];
                    if z > zb[i] {
                        continue;
                    }
                    if depth_write {
                        zb[i] = z;
                    }
                }
                let col = if src_empty {
                    0
                } else if smooth {
                    sample_linear(src, us[k], vs[k], wrap)
                } else {
                    sample_nearest(src, us[k], vs[k], wrap)
                };
                dst.px[i] = if use_alpha { blend(dst.px[i], col) } else { col };
            }
            x0 += n as i64;
        }
    }
}

/// Pixels whose arithmetic [`raster_tri`] does as one batch.
const CHUNK: usize = 16;

/// Top-left fill rule: a pixel centre exactly on an edge belongs to the
/// triangle only for top and left edges.
#[inline]
fn inside(w: f32, top_left: bool) -> bool {
    (w > 0.0) | ((w == 0.0) & top_left)
}

/// One edge p->q of a triangle being filled.
struct Edge {
    px: f32,
    py: f32,
    dx: f32,
    dy: f32,
    tl: bool,
}

impl Edge {
    fn new(p: &Vtx, q: &Vtx) -> Self {
        let (dx, dy) = (q.x - p.x, q.y - p.y);
        Edge {
            px: p.x,
            py: p.y,
            dx,
            dy,
            tl: dy < 0.0 || (dy == 0.0 && dx > 0.0),
        }
    }

    /// The row-invariant first product of the edge function.
    #[inline]
    fn row(&self, y: f32) -> f32 {
        self.dx * (y - self.py)
    }

    #[inline]
    fn at(&self, row: f32, x: f32) -> f32 {
        row - self.dy * (x - self.px)
    }

    #[inline]
    fn inside(&self, row: f32, x: f32) -> bool {
        inside(self.at(row, x), self.tl)
    }

    /// Estimated first pixel past the edge's zero crossing on a row (only a
    /// starting point for the exact search).
    #[inline]
    fn crossing(&self, row: f32) -> i64 {
        (self.px + row / self.dy - 0.5).ceil() as i64
    }
}

/// First `x` in `lo..=hi` where `before(x)` is false, given that `before` is
/// true then false over `lo..hi` (`hi` if it never turns false). Starts at the
/// estimate `guess` and walks to the exact boundary.
#[inline]
fn first_false(lo: i64, hi: i64, guess: i64, before: impl Fn(i64) -> bool) -> i64 {
    let mut x = guess.clamp(lo, hi);
    while x > lo && !before(x - 1) {
        x -= 1;
    }
    while x < hi && before(x) {
        x += 1;
    }
    x
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
    // clip against the near plane (z = -NEAR) in eye space; one plane cuts a
    // triangle into at most four vertices (kept on the stack: this runs per
    // triangle)
    let mut poly = [([0f32; 3], [0f32; 2]); 4];
    let mut len = 0;
    for i in 0..3 {
        let (p, pt) = (d[i], s[i]);
        let (n, nt) = (d[(i + 1) % 3], s[(i + 1) % 3]);
        let p_in = -p.2 >= NEAR;
        let n_in = -n.2 >= NEAR;
        if p_in {
            poly[len] = ([p.0, p.1, p.2], [pt.0, pt.1]);
            len += 1;
        }
        if p_in != n_in {
            let t = (-NEAR - p.2) / (n.2 - p.2);
            let lerp = |a: f32, b: f32| a + (b - a) * t;
            poly[len] = (
                [lerp(p.0, n.0), lerp(p.1, n.1), -NEAR],
                [lerp(pt.0, nt.0), lerp(pt.1, nt.1)],
            );
            len += 1;
        }
    }
    if len < 3 {
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
    let pv = poly.map(|e| proj(&e));
    let pv = &pv[..len];
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

/// The straightforward rasterizer the optimized one replaced, kept verbatim as
/// the pixel reference: the tests require bit-identical pixels and depth.
#[cfg(test)]
mod reference {
    use super::{Source, Target, Vtx, DEPTH_OFF, DEPTH_ON, NEAR};

    fn blend(d: u32, s: u32) -> u32 {
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

    fn wrap_or_clamp(i: i64, n: usize, wrap: bool) -> usize {
        if wrap {
            i.rem_euclid(n as i64) as usize
        } else {
            i.clamp(0, n as i64 - 1) as usize
        }
    }

    pub fn sample(src: &Source, u: f32, v: f32, smooth: bool, wrap: bool) -> u32 {
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

    fn plot(dst: &mut Target, x: usize, y: usize, c: u32, use_alpha: bool) {
        let i = y * dst.w + x;
        dst.px[i] = if use_alpha { blend(dst.px[i], c) } else { c };
    }

    pub fn raster_tri(
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
        let edge =
            |p: &Vtx, q: &Vtx, x: f32, y: f32| (q.x - p.x) * (y - p.y) - (q.y - p.y) * (x - p.x);
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
        use super::{CULL_ANTICLOCKWISE_ONLY, CULL_CLOCKWISE_ONLY};
        let (w, h) = (dst.w as f32, dst.h as f32);
        if dst.w == 0 || dst.h == 0 || depth.len() < dst.px.len() {
            return;
        }
        let fov = if dst.w > dst.h { 90.0 * h / w } else { 90.0f32 };
        let f = 1.0 / (fov.to_radians() / 2.0).tan();
        let aspect = w / h;
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
}

/// Bit-identity of the optimized rasterizer against [`reference`], and a
/// speed comparison (`cargo test --release -p qb-runtime --lib raster_bench -- --ignored --nocapture`).
#[cfg(test)]
mod identity_tests {
    use super::*;

    /// xorshift64*: fixed-seed, dependency-free.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
        /// Uniform in [lo, hi).
        fn f(&mut self, lo: f32, hi: f32) -> f32 {
            let t = (self.next() >> 40) as f32 / (1u64 << 24) as f32;
            lo + (hi - lo) * t
        }
        fn chance(&mut self, pct: u64) -> bool {
            self.below(100) < pct
        }
        /// A coordinate snapped to a coarse grid sometimes, so pixel centres
        /// land exactly on edges and the top-left rule is exercised.
        fn coord(&mut self, lo: f32, hi: f32) -> f32 {
            let x = self.f(lo, hi);
            match self.below(4) {
                0 => x.round(),
                1 => (x * 2.0).round() / 2.0,
                2 => (x * 4.0).round() / 4.0,
                _ => x,
            }
        }
    }

    fn texture(rng: &mut Rng) -> (Vec<u32>, usize, usize) {
        let (w, h) = match rng.below(6) {
            0 => (1, 1),
            1 => (8, 8),
            2 => (16, 4),
            3 => (5, 3),
            4 => (0, 0),
            _ => (7, 32),
        };
        let px = (0..w * h).map(|_| rng.next() as u32).collect();
        (px, w, h)
    }

    struct Pair {
        new_px: Vec<u32>,
        new_z: Vec<f32>,
        old_px: Vec<u32>,
        old_z: Vec<f32>,
    }

    impl Pair {
        fn new(rng: &mut Rng, w: usize, h: usize) -> Self {
            let px: Vec<u32> = (0..w * h).map(|_| rng.next() as u32).collect();
            let z: Vec<f32> = (0..w * h)
                .map(|_| if rng.chance(50) { f32::INFINITY } else { rng.f(0.0, 60.0) })
                .collect();
            Pair {
                new_px: px.clone(),
                new_z: z.clone(),
                old_px: px,
                old_z: z,
            }
        }

        fn assert_same(&self, what: &str) {
            assert_eq!(self.new_px, self.old_px, "pixels differ after {what}");
            let bits = |z: &[f32]| z.iter().map(|v| v.to_bits()).collect::<Vec<_>>();
            assert_eq!(bits(&self.new_z), bits(&self.old_z), "depth differs after {what}");
        }
    }

    fn vtx(rng: &mut Rng, cx: f32, cy: f32, size: f32, tex: f32) -> Vtx {
        Vtx {
            x: rng.coord(cx - size, cx + size),
            y: rng.coord(cy - size, cy + size),
            u: rng.f(-3.0 * tex, 4.0 * tex),
            v: rng.f(-3.0 * tex, 4.0 * tex),
            q: if rng.chance(10) { rng.f(-1.0, 1.0) } else { rng.f(0.01, 2.0) },
        }
    }

    #[test]
    fn raster_tri_matches_reference() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        for (w, h, count) in [(97, 61, 3000), (320, 200, 300), (1, 1, 200), (3, 40, 300)] {
            let mut p = Pair::new(&mut rng, w, h);
            for n in 0..count {
                let (tpx, tw, th) = texture(&mut rng);
                let src = Source { px: &tpx, w: tw, h: th };
                let size = match rng.below(5) {
                    0 => rng.f(0.0, 1.5),
                    1 => rng.f(1.0, 8.0),
                    2 => rng.f(4.0, 40.0),
                    3 => (w.max(h) as f32) * rng.f(0.5, 1.5),
                    _ => (w.max(h) as f32) * rng.f(2.0, 20.0),
                };
                let cx = rng.f(-0.3 * w as f32, 1.3 * w as f32);
                let cy = rng.f(-0.3 * h as f32, 1.3 * h as f32);
                let mut v = [0; 3].map(|_| vtx(&mut rng, cx, cy, size, tw.max(1) as f32));
                match rng.below(20) {
                    // degenerate: a repeated vertex, or three points on a line
                    0 => v[2] = v[rng.below(2) as usize],
                    1 => {
                        v[2].x = v[0].x + (v[1].x - v[0].x) * 2.0;
                        v[2].y = v[0].y + (v[1].y - v[0].y) * 2.0;
                    }
                    // axis-aligned edges
                    2 => v[1].y = v[0].y,
                    3 => v[2].x = v[1].x,
                    // coordinates too large for the span search, or overflowing
                    4 => v[0].x = [9e17, 3e18, -2e30][rng.below(3) as usize],
                    5 => v[1].y = if rng.chance(50) { -5e19 } else { f32::INFINITY },
                    _ => {}
                }
                let alpha = rng.chance(50);
                let smooth = rng.chance(50);
                let wrap = rng.chance(50);
                let mode = match rng.below(4) {
                    0 => None,
                    k => Some(k as i32 - 1),
                };
                raster_tri(
                    &mut Target { px: &mut p.new_px, w, h },
                    mode.map(|m| (&mut p.new_z[..], m)),
                    &src,
                    v,
                    alpha,
                    smooth,
                    wrap,
                );
                reference::raster_tri(
                    &mut Target { px: &mut p.old_px, w, h },
                    mode.map(|m| (&mut p.old_z[..], m)),
                    &src,
                    v,
                    alpha,
                    smooth,
                    wrap,
                );
                if n % 50 == 0 || n + 1 == count {
                    p.assert_same(&format!("triangle {n} on {w}x{h}"));
                }
            }
        }
    }

    #[test]
    fn tri3d_matches_reference() {
        let mut rng = Rng(0x0123_4567_89AB_CDEF);
        for (w, h, count) in [(97, 61, 2000), (64, 160, 500), (320, 200, 200)] {
            let mut p = Pair::new(&mut rng, w, h);
            for n in 0..count {
                let (tpx, tw, th) = texture(&mut rng);
                let src = Source { px: &tpx, w: tw, h: th };
                let size = match rng.below(4) {
                    0 => rng.f(0.001, 0.05),
                    1 => rng.f(0.05, 1.0),
                    2 => rng.f(1.0, 10.0),
                    _ => rng.f(10.0, 200.0),
                };
                let (cx, cy) = (rng.f(-15.0, 15.0), rng.f(-15.0, 15.0));
                // depth from in front of the near plane to well behind it, so
                // many triangles cross it and get clipped
                let cz = rng.f(-40.0, 2.0);
                let mut d = [0; 3].map(|_| {
                    (
                        rng.coord(cx - size, cx + size),
                        rng.coord(cy - size, cy + size),
                        rng.coord(cz - size, cz + size),
                    )
                });
                let tx = tw.max(1) as f32;
                let s = [0; 3].map(|_| (rng.f(-2.0 * tx, 3.0 * tx), rng.f(-2.0 * tx, 3.0 * tx)));
                match rng.below(12) {
                    0 => d[2] = d[0],
                    1 => d[1].2 = -NEAR,
                    2 => {
                        d[0].2 = -NEAR;
                        d[2].2 = -NEAR;
                    }
                    3 => d[2] = (d[0].0 * 2.0, d[0].1 * 2.0, d[0].2 * 2.0),
                    _ => {}
                }
                let alpha = rng.chance(50);
                let smooth = rng.chance(50);
                let cull = rng.below(3) as i32;
                let mode = rng.below(3) as i32;
                tri3d(
                    &mut Target { px: &mut p.new_px, w, h },
                    &mut p.new_z,
                    src,
                    d,
                    s,
                    alpha,
                    smooth,
                    cull,
                    mode,
                );
                reference::tri3d(
                    &mut Target { px: &mut p.old_px, w, h },
                    &mut p.old_z,
                    src,
                    d,
                    s,
                    alpha,
                    smooth,
                    cull,
                    mode,
                );
                if n % 50 == 0 || n + 1 == count {
                    p.assert_same(&format!("3D triangle {n} on {w}x{h}"));
                }
            }
        }
    }

    #[test]
    fn floor_and_round_match_std() {
        let check = |x: f32| {
            let same = |a: f32, b: f32| a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan());
            assert!(same(floor(x), x.floor()), "floor({x:e}) = {}", floor(x));
            assert!(same(round(x), x.round()), "round({x:e}) = {}", round(x));
        };
        // a spread over every exponent and sign
        for bits in (0..=u32::MAX).step_by(997) {
            check(f32::from_bits(bits));
        }
        // integers, halves and their neighbours
        for k in -3000..3000 {
            for x in [k as f32, k as f32 + 0.5, k as f32 * 4096.0 + 0.5] {
                check(x);
                check(x.next_up());
                check(x.next_down());
            }
        }
        for x in [0.0, -0.0, 0.49999997, -0.49999997, 8_388_607.5, -8_388_607.5, 8_388_608.0, f32::MAX, f32::MIN, f32::INFINITY, f32::NEG_INFINITY, f32::NAN, f32::MIN_POSITIVE, 1e-45, -1e-45] {
            check(x);
        }
    }

    #[test]
    fn sample_matches_reference() {
        let mut rng = Rng(42);
        for _ in 0..20_000 {
            let (tpx, tw, th) = texture(&mut rng);
            let src = Source { px: &tpx, w: tw, h: th };
            let r = if rng.chance(10) { 1e12 } else { 40.0 };
            let (u, v) = (rng.f(-r, r), rng.f(-r, r));
            let (smooth, wrap) = (rng.chance(50), rng.chance(50));
            assert_eq!(
                sample(&src, u, v, smooth, wrap),
                reference::sample(&src, u, v, smooth, wrap),
                "{tw}x{th} at ({u}, {v}) smooth {smooth} wrap {wrap}"
            );
        }
    }

    /// A textured mesh over an 800x600 target: `cols` x `rows` quads at depth
    /// `z`, `layers` times at growing depth.
    fn mesh(cols: usize, rows: usize, layers: usize) -> Vec<[(f32, f32, f32); 3]> {
        let mut out = Vec::new();
        for l in 0..layers {
            let z = -2.0 - l as f32 * 0.5;
            // exactly the visible part of the 4:3 view (67.5 degrees vertically)
            let hh = -z * (67.5f32.to_radians() / 2.0).tan();
            let hw = hh * 4.0 / 3.0;
            for r in 0..rows {
                for c in 0..cols {
                    let x0 = -hw + 2.0 * hw * c as f32 / cols as f32;
                    let x1 = -hw + 2.0 * hw * (c + 1) as f32 / cols as f32;
                    let y0 = -hh + 2.0 * hh * r as f32 / rows as f32;
                    let y1 = -hh + 2.0 * hh * (r + 1) as f32 / rows as f32;
                    out.push([(x0, y1, z), (x1, y1, z), (x1, y0, z)]);
                    out.push([(x0, y1, z), (x1, y0, z), (x0, y0, z)]);
                }
            }
        }
        out
    }

    fn bench_one(name: &str, tris: &[[(f32, f32, f32); 3]], smooth: bool) {
        type Tri3d = fn(
            &mut Target,
            &mut [f32],
            Source,
            [(f32, f32, f32); 3],
            [(f32, f32); 3],
            bool,
            bool,
            i32,
            i32,
        );
        let tex: Vec<u32> = (0..64 * 64u32).map(|i| 0xFF00_0000 | i.wrapping_mul(0x9E37_79B9)).collect();
        let src = Source { px: &tex, w: 64, h: 64 };
        let s = [(0.0, 0.0), (96.0, 0.0), (96.0, 96.0)];
        let (w, h) = (800, 600);
        let mut results = Vec::new();
        for (label, f) in [("old", reference::tri3d as Tri3d), ("new", tri3d as Tri3d)] {
            let mut px = vec![0u32; w * h];
            let mut z = vec![f32::INFINITY; w * h];
            let frames = 5;
            let mut best = f64::MAX;
            for _ in 0..frames {
                px.fill(0);
                z.fill(f32::INFINITY);
                let t = std::time::Instant::now();
                for d in tris {
                    f(&mut Target { px: &mut px, w, h }, &mut z, src, *d, s, false, smooth, CULL_NONE, DEPTH_ON);
                }
                best = best.min(t.elapsed().as_secs_f64() * 1000.0);
            }
            println!("{name} smooth={smooth} {label}: {best:.2} ms/frame (best of {frames})");
            results.push(px);
        }
        assert_eq!(results[0], results[1]);
    }

    #[test]
    #[ignore = "benchmark; run in release with --ignored --nocapture"]
    fn raster_bench() {
        // 160 x 84 quads = 26,880 small triangles
        let small = mesh(160, 84, 1);
        // 400 large triangles: 200 full-screen quads stacked in depth, drawn
        // back to front so every pixel passes the depth test
        let mut large = mesh(1, 1, 200);
        large.reverse();
        println!("small: {} triangles, large: {} triangles", small.len(), large.len());
        for smooth in [false, true] {
            bench_one("small mesh", &small, smooth);
            bench_one("large", &large, smooth);
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
    fn flat_triangle_fills_half_square() {
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
        let v = |x: f32, y: f32| Vtx { x, y, u: 0.5, v: 0.5, q: 1.0 };
        raster_tri(&mut t, None, &s, [v(0.0, 0.0), v(4.0, 0.0), v(0.0, 4.0)], false, false, false);
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
