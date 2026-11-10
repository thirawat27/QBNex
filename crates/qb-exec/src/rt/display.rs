//! Screen pages, image handles and the graphics/console statements of the
//! v1 interpreter. Ported from the parts of libqb.cpp that still owned this
//! state (`qbg_screen`, `validatepage`, `func__newimage`, `sub__dest`, the
//! one-line graphics shims, `end()`'s capture); the surfaces, drawing, text
//! console and capture are qb-runtime (`gfx`, `console`).

use super::builtins::Host as VmIface;
use super::{Stop, R, V};
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
    /// font tables of libqb (`font`, `fontwidth`, `fontheight`, `fontflags`):
    /// 8/9/14/15/16/17 are builtin, 32.. are `_LOADFONT` handles (`font` = qb_runtime::font handle)
    pub font: Vec<i32>,
    pub fontwidth: Vec<i32>,
    pub fontheight: Vec<i32>,
    pub fontflags: Vec<i32>,
    /// error raised through the console Host
    pub pending: i32,
    pub started: bool,
    /// `$SCREENHIDE`
    pub screen_hide: bool,
    /// `_TITLE` (window_title)
    pub title: Vec<u8>,
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
            font: vec![0; 49],
            fontwidth: vec![0; 49],
            fontheight: vec![0; 49],
            fontflags: vec![0; 49],
            pending: 0,
            started: false,
            title: Vec::new(),
            screen_hide: false,
        };
        d.fontwidth[8] = 8;
        d.fontwidth[14] = 8;
        d.fontwidth[16] = 8;
        d.fontheight[8] = 8;
        d.fontheight[14] = 14;
        d.fontheight[16] = 16;
        // double-width variants (40-column SCREEN 0)
        for f in [8, 14, 16] {
            d.fontwidth[f + 1] = 16;
            d.fontheight[f + 1] = d.fontheight[f];
        }
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
        let ok = matches!(f, 8 | 9 | 14 | 15 | 16 | 17) || (f >= 32 && self.font.get(f as usize).is_some_and(|&h| h != 0));
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

    /// `func__loadfont(file$, size, requirements$, passed)`
    pub(crate) fn loadfont(&mut self, file: &[u8], size: i32, req: &[u8], passed: i32) -> i32 {
        if size < 1 {
            self.err(5);
            return 0;
        }
        if size > 2048 || file.is_empty() {
            return -1;
        }
        let path = String::from_utf8_lossy(file).into_owned();
        let content = match std::fs::read(&path) {
            Ok(c) => c,
            Err(_) => {
                #[cfg(windows)]
                {
                    match std::fs::read(format!("C:/Windows/Fonts/{path}")) {
                        Ok(c) => c,
                        Err(_) => return -1,
                    }
                }
                #[cfg(not(windows))]
                return -1;
            }
        };
        let mut r = [0u8; 6];
        if passed != 0 && !req.is_empty() {
            let up = String::from_utf8_lossy(req).to_uppercase();
            for part in up.split(',') {
                let k = match part.trim() {
                    "BOLD" => 0,
                    "ITALIC" => 1,
                    "UNDERLINE" => 2,
                    "DONTBLEND" => 3,
                    "MONOSPACE" => 4,
                    "UNICODE" => 5,
                    _ => {
                        self.err(5);
                        return 0;
                    }
                };
                r[k] += 1;
            }
            if r.iter().any(|&c| c > 1) {
                self.err(5);
                return 0;
            }
        }
        let options = r.iter().enumerate().map(|(k, &c)| i32::from(c) << k).sum::<i32>();
        let i = match (32..self.font.len()).find(|&k| self.font[k] == 0) {
            Some(k) => k,
            None => {
                for t in [&mut self.font, &mut self.fontwidth, &mut self.fontheight, &mut self.fontflags] {
                    t.push(0);
                }
                self.font.len() - 1
            }
        };
        let h = qb_runtime::font::load(&content, size, -1, options);
        if h == 0 {
            return -1;
        }
        self.font[i] = h;
        self.fontheight[i] = size;
        self.fontwidth[i] = qb_runtime::font::width(h);
        self.fontflags[i] = options;
        i as i32
    }

    /// `func__loadimage(file$, mode, passed)` (image_bridge.cpp): 32, 256 or 257 (no
    /// palette remap); 33 (hardware) loads a software 32bpp image here
    pub(crate) fn loadimage(&mut self, file: &[u8], mut bpp: i32, passed: i32) -> i32 {
        let mut keep_palette = false;
        if bpp == 33 {
            bpp = 32;
        } else if bpp == 257 {
            bpp = 256;
            keep_palette = true;
        }
        if passed != 0 {
            if bpp != 32 && bpp != 256 {
                self.err(5);
                return 0;
            }
        } else {
            // SAFETY: valid write page.
            if unsafe { (*self.wp()).text } != 0 {
                self.err(5);
                return 0;
            }
            bpp = -1;
        }
        if file.is_empty() {
            return -1;
        }
        let Ok(content) = std::fs::read(String::from_utf8_lossy(file).as_ref()) else { return -1 };
        let Some(img) = qb_runtime::image::decode(&content) else { return -1 };
        let (w, h) = (img.w as usize, img.h as usize);
        if bpp == 256 {
            let i = self.newimage(w as i32, h as i32, 256, 1);
            if i == -1 {
                return -1;
            }
            let mut pal = [0u32; 256];
            let Some(mut px8) = qb_runtime::image::quant::to_8bpp(&img.px, w, h, &mut pal) else { return -1 };
            if !keep_palette {
                let default = &qb_runtime::gfx::data::palette_256;
                qb_runtime::image::quant::remap_palette(&mut px8, &pal, default);
                pal = *default;
            }
            // SAFETY: a fresh w x h 8bpp surface with a 256-entry palette.
            unsafe {
                let p = im((-i) as u32);
                core::ptr::copy_nonoverlapping(px8.as_ptr(), (*p).offset, w * h);
                core::ptr::copy_nonoverlapping(pal.as_ptr(), (*p).pal, 256);
            }
            return i;
        }
        let i = self.newimage(w as i32, h as i32, 32, 1);
        if i == -1 {
            return -1;
        }
        // SAFETY: a fresh w x h 32bpp surface.
        unsafe { core::ptr::copy_nonoverlapping(img.px.as_ptr(), (*im((-i) as u32)).offset, w * h * 4) };
        i
    }

    /// `sub__freefont`
    pub(crate) fn freefont(&mut self, f: i32) {
        if f < 32 || self.font.get(f as usize).is_none_or(|&h| h == 0) {
            self.err(258);
            return;
        }
        // SAFETY: registry reads.
        unsafe {
            for k in 1..nextimg() {
                let p = &*im(k as u32);
                if p.valid != 0 && p.font == f as u32 {
                    self.err(5);
                    return;
                }
            }
        }
        qb_runtime::font::free(self.font[f as usize]);
        self.font[f as usize] = 0;
    }

    /// Glyph bitmap of a custom font (FontRenderTextASCII / UTF32); `mono`: 0/255 pixels.
    fn render(&self, f: u32, ch: u32, mono: bool) -> Option<qb_runtime::font::Rendered> {
        let cp = if self.fontflags[f as usize] & 32 != 0 { ch } else { console::utf8::cp437((ch & 255) as u8) };
        qb_runtime::font::render(self.font[f as usize], &[cp], i32::from(mono))
    }

    /// 8-bit, alpha-disabled 32-bit or DONTBLEND: monochrome rendering
    unsafe fn mono_target(&self, p: *const ImgStruct, f: u32) -> bool {
        // SAFETY: valid surface.
        unsafe { (*p).bytes_per_pixel == 1 || ((*p).bytes_per_pixel == 4 && (*p).alpha_disabled != 0) || self.fontflags[f as usize] & 8 != 0 }
    }

    /// `printchr_custom`: a custom-font glyph at the cursor of the write page.
    unsafe fn printchr_custom(&self, ch: u32) {
        let w = self.wp();
        // SAFETY: valid write page; pixels go through qb_gfx_pset like libqb's pset().
        unsafe {
            let (color, bg, f) = ((*w).color, (*w).background_color, (*w).font);
            let fw = self.fontwidth[f as usize];
            let x = if fw != 0 { fw * (i32::from((*w).cursor_x) - 1) } else { i32::from((*w).cursor_x) - 1 };
            let y = (i32::from((*w).cursor_y) - 1) * self.fontheight[f as usize];
            let h = self.fontheight[f as usize];
            let mono = self.mono_target(w, f);
            let Some(r) = self.render(f, ch, mono) else { return };
            let gw = r.w;
            let px = |x2: i32, y2: i32| r.data[(y2 * gw + x2) as usize];
            let pset = |xx: i32, yy: i32, c: u32| gfx::blend::qb_gfx_pset(w, xx, yy, c);
            let mode = (*w).print_mode;
            if mono {
                for y2 in 0..h {
                    for x2 in 0..gw {
                        let on = px(x2, y2) != 0;
                        match mode {
                            3 => pset(x + x2, y + y2, if on { color } else { bg }),
                            1 if on => pset(x + x2, y + y2, color),
                            2 if !on => pset(x + x2, y + y2, bg),
                            _ => {}
                        }
                    }
                }
                return;
            }
            let (a, a2) = ((color >> 24) + 1, (bg >> 24) + 1);
            let (z, z2) = (color & 0xFF_FFFF, bg & 0xFF_FFFF);
            match mode {
                3 => {
                    let c1 = [(color >> 24) & 255, (color >> 16) & 255, (color >> 8) & 255, color & 255].map(|v| v as f32);
                    let c2 = [(bg >> 24) & 255, (bg >> 16) & 255, (bg >> 8) & 255, bg & 255].map(|v| v as f32);
                    let (alpha1, r1, g1, b1) = (c1[0], c1[1], c1[2], c1[3]);
                    let (da, dr, dg, db) = (c2[0] - alpha1, c2[1] - r1, c2[2] - g1, c2[3] - b1);
                    let cw = if alpha1 != 0.0 { c2[0] / alpha1 } else { 100000.0 };
                    let q = |v: f32| qb_runtime::round::qb_qbr_f64(f64::from(v)) as i32 as u32;
                    for y2 in 0..h {
                        for x2 in 0..gw {
                            let mut d = (255.0 - f32::from(px(x2, y2))) / 255.0;
                            let alpha3 = alpha1 + da * d;
                            d *= cw;
                            if d > 1.0 {
                                d = 1.0;
                            }
                            let (r3, g3, b3) = (r1 + dr * d, g1 + dg * d, b1 + db * d);
                            pset(x + x2, y + y2, q(b3).wrapping_add(q(g3) << 8).wrapping_add(q(r3) << 16).wrapping_add(q(alpha3) << 24));
                        }
                    }
                }
                1 => {
                    for y2 in 0..h {
                        for x2 in 0..gw {
                            let z3 = u32::from(px(x2, y2));
                            if z3 != 0 {
                                pset(x + x2, y + y2, (((z3 * a) >> 8) << 24) + z);
                            }
                        }
                    }
                }
                2 => {
                    for y2 in 0..h {
                        for x2 in 0..gw {
                            let z3 = u32::from(px(x2, y2));
                            if z3 != 255 {
                                pset(x + x2, y + y2, ((((255 - z3) * a2) >> 8) << 24) + z2);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// `func__fontwidth` / `func__fontheight` with an explicit font: builtin 8/14/16 or a loaded one
    fn font_metric(&mut self, f: i32, height: bool) -> i32 {
        let ok = matches!(f, 8 | 14 | 16) || (f >= 32 && self.font.get(f as usize).is_some_and(|&h| h != 0));
        if !ok {
            self.err(258);
            return 0;
        }
        if height { self.fontheight[f as usize] } else { self.fontwidth[f as usize] }
    }

    /// `func__printwidth(text$, handle, passed)`: a proportional font is measured by
    /// printing on a 65535-pixel wide temporary image, as libqb does
    pub(crate) fn printwidth(&mut self, cmem: *mut u8, text: &[u8], handle: i32, passed: i32) -> i32 {
        let idx = if passed != 0 {
            match self.resolve(handle) {
                Some(x) => x,
                None => return 0,
            }
        } else {
            self.write_index
        };
        // SAFETY: valid surfaces throughout.
        unsafe {
            if (*im(idx)).text != 0 {
                return text.len() as i32;
            }
            if text.is_empty() {
                return 0;
            }
            let f = (*im(idx)).font as i32;
            let fw = self.font_metric(f, false);
            if fw != 0 {
                return fw * text.len() as i32;
            }
            let fh = self.font_metric(f, true);
            let tmp = self.newimage(65535, fh, 32, 1);
            let Some(t) = self.resolve(tmp) else { return 0 };
            let (ow, or) = (self.write_index, self.read_index);
            self.write_index = t;
            self.read_index = t;
            self.font(f, 0, 0);
            {
                let mut h = VmHost { d: self, cmem };
                console::text::print(&mut h, text, false);
            }
            let mut xpos = 0;
            for i in (0..65535).rev() {
                if gfx::point::qb_gfx_point_raw(im(t), i, 0) != 0 {
                    xpos = i;
                    break;
                }
            }
            self.freeimage(tmp, 1);
            self.write_index = ow;
            self.read_index = or;
            if xpos == 0 { 0 } else { xpos + 1 }
        }
    }

    /// `chrwidth_custom`
    unsafe fn chrwidth_custom(&self, ch: u32) -> i32 {
        let w = self.wp();
        // SAFETY: valid write page.
        unsafe {
            let f = (*w).font;
            let mono = self.mono_target(w, f);
            self.render(f, ch, mono).map_or(0, |r| r.w)
        }
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

    /// Pages 1.. of the screen are freed (WIDTH keeps only the visible page 0).
    fn width_drop_pages(&mut self) {
        // SAFETY: valid write page.
        if unsafe { (*self.wp()).flags } & IMG_SCREEN == 0 {
            return;
        }
        for k in 1..self.page.len() {
            let i2 = self.page[k];
            if i2 != 0 {
                if self.display_index == i2 as u32 {
                    self.display_index = self.page[0] as u32;
                }
                if self.read_index == i2 as u32 {
                    self.read_index = self.display_index;
                }
                if self.write_index == i2 as u32 {
                    self.write_index = self.display_index;
                }
                reg::qb_img_release_owned(i2);
                self.freeimg(i2 as u32);
            }
        }
    }

    /// Mode defaults again with the colours kept, font `f`; `window`: keep WINDOW scaling (modes 9-12).
    fn width_revert(&mut self, f: i32, window: bool) {
        self.width_drop_pages();
        let w = self.wp();
        // SAFETY: valid write page.
        unsafe {
            let (col, col2) = ((*w).color, (*w).background_color);
            let (x1, y1, x2, y2) = ((*w).window_x1, (*w).window_y1, (*w).window_x2, (*w).window_y2);
            reg::qb_imgrevert(self.write_index as i32);
            if window {
                gfx::view::qb_gfx_window(w, x1, y1, x2, y2, 1 + 2);
                (*w).clipping_or_scaling = 0;
            }
            (*w).color = col;
            (*w).background_color = col2;
        }
        self.selectfont(f, w);
    }

    /// SCREEN 0 again with a new text layout.
    fn width_screen0(&mut self, cmem: *mut u8, w: i32, h: i32, f: i32) {
        if w != -1 {
            self.sub_screen_width = w;
        }
        if h != -1 {
            self.sub_screen_height = h;
        }
        if f != -1 {
            self.sub_screen_font = f;
        }
        self.screen(cmem, 0, 0, 0, 0, 0, 1);
    }

    /// `qbsub_width` (WIDTH [w][, h]; option 0 only: files/LPRINT are not routed here).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn width(&mut self, cmem: *mut u8, option: i32, value1: i32, value2: i32, value3: i32, value4: i32, passed: i32) {
        if option != 0 {
            if option == 2 && passed == 1 && (1..=255).contains(&value1) {
                return;
            }
            self.err(5);
            return;
        }
        self.width8050switch = false;
        let (mut width, mut height) = (value1, value2);
        let w = self.wp();
        // SAFETY: valid write page (all reads below).
        unsafe {
            if (*w).console != 0 {
                #[cfg(windows)]
                console::stdio::width(value1, value2, value3, value4);
                #[cfg(not(windows))]
                let _ = (value3, value4);
                return;
            }
            if passed & 3 == 0 {
                self.err(5);
                return;
            }
            let mode = i32::from((*w).compatible_mode);
            let (pw, ph, font) = (i32::from((*w).width), i32::from((*w).height), (*w).font as i32);
            if mode == 32 || mode == 256 {
                if passed & 1 == 0 {
                    width = pw;
                } else {
                    if width <= 0 {
                        self.err(5);
                        return;
                    }
                    width *= self.fontwidth[font as usize].max(1);
                }
                if passed & 2 == 0 {
                    height = ph;
                } else {
                    if height <= 0 {
                        self.err(5);
                        return;
                    }
                    height *= self.fontheight[font as usize];
                }
                if width == pw && height == ph {
                    return;
                }
                self.width_drop_pages();
                let w = self.wp();
                let (col, col2) = ((*w).color, (*w).background_color);
                (*w).width = width as _;
                (*w).height = height as _;
                let bytes = (width * height * i32::from((*w).bytes_per_pixel)) as isize;
                if (*w).flags & gfx::IMG_FREEMEM != 0 {
                    reg::qb_img_release_pixels(self.write_index as i32);
                    reg::qb_img_alloc_pixels(self.write_index as i32, bytes);
                } else {
                    core::ptr::write_bytes((*w).offset, 0, bytes as usize);
                }
                reg::qb_imgrevert(self.write_index as i32);
                (*w).color = col;
                (*w).background_color = col2;
                self.selectfont(font, w);
                return;
            }
            let f0 = |height: i32, width: i32| {
                let mut f = 8;
                if height <= 49 {
                    f = 14;
                }
                if height <= 42 {
                    f = 16;
                }
                if width <= 40 {
                    f += 1;
                }
                f
            };
            let low = (1..=8).contains(&mode) || mode == 13;
            let fh = self.fontheight[font as usize];
            if passed & 1 == 0 {
                // width omitted
                if height <= 0 {
                    self.err(5);
                    return;
                }
                if mode == 0 {
                    let f = f0(height, pw);
                    if font == f && ph == height {
                        return;
                    }
                    self.width_screen0(cmem, pw, height, f);
                    return;
                }
                if low && ph == height * 8 {
                    if font != 8 {
                        self.width_revert(8, false);
                    }
                    return;
                }
                if mode == 9 || mode == 10 {
                    let mut f = 0;
                    if ph == height * 8 {
                        f = 8;
                    }
                    if ph == height * 14 {
                        f = 14;
                    }
                    if height == 43 && ph == 350 {
                        f = 8;
                    }
                    if f != 0 {
                        if font != f {
                            self.width_revert(f, true);
                        }
                        return;
                    }
                }
                if mode == 11 || mode == 12 {
                    let mut f = 0;
                    if ph == height * 8 {
                        f = 8;
                    }
                    if ph == height * 16 {
                        f = 16;
                    }
                    if f != 0 {
                        if font != f {
                            self.width_revert(f, true);
                        }
                        return;
                    }
                }
                if height == 25 || height == 50 || height == 43 {
                    self.width_screen0(cmem, -1, height, -1);
                    return;
                }
                self.err(5);
                return;
            }
            if passed & 2 == 0 {
                // height omitted
                if width <= 0 {
                    self.err(5);
                    return;
                }
                if mode == 0 {
                    let f = f0(ph, width);
                    if font == f && pw == width {
                        return;
                    }
                    self.width_screen0(cmem, width, ph, f);
                    return;
                }
                if low && pw == width * 8 {
                    if font != 8 {
                        self.width_revert(8, false);
                    }
                    return;
                }
                if (mode == 9 || mode == 10 || mode == 11 || mode == 12) && pw == width * 8 {
                    let f = if fh > 8 { if mode <= 10 { 14 } else { 16 } } else { 8 };
                    if font != f {
                        self.width_revert(f, true);
                    }
                    return;
                }
                if width == 40 || width == 80 {
                    self.width_screen0(cmem, width, -1, -1);
                    return;
                }
                self.err(5);
                return;
            }
            // both passed
            if width <= 0 || height <= 0 {
                self.err(5);
                return;
            }
            if mode == 0 {
                let f = f0(height, width);
                if font == f && pw == width && ph == height {
                    return;
                }
                self.width_screen0(cmem, width, height, f);
                return;
            }
            if low && pw == width * 8 && ph == height * 8 {
                if font != 8 {
                    self.width_revert(8, false);
                }
                return;
            }
            if (mode == 9 || mode == 10) && pw == width * 8 {
                let mut f = 0;
                if ph == height * 8 {
                    f = 8;
                }
                if ph == height * 14 {
                    f = 14;
                }
                if height == 43 && ph == 350 {
                    f = 8;
                }
                if f != 0 {
                    if font != f {
                        self.width_revert(f, true);
                    }
                    return;
                }
            }
            if (mode == 11 || mode == 12) && pw == width * 8 {
                let mut f = 0;
                if ph == height * 8 {
                    f = 8;
                }
                if ph == height * 16 {
                    f = 16;
                }
                if f != 0 {
                    if font != f {
                        self.width_revert(f, true);
                    }
                    return;
                }
            }
            if (width == 40 || width == 80) && (height == 25 || height == 50 || height == 43) {
                let mut f = 16;
                if height == 43 {
                    f = 14;
                }
                if height == 50 {
                    f = 8;
                }
                if width == 40 {
                    f += 1;
                }
                self.width_screen0(cmem, width, height, f);
                return;
            }
            self.err(5);
        }
    }

    /// Pixel size of the display page (text pages in font cells).
    pub(crate) fn display_size(&self) -> (i32, i32) {
        if !self.started {
            return (640, 400);
        }
        // SAFETY: valid display page.
        unsafe {
            let p = &*self.dp();
            let (mut w, mut h) = (i32::from(p.width), i32::from(p.height));
            if p.text != 0 {
                w *= self.fontwidth[p.font as usize].max(1);
                h *= self.fontheight[p.font as usize].max(1);
            }
            (w, h)
        }
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
    fn printchr_custom(&mut self, ch: u32) {
        // SAFETY: the console calls this for a custom font on the write page.
        unsafe { self.d.printchr_custom(ch) }
    }
    fn chrwidth_custom(&mut self, ch: u32) -> i32 {
        // SAFETY: as above.
        unsafe { self.d.chrwidth_custom(ch) }
    }
    fn printwidth(&mut self, text: &[u8]) -> i32 {
        let cmem = self.cmem;
        self.d.printwidth(cmem, text, 0, 0)
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
pub(crate) fn call(vm: &mut dyn VmIface<'_>, name: &str, a: &[V]) -> Option<R<Option<V>>> {
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
            | "SCREENPRINT" | "SCREENNL" | "SCREENZONE" | "SCREENTAB" | "SCREENSPC" | "func_screen" 
            | "sub__delay" | "sub__title" | "func_csrlin" | "func_pos" | "qbsub_width" | "sub__setalpha" | "sub__clearcolor"
            | "func__clearcolor" | "sub__putimage" | "sub__maptriangle" | "func__loadfont" | "sub__freefont"
            | "func__printwidth" | "func__loadimage" | "func__title"
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
    let rt = vm.rt();
    let cmem = rt.cmem.as_mut_ptr();
    let console_on = rt.console_on;
    let console_only = rt.console_only;
    let d = &mut rt.display;
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
                // DRAW reads VARPTR$ offsets at cmem + 1280 (the C++ DBLOCK); the VM's DBLOCK is elsewhere
                let base = cmem.wrapping_add(super::builtins::DBLOCK - 1280);
                gerr(d, gfx::draw::qb_gfx_draw(wp, s.as_ptr(), s.len() as i32, base));
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
            "sub__display" => {
                d.capture(gfx::capture::KIND_DISPLAY);
                // with a window, _DISPLAY switches _AUTODISPLAY off (sub__display)
                if !d.screen_hide {
                    d.autodisplay = 0;
                }
            }
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
            "func__rgb32" => {
                // overloads by argument count (qbx.cpp): (i) (i,a) (r,g,b) (r,g,b,a); components clamp to 0..255
                let c = |k: usize| i(k).clamp(0, 255) as u32;
                let v = match i(4) {
                    1 => 0xFF00_0000 | (c(0) << 16) | (c(0) << 8) | c(0),
                    2 => (c(1) << 24) | (c(0) << 16) | (c(0) << 8) | c(0),
                    3 => 0xFF00_0000 | (c(0) << 16) | (c(1) << 8) | c(2),
                    _ => (c(3) << 24) | (c(0) << 16) | (c(1) << 8) | c(2),
                };
                out = Some(V::I(i64::from(v)));
            }
            "func__rgba32" => {
                let c = |k: usize| i(k).clamp(0, 255) as u32;
                out = Some(V::I(i64::from((c(3) << 24) | (c(0) << 16) | (c(1) << 8) | c(2))));
            }
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
                let height = name == "func__fontheight";
                let v = if i(1) != 0 {
                    d.font_metric(i(0), height)
                } else {
                    let f = (*wp).font as usize;
                    if height { d.fontheight[f] } else { d.fontwidth[f] }
                };
                out = Some(V::I(v.into()));
            }
            "func__loadfont" => {
                // size: the C++ int32 parameter of a DOUBLE argument (truncated)
                let v = d.loadfont(a[0].s(), f(1) as i32, a[2].s(), i(3));
                out = Some(V::I(v.into()));
            }
            "sub__freefont" => d.freefont(i(0)),
            "func__loadimage" => {
                let v = d.loadimage(a[0].s(), i(1), i(2));
                out = Some(V::I(v.into()));
            }
            "func__printwidth" => {
                let v = d.printwidth(cmem, a[0].s(), i(1), i(2));
                out = Some(V::I(v.into()));
            }
            "SCREENPRINT" => {
                // one PRINT item: makefit (new line first when it does not fit), then print
                let mut h = VmHost { d, cmem };
                console::text::makefit(&mut h, a[0].s());
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
            "sub__setalpha" => {
                // (a, c, c2, image, passed): image passed = 2
                let t = if i(4) & 2 != 0 { d.resolve(i(3)) } else { Some(d.write_index) };
                if let Some(x) = t {
                    gerr(d, gfx::imgops::qb_gfx_setalpha(im(x), i(0), u(1), u(2), i(4)));
                }
            }
            "sub__clearcolor" => {
                // (c, image, passed): _NONE = 1, colour = 2, image = 4
                let t = if i(2) & 4 != 0 { d.resolve(i(1)) } else { Some(d.write_index) };
                if let Some(x) = t {
                    gerr(d, gfx::imgops::qb_gfx_clearcolor(im(x), u(0), i(2)));
                }
            }
            "func__clearcolor" => {
                let t = if i(1) != 0 { d.resolve(i(0)) } else { Some(d.write_index) };
                out = Some(V::I(match t {
                    None => 0,
                    Some(x) => {
                        let p = &*im(x);
                        if p.text != 0 {
                            -1
                        } else if p.compatible_mode == 32 {
                            0
                        } else {
                            i64::from(p.transparent_color)
                        }
                    }
                }));
            }
            "sub__putimage" => {
                // (dx1, dy1, dx2, dy2, src, dst, sx1, sy1, sx2, sy2, passed); software surfaces only
                let mut passed = i(10);
                let s = if passed & 8 != 0 { d.resolve(i(4)) } else { Some(d.read_index) };
                let t = if passed & 32 != 0 { d.resolve(i(5)) } else { Some(d.write_index) };
                if let (Some(sx), Some(tx)) = (s, t) {
                    let (sp, tp) = (im(sx), im(tx));
                    if (*sp).text != 0 || (*tp).text != 0 || ((*sp).bytes_per_pixel == 4 && (*tp).bytes_per_pixel == 1) {
                        gerr(d, 5);
                    } else {
                        let c = gfx::putimage::PutCoords { dx1: f(0), dy1: f(1), dx2: f(2), dy2: f(3), sx1: f(6), sy1: f(7), sx2: f(8), sy2: f(9) };
                        let mut tmp = 0;
                        let mut src = sp;
                        if sx == tx {
                            // a surface cannot be put onto itself: through a temporary copy
                            tmp = d.copyimage(-(sx as i32), 0, 0);
                            passed |= 8;
                            src = im((-tmp) as u32);
                        }
                        if tmp <= 0 {
                            gerr(d, gfx::putimage::qb_gfx_putimage(src, tp, &c, passed, core::ptr::null_mut()));
                        }
                        if tmp < 0 {
                            d.freeimage(tmp, 1);
                        }
                    }
                }
            }
            "sub__maptriangle" => {
                // (cull, sx1..sy3, si, dx1, dy1, dz1, .., di, smooth, passed); software surfaces only
                let po = i(19);
                let mut passed = 0;
                if po & 1 != 0 {
                    passed += 1;
                }
                if po & 2 != 0 {
                    passed += 2;
                }
                if po & 32 != 0 {
                    passed += 4;
                }
                if po & 64 != 0 {
                    passed += 8;
                }
                let s = if passed & 2 != 0 { d.resolve(i(7)) } else { Some(d.read_index) };
                let t = if passed & 4 != 0 { d.resolve(i(17)) } else { Some(d.write_index) };
                if let (Some(sx), Some(tx)) = (s, t) {
                    let s6 = [f32_(1), f32_(2), f32_(3), f32_(4), f32_(5), f32_(6)];
                    let d6 = [f32_(8), f32_(9), f32_(11), f32_(12), f32_(14), f32_(15)];
                    gerr(d, gfx::maptri::qb_gfx_maptriangle(im(sx), im(tx), s6.as_ptr(), d6.as_ptr(), passed & 1));
                }
            }
            "qbsub_width" => d.width(cmem, i(0), i(1), i(2), i(3), i(4), i(5)),
            "sub__title" => d.title = a[0].s().to_vec(),
            "func__title" => out = Some(V::S(d.title.clone())),
            "sub__delay" => {}
            _ => {}
        }
    }
    let r = chk(&mut vm.rt().display);
    Some(r.map(|_| out))
}
