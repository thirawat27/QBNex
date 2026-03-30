#[derive(Clone, Copy, Debug)]
pub struct ColorPalette {
    pub colors: [u32; 256],
}

impl Default for ColorPalette {
    fn default() -> Self {
        let mut palette = ColorPalette { colors: [0; 256] };
        for (index, slot) in palette.colors.iter_mut().enumerate() {
            *slot = index as u32;
        }
        palette
    }
}

impl ColorPalette {
    pub fn standard_vga() -> Self {
        let mut palette = ColorPalette::default();
        palette.colors[0] = 0x000000;
        palette.colors[1] = 0x0000AA;
        palette.colors[2] = 0x00AA00;
        palette.colors[3] = 0x00AAAA;
        palette.colors[4] = 0xAA0000;
        palette.colors[5] = 0xAA00AA;
        palette.colors[6] = 0xAA5500;
        palette.colors[7] = 0xAAAAAA;
        palette.colors[8] = 0x555555;
        palette.colors[9] = 0x5555FF;
        palette.colors[10] = 0x55FF55;
        palette.colors[11] = 0x55FFFF;
        palette.colors[12] = 0xFF5555;
        palette.colors[13] = 0xFF55FF;
        palette.colors[14] = 0xFFFF55;
        palette.colors[15] = 0xFFFFFF;
        palette
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Viewport {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
    pub fill_color: u8,
    pub border_color: u8,
    pub active: bool,
}

impl Viewport {
    pub fn full_screen(width: u32, height: u32) -> Self {
        Self {
            x1: 0,
            y1: 0,
            x2: width.saturating_sub(1) as i32,
            y2: height.saturating_sub(1) as i32,
            fill_color: 0,
            border_color: 0,
            active: false,
        }
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        !self.active || (x >= self.x1 && x <= self.x2 && y >= self.y1 && y <= self.y2)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WindowCoords {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub active: bool,
}

impl WindowCoords {
    pub fn physical(width: u32, height: u32) -> Self {
        Self {
            x1: 0.0,
            y1: 0.0,
            x2: width.saturating_sub(1) as f64,
            y2: height.saturating_sub(1) as f64,
            active: false,
        }
    }

    pub fn to_physical(
        &self,
        logical_x: f64,
        logical_y: f64,
        phys_width: f64,
        phys_height: f64,
    ) -> (i32, i32) {
        if !self.active {
            return (logical_x.round() as i32, logical_y.round() as i32);
        }

        let width_span = (self.x2 - self.x1).abs();
        let height_span = (self.y2 - self.y1).abs();
        if width_span <= f64::EPSILON || height_span <= f64::EPSILON {
            return (0, 0);
        }

        let phys_x = ((logical_x - self.x1) / (self.x2 - self.x1)) * phys_width;
        let phys_y = ((self.y2 - logical_y) / (self.y2 - self.y1)) * phys_height;
        (phys_x.round() as i32, phys_y.round() as i32)
    }

    pub fn to_logical(
        &self,
        phys_x: i32,
        phys_y: i32,
        phys_width: f64,
        phys_height: f64,
    ) -> (f64, f64) {
        if !self.active {
            return (phys_x as f64, phys_y as f64);
        }

        if phys_width.abs() <= f64::EPSILON || phys_height.abs() <= f64::EPSILON {
            return (self.x1, self.y1);
        }

        let log_x = (phys_x as f64 / phys_width) * (self.x2 - self.x1) + self.x1;
        let log_y = self.y2 - (phys_y as f64 / phys_height) * (self.y2 - self.y1);
        (log_x, log_y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ScreenConfig {
    pub(super) mode: u8,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) colors: u32,
}

impl ScreenConfig {
    pub(super) fn for_mode(mode: u8) -> Self {
        match mode {
            0 | 3 => Self {
                mode,
                width: 80,
                height: 25,
                colors: 16,
            },
            1 | 4 | 5 => Self {
                mode,
                width: 320,
                height: 200,
                colors: 4,
            },
            2 | 6 => Self {
                mode,
                width: 640,
                height: 200,
                colors: 2,
            },
            7 => Self {
                mode,
                width: 320,
                height: 200,
                colors: 16,
            },
            8 => Self {
                mode,
                width: 640,
                height: 200,
                colors: 16,
            },
            9 => Self {
                mode,
                width: 640,
                height: 350,
                colors: 16,
            },
            10 => Self {
                mode,
                width: 640,
                height: 350,
                colors: 4,
            },
            11 => Self {
                mode,
                width: 640,
                height: 480,
                colors: 2,
            },
            12 => Self {
                mode,
                width: 640,
                height: 480,
                colors: 16,
            },
            13 => Self {
                mode,
                width: 320,
                height: 200,
                colors: 256,
            },
            _ => Self {
                mode: 0,
                width: 80,
                height: 25,
                colors: 16,
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Rect {
    pub(super) min_x: i32,
    pub(super) min_y: i32,
    pub(super) max_x: i32,
    pub(super) max_y: i32,
}

impl Rect {
    pub(super) fn from_points(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        max_width: u32,
        max_height: u32,
    ) -> Option<Self> {
        if max_width == 0 || max_height == 0 {
            return None;
        }

        let screen_max_x = max_width as i32 - 1;
        let screen_max_y = max_height as i32 - 1;
        let min_x = x1.min(x2).clamp(0, screen_max_x);
        let min_y = y1.min(y2).clamp(0, screen_max_y);
        let max_x = x1.max(x2).clamp(0, screen_max_x);
        let max_y = y1.max(y2).clamp(0, screen_max_y);

        if min_x > max_x || min_y > max_y {
            return None;
        }

        Some(Self {
            min_x,
            min_y,
            max_x,
            max_y,
        })
    }

    pub(super) fn width(&self) -> u16 {
        (self.max_x - self.min_x + 1) as u16
    }

    pub(super) fn height(&self) -> u16 {
        (self.max_y - self.min_y + 1) as u16
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) enum PutAction {
    Pset,
    Preset,
    Or,
    And,
    Xor,
}

impl PutAction {
    pub(super) fn parse(action: &str) -> Self {
        match action.trim().to_ascii_uppercase().as_str() {
            "PRESET" => Self::Preset,
            "OR" => Self::Or,
            "AND" => Self::And,
            "XOR" => Self::Xor,
            _ => Self::Pset,
        }
    }

    pub(super) fn apply(self, src: u8, dest: u8) -> u8 {
        match self {
            Self::Pset => src,
            Self::Preset => !src,
            Self::Or => dest | src,
            Self::And => dest & src,
            Self::Xor => dest ^ src,
        }
    }
}
