use super::{PutAction, Rect, VGAGraphics};

impl VGAGraphics {
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
