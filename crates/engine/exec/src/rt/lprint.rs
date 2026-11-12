//! LPRINT: text for the printer.
//!
//! Lines collect on a page of [`PAGE_LINES`] lines of up to `WIDTH LPRINT` columns
//! (80 by default). A full page, CHR$(12) (form feed) or the end of the program sends the
//! page:
//! - `QBNEX_LPRINT=<file>`: the text is appended to that file (CR LF line ends, CHR$(12)
//!   after each page), on every system;
//! - otherwise Windows prints the page on the default printer as an image of 8x16
//!   characters scaled to the paper (as QB64 did), and other systems hand the text to `lp`.
//!
//! `_PRINTIMAGE` prints an image the same way (scaled to the paper); with
//! `QBNEX_LPRINT` set it appends a line with the image's size and a hash of its pixels.
//!
//! A missing printer is not an error: the page is dropped, as in QB64.

/// Lines on one page (QB64's LPRINT page holds 60 lines of 8x16 characters).
pub(crate) const PAGE_LINES: usize = 60;

/// LPRINT's file number in QIR (`rt PRINTS -1 text`): file numbers are positive and 0 is
/// the screen.
pub(crate) const CHANNEL: i32 = -1;

pub(crate) struct Lprint {
    page: Vec<Vec<u8>>,
    line: Vec<u8>,
    /// `WIDTH LPRINT`
    pub(crate) width: usize,
    /// where finished pages go (tests collect them)
    sink: fn(&[Vec<u8>], usize),
}

impl Default for Lprint {
    fn default() -> Self {
        Lprint { page: Vec::new(), line: Vec::new(), width: 80, sink: send }
    }
}

impl Drop for Lprint {
    fn drop(&mut self) {
        self.eject();
    }
}

impl Lprint {
    /// LPOS: column of the next character, from 1.
    pub(crate) fn pos(&self) -> usize {
        self.line.len() + 1
    }

    pub(crate) fn write(&mut self, b: &[u8]) {
        for &c in b {
            match c {
                b'\n' => self.newline(),
                b'\r' => {}
                12 => {
                    if !self.line.is_empty() {
                        self.newline();
                    }
                    self.eject();
                }
                _ => {
                    if self.line.len() >= self.width {
                        self.newline();
                    }
                    self.line.push(c);
                }
            }
        }
    }

    pub(crate) fn newline(&mut self) {
        self.page.push(std::mem::take(&mut self.line));
        if self.page.len() >= PAGE_LINES {
            self.eject();
        }
    }

    /// PRINT's comma: on to the next 14-column zone, or a new line when none is left.
    pub(crate) fn zone(&mut self) {
        let next = (self.line.len() / 14 + 1) * 14;
        if next >= self.width {
            self.newline();
        } else {
            self.line.resize(next, b' ');
        }
    }

    /// Sends the page (with a line still being printed) when anything is on it.
    pub(crate) fn eject(&mut self) {
        if !self.line.is_empty() {
            self.page.push(std::mem::take(&mut self.line));
        }
        if self.page.is_empty() {
            return;
        }
        let page = std::mem::take(&mut self.page);
        (self.sink)(&page, self.width);
    }
}

/// The page as text: CR LF after each line, then a form feed.
fn page_text(page: &[Vec<u8>]) -> Vec<u8> {
    let mut t = Vec::new();
    for l in page {
        t.extend_from_slice(l);
        t.extend_from_slice(b"\r\n");
    }
    t.push(12);
    t
}

/// Appends `b` to the `QBNEX_LPRINT` file; false when the variable is not set.
fn to_file(b: &[u8]) -> bool {
    let Some(path) = std::env::var_os("QBNEX_LPRINT").filter(|p| !p.is_empty()) else { return false };
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = f.write_all(b);
    }
    true
}

fn send(page: &[Vec<u8>], width: usize) {
    if to_file(&page_text(page)) {
        return;
    }
    #[cfg(windows)]
    print_scaled(width.max(1) * 8, PAGE_LINES * 16, |x, y| page_pixel(page, x, y));
    #[cfg(unix)]
    {
        let _ = width;
        lp(&page_text(page));
    }
    #[cfg(not(any(windows, unix)))]
    let _ = (page, width);
}

/// Pixel of the page image (8x16 characters, black on white) at `(x, y)`.
fn page_pixel(page: &[Vec<u8>], x: usize, y: usize) -> u32 {
    use qb_runtime::gfx::data::charset8x16;
    let (row, col) = (y / 16, x / 8);
    let c = page.get(row).and_then(|l| l.get(col)).copied().unwrap_or(b' ');
    if charset8x16[(usize::from(c) * 16 + y % 16) * 8 + x % 8] != 0 { 0 } else { 0x00FF_FFFF }
}

/// `_PRINTIMAGE`: `w` x `h` pixels, 0x00RRGGBB, top row first.
pub(crate) fn print_image(w: usize, h: usize, px: &[u32]) {
    // FNV-1a 64 of the pixels: the test record of the image
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for p in px {
        for b in p.to_le_bytes() {
            hash = (hash ^ u64::from(b)).wrapping_mul(0x100_0000_01b3);
        }
    }
    if to_file(format!("[_PRINTIMAGE {w}x{h} {hash:016x}]\r\n\x0c").as_bytes()) || w == 0 || h == 0 {
        return;
    }
    #[cfg(windows)]
    print_scaled(w, h, |x, y| px[y * w + x] & 0x00FF_FFFF);
    #[cfg(unix)]
    {
        // a binary PPM, which CUPS converts
        let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
        for p in px {
            ppm.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
        }
        lp(&ppm);
    }
}

/// Prints an `iw` x `ih` image, `pixel(x, y)`, scaled to fit the paper of the default printer.
#[cfg(windows)]
fn print_scaled(iw: usize, ih: usize, pixel: impl Fn(usize, usize) -> u32) {
    use qb_runtime::printer;
    let mut d = printer::gdi::Gdi { dc: std::ptr::null_mut() };
    let Some((pw, ph)) = printer::begin(&mut d) else { return };
    // the whole image fits the paper, keeping its shape
    let scale = (f64::from(pw) / iw as f64).min(f64::from(ph) / ih as f64);
    if pw <= 0 || ph <= 0 || scale <= 0.0 {
        printer::abort(&mut d);
        return;
    }
    printer::finish_rows(&mut d, pw, ph, |y, row| {
        let sy = (f64::from(y) / scale) as usize;
        for (x, px) in row.iter_mut().enumerate() {
            let sx = (x as f64 / scale) as usize;
            *px = if sx < iw && sy < ih { pixel(sx, sy) } else { 0x00FF_FFFF };
        }
    });
}

#[cfg(unix)]
fn lp(text: &[u8]) {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let Ok(mut child) = Command::new("lp").stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() else {
        return;
    };
    if let Some(mut i) = child.stdin.take() {
        let _ = i.write_all(text);
    }
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static PAGES: Mutex<Vec<Vec<Vec<u8>>>> = Mutex::new(Vec::new());

    fn collect(page: &[Vec<u8>], _width: usize) {
        PAGES.lock().unwrap().push(page.to_vec());
    }

    fn lines(p: &[Vec<u8>]) -> Vec<String> {
        p.iter().map(|l| String::from_utf8_lossy(l).into_owned()).collect()
    }

    #[test]
    fn pages_break_on_form_feed_on_a_full_page_and_at_the_end() {
        PAGES.lock().unwrap().clear();
        {
            let mut l = Lprint { page: Vec::new(), line: Vec::new(), width: 80, sink: collect };
            l.write(b"one");
            assert_eq!(l.pos(), 4);
            l.newline();
            l.write(b"two\x0cthree");
            for _ in 0..PAGE_LINES {
                l.newline();
            }
            l.write(b"last");
        }
        let p = PAGES.lock().unwrap();
        assert_eq!(p.len(), 3);
        assert_eq!(lines(&p[0]), ["one", "two"]);
        assert_eq!(p[1].len(), PAGE_LINES);
        assert_eq!(lines(&p[1][..1]), ["three"]);
        assert_eq!(lines(&p[2]), ["last"]);
    }

    #[test]
    fn long_lines_wrap_at_the_width_and_commas_go_to_zones() {
        let mut l = Lprint { page: Vec::new(), line: Vec::new(), width: 20, sink: |_, _| {} };
        l.write(b"a");
        l.zone();
        assert_eq!(l.pos(), 15);
        l.write(b"bcdefg");
        assert_eq!(l.pos(), 1 + 20);
        l.write(b"h");
        assert_eq!(l.pos(), 2);
        assert_eq!(lines(&l.page), [format!("a{}bcdefg", " ".repeat(13))]);
        l.zone();
        l.zone();
        assert_eq!(l.pos(), 1);
    }

    #[test]
    fn the_page_image_draws_the_characters_black_on_white() {
        let page = vec![b"A".to_vec()];
        let ink = (0..16).flat_map(|y| (0..8).map(move |x| (x, y))).filter(|&(x, y)| page_pixel(&page, x, y) == 0).count();
        assert!(ink > 10);
        assert_eq!(page_pixel(&page, 8, 0), 0x00FF_FFFF);
        assert_eq!(page_pixel(&page, 0, 16 * 5), 0x00FF_FFFF);
        assert_eq!(page_text(&page), b"A\r\n\x0c");
    }
}
