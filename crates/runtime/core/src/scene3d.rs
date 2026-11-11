//! R15 G4: the runtime side of the 3D API (`_LOADMODEL`, `_MESHCUBE`,
//! `_MESHPLANE`, `_MESHBEGIN`/`_MESHVERTEX`/`_MESHTRIANGLE`/`_MESHEND`,
//! `_MESHCOLOR`, `_MESHTEXTURE`, `_FREEMESH`, `_DRAW3D`, `_CAMERA`,
//! `_CAMERAORTHO`, `_CAMERARANGE`, `_AMBIENT`, `_LIGHT`, `_LIGHTON`,
//! `_FREELIGHT`, `_CLEAR3D`, `_GPU3D`). Design: `docs/dev/3d-api.md`.
//!
//! Handles and state live here for both execution paths (compiled programs
//! through the `qb_3d_*` C ABI and the shims in libqb, QIR programs through
//! `qb-exec`). `_DRAW3D` queues a draw; `_DISPLAY` (seen through
//! [`frame_done`], called by the frame capture hook every `_DISPLAY`) turns
//! the queue into the shown 3D layer. The layer is rendered on the GPU
//! (feature `gpu`, `qb_gpu::scene`) into a transparent image and blended over
//! the SCREEN frame when the window presents it ([`qb_3d_present`]) and in
//! captured frames. Without the feature or an adapter every statement still
//! works (handles, errors) but nothing is drawn and `_GPU3D` is 0.
//!
//! Errors: an unknown mesh/light handle is error 258 (invalid handle),
//! bad arguments error 5. Functions creating handles return -1 on failure.

use std::sync::Mutex;

pub const ERR_ILLEGAL: i32 = 5;
pub const ERR_HANDLE: i32 = 258;

/// Draws queued in one frame at most (a runaway loop without `_DISPLAY`).
const MAX_QUEUE: usize = 1 << 20;

#[derive(Debug, Clone, PartialEq)]
enum Shape {
    Gltf(Vec<u8>),
    Cube(f32),
    Plane(f32, f32),
    Built { pos: Vec<[f32; 3]>, uv: Vec<[f32; 2]>, idx: Vec<u32> },
}

struct Mesh {
    #[cfg_attr(not(feature = "gpu"), allow(dead_code))]
    shape: Shape,
    color: Option<[f32; 4]>,
    texture: Option<(Vec<u32>, u32, u32)>,
    /// `_MESHSHADER` handle, 0 = lit
    shader: i32,
    #[cfg(feature = "gpu")]
    model: Option<Option<qb_gpu::scene::Model>>,
}

/// A `_SHADER`: the 16 `_SHADERPARAM` values and (with a GPU layer) the
/// compiled form.
struct ShaderEntry {
    params: [f32; 16],
    #[cfg(feature = "gpu")]
    compiled: std::sync::Arc<qb_gpu::scene::Shader>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Light3 {
    /// 1 directional, 2 point
    pub kind: i32,
    pub v: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
    pub range: f32,
    pub on: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera3 {
    pub eye: [f32; 3],
    pub target: [f32; 3],
    /// perspective: vertical field of view in degrees; orthographic: visible height
    pub fov_or_height: f32,
    pub ortho: bool,
    pub near: f32,
    pub far: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Draw3 {
    pub mesh: i32,
    /// position, rotation (degrees), scale
    pub t: [f32; 9],
}

struct Builder {
    pos: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    idx: Vec<u32>,
}

struct State {
    meshes: Vec<Option<Mesh>>,
    shaders: Vec<Option<ShaderEntry>>,
    /// message of the last `_SHADER` that did not compile (`_SHADERERROR$`)
    shader_error: String,
    lights: Vec<Option<Light3>>,
    lights_touched: bool,
    ambient: [f32; 3],
    camera: Camera3,
    builder: Option<Builder>,
    queue: Vec<Draw3>,
    shown: Vec<Draw3>,
    /// layer clear colour (`_CLEAR3D`); 0 = transparent
    clear: u32,
    /// bumped whenever what the shown layer looks like may change
    generation: u64,
}

const DEFAULT_CAMERA: Camera3 = Camera3 { eye: [0.0, 0.0, 5.0], target: [0.0; 3], fov_or_height: 60.0, ortho: false, near: 0.1, far: 1000.0 };

static STATE: Mutex<State> = Mutex::new(State {
    meshes: Vec::new(),
    shaders: Vec::new(),
    shader_error: String::new(),
    lights: Vec::new(),
    lights_touched: false,
    ambient: [0.2; 3],
    camera: DEFAULT_CAMERA,
    builder: None,
    queue: Vec::new(),
    shown: Vec::new(),
    clear: 0,
    generation: 0,
});

fn state() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

fn rgba(c: u32) -> [f32; 4] {
    let ch = |s: u32| ((c >> s) & 255) as f32 / 255.0;
    [ch(16), ch(8), ch(0), ch(24)]
}

fn add<T>(v: &mut Vec<Option<T>>, x: T) -> i32 {
    if let Some(i) = v.iter().position(Option::is_none) {
        v[i] = Some(x);
        return i as i32 + 1;
    }
    v.push(Some(x));
    v.len() as i32
}

fn slot<T>(v: &mut [Option<T>], h: i32) -> Option<&mut T> {
    if h < 1 {
        return None;
    }
    v.get_mut(h as usize - 1).and_then(Option::as_mut)
}

fn new_mesh(shape: Shape) -> Mesh {
    Mesh {
        shape,
        color: None,
        texture: None,
        shader: 0,
        #[cfg(feature = "gpu")]
        model: None,
    }
}

fn add_mesh(shape: Shape) -> i32 {
    let mut s = state();
    add(&mut s.meshes, new_mesh(shape))
}

/// Resets everything (tests, `RUN`).
pub fn reset() {
    let mut s = state();
    s.meshes.clear();
    s.shaders.clear();
    s.shader_error.clear();
    s.lights.clear();
    s.lights_touched = false;
    s.ambient = [0.2; 3];
    s.camera = DEFAULT_CAMERA;
    s.builder = None;
    s.queue.clear();
    s.shown.clear();
    s.clear = 0;
    s.generation += 1;
}

/// `_DISPLAY`: the queued draws become the shown 3D layer.
pub fn frame_done() {
    let mut s = state();
    let q = std::mem::take(&mut s.queue);
    if q.is_empty() && s.shown.is_empty() {
        return;
    }
    s.shown = q;
    s.generation += 1;
}

/// Draws of the shown layer (tests and diagnostics).
pub fn shown() -> Vec<Draw3> {
    state().shown.clone()
}

/// Draws queued for the next `_DISPLAY`.
pub fn queued() -> Vec<Draw3> {
    state().queue.clone()
}

pub fn camera() -> Camera3 {
    state().camera
}

pub fn light(h: i32) -> Option<Light3> {
    let mut s = state();
    slot(&mut s.lights, h).copied()
}

// ---- C ABI -------------------------------------------------------------------

/// `_GPU3D`: -1 when 3D can be drawn (feature `gpu` and an adapter), else 0.
#[unsafe(no_mangle)]
pub extern "C" fn qb_3d_available() -> i32 {
    #[cfg(feature = "gpu")]
    {
        if gpu::with(|_| ()).is_some() {
            return -1;
        }
    }
    0
}

/// `_LOADMODEL(file$)`: a glTF 2.0 file (`.glb`, or `.gltf` with embedded
/// buffers). -1 when it cannot be read or is not glTF.
///
/// # Safety
/// `path` readable for `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_3d_load_model(path: *const u8, len: i32) -> i32 {
    // SAFETY: caller contract.
    let p = unsafe { crate::ffi_bytes(path, len.max(0) as usize) };
    let Ok(p) = std::str::from_utf8(p) else { return -1 };
    let Ok(bytes) = std::fs::read(p) else { return -1 };
    let is_glb = bytes.starts_with(b"glTF");
    let is_json = bytes.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'{');
    if !is_glb && !is_json {
        return -1;
    }
    #[cfg(feature = "gpu")]
    if let Some(Err(_)) = gpu::with(|g| qb_gpu::scene::Model::from_gltf(&g.gpu, &bytes)) {
        return -1;
    }
    add_mesh(Shape::Gltf(bytes))
}

/// `_MESHCUBE(size!)`.
#[unsafe(no_mangle)]
pub extern "C" fn qb_3d_mesh_cube(size: f32) -> i32 {
    if !(size.is_finite() && size > 0.0) {
        return -1;
    }
    add_mesh(Shape::Cube(size))
}

/// `_MESHPLANE(width!, depth!)`.
#[unsafe(no_mangle)]
pub extern "C" fn qb_3d_mesh_plane(w: f32, d: f32) -> i32 {
    if !(w.is_finite() && d.is_finite() && w > 0.0 && d > 0.0) {
        return -1;
    }
    add_mesh(Shape::Plane(w, d))
}

/// `_MESHBEGIN`: starts a mesh built vertex by vertex (drops an unfinished one).
#[unsafe(no_mangle)]
pub extern "C" fn qb_3d_mesh_begin() {
    state().builder = Some(Builder { pos: Vec::new(), uv: Vec::new(), idx: Vec::new() });
}

/// `_MESHVERTEX x!, y!, z! [, u!, v!]` (u, v from 0 to 1 over the texture).
/// Vertices are numbered from 0. Error 5 without `_MESHBEGIN`.
#[unsafe(no_mangle)]
pub extern "C" fn qb_3d_mesh_vertex(x: f32, y: f32, z: f32, u: f32, v: f32) -> i32 {
    let mut s = state();
    let Some(b) = s.builder.as_mut() else { return ERR_ILLEGAL };
    b.pos.push([x, y, z]);
    b.uv.push([u, v]);
    0
}

/// `_MESHTRIANGLE a&, b&, c&` (vertex numbers). Error 5 without `_MESHBEGIN`
/// or for a vertex not given yet.
#[unsafe(no_mangle)]
pub extern "C" fn qb_3d_mesh_triangle(a: i32, b: i32, c: i32) -> i32 {
    let mut s = state();
    let Some(bl) = s.builder.as_mut() else { return ERR_ILLEGAL };
    let n = bl.pos.len() as i32;
    if [a, b, c].iter().any(|&i| i < 0 || i >= n) {
        return ERR_ILLEGAL;
    }
    bl.idx.extend([a as u32, b as u32, c as u32]);
    0
}

/// `_MESHEND`: the built mesh's handle; -1 without `_MESHBEGIN` or triangles.
#[unsafe(no_mangle)]
pub extern "C" fn qb_3d_mesh_end() -> i32 {
    let mut s = state();
    let Some(b) = s.builder.take() else { return -1 };
    if b.idx.is_empty() {
        return -1;
    }
    add(&mut s.meshes, new_mesh(Shape::Built { pos: b.pos, uv: b.uv, idx: b.idx }))
}

/// `_MESHCOLOR mesh&, colour~&` (`_RGBA32` value; multiplies the texture).
#[unsafe(no_mangle)]
pub extern "C" fn qb_3d_mesh_color(h: i32, color: u32) -> i32 {
    let mut s = state();
    let Some(m) = slot(&mut s.meshes, h) else { return ERR_HANDLE };
    m.color = Some(rgba(color));
    #[cfg(feature = "gpu")]
    {
        m.model = None;
    }
    s.generation += 1;
    0
}

/// `_MESHTEXTURE mesh&, image&` with the image's 32-bit pixels.
///
/// # Safety
/// `px` readable for `w*h` pixels.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_3d_mesh_texture(h: i32, px: *const u32, w: i32, hh: i32) -> i32 {
    if px.is_null() || w <= 0 || hh <= 0 {
        return ERR_ILLEGAL;
    }
    // SAFETY: caller contract.
    let v = unsafe { std::slice::from_raw_parts(px, w as usize * hh as usize) }.to_vec();
    let mut s = state();
    let Some(m) = slot(&mut s.meshes, h) else { return ERR_HANDLE };
    m.texture = Some((v, w as u32, hh as u32));
    #[cfg(feature = "gpu")]
    {
        m.model = None;
    }
    s.generation += 1;
    0
}

/// `_FREEMESH mesh&`. Queued and shown draws of it are dropped.
#[unsafe(no_mangle)]
pub extern "C" fn qb_3d_free_mesh(h: i32) -> i32 {
    let mut s = state();
    if slot(&mut s.meshes, h).is_none() {
        return ERR_HANDLE;
    }
    s.meshes[h as usize - 1] = None;
    s.queue.retain(|d| d.mesh != h);
    s.shown.retain(|d| d.mesh != h);
    s.generation += 1;
    0
}

/// `_SHADER(wgsl$)`: user WGSL defining `fn shade(i: ShadeIn) -> vec4<f32>`
/// (see `qb_gpu::scene::Shader`). Returns the handle, or -1 when it does not
/// compile (`_SHADERERROR$` tells why). Without the `gpu` feature the text is
/// kept unchecked.
///
/// # Safety
/// `src` readable for `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_3d_shader(src: *const u8, len: i32) -> i32 {
    // SAFETY: caller contract.
    #[cfg_attr(not(feature = "gpu"), allow(unused_variables))]
    let text = String::from_utf8_lossy(unsafe { crate::ffi_bytes(src, len.max(0) as usize) }).into_owned();
    #[cfg(feature = "gpu")]
    let compiled = match qb_gpu::scene::Shader::compile(&text) {
        Ok(sh) => std::sync::Arc::new(sh),
        Err(e) => {
            state().shader_error = e.to_string();
            return -1;
        }
    };
    let mut s = state();
    s.shader_error.clear();
    add(
        &mut s.shaders,
        ShaderEntry {
            params: [0.0; 16],
            #[cfg(feature = "gpu")]
            compiled,
        },
    )
}

/// `_SHADERERROR$`: why the last `_SHADER` failed ("" after a success).
pub fn shader_error() -> String {
    state().shader_error.clone()
}

/// C form of [`shader_error`]: copies up to `cap` bytes, returns the full length.
///
/// # Safety
/// `out` writable for `cap` bytes (may be null when `cap` is 0).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_3d_shader_error(out: *mut u8, cap: i32) -> i32 {
    let e = shader_error();
    let n = e.len().min(cap.max(0) as usize);
    if n > 0 {
        // SAFETY: caller contract.
        unsafe { std::ptr::copy_nonoverlapping(e.as_ptr(), out, n) };
    }
    e.len() as i32
}

/// `_SHADERPARAM shader&, index&, value!` (index 0..15 = `i.params[index / 4][index % 4]`).
#[unsafe(no_mangle)]
pub extern "C" fn qb_3d_shader_param(h: i32, index: i32, value: f32) -> i32 {
    let mut s = state();
    let Some(sh) = slot(&mut s.shaders, h) else { return ERR_HANDLE };
    if !(0..16).contains(&index) || !value.is_finite() {
        return ERR_ILLEGAL;
    }
    sh.params[index as usize] = value;
    s.generation += 1;
    0
}

/// `_MESHSHADER mesh&, shader&` (0 = the lit default).
#[unsafe(no_mangle)]
pub extern "C" fn qb_3d_mesh_shader(mesh: i32, shader: i32) -> i32 {
    let mut s = state();
    if shader != 0 && slot(&mut s.shaders, shader).is_none() {
        return ERR_HANDLE;
    }
    let Some(m) = slot(&mut s.meshes, mesh) else { return ERR_HANDLE };
    m.shader = shader;
    s.generation += 1;
    0
}

/// `_FREESHADER shader&`; meshes using it go back to the lit default.
#[unsafe(no_mangle)]
pub extern "C" fn qb_3d_free_shader(h: i32) -> i32 {
    let mut s = state();
    if slot(&mut s.shaders, h).is_none() {
        return ERR_HANDLE;
    }
    s.shaders[h as usize - 1] = None;
    for m in s.meshes.iter_mut().flatten() {
        if m.shader == h {
            m.shader = 0;
        }
    }
    s.generation += 1;
    0
}

/// Shader handle of a mesh (tests).
pub fn mesh_shader(mesh: i32) -> Option<i32> {
    let mut s = state();
    slot(&mut s.meshes, mesh).map(|m| m.shader)
}

/// `_DRAW3D mesh& [, x!, y!, z!] [, rx!, ry!, rz!] [, sx!, sy!, sz!]`:
/// queued for the next `_DISPLAY`. Angles in degrees (Y, then X, then Z).
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn qb_3d_draw(h: i32, x: f32, y: f32, z: f32, rx: f32, ry: f32, rz: f32, sx: f32, sy: f32, sz: f32) -> i32 {
    let mut s = state();
    if slot(&mut s.meshes, h).is_none() {
        return ERR_HANDLE;
    }
    let t = [x, y, z, rx, ry, rz, sx, sy, sz];
    if t.iter().any(|v| !v.is_finite()) {
        return ERR_ILLEGAL;
    }
    if s.queue.len() < MAX_QUEUE {
        s.queue.push(Draw3 { mesh: h, t });
    }
    0
}

/// `_CAMERA (x!, y!, z!) TO (tx!, ty!, tz!) [, fov!]` (`ortho` 0, `fov` degrees)
/// or `_CAMERAORTHO ... , height!` (`ortho` 1). Error 5 for eye = target or a
/// field of view outside 1..179 / height <= 0.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn qb_3d_camera(ex: f32, ey: f32, ez: f32, tx: f32, ty: f32, tz: f32, fov_or_height: f32, ortho: i32) -> i32 {
    let v = [ex, ey, ez, tx, ty, tz, fov_or_height];
    if v.iter().any(|x| !x.is_finite()) || (ex, ey, ez) == (tx, ty, tz) {
        return ERR_ILLEGAL;
    }
    let ok = if ortho != 0 { fov_or_height > 0.0 } else { (1.0..=179.0).contains(&fov_or_height) };
    if !ok {
        return ERR_ILLEGAL;
    }
    let mut s = state();
    let (near, far) = (s.camera.near, s.camera.far);
    s.camera = Camera3 { eye: [ex, ey, ez], target: [tx, ty, tz], fov_or_height, ortho: ortho != 0, near, far };
    s.generation += 1;
    0
}

/// `_CAMERARANGE near!, far!` (0 < near < far).
#[unsafe(no_mangle)]
pub extern "C" fn qb_3d_camera_range(near: f32, far: f32) -> i32 {
    if !(near.is_finite() && far.is_finite() && near > 0.0 && far > near) {
        return ERR_ILLEGAL;
    }
    let mut s = state();
    s.camera.near = near;
    s.camera.far = far;
    s.generation += 1;
    0
}

/// `_AMBIENT colour~& [, intensity!]` (intensity defaults to 1).
#[unsafe(no_mangle)]
pub extern "C" fn qb_3d_ambient(color: u32, intensity: f32) -> i32 {
    if !intensity.is_finite() || intensity < 0.0 {
        return ERR_ILLEGAL;
    }
    let c = rgba(color);
    let mut s = state();
    s.ambient = [c[0] * intensity, c[1] * intensity, c[2] * intensity];
    s.lights_touched = true;
    s.generation += 1;
    0
}

/// `_LIGHT(kind&, x!, y!, z!, colour~&, intensity! [, range!])`: kind 1 =
/// directional (x, y, z = direction the light travels), 2 = point (x, y, z =
/// position). Returns the handle, -1 for a bad kind or values (the shims
/// raise error 5 then).
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn qb_3d_light(kind: i32, x: f32, y: f32, z: f32, color: u32, intensity: f32, range: f32) -> i32 {
    if !(kind == 1 || kind == 2) || [x, y, z, intensity, range].iter().any(|v| !v.is_finite()) || intensity < 0.0 {
        return -1;
    }
    if kind == 1 && (x, y, z) == (0.0, 0.0, 0.0) {
        return -1;
    }
    let c = rgba(color);
    let mut s = state();
    s.lights_touched = true;
    s.generation += 1;
    add(&mut s.lights, Light3 { kind, v: [x, y, z], color: [c[0], c[1], c[2]], intensity, range, on: true })
}

/// `_LIGHTON light&, on&` (0 off, else on).
#[unsafe(no_mangle)]
pub extern "C" fn qb_3d_light_on(h: i32, on: i32) -> i32 {
    let mut s = state();
    let Some(l) = slot(&mut s.lights, h) else { return ERR_HANDLE };
    l.on = on != 0;
    s.generation += 1;
    0
}

/// `_FREELIGHT light&`.
#[unsafe(no_mangle)]
pub extern "C" fn qb_3d_free_light(h: i32) -> i32 {
    let mut s = state();
    if slot(&mut s.lights, h).is_none() {
        return ERR_HANDLE;
    }
    s.lights[h as usize - 1] = None;
    s.generation += 1;
    0
}

/// `_CLEAR3D [colour~&]`: drops this frame's queued draws; the layer is
/// cleared to `colour` (transparent when `passed` is 0).
#[unsafe(no_mangle)]
pub extern "C" fn qb_3d_clear(color: u32, passed: i32) {
    let mut s = state();
    s.queue.clear();
    s.clear = if passed & 1 != 0 { color } else { 0 };
    s.generation += 1;
}

/// The window presenter's hook: blends the shown 3D layer over the composed
/// frame `px` (`w`×`h`, `0xAARRGGBB`). In `_AUTODISPLAY` mode (no
/// `_DISPLAY`) the queue is shown as it is.
///
/// # Safety
/// `px` writable for `w*h` pixels.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_3d_present(px: *mut u32, w: i32, h: i32) {
    if px.is_null() || w <= 0 || h <= 0 {
        return;
    }
    {
        let mut s = state();
        if !s.queue.is_empty() && s.shown.is_empty() {
            s.shown = std::mem::take(&mut s.queue);
            s.generation += 1;
        }
    }
    // SAFETY: caller contract.
    let frame = unsafe { std::slice::from_raw_parts_mut(px, w as usize * h as usize) };
    compose(frame, w as u32, h as u32);
}

/// Blends the shown layer over `frame`; no-op without a GPU or a layer.
pub fn compose(frame: &mut [u32], w: u32, h: u32) {
    #[cfg(feature = "gpu")]
    if let Some(layer) = gpu::layer(w, h) {
        for (d, &l) in frame.iter_mut().zip(layer.iter()) {
            if l >> 24 != 0 {
                *d = crate::raster::blend(*d, l);
            }
        }
    }
    #[cfg(not(feature = "gpu"))]
    let _ = (frame, w, h);
}

/// Capture hook: composes the shown layer into a frame record's 32-bit pixels.
pub(crate) fn apply_to_record(rec: &mut [u8], w: u32, h: u32, bytes_per_pixel: u32) {
    let idle = {
        let s = state();
        s.shown.is_empty() && s.clear == 0
    };
    if bytes_per_pixel != 4 || idle {
        return;
    }
    let n = (w * h) as usize;
    let Some(body) = rec.get_mut(24..24 + n * 4) else { return };
    let mut px: Vec<u32> = body.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect();
    compose(&mut px, w, h);
    for (c, p) in body.as_chunks_mut::<4>().0.iter_mut().zip(px) {
        *c = p.to_le_bytes();
    }
}

#[cfg(feature = "gpu")]
mod gpu {
    //! The GPU side: a headless device, models built lazily, the layer cache.

    use super::{rgba, state, Camera3, Light3, Mesh, Shape};
    use qb_gpu::glam::Vec3;
    use qb_gpu::scene::{transform, Image, Instance, Light, Lighting, Material, Model, SceneRenderer};
    use qb_gpu::three::{Camera, Projection};
    use qb_gpu::{wgpu, Gpu, Target};
    use std::sync::Mutex;

    pub(super) struct Side {
        pub gpu: Gpu,
        renderer: Option<SceneRenderer>,
        /// (generation, w, h, pixels)
        cache: Option<(u64, u32, u32, Vec<u32>)>,
    }

    /// `None` = not tried yet; `Some(None)` = no adapter.
    static SIDE: Mutex<Option<Option<Side>>> = Mutex::new(None);

    /// Runs `f` with the GPU (created on first use); `None` without an adapter.
    pub(super) fn with<T>(f: impl FnOnce(&mut Side) -> T) -> Option<T> {
        let mut g = SIDE.lock().unwrap_or_else(|e| e.into_inner());
        if g.is_none() {
            *g = Some(Gpu::headless().map(|gpu| Side { gpu, renderer: None, cache: None }));
        }
        g.as_mut().and_then(Option::as_mut).map(f)
    }

    fn build(gpu: &Gpu, m: &Mesh) -> Option<Model> {
        let mat = Material { base_color: m.color.unwrap_or([1.0; 4]), texture: None };
        let mut model = match &m.shape {
            Shape::Gltf(b) => {
                let mut md = Model::from_gltf(gpu, b).ok()?;
                if let Some(c) = m.color {
                    md.set_color(c);
                }
                md
            }
            Shape::Cube(s) => Model::cube(gpu, *s, mat),
            Shape::Plane(w, d) => Model::plane(gpu, *w, *d, mat),
            Shape::Built { pos, uv, idx } => Model::from_arrays(gpu, pos, &[], uv, idx, mat, None).ok()?,
        };
        if let Some((px, w, h)) = &m.texture {
            let rgba = px
                .iter()
                .flat_map(|p| {
                    let [b, g, r, a] = p.to_le_bytes();
                    [r, g, b, a]
                })
                .collect();
            model.set_texture(gpu, Image { width: *w, height: *h, rgba });
        }
        Some(model)
    }

    fn camera(c: &Camera3) -> Camera {
        let projection = if c.ortho {
            Projection::Orthographic { height: c.fov_or_height, near: c.near, far: c.far }
        } else {
            Projection::Perspective { fov_y: c.fov_or_height.to_radians(), near: c.near, far: c.far }
        };
        let eye = Vec3::from(c.eye);
        let target = Vec3::from(c.target);
        // looking straight up or down: pick another up vector
        let dir = (target - eye).normalize_or_zero();
        let up = if dir.cross(Vec3::Y).length_squared() < 1e-6 { Vec3::Z } else { Vec3::Y };
        Camera { eye, target, up, projection }
    }

    fn light(l: &Light3) -> Light {
        if l.kind == 1 {
            Light::Directional { direction: Vec3::from(l.v), color: l.color, intensity: l.intensity }
        } else {
            Light::Point { position: Vec3::from(l.v), color: l.color, intensity: l.intensity, range: l.range }
        }
    }

    /// The shown layer rendered at `w`×`h` (`0xAARRGGBB`, transparent where
    /// nothing was drawn); `None` without a GPU or a layer.
    pub(super) fn layer(w: u32, h: u32) -> Option<Vec<u32>> {
        let mut s = state();
        if s.shown.is_empty() && s.clear == 0 {
            return None;
        }
        let generation = s.generation;
        with(|side| {
            if let Some((g, cw, ch, px)) = &side.cache
                && (*g, *cw, *ch) == (generation, w, h)
            {
                return Some(px.clone());
            }
            let gpu = &side.gpu;
            for d in s.shown.clone() {
                let i = d.mesh as usize - 1;
                if let Some(Some(m)) = s.meshes.get_mut(i)
                    && m.model.is_none()
                {
                    let built = build(gpu, m);
                    m.model = Some(built);
                }
            }
            // _MESHSHADER: the compiled shader and its parameters on each shown model
            let st = &mut *s;
            for d in &st.shown {
                let Some(Some(m)) = st.meshes.get_mut(d.mesh as usize - 1) else { continue };
                let sh = st.shaders.get(m.shader.max(1) as usize - 1).and_then(Option::as_ref).filter(|_| m.shader > 0);
                if let Some(Some(model)) = m.model.as_mut() {
                    model.shader = sh.map(|e| (e.compiled.clone(), e.params));
                }
            }
            let lighting = if s.lights_touched {
                Lighting { ambient: s.ambient, lights: s.lights.iter().flatten().filter(|l| l.on).map(light).collect() }
            } else {
                Lighting::default()
            };
            let instances: Vec<Instance> = s
                .shown
                .iter()
                .filter_map(|d| {
                    let m = s.meshes.get(d.mesh as usize - 1)?.as_ref()?.model.as_ref()?.as_ref()?;
                    let t = d.t;
                    Some(Instance {
                        model: m,
                        transform: transform(Vec3::new(t[0], t[1], t[2]), Vec3::new(t[3], t[4], t[5]), Vec3::new(t[6], t[7], t[8])),
                    })
                })
                .collect();
            let mut target = Target::offscreen_with_format(gpu, w, h, wgpu::TextureFormat::Bgra8Unorm);
            let renderer = side.renderer.get_or_insert_with(|| SceneRenderer::new(gpu, wgpu::TextureFormat::Bgra8Unorm));
            let frame = target.frame(gpu)?;
            let c = rgba(s.clear);
            let clear = wgpu::Color { r: f64::from(c[0]), g: f64::from(c[1]), b: f64::from(c[2]), a: f64::from(c[3]) };
            renderer.render(gpu, &frame, &camera(&s.camera), &lighting, Some(clear), &instances);
            frame.present(gpu);
            let mut px = target.read_pixels(gpu)?;
            // a frame above the device's texture limit was rendered smaller: scale up
            let (lw, lh) = target.size();
            if (lw, lh) != (w, h) {
                let small = std::mem::take(&mut px);
                px = (0..w * h).map(|i| small[((i / w) * lh / h * lw + (i % w) * lw / w) as usize]).collect();
            }
            side.cache = Some((generation, w, h, px.clone()));
            Some(px)
        })
        .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 3D state is global: tests take this lock and start from a reset.
    static SERIAL: Mutex<()> = Mutex::new(());

    fn fresh() -> std::sync::MutexGuard<'static, ()> {
        let g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        g
    }

    #[test]
    fn handles_are_reused_and_checked() {
        let _g = fresh();
        let a = qb_3d_mesh_cube(1.0);
        let b = qb_3d_mesh_plane(2.0, 3.0);
        assert_eq!((a, b), (1, 2));
        assert_eq!(qb_3d_mesh_cube(0.0), -1);
        assert_eq!(qb_3d_mesh_plane(-1.0, 1.0), -1);
        assert_eq!(qb_3d_free_mesh(a), 0);
        assert_eq!(qb_3d_free_mesh(a), ERR_HANDLE);
        assert_eq!(qb_3d_mesh_color(a, 0xFFFF_0000), ERR_HANDLE);
        assert_eq!(qb_3d_draw(a, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0), ERR_HANDLE);
        assert_eq!(qb_3d_mesh_cube(1.0), 1, "freed slot reused");
        assert_eq!(qb_3d_free_mesh(0), ERR_HANDLE);
        assert_eq!(qb_3d_free_mesh(99), ERR_HANDLE);
    }

    #[test]
    fn mesh_builder() {
        let _g = fresh();
        assert_eq!(qb_3d_mesh_vertex(0.0, 0.0, 0.0, 0.0, 0.0), ERR_ILLEGAL, "no _MESHBEGIN");
        assert_eq!(qb_3d_mesh_end(), -1);
        qb_3d_mesh_begin();
        assert_eq!(qb_3d_mesh_end(), -1, "no triangles");
        qb_3d_mesh_begin();
        for v in [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]] {
            assert_eq!(qb_3d_mesh_vertex(v[0], v[1], 0.0, v[0], v[1]), 0);
        }
        assert_eq!(qb_3d_mesh_triangle(0, 1, 3), ERR_ILLEGAL, "vertex 3 not given");
        assert_eq!(qb_3d_mesh_triangle(0, 1, 2), 0);
        let m = qb_3d_mesh_end();
        assert_eq!(m, 1);
        assert_eq!(qb_3d_mesh_triangle(0, 1, 2), ERR_ILLEGAL, "builder closed");
        let s = state();
        let Some(Some(mesh)) = s.meshes.first() else { panic!() };
        assert_eq!(mesh.shape, Shape::Built { pos: vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], uv: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]], idx: vec![0, 1, 2] });
    }

    #[test]
    fn draw_queue_moves_to_the_shown_layer_at_display() {
        let _g = fresh();
        let m = qb_3d_mesh_cube(1.0);
        assert_eq!(qb_3d_draw(m, 1.0, 2.0, 3.0, 0.0, 90.0, 0.0, 1.0, 1.0, 1.0), 0);
        assert_eq!(qb_3d_draw(m, 0.0, 0.0, f32::NAN, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0), ERR_ILLEGAL);
        assert_eq!(queued().len(), 1);
        frame_done();
        assert_eq!((queued().len(), shown().len()), (0, 1));
        assert_eq!(shown()[0].t[..3], [1.0, 2.0, 3.0]);
        // a frame without draws clears the layer
        frame_done();
        assert!(shown().is_empty());
        // _CLEAR3D drops this frame's draws
        qb_3d_draw(m, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        qb_3d_clear(0, 0);
        assert!(queued().is_empty());
        // freeing a mesh drops its draws
        qb_3d_draw(m, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        frame_done();
        qb_3d_free_mesh(m);
        assert!(shown().is_empty());
    }

    #[test]
    fn camera_and_lights() {
        let _g = fresh();
        assert_eq!(qb_3d_camera(0.0, 0.0, 5.0, 0.0, 0.0, 5.0, 60.0, 0), ERR_ILLEGAL, "eye = target");
        assert_eq!(qb_3d_camera(0.0, 0.0, 5.0, 0.0, 0.0, 0.0, 180.0, 0), ERR_ILLEGAL);
        assert_eq!(qb_3d_camera(0.0, 0.0, 5.0, 0.0, 0.0, 0.0, 0.0, 1), ERR_ILLEGAL);
        assert_eq!(qb_3d_camera(1.0, 2.0, 3.0, 0.0, 0.0, 0.0, 45.0, 0), 0);
        assert_eq!(qb_3d_camera_range(1.0, 0.5), ERR_ILLEGAL);
        assert_eq!(qb_3d_camera_range(0.5, 50.0), 0);
        let c = camera();
        assert_eq!((c.eye, c.fov_or_height, c.ortho, c.near, c.far), ([1.0, 2.0, 3.0], 45.0, false, 0.5, 50.0));
        assert_eq!(qb_3d_light(3, 0.0, -1.0, 0.0, 0xFFFF_FFFF, 1.0, 0.0), -1, "bad kind");
        assert_eq!(qb_3d_light(1, 0.0, 0.0, 0.0, 0xFFFF_FFFF, 1.0, 0.0), -1, "no direction");
        let l = qb_3d_light(2, 1.0, 2.0, 3.0, 0xFF80_4020, 2.0, 5.0);
        assert_eq!(l, 1);
        let got = light(l).unwrap();
        assert_eq!((got.kind, got.v, got.intensity, got.range, got.on), (2, [1.0, 2.0, 3.0], 2.0, 5.0, true));
        assert!((got.color[0] - 128.0 / 255.0).abs() < 1e-6);
        assert_eq!(qb_3d_light_on(l, 0), 0);
        assert!(!light(l).unwrap().on);
        assert_eq!(qb_3d_free_light(l), 0);
        assert_eq!(qb_3d_light_on(l, 1), ERR_HANDLE);
        assert_eq!(qb_3d_ambient(0xFFFF_FFFF, -1.0), ERR_ILLEGAL);
    }

    #[test]
    fn load_model_rejects_non_gltf() {
        let _g = fresh();
        let dir = std::env::temp_dir().join(format!("qb3d-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let bad = dir.join("x.txt");
        std::fs::write(&bad, b"hello").unwrap();
        let p = bad.to_string_lossy().into_owned();
        assert_eq!(unsafe { qb_3d_load_model(p.as_ptr(), p.len() as i32) }, -1);
        let missing = dir.join("missing.glb").to_string_lossy().into_owned();
        assert_eq!(unsafe { qb_3d_load_model(missing.as_ptr(), missing.len() as i32) }, -1);
        let _ = std::fs::remove_file(&bad);
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn display_draws_a_lit_cube_over_the_frame() {
        let _g = fresh();
        if qb_3d_available() == 0 {
            eprintln!("skipped: no GPU adapter");
            return;
        }
        let m = qb_3d_mesh_cube(2.0);
        assert_eq!(qb_3d_mesh_color(m, 0xFFFF_0000), 0);
        assert_eq!(qb_3d_camera(0.0, 0.0, 5.0, 0.0, 0.0, 0.0, 4.0, 1), 0);
        assert_eq!(qb_3d_ambient(0xFFFF_FFFF, 0.0), 0);
        assert_eq!(qb_3d_light(1, 0.0, 0.0, -1.0, 0xFFFF_FFFF, 1.0, 0.0), 1);
        assert_eq!(qb_3d_draw(m, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0), 0);
        // nothing shown before _DISPLAY in _DISPLAY mode
        let mut frame = vec![0xFF00_00FFu32; 32 * 32];
        frame_done();
        compose(&mut frame, 32, 32);
        let near = |a: u32, b: u32| (0..4).all(|k| ((a >> (k * 8)) & 255).abs_diff((b >> (k * 8)) & 255) <= 2);
        assert!(near(frame[16 * 32 + 16], 0xFFFF_0000), "{:08x}", frame[16 * 32 + 16]);
        assert_eq!(frame[0], 0xFF00_00FF, "outside the cube the frame shows through");
        // a capture record gets the same composition
        let mut rec = vec![0u8; 24];
        for _ in 0..32 * 32 {
            rec.extend(0xFF00_00FFu32.to_le_bytes());
        }
        apply_to_record(&mut rec, 32, 32, 4);
        let at = |i: usize| u32::from_le_bytes(rec[24 + i * 4..28 + i * 4].try_into().unwrap());
        assert!(near(at(16 * 32 + 16), 0xFFFF_0000));
        assert_eq!(at(0), 0xFF00_00FF);
        // a textured mesh built vertex by vertex covering the view
        reset();
        qb_3d_mesh_begin();
        for (x, y, u, v) in [(-2.0, 2.0, 0.0, 0.0), (2.0, 2.0, 1.0, 0.0), (2.0, -2.0, 1.0, 1.0), (-2.0, -2.0, 0.0, 1.0)] {
            qb_3d_mesh_vertex(x, y, 0.0, u, v);
        }
        qb_3d_mesh_triangle(0, 2, 1);
        qb_3d_mesh_triangle(0, 3, 2);
        let q = qb_3d_mesh_end();
        let tex = [0xFF00_FF00u32; 4];
        assert_eq!(unsafe { qb_3d_mesh_texture(q, tex.as_ptr(), 2, 2) }, 0);
        qb_3d_camera(0.0, 0.0, 5.0, 0.0, 0.0, 0.0, 4.0, 1);
        qb_3d_ambient(0xFFFF_FFFF, 1.0);
        qb_3d_draw(q, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        let mut frame = vec![0u32; 16 * 16];
        unsafe { qb_3d_present(frame.as_mut_ptr(), 16, 16) }; // _AUTODISPLAY: the queue is shown
        assert!(frame.iter().all(|&p| near(p, 0xFF00_FF00)), "{:08x}", frame[0]);
    }

    fn shader(src: &str) -> i32 {
        unsafe { qb_3d_shader(src.as_ptr(), src.len() as i32) }
    }

    #[test]
    fn shader_handles_params_and_mesh_assignment() {
        let _g = fresh();
        let sh = shader("fn shade(i: ShadeIn) -> vec4<f32> { return vec4<f32>(i.params[0].xyz, 1.0); }");
        assert_eq!(sh, 1);
        assert_eq!(shader_error(), "");
        assert_eq!(qb_3d_shader_param(sh, 16, 1.0), ERR_ILLEGAL);
        assert_eq!(qb_3d_shader_param(sh, 0, 0.5), 0);
        assert_eq!(qb_3d_shader_param(9, 0, 0.5), ERR_HANDLE);
        let m = qb_3d_mesh_cube(1.0);
        assert_eq!(qb_3d_mesh_shader(m, 7), ERR_HANDLE);
        assert_eq!(qb_3d_mesh_shader(m, sh), 0);
        assert_eq!(mesh_shader(m), Some(sh));
        assert_eq!(qb_3d_free_shader(sh), 0);
        assert_eq!(mesh_shader(m), Some(0), "back to the lit default");
        assert_eq!(qb_3d_free_shader(sh), ERR_HANDLE);
        assert_eq!(qb_3d_mesh_shader(m, 0), 0);
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn bad_shader_reports_the_line() {
        let _g = fresh();
        assert_eq!(shader("fn shade(i: ShadeIn) -> vec4<f32> {\n  return vec4<f32>(oops);\n}"), -1);
        let e = shader_error();
        assert!(e.starts_with("line 2"), "{e}");
        let mut buf = [0u8; 8];
        let n = unsafe { qb_3d_shader_error(buf.as_mut_ptr(), 8) };
        assert_eq!((n as usize, &buf[..]), (e.len(), &e.as_bytes()[..8]));
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn shaded_mesh_in_the_layer() {
        let _g = fresh();
        if qb_3d_available() == 0 {
            eprintln!("skipped: no GPU adapter");
            return;
        }
        let sh = shader("fn shade(i: ShadeIn) -> vec4<f32> { return vec4<f32>(i.params[0].xyz, 1.0); }");
        for (k, v) in [0.0f32, 1.0, 0.0].iter().enumerate() {
            qb_3d_shader_param(sh, k as i32, *v);
        }
        let m = qb_3d_mesh_cube(2.0);
        qb_3d_mesh_shader(m, sh);
        qb_3d_camera(0.0, 0.0, 5.0, 0.0, 0.0, 0.0, 4.0, 1);
        qb_3d_draw(m, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        frame_done();
        let mut frame = vec![0xFF00_0000u32; 16 * 16];
        compose(&mut frame, 16, 16);
        assert_eq!(frame[8 * 16 + 8] & 0x00FF_FFFF, 0x0000_FF00);
        // a new parameter value shows on the next composition
        qb_3d_shader_param(sh, 2, 1.0);
        let mut frame = vec![0xFF00_0000u32; 16 * 16];
        compose(&mut frame, 16, 16);
        assert_eq!(frame[8 * 16 + 8] & 0x00FF_FFFF, 0x0000_FFFF);
    }
}