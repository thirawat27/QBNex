//! Headless framebuffer capture for the conformance runner.
//!
//! When `QBNEX_FB_CAPTURE` names a file, every `_DISPLAY` (kind 0) and the end
//! of the program (kind 1, graphics pages only) append one record describing
//! the display page. The format is shared with the legacy oracle hook
//! (`tests/conformance/oracle/fb_capture_legacy.inc`) and read by `conformance`:
//!
//! ```text
//! u32 0x42465142 ('QBFB'), u32 kind, u32 width, u32 height,
//! u32 bytes_per_pixel, u32 compatible_mode,
//! width*height*bytes_per_pixel pixel bytes (text pages: char/attribute pairs),
//! u32 palette entries (0 or 256), then that many u32 palette values
//! ```
//! All integers are little-endian.

use super::ImgStruct;
use std::fs::File;
use std::io::Write;
use std::sync::Mutex;

pub const MAGIC: u32 = 0x4246_5142;
pub const KIND_DISPLAY: i32 = 0;
pub const KIND_END: i32 = 1;

/// Serialises one record; `None` when the frame is not captured (text page at
/// program end).
pub fn record(im: &ImgStruct, kind: i32) -> Option<Vec<u8>> {
    if kind == KIND_END && im.text != 0 {
        return None;
    }
    let n = im.width as usize * im.height as usize * im.bytes_per_pixel as usize;
    let npal: u32 = if im.bytes_per_pixel == 4 || im.pal.is_null() { 0 } else { 256 };
    let mut out = Vec::with_capacity(28 + n + 4 * npal as usize);
    for v in [
        MAGIC,
        kind as u32,
        u32::from(im.width),
        u32::from(im.height),
        u32::from(im.bytes_per_pixel),
        u32::from(im.compatible_mode),
    ] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    if n > 0 && !im.offset.is_null() {
        // SAFETY: a valid surface owns width*height*bytes_per_pixel bytes.
        out.extend_from_slice(unsafe { core::slice::from_raw_parts(im.offset, n) });
    } else {
        out.resize(out.len() + n, 0);
    }
    out.extend_from_slice(&npal.to_le_bytes());
    for i in 0..npal as usize {
        // SAFETY: palettes always hold 256 entries.
        let v = unsafe { *im.pal.add(i) };
        out.extend_from_slice(&v.to_le_bytes());
    }
    Some(out)
}

enum Sink {
    Unopened,
    Disabled,
    Open(File),
}

static SINK: Mutex<Sink> = Mutex::new(Sink::Unopened);

/// Appends a record for `display_page` when capturing is enabled.
///
/// # Safety
/// `im` must be null or point to a valid surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_fb_capture(im: *const ImgStruct, kind: i32) {
    let mut sink = match SINK.lock() {
        Ok(s) => s,
        Err(p) => p.into_inner(),
    };
    if let Sink::Unopened = *sink {
        *sink = match std::env::var_os("QBNEX_FB_CAPTURE") {
            Some(p) if !p.is_empty() => File::create(p).map(Sink::Open).unwrap_or(Sink::Disabled),
            _ => Sink::Disabled,
        };
    }
    let Sink::Open(file) = &mut *sink else { return };
    // SAFETY: guaranteed by the caller.
    let Some(im) = (unsafe { im.as_ref() }) else { return };
    if let Some(rec) = record(im, kind) {
        let _ = file.write_all(&rec);
        let _ = file.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::testutil::TestImage;

    #[test]
    fn record_layout_for_palette_image() {
        let mut t = TestImage::new(2, 1, 13);
        t.pixels[0] = 0x0000_0201;
        let r = record(&t.img, KIND_DISPLAY).unwrap();
        assert_eq!(r.len(), 24 + 2 + 4 + 1024);
        assert_eq!(&r[0..4], &MAGIC.to_le_bytes());
        assert_eq!(u32::from_le_bytes(r[8..12].try_into().unwrap()), 2);
        assert_eq!(u32::from_le_bytes(r[20..24].try_into().unwrap()), 13);
        assert_eq!(&r[24..26], &[1, 2]);
        assert_eq!(u32::from_le_bytes(r[26..30].try_into().unwrap()), 256);
    }

    #[test]
    fn end_capture_skips_text_pages_and_32bit_has_no_palette() {
        let t = TestImage::new(4, 2, 0);
        assert!(record(&t.img, KIND_END).is_none());
        assert!(record(&t.img, KIND_DISPLAY).is_some());
        let t = TestImage::new(1, 1, 32);
        let r = record(&t.img, KIND_END).unwrap();
        assert_eq!(r.len(), 24 + 4 + 4);
        assert_eq!(&r[28..32], &0u32.to_le_bytes());
    }
}
