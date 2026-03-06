use core_types::{
    memory_map::{
        TEXT_VIDEO_RAM_END, TEXT_VIDEO_RAM_START, VGA_VIDEO_RAM_END, VGA_VIDEO_RAM_START,
    },
    DosMemory,
};
use std::collections::VecDeque;

mod draw;
#[cfg(test)]
mod tests;
mod types;

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

    fn screen_bounds_contains(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32
    }

    fn is_valid_coord(&self, x: i32, y: i32) -> bool {
        self.screen_bounds_contains(x, y) && self.viewport.contains(x, y)
    }

    fn pixel_index(&self, x: i32, y: i32) -> Option<usize> {
        if !self.screen_bounds_contains(x, y) {
            return None;
        }
        Some(y as usize * self.width as usize + x as usize)
    }

    fn sync_pixel_to_memory(&self, addr: usize, color: u8) {
        match self.screen_mode {
            13 if VGA_VIDEO_RAM_START + addr <= VGA_VIDEO_RAM_END => {
                self.memory.write_byte(VGA_VIDEO_RAM_START + addr, color);
            }
            0 | 7 | 9 => {
                let text_addr = TEXT_VIDEO_RAM_START + addr * 2;
                if text_addr + 1 <= TEXT_VIDEO_RAM_END {
                    self.memory.write_byte(text_addr, color);
                    self.memory.write_byte(text_addr + 1, 0x07);
                }
            }
            _ => {}
        }
    }

    pub fn pset(&mut self, x: i32, y: i32, color: u8) {
        if !self.is_valid_coord(x, y) {
            return;
        }

        if let Some(addr) = self.pixel_index(x, y) {
            self.framebuffer[addr] = color;
            self.sync_pixel_to_memory(addr, color);
        }
    }

    pub fn preset(&mut self, x: i32, y: i32, color: u8) {
        self.pset(x, y, color);
    }

    pub fn line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: u8) {
        let dx = (x2 - x1).abs();
        let dy = (y2 - y1).abs();
        let sx = if x1 < x2 { 1 } else { -1 };
        let sy = if y1 < y2 { 1 } else { -1 };

        let mut err = dx - dy;
        let mut x = x1;
        let mut y = y1;

        loop {
            self.pset(x, y, color);
            if x == x2 && y == y2 {
                break;
            }

            let e2 = err * 2;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }

    pub fn circle(&mut self, cx: i32, cy: i32, radius: i32, color: u8) {
        if radius < 0 {
            return;
        }

        let mut x = radius;
        let mut y = 0;
        let mut err = 0;

        while x >= y {
            self.pset(cx + x, cy + y, color);
            self.pset(cx + y, cy + x, color);
            self.pset(cx - y, cy + x, color);
            self.pset(cx - x, cy + y, color);
            self.pset(cx - x, cy - y, color);
            self.pset(cx - y, cy - x, color);
            self.pset(cx + y, cy - x, color);
            self.pset(cx + x, cy - y, color);

            y += 1;
            err += 1 + 2 * y;
            if 2 * (err - x) + 1 > 0 {
                x -= 1;
                err += 1 - 2 * x;
            }
        }
    }

    pub fn get_pixel(&self, x: i32, y: i32) -> u8 {
        if let Some(addr) = self.pixel_index(x, y) {
            return self.framebuffer.get(addr).copied().unwrap_or(0);
        }
        0
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

    pub fn view(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, fill_color: u8, border_color: u8) {
        let Some(rect) = Rect::from_points(x1, y1, x2, y2, self.width, self.height) else {
            return;
        };

        self.viewport = Viewport {
            x1: rect.min_x,
            y1: rect.min_y,
            x2: rect.max_x,
            y2: rect.max_y,
            fill_color,
            border_color,
            active: true,
        };

        if fill_color != 0 {
            for y in rect.min_y..=rect.max_y {
                for x in rect.min_x..=rect.max_x {
                    self.pset(x, y, fill_color);
                }
            }
        }

        if border_color != 0 {
            self.line(rect.min_x, rect.min_y, rect.max_x, rect.min_y, border_color);
            self.line(rect.max_x, rect.min_y, rect.max_x, rect.max_y, border_color);
            self.line(rect.max_x, rect.max_y, rect.min_x, rect.max_y, border_color);
            self.line(rect.min_x, rect.max_y, rect.min_x, rect.min_y, border_color);
        }

        println!(
            "[VIEW({},{})-({},{}) Fill:{} Border:{}]",
            x1, y1, x2, y2, fill_color, border_color
        );
    }

    pub fn view_reset(&mut self) {
        self.viewport = Viewport::full_screen(self.width, self.height);
        println!("[VIEW RESET]");
    }

    pub fn window(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        self.window = WindowCoords {
            x1,
            y1,
            x2,
            y2,
            active: true,
        };
        println!("[WINDOW({},{})-({},{})]", x1, y1, x2, y2);
    }

    pub fn window_reset(&mut self) {
        self.window = WindowCoords::physical(self.width, self.height);
        println!("[WINDOW RESET]");
    }

    pub fn pmap(&self, coord: f64, func: i32) -> f64 {
        let phys_w = self.width.saturating_sub(1) as f64;
        let phys_h = self.height.saturating_sub(1) as f64;

        match func {
            0 => {
                let (phys_x, _) = self.window.to_physical(coord, 0.0, phys_w, phys_h);
                phys_x as f64
            }
            1 => {
                let (_, phys_y) = self.window.to_physical(0.0, coord, phys_w, phys_h);
                phys_y as f64
            }
            2 => {
                let (log_x, _) = self.window.to_logical(coord as i32, 0, phys_w, phys_h);
                log_x
            }
            3 => {
                let (_, log_y) = self.window.to_logical(0, coord as i32, phys_w, phys_h);
                log_y
            }
            _ => coord,
        }
    }

    pub fn paint(&mut self, x: i32, y: i32, paint_color: u8, border_color: u8) {
        if !self.is_valid_coord(x, y) {
            return;
        }

        let target_color = self.get_pixel(x, y);
        if target_color == paint_color || target_color == border_color {
            return;
        }

        let max_pixels = (self.width * self.height) as usize;
        let mut filled = 0usize;
        let mut queue = VecDeque::from([(x, y)]);

        while let Some((cx, cy)) = queue.pop_front() {
            if filled >= max_pixels || !self.is_valid_coord(cx, cy) {
                continue;
            }

            if self.get_pixel(cx, cy) != target_color {
                continue;
            }

            self.pset(cx, cy, paint_color);
            filled += 1;

            queue.push_back((cx + 1, cy));
            queue.push_back((cx - 1, cy));
            queue.push_back((cx, cy + 1));
            queue.push_back((cx, cy - 1));
        }

        println!(
            "[PAINT({},{}) Filled:{} Color:{} Border:{}]",
            x, y, filled, paint_color, border_color
        );
    }

    pub fn draw(&mut self, commands: &str) {
        DrawInterpreter::new(commands).run(self);
        println!("[DRAW {}]", commands);
    }

    pub fn get_image(&self, x1: i32, y1: i32, x2: i32, y2: i32) -> Vec<u8> {
        let Some(rect) = Rect::from_points(x1, y1, x2, y2, self.width, self.height) else {
            return vec![0, 0, 0, 0];
        };

        let mut data = Vec::with_capacity(4 + rect.width() as usize * rect.height() as usize);
        data.push((rect.width() & 0xFF) as u8);
        data.push((rect.width() >> 8) as u8);
        data.push((rect.height() & 0xFF) as u8);
        data.push((rect.height() >> 8) as u8);

        for y in rect.min_y..=rect.max_y {
            for x in rect.min_x..=rect.max_x {
                data.push(self.get_pixel(x, y));
            }
        }

        data
    }

    pub fn put_image(&mut self, x: i32, y: i32, data: &[u8], action: &str) {
        if data.len() < 4 {
            return;
        }

        let width = u16::from_le_bytes([data[0], data[1]]) as usize;
        let height = u16::from_le_bytes([data[2], data[3]]) as usize;
        if width == 0 || height == 0 {
            return;
        }

        let action = PutAction::parse(action);
        let pixels = &data[4..];

        for row in 0..height {
            for col in 0..width {
                let idx = row * width + col;
                let Some(&src_color) = pixels.get(idx) else {
                    return;
                };

                let px = x + col as i32;
                let py = y + row as i32;
                if !self.is_valid_coord(px, py) {
                    continue;
                }

                let dest_color = self.get_pixel(px, py);
                self.pset(px, py, action.apply(src_color, dest_color));
            }
        }
    }
}
