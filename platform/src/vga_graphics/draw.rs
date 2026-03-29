use super::VGAGraphics;

#[derive(Debug, Clone, Copy)]
struct DrawState {
    x: i32,
    y: i32,
    color: u8,
    move_only: bool,
    return_after: bool,
    scale: i32,
}

impl Default for DrawState {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            color: 15,
            move_only: false,
            return_after: false,
            scale: 1,
        }
    }
}

pub(super) struct DrawInterpreter<'a> {
    input: &'a str,
    cursor: usize,
    state: DrawState,
}

impl<'a> DrawInterpreter<'a> {
    pub(super) fn new(input: &'a str) -> Self {
        Self {
            input,
            cursor: 0,
            state: DrawState::default(),
        }
    }

    pub(super) fn run(mut self, gfx: &mut VGAGraphics) {
        while let Some(command) = self.next_command() {
            match command {
                ' ' | ';' => {}
                'B' => self.state.move_only = true,
                'N' => self.state.return_after = true,
                'C' => {
                    if let Some(color) = self.parse_i32() {
                        self.state.color = color.clamp(0, 255) as u8;
                    }
                }
                'S' => {
                    if let Some(scale) = self.parse_i32() {
                        self.state.scale = scale.max(1);
                    }
                }
                'M' => self.apply_move_command(gfx),
                'P' => self.apply_paint(gfx),
                'U' => self.apply_vector(gfx, 0, -1),
                'D' => self.apply_vector(gfx, 0, 1),
                'L' => self.apply_vector(gfx, -1, 0),
                'R' => self.apply_vector(gfx, 1, 0),
                'E' => self.apply_vector(gfx, 1, -1),
                'F' => self.apply_vector(gfx, 1, 1),
                'G' => self.apply_vector(gfx, -1, 1),
                'H' => self.apply_vector(gfx, -1, -1),
                'A' => {
                    let _ = self.parse_i32();
                }
                'T' => {
                    if matches!(self.peek_char(), Some('A')) {
                        self.cursor += 1;
                    }
                    let _ = self.parse_i32();
                }
                'X' => {
                    let _ = self.parse_string_operand();
                }
                _ => {}
            }
        }
    }

    fn apply_move_command(&mut self, gfx: &mut VGAGraphics) {
        self.skip_separators();
        let relative = matches!(self.peek_char(), Some('+') | Some('-'));
        let Some(first) = self.parse_i32() else {
            return;
        };
        self.consume_if(',');
        let Some(second) = self.parse_i32() else {
            return;
        };

        let (new_x, new_y) = if relative {
            (
                self.state.x + first * self.state.scale,
                self.state.y + second * self.state.scale,
            )
        } else {
            (first, second)
        };
        self.draw_to(gfx, new_x, new_y);
    }

    fn apply_paint(&mut self, gfx: &mut VGAGraphics) {
        self.skip_separators();
        let Some(paint_color) = self.parse_i32() else {
            return;
        };
        self.consume_if(',');
        let border_color = self.parse_i32().unwrap_or(0);
        gfx.paint(
            self.state.x,
            self.state.y,
            paint_color.clamp(0, 255) as u8,
            border_color.clamp(0, 255) as u8,
        );
    }

    fn apply_vector(&mut self, gfx: &mut VGAGraphics, dx: i32, dy: i32) {
        let distance = self
            .parse_i32()
            .unwrap_or(1)
            .saturating_mul(self.state.scale);
        let new_x = self.state.x + dx.saturating_mul(distance);
        let new_y = self.state.y + dy.saturating_mul(distance);
        self.draw_to(gfx, new_x, new_y);
    }

    fn draw_to(&mut self, gfx: &mut VGAGraphics, new_x: i32, new_y: i32) {
        let old_x = self.state.x;
        let old_y = self.state.y;

        if !self.state.move_only {
            gfx.line(old_x, old_y, new_x, new_y, self.state.color);
        }

        self.state.x = new_x;
        self.state.y = new_y;

        if self.state.return_after {
            self.state.x = old_x;
            self.state.y = old_y;
        }

        self.state.move_only = false;
        self.state.return_after = false;
    }

    fn next_command(&mut self) -> Option<char> {
        while let Some(ch) = self.peek_char() {
            self.cursor += ch.len_utf8();
            if ch.is_whitespace() || ch == ';' {
                continue;
            }
            return Some(ch.to_ascii_uppercase());
        }
        None
    }

    fn parse_i32(&mut self) -> Option<i32> {
        self.skip_separators();
        let start = self.cursor;
        if matches!(self.peek_char(), Some('+') | Some('-')) {
            self.cursor += 1;
        }
        while matches!(self.peek_char(), Some(ch) if ch.is_ascii_digit()) {
            self.cursor += 1;
        }
        if self.cursor == start
            || (self.cursor == start + 1
                && matches!(self.input[start..].chars().next(), Some('+') | Some('-')))
        {
            self.cursor = start;
            return None;
        }
        self.input[start..self.cursor].parse().ok()
    }

    fn parse_string_operand(&mut self) -> Option<&'a str> {
        self.skip_separators();
        let start = self.cursor;
        while matches!(self.peek_char(), Some(ch) if ch != ';') {
            self.cursor += 1;
        }
        let value = self.input[start..self.cursor].trim();
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    }

    fn skip_separators(&mut self) {
        while matches!(self.peek_char(), Some(' ' | ',' | ';')) {
            self.cursor += 1;
        }
    }

    fn consume_if(&mut self, expected: char) -> bool {
        self.skip_separators();
        if self.peek_char() == Some(expected) {
            self.cursor += expected.len_utf8();
            true
        } else {
            false
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.cursor..].chars().next()
    }
}
