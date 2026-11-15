//! Presenting the window frame through the GPU (feature `gpu`).
//!
//! The composed SCREEN frame is still drawn by the CPU (the reference);
//! only the last step — scaling it into the window per [`raster::layout`] —
//! runs on the GPU instead of `raster::scale_to_window` + softbuffer. When
//! no adapter or surface is available (or `QB_GPU=off`), the window keeps
//! the software presenter; it switches to it when the device reports an error.

use std::sync::Arc;

use qb_gpu::blit::Blitter;
use qb_gpu::{Frame, Gpu, GpuError, Target};
use winit::window::Window;

use crate::raster::Layout;

/// The destination rectangle (x, y, w, h) of a [`Layout`].
pub fn layout_rect(l: &Layout) -> (i32, i32, u32, u32) {
    (l.x1, l.y1, l.scaled_w.max(0) as u32, l.scaled_h.max(0) as u32)
}

/// Draws the frame `px` (`w`×`h`, `0xAARRGGBB`) into `frame` as
/// `raster::scale_to_window` would: black outside the layout rectangle,
/// bilinear when the layout is smooth.
pub fn draw_layout(gpu: &Gpu, blit: &mut Blitter, frame: &Frame, px: &[u32], w: usize, h: usize, l: &Layout) {
    blit.smooth = l.smooth;
    blit.draw_rect(gpu, frame, px, w as u32, h as u32, layout_rect(l));
}

/// GPU presenter of one window.
pub struct Present {
    gpu: Gpu,
    target: Target,
    blit: Blitter,
}

impl Present {
    pub fn new(window: Arc<Window>) -> Result<Present, GpuError> {
        let (gpu, target) = Gpu::for_window(window)?;
        let blit = Blitter::new(&gpu, target.format());
        if gpu.failed() {
            return Err(GpuError("the GPU presenter could not be set up".into()));
        }
        Ok(Present { gpu, target, blit })
    }

    /// The device reported an error: the window should switch to the
    /// software presenter instead of drawing through it again.
    pub fn failed(&self) -> bool {
        self.gpu.failed()
    }

    pub fn adapter_name(&self) -> String {
        self.gpu.adapter_name()
    }

    /// The next frame of a `win_w`×`win_h` window; None when the surface
    /// cannot give one now (minimised, lost). Waiting for it (vertical sync)
    /// happens here, before the caller takes the lock of the pixels it draws.
    pub fn acquire(&mut self, win_w: u32, win_h: u32) -> Option<Frame> {
        if self.target.size() != (win_w, win_h) {
            self.target.resize(&self.gpu, win_w, win_h);
        }
        self.target.frame(&self.gpu)
    }

    /// Draws `px` into `frame` with layout `l` (black when there is no picture).
    pub fn draw(&mut self, frame: &Frame, px: &[u32], w: usize, h: usize, l: &Layout) {
        if w > 0 && h > 0 && px.len() >= w * h {
            draw_layout(&self.gpu, &mut self.blit, frame, &px[..w * h], w, h, l);
        } else {
            self.blit.clear(&self.gpu, frame);
        }
    }

    pub fn present(&self, frame: Frame) {
        frame.present(&self.gpu);
    }
}
