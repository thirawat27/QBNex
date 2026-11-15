//! 3D `_MAPTRIANGLE` drawn on the GPU in batches (feature `gpu`).
//!
//! The runner queues each 3D triangle ([`queue`]) instead of rasterizing it
//! at once, and the queue is drawn ([`flush`]) before anything else can look
//! at or change an image: before the next builtin that is not a
//! `_MAPTRIANGLE` or a pure function, when a frame is shown or captured, and
//! at the end of the program. A flush uploads the part of each destination
//! image the queued triangles can cover, draws them with a depth buffer kept
//! on the GPU for that image, and reads that part back. The image in memory
//! then holds what the software renderer (`raster::tri3d`) draws, up to
//! rounding at triangle edges and texel boundaries.
//!
//! [`queue`] returns `false`, and the caller draws in software, without the
//! feature or a hardware adapter (a software one such as WARP is slower than
//! the software renderer), for an image whose pixels `_MEMIMAGE` handed out
//! (they can be read without a builtin call), and after a GPU error. A program
//! that keeps flushing a few triangles at a time (a builtin call between its
//! `_MAPTRIANGLE`s) goes back to software too: a flush costs a round trip.

use std::sync::atomic::{AtomicBool, Ordering};

/// One 3D triangle, with the values `raster::tri3d` takes.
#[derive(Debug, Clone, Copy)]
pub struct Tri {
    /// Destination vertices in eye space.
    pub d: [[f32; 3]; 3],
    /// Source vertices in source pixels.
    pub s: [[f32; 2]; 3],
    pub use_alpha: bool,
    pub smooth: bool,
    /// `raster::CULL_*`
    pub cull: i32,
    /// `raster::DEPTH_*`
    pub depth: i32,
}

static PENDING: AtomicBool = AtomicBool::new(false);

/// Triangles are waiting for [`flush`].
#[inline]
pub fn pending() -> bool {
    PENDING.load(Ordering::Relaxed)
}

/// The destination of [`queue`]: image `key`, `w`×`h` pixels at `px`.
#[derive(Debug, Clone, Copy)]
pub struct Dst {
    pub key: u32,
    pub px: *mut u32,
    pub w: usize,
    pub h: usize,
}

/// The source of [`queue`]: image `key`, `w`×`h` pixels.
#[derive(Debug, Clone, Copy)]
pub struct SrcImg<'a> {
    pub key: u32,
    pub px: &'a [u32],
    pub w: usize,
    pub h: usize,
}

/// Queues triangle `t` from `src` onto `dst`. `false`: not queued, draw it
/// in software.
///
/// # Safety
/// `dst.px` points to `w*h` pixels that stay valid, and that nothing else
/// reads or writes, until the next [`flush`] (the runner flushes before any
/// other builtin and before an image is freed).
pub unsafe fn queue(dst: Dst, src: SrcImg, t: Tri) -> bool {
    #[cfg(feature = "gpu")]
    {
        // SAFETY: the caller's contract, passed on.
        unsafe { imp::queue(dst, src, t) }
    }
    #[cfg(not(feature = "gpu"))]
    {
        let _ = (dst, src, t);
        false
    }
}

/// Draws the queued triangles into their images.
pub fn flush() {
    if pending() {
        #[cfg(feature = "gpu")]
        imp::flush();
    }
}

/// `_DEPTHBUFFER _CLEAR` of image `key` (the queue is flushed first).
pub fn clear_depth(key: u32) {
    flush();
    #[cfg(feature = "gpu")]
    imp::clear_depth(key);
    #[cfg(not(feature = "gpu"))]
    let _ = key;
}

/// Image `key` was freed: its textures and depth buffer go (the index may be reused).
pub fn forget(key: u32) {
    flush();
    #[cfg(feature = "gpu")]
    imp::forget(key);
    #[cfg(not(feature = "gpu"))]
    let _ = key;
}

/// `_MEMIMAGE` of image `key`: from now on its triangles are drawn in
/// software, at once (the program can read the pixels without a builtin).
pub fn expose(key: u32) {
    flush();
    #[cfg(feature = "gpu")]
    imp::expose(key);
    #[cfg(not(feature = "gpu"))]
    let _ = key;
}

#[cfg(feature = "gpu")]
mod imp {
    use super::{Dst, SrcImg, Tri, PENDING};
    use crate::raster;
    use qb_gpu::maptri::{Cull, DepthBuffer, DepthMode, MapTriangle3d, Smooth, SourceImage, Tri3d};
    use qb_gpu::{glam, wgpu, Gpu};
    use std::collections::{HashMap, HashSet};
    use std::sync::atomic::Ordering;
    use std::sync::{Arc, Mutex};

    /// Flushes of fewer triangles than this count as small.
    const SMALL_BATCH: usize = 4;
    /// After this many small flushes in a row the software renderer takes over.
    const SMALL_LIMIT: u32 = 256;
    /// A longer queue is drawn at once (memory bound).
    const MAX_QUEUE: usize = 1 << 16;

    struct DestTex {
        ptr: usize,
        w: u32,
        h: u32,
        tex: wgpu::Texture,
        view: wgpu::TextureView,
        depth: DepthBuffer,
    }

    struct SrcTex {
        w: u32,
        h: u32,
        px: Vec<u32>,
        img: SourceImage,
    }

    struct Queued {
        dst: u32,
        src: u32,
        t: Tri,
    }

    struct Gpu3 {
        /// the process's shared headless device (the `_DRAW3D` layer draws on it too)
        gpu: Arc<Gpu>,
        r: MapTriangle3d,
        dests: HashMap<u32, DestTex>,
        srcs: HashMap<u32, SrcTex>,
        readback: qb_gpu::Readback,
        /// largest texture side (asking wgpu each time costs more than a triangle)
        max: usize,
    }

    /// The images of the last triangle queued in this batch: a mesh sends many
    /// in a row with the same source and destination, which skip the lookups.
    #[derive(Clone, Copy, PartialEq)]
    struct Last {
        dst: u32,
        ptr: usize,
        w: usize,
        h: usize,
        src: u32,
        sw: usize,
        sh: usize,
    }

    #[derive(Default)]
    struct State {
        /// `None` = not tried yet, `Some(None)` = no GPU (or given up)
        gpu: Option<Option<Gpu3>>,
        queue: Vec<Queued>,
        /// source pixels copied in this batch: (width, height, pixels)
        batch_src: HashMap<u32, (u32, u32, Vec<u32>)>,
        /// destinations with queued triangles
        batch_dst: HashSet<u32>,
        exposed: HashSet<u32>,
        small: u32,
        last: Option<Last>,
    }

    static STATE: Mutex<Option<State>> = Mutex::new(None);

    fn with<T>(f: impl FnOnce(&mut State) -> T) -> T {
        let mut g = STATE.lock().unwrap_or_else(|e| e.into_inner());
        f(g.get_or_insert_with(State::default))
    }

    fn verbose() -> bool {
        std::env::var("QB_GPU").is_ok_and(|v| v == "verbose")
    }

    fn gpu3(s: &mut State) -> Option<&mut Gpu3> {
        s.gpu
            .get_or_insert_with(|| {
                let gpu = Gpu::shared_headless()?;
                // a software adapter draws these slower than raster::tri3d does
                if gpu.is_software() {
                    return None;
                }
                let r = MapTriangle3d::new(&gpu, wgpu::TextureFormat::Bgra8Unorm);
                if verbose() {
                    eprintln!("qb-runtime: _MAPTRIANGLE 3D on the GPU ({})", gpu.adapter_name());
                }
                let max = gpu.max_texture_size() as usize;
                Some(Gpu3 { gpu, r, dests: HashMap::new(), srcs: HashMap::new(), readback: qb_gpu::Readback::default(), max })
            })
            .as_mut()
    }

    pub(super) unsafe fn queue(dst: Dst, src: SrcImg, t: Tri) -> bool {
        let (dst_key, w, h, src_key, sw, sh) = (dst.key, dst.w, dst.h, src.key, src.w, src.h);
        let (dst, src) = (dst.px, src.px);
        let last = Last { dst: dst_key, ptr: dst as usize, w, h, src: src_key, sw, sh };
        with(|s| {
            // the same images as the triangle before (another source than the destination)
            if s.last == Some(last) && s.queue.len() < MAX_QUEUE {
                s.queue.push(Queued { dst: dst_key, src: src_key, t });
                return true;
            }
            if s.exposed.contains(&dst_key) || w == 0 || h == 0 || sw == 0 || sh == 0 {
                return false;
            }
            let Some(g) = gpu3(s) else { return false };
            if g.gpu.failed() {
                // a device error, possibly from the 3D layer on the same device:
                // what is queued goes in software, and so does everything after
                flush_locked(s);
                if s.gpu.as_ref().is_some_and(Option::is_some) {
                    if verbose() {
                        eprintln!("qb-runtime: _MAPTRIANGLE 3D falls back to software");
                    }
                    s.gpu = Some(None);
                }
                return false;
            }
            if w > g.max || h > g.max {
                return false;
            }
            // the source is an image with queued triangles: draw them first
            if s.batch_dst.contains(&src_key) || s.queue.len() >= MAX_QUEUE {
                flush_locked(s);
                if s.gpu.as_ref().is_none_or(Option::is_none) {
                    return false;
                }
            }
            s.batch_src.entry(src_key).or_insert_with(|| (sw as u32, sh as u32, src[..sw * sh].to_vec()));
            let g = s.gpu.as_mut().and_then(Option::as_mut).expect("checked above");
            let (w, h) = (w as u32, h as u32);
            match g.dests.get_mut(&dst_key) {
                Some(d) if (d.w, d.h) == (w, h) => d.ptr = dst as usize,
                _ => {
                    let tex = g.gpu.device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("qb-runtime maptriangle destination"),
                        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Bgra8Unorm,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
                        view_formats: &[],
                    });
                    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
                    let depth = DepthBuffer::new(&g.gpu, w, h);
                    g.dests.insert(dst_key, DestTex { ptr: dst as usize, w, h, tex, view, depth });
                }
            }
            s.queue.push(Queued { dst: dst_key, src: src_key, t });
            s.batch_dst.insert(dst_key);
            s.last = (src_key != dst_key).then_some(last);
            PENDING.store(true, Ordering::Relaxed);
            true
        })
    }

    pub(super) fn flush() {
        with(flush_locked);
    }

    pub(super) fn clear_depth(key: u32) {
        with(|s| {
            if let Some(Some(g)) = s.gpu.as_mut()
                && let Some(d) = g.dests.get_mut(&key)
            {
                d.depth.clear();
            }
        });
    }

    pub(super) fn forget(key: u32) {
        with(|s| {
            s.last = None;
            s.exposed.remove(&key);
            if let Some(Some(g)) = s.gpu.as_mut() {
                g.dests.remove(&key);
                g.srcs.remove(&key);
            }
        });
    }

    pub(super) fn expose(key: u32) {
        with(|s| {
            s.last = None;
            s.exposed.insert(key);
        });
    }

    /// Pixel rectangle (x, y, w, h) of `w`×`h` that `t` can cover.
    fn cover(t: &Tri, proj: &glam::Mat4, w: u32, h: u32) -> (u32, u32, u32, u32) {
        let (fw, fh) = (w as f32, h as f32);
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for p in t.d {
            // at or behind the near plane: the clipped triangle can reach anywhere
            if !p.iter().all(|v| v.is_finite()) || p[2] >= -0.1 {
                return (0, 0, w, h);
            }
            let c = *proj * glam::Vec4::new(p[0], p[1], p[2], 1.0);
            let (x, y) = ((c.x / c.w + 1.0) * 0.5 * fw, (1.0 - c.y / c.w) * 0.5 * fh);
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
        let cl = |v: f32, m: f32| v.clamp(0.0, m) as u32;
        let (ax, ay) = (cl((x0 - 2.0).floor(), fw), cl((y0 - 2.0).floor(), fh));
        let (bx, by) = (cl((x1 + 2.0).ceil(), fw), cl((y1 + 2.0).ceil(), fh));
        (ax, ay, bx.saturating_sub(ax), by.saturating_sub(ay))
    }

    fn union(a: (u32, u32, u32, u32), b: (u32, u32, u32, u32)) -> (u32, u32, u32, u32) {
        if a.2 == 0 || a.3 == 0 {
            return b;
        }
        if b.2 == 0 || b.3 == 0 {
            return a;
        }
        let (x0, y0) = (a.0.min(b.0), a.1.min(b.1));
        let (x1, y1) = ((a.0 + a.2).max(b.0 + b.2), (a.1 + a.3).max(b.1 + b.3));
        (x0, y0, x1 - x0, y1 - y0)
    }

    fn flush_locked(s: &mut State) {
        if s.queue.is_empty() {
            PENDING.store(false, Ordering::Relaxed);
            return;
        }
        let mut queue = std::mem::take(&mut s.queue);
        let mut batch_src = std::mem::take(&mut s.batch_src);
        s.batch_dst.clear();
        s.last = None;
        PENDING.store(false, Ordering::Relaxed);
        let n = queue.len();
        let drawn = match s.gpu.as_mut().and_then(Option::as_mut) {
            Some(g) => draw_gpu(g, &queue, &batch_src),
            None => false,
        };
        if !drawn {
            // GPU error: this batch in software, and the GPU is not used again
            if verbose() {
                eprintln!("qb-runtime: _MAPTRIANGLE 3D falls back to software");
            }
            let dests: HashMap<u32, (usize, u32, u32)> = match s.gpu.take().flatten() {
                Some(g) => g.dests.iter().map(|(k, d)| (*k, (d.ptr, d.w, d.h))).collect(),
                None => HashMap::new(),
            };
            s.gpu = Some(None);
            draw_software(&queue, &batch_src, &dests);
            return;
        }
        // the queue and the source map keep their capacity for the next batch
        queue.clear();
        batch_src.clear();
        s.queue = queue;
        s.batch_src = batch_src;
        if n < SMALL_BATCH {
            s.small += 1;
            if s.small >= SMALL_LIMIT {
                if verbose() {
                    eprintln!("qb-runtime: _MAPTRIANGLE 3D in small batches; using the software renderer");
                }
                s.gpu = Some(None);
            }
        } else {
            s.small = 0;
        }
    }

    /// Draws `queue` with the software renderer and a fresh depth buffer per image.
    fn draw_software(queue: &[Queued], srcs: &HashMap<u32, (u32, u32, Vec<u32>)>, dests: &HashMap<u32, (usize, u32, u32)>) {
        let mut depths: HashMap<u32, Vec<f32>> = HashMap::new();
        for q in queue {
            let (Some(&(ptr, w, h)), Some((sw, sh, spx))) = (dests.get(&q.dst), srcs.get(&q.src)) else { continue };
            let n = w as usize * h as usize;
            let depth = depths.entry(q.dst).or_insert_with(|| vec![f32::INFINITY; n]);
            // SAFETY: the pointer `queue` was given is valid until the flush (its contract).
            let px = unsafe { std::slice::from_raw_parts_mut(ptr as *mut u32, n) };
            let t = &q.t;
            raster::tri3d(
                &mut raster::Target { px, w: w as usize, h: h as usize },
                depth,
                raster::Source { px: spx, w: *sw as usize, h: *sh as usize },
                [(t.d[0][0], t.d[0][1], t.d[0][2]), (t.d[1][0], t.d[1][1], t.d[1][2]), (t.d[2][0], t.d[2][1], t.d[2][2])],
                [(t.s[0][0], t.s[0][1]), (t.s[1][0], t.s[1][1]), (t.s[2][0], t.s[2][1])],
                t.use_alpha,
                t.smooth,
                t.cull,
                t.depth,
            );
        }
    }

    /// Draws `queue` on the GPU; `false` after a device error.
    fn draw_gpu(g: &mut Gpu3, queue: &[Queued], batch_src: &HashMap<u32, (u32, u32, Vec<u32>)>) -> bool {
        // the device may have failed since the triangles were queued (the 3D layer shares it)
        if g.gpu.failed() {
            return false;
        }
        // sources: upload those that changed since their last use
        for (&k, (w, h, px)) in batch_src {
            let (w, h) = (*w, *h);
            if g.srcs.get(&k).is_some_and(|s| (s.w, s.h) == (w, h) && s.px == *px) {
                continue;
            }
            let img = g.r.source(&g.gpu, px, w, h);
            g.srcs.insert(k, SrcTex { w, h, px: px.clone(), img });
        }
        // destinations in the order of their first triangle
        let mut order: Vec<u32> = Vec::new();
        for q in queue {
            if !order.contains(&q.dst) {
                order.push(q.dst);
            }
        }
        for key in order {
            let Some(d) = g.dests.get_mut(&key) else { continue };
            let proj = qb_gpu::maptri::projection(d.w, d.h);
            let mut rect = (0, 0, 0, 0);
            for q in queue.iter().filter(|q| q.dst == key) {
                rect = union(rect, cover(&q.t, &proj, d.w, d.h));
            }
            let (x, y, rw, rh) = rect;
            if rw == 0 || rh == 0 {
                continue;
            }
            let tris: Vec<Tri3d> = queue
                .iter()
                .filter(|q| q.dst == key)
                .filter_map(|q| {
                    let src = &g.srcs.get(&q.src)?.img;
                    Some(Tri3d {
                        src,
                        d: q.t.d,
                        s: q.t.s,
                        use_alpha: q.t.use_alpha,
                        smooth: if q.t.smooth { Smooth::Both } else { Smooth::Off },
                        wrap: true,
                        cull: Cull::from_legacy(q.t.cull),
                        depth: DepthMode::from_legacy(q.t.depth),
                    })
                })
                .collect();
            let w = d.w as usize;
            let row0 = y as usize * w + x as usize;
            // SAFETY: `ptr` holds w*h pixels until the flush ends (contract of `queue`).
            let up = unsafe { std::slice::from_raw_parts((d.ptr as *const u32).add(row0), (rh as usize - 1) * w + rw as usize) };
            g.gpu.queue.write_texture(
                wgpu::TexelCopyTextureInfo { texture: &d.tex, mip_level: 0, origin: wgpu::Origin3d { x, y, z: 0 }, aspect: wgpu::TextureAspect::All },
                bytemuck_u32(up),
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(d.w * 4), rows_per_image: Some(rh) },
                wgpu::Extent3d { width: rw, height: rh, depth_or_array_layers: 1 },
            );
            g.r.draw(&g.gpu, &d.view, &mut d.depth, None, &tris);
            if g.gpu.failed() {
                return false;
            }
            let (ptr, row) = (d.ptr, rw as usize * 4);
            let ok = g.readback.read(&g.gpu, &d.tex, (x, y, rw, rh), |j, bytes| {
                let at = (y as usize + j) * w + x as usize;
                // SAFETY: the row lies inside the w*h pixels of the image (the rectangle
                // is within its bounds), valid until the flush ends (contract of `queue`).
                unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), (ptr as *mut u32).add(at).cast::<u8>(), row) };
            });
            if !ok || g.gpu.failed() {
                return false;
            }
        }
        true
    }

    fn bytemuck_u32(px: &[u32]) -> &[u8] {
        // SAFETY: u32 has no padding; the byte view covers the same memory.
        unsafe { std::slice::from_raw_parts(px.as_ptr().cast::<u8>(), std::mem::size_of_val(px)) }
    }
}

#[cfg(all(test, feature = "gpu"))]
mod tests {
    use super::*;
    use crate::raster;

    /// The queue (and the GPU state) is global: one test draws in sequence.
    #[test]
    fn queued_triangles_match_the_software_renderer() {
        let (w, h) = (64usize, 48usize);
        let red = [0xFFFF_0000u32; 4];
        let blue = [0xFF00_00FFu32; 4];
        let tris = [
            // far blue square (two triangles), near red triangle in front of it
            (2u32, Tri { d: [[-1.5, 1.5, -3.0], [1.5, 1.5, -3.0], [1.5, -1.5, -3.0]], s: [[0.0; 2]; 3], use_alpha: false, smooth: false, cull: 0, depth: 1 }),
            (2, Tri { d: [[-1.5, 1.5, -3.0], [1.5, -1.5, -3.0], [-1.5, -1.5, -3.0]], s: [[0.0; 2]; 3], use_alpha: false, smooth: false, cull: 0, depth: 1 }),
            (1, Tri { d: [[-0.5, 0.5, -2.0], [0.5, 0.5, -2.0], [0.0, -0.5, -2.0]], s: [[0.0; 2]; 3], use_alpha: false, smooth: false, cull: 0, depth: 1 }),
        ];
        // software reference
        let mut sw = vec![0xFF00_0000u32; w * h];
        let mut depth = vec![f32::INFINITY; w * h];
        for (src, t) in tris {
            let px: &[u32] = if src == 1 { &red } else { &blue };
            raster::tri3d(
                &mut raster::Target { px: &mut sw, w, h },
                &mut depth,
                raster::Source { px, w: 2, h: 2 },
                [(t.d[0][0], t.d[0][1], t.d[0][2]), (t.d[1][0], t.d[1][1], t.d[1][2]), (t.d[2][0], t.d[2][1], t.d[2][2])],
                [(0.0, 0.0); 3],
                false,
                false,
                0,
                1,
            );
        }
        // queued, then flushed
        let mut hw = vec![0xFF00_0000u32; w * h];
        let mut queued = 0;
        for (src, t) in tris {
            let px: &[u32] = if src == 1 { &red } else { &blue };
            // SAFETY: hw outlives the flush below and is not touched before it.
            if unsafe { queue(Dst { key: 100, px: hw.as_mut_ptr(), w, h }, SrcImg { key: src, px, w: 2, h: 2 }, t) } {
                queued += 1;
            }
        }
        if queued == 0 {
            eprintln!("skipped: no GPU adapter");
            return;
        }
        assert_eq!(queued, 3);
        assert!(pending());
        // nothing is drawn before the flush
        assert!(hw.iter().all(|&p| p == 0xFF00_0000));
        flush();
        assert!(!pending());
        let differ = sw.iter().zip(&hw).filter(|(a, b)| a != b).count();
        assert!(differ * 100 <= w * h * 3, "{differ} of {} pixels differ", w * h);
        assert_eq!(hw[24 * w + 32], 0xFFFF_0000, "near red in front");
        assert_eq!(hw[24 * w + 20], 0xFF00_00FF, "blue around it");
        assert_eq!(hw[0], 0xFF00_0000, "corner untouched");
        // an exposed image is drawn in software
        expose(100);
        // SAFETY: as above.
        assert!(!unsafe { queue(Dst { key: 100, px: hw.as_mut_ptr(), w, h }, SrcImg { key: 1, px: &red, w: 2, h: 2 }, tris[2].1) });
        forget(100);
    }
}
