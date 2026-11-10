//! Presents the software framebuffer on the GPU.
//!
//! The runtime draws into `u32` pixels `0xAARRGGBB`; in memory (little
//! endian) that is B, G, R, A — uploaded as-is into a `Bgra8Unorm` texture and
//! drawn into a rectangle of the target, black elsewhere. [`Blitter::draw`]
//! picks the rectangle with [`fit_rect`]; [`Blitter::draw_rect`] takes one
//! from the caller (the runtime passes its own window layout). The rectangle
//! may extend past the target; the part outside is clipped. Filtering is
//! nearest unless [`Blitter::smooth`] is set (bilinear).

use crate::{Frame, Gpu};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    /// Largest scale that keeps the aspect ratio.
    Aspect,
    /// Largest whole-number scale (at least 1), centred.
    Integer,
    /// Fill the whole target (aspect ratio not kept).
    Stretch,
}

/// The rectangle (x, y, w, h) a `fw`×`fh` framebuffer occupies in a `tw`×`th` target.
pub fn fit_rect(fw: u32, fh: u32, tw: u32, th: u32, fit: Fit) -> (u32, u32, u32, u32) {
    if fw == 0 || fh == 0 {
        return (0, 0, 0, 0);
    }
    let (w, h) = match fit {
        Fit::Stretch => (tw, th),
        Fit::Integer => {
            let k = (tw / fw).min(th / fh).max(1);
            (fw * k, fh * k)
        }
        Fit::Aspect => {
            // compare tw/th with fw/fh in integers
            if u64::from(tw) * u64::from(fh) <= u64::from(th) * u64::from(fw) {
                (tw, ((u64::from(tw) * u64::from(fh)) / u64::from(fw)) as u32)
            } else {
                (((u64::from(th) * u64::from(fw)) / u64::from(fh)) as u32, th)
            }
        }
    };
    let (w, h) = (w.min(tw), h.min(th));
    ((tw - w) / 2, (th - h) / 2, w, h)
}

const SHADER: &str = r#"
struct U {
    // destination rectangle in target pixels: x, y, w, h
    rect: vec4<f32>,
    // target size in pixels
    size: vec4<f32>,
};
@group(0) @binding(2) var<uniform> u: U;

struct Out {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Two triangles covering the rectangle; uv 0..1 over it.
@vertex
fn vs(@builtin(vertex_index) i: u32) -> Out {
    var cs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
    );
    let c = cs[i];
    let p = u.rect.xy + c * u.rect.zw;
    var o: Out;
    o.pos = vec4<f32>(p.x / u.size.x * 2.0 - 1.0, 1.0 - p.y / u.size.y * 2.0, 0.0, 1.0);
    o.uv = c;
    return o;
}

@group(0) @binding(0) var fb: texture_2d<f32>;
@group(0) @binding(1) var smp: sampler;

@fragment
fn fs(o: Out) -> @location(0) vec4<f32> {
    let c = textureSample(fb, smp, o.uv);
    return vec4<f32>(c.rgb, 1.0);
}
"#;

struct FbTexture {
    texture: wgpu::Texture,
    /// bind groups with the nearest and the linear sampler
    groups: [wgpu::BindGroup; 2],
    width: u32,
    height: u32,
}

/// Framebuffer presenter. Keeps the texture between frames (recreated when the size changes).
pub struct Blitter {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    samplers: [wgpu::Sampler; 2],
    uniform: wgpu::Buffer,
    tex: Option<FbTexture>,
    format: wgpu::TextureFormat,
    pub fit: Fit,
    /// Bilinear filtering (`_SMOOTH`, smooth full screen) instead of nearest.
    pub smooth: bool,
}

impl Blitter {
    pub fn new(gpu: &Gpu, target_format: wgpu::TextureFormat) -> Blitter {
        let d = &gpu.device;
        let module = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("qb-gpu blit"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qb-gpu blit"),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pl = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("qb-gpu blit"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("qb-gpu blit"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let sampler = |f: wgpu::FilterMode| {
            d.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("qb-gpu blit"),
                mag_filter: f,
                min_filter: f,
                ..Default::default()
            })
        };
        let samplers = [sampler(wgpu::FilterMode::Nearest), sampler(wgpu::FilterMode::Linear)];
        let uniform = d.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qb-gpu blit"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Blitter { pipeline, layout, samplers, uniform, tex: None, format: target_format, fit: Fit::Aspect, smooth: false }
    }

    /// Target format this blitter was made for.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Uploads `pixels` (`width`×`height`, `0xAARRGGBB`) and draws them into
    /// `frame` at the rectangle [`fit_rect`] gives for [`Blitter::fit`].
    pub fn draw(&mut self, gpu: &Gpu, frame: &Frame, pixels: &[u32], width: u32, height: u32) {
        let (x, y, w, h) = fit_rect(width, height, frame.width, frame.height, self.fit);
        self.draw_rect(gpu, frame, pixels, width, height, (x as i32, y as i32, w, h));
    }

    /// Uploads `pixels` and draws them into `frame` at `rect` (x, y, w, h in
    /// target pixels; parts outside the target are clipped). Black elsewhere.
    pub fn draw_rect(&mut self, gpu: &Gpu, frame: &Frame, pixels: &[u32], width: u32, height: u32, rect: (i32, i32, u32, u32)) {
        assert_eq!(pixels.len(), (width * height) as usize, "framebuffer size");
        if width == 0 || height == 0 {
            return;
        }
        if self.tex.as_ref().is_none_or(|t| t.width != width || t.height != height) {
            let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("qb-gpu framebuffer"),
                size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Bgra8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let group = |s: &wgpu::Sampler| {
                gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("qb-gpu blit"),
                    layout: &self.layout,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(s) },
                        wgpu::BindGroupEntry { binding: 2, resource: self.uniform.as_entire_binding() },
                    ],
                })
            };
            let groups = [group(&self.samplers[0]), group(&self.samplers[1])];
            self.tex = Some(FbTexture { texture, groups, width, height });
        }
        let t = self.tex.as_ref().expect("texture");
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &t.texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            bytemuck::cast_slice(pixels),
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(width * 4), rows_per_image: Some(height) },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        let (x, y, w, h) = rect;
        let u: [f32; 8] = [x as f32, y as f32, w as f32, h as f32, frame.width as f32, frame.height as f32, 0.0, 0.0];
        gpu.queue.write_buffer(&self.uniform, 0, bytemuck::cast_slice(&u));
        let mut enc = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("qb-gpu blit") });
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("qb-gpu blit"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if w > 0 && h > 0 && frame.width > 0 && frame.height > 0 {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &t.groups[usize::from(self.smooth)], &[]);
                pass.draw(0..6, 0..1);
            }
        }
        gpu.queue.submit([enc.finish()]);
    }
}
