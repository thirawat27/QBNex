use super::{Rect, VGAGraphics};

impl VGAGraphics {
    pub(super) fn screen_bounds_contains(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32
    }

    pub(super) fn active_bounds(&self) -> Rect {
        if self.viewport.active {
            Rect {
                min_x: self.viewport.x1,
                min_y: self.viewport.y1,
                max_x: self.viewport.x2,
                max_y: self.viewport.y2,
            }
        } else {
            Rect {
                min_x: 0,
                min_y: 0,
                max_x: self.width as i32 - 1,
                max_y: self.height as i32 - 1,
            }
        }
    }

    pub(super) fn is_valid_coord(&self, x: i32, y: i32) -> bool {
        self.screen_bounds_contains(x, y) && self.viewport.contains(x, y)
    }

    pub(super) fn pixel_index(&self, x: i32, y: i32) -> Option<usize> {
        if !self.screen_bounds_contains(x, y) {
            return None;
        }
        Some(y as usize * self.width as usize + x as usize)
    }

    pub(super) fn logical_to_physical(&self, x: f64, y: f64) -> (i32, i32) {
        let bounds = self.active_bounds();
        if self.window.active {
            let phys_w = (bounds.max_x - bounds.min_x).max(0) as f64;
            let phys_h = (bounds.max_y - bounds.min_y).max(0) as f64;
            let (mapped_x, mapped_y) = self.window.to_physical(x, y, phys_w, phys_h);
            (bounds.min_x + mapped_x, bounds.min_y + mapped_y)
        } else if self.viewport.active {
            (
                bounds.min_x + x.round() as i32,
                bounds.min_y + y.round() as i32,
            )
        } else {
            (x.round() as i32, y.round() as i32)
        }
    }

    pub(super) fn physical_to_logical(&self, x: i32, y: i32) -> (f64, f64) {
        let bounds = self.active_bounds();
        if self.window.active {
            let phys_w = (bounds.max_x - bounds.min_x).max(0) as f64;
            let phys_h = (bounds.max_y - bounds.min_y).max(0) as f64;
            self.window
                .to_logical(x - bounds.min_x, y - bounds.min_y, phys_w, phys_h)
        } else if self.viewport.active {
            ((x - bounds.min_x) as f64, (y - bounds.min_y) as f64)
        } else {
            (x as f64, y as f64)
        }
    }

    pub(super) fn logical_radius_to_physical(&self, radius: f64) -> (i32, i32) {
        if radius <= 0.0 {
            return (0, 0);
        }

        let bounds = self.active_bounds();
        if self.window.active {
            let width_span = (self.window.x2 - self.window.x1).abs();
            let height_span = (self.window.y2 - self.window.y1).abs();
            let phys_w = (bounds.max_x - bounds.min_x).max(0) as f64;
            let phys_h = (bounds.max_y - bounds.min_y).max(0) as f64;
            let rx = if width_span <= f64::EPSILON {
                0
            } else {
                ((radius / width_span) * phys_w).abs().round() as i32
            };
            let ry = if height_span <= f64::EPSILON {
                0
            } else {
                ((radius / height_span) * phys_h).abs().round() as i32
            };
            (rx.max(0), ry.max(0))
        } else {
            let r = radius.round().max(0.0) as i32;
            (r, r)
        }
    }

    pub fn pmap(&self, coord: f64, func: i32) -> f64 {
        match func {
            0 => self.logical_to_physical(coord, 0.0).0 as f64,
            1 => self.logical_to_physical(0.0, coord).1 as f64,
            2 => self.physical_to_logical(coord.round() as i32, 0).0,
            3 => self.physical_to_logical(0, coord.round() as i32).1,
            _ => coord,
        }
    }
}
