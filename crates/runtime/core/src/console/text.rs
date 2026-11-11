//! PRINT to a surface (`qbs_print`, `printchr`, `newline`, `tab`, `makefit`),
//! VIEW PRINT, CLS, LOCATE, CSRLIN, POS and the SCREEN() function, ported
//! from libqb.cpp. Surfaces are accessed through raw pointers that are
//! re-read from the host after every call that may change them.

use super::{cells, stdio, utf8, Host};
use super::{lpos, lprint, lprint_buffered, no_control_characters, no_control_characters2, width_lprint};
use crate::gfx::data::{charset8x16, charset8x8};
use crate::gfx::ImgStruct;
use crate::gfx::{line::qb_gfx_boxfill, line::qb_gfx_fast_boxfill};

#[inline]
unsafe fn wp(h: &mut dyn Host) -> *mut ImgStruct {
    h.write_page()
}

#[inline]
fn attr(color: u32, background: u32) -> u8 {
    ((color & 0xF) + background.wrapping_mul(16) + (color & 16) * 8) as u8
}

#[inline]
unsafe fn set_cx(w: *mut ImgStruct, v: i32) {
    (*w).cursor_x = v as i16;
}
#[inline]
unsafe fn set_cy(w: *mut ImgStruct, v: i32) {
    (*w).cursor_y = v as i16;
}

/// `printchr`: one character at the cursor of `write_page`.
pub unsafe fn printchr(h: &mut dyn Host, mut character: u32, cell_span: i32) {
    let im = wp(h);
    let color = (*im).color;
    let background = (*im).background_color;
    if (*im).text != 0 {
        let width = (*im).width as i32;
        let cell = ((*im).cursor_y as i32 - 1) * width + (*im).cursor_x as i32 - 1;
        let a = attr(color, background);
        let o = (*im).offset;
        *o.offset((cell << 1) as isize) = utf8::screen_byte(character);
        *o.offset(((cell << 1) + 1) as isize) = a;
        if cell_span > 1 && cell + 1 < width * (*im).height as i32 {
            *o.offset(((cell + 1) << 1) as isize) = 32;
            *o.offset((((cell + 1) << 1) + 1) as isize) = a;
        }
        let index = h.image_index(im);
        cells::store(index, cell, character, cell_span);
        return;
    }
    let f = (*im).font;
    if h.font_flags(f) & 32 == 0 {
        character &= 255;
    }
    if f >= 32 {
        h.printchr_custom(character);
        return;
    }
    crate::gfx::text::printchr_builtin(&*im, character as i32);
}

/// `chrwidth`: pixel width of `character` in the font of `write_page`.
pub unsafe fn chrwidth(h: &mut dyn Host, character: u32) -> i32 {
    let f = (*wp(h)).font;
    let w = h.font_width(f);
    if w != 0 {
        return w;
    }
    h.chrwidth_custom(character)
}

/// `newline`: cursor to the start of the next line, scrolling the VIEW PRINT
/// window up when it passes the bottom row.
pub unsafe fn newline(h: &mut dyn Host) {
    let w = wp(h);
    set_cy(w, (*w).cursor_y as i32 + 1);
    (*w).cursor_x = 1;
    if (*w).cursor_y <= (*w).bottom_row {
        return;
    }
    if lprint != 0 {
        h.lprint_print_page();
        cls(h, 0, 15, 2);
        lprint_buffered = 0;
        return;
    }
    let width = (*w).width as usize;
    let top = (*w).top_row as isize;
    let bottom = (*w).bottom_row as isize;
    if (*w).text != 0 {
        let row = 2 * width as isize;
        core::ptr::copy((*w).offset.offset(top * row), (*w).offset.offset((top - 1) * row), ((bottom - top) * row) as usize);
        let index = h.image_index(w);
        cells::scroll_up(index, width as i32, top as i32, bottom as i32);
        let a = attr((*w).color, (*w).background_color & 7);
        let line = (*w).offset.offset((bottom - 1) * row);
        for x in 0..width {
            *line.add(x * 2) = 32;
            *line.add(x * 2 + 1) = a;
        }
    } else {
        let fh = h.font_height((*w).font) as isize;
        let w = wp(h);
        let bpp = (*w).bytes_per_pixel as isize;
        let row = bpp * width as isize * fh;
        core::ptr::copy((*w).offset.offset(top * row), (*w).offset.offset((top - 1) * row), ((bottom - top) * row) as usize);
        let n = width * fh as usize;
        if bpp == 1 {
            core::ptr::write_bytes((*w).offset.offset((bottom - 1) * width as isize * fh), (*w).background_color as u8, n);
        } else {
            let p = ((*w).offset as *mut u32).offset((bottom - 1) * width as isize * fh);
            for k in 0..n {
                *p.add(k) = (*w).background_color;
            }
        }
    }
    let w = wp(h);
    (*w).cursor_y = (*w).bottom_row;
}

/// `makefit`: new line first if `text` does not fit on the rest of the line.
pub unsafe fn makefit(h: &mut dyn Host, text: &[u8]) {
    let w = wp(h);
    if (*w).holding_cursor != 0 || (*w).cursor_x == 1 {
        return;
    }
    if (*w).text != 0 {
        if (*w).cursor_x as i32 + text.len() as i32 - 1 > (*w).width as i32 {
            newline(h);
        }
    } else {
        let tw = h.printwidth(text);
        let w = wp(h);
        let mut x = h.font_width((*w).font);
        if x == 0 {
            x = 1;
        }
        let x = x * ((*w).cursor_x as i32 - 1);
        if x + tw > (*w).width as i32 {
            newline(h);
        }
    }
}

/// Spaces that move the console/stdout column `col` to the next 14-column
/// PRINT zone (zones start at 1, 15, 29, ...; at least one space), or `None`
/// when no whole zone is left within `width` columns and the output must go
/// on to a new line.
pub fn zone_spaces(col: i32, width: i32) -> Option<i32> {
    let next = ((col.max(1) - 1) / 14 + 1) * 14 + 1;
    if next > width - 13 { None } else { Some(next - col.max(1)) }
}

/// `tab`: the `,` separator of PRINT (14-column zones).
pub unsafe fn tab(h: &mut dyn Host) {
    let w = wp(h);
    if (*w).holding_cursor != 0 {
        newline(h);
        (*wp(h)).holding_cursor = 0;
        return;
    }
    #[cfg(windows)]
    if (*w).console != 0 {
        // 14-column zones as in QBasic. The column comes from the console, or,
        // when stdout is redirected, from what this program wrote (L-02:
        // asking the console there never advanced, so this looped forever).
        match zone_spaces(stdio::pos(), (*w).width as i32) {
            Some(n) => stdio::write_raw(&vec![b' '; n as usize]),
            None => stdio::write_raw(b"\n"),
        }
        return;
    }
    if (*w).text != 0 {
        print(h, b" ", false);
        loop {
            let w = wp(h);
            let cx = (*w).cursor_x as i32;
            let width = (*w).width as i32;
            if cx != 1 && ((cx - 1) % 14 != 0 || cx > width - 13) && cx < width {
                print(h, b" ", false);
                continue;
            }
            return;
        }
    }
    let fw = h.font_width((*w).font);
    if fw == 0 {
        let fh = h.font_height((*w).font);
        let w = wp(h);
        let x = (*w).cursor_x as i32 - 1;
        let x2 = (x / 112 + 1) * 112;
        let y1 = ((*w).cursor_y as i32 - 1) * fh;
        let y2 = (*w).cursor_y as i32 * fh - 1;
        if x2 >= (*w).width as i32 {
            qb_gfx_fast_boxfill(w, x, y1, (*w).width as i32 - 1, y2, (*w).background_color);
            newline(h);
        } else {
            qb_gfx_fast_boxfill(w, x, y1, x2 - 1, y2, (*w).background_color);
            set_cx(w, x2);
        }
        return;
    }
    let cols = (*w).width as i32 / fw;
    print(h, b" ", false);
    loop {
        let cx = (*wp(h)).cursor_x as i32;
        if cx != 1 && ((cx - 1) % 14 != 0 || cx > cols - 13) && cx < cols {
            print(h, b" ", false);
            continue;
        }
        return;
    }
}

/// `qbs_print`: PRINT `s` to `write_page`; `finish_on_new_line` = no trailing `;`/`,`.
pub unsafe fn print(h: &mut dyn Host, s: &[u8], finish_on_new_line: bool) {
    if h.new_error() {
        return;
    }
    let w = wp(h);
    if (*w).console != 0 {
        stdio::write_utf8(s, finish_on_new_line);
        return;
    }
    if (*w).holding_cursor != 0 {
        if !s.is_empty() {
            (*w).holding_cursor = 0;
            newline(h);
        } else if finish_on_new_line {
            (*w).holding_cursor = 0;
        }
    }
    let len = s.len() as i32;
    let mut entered_new_line = false;
    let mut i: i32 = 0;
    while i < len {
        let w = wp(h);
        let mut character = s[i as usize] as u32;
        if (*w).text != 0 {
            let (used, cp) = utf8::decode(&s[i as usize..]);
            if used == 0 {
                break;
            }
            character = cp;
            i += used as i32 - 1;
        } else if h.font_flags((*w).font) & 32 != 0 {
            if i > len - 4 {
                break;
            }
            let k = i as usize;
            character = u32::from_le_bytes([s[k], s[k + 1], s[k + 2], s[k + 3]]);
            i += 3;
        }
        if lprint != 0 {
            lprint_buffered = 1;
        }
        entered_new_line = false;

        if no_control_characters == 0 && no_control_characters2 == 0 {
            match control(h, character, &mut i, &mut entered_new_line) {
                Ctl::Done => {
                    i += 1;
                    continue;
                }
                Ctl::Print(c) => character = c,
            }
        }

        let w = wp(h);
        if (*w).text != 0 && utf8::is_zero_width(character) {
            let width = (*w).width as i32;
            let mut previous = -1;
            if (*w).cursor_x > 1 {
                previous = ((*w).cursor_y as i32 - 1) * width + (*w).cursor_x as i32 - 2;
            } else if (*w).cursor_y > (*w).top_row {
                previous = ((*w).cursor_y as i32 - 2) * width + width - 1;
            }
            if previous >= 0 {
                let index = h.image_index(w);
                let o = (*w).offset;
                if cells::append(index, previous, character, |c| *o.add(c << 1)) {
                    i += 1;
                    continue;
                }
            }
        }
        let mut cell_span = 1;
        if (*w).text != 0 && utf8::is_wide(character) {
            cell_span = 2;
            if (*w).cursor_x as i32 + cell_span - 1 > (*w).width as i32 {
                newline(h);
            }
        }
        let mut cw = 0;
        let f = (*wp(h)).font;
        let fixed = h.font_width(f) != 0;
        if !fixed {
            cw = chrwidth(h, character);
            let w = wp(h);
            if (*w).cursor_x as i32 + cw > (*w).width as i32 {
                newline(h);
            }
        }
        printchr(h, character, cell_span);
        if lprint != 0 {
            lpos += 1;
            if lpos > width_lprint {
                newline(h);
                entered_new_line = true;
                lpos = 1;
                i += 1;
                continue;
            }
        }
        let w = wp(h);
        if fixed {
            set_cx(w, (*w).cursor_x as i32 + cell_span);
        } else {
            set_cx(w, (*w).cursor_x as i32 + cw);
        }
        if (*w).compatible_mode != 0 {
            let mut x = h.font_width((*w).font);
            if x == 0 {
                x = 1;
            }
            let w = wp(h);
            let x2 = x * ((*w).cursor_x as i32 - 1);
            if x2 > (*w).width as i32 - x {
                if !finish_on_new_line && i == len - 1 {
                    set_cx(w, (*w).width as i32 / x);
                    (*w).holding_cursor = 1;
                    i += 1;
                    continue;
                }
                newline(h);
                entered_new_line = true;
            }
        } else if (*w).cursor_x as i32 > (*w).width as i32 {
            if !finish_on_new_line && i == len - 1 {
                (*w).cursor_x -= 1;
                (*w).holding_cursor = 1;
                i += 1;
                continue;
            }
            newline(h);
            entered_new_line = true;
        }
        i += 1;
    }
    if finish_on_new_line && !entered_new_line {
        newline(h);
    }
    if lprint != 0 && finish_on_new_line {
        lpos = 1;
    }
}

enum Ctl {
    /// Fully handled (cursor movement etc.).
    Done,
    /// Print this character.
    Print(u32),
}

/// Control characters 7, 9-13, 28-31.
unsafe fn control(h: &mut dyn Host, character: u32, i: &mut i32, entered_new_line: &mut bool) -> Ctl {
    let w = wp(h);
    let text = (*w).text != 0;
    // characters per line of a graphics surface with a fixed font
    let cols = |h: &mut dyn Host, w: *mut ImgStruct| -> i32 {
        if text {
            return (*w).width as i32;
        }
        let fw = h.font_width((*w).font);
        if fw != 0 {
            (*w).width as i32 / fw
        } else {
            (*w).width as i32
        }
    };
    match character {
        28 => {
            if lprint != 0 && lpos < width_lprint {
                lpos += 1;
            }
            let c = cols(h, w);
            if (*w).cursor_y >= (*w).bottom_row && (*w).cursor_x as i32 >= c {
                return Ctl::Done;
            }
            (*w).cursor_x += 1;
            if (*w).cursor_x as i32 > c {
                (*w).cursor_y += 1;
                (*w).cursor_x = 1;
            }
            Ctl::Done
        }
        29 => {
            if lprint != 0 && lpos > 1 {
                lpos -= 1;
            }
            if ((*w).cursor_y == (*w).top_row || (*w).cursor_y > (*w).bottom_row) && (*w).cursor_x == 1 {
                return Ctl::Done;
            }
            (*w).cursor_x -= 1;
            if (*w).cursor_x < 1 {
                (*w).cursor_y -= 1;
                let c = cols(h, w);
                set_cx(w, c);
            }
            Ctl::Done
        }
        30 => {
            if (*w).cursor_y > (*w).top_row && (*w).cursor_y <= (*w).bottom_row {
                (*w).cursor_y -= 1;
            }
            Ctl::Done
        }
        31 => {
            if (*w).cursor_y >= (*w).top_row && (*w).cursor_y < (*w).bottom_row {
                (*w).cursor_y += 1;
            }
            Ctl::Done
        }
        12 => {
            if lprint != 0 {
                h.lprint_print_page();
                lprint_buffered = 0;
            }
            cls(h, 0, 0, 0);
            if lprint != 0 {
                lpos = 1;
            }
            Ctl::Done
        }
        11 => {
            (*w).cursor_x = 1;
            (*w).cursor_y = (*w).top_row;
            if lprint != 0 {
                lpos = 1;
            }
            Ctl::Done
        }
        9 => {
            let fw = h.font_width((*w).font);
            if fw == 0 {
                let fh = h.font_height((*w).font);
                let w = wp(h);
                let x = (*w).cursor_x as i32 - 1;
                let x2 = (x / 64 + 1) * 64;
                let y1 = ((*w).cursor_y as i32 - 1) * fh;
                let y2 = (*w).cursor_y as i32 * fh - 1;
                if x2 >= (*w).width as i32 {
                    qb_gfx_fast_boxfill(w, x, y1, (*w).width as i32 - 1, y2, (*w).background_color);
                    newline(h);
                    *entered_new_line = true;
                } else {
                    qb_gfx_fast_boxfill(w, x, y1, x2 - 1, y2, (*w).background_color);
                    set_cx(w, x2);
                }
                return Ctl::Done;
            }
            if (*w).cursor_x % 8 != 0 {
                *i -= 1; // more spaces will be required
            }
            Ctl::Print(32)
        }
        7 => {
            h.sleep(250);
            Ctl::Done
        }
        10 | 13 => {
            newline(h);
            if lprint != 0 {
                lpos = 1;
            }
            Ctl::Done
        }
        _ => Ctl::Print(character),
    }
}

/// `qbg_sub_view_print`: VIEW PRINT [top TO bottom].
pub unsafe fn view_print(h: &mut dyn Host, top: i32, bottom: i32, passed: i32) {
    if h.new_error() {
        return;
    }
    let w = wp(h);
    let mut maxrows = (*w).height as i32;
    if (*w).text == 0 {
        maxrows /= h.font_height((*w).font);
    }
    if passed == 0 {
        (*w).top_row = 1;
        set_cy(w, 1);
        (*w).bottom_row = maxrows as i16;
        (*w).cursor_x = 1;
        (*w).holding_cursor = 0;
        return;
    }
    if top <= 0 || top > maxrows || bottom < top || bottom > maxrows {
        h.error(5);
        return;
    }
    (*w).top_row = top as i16;
    (*w).bottom_row = bottom as i16;
    (*w).cursor_y = (*w).top_row;
    (*w).cursor_x = 1;
    (*w).holding_cursor = 0;
}

unsafe fn center_graphics_cursor(w: *mut ImgStruct) {
    let vw = ((*w).view_x2 - (*w).view_x1 + 1) as f32;
    let vh = ((*w).view_y2 - (*w).view_y1 + 1) as f32;
    if (*w).clipping_or_scaling == 2 {
        (*w).x = vw / (*w).scaling_x / 2.0 + (*w).scaling_offset_x;
        (*w).y = vh / (*w).scaling_y / 2.0 + (*w).scaling_offset_y;
    } else {
        (*w).x = vw / 2.0;
        (*w).y = vh / 2.0;
    }
}

/// Fills the graphics view port (window coordinates when clipping/scaling).
unsafe fn clear_graphics_view(w: *mut ImgStruct, use_color: u32) {
    if (*w).bytes_per_pixel == 1 {
        if (*w).clipping_or_scaling != 0 {
            qb_gfx_boxfill(w, (*w).window_x1, (*w).window_y1, (*w).window_x2, (*w).window_y2, use_color);
        } else {
            core::ptr::write_bytes((*w).offset, use_color as u8, (*w).width as usize * (*w).height as usize);
        }
    } else {
        let a = (*w).alpha_disabled;
        (*w).alpha_disabled = 1;
        if (*w).clipping_or_scaling != 0 {
            qb_gfx_boxfill(w, (*w).window_x1, (*w).window_y1, (*w).window_x2, (*w).window_y2, use_color);
        } else {
            qb_gfx_fast_boxfill(w, 0, 0, (*w).width as i32 - 1, (*w).height as i32 - 1, use_color);
        }
        (*w).alpha_disabled = a;
    }
}

unsafe fn fill_cells(w: *mut ImgStruct, first: usize, count: usize, value: u16) {
    let p = (*w).offset.add(first * 2);
    for k in 0..count {
        *p.add(k * 2) = value as u8;
        *p.add(k * 2 + 1) = (value >> 8) as u8;
    }
}

/// `sub_cls`: CLS [method][, colour].
pub unsafe fn cls(h: &mut dyn Host, method: i32, mut use_color: u32, passed: i32) {
    if h.new_error() {
        return;
    }
    let w = wp(h);
    if (*w).console != 0 {
        stdio::cls(h, use_color, passed);
        return;
    }
    if passed & 2 != 0 {
        if (*w).bytes_per_pixel != 4 && use_color > (*w).mask {
            h.error(5);
            return;
        }
    } else {
        use_color = (*w).background_color;
    }
    if passed & 1 != 0 && !(0..=2).contains(&method) {
        h.error(5);
        return;
    }
    (*w).cursor_y = (*w).top_row;
    (*w).cursor_x = 1;
    (*w).draw_ta = 0.0;
    (*w).draw_scale = 1.0;
    let text = (*w).text != 0;
    let clearvalue: u16 = if text { ((attr((*w).color, use_color & 7) as u16) << 8) + 32 } else { 0 };
    let width = (*w).width as usize;
    let index = if text { h.image_index(w) } else { -1 };
    let top = (*w).top_row as usize;
    let bottom = (*w).bottom_row as usize;

    if passed & 1 == 0 {
        if text {
            let n = width * (bottom - top + 1);
            fill_cells(w, (top - 1) * width, n, clearvalue);
            cells::clear_range(index, ((top - 1) * width) as i32, n as i32);
            let last = ((*w).height as usize - 1) * width;
            fill_cells(w, last, width, clearvalue);
            cells::clear_range(index, last as i32, width as i32);
            h.key_redraw();
            return;
        }
        clear_graphics_view(w, use_color);
        center_graphics_cursor(w);
        h.key_redraw();
        return;
    }
    match method {
        0 => {
            if text {
                let n = (*w).height as usize * width;
                fill_cells(w, 0, n, clearvalue);
                cells::clear_range(index, 0, n as i32);
                h.key_redraw();
                return;
            }
            if (*w).bytes_per_pixel == 1 {
                core::ptr::write_bytes((*w).offset, use_color as u8, width * (*w).height as usize);
            } else {
                let a = (*w).alpha_disabled;
                (*w).alpha_disabled = 1;
                qb_gfx_fast_boxfill(w, 0, 0, width as i32 - 1, (*w).height as i32 - 1, use_color);
                (*w).alpha_disabled = a;
            }
            center_graphics_cursor(w);
            h.key_redraw();
        }
        1 => {
            if text {
                return;
            }
            clear_graphics_view(w, use_color);
            center_graphics_cursor(w);
            h.key_redraw();
        }
        _ => {
            if text {
                let n = width * (bottom - top + 1);
                fill_cells(w, (top - 1) * width, n, clearvalue);
                cells::clear_range(index, ((top - 1) * width) as i32, n as i32);
                return;
            }
            let fh = h.font_height((*w).font);
            let w = wp(h);
            if (*w).bytes_per_pixel == 1 {
                let row = width * fh as usize;
                core::ptr::write_bytes((*w).offset.add(row * (top - 1)), use_color as u8, row * (bottom - top + 1));
            } else {
                let a = (*w).alpha_disabled;
                (*w).alpha_disabled = 1;
                qb_gfx_fast_boxfill(w, 0, fh * (top as i32 - 1), width as i32 - 1, fh * bottom as i32 - 1, use_color);
                (*w).alpha_disabled = a;
            }
        }
    }
}

/// `qbg_sub_locate`: LOCATE [row][, column][, cursor][, start][, stop].
pub unsafe fn locate(h: &mut dyn Host, row: i32, column: i32, cursor: i32, mut start: i32, mut stop: i32, passed: i32) {
    if h.new_error() {
        return;
    }
    let w = wp(h);
    if (*w).console != 0 {
        stdio::locate(row, column, passed);
        return;
    }
    let (rows, cols) = if (*w).compatible_mode != 0 {
        let r = (*w).height as i32 / h.font_height((*w).font);
        let fw = h.font_width((*w).font);
        (r, if fw != 0 { (*w).width as i32 / fw } else { (*w).width as i32 })
    } else {
        ((*w).height as i32, (*w).width as i32)
    };
    let w = wp(h);
    let bad = |h: &mut dyn Host| h.error(5);
    if passed & 1 != 0 {
        if row < (*w).top_row as i32 {
            return bad(h);
        }
        if row != rows && row > (*w).bottom_row as i32 {
            if !h.take_width8050switch() || row > 50 {
                return bad(h);
            }
            if passed & 2 != 0 && (column < 1 || column > cols) {
                return bad(h);
            }
            h.width_80x50();
        }
    }
    let w = wp(h);
    if passed & 2 != 0 && (column < 1 || column > cols) {
        return bad(h);
    }
    if passed & 4 != 0 && !(0..=1).contains(&cursor) {
        return bad(h);
    }
    if passed & 8 != 0 && !(0..=31).contains(&start) {
        return bad(h);
    }
    if passed & 16 != 0 && !(0..=31).contains(&stop) {
        return bad(h);
    }
    if passed & 1 != 0 {
        set_cy(w, row);
        (*w).holding_cursor = 0;
    }
    if passed & 2 != 0 {
        set_cx(w, column);
        (*w).holding_cursor = 0;
    }
    if passed & 3 == 0 && (*w).holding_cursor != 0 {
        (*w).holding_cursor = 2;
    }
    let screen = (*w).flags & crate::gfx::IMG_SCREEN != 0;
    if passed & 4 != 0 {
        let c = (cursor != 0) as u8;
        (*w).cursor_show = c;
        if screen {
            for p in h.screen_pages() {
                (*p).cursor_show = c;
            }
        }
    }
    let w = wp(h);
    if passed & 8 != 0 {
        (*w).cursor_firstvalue = start as u8;
    } else {
        start = (*w).cursor_firstvalue as i32;
    }
    if passed & 16 != 0 {
        (*w).cursor_lastvalue = stop as u8;
    } else {
        stop = (*w).cursor_lastvalue as i32;
    }
    if passed & (8 + 16) != 0 && screen {
        for p in h.screen_pages() {
            (*p).cursor_firstvalue = start as u8;
            (*p).cursor_lastvalue = stop as u8;
        }
    }
}

/// `func_csrlin`.
pub unsafe fn csrlin(h: &mut dyn Host) -> i32 {
    let w = wp(h);
    #[cfg(windows)]
    if (*w).console != 0 {
        return stdio::csrlin();
    }
    if (*w).holding_cursor != 0 {
        if (*w).cursor_y >= (*w).bottom_row {
            return (*w).bottom_row as i32;
        }
        return (*w).cursor_y as i32 + 1;
    }
    (*w).cursor_y as i32
}

/// `func_pos`.
pub unsafe fn pos(h: &mut dyn Host) -> i32 {
    let w = wp(h);
    #[cfg(windows)]
    if (*w).console != 0 {
        return stdio::pos();
    }
    if (*w).holding_cursor != 0 {
        return 1;
    }
    (*w).cursor_x as i32
}

/// `func_screen`: SCREEN(row, column[, colorflag]) on `read_page`.
pub unsafe fn screen(h: &mut dyn Host, y: i32, x: i32, mut returncol: i32, passed: i32) -> u32 {
    if passed == 0 {
        returncol = 0;
    }
    let r = h.read_page();
    #[cfg(windows)]
    if (*r).console != 0 {
        return stdio::screen_read(y, x, returncol != 0);
    }
    if (*r).text != 0 {
        if y < 1 || y > (*r).height as i32 || x < 1 || x > (*r).width as i32 {
            h.error(5);
            return 0;
        }
        let k = (((y - 1) * (*r).width as i32 + x - 1) * 2) as usize;
        return *(*r).offset.add(k + (returncol != 0) as usize) as u32;
    }
    let f = (*r).font;
    if f != 8 && f != 14 && f != 16 {
        h.error(5);
        return 0;
    }
    let fw = h.font_width(f);
    let fh = h.font_height(f);
    let r = h.read_page();
    let cols = (*r).width as i32 / fw;
    let rows = (*r).height as i32 / fh;
    if y < 1 || y > rows || x < 1 || x > cols {
        h.error(5);
        return 0;
    }
    let (x, y) = (x - 1, y - 1);
    let mut sig = Vec::with_capacity((fw * fh) as usize);
    let mut firstcol = 0u32;
    let mut blank = true;
    for y2 in 0..fh {
        for x2 in 0..fw {
            let mut col = crate::gfx::point::point(&*r, x * fw + x2, y * fh + y2);
            if col != 0 {
                if blank {
                    blank = false;
                    firstcol = col;
                }
                col = 255;
            }
            sig.push(col as u8);
        }
    }
    if blank {
        return if returncol != 0 { 1 } else { 32 };
    }
    for i in 0..256usize {
        let glyph: &[u8] = match f {
            8 => &charset8x8[i * 64..],
            14 => &charset8x16[i * 128 + 8..],
            _ => &charset8x16[i * 128..],
        };
        if glyph[..sig.len()] == sig[..] {
            return if returncol != 0 { firstcol } else { i as u32 };
        }
    }
    if returncol != 0 {
        0
    } else {
        32
    }
}
