//! C ABI of the console (`include/qb_console.h`). libqb.cpp registers its
//! services once with [`qb_console_set_host`]; its old entry points
//! (`qbs_print`, `qbg_sub_locate`, ...) are one-line shims over these.

use super::{input, keyboard, stdio, text, Host};
use crate::gfx::ImgStruct;
use core::ffi::c_void;
use core::sync::atomic::{AtomicPtr, Ordering};

/// Function table filled by libqb.cpp (`qb_console_host` in qb_console.h).
#[repr(C)]
pub struct QbConsoleHost {
    pub write_page: unsafe extern "C" fn() -> *mut ImgStruct,
    pub read_page: unsafe extern "C" fn() -> *mut ImgStruct,
    pub image_index: unsafe extern "C" fn(*const ImgStruct) -> i32,
    pub image: unsafe extern "C" fn(i32) -> *mut ImgStruct,
    pub new_error: unsafe extern "C" fn() -> i32,
    pub error: unsafe extern "C" fn(i32),
    pub font_width: unsafe extern "C" fn(u32) -> i32,
    pub font_height: unsafe extern "C" fn(u32) -> i32,
    pub font_flags: unsafe extern "C" fn(u32) -> i32,
    pub printchr_custom: unsafe extern "C" fn(u32),
    pub chrwidth_custom: unsafe extern "C" fn(u32) -> i32,
    pub printwidth: unsafe extern "C" fn(*const u8, i32) -> i32,
    pub lprint_print_page: unsafe extern "C" fn(),
    pub key_redraw: unsafe extern "C" fn(),
    pub take_width8050switch: unsafe extern "C" fn() -> i32,
    pub width_80x50: unsafe extern "C" fn(),
    pub page_count: unsafe extern "C" fn() -> i32,
    pub page_image: unsafe extern "C" fn(i32) -> *mut ImgStruct,
    pub dest: unsafe extern "C" fn() -> i32,
    pub set_dest: unsafe extern "C" fn(i32),
    pub source: unsafe extern "C" fn() -> i32,
    pub set_source: unsafe extern "C" fn(i32),
    pub copyimage: unsafe extern "C" fn(i32) -> i32,
    pub freeimage: unsafe extern "C" fn(i32),
    pub pcopy: unsafe extern "C" fn(i32, i32),
    pub autodisplay_begin: unsafe extern "C" fn() -> i32,
    pub autodisplay_end: unsafe extern "C" fn(i32),
    pub stop_program: unsafe extern "C" fn() -> i32,
    pub bios: unsafe extern "C" fn() -> *mut u8,
    pub events: unsafe extern "C" fn(),
    pub sleep: unsafe extern "C" fn(u32),
}

static HOST: AtomicPtr<QbConsoleHost> = AtomicPtr::new(core::ptr::null_mut());

/// Registers the host table (kept for the life of the program).
///
/// # Safety
/// `host` must stay valid while console functions run.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_console_set_host(host: *const QbConsoleHost) {
    HOST.store(host as *mut _, Ordering::Release);
}

/// [`Host`] over the registered C table.
pub struct CHost(&'static QbConsoleHost);

impl CHost {
    /// The registered host. Panics (aborts across FFI) if none was set.
    pub fn get() -> CHost {
        let p = HOST.load(Ordering::Acquire);
        assert!(!p.is_null(), "qb_console_set_host was not called");
        // SAFETY: set once to a static table by the C side.
        CHost(unsafe { &*p })
    }
}

impl Host for CHost {
    fn write_page(&mut self) -> *mut ImgStruct {
        unsafe { (self.0.write_page)() }
    }
    fn read_page(&mut self) -> *mut ImgStruct {
        unsafe { (self.0.read_page)() }
    }
    fn image_index(&mut self, im: *const ImgStruct) -> i32 {
        unsafe { (self.0.image_index)(im) }
    }
    fn new_error(&mut self) -> bool {
        unsafe { (self.0.new_error)() != 0 }
    }
    fn error(&mut self, n: i32) {
        unsafe { (self.0.error)(n) }
    }
    fn font_width(&mut self, f: u32) -> i32 {
        unsafe { (self.0.font_width)(f) }
    }
    fn font_height(&mut self, f: u32) -> i32 {
        unsafe { (self.0.font_height)(f) }
    }
    fn font_flags(&mut self, f: u32) -> i32 {
        unsafe { (self.0.font_flags)(f) }
    }
    fn printchr_custom(&mut self, ch: u32) {
        unsafe { (self.0.printchr_custom)(ch) }
    }
    fn chrwidth_custom(&mut self, ch: u32) -> i32 {
        unsafe { (self.0.chrwidth_custom)(ch) }
    }
    fn printwidth(&mut self, text: &[u8]) -> i32 {
        unsafe { (self.0.printwidth)(text.as_ptr(), text.len() as i32) }
    }
    fn lprint_print_page(&mut self) {
        unsafe { (self.0.lprint_print_page)() }
    }
    fn key_redraw(&mut self) {
        unsafe { (self.0.key_redraw)() }
    }
    fn take_width8050switch(&mut self) -> bool {
        unsafe { (self.0.take_width8050switch)() != 0 }
    }
    fn width_80x50(&mut self) {
        unsafe { (self.0.width_80x50)() }
    }
    fn screen_pages(&mut self) -> Vec<*mut ImgStruct> {
        unsafe {
            (0..(self.0.page_count)()).map(|i| (self.0.page_image)(i)).filter(|p| !p.is_null()).collect()
        }
    }
    fn dest(&mut self) -> i32 {
        unsafe { (self.0.dest)() }
    }
    fn set_dest(&mut self, h: i32) {
        unsafe { (self.0.set_dest)(h) }
    }
    fn source(&mut self) -> i32 {
        unsafe { (self.0.source)() }
    }
    fn set_source(&mut self, h: i32) {
        unsafe { (self.0.set_source)(h) }
    }
    fn copyimage(&mut self, h: i32) -> i32 {
        unsafe { (self.0.copyimage)(h) }
    }
    fn freeimage(&mut self, h: i32) {
        unsafe { (self.0.freeimage)(h) }
    }
    fn pcopy(&mut self, src: i32, dst: i32) {
        unsafe { (self.0.pcopy)(src, dst) }
    }
    fn image(&mut self, h: i32) -> *mut ImgStruct {
        unsafe { (self.0.image)(h) }
    }
    fn autodisplay_begin(&mut self) -> i32 {
        unsafe { (self.0.autodisplay_begin)() }
    }
    fn autodisplay_end(&mut self, previous: i32) {
        unsafe { (self.0.autodisplay_end)(previous) }
    }
    fn stop_program(&mut self) -> bool {
        unsafe { (self.0.stop_program)() != 0 }
    }
    fn bios(&mut self) -> *mut u8 {
        unsafe { (self.0.bios)() }
    }
    fn stdin_byte(&mut self) -> Option<u8> {
        stdio::getc()
    }
    fn events(&mut self) {
        unsafe { (self.0.events)() }
    }
    fn sleep(&mut self, ms: u32) {
        unsafe { (self.0.sleep)(ms) }
    }
    fn ticks(&mut self) -> i64 {
        crate::systime::ticks()
    }
}

unsafe fn bytes<'a>(p: *const u8, len: i32) -> &'a [u8] {
    crate::ffi_bytes(p, len.max(0) as usize)
}

// --- PRINT and cursor ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_console_print(s: *const u8, len: i32, finish_on_new_line: i32) {
    text::print(&mut CHost::get(), bytes(s, len), finish_on_new_line != 0)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_console_newline() {
    text::newline(&mut CHost::get())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_console_tab() {
    text::tab(&mut CHost::get())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_console_makefit(s: *const u8, len: i32) {
    text::makefit(&mut CHost::get(), bytes(s, len))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_console_printchr(character: u32, cell_span: i32) {
    text::printchr(&mut CHost::get(), character, cell_span)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_console_chrwidth(character: u32) -> i32 {
    text::chrwidth(&mut CHost::get(), character)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_console_view_print(top: i32, bottom: i32, passed: i32) {
    text::view_print(&mut CHost::get(), top, bottom, passed)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_console_cls(method: i32, use_color: u32, passed: i32) {
    text::cls(&mut CHost::get(), method, use_color, passed)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_console_locate(row: i32, column: i32, cursor: i32, start: i32, stop: i32, passed: i32) {
    text::locate(&mut CHost::get(), row, column, cursor, start, stop, passed)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_console_csrlin() -> i32 {
    text::csrlin(&mut CHost::get())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_console_pos() -> i32 {
    text::pos(&mut CHost::get())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_console_screen(y: i32, x: i32, returncol: i32, passed: i32) -> u32 {
    text::screen(&mut CHost::get(), y, x, returncol, passed)
}

/// COLOR: console path or `qb_gfx_color` on `write_page`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_console_color(col1: u32, col2: u32, passed: i32) {
    let mut h = CHost::get();
    if h.new_error() || passed == 0 {
        return;
    }
    let w = h.write_page();
    let e = if (*w).console != 0 {
        stdio::color(col1, col2, passed)
    } else {
        crate::gfx::palette::qb_gfx_color(w, col1, col2, passed)
    };
    if e != 0 {
        h.error(e);
    }
}

/// WIDTH on a Windows console `_DEST`: returns 1 when handled.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_console_width(v1: i32, v2: i32, v3: i32, v4: i32) -> i32 {
    #[cfg(windows)]
    {
        let mut h = CHost::get();
        if (*h.write_page()).console != 0 {
            stdio::width(v1, v2, v3, v4);
            return 1;
        }
    }
    #[cfg(not(windows))]
    let _ = (v1, v2, v3, v4);
    0
}

// --- stdout / stdin ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_console_write_utf8(text: *const u8, len: i32, finish_on_new_line: i32) {
    stdio::write_utf8(bytes(text, len), finish_on_new_line != 0)
}
#[unsafe(no_mangle)]
pub extern "C" fn qb_console_cursor(visible: i32, size: i32, passed: i32) {
    stdio::console_cursor(visible, size, passed)
}
#[unsafe(no_mangle)]
pub extern "C" fn qb_console_input_event() -> i32 {
    stdio::console_input()
}
#[unsafe(no_mangle)]
pub extern "C" fn qb_console_cinp(toggle: i32, passed: i32) -> i32 {
    stdio::cinp(toggle, passed)
}

// --- keyboard ---

/// INPUT / LINE INPUT (`qbs_input`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_console_input(offs: *mut *mut c_void, types: *mut i32, n: i32, newline: i32) {
    input::input(&mut CHost::get(), offs, types, n, newline != 0)
}
/// INKEY$: writes 0-2 bytes to `out`, returns the count.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_console_inkey(out: *mut u8) -> i32 {
    let mut h = CHost::get();
    let b = h.bios();
    match keyboard::pop(b) {
        Some(k) => {
            core::ptr::copy_nonoverlapping(k.as_ptr(), out, k.len());
            k.len() as i32
        }
        None => 0,
    }
}
/// Adds a key to the INKEY$ buffer (`bios` = cmem+0x400).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_inkey_push(bios: *mut u8, b1: u8, b2: u8) {
    keyboard::push(bios, b1, b2)
}
/// Empties the INKEY$ buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_inkey_clear(bios: *mut u8) {
    keyboard::clear(bios)
}
/// Flushes the Windows console input buffer (`_KEYCLEAR`).
#[unsafe(no_mangle)]
pub extern "C" fn qb_console_flush_input() {
    stdio::flush_input()
}
/// INPUT$(n) from the keyboard into `out` (`n` bytes). Error 52 if n < 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_console_input_n(n: i32, out: *mut u8) {
    let v = keyboard::input_n(&mut CHost::get(), n);
    if !v.is_empty() {
        core::ptr::copy_nonoverlapping(v.as_ptr(), out, v.len());
    }
}
