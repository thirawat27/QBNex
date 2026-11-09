//! PNG and JPEG through the `png` and `zune-jpeg` crates (the codecs behind
//! the `image` crate), adapted to what stb_image v2.27 returned:
//! - PNG: CRCs and Adler-32 are ignored (stb never checked them); palettes,
//!   tRNS and low bit depths are expanded; 16-bit samples keep their high byte
//!   (`stbi__convert_16_to_8`), after tRNS was matched on the full 16-bit value.
//! - JPEG: lossy; the IDCT and chroma upsampling differ from stb_image, so
//!   pixels may differ slightly (deviation D-R7-2, tolerance in diff_image.cpp).

use super::stb::{mad3_valid, Ctx, Rgba, MAX_DIMENSIONS};

pub fn png_test(d: &[u8]) -> bool {
    d.starts_with(b"\x89PNG\r\n\x1a\n")
}

pub fn png_load(d: &[u8]) -> Option<Rgba> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(d));
    dec.set_limits(png::Limits { bytes: usize::MAX });
    dec.ignore_checksums(true);
    dec.set_transformations(png::Transformations::EXPAND);
    let mut reader = dec.read_info().ok()?;
    let (w, h) = {
        let info = reader.info();
        (info.width, info.height)
    };
    if w > MAX_DIMENSIONS || h > MAX_DIMENSIONS || !mad3_valid(4, w as i64, h as i64) {
        return None;
    }
    let mut buf = Vec::new();
    buf.try_reserve_exact(reader.output_buffer_size()?).ok()?;
    buf.resize(reader.output_buffer_size()?, 0);
    let out = reader.next_frame(&mut buf).ok()?;
    let step = if out.bit_depth == png::BitDepth::Sixteen {
        2
    } else {
        1
    };
    let comp = match out.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Indexed => return None, // EXPAND never leaves indices
    };
    let n = w as usize * h as usize;
    buf.truncate(n * comp * step);
    let samples = if step == 2 { super::stb::high_bytes(&buf)? } else { buf };
    Some(Rgba {
        w,
        h,
        px: super::stb::to_rgba(samples, comp, n)?,
    })
}

/// `stbi__jpeg_test`: the first marker (after 0xFF fill bytes) must be SOI.
pub fn jpeg_test(d: &[u8]) -> bool {
    let mut s = Ctx::new(d);
    if s.get8() != 0xFF {
        return false;
    }
    let mut x = 0xFF;
    while x == 0xFF {
        x = s.get8();
    }
    x == 0xD8
}

/// Width and height from the first SOFn segment, if any.
fn jpeg_frame_size(d: &[u8]) -> Option<(u32, u32)> {
    let mut i = 2;
    while i + 4 <= d.len() {
        if d[i] != 0xFF {
            i += 1;
            continue;
        }
        let m = d[i + 1];
        if m == 0xFF || m == 0x01 || (0xD0..=0xD8).contains(&m) {
            i += if m == 0xFF { 1 } else { 2 };
            continue;
        }
        let len = u16::from_be_bytes([d[i + 2], d[i + 3]]) as usize;
        if matches!(m, 0xC0..=0xCF) && !matches!(m, 0xC4 | 0xC8 | 0xCC) {
            let s = d.get(i + 5..i + 9)?;
            return Some((u16::from_be_bytes([s[2], s[3]]) as u32, u16::from_be_bytes([s[0], s[1]]) as u32));
        }
        i += 2 + len;
    }
    None
}

pub fn jpeg_load(d: &[u8]) -> Option<Rgba> {
    // zune-jpeg allocates with the infallible allocator: check the frame size
    // first, so an image too large for memory fails (like stb's malloc) instead
    // of aborting the program.
    let (fw, fh) = jpeg_frame_size(d)?;
    if !mad3_valid(4, fw as i64, fh as i64) {
        return None;
    }
    let probe = (fw as usize * fh as usize).checked_mul(12)?;
    let mut v: Vec<u8> = Vec::new();
    v.try_reserve_exact(probe).ok()?;
    drop(v);
    use zune_jpeg::zune_core::bytestream::ZCursor;
    use zune_jpeg::zune_core::colorspace::ColorSpace;
    use zune_jpeg::zune_core::options::DecoderOptions;
    let opts = DecoderOptions::default()
        .jpeg_set_out_colorspace(ColorSpace::RGBA)
        .set_max_width(MAX_DIMENSIONS as usize)
        .set_max_height(MAX_DIMENSIONS as usize);
    let mut dec = zune_jpeg::JpegDecoder::new_with_options(ZCursor::new(d), opts);
    let px = dec.decode().ok()?;
    let info = dec.info()?;
    let (w, h) = (info.width as u32, info.height as u32);
    if px.len() != w as usize * h as usize * 4 {
        return None;
    }
    Some(Rgba { w, h, px })
}
