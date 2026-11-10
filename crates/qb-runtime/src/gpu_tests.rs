//! R15 G2/G3: the GPU paths against the software reference, headless.
//!
//! * window presentation: `qb_gpu::blit` with the runtime layout must give the
//!   pixels `raster::scale_to_window` gives;
//! * 3D `_MAPTRIANGLE`: `qb_gpu::maptri` must draw what `raster::tri3d` draws
//!   (projection, near clipping, depth modes, culling, wrap, alpha). Edge
//!   pixels and texel boundaries may round differently on a GPU, so a small
//!   share of differing pixels is allowed, while sample points inside the
//!   triangles must match exactly.
//!
//! Every test passes with a note when the machine has no GPU adapter at all.
//! No windows are opened.

use qb_gpu::blit::Blitter;
use qb_gpu::maptri::{Cull, DepthBuffer, DepthMode, MapTriangle3d, Smooth, SourceImage, Tri3d};
use qb_gpu::wgpu;
use qb_gpu::{Gpu, Target};
use crate::gpu_present::draw_layout;
use crate::raster::{self, Source, Target as SwTarget};

fn gpu() -> Option<Gpu> {
    let g = Gpu::headless();
    match &g {
        Some(g) => eprintln!("qb-gpu adapter: {}", g.adapter_name()),
        None => eprintln!("skipped: no GPU adapter"),
    }
    g
}

fn pattern(w: usize, h: usize) -> Vec<u32> {
    (0..w * h)
        .map(|i| {
            let v = (i as u32).wrapping_mul(0x9E37_79B9);
            0xFF00_0000 | (v >> 8)
        })
        .collect()
}

// ---- presentation ----------------------------------------------------------

#[test]
fn gpu_present_matches_software_scaling() {
    let Some(g) = gpu() else { return };
    // (screen w, h, window w, h, full_screen, fullscreen_smooth, resize_auto)
    let cases = [
        (4, 3, 4, 3, 0, false, 0),   // same size
        (4, 3, 8, 6, 0, false, 1),   // _STRETCH 2x
        (4, 3, 12, 6, 0, false, 1),  // stretched, not square
        (4, 3, 16, 6, 2, false, 0),  // full screen, square pixels: vertical bars
        (4, 3, 8, 12, 2, false, 0),  // horizontal bars
        (8, 6, 5, 4, 0, false, 0),   // cannot scale, window smaller: clipped
        (4, 3, 10, 5, 0, false, 0),  // cannot scale, blank right and bottom
    ];
    for (sw, sh, ww, wh, fs, fsm, ra) in cases {
        let frame_px = pattern(sw, sh);
        let l = raster::layout(sw as i32, sh as i32, ww as i32, wh as i32, fs, fsm, ra);
        let mut soft = vec![0u32; ww * wh];
        raster::scale_to_window(&mut SwTarget { px: &mut soft, w: ww, h: wh }, Source { px: &frame_px, w: sw, h: sh }, &l);
        let mut t = Target::offscreen_with_format(&g, ww as u32, wh as u32, wgpu::TextureFormat::Bgra8Unorm);
        let mut b = Blitter::new(&g, t.format());
        let f = t.frame(&g).expect("frame");
        draw_layout(&g, &mut b, &f, &frame_px, sw, sh, &l);
        f.present(&g);
        let hw: Vec<u32> = t.read_pixels(&g).expect("readback").iter().map(|p| p & 0x00FF_FFFF).collect();
        assert_eq!(hw, soft, "screen {sw}x{sh} in window {ww}x{wh} (fs {fs}, resize {ra})");
    }
}

// ---- 3D _MAPTRIANGLE --------------------------------------------------------

const W: usize = 64;
const H: usize = 48;

struct Img {
    px: Vec<u32>,
    w: usize,
    h: usize,
}

fn solid(c: u32) -> Img {
    Img { px: vec![c; 4], w: 2, h: 2 }
}

/// 8x8 checker of 2x2-texel blocks in four colours.
fn checker() -> Img {
    let cols = [0xFFFF_0000, 0xFF00_FF00, 0xFF00_00FF, 0xFFFF_FFFF];
    let px = (0..64).map(|i| cols[((i % 8) / 2 + (i / 8) / 2) % 4]).collect();
    Img { px, w: 8, h: 8 }
}

#[derive(Clone, Copy)]
struct T {
    img: usize,
    d: [[f32; 3]; 3],
    s: [[f32; 2]; 3],
    alpha: bool,
    smooth: bool,
    cull: i32,
    depth: i32,
}

fn tri(img: usize, d: [[f32; 3]; 3], s: [[f32; 2]; 3]) -> T {
    T { img, d, s, alpha: false, smooth: false, cull: 0, depth: 1 }
}

/// A square at depth `z` (two triangles, both drawn clockwise on screen).
fn square(img: usize, x: f32, y: f32, r: f32, z: f32, tex: f32) -> [T; 2] {
    let (a, b, c, d) = ([x - r, y + r, z], [x + r, y + r, z], [x + r, y - r, z], [x - r, y - r, z]);
    [tri(img, [a, b, c], [[0.0, 0.0], [tex, 0.0], [tex, tex]]), tri(img, [a, c, d], [[0.0, 0.0], [tex, tex], [0.0, tex]])]
}

fn software(imgs: &[Img], tris: &[T]) -> Vec<u32> {
    let mut px = vec![0u32; W * H];
    let mut depth = vec![f32::INFINITY; W * H];
    for t in tris {
        let i = &imgs[t.img];
        raster::tri3d(
            &mut SwTarget { px: &mut px, w: W, h: H },
            &mut depth,
            Source { px: &i.px, w: i.w, h: i.h },
            [(t.d[0][0], t.d[0][1], t.d[0][2]), (t.d[1][0], t.d[1][1], t.d[1][2]), (t.d[2][0], t.d[2][1], t.d[2][2])],
            [(t.s[0][0], t.s[0][1]), (t.s[1][0], t.s[1][1]), (t.s[2][0], t.s[2][1])],
            t.alpha,
            t.smooth,
            t.cull,
            t.depth,
        );
    }
    px
}

fn hardware(g: &Gpu, imgs: &[Img], tris: &[T]) -> Vec<u32> {
    let target = Target::offscreen_with_format(g, W as u32, H as u32, wgpu::TextureFormat::Bgra8Unorm);
    let mut r = MapTriangle3d::new(g, target.format());
    let srcs: Vec<SourceImage> = imgs.iter().map(|i| r.source(g, &i.px, i.w as u32, i.h as u32)).collect();
    let mut depth = DepthBuffer::new(g, W as u32, H as u32);
    let cmds: Vec<Tri3d> = tris
        .iter()
        .map(|t| Tri3d {
            src: &srcs[t.img],
            d: t.d,
            s: t.s,
            use_alpha: t.alpha,
            smooth: if t.smooth { Smooth::Both } else { Smooth::Off },
            wrap: true,
            cull: Cull::from_legacy(t.cull),
            depth: DepthMode::from_legacy(t.depth),
        })
        .collect();
    let Target::Offscreen { texture, .. } = &target else { unreachable!() };
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    r.draw(g, &view, &mut depth, Some(wgpu::Color::TRANSPARENT), &cmds);
    target.read_pixels(g).expect("readback")
}

fn close(a: u32, b: u32, tol: u32) -> bool {
    (0..4).all(|k| ((a >> (k * 8)) & 255).abs_diff((b >> (k * 8)) & 255) <= tol)
}

/// GPU and software agree: at most 3% of the covered pixels differ, and the
/// listed points are equal.
fn same(name: &str, imgs: &[Img], tris: &[T], tol: u32, points: &[(usize, usize)]) -> Vec<u32> {
    let g = gpu().expect("checked by caller");
    let sw = software(imgs, tris);
    let hw = hardware(&g, imgs, tris);
    let covered = sw.iter().zip(&hw).filter(|(a, b)| **a != 0 || **b != 0).count();
    let differ = sw.iter().zip(&hw).filter(|(a, b)| !close(**a, **b, tol)).count();
    assert!(covered > 0, "{name}: nothing drawn");
    assert!(differ * 100 <= covered * 3, "{name}: {differ} of {covered} pixels differ");
    for &(x, y) in points {
        let i = y * W + x;
        assert!(close(sw[i], hw[i], tol), "{name}: pixel {x},{y} software {:08x} gpu {:08x}", sw[i], hw[i]);
    }
    sw
}

fn have_gpu() -> bool {
    gpu().is_some()
}

#[test]
fn maptriangle3d_depth_buffer_matches_software() {
    if !have_gpu() {
        return;
    }
    let imgs = [solid(0xFF00_00FF), solid(0xFFFF_0000), solid(0xFF00_FF00)];
    let mut tris = Vec::new();
    tris.extend(square(0, 0.0, 0.0, 1.5, -3.0, 2.0)); // far blue
    tris.push(tri(1, [[-0.5, 0.5, -2.0], [0.5, 0.5, -2.0], [0.0, -0.5, -2.0]], [[0.0; 2]; 3])); // near red
    tris.push(tri(2, [[-0.8, 0.8, -4.0], [0.8, 0.8, -4.0], [0.0, -0.8, -4.0]], [[0.0; 2]; 3])); // hidden green
    let sw = same("depth", &imgs, &tris, 0, &[(32, 24), (10, 24), (32, 6), (0, 0)]);
    assert_eq!(sw[24 * W + 32], 0xFFFF_0000, "near red in front");
    assert!(!sw.contains(&0xFF00_FF00), "green is hidden");
}

#[test]
fn maptriangle3d_depth_lock_and_off_match_software() {
    if !have_gpu() {
        return;
    }
    let imgs = [solid(0xFF00_00FF), solid(0xFFFF_0000), solid(0xFF00_FF00)];
    let mut tris = Vec::new();
    tris.extend(square(0, 0.0, 0.0, 1.5, -3.0, 2.0));
    let mut near = tri(1, [[-0.5, 0.5, -2.0], [0.5, 0.5, -2.0], [0.0, -0.5, -2.0]], [[0.0; 2]; 3]);
    near.depth = 2; // _DEPTHBUFFER LOCK: tested, not written
    tris.push(near);
    // mid-depth green behind the red: drawn over it, since red left no depth
    tris.push(tri(2, [[-0.6, 0.6, -2.5], [0.6, 0.6, -2.5], [0.0, -0.6, -2.5]], [[0.0; 2]; 3]));
    let mut off = tri(1, [[-0.2, -0.6, -5.0], [0.2, -0.6, -5.0], [0.0, -0.9, -5.0]], [[0.0; 2]; 3]);
    off.depth = 0; // behind everything but drawn anyway
    tris.push(off);
    let sw = same("lock/off", &imgs, &tris, 0, &[(32, 22), (32, 40)]);
    assert_eq!(sw[22 * W + 32], 0xFF00_FF00);
}

#[test]
fn maptriangle3d_culling_matches_software() {
    if !have_gpu() {
        return;
    }
    let imgs = [solid(0xFFFF_0000), solid(0xFF00_FF00)];
    // left: clockwise on screen; right: anticlockwise
    let cw = tri(0, [[-1.2, 0.5, -2.0], [-0.2, 0.5, -2.0], [-0.7, -0.5, -2.0]], [[0.0; 2]; 3]);
    let acw = tri(1, [[0.2, 0.5, -2.0], [0.7, -0.5, -2.0], [1.2, 0.5, -2.0]], [[0.0; 2]; 3]);
    for cull in [0, 1, 2] {
        let (mut a, mut b) = (cw, acw);
        a.cull = cull;
        b.cull = cull;
        let sw = same(&format!("cull {cull}"), &imgs, &[a, b], 0, &[(16, 20), (48, 20)]);
        let (left, right) = (sw[20 * W + 16] != 0, sw[20 * W + 48] != 0);
        assert_eq!((left, right), [(true, true), (true, false), (false, true)][cull as usize], "cull {cull}");
    }
}

#[test]
fn maptriangle3d_near_clip_and_wrap_match_software() {
    if !have_gpu() {
        return;
    }
    let imgs = [checker()];
    // one vertex behind the camera: clipped at z = -0.1
    let t = tri(0, [[-1.0, -0.5, -1.0], [1.0, -0.5, -1.0], [0.0, -0.5, 1.0]], [[0.0, 0.0], [32.0, 0.0], [16.0, 32.0]]);
    same("near clip", &imgs, &[t], 0, &[]);
    // texture coordinates past the image repeat (4x4 tiles)
    let q = square(0, 0.0, 0.0, 1.0, -2.0, 32.0);
    same("wrap", &imgs, &q, 0, &[(32, 24)]);
}

#[test]
fn maptriangle3d_alpha_and_smooth_match_software() {
    if !have_gpu() {
        return;
    }
    let imgs = [solid(0xFF00_00FF), solid(0x80FF_0000), checker()];
    let mut tris = Vec::new();
    tris.extend(square(0, 0.0, 0.0, 1.5, -3.0, 2.0));
    for mut t in square(1, 0.0, 0.0, 0.5, -2.0, 2.0) {
        t.alpha = true;
        tris.push(t);
    }
    let sw = same("alpha", &imgs, &tris, 1, &[(32, 24)]);
    let c = sw[24 * W + 32];
    assert!(close(c, 0xFF80_007F, 1), "blend {c:08x}");
    let mut sm = square(2, 0.0, 0.0, 1.0, -2.0, 8.0);
    for t in &mut sm {
        t.smooth = true;
    }
    same("smooth", &imgs, &sm, 2, &[(32, 24)]);
}
