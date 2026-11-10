//! Console unit tests against a mock [`Host`]. Expected values follow the
//! legacy libqb.cpp behaviour (verified against the verbatim copy by
//! `tests/diff/diff_console.cpp`).

use super::*;
use crate::gfx::ImgStruct;
use std::collections::VecDeque;
use std::sync::Mutex;

/// Console state is global (LPRINT, _CONTROLCHR, UTF-8 cell tables): run serially.
static SERIAL: Mutex<()> = Mutex::new(());

pub struct Surface {
    pub im: Box<ImgStruct>,
    pub buf: Vec<u8>,
    pub freed: bool,
}

pub struct Mock {
    pub s: Vec<Surface>, // handle -i => s[i], s[0] unused
    pub dest: usize,
    pub source: usize,
    pub base: i32,
    pub errors: Vec<i32>,
    pub custom: Vec<u32>,
    pub bios: Vec<u8>,
    pub stdin: VecDeque<u8>,
    pub switch8050: bool,
    pub switched: bool,
    pub keyredraw: i32,
    pub lprint_pages: i32,
    pub autodisplay: i32,
    pub key_script: VecDeque<(u8, u8)>,
}

impl Mock {
    pub fn new(base: i32) -> Mock {
        let mut bios = vec![0u8; 256];
        bios[0x1a] = 30;
        bios[0x1c] = 30;
        let dummy = Surface { im: Box::default(), buf: vec![], freed: true };
        Mock {
            s: vec![dummy],
            dest: 0,
            source: 0,
            base,
            errors: vec![],
            custom: vec![],
            bios,
            stdin: VecDeque::new(),
            switch8050: false,
            switched: false,
            keyredraw: 0,
            lprint_pages: 0,
            autodisplay: 1,
            key_script: VecDeque::new(),
        }
    }
    fn add(&mut self, mut im: ImgStruct, bytes: usize) -> usize {
        let mut buf = vec![0u8; bytes];
        im.offset = buf.as_mut_ptr();
        im.valid = 1;
        self.s.push(Surface { im: Box::new(im), buf, freed: false });
        let i = self.s.len() - 1;
        if self.dest == 0 {
            self.dest = i;
            self.source = i;
        }
        if self.s[i].im.text != 0 {
            cells::init(self.base + i as i32, (self.s[i].im.width as usize) * (self.s[i].im.height as usize));
        }
        i
    }
    pub fn text(&mut self, w: u16, h: u16) -> usize {
        let im = ImgStruct {
            text: 1,
            width: w,
            height: h,
            bytes_per_pixel: 2,
            color: 7,
            background_color: 0,
            font: 16,
            top_row: 1,
            bottom_row: h as i16,
            cursor_x: 1,
            cursor_y: 1,
            mask: 15,
            draw_scale: 1.0,
            ..Default::default()
        };
        let i = self.add(im, w as usize * h as usize * 2);
        // a fresh text surface is spaces with attribute 7
        for c in self.s[i].buf.chunks_mut(2) {
            c[0] = 32;
            c[1] = 7;
        }
        i
    }
    pub fn gfx8(&mut self, w: u16, h: u16, font: u32) -> usize {
        let fh = match font {
            8 => 8,
            14 => 14,
            _ => 16,
        };
        let im = ImgStruct {
            width: w,
            height: h,
            bytes_per_pixel: 1,
            bits_per_pixel: 8,
            compatible_mode: 13,
            color: 15,
            background_color: 0,
            font,
            top_row: 1,
            bottom_row: (h as i32 / fh) as i16,
            cursor_x: 1,
            cursor_y: 1,
            mask: 255,
            view_x2: w as i32 - 1,
            view_y2: h as i32 - 1,
            print_mode: 3,
            draw_scale: 1.0,
            ..Default::default()
        };
        self.add(im, w as usize * h as usize)
    }
    pub fn w(&mut self) -> &mut ImgStruct {
        &mut self.s[self.dest].im
    }
    pub fn row(&self, i: usize, y: usize) -> String {
        let im = &self.s[i].im;
        let w = im.width as usize;
        self.s[i].buf[(y - 1) * w * 2..y * w * 2].chunks(2).map(|c| c[0] as char).collect::<String>()
    }
    pub fn attr(&self, i: usize, x: usize, y: usize) -> u8 {
        let w = self.s[i].im.width as usize;
        self.s[i].buf[((y - 1) * w + x - 1) * 2 + 1]
    }
}

impl Host for Mock {
    fn write_page(&mut self) -> *mut ImgStruct {
        &mut *self.s[self.dest].im
    }
    fn read_page(&mut self) -> *mut ImgStruct {
        &mut *self.s[self.source].im
    }
    fn image_index(&mut self, im: *const ImgStruct) -> i32 {
        for (i, s) in self.s.iter().enumerate() {
            if core::ptr::eq(&*s.im, im) {
                return self.base + i as i32;
            }
        }
        -1
    }
    fn new_error(&mut self) -> bool {
        false
    }
    fn error(&mut self, n: i32) {
        self.errors.push(n)
    }
    fn font_width(&mut self, f: u32) -> i32 {
        if f >= 32 {
            0
        } else {
            8
        }
    }
    fn font_height(&mut self, f: u32) -> i32 {
        match f {
            8 => 8,
            14 => 14,
            _ => 16,
        }
    }
    fn font_flags(&mut self, _f: u32) -> i32 {
        0
    }
    fn printchr_custom(&mut self, ch: u32) {
        self.custom.push(ch)
    }
    fn chrwidth_custom(&mut self, _ch: u32) -> i32 {
        7
    }
    fn printwidth(&mut self, text: &[u8]) -> i32 {
        text.len() as i32 * 7
    }
    fn lprint_print_page(&mut self) {
        self.lprint_pages += 1
    }
    fn key_redraw(&mut self) {
        self.keyredraw += 1
    }
    fn take_width8050switch(&mut self) -> bool {
        core::mem::take(&mut self.switch8050)
    }
    fn width_80x50(&mut self) {
        self.switched = true;
        let w = self.w();
        w.height = 50;
        w.bottom_row = 50;
    }
    fn screen_pages(&mut self) -> Vec<*mut ImgStruct> {
        vec![]
    }
    fn dest(&mut self) -> i32 {
        -(self.dest as i32)
    }
    fn set_dest(&mut self, h: i32) {
        self.dest = (-h) as usize
    }
    fn source(&mut self) -> i32 {
        -(self.source as i32)
    }
    fn set_source(&mut self, h: i32) {
        self.source = (-h) as usize
    }
    fn copyimage(&mut self, h: i32) -> i32 {
        let src = &self.s[(-h) as usize];
        let im = *src.im;
        let len = src.buf.len();
        let data = src.buf.clone();
        let i = self.add(im, len);
        self.s[i].buf.copy_from_slice(&data);
        -(i as i32)
    }
    fn freeimage(&mut self, h: i32) {
        self.s[(-h) as usize].freed = true
    }
    fn pcopy(&mut self, src: i32, dst: i32) {
        let data = self.s[(-src) as usize].buf.clone();
        self.s[(-dst) as usize].buf.copy_from_slice(&data);
    }
    fn image(&mut self, h: i32) -> *mut ImgStruct {
        &mut *self.s[(-h) as usize].im
    }
    fn autodisplay_begin(&mut self) -> i32 {
        let p = self.autodisplay;
        self.autodisplay = 1;
        p
    }
    fn autodisplay_end(&mut self, previous: i32) {
        if previous == 0 {
            self.autodisplay = 0
        }
    }
    fn stop_program(&mut self) -> bool {
        false
    }
    fn bios(&mut self) -> *mut u8 {
        // feed one scripted key whenever the buffer is empty
        if self.bios[0x1a] == self.bios[0x1c]
            && let Some((a, b)) = self.key_script.pop_front()
        {
            let p = self.bios.as_mut_ptr();
            unsafe { keyboard::push(p, a, b) };
        }
        self.bios.as_mut_ptr()
    }
    fn stdin_byte(&mut self) -> Option<u8> {
        self.stdin.pop_front()
    }
    fn events(&mut self) {}
    fn sleep(&mut self, _ms: u32) {}
    fn ticks(&mut self) -> i64 {
        0
    }
}

fn lock() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

/// The string engine is one global allocator shared with the qbs tests.
fn qbs_lock() -> std::sync::MutexGuard<'static, ()> {
    let g = crate::qbs::tests::LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { crate::qbs::qb_qbs_reset() };
    g
}

fn qstr(q: *mut crate::qbs::Qbs) -> Vec<u8> {
    unsafe {
        let q = &*q;
        if q.len <= 0 {
            return vec![];
        }
        std::slice::from_raw_parts(q.chr, q.len as usize).to_vec()
    }
}

#[test]
fn print_text_cells_and_cursor() {
    let _g = lock();
    let mut m = Mock::new(1000);
    let t = m.text(80, 25);
    m.w().color = 14;
    m.w().background_color = 1;
    unsafe { text::print(&mut m, b"AB", false) };
    assert!(m.row(t, 1).starts_with("AB "));
    assert_eq!(m.attr(t, 1, 1), 14 + 16);
    assert_eq!((m.w().cursor_x, m.w().cursor_y), (3, 1));
    unsafe { text::print(&mut m, b"", true) };
    assert_eq!((m.w().cursor_x, m.w().cursor_y), (1, 2));
    // blink bit 16 -> attribute bit 7
    m.w().color = 16 + 2;
    unsafe { text::print(&mut m, b"X", true) };
    assert_eq!(m.attr(t, 1, 2), 2 + 16 + 128);
}

#[test]
fn print_holds_cursor_at_end_of_line() {
    let _g = lock();
    let mut m = Mock::new(1100);
    m.text(10, 5);
    unsafe { text::print(&mut m, b"0123456789", false) };
    assert_eq!((m.w().cursor_x, m.w().cursor_y, m.w().holding_cursor), (10, 1, 1));
    assert_eq!(unsafe { text::pos(&mut m) }, 1);
    assert_eq!(unsafe { text::csrlin(&mut m) }, 2);
    unsafe { text::print(&mut m, b"A", false) };
    assert_eq!((m.w().cursor_x, m.w().cursor_y, m.w().holding_cursor), (2, 2, 0));
    // a full line with newline does not add a blank line
    unsafe { text::print(&mut m, b"", true) };
    unsafe { text::print(&mut m, b"0123456789", true) };
    assert_eq!((m.w().cursor_x, m.w().cursor_y), (1, 4));
}

#[test]
fn newline_scrolls_view_print_window() {
    let _g = lock();
    let mut m = Mock::new(1200);
    let t = m.text(4, 5);
    unsafe {
        text::view_print(&mut m, 2, 4, 1);
        assert_eq!((m.w().cursor_x, m.w().cursor_y), (1, 2));
        for s in [&b"aa"[..], b"bb", b"cc", b"dd"] {
            text::print(&mut m, s, true);
        }
    }
    assert_eq!(m.row(t, 1), "    ");
    assert_eq!(m.row(t, 2), "cc  ");
    assert_eq!(m.row(t, 3), "dd  ");
    assert_eq!(m.row(t, 4), "    ");
    assert_eq!(m.w().cursor_y, 4);
    unsafe { text::view_print(&mut m, 4, 2, 1) };
    assert_eq!(m.errors, vec![5]);
    unsafe { text::view_print(&mut m, 0, 0, 0) };
    assert_eq!((m.w().top_row, m.w().bottom_row), (1, 5));
}

#[test]
fn comma_zones_and_control_characters() {
    let _g = lock();
    let mut m = Mock::new(1300);
    let t = m.text(80, 25);
    unsafe {
        text::print(&mut m, b"A", false);
        text::tab(&mut m);
    }
    assert_eq!(m.w().cursor_x, 15);
    unsafe { text::print(&mut m, b"\x1c\x1c", false) };
    assert_eq!(m.w().cursor_x, 17);
    unsafe { text::print(&mut m, b"\x1d", false) };
    assert_eq!(m.w().cursor_x, 16);
    unsafe { text::print(&mut m, b"\x1f\x1f\x1e", false) };
    assert_eq!(m.w().cursor_y, 2);
    unsafe { text::print(&mut m, b"\x0b", false) };
    assert_eq!((m.w().cursor_x, m.w().cursor_y), (1, 1));
    unsafe { text::print(&mut m, b"x\ty", false) };
    assert_eq!(m.w().cursor_x, 10);
    assert_eq!(&m.row(t, 1)[..9], "x       y");
    unsafe {
        no_control_characters2 = 1;
        text::print(&mut m, b"\x0d", false);
        no_control_characters2 = 0;
    }
    assert_eq!(m.s[t].buf[9 * 2], 13);
    unsafe { text::print(&mut m, b"\x0c", false) };
    assert_eq!(m.row(t, 1), " ".repeat(80));
    assert_eq!(m.keyredraw, 1);
}

#[test]
fn wide_and_zero_width_characters() {
    let _g = lock();
    let mut m = Mock::new(1400);
    let t = m.text(4, 3);
    unsafe { text::print(&mut m, "a一".as_bytes(), false) };
    assert_eq!(m.w().cursor_x, 4);
    // legacy maps characters outside code page 437 to byte 0 (its '?' fallback is unreachable)
    assert_eq!(m.s[t].buf[2], 0);
    assert_eq!(cells::span(1400 + t as i32, 1), 2);
    unsafe { text::print(&mut m, "一".as_bytes(), false) };
    // does not fit: new line first
    assert_eq!((m.w().cursor_x, m.w().cursor_y), (3, 2));
    unsafe { text::print(&mut m, "\u{301}".as_bytes(), false) };
    assert_eq!(cells::content(1400 + t as i32, 4), "一\u{301}".as_bytes());
}

#[test]
fn locate_validation_and_hold() {
    let _g = lock();
    let mut m = Mock::new(1500);
    m.text(80, 25);
    unsafe {
        text::locate(&mut m, 5, 10, 0, 0, 0, 3);
        assert_eq!((m.w().cursor_x, m.w().cursor_y), (10, 5));
        text::locate(&mut m, 0, 81, 0, 0, 0, 2);
        text::locate(&mut m, 26, 1, 0, 0, 0, 1);
        text::locate(&mut m, 0, 0, 2, 0, 0, 4);
        text::locate(&mut m, 0, 0, 0, 32, 0, 8);
        assert_eq!(m.errors, vec![5, 5, 5, 5]);
        text::locate(&mut m, 0, 0, 1, 3, 7, 4 + 8 + 16);
        assert_eq!((m.w().cursor_show, m.w().cursor_firstvalue, m.w().cursor_lastvalue), (1, 3, 7));
        m.w().holding_cursor = 1;
        text::locate(&mut m, 0, 0, 0, 0, 0, 0);
        assert_eq!(m.w().holding_cursor, 2);
        // row 25 is allowed outside VIEW PRINT
        text::view_print(&mut m, 1, 10, 1);
        text::locate(&mut m, 25, 1, 0, 0, 0, 1);
        assert_eq!(m.w().cursor_y, 25);
        text::locate(&mut m, 11, 1, 0, 0, 0, 1);
        assert_eq!(m.errors.len(), 5);
        // first LOCATE below 25 switches to 80x50
        text::view_print(&mut m, 0, 0, 0);
        m.switch8050 = true;
        text::locate(&mut m, 40, 3, 0, 0, 0, 3);
    }
    assert!(m.switched);
    assert_eq!((m.w().cursor_x, m.w().cursor_y), (3, 40));
}

#[test]
fn cls_text_methods() {
    let _g = lock();
    let mut m = Mock::new(1600);
    let t = m.text(4, 4);
    unsafe {
        for s in [&b"aaaa"[..], b"bbbb", b"cccc", b"dddd"] {
            text::print(&mut m, s, false);
        }
        text::view_print(&mut m, 2, 3, 1);
        m.w().color = 4;
        text::cls(&mut m, 2, 0, 1);
    }
    assert_eq!(m.row(t, 1), "aaaa");
    assert_eq!(m.row(t, 2), "    ");
    assert_eq!(m.attr(t, 1, 2), 4);
    assert_eq!(m.row(t, 4), "dddd");
    assert_eq!(m.keyredraw, 0);
    unsafe { text::cls(&mut m, 0, 0, 0) };
    // no method: view port AND bottom line
    assert_eq!(m.row(t, 1), "aaaa");
    assert_eq!(m.row(t, 4), "    ");
    assert_eq!(m.keyredraw, 1);
    unsafe { text::cls(&mut m, 0, 2, 3) };
    assert_eq!(m.row(t, 1), "    ");
    assert_eq!(m.attr(t, 1, 1), 4 + 32);
    unsafe { text::cls(&mut m, 3, 0, 1) };
    unsafe { text::cls(&mut m, 0, 16, 2) };
    assert_eq!(m.errors, vec![5, 5]);
    assert_eq!((m.w().cursor_x, m.w().cursor_y), (1, 2));
}

#[test]
fn graphics_print_and_cls() {
    let _g = lock();
    let mut m = Mock::new(1700);
    let g = m.gfx8(64, 32, 8);
    unsafe { text::print(&mut m, b"\xdb", false) };
    // full block glyph: first row of the cell is ink
    assert_eq!(&m.s[g].buf[0..8], &[15u8; 8]);
    assert_eq!(m.w().cursor_x, 2);
    unsafe { text::print(&mut m, b"1234567", false) };
    assert_eq!((m.w().cursor_x, m.w().holding_cursor), (8, 1));
    unsafe { text::print(&mut m, b"", true) };
    assert_eq!((m.w().cursor_x, m.w().cursor_y), (1, 2));
    // scrolling moves pixel rows of whole text lines
    unsafe { text::print(&mut m, b"\n\n\n", false) };
    assert_eq!(m.w().cursor_y, 4);
    assert_eq!(&m.s[g].buf[0..8], &[0u8; 8]);
    unsafe { text::locate(&mut m, 1, 1, 0, 0, 0, 3) };
    unsafe { text::print(&mut m, b"\xdb", false) };
    assert_eq!(unsafe { text::screen(&mut m, 1, 1, 0, 0) }, 0xdb);
    assert_eq!(unsafe { text::screen(&mut m, 1, 1, 1, 1) }, 15);
    assert_eq!(unsafe { text::screen(&mut m, 1, 2, 1, 1) }, 1);
    m.w().background_color = 3;
    unsafe { text::cls(&mut m, 0, 0, 0) };
    assert!(m.s[g].buf.iter().all(|&b| b == 3));
    assert_eq!((m.w().x, m.w().y), (32.0, 16.0));
}

#[test]
fn variable_width_font_uses_host() {
    let _g = lock();
    let mut m = Mock::new(1800);
    m.gfx8(64, 32, 8);
    m.w().font = 32;
    unsafe { text::print(&mut m, b"ab", false) };
    assert_eq!(m.custom, vec![97, 98]);
    assert_eq!(m.w().cursor_x, 15);
    unsafe { text::tab(&mut m) };
    assert_eq!((m.w().cursor_x, m.w().cursor_y), (1, 2));
}

#[test]
fn screen_function_text() {
    let _g = lock();
    let mut m = Mock::new(1900);
    m.text(80, 25);
    m.w().color = 2;
    unsafe { text::print(&mut m, b"Q", false) };
    assert_eq!(unsafe { text::screen(&mut m, 1, 1, 0, 0) }, b'Q' as u32);
    assert_eq!(unsafe { text::screen(&mut m, 1, 1, 1, 1) }, 2);
    assert_eq!(unsafe { text::screen(&mut m, 26, 1, 0, 0) }, 0);
    assert_eq!(m.errors, vec![5]);
}

#[test]
fn inkey_ring_buffer() {
    let mut bios = vec![0u8; 256];
    let p = bios.as_mut_ptr();
    unsafe {
        keyboard::clear(p);
        assert_eq!(keyboard::pop(p), None);
        keyboard::push(p, b'a', 0x1e);
        keyboard::push(p, 0, 0x48);
        keyboard::push(p, 0, 0);
        assert_eq!(keyboard::pop(p), Some(vec![b'a']));
        assert_eq!(keyboard::pop(p), Some(vec![0, 0x48]));
        assert_eq!(keyboard::pop(p), Some(vec![0]));
        // 15 keys fit, the 16th is dropped
        for i in 0..16 {
            keyboard::push(p, b'0' + i, 0);
        }
        let mut n = 0;
        while keyboard::pop(p).is_some() {
            n += 1;
        }
        assert_eq!(n, 15);
    }
}

fn run_input(m: &mut Mock, keys: &str, types: &mut [i32], offs: &mut [*mut core::ffi::c_void]) {
    for b in keys.bytes() {
        m.key_script.push_back((b, 0));
    }
    unsafe { input::input(m, offs.as_mut_ptr(), types.as_mut_ptr(), (types.len() - 1) as i32, true) };
}

#[test]
fn input_numbers_and_strings() {
    let _g = lock();
    let _q = qbs_lock();
    let mut m = Mock::new(2000);
    let t = m.text(40, 5);
    let mut a: i16 = 99;
    let s = unsafe { crate::qbs::qb_qbs_new(0, 0) };
    let mut d: f64 = 0.0;
    let mut types = [0, 16, input::ISSTRING, input::ISFLOAT | 64];
    let mut offs = [
        core::ptr::null_mut(),
        &mut a as *mut i16 as *mut core::ffi::c_void,
        s as *mut core::ffi::c_void,
        &mut d as *mut f64 as *mut core::ffi::c_void,
    ];
    // 'x' is rejected for the INTEGER (typed then removed again)
    run_input(&mut m, "-12x,\"hi, there\",2.5e1\r", &mut types, &mut offs);
    assert_eq!(a, -12);
    assert_eq!(qstr(s), b"hi, there");
    assert_eq!(d, 25.0);
    assert_eq!(m.row(t, 1).trim_end(), "-12,\"hi, there\",2.5e1");
    assert_eq!((m.w().cursor_x, m.w().cursor_y), (1, 2));
}

#[test]
fn input_hex_overflow_and_line_input() {
    let _g = lock();
    let _q = qbs_lock();
    let mut m = Mock::new(2100);
    let t = m.text(40, 5);
    let mut b: u8 = 0;
    let mut types = [0, 8 | input::ISUNSIGNED];
    let mut offs = [core::ptr::null_mut(), &mut b as *mut u8 as *mut core::ffi::c_void];
    // &H1FF does not fit a _UNSIGNED _BYTE: the 3rd digit is refused
    run_input(&mut m, "&HFFF\r", &mut types, &mut offs);
    assert_eq!(b, 255);
    assert_eq!(m.row(t, 1).trim_end(), "&HFF");
    let s = unsafe { crate::qbs::qb_qbs_new(0, 0) };
    let mut types = [0, input::ISSTRING | 512];
    let mut offs = [core::ptr::null_mut(), s as *mut core::ffi::c_void];
    run_input(&mut m, "a,\"b\x08c\r", &mut types, &mut offs);
    assert_eq!(qstr(s), b"a,\"c");
}

#[test]
fn input_validation_rules() {
    use input::{validate, Check, ISFLOAT, ISUNSIGNED};
    assert_eq!(validate(b"32767", 16), Check::Valid);
    assert_eq!(validate(b"32768", 16), Check::Invalid);
    assert_eq!(validate(b"-32768", 16), Check::Valid);
    assert_eq!(validate(b"01", 16), Check::Invalid);
    assert_eq!(validate(b"-", 16), Check::Valid);
    assert_eq!(validate(b"&O777", 8), Check::Invalid);
    assert_eq!(validate(b"255", 8 | ISUNSIGNED), Check::Valid);
    assert_eq!(validate(b"-1", 8 | ISUNSIGNED), Check::Invalid);
    assert_eq!(validate(b"9223372036854775807", 64), Check::Valid);
    assert_eq!(validate(b"9223372036854775808", 64), Check::Invalid);
    assert_eq!(validate(b"18446744073709551615", 64 | ISUNSIGNED), Check::Valid);
    assert_eq!(validate(b"1.5d3", 32 | ISFLOAT), Check::Invalid);
    assert_eq!(validate(b"1.5d3", 64 | ISFLOAT), Check::Valid);
    assert_eq!(validate(b"3.402823E+38", 32 | ISFLOAT), Check::Valid);
    assert_eq!(validate(b"3.402824E+38", 32 | ISFLOAT), Check::Invalid);
    assert_eq!(validate(b"1.23456789", 32 | ISFLOAT), Check::Invalid);
    assert_eq!(validate(b"00", 64 | ISFLOAT), Check::Invalid);
    assert_eq!(validate(b"0.", 64 | ISFLOAT), Check::Valid);
}

#[test]
fn input_dollar_from_keyboard() {
    let _g = lock();
    let mut m = Mock::new(2200);
    m.text(10, 3);
    for (a, b) in [(b'x', 0x2d), (0, 0x48), (b'y', 0x15)] {
        m.key_script.push_back((a, b));
    }
    let r = unsafe { keyboard::input_n(&mut m, 3) };
    assert_eq!(r, vec![b'x', 0, b'y']);
}
