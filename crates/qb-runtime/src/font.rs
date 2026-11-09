//! TrueType/OpenType fonts for `_LOADFONT`, `_FONTWIDTH`, `_PRINTWIDTH`,
//! `_UPRINTSTRING` ... (was `internal/c/parts/video/font/ttf/src.c`, a wrapper
//! around FreeType 2.4.9).
//!
//! The wrapper logic (`FontLoad`, `FontRenderTextUTF32`, `FontWidth`,
//! `FontFree`) is ported line by line. FreeType itself is replaced by
//! `skrifa` (outlines, TrueType bytecode hinting) and `ab_glyph_rasterizer`
//! (anti-aliased coverage):
//! - size metrics (ascender, height, so the baseline) follow FreeType's
//!   fixed-point formulas exactly, including the TrueType "round ppem" rule;
//! - glyph advances use the hinted phantom points and the `hdmx` table like
//!   FreeType's TrueType loader;
//! - glyph bitmaps come from the control box of the hinted outline, floored
//!   and ceiled to whole pixels, like FreeType's renderers.
//!
//! Deviation D-R7-6: pixel coverage and hinting are not FreeType's, so glyph
//! pixels may differ (see the tolerance in `tests/diff/diff_font.cpp`), and only
//! TrueType/OpenType (incl. collections) load; FreeType also read Type 1,
//! CID, PFR, Type 42, Windows FNT/FON, PCF and BDF fonts, and embedded
//! bitmap strikes.

use std::sync::Mutex;

use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{
    DrawSettings, Engine, HintingInstance, HintingOptions, OutlinePen, SmoothMode, Target,
};
use skrifa::raw::{FontRef, TableProvider};
use skrifa::{GlyphId, MetadataProvider};

// ------------------------------------------------- FreeType fixed point ---

/// `FT_MulFix` (portable C version of FreeType 2.4.9).
fn mul_fix(a: i64, b: i64) -> i64 {
    let s = (a < 0) != (b < 0);
    let c = (a.abs() * b.abs() + 0x8000) >> 16;
    if s {
        -c
    } else {
        c
    }
}

/// `FT_DivFix` (portable C version of FreeType 2.4.9).
fn div_fix(a: i64, b: i64) -> i64 {
    let s = (a < 0) != (b < 0);
    let (a, b) = (a.abs(), b.abs());
    let q = if b == 0 {
        0x7FFF_FFFF
    } else {
        (((a << 16) + (b >> 1)) / b) as u32 as i64
    };
    if s {
        -q
    } else {
        q
    }
}

fn pix_round(x: i64) -> i64 {
    (x + 32) & !63
}

fn pix_ceil(x: i64) -> i64 {
    (x + 63) & !63
}

fn pix_floor(x: i64) -> i64 {
    x & !63
}

/// Face-level values FreeType derives from the font (`sfnt_load_face`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaceMetrics {
    pub units_per_em: i64,
    pub ascender: i64,
    pub descender: i64,
    pub height: i64,
    /// TrueType outlines and `head.flags` bit 3 ("round ppem") set.
    pub round_ppem: bool,
}

/// `FT_Size_Metrics` after `FT_Set_Pixel_Sizes(face, 0, pixel_height)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizeMetrics {
    pub ppem: i64,
    pub x_scale: i64,
    pub y_scale: i64,
    /// 26.6 pixels.
    pub ascender: i64,
    pub height: i64,
}

impl FaceMetrics {
    fn read(font: &FontRef) -> Option<FaceMetrics> {
        let head = font.head().ok()?;
        let hhea = font.hhea().ok()?;
        let mut ascender = hhea.ascender().to_i16() as i64;
        let mut descender = hhea.descender().to_i16() as i64;
        let mut height = (ascender - descender + hhea.line_gap().to_i16() as i64) as i16 as i64;
        if ascender == 0 && descender == 0
            && let Ok(os2) = font.os2() {
                if os2.s_typo_ascender() != 0 || os2.s_typo_descender() != 0 {
                    ascender = os2.s_typo_ascender() as i64;
                    descender = os2.s_typo_descender() as i64;
                    height = (ascender - descender + os2.s_typo_line_gap() as i64) as i16 as i64;
                } else {
                    ascender = os2.us_win_ascent() as i16 as i64;
                    descender = -(os2.us_win_descent() as i16 as i64);
                    height = (ascender - descender) as u16 as i64;
                }
            }
        Some(FaceMetrics {
            units_per_em: head.units_per_em() as i64,
            ascender,
            descender,
            height,
            round_ppem: font.glyf().is_ok() && head.flags().bits() & 8 != 0,
        })
    }

    /// `FT_Set_Pixel_Sizes` -> `FT_Request_Metrics` (+ `tt_size_reset`).
    pub fn size(&self, pixel_height: i32) -> SizeMetrics {
        let px = (pixel_height.max(1) as i64).min(0xFFFF);
        let scaled = px << 6;
        let scale = div_fix(scaled, self.units_per_em.abs());
        let ppem = (scaled + 32) >> 6;
        let (ascender, height) = if self.round_ppem {
            let s = div_fix(ppem << 6, self.units_per_em);
            (
                pix_round(mul_fix(self.ascender, s)),
                pix_round(mul_fix(self.height, s)),
            )
        } else {
            (
                pix_ceil(mul_fix(self.ascender, scale)),
                pix_round(mul_fix(self.height, scale)),
            )
        };
        SizeMetrics {
            ppem,
            x_scale: scale,
            y_scale: scale,
            ascender,
            height,
        }
    }
}

/// `fonts[i].baseline = qbr((m_up / m_height) * m_h)`, all in `float`.
pub fn baseline(size: &SizeMetrics, pixel_height: i32) -> i32 {
    let m_height = (size.height as f32 as f64 / 64.0) as f32;
    let m_up = (size.ascender as f32 as f64 / 64.0) as f32;
    let m_h = pixel_height as f32;
    let v = (m_up / m_height) * m_h;
    crate::round::qbr(crate::round::f80_from_f64(v as f64)) as i32
}

// ------------------------------------------------------------- glyphs ---

#[derive(Debug, Clone, Copy)]
enum El {
    Move(f32, f32),
    Line(f32, f32),
    Quad(f32, f32, f32, f32),
    Cubic(f32, f32, f32, f32, f32, f32),
}

#[derive(Default)]
struct Path(Vec<El>);

impl OutlinePen for Path {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push(El::Move(x, y));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push(El::Line(x, y));
    }
    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.0.push(El::Quad(cx0, cy0, x, y));
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0.push(El::Cubic(cx0, cy0, cx1, cy1, x, y));
    }
    fn close(&mut self) {}
}

/// What `FT_Load_Glyph` + `FT_Render_Glyph` left in the glyph slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphBitmap {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub rows: i32,
    /// `advance.x` in 26.6.
    pub advance: i64,
    /// `width * rows` coverage values; mono bitmaps hold 0 or 255.
    pub data: Vec<u8>,
}

fn to26(v: f32) -> i64 {
    (v as f64 * 64.0).round() as i64
}

/// Anti-aliased coverage of `path` on a `w x h` grid whose top-left corner is
/// at (`x0`, `y0`) in outline pixel units (y up): area coverage like
/// FreeType's "smooth" renderer (0..255).
fn rasterize_aa(path: &[El], x0: f32, y0: f32, w: usize, h: usize) -> Vec<u8> {
    use ab_glyph_rasterizer::{point, Rasterizer};
    let mut r = Rasterizer::new(w, h);
    for_each_segment(path, |seg| {
        let p = |x: f32, y: f32| point(x - x0, y0 - y);
        match seg {
            Seg::Line(a, b) => r.draw_line(p(a.0, a.1), p(b.0, b.1)),
            Seg::Quad(a, c, b) => r.draw_quad(p(a.0, a.1), p(c.0, c.1), p(b.0, b.1)),
            Seg::Cubic(a, c, d, b) => {
                r.draw_cubic(p(a.0, a.1), p(c.0, c.1), p(d.0, d.1), p(b.0, b.1))
            }
        }
    });
    let mut out = vec![0u8; w * h];
    r.for_each_pixel(|i, c| out[i] = ((c * 256.0) as i32).clamp(0, 255) as u8);
    out
}

/// Monochrome bitmap like FreeType's "raster1" renderer: a pixel is set when
/// its centre lies inside the outline (non-zero winding). FreeType's dropout
/// control for thin stems is not reproduced (D-R7-6).
fn rasterize_mono(path: &[El], x0: f32, y0: f32, w: usize, h: usize) -> Vec<u8> {
    let mut lines: Vec<((f32, f32), (f32, f32))> = Vec::new();
    for_each_segment(path, |seg| match seg {
        Seg::Line(a, b) => lines.push((a, b)),
        Seg::Quad(a, c, b) => {
            let n = 16;
            let mut prev = a;
            for i in 1..=n {
                let t = i as f32 / n as f32;
                let mt = 1.0 - t;
                let q = (
                    mt * mt * a.0 + 2.0 * mt * t * c.0 + t * t * b.0,
                    mt * mt * a.1 + 2.0 * mt * t * c.1 + t * t * b.1,
                );
                lines.push((prev, q));
                prev = q;
            }
        }
        Seg::Cubic(a, c, d, b) => {
            let n = 24;
            let mut prev = a;
            for i in 1..=n {
                let t = i as f32 / n as f32;
                let mt = 1.0 - t;
                let f = |p0: f32, p1: f32, p2: f32, p3: f32| {
                    mt * mt * mt * p0
                        + 3.0 * mt * mt * t * p1
                        + 3.0 * mt * t * t * p2
                        + t * t * t * p3
                };
                let q = (f(a.0, c.0, d.0, b.0), f(a.1, c.1, d.1, b.1));
                lines.push((prev, q));
                prev = q;
            }
        }
    });
    let mut out = vec![0u8; w * h];
    let mut xs: Vec<(f32, i32)> = Vec::new();
    for row in 0..h {
        let yc = y0 - (row as f32 + 0.5);
        xs.clear();
        for &((xa, ya), (xb, yb)) in &lines {
            if ya == yb {
                continue;
            }
            let (lo, hi) = if ya < yb { (ya, yb) } else { (yb, ya) };
            if yc < lo || yc >= hi {
                continue;
            }
            let x = xa + (yc - ya) * (xb - xa) / (yb - ya);
            xs.push((x, if yb > ya { 1 } else { -1 }));
        }
        xs.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut winding = 0;
        let mut start = 0.0;
        for &(x, d) in &xs {
            let was = winding;
            winding += d;
            if was == 0 && winding != 0 {
                start = x;
            } else if was != 0 && winding == 0 {
                let c0 = (start - x0 - 0.5).ceil().max(0.0) as usize;
                let c1 = (x - x0 - 0.5).floor();
                if c1 >= 0.0 {
                    for c in c0..=(c1 as usize).min(w.saturating_sub(1)) {
                        out[row * w + c] = 255;
                    }
                }
            }
        }
    }
    out
}

enum Seg {
    Line((f32, f32), (f32, f32)),
    Quad((f32, f32), (f32, f32), (f32, f32)),
    Cubic((f32, f32), (f32, f32), (f32, f32), (f32, f32)),
}

/// Walks the closed contours of `path` as absolute segments.
fn for_each_segment(path: &[El], mut f: impl FnMut(Seg)) {
    let mut cur = (0.0, 0.0);
    let mut start = cur;
    for el in path {
        match *el {
            El::Move(x, y) => {
                if cur != start {
                    f(Seg::Line(cur, start));
                }
                cur = (x, y);
                start = cur;
            }
            El::Line(x, y) => {
                f(Seg::Line(cur, (x, y)));
                cur = (x, y);
            }
            El::Quad(cx, cy, x, y) => {
                f(Seg::Quad(cur, (cx, cy), (x, y)));
                cur = (x, y);
            }
            El::Cubic(ax, ay, bx, by, x, y) => {
                f(Seg::Cubic(cur, (ax, ay), (bx, by), (x, y)));
                cur = (x, y);
            }
        }
    }
    if cur != start {
        f(Seg::Line(cur, start));
    }
}

/// `hdmx` device width of `gid` at `ppem` (`tt_face_load_hdmx` +
/// `tt_face_get_device_metrics`).
fn hdmx_width(font: &FontRef, ppem: i64, gid: u32) -> Option<i64> {
    let data = font.table_data(skrifa::raw::types::Tag::new(b"hdmx"))?;
    let d = data.as_bytes();
    if d.len() < 8 {
        return None;
    }
    let version = u16::from_be_bytes([d[0], d[1]]);
    let num_records = u16::from_be_bytes([d[2], d[3]]) as usize;
    let mut record_size = u32::from_be_bytes([d[4], d[5], d[6], d[7]]) as usize;
    if record_size >= 0xFFFF_0000 {
        record_size &= 0xFFFF;
    }
    if version != 0 || num_records > 255 || record_size > 0x10001 {
        return None;
    }
    let mut p = 8;
    for _ in 0..num_records {
        if p + record_size > d.len() {
            break;
        }
        if record_size > 0 && d[p] as i64 == ppem {
            let i = gid as usize + 2;
            return (i < record_size).then(|| d[p + i] as i64);
        }
        p += record_size;
    }
    None
}

/// FreeType 2.4.9 `FT_Load_Glyph(FT_LOAD_DEFAULT)`: TrueType fonts whose
/// glyphs carry no instructions (`maxp.maxSizeOfInstructions == 0`) go to the
/// auto-hinter, everything else to the native hinter (TrueType bytecode in
/// full "v35" strength, which `Target::Mono` selects; CFF hints).
fn hinting_options(font: &FontRef) -> HintingOptions {
    let no_instructions = font.glyf().is_ok()
        && font
            .maxp()
            .ok()
            .and_then(|m| m.max_size_of_instructions())
            .unwrap_or(0)
            == 0;
    if no_instructions {
        HintingOptions {
            engine: Engine::Auto(None),
            target: Target::Smooth {
                mode: SmoothMode::Normal,
                symmetric_rendering: true,
                preserve_linear_metrics: false,
            },
        }
    } else {
        HintingOptions {
            engine: Engine::Interpreter,
            target: Target::Mono,
        }
    }
}

struct Font {
    data: Box<[u8]>,
    index: u32,
    default_pixel_height: i32,
    size: SizeMetrics,
    hinting: Option<HintingInstance>,
    fixed_pitch: bool,
    baseline: i32,
    monospace: bool,
    monospace_width: i32,
}

impl Font {
    fn face(&self) -> Option<FontRef<'_>> {
        FontRef::from_index(&self.data, self.index).ok()
    }

    /// `FT_Get_Char_Index` + `FT_Load_Glyph(FT_LOAD_DEFAULT)` + `FT_Render_Glyph`.
    fn glyph(&self, codepoint: u32, mono: bool) -> GlyphBitmap {
        let empty = GlyphBitmap {
            left: 0,
            top: 0,
            width: 0,
            rows: 0,
            advance: 0,
            data: Vec::new(),
        };
        let Some(font) = self.face() else {
            return empty;
        };
        let gid = font.charmap().map(codepoint).unwrap_or(GlyphId::new(0));
        let outlines = font.outline_glyphs();
        let mut path = Path::default();
        let mut advance = None;
        if let Some(glyph) = outlines.get(gid) {
            let size = Size::new(self.size.ppem as f32);
            let drawn = match &self.hinting {
                Some(h) => glyph.draw(DrawSettings::hinted(h, false), &mut path),
                None => glyph.draw(
                    DrawSettings::unhinted(size, LocationRef::default()),
                    &mut path,
                ),
            };
            match drawn {
                Ok(m) => advance = m.advance_width.map(to26),
                Err(_) => path.0.clear(),
            }
        }
        let mut advance = match advance {
            Some(a) => pix_round(a),
            None => {
                let units = font
                    .glyph_metrics(Size::unscaled(), LocationRef::default())
                    .advance_width(gid)
                    .unwrap_or(0.0) as i64;
                pix_round(mul_fix(units, self.size.x_scale))
            }
        };
        if !self.fixed_pitch
            && let Some(w) = hdmx_width(&font, self.size.ppem, gid.to_u32()) {
                advance = w << 6;
            }
        // Control box in 26.6 (on- and off-curve points), as FreeType's cbox.
        let (mut x_min, mut y_min, mut x_max, mut y_max) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
        let mut add = |x: f32, y: f32| {
            let (x, y) = (to26(x), to26(y));
            x_min = x_min.min(x);
            y_min = y_min.min(y);
            x_max = x_max.max(x);
            y_max = y_max.max(y);
        };
        for el in &path.0 {
            match *el {
                El::Move(x, y) | El::Line(x, y) => add(x, y),
                El::Quad(a, b, x, y) => {
                    add(a, b);
                    add(x, y);
                }
                El::Cubic(a, b, c, d, x, y) => {
                    add(a, b);
                    add(c, d);
                    add(x, y);
                }
            }
        }
        if path.0.is_empty() {
            return GlyphBitmap { advance, ..empty };
        }
        // ft_raster1_render rounds the control box, ft_smooth_render floors/ceils it.
        let (x_min, y_min, x_max, y_max) = if mono {
            (
                pix_round(x_min),
                pix_round(y_min),
                pix_round(x_max),
                pix_round(y_max),
            )
        } else {
            (
                pix_floor(x_min),
                pix_floor(y_min),
                pix_ceil(x_max),
                pix_ceil(y_max),
            )
        };
        let width = ((x_max - x_min) >> 6) as usize;
        let rows = ((y_max - y_min) >> 6) as usize;
        let (x0, y0) = ((x_min >> 6) as f32, (y_max >> 6) as f32);
        let data = if mono {
            rasterize_mono(&path.0, x0, y0, width, rows)
        } else {
            rasterize_aa(&path.0, x0, y0, width, rows)
        };
        GlyphBitmap {
            left: (x_min >> 6) as i32,
            top: (y_max >> 6) as i32,
            width: width as i32,
            rows: rows as i32,
            advance,
            data,
        }
    }
}

/// One rendered string, as returned through `FontRenderTextUTF32`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    pub data: Vec<u8>,
    pub w: i32,
    pub h: i32,
    pub pre_increment: i32,
    pub post_increment: i32,
}

/// The legacy `fonts[]` table; index 0 is never used.
static FONTS: Mutex<Vec<Option<Font>>> = Mutex::new(Vec::new());

fn table() -> std::sync::MutexGuard<'static, Vec<Option<Font>>> {
    FONTS.lock().unwrap_or_else(|e| e.into_inner())
}

/// `FontLoad`: returns a handle >= 1, or 0 on failure.
/// `options`: 16 = monospace (other bits are handled by libqb.cpp).
pub fn load(content: &[u8], default_pixel_height: i32, which_font: i32, options: i32) -> i32 {
    let which_font = if which_font == -1 { 0 } else { which_font };
    let mut fonts = table();
    if fonts.is_empty() {
        fonts.push(None);
    }
    let i = match fonts.iter().skip(1).position(|f| f.is_none()) {
        Some(p) => p + 1,
        None => {
            fonts.push(None);
            fonts.len() - 1
        }
    };
    let data: Box<[u8]> = content.into();
    let Ok(face) = FontRef::from_index(&data, which_font as u32) else {
        return 0;
    };
    let Some(metrics) = FaceMetrics::read(&face) else {
        return 0;
    };
    if metrics.units_per_em == 0 {
        return 0;
    }
    let size = metrics.size(default_pixel_height);
    let outlines = face.outline_glyphs();
    let hinting = HintingInstance::new(
        &outlines,
        Size::new(size.ppem as f32),
        LocationRef::default(),
        hinting_options(&face),
    )
    .ok();
    let fixed_pitch = face
        .post()
        .map(|p| p.is_fixed_pitch() != 0)
        .unwrap_or(false);
    let mut font = Font {
        data,
        index: which_font as u32,
        default_pixel_height,
        baseline: baseline(&size, default_pixel_height),
        size,
        hinting,
        fixed_pitch,
        monospace: false,
        monospace_width: 0,
    };
    if options & 16 != 0 {
        // the width of capital W, rendered in monochrome
        let r = render_font(&font, &[87], 1).expect("one codepoint always renders");
        font.monospace_width = r.w;
        font.monospace = true;
    }
    fonts[i] = Some(font);
    i as i32
}

/// `FontFree`
pub fn free(i: i32) {
    if let Some(slot) = table().get_mut(i as usize) {
        *slot = None;
    }
}

/// `FontWidth`: the monospace width, or 0 for proportional fonts.
pub fn width(i: i32) -> i32 {
    match table().get(i as usize) {
        Some(Some(f)) if f.monospace => f.monospace_width,
        _ => 0,
    }
}

/// `FontRenderTextUTF32`: `None` = failure (return 0).
/// `options`: 1 = monochrome (0 or 255 per pixel).
pub fn render(i: i32, codepoints: &[u32], options: i32) -> Option<Rendered> {
    let fonts = table();
    let font = fonts.get(i as usize)?.as_ref()?;
    render_font(font, codepoints, options)
}

fn render_font(font: &Font, codepoints: &[u32], options: i32) -> Option<Rendered> {
    let h2 = font.default_pixel_height;
    if codepoints.is_empty() {
        return Some(Rendered {
            data: Vec::new(),
            w: 0,
            h: h2,
            pre_increment: 0,
            post_increment: 0,
        });
    }
    let mono = options & 1 != 0;
    let mut pre_increment = 0;
    let mut parts: Vec<(Vec<u8>, i32, i32)> = Vec::with_capacity(codepoints.len());
    let (mut codepoint_w, mut codepoint_ox) = (0i32, 0i32);
    for (ci, &cp) in codepoints.iter().enumerate() {
        let g = font.glyph(cp, mono);
        let (w1, h1) = (g.width, g.rows);
        let mut ox = g.left;
        let mut w2 = (g.advance / 64) as i32;
        if w2 < w1 {
            w2 = w1;
        }
        let mut ox2 = 0;
        if ox > 0 {
            if w1 + ox > w2 {
                w2 = w1 + ox;
            }
            ox2 = ox;
        }
        if ox < 0 {
            w2 += -ox;
        }
        if font.monospace && w2 != font.monospace_width {
            w2 = font.monospace_width;
            ox = 0;
            ox2 = w2 / 2 - w1 / 2;
        }
        let mut data2 = vec![0u8; (w2.max(0) as usize) * (h2.max(0) as usize)];
        let oy2 = font.baseline - g.top;
        for y1 in 0..h1 {
            let y2 = y1 + oy2;
            if y2 < 0 || y2 >= h2 {
                continue;
            }
            for x1 in 0..w1 {
                let x2 = x1 + ox2;
                if x2 >= 0 && x2 < w2 {
                    data2[(x2 + y2 * w2) as usize] = g.data[(x1 + y1 * w1) as usize];
                }
            }
        }
        if codepoints.len() == 1 {
            let pre = if ox < 0 { ox } else { 0 };
            return Some(Rendered {
                data: data2,
                w: w2,
                h: h2,
                pre_increment: pre,
                post_increment: 0,
            });
        }
        if ci == 0 {
            pre_increment = if ox < 0 { ox } else { 0 };
        } else if ox < 0 {
            codepoint_ox = if codepoint_ox + ox >= 0 {
                codepoint_ox + ox
            } else {
                0
            };
        }
        parts.push((data2, w2, codepoint_ox));
        codepoint_ox += w2;
        if codepoint_ox > codepoint_w {
            codepoint_w = codepoint_ox;
        }
    }
    let w2 = codepoint_w;
    let mut data2 = vec![0u8; (w2.max(0) as usize) * (h2.max(0) as usize)];
    for (data1, w1, ox2) in parts {
        for y1 in 0..h2 {
            for x1 in 0..w1 {
                let v = data1[(x1 + y1 * w1) as usize];
                let o = (x1 + ox2 + y1 * w2) as usize;
                if v > data2[o] {
                    data2[o] = v;
                }
            }
        }
    }
    Some(Rendered {
        data: data2,
        w: w2,
        h: h2,
        pre_increment,
        post_increment: 0,
    })
}

// ------------------------------------------------------------- C ABI ---

/// # Safety
/// `content` must point to `bytes` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_font_load(
    content: *const u8,
    bytes: i32,
    pixel_height: i32,
    which_font: i32,
    options: i32,
) -> i32 {
    if content.is_null() || bytes <= 0 {
        return 0;
    }
    // SAFETY: caller guarantees the buffer.
    load(
        unsafe { std::slice::from_raw_parts(content, bytes as usize) },
        pixel_height,
        which_font,
        options,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_font_free(i: i32) {
    free(i)
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_font_width(i: i32) -> i32 {
    width(i)
}

/// Same contract as the legacy `FontRenderTextUTF32`; `*out_data` is
/// `malloc`ed (NULL when `codepoints` <= 0) and released by the caller.
///
/// # Safety
/// `codepoint` must hold `codepoints` values; the out-pointers must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_font_render_utf32(
    i: i32,
    codepoint: *const u32,
    codepoints: i32,
    options: i32,
    out_data: *mut *mut u8,
    out_x: *mut i32,
    out_y: *mut i32,
    out_x_pre_increment: *mut i32,
    out_x_post_increment: *mut i32,
) -> i32 {
    // SAFETY (whole body): caller guarantees writable out-pointers and
    // `codepoints` readable values (possibly unaligned).
    unsafe {
        if codepoints <= 0 {
            *out_data = std::ptr::null_mut();
            *out_x = 0;
            *out_y = default_height(i);
            *out_x_pre_increment = 0;
            *out_x_post_increment = 0;
            return (codepoints == 0) as i32;
        }
        let cps: Vec<u32> = (0..codepoints as usize)
            .map(|k| codepoint.add(k).read_unaligned())
            .collect();
        match render(i, &cps, options) {
            Some(r) => {
                *out_data = crate::cmem::to_c_heap(&r.data);
                *out_x = r.w;
                *out_y = r.h;
                *out_x_pre_increment = r.pre_increment;
                *out_x_post_increment = r.post_increment;
                1
            }
            None => {
                // unknown handle (undefined behaviour in the legacy code)
                *out_data = std::ptr::null_mut();
                *out_x = 0;
                *out_y = 0;
                *out_x_pre_increment = 0;
                *out_x_post_increment = 0;
                0
            }
        }
    }
}

fn default_height(i: i32) -> i32 {
    match table().get(i as usize) {
        Some(Some(f)) => f.default_pixel_height,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font_file(name: &str) -> Vec<u8> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/diff/data/fonts")
            .join(name);
        std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
    }

    fn fnv1a(b: &[u8]) -> u64 {
        b.iter().fold(0xcbf29ce484222325u64, |h, &x| {
            (h ^ x as u64).wrapping_mul(0x100000001b3)
        })
    }

    // The font table is global; tests that load fonts take this lock.
    static SERIAL: Mutex<()> = Mutex::new(());

    #[test]
    fn fixed_point_matches_freetype() {
        assert_eq!(mul_fix(1000, 0x10000), 1000);
        assert_eq!(mul_fix(-3, 0x8000), -2); // -1.5 rounds away from zero
        assert_eq!(mul_fix(3, 0x8000), 2);
        assert_eq!(div_fix(16 << 6, 1000), 67109);
        assert_eq!(div_fix(-1, 3), -21845);
        assert_eq!(div_fix(5, 0), 0x7FFF_FFFF);
        assert_eq!((pix_round(95), pix_ceil(65), pix_floor(-1)), (64, 128, -64));
    }

    /// (pixel height, ascender, height in 26.6, baseline) captured from the
    /// legacy FreeType 2.4.9 wrapper (`diff_font --dump`).
    #[test]
    fn size_metrics_and_baseline_match_legacy() {
        for (file, want) in [
            (
                "NotoSans-Subset.ttf",
                [
                    (8, 576, 704, 7),
                    (13, 896, 1152, 10),
                    (16, 1152, 1408, 13),
                    (24, 1664, 2112, 19),
                    (40, 2752, 3456, 32),
                ],
            ),
            (
                "NotoSansThai-Regular.ttf",
                [
                    (8, 576, 768, 6),
                    (13, 896, 1280, 9),
                    (16, 1088, 1536, 11),
                    (24, 1664, 2304, 17),
                    (40, 2752, 3840, 29),
                ],
            ),
        ] {
            let data = font_file(file);
            let face = FontRef::new(&data).unwrap();
            let m = FaceMetrics::read(&face).unwrap();
            for (px, asc, height, base) in want {
                let s = m.size(px);
                assert_eq!(
                    (s.ascender, s.height, baseline(&s, px)),
                    (asc, height, base),
                    "{file} {px}px"
                );
            }
        }
    }

    /// NotoSansSymbols2 (TrueType bytecode hinted) against FreeType:
    /// (codepoint, options, w, h, pre-increment, FNV-1a of the pixels).
    #[test]
    fn hinted_symbols_match_legacy_metrics() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let data = font_file("NotoSansSymbols2-Subset.ttf");
        #[rustfmt::skip]
        type Glyph = (u32, i32, i32, i32, i32, u64);
        let cases: [(i32, &[Glyph]); 2] = [
            (
                0,
                &[
                    (0x25A0, 0, 13, 16, 0, 0xfb212fb870163d4b),
                    (0x25A0, 1, 13, 16, 0, 0xe43de46d98135f75),
                    (0x25B2, 0, 15, 16, 0, 0x3d8c5e359ed187df),
                    (0x25B2, 1, 15, 16, 0, 0x2790055ac6f18e2b),
                    (0x25CF, 0, 14, 16, 0, 0x75847d49322ad409),
                    (0x25CF, 1, 14, 16, 0, 0x192d4b8e186c54ad),
                    (0x25C6, 0, 18, 16, 0, 0xc63f5f874a4cbc6b),
                    (0x25C6, 1, 18, 16, 0, 0xafc5ee61e5b9f319),
                ],
            ),
            (
                16,
                &[
                    (0x25A0, 0, 10, 16, 0, 0x66136aebdb3804e1),
                    (0x25A0, 1, 10, 16, 0, 0x66136aebdb3804e1),
                    (0x25B2, 0, 10, 16, 0, 0x5c0e2ad7f350da9e),
                    (0x25B2, 1, 10, 16, 0, 0xe3f8d75224cc9601),
                    (0x25CF, 0, 10, 16, 0, 0x469b429833aedae3),
                    (0x25CF, 1, 10, 16, 0, 0x447c62fb8ba2394e),
                    (0x25C6, 0, 10, 16, 0, 0x169997e3a067ef7b),
                    (0x25C6, 1, 10, 16, 0, 0x136842bc90c0df73),
                ],
            ),
        ];
        for (options, glyphs) in cases {
            let h = load(&data, 16, -1, options);
            assert!(h > 0);
            assert_eq!(width(h), if options & 16 != 0 { 10 } else { 0 });
            for &(cp, opt, w, hh, pre, hash) in glyphs {
                let r = render(h, &[cp], opt).unwrap();
                assert_eq!(
                    (r.w, r.h, r.pre_increment),
                    (w, hh, pre),
                    "U+{cp:04X} opt {opt} options {options}"
                );
                // Axis-aligned outlines rasterise identically; diagonal and
                // curved edges may differ in coverage (D-R7-6).
                if cp == 0x25A0 {
                    assert_eq!(
                        fnv1a(&r.data),
                        hash,
                        "U+{cp:04X} opt {opt} options {options}"
                    );
                }
            }
            free(h);
        }
    }

    #[test]
    fn handles_are_reused_and_bad_data_fails() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let data = font_file("NotoSans-Subset.ttf");
        assert_eq!(load(b"not a font at all", 16, -1, 0), 0);
        let a = load(&data, 16, -1, 0);
        let b = load(&data, 20, -1, 0);
        assert!(a >= 1 && b == a + 1);
        free(a);
        assert_eq!(load(&data, 12, -1, 0), a);
        assert_eq!(load(&data, 12, 3, 0), 0); // no face 3 in a plain TTF
        free(a);
        free(b);
        assert!(render(b, &[65], 0).is_none());
    }

    #[test]
    fn strings_join_glyph_cells_with_max_blend() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let data = font_file("NotoSans-Subset.ttf");
        let h = load(&data, 16, -1, 0);
        let empty = render(h, &[], 0).unwrap();
        assert_eq!((empty.w, empty.h, empty.data.len()), (0, 16, 0));
        let a = render(h, &[72], 0).unwrap();
        let b = render(h, &[105], 0).unwrap();
        let ab = render(h, &[72, 105], 0).unwrap();
        assert_eq!((a.pre_increment, b.pre_increment), (0, 0));
        assert_eq!(ab.w, a.w + b.w);
        for y in 0..16usize {
            let row = &ab.data[y * ab.w as usize..(y + 1) * ab.w as usize];
            assert_eq!(
                &row[..a.w as usize],
                &a.data[y * a.w as usize..(y + 1) * a.w as usize]
            );
            assert_eq!(
                &row[a.w as usize..],
                &b.data[y * b.w as usize..(y + 1) * b.w as usize]
            );
        }
        // monochrome output only holds 0 and 255
        assert!(render(h, &[64, 103], 1)
            .unwrap()
            .data
            .iter()
            .all(|&v| v == 0 || v == 255));
        free(h);
    }

    #[test]
    fn ffi_contract() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let data = font_file("NotoSans-Subset.ttf");
        // SAFETY: valid buffers and out-pointers; the result is freed with free().
        unsafe {
            let h = qb_font_load(data.as_ptr(), data.len() as i32, 16, -1, 16);
            assert!(h > 0);
            let (mut p, mut x, mut y, mut pre, mut post) = (std::ptr::null_mut(), 0, 0, 0, 0);
            assert_eq!(
                qb_font_render_utf32(
                    h,
                    std::ptr::null(),
                    0,
                    0,
                    &mut p,
                    &mut x,
                    &mut y,
                    &mut pre,
                    &mut post
                ),
                1
            );
            assert!(p.is_null());
            assert_eq!((x, y), (0, 16));
            assert_eq!(
                qb_font_render_utf32(
                    h,
                    std::ptr::null(),
                    -1,
                    0,
                    &mut p,
                    &mut x,
                    &mut y,
                    &mut pre,
                    &mut post
                ),
                0
            );
            let cps = [65u32, 66];
            assert_eq!(
                qb_font_render_utf32(
                    h,
                    cps.as_ptr(),
                    2,
                    0,
                    &mut p,
                    &mut x,
                    &mut y,
                    &mut pre,
                    &mut post
                ),
                1
            );
            assert_eq!((x, y), (2 * qb_font_width(h), 16));
            unsafe extern "C" {
                fn free(p: *mut std::os::raw::c_void);
            }
            free(p as *mut _);
            qb_font_free(h);
        }
    }
}
