use super::{DrawInterpreter, VGAGraphics};
use std::collections::VecDeque;

impl VGAGraphics {
    pub(super) fn sync_pixel_to_memory(&self, addr: usize, color: u8) {
        match self.screen_mode {
            13 if super::VGA_VIDEO_RAM_START + addr <= super::VGA_VIDEO_RAM_END => {
                self.memory
                    .write_byte(super::VGA_VIDEO_RAM_START + addr, color);
            }
            0 | 7 | 9 => {
                let text_addr = super::TEXT_VIDEO_RAM_START + addr * 2;
                if text_addr < super::TEXT_VIDEO_RAM_END {
                    self.memory.write_byte(text_addr, color);
                    self.memory.write_byte(text_addr + 1, 0x07);
                }
            }
            _ => {}
        }
    }

    pub(super) fn raw_pset(&mut self, x: i32, y: i32, color: u8) {
        if !self.is_valid_coord(x, y) {
            return;
        }

        if let Some(addr) = self.pixel_index(x, y) {
            self.framebuffer[addr] = color;
            self.sync_pixel_to_memory(addr, color);
        }
    }

    pub(super) fn raw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: u8) {
        let dx = (x2 - x1).abs();
        let dy = (y2 - y1).abs();
        let sx = if x1 < x2 { 1 } else { -1 };
        let sy = if y1 < y2 { 1 } else { -1 };

        let mut err = dx - dy;
        let mut x = x1;
        let mut y = y1;

        loop {
            self.raw_pset(x, y, color);
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

    pub(super) fn raw_circle(&mut self, cx: i32, cy: i32, radius: i32, color: u8) {
        if radius < 0 {
            return;
        }

        let mut x = radius;
        let mut y = 0;
        let mut err = 0;

        while x >= y {
            self.raw_pset(cx + x, cy + y, color);
            self.raw_pset(cx + y, cy + x, color);
            self.raw_pset(cx - y, cy + x, color);
            self.raw_pset(cx - x, cy + y, color);
            self.raw_pset(cx - x, cy - y, color);
            self.raw_pset(cx - y, cy - x, color);
            self.raw_pset(cx + y, cy - x, color);
            self.raw_pset(cx + x, cy - y, color);

            y += 1;
            err += 1 + 2 * y;
            if 2 * (err - x) + 1 > 0 {
                x -= 1;
                err += 1 - 2 * x;
            }
        }
    }

    pub(super) fn raw_ellipse(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, color: u8) {
        if rx <= 0 && ry <= 0 {
            self.raw_pset(cx, cy, color);
            return;
        }

        let steps = (rx.max(ry).max(1) * 8) as usize;
        for step in 0..=steps {
            let theta = (step as f64 / steps as f64) * std::f64::consts::TAU;
            let x = cx + (rx as f64 * theta.cos()).round() as i32;
            let y = cy + (ry as f64 * theta.sin()).round() as i32;
            self.raw_pset(x, y, color);
        }
    }

    pub(super) fn raw_get_pixel(&self, x: i32, y: i32) -> u8 {
        if let Some(addr) = self.pixel_index(x, y) {
            return self.framebuffer.get(addr).copied().unwrap_or(0);
        }
        0
    }

    pub fn pset(&mut self, x: i32, y: i32, color: u8) {
        let (x, y) = self.logical_to_physical(x as f64, y as f64);
        self.raw_pset(x, y, color);
    }

    pub fn preset(&mut self, x: i32, y: i32, color: u8) {
        self.pset(x, y, color);
    }

    pub fn line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: u8) {
        let (x1, y1) = self.logical_to_physical(x1 as f64, y1 as f64);
        let (x2, y2) = self.logical_to_physical(x2 as f64, y2 as f64);
        self.raw_line(x1, y1, x2, y2, color);
    }

    pub fn circle(&mut self, cx: i32, cy: i32, radius: i32, color: u8) {
        if radius < 0 {
            return;
        }
        let (cx, cy) = self.logical_to_physical(cx as f64, cy as f64);
        let (rx, ry) = self.logical_radius_to_physical(radius as f64);
        if rx == ry {
            self.raw_circle(cx, cy, rx, color);
        } else {
            self.raw_ellipse(cx, cy, rx, ry, color);
        }
    }

    pub fn get_pixel(&self, x: i32, y: i32) -> u8 {
        let (x, y) = self.logical_to_physical(x as f64, y as f64);
        self.raw_get_pixel(x, y)
    }

    pub fn paint(&mut self, x: i32, y: i32, paint_color: u8, border_color: u8) {
        let (x, y) = self.logical_to_physical(x as f64, y as f64);
        if !self.is_valid_coord(x, y) {
            return;
        }

        let target_color = self.raw_get_pixel(x, y);
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

            if self.raw_get_pixel(cx, cy) != target_color {
                continue;
            }

            self.raw_pset(cx, cy, paint_color);
            filled += 1;

            queue.push_back((cx + 1, cy));
            queue.push_back((cx - 1, cy));
            queue.push_back((cx, cy + 1));
            queue.push_back((cx, cy - 1));
        }
    }

    pub fn draw(&mut self, commands: &str) {
        DrawInterpreter::new(commands).run(self);
    }
}
