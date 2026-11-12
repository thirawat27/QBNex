//! Models, lights and a lit scene renderer — the GPU side of the
//! `_MESH` / `_LOADMODEL` / `_CAMERA` / `_LIGHT` / `_DRAW3D` BASIC API
//! (design: `docs/dev/3d-api.md`).
//!
//! - [`Model`]: one or more [`Part`]s (vertex/index buffers, a [`Material`],
//!   a local transform), from glTF 2.0 ([`Model::from_gltf`]), from arrays
//!   ([`Model::from_arrays`]) or primitives ([`Model::cube`], [`Model::plane`]).
//! - [`Lighting`]: ambient colour plus up to [`MAX_LIGHTS`] [`Light`]s,
//!   directional or point.
//! - [`SceneRenderer`]: draws [`Instance`]s with a [`Camera`] and a depth
//!   buffer, Lambert diffuse:
//!   `base × texture × (ambient + Σ max(n·l, 0) × colour × intensity × atten)`,
//!   point attenuation `1 / (1 + (d/range)²)` (`range <= 0`: `1/d²`).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::three::Camera;
use crate::{Frame, Gpu};

/// Lights a [`Lighting`] can hold; more are ignored.
pub const MAX_LIGHTS: usize = 8;

/// A lit-mesh vertex.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct MeshVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

/// RGBA8 texture pixels (not sRGB: values are used as they are).
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Surface description of a part.
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    /// Linear RGBA multiplier.
    pub base_color: [f32; 4],
    /// Index into [`Model::images`].
    pub texture: Option<usize>,
}

impl Default for Material {
    fn default() -> Material {
        Material { base_color: [1.0; 4], texture: None }
    }
}

/// One drawable piece of a model.
pub struct Part {
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    pub index_count: u32,
    pub vertex_count: u32,
    pub material: Material,
    /// Node transform within the model (glTF node tree).
    pub local: Mat4,
}

/// A model: parts, the images they use, and its bounds.
pub struct Model {
    pub parts: Vec<Part>,
    pub images: Vec<Image>,
    textures: Vec<wgpu::TextureView>,
    /// Axis-aligned bounds in model space (after node transforms).
    pub bounds: (Vec3, Vec3),
    /// `_MESHSHADER`: the shader and its 16 `_SHADERPARAM` values (`None`: lit).
    pub shader: Option<(Arc<Shader>, [f32; 16])>,
}

#[derive(Debug)]
pub struct LoadError(pub String);

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "qb-gpu model: {}", self.0)
    }
}

impl std::error::Error for LoadError {}

/// Smooth vertex normals from triangle faces (area weighted).
pub fn compute_normals(pos: &[[f32; 3]], idx: &[u32]) -> Vec<[f32; 3]> {
    let mut n = vec![Vec3::ZERO; pos.len()];
    for t in idx.as_chunks::<3>().0 {
        let (a, b, c) = (t[0] as usize, t[1] as usize, t[2] as usize);
        if a >= pos.len() || b >= pos.len() || c >= pos.len() {
            continue;
        }
        let (pa, pb, pc) = (Vec3::from(pos[a]), Vec3::from(pos[b]), Vec3::from(pos[c]));
        let f = (pb - pa).cross(pc - pa);
        n[a] += f;
        n[b] += f;
        n[c] += f;
    }
    n.into_iter().map(|v| v.normalize_or(Vec3::Y).to_array()).collect()
}

struct PartData {
    verts: Vec<MeshVertex>,
    idx: Vec<u32>,
    material: Material,
    local: Mat4,
}

impl Model {
    fn build(gpu: &Gpu, data: Vec<PartData>, images: Vec<Image>) -> Model {
        let (mut lo, mut hi) = (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
        let parts = data
            .into_iter()
            .map(|p| {
                for v in &p.verts {
                    let w = p.local.transform_point3(Vec3::from(v.pos));
                    lo = lo.min(w);
                    hi = hi.max(w);
                }
                let vbuf = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("qb-gpu model vertices"),
                    contents: bytemuck::cast_slice(&p.verts),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let ibuf = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("qb-gpu model indices"),
                    contents: bytemuck::cast_slice(&p.idx),
                    usage: wgpu::BufferUsages::INDEX,
                });
                Part {
                    vbuf,
                    ibuf,
                    index_count: p.idx.len() as u32,
                    vertex_count: p.verts.len() as u32,
                    material: p.material,
                    local: p.local,
                }
            })
            .collect();
        let textures = images.iter().map(|i| upload(gpu, i)).collect();
        if lo.x > hi.x {
            (lo, hi) = (Vec3::ZERO, Vec3::ZERO);
        }
        Model { parts, images, textures, bounds: (lo, hi), shader: None }
    }

    /// A model from arrays (`_MESH`). `normals` empty: computed. `uvs` empty: zeros.
    /// `indices` empty: consecutive triangles.
    pub fn from_arrays(
        gpu: &Gpu,
        positions: &[[f32; 3]],
        normals: &[[f32; 3]],
        uvs: &[[f32; 2]],
        indices: &[u32],
        material: Material,
        image: Option<Image>,
    ) -> Result<Model, LoadError> {
        let idx: Vec<u32> = if indices.is_empty() { (0..positions.len() as u32).collect() } else { indices.to_vec() };
        if !idx.len().is_multiple_of(3) {
            return Err(LoadError("index count is not a multiple of 3".into()));
        }
        if let Some(&bad) = idx.iter().find(|&&i| i as usize >= positions.len()) {
            return Err(LoadError(format!("index {bad} out of range")));
        }
        if !normals.is_empty() && normals.len() != positions.len() || !uvs.is_empty() && uvs.len() != positions.len() {
            return Err(LoadError("normals/uvs must match the positions".into()));
        }
        let n = if normals.is_empty() { compute_normals(positions, &idx) } else { normals.to_vec() };
        let verts = (0..positions.len())
            .map(|i| MeshVertex { pos: positions[i], normal: n[i], uv: uvs.get(i).copied().unwrap_or([0.0; 2]) })
            .collect();
        let mut material = material;
        let images = match image {
            Some(img) => {
                material.texture = Some(0);
                vec![img]
            }
            None => {
                material.texture = None;
                vec![]
            }
        };
        Ok(Model::build(gpu, vec![PartData { verts, idx, material, local: Mat4::IDENTITY }], images))
    }

    /// `_MESHTEXTURE`: every part uses `image` from now on.
    pub fn set_texture(&mut self, gpu: &Gpu, image: Image) {
        self.textures.push(upload(gpu, &image));
        self.images.push(image);
        let i = self.images.len() - 1;
        for p in &mut self.parts {
            p.material.texture = Some(i);
        }
    }

    /// `_MESHCOLOR`: every part's base colour.
    pub fn set_color(&mut self, color: [f32; 4]) {
        for p in &mut self.parts {
            p.material.base_color = color;
        }
    }

    /// `_MESHCUBE`: a cube of edge `size` centred on the origin, flat faces.
    pub fn cube(gpu: &Gpu, size: f32, material: Material) -> Model {
        let h = size / 2.0;
        let faces: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
            ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
            ([0.0, 0.0, -1.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
            ([1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]),
            ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
            ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]),
            ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ];
        let (mut verts, mut idx) = (Vec::new(), Vec::new());
        for (n, u, v) in faces {
            let (n, u, v) = (Vec3::from(n), Vec3::from(u), Vec3::from(v));
            let base = verts.len() as u32;
            for (su, sv, uv) in [(-1.0, -1.0, [0.0, 1.0]), (1.0, -1.0, [1.0, 1.0]), (1.0, 1.0, [1.0, 0.0]), (-1.0, 1.0, [0.0, 0.0])] {
                let p = (n + u * su + v * sv) * h;
                verts.push(MeshVertex { pos: p.to_array(), normal: n.to_array(), uv });
            }
            idx.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        Model::build(gpu, vec![PartData { verts, idx, material, local: Mat4::IDENTITY }], vec![])
    }

    /// `_MESHPLANE`: a `w`×`d` plane in XZ facing +Y.
    pub fn plane(gpu: &Gpu, w: f32, d: f32, material: Material) -> Model {
        let (x, z) = (w / 2.0, d / 2.0);
        let n = [0.0, 1.0, 0.0];
        let verts = vec![
            MeshVertex { pos: [-x, 0.0, z], normal: n, uv: [0.0, 1.0] },
            MeshVertex { pos: [x, 0.0, z], normal: n, uv: [1.0, 1.0] },
            MeshVertex { pos: [x, 0.0, -z], normal: n, uv: [1.0, 0.0] },
            MeshVertex { pos: [-x, 0.0, -z], normal: n, uv: [0.0, 0.0] },
        ];
        Model::build(gpu, vec![PartData { verts, idx: vec![0, 1, 2, 0, 2, 3], material, local: Mat4::IDENTITY }], vec![])
    }

    /// `_LOADMODEL`: glTF 2.0, `.glb` or `.gltf` with embedded (data URI) buffers.
    pub fn from_gltf(gpu: &Gpu, bytes: &[u8]) -> Result<Model, LoadError> {
        let (doc, buffers, images) = gltf::import_slice(bytes).map_err(|e| LoadError(e.to_string()))?;
        Model::from_gltf_parts(gpu, &doc, &buffers, &images)
    }

    /// `_LOADMODEL` from a file (external buffers/images resolved next to it).
    pub fn from_gltf_file(gpu: &Gpu, path: &std::path::Path) -> Result<Model, LoadError> {
        let (doc, buffers, images) = gltf::import(path).map_err(|e| LoadError(e.to_string()))?;
        Model::from_gltf_parts(gpu, &doc, &buffers, &images)
    }

    fn from_gltf_parts(gpu: &Gpu, doc: &gltf::Document, buffers: &[gltf::buffer::Data], images: &[gltf::image::Data]) -> Result<Model, LoadError> {
        let imgs: Vec<Image> = images.iter().map(to_rgba).collect();
        let mut data = Vec::new();
        for (node, m) in scene_nodes(doc) {
            if let Some(mesh) = node.mesh() {
                for prim in mesh.primitives() {
                    if prim.mode() != gltf::mesh::Mode::Triangles {
                        continue;
                    }
                    let r = prim.reader(|b| buffers.get(b.index()).map(|d| &d.0[..]));
                    let Some(pos) = r.read_positions() else { continue };
                    let pos: Vec<[f32; 3]> = pos.collect();
                    let idx: Vec<u32> = match r.read_indices() {
                        Some(i) => i.into_u32().collect(),
                        None => (0..pos.len() as u32).collect(),
                    };
                    let nrm: Vec<[f32; 3]> = match r.read_normals() {
                        Some(n) => n.collect(),
                        None => compute_normals(&pos, &idx),
                    };
                    let uv: Vec<[f32; 2]> = match r.read_tex_coords(0) {
                        Some(t) => t.into_f32().collect(),
                        None => vec![[0.0; 2]; pos.len()],
                    };
                    if nrm.len() != pos.len() || uv.len() != pos.len() || idx.iter().any(|&i| i as usize >= pos.len()) {
                        return Err(LoadError("inconsistent primitive".into()));
                    }
                    let pbr = prim.material().pbr_metallic_roughness();
                    let texture = pbr.base_color_texture().map(|t| t.texture().source().index()).filter(|&i| i < imgs.len());
                    let material = Material { base_color: pbr.base_color_factor(), texture };
                    let verts = (0..pos.len()).map(|i| MeshVertex { pos: pos[i], normal: nrm[i], uv: uv[i] }).collect();
                    data.push(PartData { verts, idx, material, local: m });
                }
            }
        }
        if data.is_empty() {
            return Err(LoadError("no triangle meshes".into()));
        }
        Ok(Model::build(gpu, data, imgs))
    }
}

/// Nodes of the default scene (or of the file) with their world matrices.
/// A node reached twice, e.g. through a cycle in a malformed file, is visited
/// once (previously the walk looped forever and grew its stack without bound).
fn scene_nodes(doc: &gltf::Document) -> Vec<(gltf::Node<'_>, Mat4)> {
    let scene = doc.default_scene().or_else(|| doc.scenes().next());
    let roots: Vec<gltf::Node> = match scene {
        Some(s) => s.nodes().collect(),
        None => doc.nodes().collect(),
    };
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    let mut stack: Vec<(gltf::Node, Mat4)> = roots.into_iter().map(|n| (n, Mat4::IDENTITY)).collect();
    while let Some((node, parent)) = stack.pop() {
        if !seen.insert(node.index()) {
            continue;
        }
        let m = parent * Mat4::from_cols_array_2d(&node.transform().matrix());
        for c in node.children() {
            stack.push((c, m));
        }
        out.push((node, m));
    }
    out
}

fn to_rgba(d: &gltf::image::Data) -> Image {
    use gltf::image::Format as F;
    let n = (d.width * d.height) as usize;
    let rgba = match d.format {
        F::R8G8B8A8 => d.pixels.clone(),
        F::R8G8B8 => d.pixels.as_chunks::<3>().0.iter().flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        F::R8 => d.pixels.iter().flat_map(|&c| [c, c, c, 255]).collect(),
        F::R8G8 => d.pixels.as_chunks::<2>().0.iter().flat_map(|c| [c[0], c[0], c[0], c[1]]).collect(),
        _ => vec![255; n * 4],
    };
    Image { width: d.width, height: d.height, rgba }
}

fn upload(gpu: &Gpu, img: &Image) -> wgpu::TextureView {
    // short data padded, oversized images scaled down to the device limit
    let (texels, w, h) = crate::fit_texture(img.rgba.as_chunks::<4>().0, img.width, img.height, gpu.max_texture_size());
    let img = &Image { width: w, height: h, rgba: texels.as_flattened().to_vec() };
    let size = wgpu::Extent3d { width: img.width.max(1), height: img.height.max(1), depth_or_array_layers: 1 };
    let tex = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("qb-gpu model texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    if img.rgba.len() == (img.width * img.height * 4) as usize && img.width > 0 {
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &img.rgba,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(img.width * 4), rows_per_image: Some(img.height) },
            size,
        );
    }
    tex.create_view(&wgpu::TextureViewDescriptor::default())
}

/// A light source.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Light {
    /// Light travelling along `direction` (from the light towards the scene).
    Directional { direction: Vec3, color: [f32; 3], intensity: f32 },
    /// Light at `position`; `range` softens the fall-off (see module docs).
    Point { position: Vec3, color: [f32; 3], intensity: f32, range: f32 },
}

/// Ambient light plus the active lights.
#[derive(Debug, Clone, PartialEq)]
pub struct Lighting {
    pub ambient: [f32; 3],
    pub lights: Vec<Light>,
}

impl Default for Lighting {
    /// Dim ambient, one white light from above-front.
    fn default() -> Lighting {
        Lighting {
            ambient: [0.2; 3],
            lights: vec![Light::Directional { direction: Vec3::new(-0.3, -1.0, -0.5), color: [1.0; 3], intensity: 0.8 }],
        }
    }
}

/// One model drawn with a world transform.
#[derive(Clone, Copy)]
pub struct Instance<'a> {
    pub model: &'a Model,
    pub transform: Mat4,
}

/// `_DRAW3D` transform: translate × rotate (degrees, Y then X then Z) × scale.
pub fn transform(pos: Vec3, rot_deg: Vec3, scale: Vec3) -> Mat4 {
    let r = Mat4::from_rotation_y(rot_deg.y.to_radians()) * Mat4::from_rotation_x(rot_deg.x.to_radians()) * Mat4::from_rotation_z(rot_deg.z.to_radians());
    Mat4::from_translation(pos) * r * Mat4::from_scale(scale)
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuLight {
    /// xyz direction or position, w: 0 directional, 1 point
    pos: [f32; 4],
    /// rgb × intensity, w: range
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct FrameUniform {
    view_proj: [f32; 16],
    ambient: [f32; 4],
    /// x: light count
    count: [u32; 4],
    lights: [GpuLight; MAX_LIGHTS],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct DrawUniform {
    model: [f32; 16],
    normal: [f32; 16],
    base: [f32; 4],
    /// `_SHADERPARAM` values 0..15
    params: [f32; 16],
}

/// Shader text before the user's code: bindings, vertex stage, `ShadeIn`,
/// the lighting helper and the default `shade`.
const PRELUDE: &str = r#"
struct Light { pos: vec4<f32>, color: vec4<f32> };
struct FrameU {
    view_proj: mat4x4<f32>,
    ambient: vec4<f32>,
    count: vec4<u32>,
    lights: array<Light, 8>,
};
struct DrawU { model: mat4x4<f32>, normal: mat4x4<f32>, base: vec4<f32>, params: array<vec4<f32>, 4> };

@group(0) @binding(0) var<uniform> fu: FrameU;
@group(1) @binding(0) var<uniform> du: DrawU;
@group(2) @binding(0) var tex: texture_2d<f32>;
@group(2) @binding(1) var smp: sampler;

struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vs(@location(0) pos: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) uv: vec2<f32>) -> Out {
    var o: Out;
    let w = du.model * vec4<f32>(pos, 1.0);
    o.clip = fu.view_proj * w;
    o.world = w.xyz;
    o.normal = (du.normal * vec4<f32>(normal, 0.0)).xyz;
    o.uv = uv;
    return o;
}

/// What a `_SHADER` sees for one pixel.
struct ShadeIn {
    world: vec3<f32>,   // position in the world
    normal: vec3<f32>,  // unit normal, facing the camera
    uv: vec2<f32>,      // texture coordinate
    base: vec4<f32>,    // _MESHCOLOR x texture
    light: vec3<f32>,   // ambient + lights (Lambert) at this pixel
    params: array<vec4<f32>, 4>, // _SHADERPARAM 0..15 (params[i / 4][i % 4])
};

/// The default shading: base colour lit.
fn shade_lit(i: ShadeIn) -> vec4<f32> {
    return vec4<f32>(i.base.rgb * i.light, i.base.a);
}

fn lighting(n: vec3<f32>, world: vec3<f32>) -> vec3<f32> {
    var light = fu.ambient.rgb;
    for (var i = 0u; i < fu.count.x; i = i + 1u) {
        let l = fu.lights[i];
        var dir: vec3<f32>;
        var att = 1.0;
        if (l.pos.w < 0.5) {
            dir = -normalize(l.pos.xyz);
        } else {
            let d = l.pos.xyz - world;
            let dist = length(d);
            dir = d / max(dist, 1e-6);
            if (l.color.w > 0.0) {
                let k = dist / l.color.w;
                att = 1.0 / (1.0 + k * k);
            } else {
                att = 1.0 / max(dist * dist, 1e-6);
            }
        }
        light = light + max(dot(n, dir), 0.0) * l.color.rgb * att;
    }
    return light;
}
"#;

const EPILOGUE: &str = r#"
@fragment
fn fs(o: Out, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    var n = normalize(o.normal);
    if (!front) { n = -n; }
    let base = du.base * textureSample(tex, smp, o.uv);
    let i = ShadeIn(o.world, n, o.uv, base, lighting(n, o.world), du.params);
    return shade(i);
}
"#;

const DEFAULT_SHADE: &str = "fn shade(i: ShadeIn) -> vec4<f32> { return shade_lit(i); }\n";

/// Lines of shader text before the user's first line.
fn prelude_lines() -> u32 {
    PRELUDE.matches('\n').count() as u32
}

static NEXT_SHADER: AtomicU64 = AtomicU64::new(1);

/// A compiled `_SHADER`: user WGSL that defines
/// `fn shade(i: ShadeIn) -> vec4<f32>`, called for every pixel of the meshes
/// that use it. `ShadeIn` gives the world position, the normal, the texture
/// coordinate, the base colour (mesh colour × texture), the light arriving
/// there and the 16 `_SHADERPARAM` values; `shade_lit(i)` is the default
/// look. Checked with naga when compiled, so a bad shader is an error message,
/// never a GPU validation panic.
#[derive(Debug)]
pub struct Shader {
    id: u64,
    source: String,
}

/// Why a shader did not compile: position in the user's text (line 0 when
/// the problem is not at one place of it) and naga's message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderError {
    pub line: u32,
    pub column: u32,
    pub message: String,
}

impl std::fmt::Display for ShaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line > 0 {
            write!(f, "line {}, column {}: {}", self.line, self.column, self.message)
        } else {
            f.write_str(&self.message)
        }
    }
}

impl std::error::Error for ShaderError {}

impl Shader {
    /// Compiles and validates user WGSL. Pure CPU (no GPU needed).
    pub fn compile(user: &str) -> Result<Shader, ShaderError> {
        if !user.contains("fn shade") {
            return Err(ShaderError { line: 0, column: 0, message: "the shader must define fn shade(i: ShadeIn) -> vec4<f32>".into() });
        }
        let source = format!("{PRELUDE}{user}\n{EPILOGUE}");
        let user_lines = user.matches('\n').count() as u32 + 1;
        let place = |loc: Option<naga::SourceLocation>, message: String| {
            let (off, l) = (prelude_lines(), loc.map_or(0, |l| l.line_number));
            if l > off && l <= off + user_lines {
                ShaderError { line: l - off, column: loc.map_or(0, |l| l.line_position), message }
            } else {
                ShaderError {
                    line: 0,
                    column: 0,
                    message: format!("{message} (check that shade takes ShadeIn and returns vec4<f32>, and that no name clashes with ShadeIn, shade_lit, lighting, vs or fs)"),
                }
            }
        };
        let module = naga::front::wgsl::parse_str(&source).map_err(|e| place(e.location(&source), e.message().to_string()))?;
        let mut v = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::empty());
        v.validate(&module).map_err(|e| {
            let mut msg = e.as_inner().to_string();
            let mut src = std::error::Error::source(e.as_inner());
            while let Some(s) = src {
                msg.push_str(": ");
                msg.push_str(&s.to_string());
                src = s.source();
            }
            place(e.location(&source), msg)
        })?;
        Ok(Shader { id: NEXT_SHADER.fetch_add(1, Ordering::Relaxed), source })
    }

    fn default_lit() -> Shader {
        Shader { id: 0, source: format!("{PRELUDE}{DEFAULT_SHADE}\n{EPILOGUE}") }
    }
}

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// Lit renderer for one colour format.
///
/// Draws are batched: all per-draw uniforms of a frame go into one
/// buffer bound with dynamic offsets, texture bind groups are shared by every
/// draw using the same texture, and pipeline/buffer changes are only issued
/// when they differ from the previous draw.
pub struct SceneRenderer {
    format: wgpu::TextureFormat,
    pipeline_layout: wgpu::PipelineLayout,
    frame_layout: wgpu::BindGroupLayout,
    draw_layout: wgpu::BindGroupLayout,
    tex_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    white: wgpu::TextureView,
    default_shader: Shader,
    pipelines: HashMap<u64, wgpu::RenderPipeline>,
    draw_buf: Option<(wgpu::Buffer, u64)>,
    depth: Option<(wgpu::TextureView, u32, u32)>,
    /// Draw calls of the last [`SceneRenderer::render`].
    pub last_draw_calls: u32,
}

impl SceneRenderer {
    pub fn new(gpu: &Gpu, target_format: wgpu::TextureFormat) -> SceneRenderer {
        let d = &gpu.device;
        let frame_layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qb-gpu scene frame"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let draw_layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qb-gpu scene draw"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<DrawUniform>() as u64),
                },
                count: None,
            }],
        });
        let tex_layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qb-gpu scene texture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("qb-gpu scene"),
            bind_group_layouts: &[Some(&frame_layout), Some(&draw_layout), Some(&tex_layout)],
            immediate_size: 0,
        });
        let sampler = d.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("qb-gpu scene"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let white = upload(gpu, &Image { width: 1, height: 1, rgba: vec![255; 4] });
        SceneRenderer {
            format: target_format,
            pipeline_layout,
            frame_layout,
            draw_layout,
            tex_layout,
            sampler,
            white,
            default_shader: Shader::default_lit(),
            pipelines: HashMap::new(),
            draw_buf: None,
            depth: None,
            last_draw_calls: 0,
        }
    }

    fn ensure_pipeline(&mut self, gpu: &Gpu, shader: &Shader) {
        if self.pipelines.contains_key(&shader.id) {
            return;
        }
        let d = &gpu.device;
        let module = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("qb-gpu scene"),
            source: wgpu::ShaderSource::Wgsl(shader.source.as_str().into()),
        });
        let p = d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("qb-gpu scene"),
            layout: Some(&self.pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<MeshVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        self.pipelines.insert(shader.id, p);
    }

    fn depth_view(&mut self, gpu: &Gpu, w: u32, h: u32) {
        if self.depth.as_ref().is_none_or(|d| d.1 != w || d.2 != h) {
            let t = gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("qb-gpu scene depth"),
                size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.depth = Some((t.create_view(&wgpu::TextureViewDescriptor::default()), w, h));
        }
    }

    /// Draws `instances` into `frame` (`clear`: colour to clear to first; depth is always cleared).
    pub fn render(&mut self, gpu: &Gpu, frame: &Frame, camera: &Camera, lighting: &Lighting, clear: Option<wgpu::Color>, instances: &[Instance]) {
        let aspect = frame.width.max(1) as f32 / frame.height.max(1) as f32;
        let mut fu = FrameUniform {
            view_proj: camera.view_proj(aspect).to_cols_array(),
            ambient: [lighting.ambient[0], lighting.ambient[1], lighting.ambient[2], 0.0],
            count: [lighting.lights.len().min(MAX_LIGHTS) as u32, 0, 0, 0],
            lights: [GpuLight::zeroed(); MAX_LIGHTS],
        };
        for (slot, l) in fu.lights.iter_mut().zip(&lighting.lights) {
            *slot = match *l {
                Light::Directional { direction, color, intensity } => GpuLight {
                    pos: [direction.x, direction.y, direction.z, 0.0],
                    color: [color[0] * intensity, color[1] * intensity, color[2] * intensity, 0.0],
                },
                Light::Point { position, color, intensity, range } => GpuLight {
                    pos: [position.x, position.y, position.z, 1.0],
                    color: [color[0] * intensity, color[1] * intensity, color[2] * intensity, range],
                },
            };
        }
        // pipelines of every shader used this frame
        let default = Shader { id: self.default_shader.id, source: String::new() };
        let default_id = default.id;
        let mut shaders: Vec<Arc<Shader>> = Vec::new();
        for inst in instances {
            if let Some((s, _)) = &inst.model.shader
                && !shaders.iter().any(|x| x.id == s.id)
            {
                shaders.push(s.clone());
            }
        }
        let lit = std::mem::replace(&mut self.default_shader, default);
        self.ensure_pipeline(gpu, &lit);
        self.default_shader = lit;
        for s in &shaders {
            self.ensure_pipeline(gpu, s);
        }
        self.depth_view(gpu, frame.width, frame.height);
        // all per-draw uniforms in one buffer (dynamic offsets)
        let d = &gpu.device;
        let size = std::mem::size_of::<DrawUniform>() as u64;
        let align = u64::from(d.limits().min_uniform_buffer_offset_alignment);
        let stride = size.div_ceil(align) * align;
        struct One<'a> {
            part: &'a Part,
            view: &'a wgpu::TextureView,
            shader: u64,
        }
        let mut draws: Vec<One> = Vec::new();
        let mut bytes: Vec<u8> = Vec::new();
        for inst in instances {
            let (shader, params) = match &inst.model.shader {
                Some((s, p)) => (s.id, *p),
                None => (default_id, [0.0; 16]),
            };
            for part in &inst.model.parts {
                let m = inst.transform * part.local;
                let nm = Mat4::from_mat3(Mat3::from_mat4(m).inverse().transpose());
                let du = DrawUniform { model: m.to_cols_array(), normal: nm.to_cols_array(), base: part.material.base_color, params };
                bytes.extend_from_slice(bytemuck::bytes_of(&du));
                bytes.resize(bytes.len() + (stride - size) as usize, 0);
                let view = part.material.texture.and_then(|i| inst.model.textures.get(i)).unwrap_or(&self.white);
                draws.push(One { part, view, shader });
            }
        }
        let fbuf = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("qb-gpu scene frame"),
            contents: bytemuck::bytes_of(&fu),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let fgroup = d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("qb-gpu scene frame"),
            layout: &self.frame_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: fbuf.as_entire_binding() }],
        });
        let need = (bytes.len() as u64).max(stride);
        if self.draw_buf.as_ref().is_none_or(|b| b.1 < need) {
            let cap = need.next_power_of_two();
            let buf = d.create_buffer(&wgpu::BufferDescriptor {
                label: Some("qb-gpu scene draws"),
                size: cap,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.draw_buf = Some((buf, cap));
        }
        let draw_buf = &self.draw_buf.as_ref().expect("draw buffer").0;
        if !bytes.is_empty() {
            gpu.queue.write_buffer(draw_buf, 0, &bytes);
        }
        let dgroup = d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("qb-gpu scene draws"),
            layout: &self.draw_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: draw_buf, offset: 0, size: wgpu::BufferSize::new(size) }),
            }],
        });
        // one texture bind group per distinct texture
        let mut tex_groups: HashMap<*const wgpu::TextureView, wgpu::BindGroup> = HashMap::new();
        for dr in &draws {
            tex_groups.entry(std::ptr::from_ref(dr.view)).or_insert_with(|| {
                d.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("qb-gpu scene texture"),
                    layout: &self.tex_layout,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(dr.view) },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                    ],
                })
            });
        }
        let depth = &self.depth.as_ref().expect("depth").0;
        let mut enc = d.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("qb-gpu scene") });
        let mut calls = 0u32;
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("qb-gpu scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: clear.map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &fgroup, &[]);
            let (mut cur_shader, mut cur_view, mut cur_part) = (u64::MAX, std::ptr::null(), std::ptr::null());
            for (k, dr) in draws.iter().enumerate() {
                if dr.shader != cur_shader {
                    pass.set_pipeline(&self.pipelines[&dr.shader]);
                    cur_shader = dr.shader;
                }
                let v = std::ptr::from_ref(dr.view);
                if v != cur_view {
                    pass.set_bind_group(2, &tex_groups[&v], &[]);
                    cur_view = v;
                }
                let p = std::ptr::from_ref(dr.part);
                if p != cur_part {
                    pass.set_vertex_buffer(0, dr.part.vbuf.slice(..));
                    pass.set_index_buffer(dr.part.ibuf.slice(..), wgpu::IndexFormat::Uint32);
                    cur_part = p;
                }
                pass.set_bind_group(1, &dgroup, &[(k as u64 * stride) as u32]);
                pass.draw_indexed(0..dr.part.index_count, 0, 0..1);
                calls += 1;
            }
        }
        gpu.queue.submit([enc.finish()]);
        self.last_draw_calls = calls;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cycle in the node hierarchy of a malformed file must not hang
    /// the loader.
    #[test]
    fn cyclic_node_hierarchy_terminates() {
        let json = br#"{"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],
            "nodes":[{"children":[1]},{"children":[0,1]}]}"#;
        let Ok(doc) = gltf::Gltf::from_slice_without_validation(json) else { return };
        let nodes = scene_nodes(&doc.document);
        assert_eq!(nodes.len(), 2);
    }
}
