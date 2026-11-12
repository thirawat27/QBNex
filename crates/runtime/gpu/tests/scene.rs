//! Headless 3D tests: lit models (`_MESH`, `_LOADMODEL`, `_CAMERA`,
//! `_LIGHT`). Rendered offscreen and read back; every test skips (passes with
//! a note) when the machine has no GPU adapter. No windows are opened.

use glam::{Mat4, Vec3};
use qb_gpu::scene::{compute_normals, transform, Image, Instance, Light, Lighting, Material, Model, SceneRenderer};
use qb_gpu::three::Camera;
use qb_gpu::wgpu;
use qb_gpu::{Gpu, Target};

const N: u32 = 32;

fn gpu() -> Option<Gpu> {
    let g = Gpu::headless();
    match &g {
        Some(g) => eprintln!("qb-gpu adapter: {}", g.adapter_name()),
        None => eprintln!("skipped: no GPU adapter"),
    }
    g
}

fn render(g: &Gpu, cam: &Camera, lighting: &Lighting, inst: &[Instance]) -> Vec<u8> {
    let mut t = Target::offscreen(g, N, N);
    let mut r = SceneRenderer::new(g, t.format());
    let f = t.frame(g).expect("frame");
    r.render(g, &f, cam, lighting, Some(wgpu::Color::BLACK), inst);
    f.present(g);
    t.read_rgba(g).expect("readback")
}

fn px(rgba: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * N + x) * 4) as usize;
    [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
}

fn near(a: [u8; 4], b: [u8; 4], tol: u8) -> bool {
    a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= tol)
}

fn front_cam() -> Camera {
    Camera::orthographic(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, 4.0)
}

fn white() -> Material {
    Material::default()
}

fn only(lights: Vec<Light>, ambient: f32) -> Lighting {
    Lighting { ambient: [ambient; 3], lights }
}

#[test]
fn normals_are_computed_from_faces() {
    let n = compute_normals(&[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], &[0, 1, 2]);
    assert_eq!(n, vec![[0.0, 0.0, 1.0]; 3]);
}

#[test]
fn directional_light_follows_the_angle() {
    let Some(g) = gpu() else { return };
    let cube = Model::cube(&g, 2.0, white());
    let inst = [Instance { model: &cube, transform: Mat4::IDENTITY }];
    // straight onto the front face: full brightness
    let head_on = only(vec![Light::Directional { direction: Vec3::new(0.0, 0.0, -1.0), color: [1.0; 3], intensity: 1.0 }], 0.0);
    let c = px(&render(&g, &front_cam(), &head_on, &inst), 16, 16);
    assert!(near(c, [255, 255, 255, 255], 2), "{c:?}");
    // 60 degrees off: cos = 0.5
    let a = 60f32.to_radians();
    let slanted = only(vec![Light::Directional { direction: Vec3::new(a.sin(), 0.0, -a.cos()), color: [1.0; 3], intensity: 1.0 }], 0.0);
    let c = px(&render(&g, &front_cam(), &slanted, &inst), 16, 16);
    assert!(near(c, [128, 128, 128, 255], 2), "{c:?}");
    // from behind: only ambient (none) -> black
    let behind = only(vec![Light::Directional { direction: Vec3::new(0.0, 0.0, 1.0), color: [1.0; 3], intensity: 1.0 }], 0.0);
    let c = px(&render(&g, &front_cam(), &behind, &inst), 16, 16);
    assert!(near(c, [0, 0, 0, 255], 1), "{c:?}");
}

#[test]
fn ambient_and_light_colour_multiply_the_base_colour() {
    let Some(g) = gpu() else { return };
    let cube = Model::cube(&g, 2.0, Material { base_color: [1.0, 0.5, 1.0, 1.0], texture: None });
    let inst = [Instance { model: &cube, transform: Mat4::IDENTITY }];
    let c = px(&render(&g, &front_cam(), &only(vec![], 0.25), &inst), 16, 16);
    assert!(near(c, [64, 32, 64, 255], 2), "{c:?}");
    let red = only(vec![Light::Directional { direction: Vec3::new(0.0, 0.0, -1.0), color: [1.0, 0.0, 0.0], intensity: 1.0 }], 0.0);
    let c = px(&render(&g, &front_cam(), &red, &inst), 16, 16);
    assert!(near(c, [255, 0, 0, 255], 2), "{c:?}");
}

#[test]
fn point_light_falls_off_with_distance() {
    let Some(g) = gpu() else { return };
    let cube = Model::cube(&g, 1.0, white());
    let inst = [Instance { model: &cube, transform: Mat4::IDENTITY }];
    // front face at z = 0.5; range 0 means 1/d^2
    let at = |z: f32| only(vec![Light::Point { position: Vec3::new(0.0, 0.0, z), color: [1.0; 3], intensity: 1.0, range: 0.0 }], 0.0);
    let close = px(&render(&g, &front_cam(), &at(1.5), &inst), 16, 16); // d = 1
    let far = px(&render(&g, &front_cam(), &at(4.0), &inst), 16, 16); // d = 3.5 -> 1/12.25
    assert!(near(close, [255, 255, 255, 255], 4), "{close:?}");
    assert!(near(far, [21, 21, 21, 255], 3), "{far:?}");
    // with a range: 1 / (1 + (d/range)^2) = 0.5 at d = range
    let ranged = only(vec![Light::Point { position: Vec3::new(0.0, 0.0, 2.5), color: [1.0; 3], intensity: 1.0, range: 2.0 }], 0.0);
    let c = px(&render(&g, &front_cam(), &ranged, &inst), 16, 16);
    assert!(near(c, [128, 128, 128, 255], 2), "{c:?}");
}

#[test]
fn depth_keeps_the_nearest_model_in_any_draw_order() {
    let Some(g) = gpu() else { return };
    let red = Model::cube(&g, 1.0, Material { base_color: [1.0, 0.0, 0.0, 1.0], texture: None });
    let blue = Model::cube(&g, 3.0, Material { base_color: [0.0, 0.0, 1.0, 1.0], texture: None });
    let lit = only(vec![], 1.0);
    let cam = Camera::perspective(Vec3::new(0.0, 0.0, 6.0), Vec3::ZERO, 60f32.to_radians());
    let a = Instance { model: &red, transform: Mat4::IDENTITY };
    let b = Instance { model: &blue, transform: transform(Vec3::new(0.0, 0.0, -3.0), Vec3::ZERO, Vec3::ONE) };
    for order in [[a, b], [b, a]] {
        let out = render(&g, &cam, &lit, &order);
        assert!(near(px(&out, 16, 16), [255, 0, 0, 255], 1), "centre");
        assert!(near(px(&out, 16, 12), [0, 0, 255, 255], 1), "around");
        assert!(near(px(&out, 0, 0), [0, 0, 0, 255], 0), "background");
    }
}

#[test]
fn textured_mesh_from_arrays() {
    let Some(g) = gpu() else { return };
    let img = Image { width: 2, height: 2, rgba: vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255] };
    let pos = [[-2.0, 2.0, 0.0], [2.0, 2.0, 0.0], [2.0, -2.0, 0.0], [-2.0, -2.0, 0.0]];
    let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let m = Model::from_arrays(&g, &pos, &[], &uv, &[0, 2, 1, 0, 3, 2], white(), Some(img)).expect("mesh");
    assert_eq!((m.parts.len(), m.parts[0].vertex_count, m.parts[0].index_count), (1, 4, 6));
    let out = render(&g, &front_cam(), &only(vec![], 1.0), &[Instance { model: &m, transform: Mat4::IDENTITY }]);
    assert!(near(px(&out, 8, 8), [255, 0, 0, 255], 20)); // bilinear: ~6% of the neighbour texel
    assert!(near(px(&out, 24, 8), [0, 255, 0, 255], 20)); // bilinear: ~6% of the neighbour texel
    assert!(near(px(&out, 8, 24), [0, 0, 255, 255], 20)); // bilinear: ~6% of the neighbour texel
    assert!(near(px(&out, 24, 24), [255, 255, 255, 255], 20)); // bilinear: ~6% of the neighbour texel
    assert!(Model::from_arrays(&g, &pos, &[], &[], &[0, 1, 9], white(), None).is_err());
}

/// A binary glTF: one 1x1 quad in XY (no normals), node translated by +1 in x
/// and scaled 2x, material base colour green.
fn quad_glb() -> Vec<u8> {
    let mut bin = Vec::new();
    for p in [[-0.5f32, -0.5, 0.0], [0.5, -0.5, 0.0], [0.5, 0.5, 0.0], [-0.5, 0.5, 0.0]] {
        for c in p {
            bin.extend(c.to_le_bytes());
        }
    }
    for i in [0u32, 1, 2, 0, 2, 3] {
        bin.extend(i.to_le_bytes());
    }
    let json = format!(
        concat!(
            r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0]}}],"#,
            r#""nodes":[{{"mesh":0,"translation":[1,0,0],"scale":[2,2,2]}}],"#,
            r#""meshes":[{{"primitives":[{{"attributes":{{"POSITION":0}},"indices":1,"material":0}}]}}],"#,
            r#""materials":[{{"pbrMetallicRoughness":{{"baseColorFactor":[0,1,0,1]}}}}],"#,
            r#""buffers":[{{"byteLength":{}}}],"#,
            r#""bufferViews":[{{"buffer":0,"byteOffset":0,"byteLength":48}},{{"buffer":0,"byteOffset":48,"byteLength":24}}],"#,
            r#""accessors":[{{"bufferView":0,"componentType":5126,"count":4,"type":"VEC3","min":[-0.5,-0.5,0],"max":[0.5,0.5,0]}},"#,
            r#"{{"bufferView":1,"componentType":5125,"count":6,"type":"SCALAR"}}]}}"#
        ),
        bin.len()
    );
    let mut j = json.into_bytes();
    while j.len() % 4 != 0 {
        j.push(b' ');
    }
    let total = 12 + 8 + j.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend(b"glTF");
    out.extend(2u32.to_le_bytes());
    out.extend((total as u32).to_le_bytes());
    out.extend((j.len() as u32).to_le_bytes());
    out.extend(0x4E4F_534Au32.to_le_bytes());
    out.extend(&j);
    out.extend((bin.len() as u32).to_le_bytes());
    out.extend(0x004E_4942u32.to_le_bytes());
    out.extend(&bin);
    out
}

#[test]
fn glb_model_with_node_transform_and_material() {
    let Some(g) = gpu() else { return };
    let m = Model::from_gltf(&g, &quad_glb()).expect("glb");
    assert_eq!(m.parts.len(), 1);
    assert_eq!(m.parts[0].material.base_color, [0.0, 1.0, 0.0, 1.0]);
    assert_eq!(m.bounds, (Vec3::new(0.0, -1.0, 0.0), Vec3::new(2.0, 1.0, 0.0)));
    let cam = Camera::orthographic(Vec3::new(1.0, 0.0, 5.0), Vec3::new(1.0, 0.0, 0.0), 4.0);
    // computed normals face +z: a head-on light gives full green
    let lit = only(vec![Light::Directional { direction: Vec3::new(0.0, 0.0, -1.0), color: [1.0; 3], intensity: 1.0 }], 0.0);
    let out = render(&g, &cam, &lit, &[Instance { model: &m, transform: Mat4::IDENTITY }]);
    assert!(near(px(&out, 16, 16), [0, 255, 0, 255], 2));
    // the quad spans x 0..2 of the 4-unit view centred on x = 1: columns 8..24
    assert!(near(px(&out, 4, 16), [0, 0, 0, 255], 0));
    assert!(near(px(&out, 10, 16), [0, 255, 0, 255], 2));
    assert!(Model::from_gltf(&g, b"not a model").is_err());
}
