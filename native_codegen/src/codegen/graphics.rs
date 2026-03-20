use super::CodeGenerator;
use core_types::QResult;
use syntax_tree::ast_nodes::Statement;

impl CodeGenerator {
    pub(super) fn write_graphics_prelude(&mut self) {
        self.output.push_str(
            r#"use core_types::DosMemory;
use hal_layer::VGAGraphics;
use minifb::{Window, WindowOptions, Scale, Key};
use std::time::{Duration, Instant};
use std::thread;

static mut WINDOW_BUFFER: Vec<u32> = Vec::new();
static mut WINDOW_WIDTH: usize = 320;
static mut WINDOW_HEIGHT: usize = 200;
static mut WINDOW: Option<Window> = None;
static mut GRAPHICS: Option<VGAGraphics> = None;
static mut CURRENT_COLOR: u8 = 15;
static mut KEY_BUFFER: Vec<Key> = Vec::new();
static mut LAST_UPDATE: Option<Instant> = None;
static mut TEXT_X: usize = 0;
static mut TEXT_Y: usize = 0;

fn init_graphics(width: usize, height: usize) {
    unsafe {
        WINDOW_WIDTH = width;
        WINDOW_HEIGHT = height;
        WINDOW_BUFFER = vec![0; width * height];
        let mut graphics = VGAGraphics::new(DosMemory::new());
        let mode = match (width, height) {
            (320, 200) => 13,
            (640, 480) => 12,
            _ => 0,
        };
        graphics.set_screen_mode(mode);
        GRAPHICS = Some(graphics);
        CURRENT_COLOR = 15;
        TEXT_X = 0;
        TEXT_Y = 0;
        let mut window = Window::new(
            "QBNex Graphics",
            width,
            height,
            WindowOptions {
                resize: true,
                scale: Scale::X2,
                ..WindowOptions::default()
            },
        ).unwrap_or_else(|e| {
            panic!("{}", e);
        });
        window.set_target_fps(60);
        WINDOW = Some(window);
        LAST_UPDATE = Some(Instant::now());
    }
}

#[allow(static_mut_refs)]
fn sync_window_buffer() {
    unsafe {
        if let Some(gfx) = GRAPHICS.as_ref() {
            let framebuffer = gfx.get_framebuffer();
            if WINDOW_BUFFER.len() != framebuffer.len() {
                WINDOW_BUFFER.resize(framebuffer.len(), 0);
            }
            for (idx, &pixel) in framebuffer.iter().enumerate() {
                WINDOW_BUFFER[idx] = gfx.get_palette_color(pixel) | 0xFF000000;
            }
        }
    }
}

#[allow(static_mut_refs)]
fn update_screen() {
    unsafe {
        if let Some(window) = WINDOW.as_mut() {
            if let Some(last) = LAST_UPDATE {
                if last.elapsed() < Duration::from_millis(16) {
                    return;
                }
            }

            if window.is_open() {
                sync_window_buffer();
                let _ = window.update_with_buffer(&WINDOW_BUFFER, WINDOW_WIDTH, WINDOW_HEIGHT);
                let keys = window.get_keys_pressed(minifb::KeyRepeat::No);
                KEY_BUFFER.extend(keys);
                LAST_UPDATE = Some(Instant::now());
            }
        }
    }
}

#[allow(static_mut_refs)]
fn cls() {
    unsafe {
        if let Some(gfx) = GRAPHICS.as_mut() {
            gfx.get_framebuffer_mut().fill(0);
            TEXT_X = 0;
            TEXT_Y = 0;
            update_screen();
        } else {
             print!("\x1B[2J\x1B[1;1H");
        }
    }
}

fn locate(row: f64, col: f64) {
    unsafe {
        TEXT_Y = (row as usize).saturating_sub(1);
        TEXT_X = (col as usize).saturating_sub(1);
    }
}

fn map_color(c: f64) -> u32 {
    match c as u32 {
        0 => 0x000000,
        1 => 0x0000AA,
        2 => 0x00AA00,
        3 => 0x00AAAA,
        4 => 0xAA0000,
        5 => 0xAA00AA,
        6 => 0xAA5500,
        7 => 0xAAAAAA,
        8 => 0x555555,
        9 => 0x5555FF,
        10 => 0x55FF55,
        11 => 0x55FFFF,
        12 => 0xFF5555,
        13 => 0xFF55FF,
        14 => 0xFFFF55,
        15 => 0xFFFFFF,
        _ => 0xFFFFFF,
    }
}

fn normalize_color(c: f64) -> u8 {
    (c.round() as i32).clamp(0, 255) as u8
}

fn qb_point(x: f64, y: f64) -> f64 {
    unsafe {
        GRAPHICS
            .as_ref()
            .map(|gfx| gfx.get_pixel(x.round() as i32, y.round() as i32) as f64)
            .unwrap_or(0.0)
    }
}

fn qb_pmap(coord: f64, func: f64) -> f64 {
    unsafe {
        GRAPHICS
            .as_ref()
            .map(|gfx| gfx.pmap(coord, func.round() as i32))
            .unwrap_or(coord)
    }
}

fn qb_color(foreground: f64) {
    unsafe {
        CURRENT_COLOR = normalize_color(foreground);
    }
}

fn inkey() -> String {
    unsafe {
        if WINDOW.is_some() {
             update_screen();
             if !KEY_BUFFER.is_empty() {
                 let key = KEY_BUFFER.remove(0);
                 return match key {
                     Key::A => "a".to_string(),
                     Key::B => "b".to_string(),
                     Key::C => "c".to_string(),
                     Key::D => "d".to_string(),
                     Key::E => "e".to_string(),
                     Key::F => "f".to_string(),
                     Key::G => "g".to_string(),
                     Key::H => "h".to_string(),
                     Key::I => "i".to_string(),
                     Key::J => "j".to_string(),
                     Key::K => "k".to_string(),
                     Key::L => "l".to_string(),
                     Key::M => "m".to_string(),
                     Key::N => "n".to_string(),
                     Key::O => "o".to_string(),
                     Key::P => "p".to_string(),
                     Key::Q => "q".to_string(),
                     Key::R => "r".to_string(),
                     Key::S => "s".to_string(),
                     Key::T => "t".to_string(),
                     Key::U => "u".to_string(),
                     Key::V => "v".to_string(),
                     Key::W => "w".to_string(),
                     Key::X => "x".to_string(),
                     Key::Y => "y".to_string(),
                     Key::Z => "z".to_string(),
                     Key::Space => " ".to_string(),
                     Key::Enter => "\r".to_string(),
                     Key::Escape => "\x1B".to_string(),
                     _ => "".to_string(),
                 };
             }
             "".to_string()
        } else {
             "".to_string()
        }
    }
}

fn qb_sleep(seconds: f64) {
    unsafe {
        if let Some(window) = WINDOW.as_mut() {
            if seconds <= 0.0 {
                update_screen();
                std::thread::sleep(Duration::from_millis(100));
            } else {
                let start = Instant::now();
                let duration = Duration::from_secs_f64(seconds);

                while start.elapsed() < duration && window.is_open() {
                    update_screen();
                    std::thread::sleep(Duration::from_millis(16));
                }
            }
        } else {
             if seconds <= 0.0 {
                 use std::io::IsTerminal;
                 if std::io::stdin().is_terminal() {
                     let _ = std::io::stdin().read_line(&mut String::new());
                 }
             } else {
                 std::thread::sleep(Duration::from_secs_f64(seconds));
             }
        }
    }
}

fn pset(x: f64, y: f64, color: f64) {
    unsafe {
        if let Some(gfx) = GRAPHICS.as_mut() {
            let color = if color < 0.0 { CURRENT_COLOR } else { normalize_color(color) };
            gfx.pset(x.round() as i32, y.round() as i32, color);
        }
    }
}

fn preset(x: f64, y: f64, color: f64) {
    unsafe {
        if let Some(gfx) = GRAPHICS.as_mut() {
            let color = if color < 0.0 { 0 } else { normalize_color(color) };
            gfx.preset(x.round() as i32, y.round() as i32, color);
        }
    }
}

fn line(x1: f64, y1: f64, x2: f64, y2: f64, color: f64) {
    unsafe {
        if let Some(gfx) = GRAPHICS.as_mut() {
            let color = if color < 0.0 { CURRENT_COLOR } else { normalize_color(color) };
            gfx.line(x1.round() as i32, y1.round() as i32, x2.round() as i32, y2.round() as i32, color);
        }
    }
}

fn circle(x: f64, y: f64, radius: f64, color: f64) {
    unsafe {
        if let Some(gfx) = GRAPHICS.as_mut() {
            let color = if color < 0.0 { CURRENT_COLOR } else { normalize_color(color) };
            gfx.circle(x.round() as i32, y.round() as i32, radius.round() as i32, color);
        }
    }
}

fn paint(x: f64, y: f64, paint_color: f64, border_color: f64) {
    unsafe {
        if let Some(gfx) = GRAPHICS.as_mut() {
            let paint_color = if paint_color < 0.0 { CURRENT_COLOR } else { normalize_color(paint_color) };
            let border_color = if border_color < 0.0 { 0 } else { normalize_color(border_color) };
            gfx.paint(x.round() as i32, y.round() as i32, paint_color, border_color);
        }
    }
}

fn draw_cmd(commands: &str) {
    unsafe {
        if let Some(gfx) = GRAPHICS.as_mut() {
            gfx.draw(commands);
        }
    }
}

fn palette_set(attribute: f64, color: f64) {
    unsafe {
        if let Some(gfx) = GRAPHICS.as_mut() {
            gfx.palette(normalize_color(attribute), normalize_color(color));
        }
    }
}

fn view_rect(x1: f64, y1: f64, x2: f64, y2: f64, fill_color: f64, border_color: f64) {
    unsafe {
        if let Some(gfx) = GRAPHICS.as_mut() {
            let fill_color = if fill_color < 0.0 { 0 } else { normalize_color(fill_color) };
            let border_color = if border_color < 0.0 { 0 } else { normalize_color(border_color) };
            gfx.view(x1.round() as i32, y1.round() as i32, x2.round() as i32, y2.round() as i32, fill_color, border_color);
        }
    }
}

fn view_reset_qb() {
    unsafe {
        if let Some(gfx) = GRAPHICS.as_mut() {
            gfx.view_reset();
        }
    }
}

fn window_set(x1: f64, y1: f64, x2: f64, y2: f64) {
    unsafe {
        if let Some(gfx) = GRAPHICS.as_mut() {
            gfx.window(x1, y1, x2, y2);
        }
    }
}

fn window_reset_qb() {
    unsafe {
        if let Some(gfx) = GRAPHICS.as_mut() {
            gfx.window_reset();
        }
    }
}

fn get_image_to_array(x1: f64, y1: f64, x2: f64, y2: f64, arr_idx: usize, arr_vars: &mut [Vec<f64>]) {
    unsafe {
        if let Some(gfx) = GRAPHICS.as_ref() {
            let data = gfx.get_image(x1.round() as i32, y1.round() as i32, x2.round() as i32, y2.round() as i32);
            if arr_idx < arr_vars.len() {
                arr_vars[arr_idx] = data.into_iter().map(|byte| byte as f64).collect();
            }
        }
    }
}

fn put_image_from_array(x: f64, y: f64, arr_idx: usize, action: &str, arr_vars: &[Vec<f64>]) {
    unsafe {
        if let Some(gfx) = GRAPHICS.as_mut() {
            if let Some(data) = arr_vars.get(arr_idx) {
                let bytes: Vec<u8> = data.iter().map(|value| (*value as i32).clamp(0, 255) as u8).collect();
                gfx.put_image(x.round() as i32, y.round() as i32, &bytes, action);
            }
        }
    }
}
"#,
        );
    }

    pub(super) fn emit_graphics_statement(
        &mut self,
        indent: &str,
        stmt: &Statement,
    ) -> QResult<bool> {
        match stmt {
            Statement::Screen { mode } if self.use_graphics => {
                let mode_code = if let Some(m) = mode {
                    self.generate_expression(m).unwrap_or("0.0".to_string())
                } else {
                    "0.0".to_string()
                };
                self.output
                    .push_str(&format!("{}match {} as i32 {{\n", indent, mode_code));
                self.output
                    .push_str(&format!("{}    13 => init_graphics(320, 200),\n", indent));
                self.output
                    .push_str(&format!("{}    12 => init_graphics(640, 480),\n", indent));
                self.output
                    .push_str(&format!("{}    _ => init_graphics(80, 25),\n", indent));
                self.output.push_str(&format!("{}}}\n", indent));
                Ok(true)
            }
            Statement::Pset { coords, color } if self.use_graphics => {
                let x = self.generate_expression(&coords.0)?;
                let y = self.generate_expression(&coords.1)?;
                let color = self.graphics_color_arg(color.as_ref())?;
                self.output
                    .push_str(&format!("{}pset({}, {}, {});\n", indent, x, y, color));
                Ok(true)
            }
            Statement::Preset { coords, color } if self.use_graphics => {
                let x = self.generate_expression(&coords.0)?;
                let y = self.generate_expression(&coords.1)?;
                let color = self.graphics_color_arg(color.as_ref())?;
                self.output
                    .push_str(&format!("{}preset({}, {}, {});\n", indent, x, y, color));
                Ok(true)
            }
            Statement::Line { coords, color, .. } if self.use_graphics => {
                let x1 = self.generate_expression(&coords.0 .0)?;
                let y1 = self.generate_expression(&coords.0 .1)?;
                let x2 = self.generate_expression(&coords.1 .0)?;
                let y2 = self.generate_expression(&coords.1 .1)?;
                let color = self.graphics_color_arg(color.as_ref())?;
                self.output.push_str(&format!(
                    "{}line({}, {}, {}, {}, {});\n{}update_screen();\n",
                    indent, x1, y1, x2, y2, color, indent
                ));
                Ok(true)
            }
            Statement::Circle {
                center,
                radius,
                color,
                ..
            } if self.use_graphics => {
                let x = self.generate_expression(&center.0)?;
                let y = self.generate_expression(&center.1)?;
                let radius = self.generate_expression(radius)?;
                let color = self.graphics_color_arg(color.as_ref())?;
                self.output.push_str(&format!(
                    "{}circle({}, {}, {}, {});\n{}update_screen();\n",
                    indent, x, y, radius, color, indent
                ));
                Ok(true)
            }
            Statement::Paint {
                coords,
                paint_color,
                border_color,
            } if self.use_graphics => {
                let x = self.generate_expression(&coords.0)?;
                let y = self.generate_expression(&coords.1)?;
                let paint = self.graphics_color_arg(paint_color.as_ref())?;
                let border = self.graphics_color_arg(border_color.as_ref())?;
                self.output.push_str(&format!(
                    "{}paint({}, {}, {}, {});\n{}update_screen();\n",
                    indent, x, y, paint, border, indent
                ));
                Ok(true)
            }
            Statement::Draw { commands } if self.use_graphics => {
                let commands = self.generate_expression(commands)?;
                self.output.push_str(&format!(
                    "{}draw_cmd(&{});\n{}update_screen();\n",
                    indent, commands, indent
                ));
                Ok(true)
            }
            Statement::Palette { attribute, color } if self.use_graphics => {
                let attribute = self.generate_expression(attribute)?;
                let color = if let Some(color) = color {
                    self.generate_expression(color)?
                } else {
                    "0.0".to_string()
                };
                self.output.push_str(&format!(
                    "{}palette_set({}, {});\n{}update_screen();\n",
                    indent, attribute, color, indent
                ));
                Ok(true)
            }
            Statement::View {
                coords,
                fill_color,
                border_color,
            } if self.use_graphics => {
                let x1 = self.generate_expression(&coords.0 .0)?;
                let y1 = self.generate_expression(&coords.0 .1)?;
                let x2 = self.generate_expression(&coords.1 .0)?;
                let y2 = self.generate_expression(&coords.1 .1)?;
                let fill = self.graphics_color_arg(fill_color.as_ref())?;
                let border = self.graphics_color_arg(border_color.as_ref())?;
                self.output.push_str(&format!(
                    "{}view_rect({}, {}, {}, {}, {}, {});\n{}update_screen();\n",
                    indent, x1, y1, x2, y2, fill, border, indent
                ));
                Ok(true)
            }
            Statement::ViewReset if self.use_graphics => {
                self.output.push_str(&format!(
                    "{}view_reset_qb();\n{}update_screen();\n",
                    indent, indent
                ));
                Ok(true)
            }
            Statement::Window { coords } if self.use_graphics => {
                let x1 = self.generate_expression(&coords.0 .0)?;
                let y1 = self.generate_expression(&coords.0 .1)?;
                let x2 = self.generate_expression(&coords.1 .0)?;
                let y2 = self.generate_expression(&coords.1 .1)?;
                self.output.push_str(&format!(
                    "{}window_set({}, {}, {}, {});\n",
                    indent, x1, y1, x2, y2
                ));
                Ok(true)
            }
            Statement::WindowReset if self.use_graphics => {
                self.output
                    .push_str(&format!("{}window_reset_qb();\n", indent));
                Ok(true)
            }
            Statement::GetImage { coords, variable } if self.use_graphics => {
                if let Some(array_name) = self.expr_to_array_name(variable) {
                    let arr_idx = self.get_arr_var_idx(&array_name);
                    let x1 = self.generate_expression(&coords.0 .0)?;
                    let y1 = self.generate_expression(&coords.0 .1)?;
                    let x2 = self.generate_expression(&coords.1 .0)?;
                    let y2 = self.generate_expression(&coords.1 .1)?;
                    self.output.push_str(&format!(
                        "{}get_image_to_array({}, {}, {}, {}, {}, &mut arr_vars);\n",
                        indent, x1, y1, x2, y2, arr_idx
                    ));
                }
                Ok(true)
            }
            Statement::PutImage {
                coords,
                variable,
                action,
            } if self.use_graphics => {
                if let Some(array_name) = self.expr_to_array_name(variable) {
                    let arr_idx = self.get_arr_var_idx(&array_name);
                    let x = self.generate_expression(&coords.0)?;
                    let y = self.generate_expression(&coords.1)?;
                    let action = if let Some(expr) = action {
                        self.generate_expression(expr)?
                    } else {
                        "\"PSET\".to_string()".to_string()
                    };
                    self.output.push_str(&format!(
                        "{}put_image_from_array({}, {}, {}, &{}, &arr_vars);\n{}update_screen();\n",
                        indent, x, y, arr_idx, action, indent
                    ));
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn graphics_color_arg(
        &mut self,
        expr: Option<&syntax_tree::ast_nodes::Expression>,
    ) -> QResult<String> {
        if let Some(expr) = expr {
            self.generate_expression(expr)
        } else {
            Ok("-1.0".to_string())
        }
    }
}
