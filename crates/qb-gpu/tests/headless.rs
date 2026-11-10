//! Headless GPU tests: render offscreen, read the pixels back. Every test
//! skips (passes with a note) when the machine has no GPU adapter at all,
//! hardware or software. No windows are opened.

use qb_gpu::blit::{fit_rect, Blitter, Fit};
use qb_gpu::three::{quad, Camera, Draw, Renderer3d, Texture};
use qb_gpu::{Gpu, Target};

fn gpu() -> Option<Gpu> {
    let g = Gpu::headless();
    match &g {
        Some(g) => eprintln!("qb-gpu adapter: {}", g.adapter_name()),
        None => eprintln!("skipped: no GPU adapter"),
    }
    g
}

fn px(rgba: &[u8], w: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * w + x) * 4) as usize;
    [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
}

fn fnv(b: &[u8]) -> u64 {
    b.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, x| (h ^ u64::from(*x)).wrapping_mul(0x0100_0000_01b3))
}

/// 0xAARRGGBB -> RGBA bytes as read back from an RGBA8 target.
fn rgba_of(c: u32) -> [u8; 4] {
    [(c >> 16) as u8, (c >> 8) as u8, c as u8, 0xFF]
}

const FB: [u32; 8] = [
    0xFFFF_0000, 0xFF00_FF00, 0xFF00_00FF, 0xFFFF_FFFF, //
    0xFF12_3456, 0xFF80_8080, 0xFF00_0000, 0xFFFE_DCBA,
];

#[test]
fn fit_rect_keeps_the_aspect_ratio() {
    assert_eq!(fit_rect(320, 200, 640, 400, Fit::Aspect), (0, 0, 640, 400));
    assert_eq!(fit_rect(320, 200, 640, 640, Fit::Aspect), (0, 120, 640, 400));
    assert_eq!(fit_rect(320, 200, 1000, 400, Fit::Aspect), (180, 0, 640, 400));
    assert_eq!(fit_rect(320, 200, 1000, 700, Fit::Integer), (20, 50, 960, 600));
    assert_eq!(fit_rect(320, 200, 900, 700, Fit::Integer), (130, 150, 640, 400));
    assert_eq!(fit_rect(320, 200, 100, 100, Fit::Integer), (0, 0, 100, 100));
    assert_eq!(fit_rect(4, 2, 7, 3, Fit::Stretch), (0, 0, 7, 3));
}

#[test]
fn blit_scales_framebuffer_pixels_exactly() {
    let Some(g) = gpu() else { return };
    let mut t = Target::offscreen(&g, 8, 4);
    let mut b = Blitter::new(&g, t.format());
    let f = t.frame(&g).expect("offscreen frame");
    b.draw(&g, &f, &FB, 4, 2);
    f.present(&g);
    let out = t.read_rgba(&g).expect("readback");
    for y in 0..4 {
        for x in 0..8 {
            assert_eq!(px(&out, 8, x, y), rgba_of(FB[((y / 2) * 4 + x / 2) as usize]), "pixel {x},{y}");
        }
    }
}

#[test]
fn blit_letterboxes_when_the_aspect_differs() {
    let Some(g) = gpu() else { return };
    let mut t = Target::offscreen(&g, 8, 8);
    let mut b = Blitter::new(&g, t.format());
    let f = t.frame(&g).expect("offscreen frame");
    b.draw(&g, &f, &FB, 4, 2);
    f.present(&g);
    let out = t.read_rgba(&g).expect("readback");
    // 4x2 scaled 2x into rows 2..6; black bars above and below.
    for x in 0..8 {
        for y in [0, 1, 6, 7] {
            assert_eq!(px(&out, 8, x, y), [0, 0, 0, 255], "bar {x},{y}");
        }
        for y in 2..6 {
            assert_eq!(px(&out, 8, x, y), rgba_of(FB[(((y - 2) / 2) * 4 + x / 2) as usize]), "image {x},{y}");
        }
    }
}

#[test]
fn depth_buffer_keeps_the_nearest_surface() {
    let Some(g) = gpu() else { return };
    let mut t = Target::offscreen(&g, 32, 32);
    let mut r = Renderer3d::new(&g, t.format());
    let white = Texture::white(&g, &r);
    // Near red square drawn first, far blue square (larger) drawn after.
    let near = quad(&g, 0.0, 0.0, 0.5, 1.0, [1.0, 0.0, 0.0, 1.0]);
    // Seen from z = 3 with a 60 degree field of view the far square covers rows ~4..28,
    // the near one rows ~11..21 of the 32x32 target.
    let far = quad(&g, 0.0, 0.0, -0.5, 3.0, [0.0, 0.0, 1.0, 1.0]);
    let cam = Camera::perspective(glam::Vec3::new(0.0, 0.0, 3.0), glam::Vec3::ZERO, 60f32.to_radians());
    let f = t.frame(&g).expect("frame");
    let draws = [
        Draw { mesh: &near, texture: &white, model: glam::Mat4::IDENTITY },
        Draw { mesh: &far, texture: &white, model: glam::Mat4::IDENTITY },
    ];
    r.render(&g, &f, &cam, qb_gpu::wgpu::Color::BLACK, &draws);
    f.present(&g);
    let out = t.read_rgba(&g).expect("readback");
    assert_eq!(px(&out, 32, 16, 16), [255, 0, 0, 255], "near surface in front");
    // Outside the red square but inside the blue one.
    let edge = px(&out, 32, 16, 7);
    assert_eq!(edge, [0, 0, 255, 255], "far surface around it");
    assert_eq!(px(&out, 32, 0, 0), [0, 0, 0, 255], "background");
}

#[test]
fn textured_quad_and_stable_checksum() {
    let Some(g) = gpu() else { return };
    let render = || {
        let mut t = Target::offscreen(&g, 16, 16);
        let mut r = Renderer3d::new(&g, t.format());
        // 2x2 texture: red green / blue white
        let tex = Texture::from_rgba(
            &g,
            &r,
            2,
            2,
            &[255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255],
            false,
        );
        let m = quad(&g, 0.0, 0.0, 0.0, 2.0, [1.0; 4]);
        let cam = Camera::orthographic(glam::Vec3::new(0.0, 0.0, 5.0), glam::Vec3::ZERO, 2.0);
        let f = t.frame(&g).expect("frame");
        r.render(&g, &f, &cam, qb_gpu::wgpu::Color::BLACK, &[Draw { mesh: &m, texture: &tex, model: glam::Mat4::IDENTITY }]);
        f.present(&g);
        t.read_rgba(&g).expect("readback")
    };
    let a = render();
    // The quad fills the 16x16 view: texel quadrants map to screen quadrants (v = 0 at the top).
    assert_eq!(px(&a, 16, 3, 3), [255, 0, 0, 255]);
    assert_eq!(px(&a, 16, 12, 3), [0, 255, 0, 255]);
    assert_eq!(px(&a, 16, 3, 12), [0, 0, 255, 255]);
    assert_eq!(px(&a, 16, 12, 12), [255, 255, 255, 255]);
    // Same scene, same pixels.
    assert_eq!(fnv(&a), fnv(&render()));
}
