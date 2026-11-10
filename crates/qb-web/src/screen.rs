//! Turns a screen page record (`qb_runtime::gfx::capture` format) into what a
//! canvas shows: RGBA pixels, plus the characters of a text page.
//!
//! Text pages (SCREEN 0) are drawn with the builtin 8x16 font (8x8 above 30
//! rows, double width at 40 columns or less); palette pages (SCREEN 1-13 and
//! 8-bit `_NEWIMAGE`) map indexes through the page palette; 32-bit pages are
//! copied.

use qb_runtime::gfx::data::{charset8x16, charset8x8};

/// A decoded screen page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Page {
    /// Columns and rows (text) or pixels (graphics).
    pub width: u32,
    pub height: u32,
    pub bytes_per_pixel: u32,
    /// `SCREEN` mode (0 = text).
    pub mode: u32,
    pub data: Vec<u8>,
    /// ARGB palette (empty for 32-bit pages).
    pub palette: Vec<u32>,
}

/// What the page draws: pixel size, RGBA bytes and (text pages) the text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub text: String,
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// Parses one `QBFB` record.
pub fn parse(rec: &[u8]) -> Option<Page> {
    if u32_at(rec, 0)? != qb_runtime::gfx::capture::MAGIC {
        return None;
    }
    let (width, height, bpp, mode) = (u32_at(rec, 8)?, u32_at(rec, 12)?, u32_at(rec, 16)?, u32_at(rec, 20)?);
    // R14: a page has 1..=4 bytes per pixel, and the RGBA frame (4 bytes per
    // pixel) must fit usize (wasm32); bpp 0 made `render` index empty data.
    if !(1..=4).contains(&bpp) {
        return None;
    }
    (width as usize).checked_mul(height as usize)?.checked_mul(4)?;
    let n = (width as usize).checked_mul(height as usize)?.checked_mul(bpp as usize)?;
    let data = rec.get(24..24 + n)?.to_vec();
    let npal = u32_at(rec, 24 + n)? as usize;
    let palette = (0..npal).map(|i| u32_at(rec, 28 + n + 4 * i)).collect::<Option<Vec<_>>>()?;
    Some(Page { width, height, bytes_per_pixel: bpp, mode, data, palette })
}

fn put(rgba: &mut [u8], i: usize, argb: u32) {
    rgba[4 * i] = (argb >> 16) as u8;
    rgba[4 * i + 1] = (argb >> 8) as u8;
    rgba[4 * i + 2] = argb as u8;
    rgba[4 * i + 3] = 255;
}

/// Character a text cell shows as text (control codes and NUL as spaces;
/// CP437 above 127 is kept as the Latin-1 code point for the page to map).
fn cell_char(c: u8) -> char {
    match c {
        0..=31 => ' ',
        _ => char::from(c),
    }
}

/// Renders a page.
pub fn render(p: &Page) -> Frame {
    if p.mode == 0 {
        return render_text(p);
    }
    let (w, h) = (p.width as usize, p.height as usize);
    let mut rgba = vec![0u8; w * h * 4];
    for i in 0..w * h {
        let argb = if p.bytes_per_pixel == 4 {
            u32::from_le_bytes(p.data[4 * i..4 * i + 4].try_into().unwrap())
        } else {
            let k = p.data[i] as usize;
            p.palette.get(k).copied().unwrap_or(0)
        };
        put(&mut rgba, i, argb);
    }
    Frame { width: p.width, height: p.height, rgba, text: String::new() }
}

fn render_text(p: &Page) -> Frame {
    let (cols, rows) = (p.width as usize, p.height as usize);
    let (fh, font): (usize, &[u8]) = if rows > 30 { (8, &charset8x8[..]) } else { (16, &charset8x16[..]) };
    let sx = if cols <= 40 { 2 } else { 1 };
    let (w, h) = (cols * 8 * sx, rows * fh);
    let mut rgba = vec![0u8; w * h * 4];
    let mut text = String::with_capacity((cols + 1) * rows);
    let pal = |k: usize| p.palette.get(k).copied().unwrap_or(if k == 0 { 0 } else { 0xFFAA_AAAA });
    for row in 0..rows {
        let mut line = String::with_capacity(cols);
        for col in 0..cols {
            let at = 2 * (row * cols + col);
            let (ch, attr) = (p.data.get(at).copied().unwrap_or(0), p.data.get(at + 1).copied().unwrap_or(7));
            line.push(cell_char(ch));
            let (fg, bg) = (pal(usize::from(attr & 15)), pal(usize::from((attr >> 4) & 7)));
            for y in 0..fh {
                for x in 0..8 * sx {
                    let on = font[(usize::from(ch) * fh + y) * 8 + x / sx] != 0;
                    let i = (row * fh + y) * w + col * 8 * sx + x;
                    put(&mut rgba, i, if on { fg } else { bg });
                }
            }
        }
        text.push_str(line.trim_end());
        text.push('\n');
    }
    while text.ends_with("\n\n") {
        text.pop();
    }
    if text == "\n" {
        text.clear();
    }
    Frame { width: w as u32, height: h as u32, rgba, text }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(w: u32, h: u32, bpp: u32, mode: u32, data: &[u8], pal: &[u32]) -> Vec<u8> {
        let mut v = Vec::new();
        for x in [qb_runtime::gfx::capture::MAGIC, 0, w, h, bpp, mode] {
            v.extend_from_slice(&x.to_le_bytes());
        }
        v.extend_from_slice(data);
        v.extend_from_slice(&(pal.len() as u32).to_le_bytes());
        for p in pal {
            v.extend_from_slice(&p.to_le_bytes());
        }
        v
    }

    #[test]
    fn palette_page_maps_indexes() {
        let p = parse(&rec(2, 1, 1, 13, &[1, 0], &[0xFF00_0000, 0xFFAA_0000])).unwrap();
        let f = render(&p);
        assert_eq!((f.width, f.height), (2, 1));
        assert_eq!(f.rgba, vec![0xAA, 0, 0, 255, 0, 0, 0, 255]);
    }

    #[test]
    fn rgb32_page_is_copied() {
        let px = 0xFF12_3456u32.to_le_bytes();
        let f = render(&parse(&rec(1, 1, 4, 32, &px, &[])).unwrap());
        assert_eq!(f.rgba, vec![0x12, 0x34, 0x56, 255]);
    }

    #[test]
    fn text_page_draws_glyphs_and_keeps_text() {
        let mut cells = vec![0u8; 80 * 25 * 2];
        cells[0] = b'A';
        cells[1] = 0x1E; // yellow on blue
        cells[2] = b'B';
        cells[3] = 7;
        let mut pal = vec![0u32; 256];
        pal[1] = 0xFF00_00AA;
        pal[7] = 0xFFAA_AAAA;
        pal[14] = 0xFFFF_FF55;
        let f = render(&parse(&rec(80, 25, 2, 0, &cells, &pal)).unwrap());
        assert_eq!((f.width, f.height), (640, 400));
        assert_eq!(f.text, "AB\n");
        // top-left pixel of 'A' cell is background (blue), some pixel is foreground (yellow)
        assert_eq!(&f.rgba[0..4], &[0, 0, 0xAA, 255]);
        let cell: Vec<&[u8]> = (0..16).flat_map(|y| (0..8).map(move |x| (y * 640 + x) * 4)).map(|i| &f.rgba[i..i + 4]).collect();
        assert!(cell.iter().any(|p| *p == [0xFF, 0xFF, 0x55, 255]));
    }

    #[test]
    fn bad_records_are_rejected() {
        assert_eq!(parse(b"nope"), None);
        let mut r = rec(4, 4, 1, 13, &[0; 16], &[]);
        r.truncate(30);
        assert_eq!(parse(&r), None);
    }

    /// R14: odd pixel sizes must not panic in `render`.
    #[test]
    fn odd_pixel_sizes_do_not_panic() {
        for bpp in [0u32, 2, 3, 5, 9] {
            let data = vec![1u8; 4 * 4 * bpp as usize];
            if let Some(p) = parse(&rec(4, 4, bpp, 13, &data, &[0xFF00_0000; 4])) {
                let f = render(&p);
                assert_eq!(f.rgba.len(), 4 * 4 * 4);
            }
        }
        assert_eq!(parse(&rec(0x10000, 0x10000, 1, 13, &[], &[])), None);
    }
}
