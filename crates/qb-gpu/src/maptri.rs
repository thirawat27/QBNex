//! 3D `_MAPTRIANGLE` on the GPU (R15 G3).
//!
//! `_MAPTRIANGLE (sx1,sy1)-(sx2,sy2)-(sx3,sy3), src TO (dx1,dy1,dz1)-(...)-(...), dst`
//! with z coordinates maps a triangle of a hardware image onto the display (or
//! another hardware image) in eye space: camera at the origin looking down -Z,
//! `gluPerspective(fov, w/h, 0.1, 10000)` where `fov` is 90 degrees on the
//! shorter side, exactly as the legacy `set_view(VIEW_MODE__3D)` and the
//! software path (`qb-runtime` `raster::tri3d`) do. This module draws the same
//! triangles with a real depth buffer:
//!
//! - texture coordinates in source pixels (texel centres at +0.5), repeated
//!   outside the image like legacy `GL_REPEAT` ([`Tri3d::wrap`]; `false`
//!   clamps to the edge instead);
//! - `_SMOOTH` / `_SMOOTHSHRUNK` / `_SMOOTHSTRETCHED` choose bilinear filtering
//!   for both / minification / magnification ([`Smooth`]);
//! - `_CLOCKWISE` / `_ANTICLOCKWISE` keep only triangles with that winding on
//!   screen ([`Cull`]);
//! - `_DEPTHBUFFER ON / OFF / LOCK` ([`DepthMode`]): test and write, neither,
//!   test only; the buffer persists until [`DepthBuffer::clear`]
//!   (`_DEPTHBUFFER _CLEAR`);
//! - alpha blending `glBlendFuncSeparate(SRC_ALPHA, ONE_MINUS_SRC_ALPHA, ONE, ONE)`
//!   unless the source has `_DONTBLEND`.
//!
//! Near-plane clipping is the GPU's (z = -0.1); unlike the software path the
//! far plane (z = -10000) also clips, as it did with legacy OpenGL.

use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use wgpu::util::DeviceExt;

use crate::Gpu;

/// Depth buffer mode of the destination (`DEPTHBUFFER_MODE__*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DepthMode {
    /// No test, no write.
    Off,
    /// Nearer or equal passes; the depth is written.
    On,
    /// Nearer or equal passes; the buffer is not changed.
    Locked,
}

impl DepthMode {
    /// From `DEPTHBUFFER_MODE__OFF/ON/LOCKED` (0, 1, 2).
    pub fn from_legacy(v: i32) -> DepthMode {
        match v {
            1 => DepthMode::On,
            2 => DepthMode::Locked,
            _ => DepthMode::Off,
        }
    }
}

/// Winding filter (`CULL_MODE__*`), judged on screen (y down).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cull {
    None,
    /// `_CLOCKWISE`: draw clockwise triangles only.
    ClockwiseOnly,
    /// `_ANTICLOCKWISE`: draw anticlockwise triangles only.
    AntiClockwiseOnly,
}

impl Cull {
    /// From `CULL_MODE__NONE/CLOCKWISE_ONLY/ANTICLOCKWISE_ONLY` (0, 1, 2).
    pub fn from_legacy(v: i32) -> Cull {
        match v {
            1 => Cull::ClockwiseOnly,
            2 => Cull::AntiClockwiseOnly,
            _ => Cull::None,
        }
    }
}

/// Texture filtering (`_SMOOTH*` options).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Smooth {
    Off,
    /// `_SMOOTH`: bilinear when shrinking and stretching.
    Both,
    /// `_SMOOTHSHRUNK`: bilinear when shrinking only.
    Shrunk,
    /// `_SMOOTHSTRETCHED`: bilinear when stretching only.
    Stretched,
}

impl Smooth {
    /// From the `smooth` value of the hardware command (0 none, 1 `_SMOOTH`,
    /// 2 `_SMOOTHSHRUNK`, 3 `_SMOOTHSTRETCHED`).
    pub fn from_legacy(v: i32) -> Smooth {
        match v {
            1 => Smooth::Both,
            2 => Smooth::Shrunk,
            3 => Smooth::Stretched,
            _ => Smooth::Off,
        }
    }

    fn filters(self) -> (wgpu::FilterMode, wgpu::FilterMode) {
        use wgpu::FilterMode::{Linear, Nearest};
        // (min, mag)
        match self {
            Smooth::Off => (Nearest, Nearest),
            Smooth::Both => (Linear, Linear),
            Smooth::Shrunk => (Linear, Nearest),
            Smooth::Stretched => (Nearest, Linear),
        }
    }

    fn index(self) -> usize {
        match self {
            Smooth::Off => 0,
            Smooth::Both => 1,
            Smooth::Shrunk => 2,
            Smooth::Stretched => 3,
        }
    }
}

/// A source image (hardware image) uploaded as a texture.
pub struct SourceImage {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

/// One 3D `_MAPTRIANGLE`.
#[derive(Clone, Copy)]
pub struct Tri3d<'a> {
    pub src: &'a SourceImage,
    /// Destination vertices in eye space (x, y, z), z negative in front of the camera.
    pub d: [[f32; 3]; 3],
    /// Source vertices in source pixels.
    pub s: [[f32; 2]; 3],
    pub use_alpha: bool,
    pub smooth: Smooth,
    /// Repeat texture coordinates outside the source (legacy); `false` clamps.
    pub wrap: bool,
    pub cull: Cull,
    pub depth: DepthMode,
}

/// The depth buffer of a destination. Starts cleared; [`DepthBuffer::clear`]
/// clears it again at the next draw.
pub struct DepthBuffer {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
    pending_clear: bool,
}

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

impl DepthBuffer {
    /// Sides above the device limit are clamped like [crate::Target::offscreen].
    pub fn new(gpu: &Gpu, width: u32, height: u32) -> DepthBuffer {
        let (width, height) = crate::fit_size(width, height, gpu.max_texture_size());
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("qb-gpu maptriangle depth"),
            size: wgpu::Extent3d { width: width.max(1), height: height.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        DepthBuffer { _texture: texture, view, width, height, pending_clear: true }
    }

    /// `_DEPTHBUFFER _CLEAR`: everything is far again.
    pub fn clear(&mut self) {
        self.pending_clear = true;
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vert {
    pos: [f32; 3],
    uv: [f32; 2],
}

const SHADER: &str = r#"
struct U { proj: mat4x4<f32> };
@group(0) @binding(0) var<uniform> u: U;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var smp: sampler;

struct Out {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@location(0) p: vec3<f32>, @location(1) uv: vec2<f32>) -> Out {
    var o: Out;
    o.pos = u.proj * vec4<f32>(p, 1.0);
    o.uv = uv;
    return o;
}

@fragment
fn fs(o: Out) -> @location(0) vec4<f32> {
    return textureSample(tex, smp, o.uv);
}
"#;

const NEAR: f32 = 0.1;
const FAR: f32 = 10000.0;

/// The legacy 3D projection for a `width`×`height` destination.
pub fn projection(width: u32, height: u32) -> Mat4 {
    let (w, h) = (width.max(1) as f32, height.max(1) as f32);
    let fov = if width > height { 90.0 * h / w } else { 90.0f32 };
    glam::camera::rh::proj::directx::perspective(fov.to_radians(), w / h, NEAR, FAR)
}

type PipelineKey = (bool, Cull, DepthMode);

/// Renderer for 3D `_MAPTRIANGLE`s onto one colour format.
pub struct MapTriangle3d {
    module: wgpu::ShaderModule,
    pipeline_layout: wgpu::PipelineLayout,
    tex_layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
    uniform_group: wgpu::BindGroup,
    /// [smooth index][wrap]
    samplers: Vec<wgpu::Sampler>,
    pipelines: HashMap<PipelineKey, wgpu::RenderPipeline>,
    format: wgpu::TextureFormat,
}

impl MapTriangle3d {
    pub fn new(gpu: &Gpu, target_format: wgpu::TextureFormat) -> MapTriangle3d {
        let d = &gpu.device;
        let module = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("qb-gpu maptriangle"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let uniform_layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qb-gpu maptriangle uniform"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let tex_layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qb-gpu maptriangle texture"),
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
            label: Some("qb-gpu maptriangle"),
            bind_group_layouts: &[Some(&uniform_layout), Some(&tex_layout)],
            immediate_size: 0,
        });
        let uniform = d.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qb-gpu maptriangle uniform"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_group = d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("qb-gpu maptriangle uniform"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniform.as_entire_binding() }],
        });
        let mut samplers = Vec::with_capacity(8);
        for smooth in [Smooth::Off, Smooth::Both, Smooth::Shrunk, Smooth::Stretched] {
            for wrap in [false, true] {
                let (min, mag) = smooth.filters();
                let mode = if wrap { wgpu::AddressMode::Repeat } else { wgpu::AddressMode::ClampToEdge };
                samplers.push(d.create_sampler(&wgpu::SamplerDescriptor {
                    label: Some("qb-gpu maptriangle"),
                    address_mode_u: mode,
                    address_mode_v: mode,
                    mag_filter: mag,
                    min_filter: min,
                    ..Default::default()
                }));
            }
        }
        MapTriangle3d {
            module,
            pipeline_layout,
            tex_layout,
            uniform,
            uniform_group,
            samplers,
            pipelines: HashMap::new(),
            format: target_format,
        }
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Uploads a source image (`0xAARRGGBB` pixels, `width`×`height`).
    /// Short data is padded; sides above the device limit are scaled down (texture
    /// coordinates stay in the original pixels).
    pub fn source(&self, gpu: &Gpu, pixels: &[u32], width: u32, height: u32) -> SourceImage {
        let (orig_w, orig_h) = (width, height);
        let (texels, width, height) = crate::fit_texture(pixels, width, height, gpu.max_texture_size());
        let pixels = &texels[..];
        let size = wgpu::Extent3d { width: width.max(1), height: height.max(1), depth_or_array_layers: 1 };
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("qb-gpu maptriangle source"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        if width > 0 && height > 0 {
            gpu.queue.write_texture(
                wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                bytemuck::cast_slice(pixels),
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(width * 4), rows_per_image: Some(height) },
                size,
            );
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        SourceImage { _texture: texture, view, width: orig_w, height: orig_h }
    }

    fn pipeline(&mut self, gpu: &Gpu, key: PipelineKey) -> &wgpu::RenderPipeline {
        let (use_alpha, cull, depth) = key;
        let (module, layout, format) = (&self.module, &self.pipeline_layout, self.format);
        self.pipelines.entry(key).or_insert_with(|| {
            let blend = use_alpha.then_some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            });
            // On screen (y down) a clockwise triangle stays clockwise in framebuffer
            // coordinates; keep the wanted winding as the front face and cull the back.
            let (front_face, cull_mode) = match cull {
                Cull::None => (wgpu::FrontFace::Ccw, None),
                Cull::ClockwiseOnly => (wgpu::FrontFace::Cw, Some(wgpu::Face::Back)),
                Cull::AntiClockwiseOnly => (wgpu::FrontFace::Ccw, Some(wgpu::Face::Back)),
            };
            let (compare, write) = match depth {
                DepthMode::Off => (wgpu::CompareFunction::Always, false),
                DepthMode::On => (wgpu::CompareFunction::LessEqual, true),
                DepthMode::Locked => (wgpu::CompareFunction::LessEqual, false),
            };
            gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("qb-gpu maptriangle"),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vert>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2],
                    })],
                },
                fragment: Some(wgpu::FragmentState {
                    module,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask: wgpu::ColorWrites::ALL })],
                }),
                primitive: wgpu::PrimitiveState { front_face, cull_mode, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(write),
                    depth_compare: Some(compare),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        })
    }

    /// Draws `tris` in order onto `color` (a `width`×`height` view of this
    /// renderer's format) with `depth`. `clear`: clear the colour first,
    /// otherwise draw over what is there.
    pub fn draw(&mut self, gpu: &Gpu, color: &wgpu::TextureView, depth: &mut DepthBuffer, clear: Option<wgpu::Color>, tris: &[Tri3d]) {
        let (w, h) = (depth.width, depth.height);
        let proj = projection(w, h);
        gpu.queue.write_buffer(&self.uniform, 0, bytemuck::cast_slice(&proj.to_cols_array()));
        let mut verts = Vec::with_capacity(tris.len() * 3);
        for t in tris {
            let (sw, sh) = (t.src.width.max(1) as f32, t.src.height.max(1) as f32);
            for i in 0..3 {
                verts.push(Vert { pos: t.d[i], uv: [(t.s[i][0] + 0.5) / sw, (t.s[i][1] + 0.5) / sh] });
            }
        }
        for t in tris {
            self.pipeline(gpu, (t.use_alpha, t.cull, t.depth));
        }
        let groups: Vec<wgpu::BindGroup> = tris
            .iter()
            .map(|t| {
                let sampler = &self.samplers[t.smooth.index() * 2 + usize::from(t.wrap)];
                gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("qb-gpu maptriangle texture"),
                    layout: &self.tex_layout,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&t.src.view) },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
                    ],
                })
            })
            .collect();
        let vbuf = (!verts.is_empty()).then(|| {
            gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("qb-gpu maptriangle vertices"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            })
        });
        let depth_load = if std::mem::take(&mut depth.pending_clear) { wgpu::LoadOp::Clear(1.0) } else { wgpu::LoadOp::Load };
        let mut enc = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("qb-gpu maptriangle") });
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("qb-gpu maptriangle"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: clear.map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    depth_ops: Some(wgpu::Operations { load: depth_load, store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let Some(vbuf) = &vbuf {
                pass.set_bind_group(0, &self.uniform_group, &[]);
                pass.set_vertex_buffer(0, vbuf.slice(..));
                for (i, t) in tris.iter().enumerate() {
                    let p = &self.pipelines[&(t.use_alpha, t.cull, t.depth)];
                    pass.set_pipeline(p);
                    pass.set_bind_group(1, &groups[i], &[]);
                    let v = (i * 3) as u32;
                    pass.draw(v..v + 3, 0..1);
                }
            }
        }
        gpu.queue.submit([enc.finish()]);
    }
}
