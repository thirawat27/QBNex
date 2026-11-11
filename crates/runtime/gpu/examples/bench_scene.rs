//! R15 G6 benchmark: a 10k-triangle scene rendered offscreen (no window).
//!
//! `cargo run --release -p qb-gpu --example bench_scene [frames]`
//!
//! Scenes: 834 instances of a cube (12 triangles each, 10 008 triangles,
//! one draw per instance: the `_DRAW3D`-in-a-loop case) and one mesh of
//! 10 000 triangles (one draw). Each frame waits for the GPU, so the time is
//! the whole frame. Target: >= 60 fps.

use std::time::Instant;

use qb_gpu::glam::{Mat4, Vec3};
use qb_gpu::scene::{transform, Instance, Lighting, Material, Model, SceneRenderer};
use qb_gpu::three::Camera;
use qb_gpu::{wgpu, Gpu, Target};

fn time(name: &str, gpu: &Gpu, frames: u32, instances: &[Instance], tris: usize) {
    let mut t = Target::offscreen(gpu, 512, 512);
    let mut r = SceneRenderer::new(gpu, t.format());
    let cam = Camera::perspective(Vec3::new(0.0, 12.0, 40.0), Vec3::ZERO, 60f32.to_radians());
    let lit = Lighting::default();
    let mut frame = |gpu: &Gpu| {
        let f = t.frame(gpu).expect("frame");
        r.render(gpu, &f, &cam, &lit, Some(wgpu::Color::BLACK), instances);
        f.present(gpu);
        gpu.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).expect("poll");
    };
    for _ in 0..5 {
        frame(gpu);
    }
    let start = Instant::now();
    for _ in 0..frames {
        frame(gpu);
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    println!("{name}: {tris} triangles, {} draws: {ms:.2} ms/frame, {:.0} fps", instances.len(), 1000.0 / ms);
}

fn main() {
    let frames = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(200u32);
    let Some(gpu) = Gpu::headless() else {
        println!("no GPU adapter");
        return;
    };
    println!("adapter: {}", gpu.adapter_name());
    let cube = Model::cube(&gpu, 0.8, Material::default());
    let instances: Vec<Instance> = (0..834)
        .map(|i| {
            let (x, z) = ((i % 29) as f32 - 14.0, (i / 29) as f32 - 14.0);
            Instance { model: &cube, transform: transform(Vec3::new(x, 0.0, z), Vec3::new(0.0, i as f32 * 7.0, 0.0), Vec3::ONE) }
        })
        .collect();
    time("834 cubes", &gpu, frames, &instances, 834 * 12);
    // one 10 000-triangle grid mesh (100 x 50 quads)
    let (nx, nz) = (100usize, 50usize);
    let mut pos = Vec::new();
    for z in 0..=nz {
        for x in 0..=nx {
            let (fx, fz) = (x as f32 / 4.0 - 12.5, z as f32 / 4.0 - 6.25);
            pos.push([fx, (fx * 0.7).sin() * (fz * 0.5).cos(), fz]);
        }
    }
    let mut idx = Vec::new();
    for z in 0..nz {
        for x in 0..nx {
            let a = (z * (nx + 1) + x) as u32;
            let b = a + (nx + 1) as u32;
            idx.extend([a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    let grid = Model::from_arrays(&gpu, &pos, &[], &[], &idx, Material::default(), None).expect("grid");
    time("grid mesh", &gpu, frames, &[Instance { model: &grid, transform: Mat4::IDENTITY }], idx.len() / 3);
}
