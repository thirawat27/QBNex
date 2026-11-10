//! R15 G5 `_SHADER` (user WGSL `fn shade`) and G6 batching, headless.
//! Compilation tests need no GPU; rendering tests skip without an adapter.

use std::sync::Arc;

use qb_gpu::glam::Vec3;
use qb_gpu::scene::{transform, Instance, Lighting, Material, Model, SceneRenderer, Shader};
use qb_gpu::three::Camera;
use qb_gpu::{wgpu, Gpu, Target};

const N: u32 = 32;

#[test]
fn a_good_shader_compiles() {
    Shader::compile("fn shade(i: ShadeIn) -> vec4<f32> {\n    return vec4<f32>(i.uv, 0.0, 1.0);\n}").expect("compiles");
    // helpers and shade_lit may be used
    Shader::compile("fn half(c: vec3<f32>) -> vec3<f32> { return c * 0.5; }\nfn shade(i: ShadeIn) -> vec4<f32> { let c = shade_lit(i); return vec4<f32>(half(c.rgb), c.a); }").expect("compiles");
}

#[test]
fn errors_point_at_the_users_line() {
    let e = Shader::compile("fn shade(i: ShadeIn) -> vec4<f32> {\n    let x = ;\n    return vec4<f32>(1.0);\n}").unwrap_err();
    assert_eq!(e.line, 2, "{e}");
    assert!(e.to_string().starts_with("line 2, column "), "{e}");
    let e = Shader::compile("fn shade(i: ShadeIn) -> vec4<f32> {\n    let x: f32 = i.uv;\n    return vec4<f32>(x);\n}").unwrap_err();
    assert_eq!(e.line, 2, "{e}");
    let e = Shader::compile("fn colour() -> f32 { return 1.0; }").unwrap_err();
    assert_eq!(e.line, 0);
    assert!(e.message.contains("fn shade"), "{e}");
    // wrong signature: reported without a user line, with a hint
    let e = Shader::compile("fn shade(i: ShadeIn) -> vec3<f32> { return i.light; }").unwrap_err();
    assert!(e.message.contains("check that shade"), "{e}");
}

fn gpu() -> Option<Gpu> {
    let g = Gpu::headless();
    match &g {
        Some(g) => eprintln!("qb-gpu adapter: {}", g.adapter_name()),
        None => eprintln!("skipped: no GPU adapter"),
    }
    g
}

fn px(rgba: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * N + x) * 4) as usize;
    [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
}

fn near(a: [u8; 4], b: [u8; 4], tol: u8) -> bool {
    a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= tol)
}

#[test]
fn shader_params_reach_the_pixels_and_mix_with_lit_models() {
    let Some(g) = gpu() else { return };
    let sh = Arc::new(Shader::compile("fn shade(i: ShadeIn) -> vec4<f32> { return vec4<f32>(i.params[0].xyz, 1.0); }").expect("shader"));
    let mut left = Model::cube(&g, 1.0, Material::default());
    let mut params = [0.0f32; 16];
    params[..3].copy_from_slice(&[0.25, 0.5, 1.0]);
    left.shader = Some((sh, params));
    let right = Model::cube(&g, 1.0, Material { base_color: [1.0, 0.0, 0.0, 1.0], texture: None });
    let mut t = Target::offscreen(&g, N, N);
    let mut r = SceneRenderer::new(&g, t.format());
    let cam = Camera::orthographic(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, 4.0);
    let lit = Lighting { ambient: [1.0; 3], lights: vec![] };
    let inst = [
        Instance { model: &left, transform: transform(Vec3::new(-1.0, 0.0, 0.0), Vec3::ZERO, Vec3::ONE) },
        Instance { model: &right, transform: transform(Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO, Vec3::ONE) },
    ];
    let f = t.frame(&g).expect("frame");
    r.render(&g, &f, &cam, &lit, Some(wgpu::Color::BLACK), &inst);
    f.present(&g);
    let out = t.read_rgba(&g).expect("readback");
    // left cube centre at x = -1 -> column 8, right at column 24
    assert!(near(px(&out, 8, 16), [64, 128, 255, 255], 1), "{:?}", px(&out, 8, 16));
    assert!(near(px(&out, 24, 16), [255, 0, 0, 255], 1), "{:?}", px(&out, 24, 16));
    assert_eq!(r.last_draw_calls, 2);
}

#[test]
fn many_instances_render_in_one_batch_like_single_draws() {
    let Some(g) = gpu() else { return };
    let cube = Model::cube(&g, 0.5, Material::default());
    let cam = Camera::orthographic(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, 4.0);
    let lit = Lighting::default();
    let render = |inst: &[Instance]| {
        let mut t = Target::offscreen(&g, N, N);
        let mut r = SceneRenderer::new(&g, t.format());
        let f = t.frame(&g).expect("frame");
        r.render(&g, &f, &cam, &lit, Some(wgpu::Color::BLACK), inst);
        f.present(&g);
        (t.read_rgba(&g).expect("readback"), r.last_draw_calls)
    };
    let all: Vec<Instance> = (0..16)
        .map(|i| Instance { model: &cube, transform: transform(Vec3::new((i % 4) as f32 - 1.5, (i / 4) as f32 - 1.5, 0.0), Vec3::new(30.0, i as f32 * 20.0, 0.0), Vec3::ONE) })
        .collect();
    let (batched, calls) = render(&all);
    assert_eq!(calls, 16);
    // the same frame drawn by accumulating one instance at a time onto a target
    let mut t = Target::offscreen(&g, N, N);
    let mut r = SceneRenderer::new(&g, t.format());
    let f = t.frame(&g).expect("frame");
    r.render(&g, &f, &cam, &lit, Some(wgpu::Color::BLACK), &all[..1]);
    f.present(&g);
    // instances do not overlap, so drawing them separately (depth cleared each time) matches
    for inst in &all[1..] {
        let f = t.frame(&g).expect("frame");
        r.render(&g, &f, &cam, &lit, None, std::slice::from_ref(inst));
        f.present(&g);
    }
    let single = t.read_rgba(&g).expect("readback");
    assert_eq!(batched, single);
}
