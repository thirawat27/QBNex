use super::{Rect, VGAGraphics, Viewport, WindowCoords};

impl VGAGraphics {
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
                    self.raw_pset(x, y, fill_color);
                }
            }
        }

        if border_color != 0 {
            self.raw_line(rect.min_x, rect.min_y, rect.max_x, rect.min_y, border_color);
            self.raw_line(rect.max_x, rect.min_y, rect.max_x, rect.max_y, border_color);
            self.raw_line(rect.max_x, rect.max_y, rect.min_x, rect.max_y, border_color);
            self.raw_line(rect.min_x, rect.max_y, rect.min_x, rect.min_y, border_color);
        }
    }

    pub fn view_reset(&mut self) {
        self.viewport = Viewport::full_screen(self.width, self.height);
    }

    pub fn window(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        self.window = WindowCoords {
            x1,
            y1,
            x2,
            y2,
            active: true,
        };
    }

    pub fn window_reset(&mut self) {
        self.window = WindowCoords::physical(self.width, self.height);
    }
}
