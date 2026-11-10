//! Text console (phase R12a): PRINT to a surface, LOCATE, CSRLIN, POS, CLS,
//! VIEW PRINT, SCREEN(), the SCREEN 0 cell buffer and its UTF-8 side table,
//! INPUT / LINE INPUT / INPUT$ from the keyboard, the BIOS INKEY$ buffer and
//! the `$CONSOLE` stdout/stdin paths. Ported from `libqb.cpp` bug for bug.
//!
//! Everything works on [`ImgStruct`] surfaces (`write_page`/`read_page`).
//! What is still owned elsewhere (font tables and custom-font glyphs, image
//! handles, `_DEST`/`_SOURCE`, the display thread) is reached through the
//! [`Host`] trait. libqb.cpp registers a C function table with
//! [`qb_console_set_host`]; `qb-exec` can implement [`Host`] directly.
//!
//! The functions take raw surface pointers and re-read them after every host
//! call, because host calls may run legacy code that changes the surface
//! (exactly like the C++ globals they replace).

#![allow(unsafe_op_in_unsafe_fn)]
#![allow(clippy::missing_safety_doc)]

pub mod cells;
pub mod ffi;
pub mod input;
pub mod keyboard;
pub mod stdio;
pub mod text;
pub mod utf8;

use crate::gfx::ImgStruct;

/// Services the console needs from the rest of the runtime.
pub trait Host {
    /// Current `_DEST` surface.
    fn write_page(&mut self) -> *mut ImgStruct;
    /// Current `_SOURCE` surface.
    fn read_page(&mut self) -> *mut ImgStruct;
    /// Image number of a surface (index in the image registry), -1 if unknown.
    fn image_index(&mut self, im: *const ImgStruct) -> i32;
    /// A runtime error is pending (`new_error`).
    fn new_error(&mut self) -> bool;
    /// Raises a QBasic error.
    fn error(&mut self, n: i32);
    /// `fontwidth[f]` (0 = variable width).
    fn font_width(&mut self, f: u32) -> i32;
    /// `fontheight[f]`.
    fn font_height(&mut self, f: u32) -> i32;
    /// `fontflags[f]` (32 = unicode font).
    fn font_flags(&mut self, f: u32) -> i32;
    /// Draws `ch` with the custom (`_LOADFONT`) font of `write_page` at its cursor.
    fn printchr_custom(&mut self, ch: u32);
    /// Width in pixels of `ch` in the custom font of `write_page`.
    fn chrwidth_custom(&mut self, ch: u32) -> i32;
    /// `_PRINTWIDTH(text)` on `write_page`.
    fn printwidth(&mut self, text: &[u8]) -> i32;
    /// Sends the LPRINT page image to the printer (`sub__printimage(lprint_image)`).
    fn lprint_print_page(&mut self);
    /// Redraw the KEY ON bar (`key_display_redraw=1; key_update()`).
    fn key_redraw(&mut self);
    /// Reads and clears the "first LOCATE below row 25 switches to 80x50" flag.
    fn take_width8050switch(&mut self) -> bool;
    /// `WIDTH 80,50` keeping the 80x25 screen contents and colours.
    fn width_80x50(&mut self);
    /// Surfaces of the screen pages (for page-linked cursor attributes).
    fn screen_pages(&mut self) -> Vec<*mut ImgStruct>;
    /// `_DEST` / `_SOURCE` handles and selection.
    fn dest(&mut self) -> i32;
    fn set_dest(&mut self, h: i32);
    fn source(&mut self) -> i32;
    fn set_source(&mut self, h: i32);
    /// `_COPYIMAGE(h)` (-1 on failure), `_FREEIMAGE h`, `PCOPY s,d`.
    fn copyimage(&mut self, h: i32) -> i32;
    fn freeimage(&mut self, h: i32);
    fn pcopy(&mut self, src: i32, dst: i32);
    /// Surface of image handle `h` (negative handle).
    fn image(&mut self, h: i32) -> *mut ImgStruct;
    /// Forces `_AUTODISPLAY` on; returns the previous value.
    fn autodisplay_begin(&mut self) -> i32;
    /// Restores `_DISPLAY` mode after INPUT (previous value from `autodisplay_begin`).
    fn autodisplay_end(&mut self, previous: i32);
    /// The program is being stopped (window closed, END from another thread).
    fn stop_program(&mut self) -> bool;
    /// BIOS data area (`cmem+0x400`): INKEY$ ring buffer.
    fn bios(&mut self) -> *mut u8;
    /// One byte from stdin for `$CONSOLE` INPUT; None at end of input.
    fn stdin_byte(&mut self) -> Option<u8>;
    /// `evnt(0)`: process pending events (INPUT$ wait loop).
    fn events(&mut self);
    fn sleep(&mut self, ms: u32);
    fn ticks(&mut self) -> i64;
}

// --- state shared with libqb.cpp (LPRINT, _CONTROLCHR) ---

/// Set to 1 while LPRINT prints to its page image.
#[unsafe(no_mangle)]
pub static mut lprint: i32 = 0;
/// LPOS: column of the printer head.
#[unsafe(no_mangle)]
pub static mut lpos: i32 = 1;
/// `WIDTH LPRINT`.
#[unsafe(no_mangle)]
pub static mut width_lprint: i32 = 80;
/// Content is waiting on the LPRINT page.
#[unsafe(no_mangle)]
pub static mut lprint_buffered: i32 = 0;
/// Control characters are printed as glyphs (internal use).
#[unsafe(no_mangle)]
pub static mut no_control_characters: i32 = 0;
/// `_CONTROLCHR OFF`.
#[unsafe(no_mangle)]
pub static mut no_control_characters2: i32 = 0;

#[cfg(test)]
mod tests;
