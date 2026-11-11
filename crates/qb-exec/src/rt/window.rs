//! The program window of a GUI program (no `$CONSOLE`): qb-runtime's window
//! layer (`qb_runtime::window`, winit + softbuffer, or headless with
//! `QBNEX_HEADLESS=1` / no display) runs on the main thread, the VM on a
//! program thread. Port of the parts of libqb.cpp's MAIN_LOOP and
//! `display()` the VM needs:
//!
//! * frames: [`compose`] turns the display page into 0xAARRGGBB pixels
//!   (text pages through the builtin 8x8/8x14/8x16 fonts with blinking and
//!   the text cursor, palette and 32-bit pages through the page palette);
//!   [`present`] hands them to `qb_window_present`;
//! * timing: a ticker thread raises [`PRESENT_EVENT`] every 32 ms (MAIN_LOOP
//!   displays every second 16 ms loop, `frame` counts those), the VM presents
//!   at its next safe point while `_AUTODISPLAY` is on;
//! * close: the close box (or `close` in a `QBNEX_INPUT_SCRIPT`) raises
//!   [`CLOSE_EVENT`]; the program ends at its next safe point, or at once
//!   while it waits for input (`exit_value` / `end_program` of libqb.cpp;
//!   `_EXIT` is not available to QIR programs, so a close always ends it).

use super::display::Display;
use crate::vm::EventHandle;
use qb_runtime::gfx::data::{charset8x16, charset8x8};
use qb_runtime::gfx::ImgStruct;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::sync::OnceLock;

/// Event bit: present a frame (ticker thread).
pub(crate) const PRESENT_EVENT: u32 = 31;
/// Event bit: the window was closed.
pub(crate) const CLOSE_EVENT: u32 = 30;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static CLOSED: AtomicBool = AtomicBool::new(false);
static EVENTS: OnceLock<EventHandle> = OnceLock::new();
/// `frame` of libqb.cpp: displays so far (~31 per second), drives blinking.
static FRAME: AtomicU32 = AtomicU32::new(0);
/// Size of the last presented frame (mouse coordinate mapping).
static FRAME_W: AtomicI32 = AtomicI32::new(640);
static FRAME_H: AtomicI32 = AtomicI32::new(400);

/// True once the program runs with a window.
pub(crate) fn active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

/// True after the window was closed.
pub(crate) fn closed() -> bool {
    CLOSED.load(Ordering::Relaxed)
}

/// Marks the window mode and starts the frame ticker (program thread, once).
pub(crate) fn activate(ev: EventHandle) {
    if ACTIVE.swap(true, Ordering::SeqCst) {
        return;
    }
    let _ = EVENTS.set(ev.clone());
    std::thread::spawn(move || loop {
        qb_runtime::systime::sleep_ms(32);
        ev.raise(PRESENT_EVENT);
    });
}

extern "C" fn cb_close() {
    CLOSED.store(true, Ordering::SeqCst);
    if let Some(ev) = EVENTS.get() {
        ev.raise(CLOSE_EVENT);
    }
}

/// Window pixel position -> display page pixel position (the window shows the
/// frame scaled and letterboxed by `qb_runtime::raster::layout`).
fn to_frame(x: i32, y: i32) -> (i32, i32) {
    let (fw, fh) = (FRAME_W.load(Ordering::Relaxed), FRAME_H.load(Ordering::Relaxed));
    let (mut ww, mut wh) = (0, 0);
    // SAFETY: two valid out pointers.
    unsafe { qb_runtime::window::qb_window_size(&mut ww, &mut wh) };
    if ww <= 0 || wh <= 0 || (ww == fw && wh == fh) {
        return (x, y);
    }
    let (mut o, mut s) = ([0i32; 8], [0f32; 2]);
    // SAFETY: buffers of the documented sizes.
    unsafe { qb_runtime::window::qb_layout(fw, fh, ww, wh, 0, 0, 0, o.as_mut_ptr(), s.as_mut_ptr()) };
    let (x1, y1, x2, y2) = (o[0], o[1], o[2], o[3]);
    let map = |v: i32, a: i32, b: i32, n: i32| if b > a { ((v - a) as i64 * n as i64 / (b - a + 1) as i64) as i32 } else { v };
    (map(x, x1, x2, fw), map(y, y1, y2, fh))
}

extern "C" fn cb_button(b: i32, down: i32, x: i32, y: i32) {
    let (x, y) = to_frame(x, y);
    super::input::cb_button(b, down, x, y);
}
extern "C" fn cb_move(x: i32, y: i32) {
    let (x, y) = to_frame(x, y);
    super::input::cb_move(x, y);
}
extern "C" fn cb_wheel(dir: i32, x: i32, y: i32) {
    let (x, y) = to_frame(x, y);
    super::input::cb_wheel(dir, x, y);
}

/// Runs the window layer on the calling (main) thread: a real window, or the
/// headless stand-in (`QBNEX_HEADLESS=1`, no display), which also replays
/// `QBNEX_INPUT_SCRIPT`. Returns only when no event loop could run at all.
pub fn run_main(title: &str, hidden: bool) {
    let cb = qb_runtime::window::Callbacks {
        key: Some(super::input::cb_key),
        mouse_button: Some(cb_button),
        mouse_move: Some(cb_move),
        mouse_wheel: Some(cb_wheel),
        mouse_relative: None,
        resize: None,
        close: Some(cb_close),
        focus: None,
        file_drop: None,
        created: None,
    };
    let t = std::ffi::CString::new(title.replace('\0', "")).unwrap_or_default();
    // SAFETY: valid callbacks and a NUL-terminated title.
    unsafe { qb_runtime::window::qb_window_run(&cb, 640, 400, t.as_ptr(), 0, i32::from(hidden)) };
}

/// Builds the next frame of the display page and shows it.
pub(crate) fn present(d: &Display) {
    if !d.started || d.screen_hide {
        return;
    }
    let frame = FRAME.fetch_add(1, Ordering::Relaxed);
    // SAFETY: the display page of a started screen is a live registry entry.
    let p = unsafe { &*d.display_page() };
    let Some((px, w, h)) = compose(p, &d.fontwidth, &d.fontheight, frame & 8 != 0) else { return };
    FRAME_W.store(w as i32, Ordering::Relaxed);
    FRAME_H.store(h as i32, Ordering::Relaxed);
    // SAFETY: `px` holds w*h pixels.
    unsafe { qb_runtime::window::qb_window_present(px.as_ptr(), w as i32, h as i32, 0, 0, 0) };
}

/// The next frame of the display page without showing it (SUB _GL draws it
/// behind its OpenGL output): `(argb, width, height)`.
pub(crate) fn pixels(d: &Display) -> Option<(Vec<u32>, usize, usize)> {
    // (a hidden screen still has its pixels: SUB _GL draws over them offscreen)
    if !d.started {
        return None;
    }
    let frame = FRAME.fetch_add(1, Ordering::Relaxed);
    // SAFETY: the display page of a started screen is a live registry entry.
    let p = unsafe { &*d.display_page() };
    let r = compose(p, &d.fontwidth, &d.fontheight, frame & 8 != 0)?;
    FRAME_W.store(r.1 as i32, Ordering::Relaxed);
    FRAME_H.store(r.2 as i32, Ordering::Relaxed);
    Some(r)
}

/// Sets the window title (`_TITLE`).
pub(crate) fn set_title(t: &[u8]) {
    let t: Vec<u8> = t.iter().copied().filter(|&c| c != 0).collect();
    if let Ok(c) = std::ffi::CString::new(t) {
        // SAFETY: NUL-terminated string.
        unsafe { qb_runtime::window::qb_window_set_title(c.as_ptr()) };
    }
}

/// The pixels a page shows (`display()` of libqb.cpp): `(argb, width, height)`.
/// `phase`: blink/cursor phase (`frame & 8`).
pub(crate) fn compose(p: &ImgStruct, fontwidth: &[i32], fontheight: &[i32], phase: bool) -> Option<(Vec<u32>, usize, usize)> {
    if p.offset.is_null() || p.width == 0 || p.height == 0 {
        return None;
    }
    let pal = |k: usize| -> u32 {
        if p.pal.is_null() {
            0xFF00_0000
        } else {
            // SAFETY: palettes hold 256 entries.
            unsafe { *p.pal.add(k & 255) }
        }
    };
    let (w, h) = (usize::from(p.width), usize::from(p.height));
    if p.compatible_mode != 0 {
        let mut px = vec![0u32; w * h];
        if p.bytes_per_pixel == 4 {
            // SAFETY: a 32-bit surface holds w*h pixels.
            px.copy_from_slice(unsafe { core::slice::from_raw_parts(p.offset32(), w * h) });
        } else {
            // SAFETY: a palette surface holds w*h bytes.
            let src = unsafe { core::slice::from_raw_parts(p.offset, w * h) };
            let lut: Vec<u32> = (0..256).map(pal).collect();
            for (d, s) in px.iter_mut().zip(src) {
                *d = lut[usize::from(*s)];
            }
        }
        return Some((px, w, h));
    }
    // text page: builtin fonts 8/14/16 (+1 = double width); `_LOADFONT` fonts are drawn with 8x16
    let f = p.font as usize;
    let (fw, fh) = match (fontwidth.get(f), fontheight.get(f)) {
        (Some(&x), Some(&y)) if x > 0 && y > 0 => (x as usize, y as usize),
        _ => (8, 16),
    };
    let (glyphs, row0, grows): (&[u8], usize, usize) = match f {
        8 | 9 => (&charset8x8[..], 0, 8),
        14 | 15 => (&charset8x16[..], 1, 16),
        _ => (&charset8x16[..], 0, 16),
    };
    let dbl = matches!(f, 9 | 15 | 17);
    let (fw_, fh_) = (w * fw, h * fh);
    let mut px = vec![0u32; fw_ * fh_];
    // SAFETY: a text surface holds w*h character/attribute pairs.
    let cells = unsafe { core::slice::from_raw_parts(p.offset, w * h * 2) };
    let (cx, mut cy) = (i32::from(p.cursor_x) - 1, i32::from(p.cursor_y) - 1);
    let mut cx = cx;
    if p.holding_cursor == 2 && cy < h as i32 - 1 {
        cy += 1;
        cx = 0;
    }
    for y in 0..h {
        for x in 0..w {
            let (chr, col) = (usize::from(cells[2 * (y * w + x)]), cells[2 * (y * w + x) + 1]);
            let mut c = usize::from(col & 15);
            let bg = usize::from((col >> 4) & 7);
            if col & 128 != 0 && phase {
                c = bg;
            }
            let (fg, bg) = (pal(c), pal(bg));
            for y3 in 0..fh {
                let gy = row0 + y3;
                let line = &mut px[(y * fh + y3) * fw_ + x * fw..][..fw];
                for (x3, d) in line.iter_mut().enumerate() {
                    let gx = if dbl { x3 / 2 } else { x3 };
                    let on = gy < grows && gx < 8 && glyphs[(chr * grows + gy) * 8 + gx] != 0;
                    *d = if on { fg } else { bg };
                }
            }
            if p.cursor_show != 0 && phase && cx == x as i32 && cy == y as i32 {
                for y3 in cursor_rows(p.cursor_firstvalue, p.cursor_lastvalue, fh) {
                    px[(y * fh + y3) * fw_ + x * fw..][..fw].fill(fg);
                }
            }
        }
    }
    Some((px, fw_, fh_))
}

/// Scanlines of the text cursor (`LOCATE , , , start, stop`), rules of libqb.cpp `display()`.
fn cursor_rows(v1: u8, v2: u8, ch: usize) -> std::ops::Range<usize> {
    let bottom = |size: usize| {
        let last = if ch == 16 { 14 } else { ch - 1 };
        (last + 1).saturating_sub(size)..last + 1
    };
    let from = |begin: usize, size: Option<usize>| {
        if begin >= ch {
            return 0..0;
        }
        let size = size.unwrap_or(ch - begin);
        begin..(begin + size).min(ch)
    };
    let (v1, v2) = (usize::from(v1), usize::from(v2));
    if v2 == 0 {
        return if v1 == 0 { from(0, Some(1)) } else { 0..0 };
    }
    if v2 < v1 {
        return from(v2, None);
    }
    if v1 == v2 {
        return if v1 <= 3 { from(v1, Some(1)) } else { bottom(1) };
    }
    if v1 >= 3 {
        return match v2 - v1 {
            1 => bottom(2),
            2 => bottom(3),
            // half cursor: the lower half, from the last scanline up
            _ => ch - ch / 2..ch,
        };
    }
    if v1 <= 1 {
        return if v2 <= 3 { from(v1, Some(v2 - v1 + 1)) } else { from(v1, None) };
    }
    match v2 {
        3 => from(2, Some(2)),
        4 => bottom(3),
        _ => from(2, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn widths() -> (Vec<i32>, Vec<i32>) {
        let (mut w, mut h) = (vec![0; 49], vec![0; 49]);
        for (f, fh) in [(8, 8), (14, 14), (16, 16)] {
            w[f] = 8;
            h[f] = fh;
            w[f + 1] = 16;
            h[f + 1] = fh;
        }
        (w, h)
    }

    #[test]
    fn text_page_draws_glyphs_attributes_and_cursor() {
        let mut cells = vec![0u8; 2 * 2 * 1];
        cells[0] = b'A';
        cells[1] = 0x1E; // yellow on blue
        cells[2] = b' ';
        cells[3] = 0x07;
        let mut pal: Vec<u32> = (0..256).map(|i| 0xFF00_0000 | i as u32).collect();
        let mut p = ImgStruct { width: 2, height: 1, font: 16, offset: cells.as_mut_ptr(), pal: pal.as_mut_ptr(), ..Default::default() };
        p.cursor_x = 2;
        p.cursor_y = 1;
        p.cursor_show = 1;
        p.cursor_firstvalue = 4;
        p.cursor_lastvalue = 4;
        let (w, h) = widths();
        let (px, fw, fh) = compose(&p, &w, &h, true).unwrap();
        assert_eq!((fw, fh), (16, 16));
        // 'A' in 8x16: background row 0, some foreground pixels in the middle rows
        assert_eq!(px[0], 0xFF00_0001);
        assert!(px[..8 * 16].iter().any(|&c| c == 0xFF00_000E));
        assert!(px.iter().enumerate().all(|(i, &c)| i % 16 >= 8 || c == 0xFF00_0001 || c == 0xFF00_000E));
        // cursor (single scanline at the bottom: row 14) in the foreground colour of cell 2
        assert!(px[14 * 16 + 8..14 * 16 + 16].iter().all(|&c| c == 0xFF00_0007));
        assert!(px[13 * 16 + 8..13 * 16 + 16].iter().all(|&c| c == 0xFF00_0000));
        // other phase: no cursor
        let (px, ..) = compose(&p, &w, &h, false).unwrap();
        assert!(px[14 * 16 + 8..14 * 16 + 16].iter().all(|&c| c == 0xFF00_0000));
    }

    #[test]
    fn graphics_pages_map_through_the_palette() {
        let mut data = vec![1u8, 2, 3, 0];
        let mut pal: Vec<u32> = (0..256).map(|i| 0xFF00_0000 | (i as u32) << 8).collect();
        let p = ImgStruct { width: 2, height: 2, compatible_mode: 13, bytes_per_pixel: 1, offset: data.as_mut_ptr(), pal: pal.as_mut_ptr(), ..Default::default() };
        let (w, h) = widths();
        let (px, fw, fh) = compose(&p, &w, &h, false).unwrap();
        assert_eq!((fw, fh), (2, 2));
        assert_eq!(px, vec![0xFF00_0100, 0xFF00_0200, 0xFF00_0300, 0xFF00_0000]);
        let mut d32 = vec![0x1122_3344u32, 5, 6, 7];
        let p = ImgStruct { width: 2, height: 2, compatible_mode: 32, bytes_per_pixel: 4, offset: d32.as_mut_ptr() as *mut u8, ..Default::default() };
        assert_eq!(compose(&p, &w, &h, false).unwrap().0, d32);
    }

    #[test]
    fn cursor_scanline_rules() {
        assert_eq!(cursor_rows(4, 4, 16), 14..15);
        assert_eq!(cursor_rows(0, 0, 16), 0..1);
        assert_eq!(cursor_rows(5, 0, 16), 0..0);
        assert_eq!(cursor_rows(0, 31, 16), 0..16);
        assert_eq!(cursor_rows(6, 7, 16), 13..15);
        assert_eq!(cursor_rows(2, 3, 8), 2..4);
    }
}
