//! `qb-gpu` — GPU backend for QBNex, on wgpu (Vulkan / DX12 /
//! Metal / WebGPU). Pure Rust.
//!
//! - [`Gpu`]: instance, adapter, device and queue. [`Gpu::headless`] picks a
//!   hardware adapter or the platform's software one (WARP, llvmpipe, ...)
//!   and returns `None` when there is no adapter at all, so tests and
//!   tools can skip cleanly.
//! - [`Target`]: where frames go — an offscreen texture (readable back with
//!   [`Target::read_rgba`]) or a winit window surface.
//! - [`blit::Blitter`]: presents the software framebuffer the runtime draws
//!   (`u32` pixels `0xAARRGGBB`, i.e. BGRA bytes) scaled to the target with
//!   the aspect ratio kept (letterbox), nearest filtering.
//! - [`three`]: groundwork for 3D (`_MAPTRIANGLE` 3D, a future `_MESH`
//!   API): vertex/index buffers, perspective/orthographic camera, depth
//!   buffer and a textured-triangle WGSL pipeline.
//!
//! The CPU renderer stays the reference: the GPU path presents its pixels
//! unchanged (offscreen targets use a non-sRGB format, so values round-trip).

pub mod blit;
pub mod maptri;
pub mod scene;
pub mod three;

use std::sync::Arc;

pub use glam;
pub use wgpu;

/// GPU context.
pub struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

#[derive(Debug)]
pub struct GpuError(pub String);

impl std::fmt::Display for GpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "qb-gpu: {}", self.0)
    }
}

impl std::error::Error for GpuError {}

fn instance() -> wgpu::Instance {
    wgpu::Instance::new(wgpu::InstanceDescriptor { backends: wgpu::Backends::PRIMARY, ..wgpu::InstanceDescriptor::new_without_display_handle_from_env() })
}

/// The process's one instance and adapter, made on first use.
///
/// Creating a Vulkan instance or enumerating adapters makes the Vulkan loader
/// load and unload driver libraries; doing it while another thread uses a
/// device crashed that thread inside the loader (seen with Mesa's lavapipe).
/// So it happens once, and every [`Gpu`] gets its own device on the shared
/// adapter. `QB_GPU_FALLBACK=1` (read then) picks the software adapter.
fn shared() -> Option<(wgpu::Instance, wgpu::Adapter)> {
    static SHARED: std::sync::OnceLock<Option<(wgpu::Instance, wgpu::Adapter)>> = std::sync::OnceLock::new();
    SHARED
        .get_or_init(|| {
            let instance = instance();
            let pick = |fallback: bool| {
                pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::default(),
                    force_fallback_adapter: fallback,
                    compatible_surface: None,
                    apply_limit_buckets: false,
                }))
                .ok()
            };
            let adapter = if std::env::var("QB_GPU_FALLBACK").is_ok_and(|v| v == "1") {
                pick(true)
            } else {
                pick(false).or_else(|| pick(true))
            }?;
            Some((instance, adapter))
        })
        .clone()
}

fn device(adapter: &wgpu::Adapter) -> Result<(wgpu::Device, wgpu::Queue), GpuError> {
    let desc = wgpu::DeviceDescriptor {
        label: Some("qb-gpu"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
        ..Default::default()
    };
    pollster::block_on(adapter.request_device(&desc)).map_err(|e| GpuError(e.to_string()))
}

impl Gpu {
    /// A GPU for offscreen rendering: hardware if there is one, else the
    /// software adapter; `None` when the system has no adapter at all.
    /// `QB_GPU=off` disables the GPU, `QB_GPU_FALLBACK=1` forces the software adapter.
    pub fn headless() -> Option<Gpu> {
        // QB_GPU=off: behave as if there were no adapter.
        if std::env::var("QB_GPU").is_ok_and(|v| v == "off") {
            return None;
        }
        let (instance, adapter) = shared()?;
        let (device, queue) = device(&adapter).ok()?;
        Some(Gpu { instance, adapter, device, queue })
    }

    /// A GPU and a surface for `window`.
    /// `QB_GPU=off` refuses (the caller keeps its software presenter).
    pub fn for_window(window: Arc<winit::window::Window>) -> Result<(Gpu, Target), GpuError> {
        if std::env::var("QB_GPU").is_ok_and(|v| v == "off") {
            return Err(GpuError("disabled by QB_GPU=off".into()));
        }
        let (instance, shared_adapter) = shared().ok_or_else(|| GpuError("no adapter".into()))?;
        let surface = instance.create_surface(window.clone()).map_err(|e| GpuError(e.to_string()))?;
        // the shared adapter when it can present here; asking for another one enumerates again
        let adapter = if shared_adapter.is_surface_supported(&surface) {
            shared_adapter
        } else {
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
                apply_limit_buckets: false,
            }))
            .map_err(|e| GpuError(e.to_string()))?
        };
        let (device, queue) = device(&adapter)?;
        let gpu = Gpu { instance, adapter, device, queue };
        let size = window.inner_size();
        let caps = surface.get_capabilities(&gpu.adapter);
        // Prefer a non-sRGB format: framebuffer values are shown as they are.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .or_else(|| caps.formats.first().copied())
            .ok_or_else(|| GpuError("surface has no format".into()))?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: fit_size(size.width, size.height, gpu.max_texture_size()).0,
            height: fit_size(size.width, size.height, gpu.max_texture_size()).1,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes.first().copied().unwrap_or(wgpu::CompositeAlphaMode::Auto),
            view_formats: vec![],
            color_space: Default::default(),
        };
        surface.configure(&gpu.device, &config);
        Ok((gpu, Target::Window { surface, config, window }))
    }

    /// Largest texture side the device accepts.
    pub fn max_texture_size(&self) -> u32 {
        self.device.limits().max_texture_dimension_2d
    }

    pub fn adapter_name(&self) -> String {
        self.adapter.get_info().name
    }
}

/// Where frames are drawn.
pub enum Target {
    Offscreen { texture: wgpu::Texture, width: u32, height: u32, format: wgpu::TextureFormat },
    Window { surface: wgpu::Surface<'static>, config: wgpu::SurfaceConfiguration, window: Arc<winit::window::Window> },
}

/// Offscreen color format: 8-bit RGBA, not sRGB (values round-trip exactly).
pub const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// A frame being drawn: render into `view`, then [`Frame::present`].
pub struct Frame {
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
    surface: Option<wgpu::SurfaceTexture>,
}

impl Frame {
    pub fn present(self, gpu: &Gpu) {
        if let Some(s) = self.surface {
            gpu.queue.present(s);
        }
    }
}

impl Target {
    pub fn offscreen(gpu: &Gpu, width: u32, height: u32) -> Target {
        Target::offscreen_with_format(gpu, width, height, OFFSCREEN_FORMAT)
    }

    /// An offscreen target of a given 4-byte colour format (`Bgra8Unorm` reads back as runtime pixels).
    /// Sides above the device limit are clamped (keeping the aspect ratio).
    pub fn offscreen_with_format(gpu: &Gpu, width: u32, height: u32, format: wgpu::TextureFormat) -> Target {
        let (width, height) = fit_size(width, height, gpu.max_texture_size());
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("qb-gpu offscreen"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        Target::Offscreen { texture, width, height, format }
    }

    pub fn size(&self) -> (u32, u32) {
        match self {
            Target::Offscreen { width, height, .. } => (*width, *height),
            Target::Window { config, .. } => (config.width, config.height),
        }
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        match self {
            Target::Offscreen { format, .. } => *format,
            Target::Window { config, .. } => config.format,
        }
    }

    /// Follows a window resize.
    pub fn resize(&mut self, gpu: &Gpu, width: u32, height: u32) {
        if let Target::Window { surface, config, .. } = self {
            (config.width, config.height) = fit_size(width, height, gpu.max_texture_size());
            surface.configure(&gpu.device, config);
        }
    }

    /// The next frame, or `None` when the window cannot draw now (minimized, timeout).
    pub fn frame(&mut self, gpu: &Gpu) -> Option<Frame> {
        match self {
            Target::Offscreen { texture, width, height, format } => Some(Frame {
                view: texture.create_view(&wgpu::TextureViewDescriptor::default()),
                width: *width,
                height: *height,
                format: *format,
                surface: None,
            }),
            Target::Window { surface, config, .. } => {
                let st = match surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
                    wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                        surface.configure(&gpu.device, config);
                        return None;
                    }
                    _ => return None,
                };
                Some(Frame {
                    view: st.texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    width: config.width,
                    height: config.height,
                    format: config.format,
                    surface: Some(st),
                })
            }
        }
    }

    /// Reads an offscreen target back as tightly packed rows of its format
    /// (RGBA8 for [Target::offscreen]).
    pub fn read_rgba(&self, gpu: &Gpu) -> Option<Vec<u8>> {
        let Target::Offscreen { texture, width, height, .. } = self else { return None };
        let (w, h) = (*width, *height);
        let row = w * 4;
        let padded = row.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qb-gpu readback"),
            size: u64::from(padded) * u64::from(h),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(padded), rows_per_image: Some(h) },
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        gpu.queue.submit([enc.finish()]);
        let slice = buf.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        gpu.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).ok()?;
        let data = slice.get_mapped_range().ok()?;
        let mut out = Vec::with_capacity((row * h) as usize);
        for y in 0..h as usize {
            let s = y * padded as usize;
            out.extend_from_slice(&data[s..s + row as usize]);
        }
        drop(data);
        buf.unmap();
        Some(out)
    }

    /// Reads a `Bgra8Unorm` offscreen target back as runtime pixels (`0xAARRGGBB`).
    pub fn read_pixels(&self, gpu: &Gpu) -> Option<Vec<u32>> {
        let bytes = self.read_rgba(gpu)?;
        Some(bytes.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect())
    }
}

/// Size that fits `max` on both sides keeping the aspect ratio (at least 1×1).
pub fn fit_size(w: u32, h: u32, max: u32) -> (u32, u32) {
    let (w, h, max) = (w.max(1), h.max(1), max.max(1));
    if w <= max && h <= max {
        return (w, h);
    }
    let big = w.max(h);
    let s = |v: u32| ((u64::from(v) * u64::from(max) / u64::from(big)) as u32).clamp(1, max);
    (s(w), s(h))
}

/// Texel data ready for a texture of at most `max`×`max`: a `w`×`h` image
/// (`px` shorter than `w*h` is padded with the default value, extra data is
/// ignored) scaled down with nearest sampling when a side exceeds `max`
/// (wgpu panics on textures above `max_texture_dimension_2d`).
pub fn fit_texture<T: Copy + Default>(px: &[T], w: u32, h: u32, max: u32) -> (std::borrow::Cow<'_, [T]>, u32, u32) {
    let (fw, fh) = fit_size(w, h, max);
    let n = w as usize * h as usize;
    if (fw, fh) == (w, h) && px.len() == n {
        return (std::borrow::Cow::Borrowed(px), w, h);
    }
    let get = |x: usize, y: usize| px.get(y * w as usize + x).copied().unwrap_or_default();
    let mut out = Vec::with_capacity(fw as usize * fh as usize);
    for y in 0..fh as usize {
        let sy = (y * h.max(1) as usize / fh as usize).min(h.max(1) as usize - 1);
        for x in 0..fw as usize {
            let sx = (x * w.max(1) as usize / fw as usize).min(w.max(1) as usize - 1);
            out.push(if w == 0 || h == 0 { T::default() } else { get(sx, sy) });
        }
    }
    (std::borrow::Cow::Owned(out), fw, fh)
}