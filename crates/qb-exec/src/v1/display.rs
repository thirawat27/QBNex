//! Screen pages, image handles and the graphics/console statements of the
//! v1 interpreter. Ported from the parts of libqb.cpp that still owned this
//! state (`qbg_screen`, `validatepage`, `func__newimage`, `sub__dest`, the
//! one-line graphics shims, `end()`'s capture); the surfaces, drawing, text
//! console and capture are qb-runtime (`gfx`, `console`).

use super::{Stop, Vm, R, V};
use qb_runtime::console::{self, Host};
use qb_runtime::gfx::{self, ImgStruct, IMG_FREEPAL, IMG_SCREEN};
use qb_runtime::gfx::registry as reg;

/// Byte offsets of the video memory mirrors in conventional memory.
const CMEM_A000: usize = 655_360;
const CMEM_B800: usize = 753_664;

pub(crate) struct Display {
    pub page: Vec<i32>,
    pub write_index: u32,
    pub read_index: u32,
    pub display_index: u32,
    pub console_image: i32,
    pub autodisplay: i32,
    pub width8050switch: bool,
    pub sub_screen_width: i32,
    pub sub_screen_height: i32,
    pub sub_screen_font: i32,
    pub sub_screen_keep_page0: bool,
    pub last_active_page: i32,
    pub fontwidth: [i32; 64],
    pub fontheight: [i32; 64],
    pub fontflags: [i32; 64],
    /// error raised through the console Host
    pub pending: i32,
    pub started: bool,
}

impl Default for Display {
    fn default() -> Self {
        let mut d = Display {
            page: vec![0],
            write_index: 0,
            read_index: 0,
            display_index: 0,
            console_image: -1,
            autodisplay: 1,
            width8050switch: true,
            sub_screen_width: -1,
            sub_screen_height: -1,
            sub_screen_font: -1,
            sub_screen_keep_page0: false,
            last_active_page: 0,
            fontwidth: [0; 64],
            fontheight: [0; 64],
            fontflags: [0; 64],
            pending: 0,
            started: false,
        };
        d.fontwidth[8] = 8;
        d.fontwidth[14] = 8;
        d.fontwidth[16] = 8;
        d.fontheight[8] = 8;
        d.fontheight[14] = 14;
        d.fontheight[16] = 16;
        // builtin fonts are monospace
        for f in [8, 9, 14, 15, 16, 17] {
            d.fontflags[f] = 16;
        }
        d
    }
}

#[inline]
fn im(i: u32) -> *mut ImgStruct {
    // SAFETY: indexes come from the registry; `img` is its base.
    unsafe { reg::img.add(i as usize) }
}

fn nextimg() -> i32 {
    // SAFETY: registry global, program thread only.
    unsafe { reg::nextimg }
}

impl Display {
    fn err(&mut self, n: i32) {
        if self.pending == 0 {
            self.pending = n;
        }
    }
    fn wp(&self) -> *mut ImgStruct {
        im(self.write_index)
    }
    fn rp(&self) -> *mut ImgStruct {
        im(self.read_index)
    }
    fn dp(&self) -> *mut ImgStruct {
        im(self.display_index)
    }
    fn select_all(&mut self, i: u32) {
        self.write_index = i;
        self.read_index = i;
        self.display_index = i;
    }
    fn ensure_pages(&mut self, n: usize) {
        if self.page.len() < n {
            self.page.resize(n, 0);
        }
    }

    fn imgnew(&mut self, x: i32, y: i32, bpp: i32) -> i32 {
        let mut e = 0;
        // SAFETY: registry call.
        let i = unsafe { reg::qb_imgnew(x, y, bpp, &mut e) };
        if e != 0 {
            self.err(e);
        }
        if i != 0 {
            self.text_cells(i);
        }
        i
    }
    fn imgframe(&mut self, o: *mut u8, x: i32, y: i32, bpp: i32) -> i32 {
        let mut e = 0;
        // SAFETY: `o` addresses VM conventional memory large enough for the frame.
        let i = unsafe { reg::qb_imgframe(o, x, y, bpp, &mut e) };
        if e != 0 {
            self.err(e);
        }
        if i != 0 {
            self.text_cells(i);
        }
        i
    }
    fn text_cells(&mut self, i: i32) {
        let p = im(i as u32);
        // SAFETY: valid surface.
        unsafe {
            if (*p).text != 0 {
                console::cells::qb_text_cells_init(i, i32::from((*p).width) * i32::from((*p).height));
            } else {
                console::cells::qb_text_cells_release(i);
            }
        }
    }
    fn freeimg(&mut self, i: u32) {
        // SAFETY: registry call on a valid index.
        unsafe {
            if i as i32 >= reg::nimg || (*im(i)).valid == 0 {
                return;
            }
            if (*im(i)).lock_id != 0 {
                qb_runtime::mem::free_mem_lock((*im(i)).lock_offset as *mut qb_runtime::mem::MemLock);
            }
        }
        console::cells::qb_text_cells_release(i as i32);
        for p in self.page.iter_mut() {
            if *p == i as i32 {
                *p = 0;
            }
        }
        reg::qb_img_free(i);
    }
    fn selectfont(&self, f: i32, p: *mut ImgStruct) {
        // SAFETY: valid surface.
        unsafe {
            (*p).font = f as u32;
            (*p).cursor_x = 1;
            (*p).cursor_y = 1;
            (*p).top_row = 1;
            let fh = self.fontheight[f as usize].max(1);
            (*p).bottom_row = if (*p).compatible_mode != 0 { ((*p).height as i32 / fh) as i16 } else { (*p).height as i16 };
            (*p).bottom_row -= 1;
            if (*p).bottom_row <= 0 {
                (*p).bottom_row = 1;
            }
        }
    }

    pub(crate) fn validatepage(&mut self, n: i32) {
        if n < 0 {
            return;
        }
        let n = n as usize;
        self.ensure_pages(n + 1);
        if self.page[n] == 0 {
            let i = self.page[0] as u32;
            let p0 = im(i);
            // SAFETY: page 0 is a valid surface.
            let (w, h, m, pal, font) = unsafe { ((*p0).width, (*p0).height, (*p0).compatible_mode, (*p0).pal, (*p0).font) };
            let i2 = self.imgnew(i32::from(w), i32::from(h), i32::from(m));
            let p2 = im(i2 as u32);
            // SAFETY: fresh surface.
            unsafe {
                if (*p2).bytes_per_pixel != 4 {
                    reg::qb_img_release_palette(i2);
                    (*p2).flags ^= IMG_FREEPAL;
                    reg::qb_img_set_external_palette(i2, pal);
                }
                (*p2).flags |= IMG_SCREEN;
            }
            self.selectfont(font as i32, p2);
            self.page[n] = i2;
        }
    }

    /// `resolve_page_or_image_index`
    pub(crate) fn resolve(&mut self, handle: i32) -> Option<u32> {
        if handle >= 0 {
            self.validatepage(handle);
            return Some(self.page[handle as usize] as u32);
        }
        let h = -handle;
        // SAFETY: registry read.
        if h >= nextimg() || unsafe { (*im(h as u32)).valid } == 0 {
            self.err(258);
            return None;
        }
        Some(h as u32)
    }

    /// Program start: SCREEN 0, the console image and the two icon images.
    pub(crate) fn start(&mut self, cmem: *mut u8, console_on: bool, console_only: bool) {
        if self.started {
            return;
        }
        self.started = true;
        reg::qb_img_init();
        self.screen(cmem, 0, 0, 0, 0, 0, 1);
        self.width8050switch = true;
        if console_on {
            self.console_image = self.newimage(80, 25, 0, 0);
            let i = -self.console_image;
            // SAFETY: fresh surface.
            unsafe { (*im(i as u32)).console = 1 };
        }
        // icon images (16x16 and 32x32), so image handles match the C++ backend
        self.newimage(16, 16, 32, 1);
        self.newimage(32, 32, 32, 1);
        if console_only && self.console_image < 0 {
            if let Some(x) = self.resolve(self.console_image) {
                self.write_index = x;
            }
        }
    }

    pub(crate) fn newimage(&mut self, x: i32, y: i32, mut bpp: i32, passed: i32) -> i32 {
        if x <= 0 || y <= 0 {
            self.err(5);
            return 0;
        }
        let w = self.wp();
        if passed == 0 {
            // SAFETY: write page is valid.
            bpp = i32::from(unsafe { (*w).compatible_mode });
        } else if !((0..=2).contains(&bpp) || (7..=13).contains(&bpp) || bpp == 256 || bpp == 32) {
            self.err(5);
            return 0;
        }
        let i = self.imgnew(x, y, bpp);
        if i == 0 {
            return -1;
        }
        if passed == 0 {
            let w = self.wp();
            let n = im(i as u32);
            // SAFETY: both surfaces valid.
            unsafe {
                if !(*w).pal.is_null() && !(*n).pal.is_null() {
                    core::ptr::copy_nonoverlapping((*w).pal, (*n).pal, 256);
                }
                let f = (*w).font as i32;
                self.font(f, -i, 1);
                (*n).color = (*w).color;
                (*n).background_color = (*w).background_color;
                (*n).transparent_color = (*w).transparent_color;
                (*n).alpha_disabled = (*w).alpha_disabled;
                (*n).print_mode = (*w).print_mode;
            }
        }
        -i
    }

    pub(crate) fn font(&mut self, f: i32, i: i32, passed: i32) {
        let idx = if passed & 1 != 0 {
            match self.resolve(i) {
                Some(x) => x,
                None => return,
            }
        } else {
            self.write_index
        };
        let ok = matches!(f, 8 | 9 | 14 | 15 | 16 | 17) || ((32..64).contains(&f) && self.fontwidth[f as usize] != 0);
        if !ok {
            self.err(258);
            return;
        }
        let p = im(idx);
        // SAFETY: valid surface.
        unsafe {
            if (*p).text != 0 && self.fontflags[f as usize] & 16 == 0 {
                self.err(5);
                return;
            }
            if (*p).text != 0 && (*p).flags & IMG_SCREEN != 0 {
                for &pg in &self.page {
                    if pg != 0 {
                        (*im(pg as u32)).font = f as u32;
                    }
                }
                return;
            }
        }
        self.selectfont(f, p);
    }

    pub(crate) fn freeimage(&mut self, i: i32, passed: i32) {
        let idx = if passed != 0 {
            if i >= 0 {
                self.err(5);
                return;
            }
            match self.resolve(i) {
                Some(x) => x,
                None => return,
            }
        } else {
            self.write_index
        };
        // SAFETY: valid surface.
        if unsafe { (*im(idx)).flags } & IMG_SCREEN != 0 {
            self.err(5);
            return;
        }
        if self.write_index == idx {
            self.write_index = self.display_index;
        }
        if self.read_index == idx {
            self.read_index = self.display_index;
        }
        reg::qb_img_release_owned(idx as i32);
        self.freeimg(idx);
    }

    pub(crate) fn copyimage(&mut self, i: i32, mode: i32, passed: i32) -> i32 {
        let Some(src) = self.resolve(i) else { return 0 };
        // SAFETY: valid source surface.
        let s = unsafe { *im(src) };
        if passed & 1 != 0 && mode != i32::from(s.compatible_mode) {
            self.err(5);
            return -1;
        }
        let mut e = 0;
        // SAFETY: registry call.
        let i2 = unsafe { reg::qb_img_new(&mut e) };
        if e != 0 {
            self.err(e);
            return -1;
        }
        // SAFETY: fresh slot; the copy follows func__copyimage.
        unsafe {
            let s = &*im(src);
            let d = im(i2);
            *d = *s;
            (*d).lock_id = 0;
            (*d).lock_offset = core::ptr::null_mut();
            let bytes = (*d).width as isize * (*d).height as isize * (*d).bytes_per_pixel as isize;
            reg::qb_img_alloc_pixels(i2 as i32, bytes);
            if (*d).offset.is_null() {
                self.freeimg(i2);
                return -1;
            }
            core::ptr::copy_nonoverlapping((*im(src)).offset, (*d).offset, bytes as usize);
            (*d).flags |= gfx::IMG_FREEMEM;
            if !(*d).pal.is_null() {
                reg::qb_img_alloc_palette(i2 as i32);
                core::ptr::copy_nonoverlapping((*im(src)).pal, (*d).pal, 256);
                (*d).flags |= IMG_FREEPAL;
            }
            if (*d).text != 0 {
                console::cells::qb_text_cells_copy(src as i32, i2 as i32);
            }
            if (*d).flags & IMG_SCREEN != 0 {
                (*d).flags ^= IMG_SCREEN;
            }
        }
        -(i2 as i32)
    }

    pub(crate) fn pcopy(&mut self, src: i32, dst: i32) {
        let Some(s) = self.resolve(src) else {
            self.err(5);
            return;
        };
        let Some(d) = self.resolve(dst) else {
            self.err(5);
            return;
        };
        // SAFETY: both surfaces valid.
        let r = unsafe { gfx::getput::qb_gfx_pcopy(im(s), im(d)) };
        if r != 0 {
            self.err(r);
        }
    }

    /// `qbg_screen`
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn screen(&mut self, cmem: *mut u8, mode: i32, _color_switch: i32, active_page: i32, mut visual_page: i32, _refresh: i32, mut passed: i32) {
        if self.width8050switch && (passed != 1 || mode != 0) {
            self.width8050switch = false;
        }
        let mut i = 0;
        let i2 = self.page[0];
        if passed & 1 != 0 {
            if mode < 0 {
                let i3 = -mode;
                // SAFETY: registry read.
                if i3 >= nextimg() || unsafe { (*im(i3 as u32)).valid } == 0 {
                    self.err(258);
                    return;
                }
                if i3 != i2 {
                    i = 1;
                }
            } else {
                if gfx::screen::qb_gfx_screen_mode_valid(mode) == 0 {
                    self.err(5);
                    return;
                }
                if i2 != 0 {
                    // SAFETY: page 0 surface.
                    if i32::from(unsafe { (*im(i2 as u32)).compatible_mode }) != mode {
                        i = 1;
                    }
                } else {
                    i = 1;
                }
                if self.sub_screen_width != -1 || self.sub_screen_height != -1 || self.sub_screen_font != -1 {
                    i = 1;
                }
            }
        }
        if passed & 4 != 0 {
            if active_page < 0 {
                self.err(5);
                return;
            }
            if passed & 8 == 0 {
                passed |= 8;
                visual_page = active_page;
            }
            if i & 1 == 0 {
                self.validatepage(active_page);
                let i2 = self.page[active_page as usize] as u32;
                if i2 != self.read_index || i2 != self.write_index {
                    i |= 2;
                }
            }
        }
        if passed & 8 != 0 {
            if visual_page < 0 {
                self.err(5);
                return;
            }
            if i & 1 == 0 {
                self.validatepage(visual_page);
                if self.page[visual_page as usize] as u32 != self.display_index {
                    i |= 2;
                }
            }
        }
        if i == 0 {
            return;
        }
        if passed & 1 != 0 && i & 1 != 0 {
            self.set_mode(cmem, mode);
        }
        if passed & 4 != 0 {
            self.validatepage(active_page);
            let i = self.page[active_page as usize] as u32;
            if self.write_index != i || self.read_index != i {
                self.write_index = i;
                self.read_index = i;
                // SAFETY: valid surfaces.
                unsafe {
                    gfx::palette::qb_gfx_defaultcolors(self.wp());
                    self.reset_view_print();
                    if self.last_active_page != active_page {
                        let old = im(self.page[self.last_active_page as usize] as u32);
                        let w = self.wp();
                        if (*w).text == 0 {
                            let a = core::ptr::addr_of_mut!((*w).apm_p1) as *mut u8;
                            let b = core::ptr::addr_of_mut!((*w).apm_p2) as *mut u8;
                            let o = core::ptr::addr_of!((*old).apm_p1) as *const u8;
                            core::ptr::copy(o, a, b.offset_from(a) as usize);
                        }
                        self.last_active_page = active_page;
                    }
                }
            }
        }
        if passed & 8 != 0 {
            self.validatepage(visual_page);
            let i = self.page[visual_page as usize] as u32;
            if self.display_index != i {
                self.display_index = i;
                // SAFETY: valid surface.
                unsafe {
                    gfx::palette::qb_gfx_defaultcolors(self.wp());
                    self.reset_view_print();
                }
            }
        }
    }

    unsafe fn reset_view_print(&self) {
        let w = self.wp();
        // SAFETY: caller passes valid surfaces.
        unsafe {
            (*w).top_row = 1;
            (*w).bottom_row = if (*w).text == 0 {
                ((*w).height as u32 / (*w).font.max(1)) as i16
            } else {
                (*w).height as i16
            };
            if (*w).cursor_y > (*w).bottom_row {
                (*w).cursor_y = 1;
            }
            (*w).bottom_row -= 1;
            if (*w).bottom_row <= 0 {
                (*w).bottom_row = 1;
            }
        }
    }

    fn set_mode(&mut self, cmem: *mut u8, mode: i32) {
        let (mut prev_w, mut prev_h) = (0, 0);
        let p0 = self.page[0];
        if p0 != 0 {
            // SAFETY: page 0 surface.
            unsafe {
                let s = &*im(p0 as u32);
                if s.compatible_mode == 0 {
                    prev_w = i32::from(s.width);
                    prev_h = i32::from(s.height);
                } else {
                    let x = self.fontwidth[s.font as usize].max(1);
                    prev_w = i32::from(s.width) / x;
                    prev_h = i32::from(s.height) / self.fontheight[s.font as usize].max(1);
                }
            }
        }
        if p0 != 0 {
            for k in (1..self.page.len()).rev() {
                let i2 = self.page[k];
                if i2 != 0 {
                    reg::qb_img_release_owned(i2);
                    self.freeimg(i2 as u32);
                }
            }
            if self.sub_screen_keep_page0 {
                // SAFETY: page 0 surface.
                unsafe { (*im(p0 as u32)).flags ^= IMG_SCREEN };
            } else {
                reg::qb_img_release_owned(p0);
                self.freeimg(p0 as u32);
            }
        }
        self.sub_screen_keep_page0 = false;
        self.page = vec![0];
        if mode < 0 {
            let i = -mode as u32;
            self.page[0] = i as i32;
            // SAFETY: validated custom surface.
            unsafe { (*im(i)).flags |= IMG_SCREEN };
            self.select_all(i);
            self.sub_screen_keep_page0 = true;
        }
        if mode > 0 {
            let mut geo = [0i32; 3];
            // SAFETY: geo has 3 slots.
            if unsafe { gfx::screen::qb_gfx_screen_geometry(mode, prev_w, prev_h, geo.as_mut_ptr()) } != 0 {
                let i = if mode == 13 {
                    // SAFETY: the 64 KB VGA window lies inside conventional memory.
                    let o = unsafe { cmem.add(CMEM_A000) };
                    let i = self.imgframe(o, geo[0], geo[1], 13);
                    // SAFETY: as above.
                    unsafe { core::ptr::write_bytes(o, 0, 320 * 200) };
                    i
                } else {
                    let i = self.imgnew(geo[0], geo[1], mode);
                    if geo[2] != 0 {
                        self.selectfont(geo[2], im(i as u32));
                    }
                    i
                };
                self.page[0] = i;
                // SAFETY: fresh surface.
                unsafe { (*im(i as u32)).flags |= IMG_SCREEN };
                self.select_all(i as u32);
            }
        }
        if mode == 0 {
            let mut lay = [0i32; 5];
            let (mut w, mut h, mut f) = (self.sub_screen_width, self.sub_screen_height, self.sub_screen_font);
            // SAFETY: out-pointers.
            unsafe { gfx::screen::qb_gfx_screen0_layout(prev_w, prev_h, &mut w, &mut h, &mut f, lay.as_mut_ptr()) };
            self.sub_screen_width = w;
            self.sub_screen_height = h;
            self.sub_screen_font = f;
            let (x, y, fnt, gran, p) = (lay[0], lay[1], lay[2], lay[3], lay[4]);
            self.ensure_pages(8);
            for i3 in 0..8 {
                let i = if i3 < p {
                    // SAFETY: text pages lie inside conventional memory.
                    self.imgframe(unsafe { cmem.add(CMEM_B800 + (gran * i3) as usize) }, x, y, 0)
                } else {
                    self.imgnew(x, y, 0)
                };
                self.selectfont(fnt, im(i as u32));
                // SAFETY: fresh surface.
                unsafe { (*im(i as u32)).flags |= IMG_SCREEN };
                self.page[i3 as usize] = i;
            }
            for k in (0..65536).step_by(2) {
                // SAFETY: inside conventional memory.
                unsafe {
                    *cmem.add(CMEM_B800 + k) = 32;
                    *cmem.add(CMEM_B800 + k + 1) = 7;
                }
            }
            self.select_all(self.page[0] as u32);
        }
        // SAFETY: valid write page.
        unsafe {
            let w = self.wp();
            (*w).draw_ta = 0.0;
            (*w).draw_scale = 1.0;
        }
        self.last_active_page = 0;
    }

    /// The destination is a surface (not the `$CONSOLE` stdout image).
    pub(crate) fn write_is_screen(&self) -> bool {
        // SAFETY: valid write page.
        unsafe { (*self.wp()).console == 0 }
    }

    /// Frame record at END/SYSTEM (`end()`), or at `_DISPLAY`.
    pub(crate) fn capture(&self, kind: i32) {
        if !self.started {
            return;
        }
        // SAFETY: display page is valid.
        unsafe { gfx::capture::qb_fb_capture(self.dp(), kind) };
    }
}

/// The console module's view of the VM display.
pub(crate) struct VmHost<'a> {
    pub d: &'a mut Display,
    pub cmem: *mut u8,
}

impl Host for VmHost<'_> {
    fn write_page(&mut self) -> *mut ImgStruct {
        self.d.wp()
    }
    fn read_page(&mut self) -> *mut ImgStruct {
        self.d.rp()
    }
    fn image_index(&mut self, p: *const ImgStruct) -> i32 {
        // SAFETY: pointer arithmetic inside the registry.
        unsafe { p.offset_from(reg::img) as i32 }
    }
    fn new_error(&mut self) -> bool {
        self.d.pending != 0
    }
    fn error(&mut self, n: i32) {
        self.d.err(n);
    }
    fn font_width(&mut self, f: u32) -> i32 {
        self.d.fontwidth.get(f as usize).copied().unwrap_or(0)
    }
    fn font_height(&mut self, f: u32) -> i32 {
        self.d.fontheight.get(f as usize).copied().unwrap_or(0)
    }
    fn font_flags(&mut self, f: u32) -> i32 {
        self.d.fontflags.get(f as usize).copied().unwrap_or(0)
    }
    fn printchr_custom(&mut self, _ch: u32) {}
    fn chrwidth_custom(&mut self, _ch: u32) -> i32 {
        0
    }
    fn printwidth(&mut self, text: &[u8]) -> i32 {
        let w = self.d.wp();
        // SAFETY: valid write page.
        let f = unsafe { (*w).font } as usize;
        self.d.fontwidth.get(f).copied().unwrap_or(8) * text.len() as i32
    }
    fn lprint_print_page(&mut self) {}
    fn key_redraw(&mut self) {}
    fn take_width8050switch(&mut self) -> bool {
        std::mem::replace(&mut self.d.width8050switch, false)
    }
    fn width_80x50(&mut self) {
        let w = self.d.wp();
        // SAFETY: valid write page; conventional memory holds the 80x25 text page.
        unsafe {
            let (c, c2) = ((*w).color, (*w).background_color);
            let mut buf = vec![0u8; 80 * 25 * 2];
            core::ptr::copy_nonoverlapping(self.cmem.add(0xB8000), buf.as_mut_ptr(), buf.len());
            self.d.sub_screen_width = 80;
            self.d.sub_screen_height = 50;
            self.d.screen(self.cmem, 0, 0, 0, 0, 0, 1);
            core::ptr::copy_nonoverlapping(buf.as_ptr(), self.cmem.add(0xB8000), buf.len());
            let w = self.d.wp();
            (*w).color = c;
            (*w).background_color = c2;
        }
    }
    fn screen_pages(&mut self) -> Vec<*mut ImgStruct> {
        (0..self.d.page.len()).map(|i| if self.d.page[i] != 0 { im(i as u32) } else { core::ptr::null_mut() }).collect()
    }
    fn dest(&mut self) -> i32 {
        -(self.d.write_index as i32)
    }
    fn set_dest(&mut self, h: i32) {
        if let Some(i) = self.d.resolve(h) {
            self.d.write_index = i;
        }
    }
    fn source(&mut self) -> i32 {
        -(self.d.read_index as i32)
    }
    fn set_source(&mut self, h: i32) {
        if let Some(i) = self.d.resolve(h) {
            self.d.read_index = i;
        }
    }
    fn copyimage(&mut self, h: i32) -> i32 {
        self.d.copyimage(h, 0, 0)
    }
    fn freeimage(&mut self, h: i32) {
        self.d.freeimage(h, 1);
    }
    fn pcopy(&mut self, src: i32, dst: i32) {
        self.d.pcopy(src, dst);
    }
    fn image(&mut self, h: i32) -> *mut ImgStruct {
        im((-h) as u32)
    }
    fn autodisplay_begin(&mut self) -> i32 {
        std::mem::replace(&mut self.d.autodisplay, 1)
    }
    fn autodisplay_end(&mut self, previous: i32) {
        if previous == 0 {
            self.d.autodisplay = 0;
        }
    }
    fn stop_program(&mut self) -> bool {
        false
    }
    fn bios(&mut self) -> *mut u8 {
        // SAFETY: inside conventional memory.
        unsafe { self.cmem.add(0x400) }
    }
    fn stdin_byte(&mut self) -> Option<u8> {
        console::stdio::getc()
    }
    fn events(&mut self) {}
    fn sleep(&mut self, ms: u32) {
        std::thread::sleep(std::time::Duration::from_millis(u64::from(ms)));
    }
    fn ticks(&mut self) -> i64 {
        qb_runtime::systime::qb_get_ticks()
    }
}

fn chk(d: &mut Display) -> R<()> {
    let p = std::mem::take(&mut d.pending);
    if p != 0 {
        return Err(Stop::Err(p));
    }
    Ok(())
}

fn gerr(d: &mut Display, err: i32) {
    if err != 0 {
        d.err(err);
    }
}

/// Handles graphics/console builtins (registry callnames); `None` otherwise.
pub(crate) fn call(vm: &mut Vm, name: &str, a: &[V]) -> Option<R<Option<V>>> {
    let i = |k: usize| a[k].i() as i32;
    let u = |k: usize| a[k].i() as u32;
    let f32_ = |k: usize| a[k].f() as f32;
    let f = |k: usize| a[k].f();
    let known = matches!(
        name,
        "qbg_screen" | "sub_pset" | "sub_preset" | "sub_line" | "sub_circle" | "sub_paint" | "sub_paint.s" | "qbg_palette" | "qbg_sub_color"
            | "qbg_sub_view" | "qbg_sub_view_print" | "qbg_sub_window" | "func_pmap" | "func_point" | "sub_pcopy" | "sub_draw"
            | "sub_cls" | "qbg_sub_locate" | "sub__dest" | "sub__source" | "func__dest" | "func__source" | "func__display"
            | "func__console" | "func__newimage" | "sub__freeimage" | "func__copyimage" | "sub__display" | "func__width"
            | "func__height" | "func__pixelsize" | "func__rgb" | "func__rgba" | "func__red" | "func__green" | "func__blue"
            | "func__alpha" | "func__rgb32" | "func__rgba32" | "func__red32" | "func__green32" | "func__blue32"
            | "func__alpha32" | "func__palettecolor" | "sub__palettecolor" | "sub_graphics_get" | "sub_graphics_put"
            | "sub__font" | "func__font" | "func__fontwidth" | "func__fontheight" | "sub__autodisplay" | "func__autodisplay"
            | "SCREENPRINT" | "SCREENNL" | "SCREENZONE" | "SCREENTAB" | "SCREENSPC" | "func_screen" | "sub__limit"
            | "sub__delay" | "sub__title" | "func_csrlin" | "func_pos" | "qbsub_width"
    );
    if !known {
        return None;
    }
    let ext = if name == "sub_graphics_get" || name == "sub_graphics_put" {
        let ap = if name == "sub_graphics_get" { 5 } else { 3 };
        vm.array_extent(a[ap].p()).ok()
    } else {
        None
    };
    let cmem = vm.cmem.as_mut_ptr();
    let console_on = vm.console_on;
    let console_only = vm.console_only;
    let d = &mut vm.display;
    d.start(cmem, console_on, console_only);
    let wp = d.wp();
    let mut out: Option<V> = None;
    // SAFETY (all gfx calls): `wp`/`rp` and resolved surfaces are valid registry entries.
    unsafe {
        match name {
            "qbg_screen" => d.screen(cmem, i(0), i(1), i(2), i(3), i(4), i(5)),
            "sub_pset" => gerr(d, gfx::point::qb_gfx_pset_stmt(wp, f32_(0), f32_(1), u(2), i(3))),
            "sub_preset" => gerr(d, gfx::point::qb_gfx_preset_stmt(wp, f32_(0), f32_(1), u(2), i(3))),
            "sub_line" => gerr(d, gfx::line::qb_gfx_line(wp, f32_(0), f32_(1), f32_(2), f32_(3), u(4), i(5), u(6), i(7))),
            "sub_circle" => gerr(d, gfx::circle::qb_gfx_circle(wp, f(0), f(1), f(2), u(3), f(4), f(5), f(6), i(7))),
            "sub_paint" => gerr(d, gfx::paint::qb_gfx_paint(wp, f32_(0), f32_(1), u(2), u(3), i(5))),
            "sub_paint.s" => {
                if (*wp).text != 0 {
                    gerr(d, 5);
                } else {
                    let s = a[2].s();
                    gerr(d, gfx::paint::qb_gfx_paint_tile(wp, f32_(0), f32_(1), s.as_ptr(), s.len() as i32, u(3), i(5)));
                }
            }
            "qbg_palette" => gerr(d, gfx::palette::qb_gfx_palette(wp, u(0), u(1), i(2))),
            "qbg_sub_color" => {
                let passed = i(3);
                if passed != 0 {
                    let r = if (*wp).console != 0 {
                        console::stdio::color(u(0), u(1), passed)
                    } else {
                        gfx::palette::qb_gfx_color(wp, u(0), u(1), passed)
                    };
                    gerr(d, r);
                }
            }
            "qbg_sub_view" => gerr(d, gfx::view::qb_gfx_view(wp, i(0), i(1), i(2), i(3), i(4), i(5), i(6))),
            "qbg_sub_view_print" => {
                let mut h = VmHost { d, cmem };
                console::text::view_print(&mut h, i(0), i(1), i(2));
            }
            "qbg_sub_window" => gerr(d, gfx::view::qb_gfx_window(wp, f32_(0), f32_(1), f32_(2), f32_(3), i(4))),
            "func_pmap" => {
                let mut err = 0;
                let v = gfx::view::qb_gfx_pmap(wp, f32_(0), i(1), &mut err);
                gerr(d, err);
                out = Some(V::F(f64::from(v)));
            }
            "func_point" => {
                let mut err = 0;
                let v = gfx::point::qb_gfx_point(wp, d.rp(), f32_(0), f32_(1), i(2), &mut err);
                gerr(d, err);
                out = Some(V::F(v));
            }
            "sub_pcopy" => d.pcopy(i(0), i(1)),
            "sub_draw" => {
                let s = a[0].s();
                gerr(d, gfx::draw::qb_gfx_draw(wp, s.as_ptr(), s.len() as i32, cmem));
            }
            "sub_cls" => {
                let mut h = VmHost { d, cmem };
                console::text::cls(&mut h, i(0), u(1), i(2));
            }
            "qbg_sub_locate" => {
                let mut h = VmHost { d, cmem };
                console::text::locate(&mut h, i(0), i(1), i(2), i(3), i(4), i(5));
            }
            "func_csrlin" => {
                let mut h = VmHost { d, cmem };
                out = Some(V::I(console::text::csrlin(&mut h).into()));
            }
            "func_pos" => {
                let mut h = VmHost { d, cmem };
                out = Some(V::I(console::text::pos(&mut h).into()));
            }
            "func_screen" => {
                let mut h = VmHost { d, cmem };
                out = Some(V::I(console::text::screen(&mut h, i(0), i(1), i(2), i(3)).into()));
            }
            "sub__dest" => {
                if let Some(x) = d.resolve(i(0)) {
                    d.write_index = x;
                }
            }
            "sub__source" => {
                if let Some(x) = d.resolve(i(0)) {
                    d.read_index = x;
                }
            }
            "func__dest" => out = Some(V::I(-(d.write_index as i64))),
            "func__source" => out = Some(V::I(-(d.read_index as i64))),
            "func__display" => out = Some(V::I(-(d.display_index as i64))),
            "func__console" => out = Some(V::I(d.console_image.into())),
            "func__newimage" => out = Some(V::I(d.newimage(i(0), i(1), i(2), i(3)).into())),
            "sub__freeimage" => d.freeimage(i(0), i(1)),
            "func__copyimage" => out = Some(V::I(d.copyimage(i(0), i(1), i(2)).into())),
            "sub__display" => d.capture(gfx::capture::KIND_DISPLAY),
            "sub__autodisplay" => d.autodisplay = 1,
            "func__autodisplay" => out = Some(V::I(if d.autodisplay != 0 { -1 } else { 0 })),
            "func__width" | "func__height" | "func__pixelsize" => {
                let passed = i(1);
                let idx = if passed != 0 { d.resolve(i(0)) } else { Some(d.write_index) };
                let v = match idx {
                    None => 0,
                    Some(x) => {
                        let s = &*super::display::im(x);
                        match name {
                            "func__width" => i64::from(s.width),
                            "func__height" => i64::from(s.height),
                            _ => match s.compatible_mode {
                                32 => 4,
                                0 => 0,
                                _ => 1,
                            },
                        }
                    }
                };
                out = Some(V::I(v));
            }
            "func__rgb" | "func__rgba" => {
                let (n, passed) = if name == "func__rgb" { (3, i(4)) } else { (4, i(5)) };
                let target = if passed != 0 { d.resolve(i(n)).map(im) } else { Some(wp) };
                out = Some(V::I(match target {
                    None => 0,
                    Some(t) if name == "func__rgb" => i64::from(gfx::imgops::qb_gfx_rgb(t, i(0), i(1), i(2))),
                    Some(t) => i64::from(gfx::imgops::qb_gfx_rgba(t, i(0), i(1), i(2), i(3))),
                }));
            }
            "func__red" | "func__green" | "func__blue" | "func__alpha" => {
                let comp = match name {
                    "func__red" => 0,
                    "func__green" => 1,
                    "func__blue" => 2,
                    _ => 3,
                };
                let target = if i(2) != 0 { d.resolve(i(1)).map(im) } else { Some(wp) };
                let v = match target {
                    None => 0,
                    Some(t) => {
                        let mut err = 0;
                        let v = gfx::imgops::qb_gfx_component(t, u(0), comp, &mut err);
                        gerr(d, err);
                        v
                    }
                };
                out = Some(V::I(v.into()));
            }
            "func__rgb32" => out = Some(V::I(i64::from(0xFF00_0000u32 | ((u(0) & 255) << 16) | ((u(1) & 255) << 8) | (u(2) & 255)))),
            "func__rgba32" => out = Some(V::I(i64::from(((u(3) & 255) << 24) | ((u(0) & 255) << 16) | ((u(1) & 255) << 8) | (u(2) & 255)))),
            "func__red32" => out = Some(V::I(i64::from((u(0) >> 16) & 255))),
            "func__green32" => out = Some(V::I(i64::from((u(0) >> 8) & 255))),
            "func__blue32" => out = Some(V::I(i64::from(u(0) & 255))),
            "func__alpha32" => out = Some(V::I(i64::from(u(0) >> 24))),
            "func__palettecolor" => {
                let idx = if i(2) != 0 { d.resolve(i(1)) } else { Some(d.write_index) };
                let v = match idx {
                    None => 0,
                    Some(x) => {
                        let mut err = 0;
                        let v = gfx::imgops::qb_gfx_palettecolor_get(im(x), i(0), &mut err);
                        gerr(d, err);
                        v
                    }
                };
                out = Some(V::I(i64::from(v)));
            }
            "sub__palettecolor" => {
                let idx = if i(3) != 0 { d.resolve(i(2)) } else { Some(d.write_index) };
                if let Some(x) = idx {
                    gerr(d, gfx::imgops::qb_gfx_palettecolor_set(im(x), i(0), u(1)));
                }
            }
            "sub_graphics_get" | "sub_graphics_put" => {
                // element address and the bytes from there to the end of its array
                let ep = if name == "sub_graphics_get" { 4 } else { 2 };
                let elem = a[ep].p();
                let (base, total) = match ext {
                    Some(x) => x,
                    None => {
                        d.err(5);
                        (0, 0)
                    }
                };
                let len = (base + total).saturating_sub(elem) as i32;
                if name == "sub_graphics_get" {
                    gerr(d, gfx::getput::qb_gfx_get(d.rp(), f32_(0), f32_(1), f32_(2), f32_(3), elem as *mut u8, len, u(6), i(7)));
                } else {
                    gerr(d, gfx::getput::qb_gfx_put(wp, f32_(0), f32_(1), elem as *const u8, len, i(4), u(5), i(6)));
                }
            }
            "sub__font" => d.font(i(0), i(1), i(2)),
            "func__font" => {
                let idx = if i(1) != 0 { d.resolve(i(0)) } else { Some(d.write_index) };
                out = Some(V::I(idx.map_or(0, |x| i64::from((*im(x)).font))));
            }
            "func__fontwidth" | "func__fontheight" => {
                let mut fnt = i(0);
                if i(1) != 0 {
                    if !(matches!(fnt, 8 | 14 | 16) || ((32..64).contains(&fnt) && d.fontwidth[fnt as usize] != 0)) {
                        d.err(258);
                        fnt = 0;
                    }
                } else {
                    fnt = (*wp).font as i32;
                }
                let tbl = if name == "func__fontwidth" { &d.fontwidth } else { &d.fontheight };
                out = Some(V::I(tbl[fnt as usize].into()));
            }
            "SCREENPRINT" => {
                let mut h = VmHost { d, cmem };
                console::text::print(&mut h, a[0].s(), false);
            }
            "SCREENNL" => {
                let mut h = VmHost { d, cmem };
                console::text::print(&mut h, b"", true);
            }
            "SCREENZONE" => {
                let mut h = VmHost { d, cmem };
                console::text::tab(&mut h);
            }
            "SCREENTAB" | "SCREENSPC" => {
                let mut h = VmHost { d, cmem };
                let n = a[0].i();
                if name == "SCREENSPC" {
                    console::text::print(&mut h, &vec![b' '; n.max(0) as usize], false);
                } else {
                    let pos = console::text::pos(&mut h) as i64;
                    if n > 0 && pos < n {
                        console::text::print(&mut h, &vec![b' '; (n - pos) as usize], false);
                    }
                }
            }
            "qbsub_width" => {}
            "sub__limit" | "sub__delay" | "sub__title" => {}
            _ => {}
        }
    }
    let r = chk(&mut vm.display);
    Some(r.map(|_| out))
}
