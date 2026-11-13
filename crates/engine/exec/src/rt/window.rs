//! The program window of a GUI program (no `$CONSOLE`): qb-runtime's window
//! layer (`qb_runtime::window`, winit + softbuffer, or headless with
//! `QBNEX_HEADLESS=1` / no display) runs on the main thread, the VM on a
//! program thread. The main loop and display rules, as in QB64:
//!
//! * frames: [`compose`] turns the display page into 0xAARRGGBB pixels
//!   (text pages through the builtin 8x8/8x14/8x16 fonts with blinking and
//!   the text cursor, palette and 32-bit pages through the page palette);
//!   [`present`] hands them to `qb_runtime::window::qb_window_present`;
//! * timing: a ticker thread raises [`PRESENT_EVENT`] every 32 ms (MAIN_LOOP
//!   displays every second 16 ms loop, `frame` counts those), the VM presents
//!   at its next safe point while `_AUTODISPLAY` is on;
//! * close: the close box (or `close` in a `QBNEX_INPUT_SCRIPT`) raises
//!   [`CLOSE_EVENT`]; the program ends at its next safe point, or at once
//!   while it waits for input;
//!   after the program called `_EXIT` the request only waits for its next call;
//! * resize, focus, dropped files: the window layer's callbacks record them for
//!   `_RESIZE`, `_WINDOWHASFOCUS` and `_DROPPEDFILE$`.

use super::display::Display;
use crate::vm::EventHandle;
use qb_runtime::gfx::data::{charset8x16, charset8x8};
use qb_runtime::gfx::ImgStruct;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

/// Event bit: present a frame (ticker thread).
pub(crate) const PRESENT_EVENT: u32 = 31;
/// Event bit: the window was closed.
pub(crate) const CLOSE_EVENT: u32 = 30;

/// Bit of [`take_exit_request`]: the window's close box was used.
pub(crate) const EXIT_CLOSE_BOX: i32 = 1;

/// `_EXIT` was called: the program handles close requests itself.
static EXIT_BLOCKED: AtomicBool = AtomicBool::new(false);
/// Close requests the program has not read yet.
static EXIT_VALUE: AtomicI32 = AtomicI32::new(0);
/// `_FULLSCREEN` mode (0 windowed, 1 stretched, 2 square pixels) and `_SMOOTH`.
static FULL_SCREEN: AtomicI32 = AtomicI32::new(0);
static SMOOTH: AtomicBool = AtomicBool::new(false);
/// `_RESIZE`: the user may resize the window (else it snaps back to the page's size).
static RESIZE_ALLOWED: AtomicBool = AtomicBool::new(false);
/// `_RESIZE , _STRETCH | _SMOOTH` (1, 2): the page is scaled to the window; 0 = shown at its own size.
static RESIZE_SCALE: AtomicI32 = AtomicI32::new(0);
/// The window got a new size the program has not asked `_RESIZE` about yet, and that size.
static RESIZE_EVENT: AtomicBool = AtomicBool::new(false);
static RESIZE_W: AtomicI32 = AtomicI32::new(0);
static RESIZE_H: AtomicI32 = AtomicI32::new(0);
static FOCUSED: AtomicBool = AtomicBool::new(true);
/// Native window handle (HWND on Windows), 0 until the window exists.
static HANDLE: AtomicUsize = AtomicUsize::new(0);
/// The window (or its headless stand-in) exists.
static CREATED: AtomicBool = AtomicBool::new(false);
/// `_ACCEPTFILEDROP` and the files dropped since `_FINISHDROP`.
static ACCEPT_DROP: AtomicBool = AtomicBool::new(false);
static DROPPED: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());
/// Milliseconds between two frames (`_FPS`); the default shows about 31 a second.
const DEFAULT_FRAME_MS: u32 = 32;
static FRAME_MS: AtomicU32 = AtomicU32::new(DEFAULT_FRAME_MS);
/// `_FPS` accepts 1 to 200 frames a second.
const MAX_FPS: f64 = 200.0;
static ACTIVE: AtomicBool = AtomicBool::new(false);
static CLOSED: AtomicBool = AtomicBool::new(false);
static EVENTS: OnceLock<EventHandle> = OnceLock::new();
/// Displays so far (~31 per second), drives blinking.
static FRAME: AtomicU32 = AtomicU32::new(0);
/// A first frame was shown: the window has the size of a page.
static SIZED: AtomicBool = AtomicBool::new(false);
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

/// `_EXIT`: from this call on a close request no longer ends the program; the
/// program reads it here. Returns and clears the pending requests
/// ([`EXIT_CLOSE_BOX`]).
pub(crate) fn take_exit_request() -> i32 {
    EXIT_BLOCKED.store(true, Ordering::SeqCst);
    EXIT_VALUE.swap(0, Ordering::SeqCst)
}

/// The program has ended and only its window is left: the close box closes it again.
pub(crate) fn release_exit() {
    EXIT_BLOCKED.store(false, Ordering::SeqCst);
    // a request the program never read closes the window now
    if EXIT_VALUE.swap(0, Ordering::SeqCst) != 0 {
        cb_close();
    }
}

/// `_FULLSCREEN`: `mode` 0 windowed, 1 stretched, 2 square pixels; `smooth` filters the scaling.
pub(crate) fn set_fullscreen(mode: i32, smooth: bool) {
    let old = FULL_SCREEN.swap(mode, Ordering::Relaxed);
    SMOOTH.store(smooth, Ordering::Relaxed);
    if (old != 0) != (mode != 0) {
        qb_runtime::window::qb_window_set_fullscreen(i32::from(mode != 0));
        if mode == 0 {
            // back to a window of the page's size
            qb_runtime::window::qb_window_set_size(FRAME_W.load(Ordering::Relaxed), FRAME_H.load(Ordering::Relaxed));
        }
    }
}

/// `_ALLOWFULLSCREEN`: what ALT+ENTER may switch to. Mode 0 = any, 1 = stretched only,
/// 2 = square pixels only, -1 = nothing (the keys reach the program). Smooth 0 = any,
/// 1 = smooth only, -1 = never smooth.
static ALLOWED_MODE: AtomicI32 = AtomicI32::new(0);
static ALLOWED_SMOOTH: AtomicI32 = AtomicI32::new(0);

/// `_ALLOWFULLSCREEN [{_STRETCH|_SQUAREPIXELS|_OFF|_ALL|OFF}][, {_SMOOTH|_OFF|_ALL|OFF}]`:
/// `method` and `smooth` are the numbers of the options (0 = left out, which means all).
pub(crate) fn allow_fullscreen(method: i32, smooth: i32) {
    let mode = match method {
        1 | 2 => method,
        3 | 5 => -1,
        _ => 0,
    };
    let smooth = match smooth {
        1 => 1,
        2 | 4 => -1,
        _ => 0,
    };
    ALLOWED_MODE.store(mode, Ordering::Relaxed);
    ALLOWED_SMOOTH.store(smooth, Ordering::Relaxed);
}

/// The mode after `(mode, smooth)` in the round ALT+ENTER walks: window, stretched,
/// stretched and smooth, square pixels, square pixels and smooth; the ones
/// `_ALLOWFULLSCREEN` rules out are skipped (the window never is).
fn next_fullscreen(mode: i32, smooth: bool, allowed_mode: i32, allowed_smooth: i32) -> (i32, bool) {
    const ROUND: [(i32, bool); 5] = [(0, false), (1, false), (1, true), (2, false), (2, true)];
    let mut at = ROUND.iter().position(|&m| m == (mode, smooth && mode != 0)).unwrap_or(0);
    loop {
        at = (at + 1) % ROUND.len();
        let (m, s) = ROUND[at];
        let ruled_out = (allowed_mode > 0 && m != allowed_mode) || (allowed_smooth == 1 && !s) || (allowed_smooth == -1 && s);
        if at == 0 || !ruled_out {
            return (m, s);
        }
    }
}

/// ALT+ENTER was pressed: switches to the next full screen mode. False when
/// `_ALLOWFULLSCREEN _OFF` leaves the keys to the program.
pub(crate) fn alt_enter() -> bool {
    let allowed = ALLOWED_MODE.load(Ordering::Relaxed);
    if allowed < 0 || !active() {
        return false;
    }
    let (mode, smooth) = fullscreen();
    let (mode, smooth) = next_fullscreen(mode, smooth, allowed, ALLOWED_SMOOTH.load(Ordering::Relaxed));
    set_fullscreen(mode, smooth);
    true
}

/// (`_FULLSCREEN` mode, `_SMOOTH`).
pub(crate) fn fullscreen() -> (i32, bool) {
    (FULL_SCREEN.load(Ordering::Relaxed), SMOOTH.load(Ordering::Relaxed))
}

/// `_SCALEDWIDTH`, `_SCALEDHEIGHT`: the size the page is shown at inside the window.
pub(crate) fn scaled_size() -> (i32, i32) {
    let (fw, fh) = (FRAME_W.load(Ordering::Relaxed), FRAME_H.load(Ordering::Relaxed));
    let (ww, wh) = qb_runtime::window::qb_window_size();
    if ww <= 0 || wh <= 0 {
        return (fw, fh);
    }
    let (mode, smooth) = fullscreen();
    let l = qb_runtime::raster::layout(fw, fh, ww, wh, mode, smooth, RESIZE_SCALE.load(Ordering::Relaxed));
    (l.scaled_w, l.scaled_h)
}

/// `_RESIZE [ON|OFF][, _STRETCH|_SMOOTH]`: `allow` None keeps the setting;
/// `scale` 0 none, 1 stretch, 2 smooth.
pub(crate) fn set_resize(allow: Option<bool>, scale: i32) {
    if let Some(on) = allow {
        RESIZE_ALLOWED.store(on, Ordering::Relaxed);
        qb_runtime::window::qb_window_set_resizable(i32::from(on));
    }
    RESIZE_SCALE.store(scale, Ordering::Relaxed);
}

/// `_RESIZE`: true once for every new window size, while resizing is allowed.
pub(crate) fn take_resize() -> bool {
    RESIZE_ALLOWED.load(Ordering::Relaxed) && RESIZE_EVENT.swap(false, Ordering::Relaxed)
}

/// `_RESIZEWIDTH`, `_RESIZEHEIGHT`: the size the window was last given.
pub(crate) fn resize_size() -> (i32, i32) {
    (RESIZE_W.load(Ordering::Relaxed), RESIZE_H.load(Ordering::Relaxed))
}

/// `_WINDOWHASFOCUS`
pub(crate) fn focused() -> bool {
    FOCUSED.load(Ordering::Relaxed)
}

/// `_WINDOWHANDLE`: waits for the window to exist (a hidden one never does: 0).
pub(crate) fn handle(hidden: bool) -> i64 {
    /// How long a program waits for its window to be created.
    const CREATE_WAIT_MS: i64 = 2000;
    if !hidden {
        let start = qb_runtime::systime::ticks();
        while !CREATED.load(Ordering::Relaxed) && !closed() && qb_runtime::systime::ticks() - start < CREATE_WAIT_MS {
            qb_runtime::systime::sleep_ms(10);
        }
    }
    // (0 where the system has no window handle to give)
    HANDLE.load(Ordering::Relaxed) as i64
}

/// `_ACCEPTFILEDROP [ON|OFF]`
pub(crate) fn accept_drop(on: bool) {
    ACCEPT_DROP.store(on, Ordering::Relaxed);
}

pub(crate) fn accepts_drop() -> bool {
    ACCEPT_DROP.load(Ordering::Relaxed)
}

/// The files dropped on the window and not yet finished with.
pub(crate) fn dropped() -> std::sync::MutexGuard<'static, Vec<Vec<u8>>> {
    DROPPED.lock().unwrap_or_else(|p| p.into_inner())
}

/// `_FPS n | _AUTO`: `None` restores the default rate. Error 5 below 1 frame a second.
pub(crate) fn set_fps(fps: Option<f64>) -> Result<(), i32> {
    let ms = match fps {
        None => DEFAULT_FRAME_MS,
        // (also a value that is not a number)
        Some(f) if f.is_nan() || f < 1.0 => return Err(5),
        Some(f) => (1000.0 / f.min(MAX_FPS)).round() as u32,
    };
    FRAME_MS.store(ms.max(1), Ordering::Relaxed);
    Ok(())
}

/// Marks the window mode and starts the frame ticker (program thread, once).
pub(crate) fn activate(ev: EventHandle) {
    if ACTIVE.swap(true, Ordering::SeqCst) {
        return;
    }
    let _ = EVENTS.set(ev.clone());
    std::thread::spawn(move || loop {
        qb_runtime::systime::sleep_ms(u64::from(FRAME_MS.load(Ordering::Relaxed)));
        ev.raise(PRESENT_EVENT);
    });
}

fn cb_close() {
    // after `_EXIT` the program answers close requests itself
    if EXIT_BLOCKED.load(Ordering::SeqCst) {
        EXIT_VALUE.fetch_or(EXIT_CLOSE_BOX, Ordering::SeqCst);
        return;
    }
    CLOSED.store(true, Ordering::SeqCst);
    if let Some(ev) = EVENTS.get() {
        ev.raise(CLOSE_EVENT);
    }
}

/// Window pixel position -> display page pixel position (the window shows the
/// frame scaled and letterboxed by `qb_runtime::raster::layout`).
fn to_frame(x: i32, y: i32) -> (i32, i32) {
    let (fw, fh) = (FRAME_W.load(Ordering::Relaxed), FRAME_H.load(Ordering::Relaxed));
    let (ww, wh) = qb_runtime::window::qb_window_size();
    if ww <= 0 || wh <= 0 || (ww == fw && wh == fh) {
        return (x, y);
    }
    let (mode, smooth) = fullscreen();
    let l = qb_runtime::raster::layout(fw, fh, ww, wh, mode, smooth, RESIZE_SCALE.load(Ordering::Relaxed));
    let (x1, y1, x2, y2) = (l.x1, l.y1, l.x2, l.y2);
    let map = |v: i32, a: i32, b: i32, n: i32| if b > a { ((v - a) as i64 * n as i64 / (b - a + 1) as i64) as i32 } else { v };
    (map(x, x1, x2, fw), map(y, y1, y2, fh))
}

/// `_MOUSEMOVE`: puts the pointer on page pixel (`x`, `y`). The headless window
/// has no pointer: the move is reported as if the system had made it.
pub(crate) fn warp_mouse(x: i32, y: i32) {
    if qb_runtime::window::qb_window_headless() != 0 {
        super::input::cb_move(x, y);
        return;
    }
    let (fw, fh) = (FRAME_W.load(Ordering::Relaxed), FRAME_H.load(Ordering::Relaxed));
    let (ww, wh) = qb_runtime::window::qb_window_size();
    let (mode, smooth) = fullscreen();
    let l = qb_runtime::raster::layout(fw, fh, ww, wh, mode, smooth, RESIZE_SCALE.load(Ordering::Relaxed));
    qb_runtime::window::qb_window_warp_mouse((x as f32 * l.x_scale) as i32 + l.x1, (y as f32 * l.y_scale) as i32 + l.y1);
}

fn cb_button(b: i32, down: i32, x: i32, y: i32) {
    let (x, y) = to_frame(x, y);
    super::input::cb_button(b, down, x, y);
}
fn cb_move(x: i32, y: i32) {
    let (x, y) = to_frame(x, y);
    super::input::cb_move(x, y);
}
fn cb_wheel(dir: i32, x: i32, y: i32) {
    let (x, y) = to_frame(x, y);
    super::input::cb_wheel(dir, x, y);
}

fn cb_resize(w: i32, h: i32) {
    RESIZE_W.store(w, Ordering::Relaxed);
    RESIZE_H.store(h, Ordering::Relaxed);
    RESIZE_EVENT.store(true, Ordering::Relaxed);
    // a window the user may not resize goes back to the size of the page
    let (fw, fh) = (FRAME_W.load(Ordering::Relaxed), FRAME_H.load(Ordering::Relaxed));
    if !RESIZE_ALLOWED.load(Ordering::Relaxed) && FULL_SCREEN.load(Ordering::Relaxed) == 0 && SIZED.load(Ordering::Relaxed) && (w, h) != (fw, fh) {
        qb_runtime::window::qb_window_set_size(fw, fh);
    }
}
fn cb_focus(focused: i32) {
    FOCUSED.store(focused != 0, Ordering::Relaxed);
}
fn cb_file_drop(path: &[u8]) {
    if !ACCEPT_DROP.load(Ordering::Relaxed) {
        return;
    }
    dropped().push(path.to_vec());
}
fn cb_created(handle: *mut std::ffi::c_void) {
    HANDLE.store(handle as usize, Ordering::Relaxed);
    CREATED.store(true, Ordering::Relaxed);
}

/// Runs the window layer on the calling (main) thread: a real window, or the
/// headless stand-in (`QBNEX_HEADLESS=1`, no display), which also replays
/// `QBNEX_INPUT_SCRIPT`. Returns only when no event loop could run at all.
///
/// `resize`: `$RESIZE` of the program: 0 off, 1 on, 2 stretch, 3 smooth.
pub fn run_main(title: &str, hidden: bool, resize: i32) {
    if resize > 0 {
        RESIZE_ALLOWED.store(true, Ordering::Relaxed);
        RESIZE_SCALE.store(resize - 1, Ordering::Relaxed);
    }
    let cb = qb_runtime::window::Callbacks {
        key: Some(super::input::cb_key),
        mouse_button: Some(cb_button),
        mouse_move: Some(cb_move),
        mouse_wheel: Some(cb_wheel),
        mouse_relative: Some(super::input::cb_relative),
        resize: Some(cb_resize),
        close: Some(cb_close),
        focus: Some(cb_focus),
        file_drop: Some(cb_file_drop),
        created: Some(cb_created),
    };
    qb_runtime::window::qb_window_run(&cb, 640, 400, &title.replace('\0', ""), resize > 0, hidden);
}

/// Records the size of the frame about to be shown. When it differs from the
/// previous frame's, the window is sized to it: SCREEN, WIDTH and _FONT change
/// the page size, and a window left at its old size would crop or letterbox it.
fn frame_size(w: usize, h: usize) {
    let (w, h) = (w as i32, h as i32);
    let old = (FRAME_W.swap(w, Ordering::Relaxed), FRAME_H.swap(h, Ordering::Relaxed));
    let first = !SIZED.swap(true, Ordering::Relaxed);
    // (a fullscreen window keeps its size and scales the page)
    if first || old != (w, h) {
        super::input::set_page_size(w, h);
        if FULL_SCREEN.load(Ordering::Relaxed) == 0 {
            qb_runtime::window::qb_window_set_size(w, h);
        }
    }
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
    frame_size(w, h);
    qb_runtime::window::qb_window_present(&px, w as i32, h as i32, FULL_SCREEN.load(Ordering::Relaxed), i32::from(SMOOTH.load(Ordering::Relaxed)), RESIZE_SCALE.load(Ordering::Relaxed));
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
    frame_size(r.1, r.2);
    Some(r)
}

/// Shows `px` (0xAARRGGBB, `w` x `h`, drawn elsewhere: an offscreen SUB _GL
/// frame read back) as the next frame.
pub(crate) fn present_pixels(px: &[u32], w: usize, h: usize) {
    if w == 0 || h == 0 || px.len() < w * h {
        return;
    }
    frame_size(w, h);
    qb_runtime::window::qb_window_present(px, w as i32, h as i32, FULL_SCREEN.load(Ordering::Relaxed), i32::from(SMOOTH.load(Ordering::Relaxed)), RESIZE_SCALE.load(Ordering::Relaxed));
}

/// Sets the window title (`_TITLE`).
pub(crate) fn set_title(t: &[u8]) {
    let t: Vec<u8> = t.iter().copied().filter(|&c| c != 0).collect();
    qb_runtime::window::qb_window_set_title(&t);
}

/// The pixels a page shows: `(argb, width, height)`.
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
    let blink = super::display::BLINK.load(Ordering::Relaxed);
    if p.holding_cursor == 2 && cy < h as i32 - 1 {
        cy += 1;
        cx = 0;
    }
    for y in 0..h {
        for x in 0..w {
            let (chr, col) = (usize::from(cells[2 * (y * w + x)]), cells[2 * (y * w + x) + 1]);
            let mut c = usize::from(col & 15);
            // attribute bit 7: blinking, or (`_BLINK OFF`) a bright background colour
            let bg = if blink { usize::from((col >> 4) & 7) } else { usize::from(col >> 4) };
            if blink && col & 128 != 0 && phase {
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

/// Scanlines of the text cursor (`LOCATE , , , start, stop`), with QB64's rules.
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
        let mut cells = vec![0u8; 2 * 2]; // 2 cells x (char, attribute)
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
        assert!(px[..8 * 16].contains(&0xFF00_000E));
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
        // ALT+ENTER: every mode in turn, then only what _ALLOWFULLSCREEN leaves
        assert_eq!(next_fullscreen(0, false, 0, 0), (1, false));
        assert_eq!(next_fullscreen(1, false, 0, 0), (1, true));
        assert_eq!(next_fullscreen(1, true, 0, 0), (2, false));
        assert_eq!(next_fullscreen(2, true, 0, 0), (0, false));
        assert_eq!(next_fullscreen(0, true, 0, 0), (1, false));
        assert_eq!(next_fullscreen(0, false, 2, 1), (2, true));
        assert_eq!(next_fullscreen(2, true, 2, 1), (0, false));
        assert_eq!(next_fullscreen(0, false, 1, -1), (1, false));
        assert_eq!(next_fullscreen(1, false, 1, -1), (0, false));
        assert_eq!(cursor_rows(4, 4, 16), 14..15);
        assert_eq!(cursor_rows(0, 0, 16), 0..1);
        assert_eq!(cursor_rows(5, 0, 16), 0..0);
        assert_eq!(cursor_rows(0, 31, 16), 0..16);
        assert_eq!(cursor_rows(6, 7, 16), 13..15);
        assert_eq!(cursor_rows(2, 3, 8), 2..4);
    }
}
