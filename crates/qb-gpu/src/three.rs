//! 3D groundwork: meshes (vertex + index buffers), a camera (perspective or
//! orthographic), a depth buffer and a textured-triangle pipeline. This is
//! what `_MAPTRIANGLE` 3D and a future `_MESH` API build on.
//!
//! Conventions: right-handed world, camera looks along −Z by default, clip
//! depth 0..1 (wgpu), counter-clockwise front faces, no culling by default
//! (QB64's `_MAPTRIANGLE` draws both sides unless `_CLOCKWISE`/`_ANTICLOCKWISE`).

use crate::{Frame, Gpu};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

/// A vertex: position, texture coordinate, colour (multiplied with the texture).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex {
    pub const fn new(pos: [f32; 3], uv: [f32; 2], color: [f32; 4]) -> Vertex {
        Vertex { pos, uv, color }
    }

    const ATTRS: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Projection {
    /// Vertical field of view in radians.
    Perspective { fov_y: f32, near: f32, far: f32 },
    /// Visible height in world units (width follows the aspect ratio).
    Orthographic { height: f32, near: f32, far: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub projection: Projection,
}

impl Camera {
    pub fn perspective(eye: Vec3, target: Vec3, fov_y: f32) -> Camera {
        Camera { eye, target, up: Vec3::Y, projection: Projection::Perspective { fov_y, near: 0.1, far: 1000.0 } }
    }

    pub fn orthographic(eye: Vec3, target: Vec3, height: f32) -> Camera {
        Camera { eye, target, up: Vec3::Y, projection: Projection::Orthographic { height, near: 0.1, far: 1000.0 } }
    }

    /// Projection × view for a target of the given aspect ratio (width / height).
    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let view = glam::camera::rh::view::look_at_mat4(self.eye, self.target, self.up);
        let proj = match self.projection {
            Projection::Perspective { fov_y, near, far } => glam::camera::rh::proj::directx::perspective(fov_y, aspect, near, far),
            Projection::Orthographic { height, near, far } => {
                let (h, w) = (height / 2.0, height * aspect / 2.0);
                glam::camera::rh::proj::directx::orthographic(-w, w, -h, h, near, far)
            }
        };
        proj * view
    }
}

/// Vertex and index buffers.
pub struct Mesh {
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    count: u32,
}

impl Mesh {
    pub fn new(gpu: &Gpu, vertices: &[Vertex], indices: &[u32]) -> Mesh {
        let vbuf = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("qb-gpu vertices"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ibuf = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("qb-gpu indices"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        Mesh { vbuf, ibuf, count: indices.len() as u32 }
    }
}

/// An RGBA8 texture with its sampler binding.
pub struct Texture {
    bind: wgpu::BindGroup,
}

impl Texture {
    /// `rgba`: `width`×`height` pixels, 4 bytes each. `smooth`: linear filtering (`_SMOOTH`).
    /// Short data is padded, sides above the device limit are scaled down.
    pub fn from_rgba(gpu: &Gpu, r: &Renderer3d, width: u32, height: u32, rgba: &[u8], smooth: bool) -> Texture {
        let (texels, width, height) = crate::fit_texture(rgba.as_chunks::<4>().0, width, height, gpu.max_texture_size());
        let rgba = texels.as_flattened();
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let tex = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("qb-gpu texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            rgba,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(width * 4), rows_per_image: Some(height) },
            size,
        );
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let filter = if smooth { wgpu::FilterMode::Linear } else { wgpu::FilterMode::Nearest };
        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("qb-gpu texture"),
            mag_filter: filter,
            min_filter: filter,
            ..Default::default()
        });
        let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("qb-gpu texture"),
            layout: &r.tex_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });
        Texture { bind }
    }

    /// A 1×1 white texture (vertex colours only).
    pub fn white(gpu: &Gpu, r: &Renderer3d) -> Texture {
        Texture::from_rgba(gpu, r, 1, 1, &[255, 255, 255, 255], false)
    }
}

/// One mesh to draw: model matrix and texture.
pub struct Draw<'a> {
    pub mesh: &'a Mesh,
    pub texture: &'a Texture,
    pub model: Mat4,
}

const SHADER: &str = r#"
struct U { mvp: mat4x4<f32> };
@group(0) @binding(0) var<uniform> u: U;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var smp: sampler;

struct Out {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs(@location(0) pos: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) color: vec4<f32>) -> Out {
    var o: Out;
    o.pos = u.mvp * vec4<f32>(pos, 1.0);
    o.uv = uv;
    o.color = color;
    return o;
}

@fragment
fn fs(o: Out) -> @location(0) vec4<f32> {
    return textureSample(tex, smp, o.uv) * o.color;
}
"#;

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// Textured triangles with a depth buffer.
pub struct Renderer3d {
    pipeline: wgpu::RenderPipeline,
    uni_layout: wgpu::BindGroupLayout,
    tex_layout: wgpu::BindGroupLayout,
    depth: Option<(wgpu::TextureView, u32, u32)>,
}

impl Renderer3d {
    pub fn new(gpu: &Gpu, target_format: wgpu::TextureFormat) -> Renderer3d {
        let d = &gpu.device;
        let module = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("qb-gpu 3d"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let uni_layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qb-gpu 3d uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let tex_layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qb-gpu 3d texture"),
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
        let pl = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("qb-gpu 3d"),
            bind_group_layouts: &[Some(&uni_layout), Some(&tex_layout)],
            immediate_size: 0,
        });
        let pipeline = d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("qb-gpu 3d"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[Some(Vertex::layout())],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
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
        Renderer3d { pipeline, uni_layout, tex_layout, depth: None }
    }

    fn depth_view(&mut self, gpu: &Gpu, w: u32, h: u32) -> &wgpu::TextureView {
        if self.depth.as_ref().is_none_or(|d| d.1 != w || d.2 != h) {
            let t = gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("qb-gpu depth"),
                size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.depth = Some((t.create_view(&wgpu::TextureViewDescriptor::default()), w, h));
        }
        &self.depth.as_ref().expect("depth").0
    }

    /// Clears `frame` to `clear` (and the depth buffer) and draws `draws` seen from `camera`.
    pub fn render(&mut self, gpu: &Gpu, frame: &Frame, camera: &Camera, clear: wgpu::Color, draws: &[Draw]) {
        let aspect = frame.width as f32 / frame.height.max(1) as f32;
        let vp = camera.view_proj(aspect);
        let binds: Vec<wgpu::BindGroup> = draws
            .iter()
            .map(|d| {
                let m = vp * d.model;
                let buf = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("qb-gpu mvp"),
                    contents: bytemuck::bytes_of(&m.to_cols_array()),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("qb-gpu mvp"),
                    layout: &self.uni_layout,
                    entries: &[wgpu::BindGroupEntry { binding: 0, resource: buf.as_entire_binding() }],
                })
            })
            .collect();
        let (w, h) = (frame.width, frame.height);
        let depth = self.depth_view(gpu, w, h).clone();
        let mut enc = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("qb-gpu 3d") });
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("qb-gpu 3d"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(clear), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            for (d, b) in draws.iter().zip(&binds) {
                pass.set_bind_group(0, b, &[]);
                pass.set_bind_group(1, &d.texture.bind, &[]);
                pass.set_vertex_buffer(0, d.mesh.vbuf.slice(..));
                pass.set_index_buffer(d.mesh.ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..d.mesh.count, 0, 0..1);
            }
        }
        gpu.queue.submit([enc.finish()]);
    }
}

/// An axis-aligned square in the plane z = `z`, centre (`x`, `y`), side `s`, one colour.
pub fn quad(gpu: &Gpu, x: f32, y: f32, z: f32, s: f32, color: [f32; 4]) -> Mesh {
    let h = s / 2.0;
    let v = [
        Vertex::new([x - h, y - h, z], [0.0, 1.0], color),
        Vertex::new([x + h, y - h, z], [1.0, 1.0], color),
        Vertex::new([x + h, y + h, z], [1.0, 0.0], color),
        Vertex::new([x - h, y + h, z], [0.0, 0.0], color),
    ];
    Mesh::new(gpu, &v, &[0, 1, 2, 0, 2, 3])
}
