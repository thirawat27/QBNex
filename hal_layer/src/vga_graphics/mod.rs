use core_types::{
    memory_map::{
        TEXT_VIDEO_RAM_END, TEXT_VIDEO_RAM_START, VGA_VIDEO_RAM_END, VGA_VIDEO_RAM_START,
    },
    DosMemory,
};

mod coords;
mod draw;
mod image;
mod raster;
#[cfg(test)]
mod tests;
mod types;
mod view;

use draw::DrawInterpreter;
pub use types::{ColorPalette, Viewport, WindowCoords};
use types::{PutAction, Rect, ScreenConfig};

#[derive(Clone)]
pub struct VGAGraphics {
    pub screen_mode: u8,
    pub width: u32,
    pub height: u32,
    pub colors: u32,
    pub memory: DosMemory,
    pub framebuffer: Vec<u8>,
    pub palette: ColorPalette,
    pub viewport: Viewport,
    pub window: WindowCoords,
}

impl VGAGraphics {
    pub fn new(memory: DosMemory) -> Self {
        Self::from_config(ScreenConfig::for_mode(0), memory)
    }

    fn from_config(config: ScreenConfig, memory: DosMemory) -> Self {
        let framebuffer = vec![0; (config.width * config.height) as usize];
        Self {
            screen_mode: config.mode,
            width: config.width,
            height: config.height,
            colors: config.colors,
            memory,
            framebuffer,
            palette: ColorPalette::standard_vga(),
            viewport: Viewport::full_screen(config.width, config.height),
            window: WindowCoords::physical(config.width, config.height),
        }
    }

    pub fn set_screen_mode(&mut self, mode: u8) {
        let config = ScreenConfig::for_mode(mode);
        self.screen_mode = config.mode;
        self.width = config.width;
        self.height = config.height;
        self.colors = config.colors;
        self.framebuffer = vec![0; (config.width * config.height) as usize];
        self.viewport = Viewport::full_screen(config.width, config.height);
        self.window = WindowCoords::physical(config.width, config.height);
        self.palette = ColorPalette::standard_vga();

        println!(
            "[VGA] Set screen mode {} ({}x{}, {} colors)",
            self.screen_mode, self.width, self.height, self.colors
        );
    }

    pub fn get_framebuffer(&self) -> &[u8] {
        &self.framebuffer
    }

    pub fn get_framebuffer_mut(&mut self) -> &mut Vec<u8> {
        &mut self.framebuffer
    }

    #[allow(dead_code)]
    pub fn get_vram_buffer(&self) -> Vec<u8> {
        self.memory.get_vga_buffer()
    }

    pub fn palette(&mut self, attribute: u8, color: u8) {
        if let Some(slot) = self.palette.colors.get_mut(attribute as usize) {
            *slot = color as u32;
            println!("[PALETTE {} -> {}]", attribute, color);
        }
    }

    pub fn set_palette(&mut self, attr: u8, r: u8, g: u8, b: u8) {
        if let Some(slot) = self.palette.colors.get_mut(attr as usize) {
            *slot = ((r as u32) << 16) | ((g as u32) << 8) | b as u32;
        }
    }

    pub fn get_palette_color(&self, attr: u8) -> u32 {
        self.palette.colors.get(attr as usize).copied().unwrap_or(0)
    }
}
