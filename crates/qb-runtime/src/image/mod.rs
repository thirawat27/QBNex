//! `_LOADIMAGE` codecs (was `internal/c/parts/video/image/src.c`: dr_pcx +
//! stb_image v2.27, plus the 8bpp quantizer and palette remapper).
//!
//! - PCX: port of dr_pcx (`pcx.rs`).
//! - BMP, TGA, GIF, PNM: ports of the stb_image loaders (`stb.rs`); their
//!   quirks (palette size taken from the data offset, all-zero alpha made
//!   opaque, GIF background stored as B,G,R...) are kept bit for bit.
//! - PNG, JPEG: the `png` and `zune-jpeg` crates (`codecs.rs`).
//! - PSD, PIC, HDR: no longer decoded (deviation D-R7-3).

mod codecs;
pub mod pcx;
pub mod quant;
mod stb;

use stb::Ctx;

/// A decoded image: `w * h` pixels, 4 bytes each, in libqb's BGRA order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bgra {
    pub w: u32,
    pub h: u32,
    pub px: Vec<u8>,
}

/// Decodes like the legacy `func__loadimage`: dr_pcx first, then stb_image.
pub fn decode(data: &[u8]) -> Option<Bgra> {
    let (w, h, mut px) = match pcx::decode(data) {
        Some(img) => img,
        None => {
            let img = decode_stb(data)?;
            (img.w, img.h, img.px)
        }
    };
    for p in px.as_chunks_mut::<4>().0 {
        p.swap(0, 2);
    }
    Some(Bgra { w, h, px })
}

/// `stbi__load_main`: the first format whose test matches decides; a failed
/// decode does not fall through to the next format.
fn decode_stb(data: &[u8]) -> Option<stb::Rgba> {
    let mut s = Ctx::new(data);
    if codecs::png_test(data) {
        codecs::png_load(data)
    } else if stb::bmp_test(&mut s) {
        stb::bmp_load(&mut s)
    } else if stb::gif_test(&mut s) {
        stb::gif_load(&mut s)
    } else if stb::unsupported_test(&mut s) {
        None
    } else if codecs::jpeg_test(data) {
        codecs::jpeg_load(data)
    } else if stb::pnm_test(&mut s) {
        stb::pnm_load(&mut s)
    } else if stb::tga_test(&mut s) {
        stb::tga_load(&mut s)
    } else {
        None
    }
}

/// C ABI, see `qb_runtime.h`.
///
/// # Safety
/// `data` must point to `len` readable bytes (or be null with `len <= 0`);
/// `w` and `h` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_image_decode(
    data: *const u8,
    len: i32,
    w: *mut i32,
    h: *mut i32,
) -> *mut u8 {
    let bytes: &[u8] = if data.is_null() || len <= 0 {
        &[]
    } else {
        // SAFETY: caller guarantees `len` readable bytes.
        unsafe { std::slice::from_raw_parts(data, len as usize) }
    };
    match decode(bytes) {
        Some(img) => {
            let p = crate::cmem::to_c_heap(&img.px);
            if !p.is_null() {
                // SAFETY: caller guarantees writable out-pointers.
                unsafe {
                    *w = img.w as i32;
                    *h = img.h as i32;
                }
            }
            p
        }
        None => std::ptr::null_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus(name: &str) -> Vec<u8> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/diff/data/images")
            .join(name);
        std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
    }

    fn fnv1a(b: &[u8]) -> u64 {
        b.iter().fold(0xcbf29ce484222325u64, |h, &x| {
            (h ^ x as u64).wrapping_mul(0x100000001b3)
        })
    }

    type Expected = Option<(u32, u32, u64)>;

    /// (file, Some((w, h, FNV-1a of the BGRA pixels))) captured from the legacy
    /// dr_pcx + stb_image decoders (`diff_image --dump`); None = legacy failed.
    #[rustfmt::skip]
    const LEGACY: &[(&str, Expected)] = &[
        ("bad_empty.bin", None),
        ("bad_png_truncated.png", None),
        ("bad_random.bin", None),
        ("bmp_bitfields32_v4.bmp", Some((21, 11, 0xd16208bc34bcb65e))),
        ("bmp_bitfields32_v5.bmp", Some((21, 11, 0xd16208bc34bcb65e))),
        ("bmp_pal1.bmp", Some((21, 11, 0xa96ca199b0f8c12c))),
        ("bmp_pal4.bmp", Some((21, 11, 0xbaa5befc93eb6d70))),
        ("bmp_pal4_os2.bmp", Some((21, 11, 0x20a3122ae6809ddb))),
        ("bmp_pal8.bmp", Some((21, 11, 0x3d17aef202233193))),
        ("bmp_pal8_os2.bmp", Some((21, 11, 0xb08d1160d98e2d24))),
        ("bmp_pal8_short_palette.bmp", Some((21, 11, 0x333ac100762ea082))),
        ("bmp_rgb16_555.bmp", Some((21, 11, 0xef386941d041a0ae))),
        ("bmp_rgb16_565.bmp", Some((21, 11, 0x19e7891b4edc5d84))),
        ("bmp_rgb24.bmp", Some((21, 11, 0xe39eb4acf80eba58))),
        ("bmp_rgb24_os2.bmp", Some((21, 11, 0xe39eb4acf80eba58))),
        ("bmp_rgb24_topdown.bmp", Some((21, 11, 0xe39eb4acf80eba58))),
        ("bmp_rgb24_v5.bmp", Some((21, 11, 0xe39eb4acf80eba58))),
        ("bmp_rgb32_alpha.bmp", Some((21, 11, 0xd16208bc34bcb65e))),
        ("bmp_rgb32_zero_alpha.bmp", Some((21, 11, 0xe39eb4acf80eba58))),
        ("bmp_rle8.bmp", None),
        ("gif_animated.gif", Some((29, 21, 0xcf1c8465fdbd06c4))),
        ("gif_still.gif", Some((29, 21, 0xcf1c8465fdbd06c4))),
        ("gif_transparent_interlaced.gif", Some((29, 21, 0x08f92294b23236bc))),
        ("pcx_1bit_2planes.pcx", Some((23, 7, 0xa09e704683f35f0f))),
        ("pcx_1bit_3planes.pcx", Some((23, 7, 0x64cc924e6516643b))),
        ("pcx_1bit_4planes.pcx", Some((23, 7, 0xca76210b339b5fb0))),
        ("pcx_1bit_mono.pcx", Some((23, 7, 0x2d994cd9967b5d31))),
        ("pcx_2bit_cga.pcx", Some((23, 7, 0xe68ff00674d97980))),
        ("pcx_2bit_cga_v3.pcx", Some((23, 7, 0x94ca2832b0a9e2ca))),
        ("pcx_4bit_1plane.pcx", Some((23, 7, 0x025b493c4c0da111))),
        ("pcx_8bit_long_runs.pcx", Some((130, 4, 0xf101526095266e48))),
        ("pcx_8bit_gray.pcx", Some((23, 7, 0x1e1d8dca7dcc1a81))),
        ("pcx_8bit_pal.pcx", Some((23, 7, 0x24f9ef86764b8785))),
        ("pcx_8bit_rgb.pcx", Some((23, 7, 0x417e2af6dac71c4d))),
        ("pcx_8bit_rgb_offset_swapped.pcx", Some((23, 7, 0x80e143133850ceb9))),
        ("pcx_8bit_rgba.pcx", Some((23, 7, 0x7bb361f35a57d30e))),
        ("pcx_8bit_truncated.pcx", Some((40, 30, 0x102df0018b9e1246))),
        ("png_1x1.png", Some((1, 1, 0xdbd09687ea36bd25))),
        ("png_gray1.png", Some((19, 13, 0xaeaac1e856d7e3f0))),
        ("png_gray16.png", Some((19, 13, 0x73d70ab10ff675a3))),
        ("png_gray16_trns.png", Some((19, 13, 0x9caac30cdaffde2a))),
        ("png_gray1_interlaced.png", Some((19, 13, 0xaeaac1e856d7e3f0))),
        ("png_gray2.png", Some((19, 13, 0x338d9b5fc4362460))),
        ("png_gray2_interlaced.png", Some((19, 13, 0x338d9b5fc4362460))),
        ("png_gray4.png", Some((19, 13, 0xaf3baaacea161c00))),
        ("png_gray4_interlaced.png", Some((19, 13, 0xaf3baaacea161c00))),
        ("png_gray8.png", Some((19, 13, 0x1914221437e2a46d))),
        ("png_gray8_trns.png", Some((19, 13, 0x9de4aa6655ca0a08))),
        ("png_graya16_interlaced.png", Some((19, 13, 0x00c70fe5b077c5b2))),
        ("png_graya8.png", Some((19, 13, 0x00c70fe5b077c5b2))),
        ("png_pal1.png", Some((19, 13, 0x752c377f90450fc3))),
        ("png_pal2.png", Some((19, 13, 0x18d8d2aa8d4ba559))),
        ("png_pal4.png", Some((19, 13, 0x3b5e543298c88202))),
        ("png_pal8.png", Some((19, 13, 0x752038ddcbf457ec))),
        ("png_pal8_interlaced.png", Some((19, 13, 0x752038ddcbf457ec))),
        ("png_pal8_trns.png", Some((19, 13, 0x1b094307d9578288))),
        ("png_rgb16.png", Some((19, 13, 0xe14adfd3e46cecbf))),
        ("png_rgb16_trns.png", Some((19, 13, 0x1f81651062e29822))),
        ("png_rgb8.png", Some((19, 13, 0xe14adfd3e46cecbf))),
        ("png_rgb8_trns.png", Some((19, 13, 0xa301cb9e58c4dde2))),
        ("png_rgba16.png", Some((19, 13, 0x1e8bc2d37bbb02f2))),
        ("png_rgba8.png", Some((19, 13, 0x1e8bc2d37bbb02f2))),
        ("png_rgba8_interlaced.png", Some((19, 13, 0x1e8bc2d37bbb02f2))),
        ("png_wide.png", Some((300, 2, 0xa6670777d5693658))),
        ("pnm_p5.pgm", Some((13, 5, 0xd1216bfc1c61a128))),
        ("pnm_p6.ppm", Some((13, 5, 0x400f963b5b990f3c))),
        ("tga_cmap16.tga", Some((17, 9, 0xf2bd6d41ca196535))),
        ("tga_cmap16_rle.tga", Some((17, 9, 0x97eea2105b9d1101))),
        ("tga_cmap24.tga", Some((17, 9, 0xa0a07a04d9d249c1))),
        ("tga_cmap24_rle.tga", Some((17, 9, 0x96164f74bcf581ce))),
        ("tga_cmap32.tga", Some((17, 9, 0x31c00923d1247d27))),
        ("tga_cmap32_rle.tga", Some((17, 9, 0x28bd00bad8f131d7))),
        ("tga_gray8.tga", Some((17, 9, 0xcd9de1e6d6cc8229))),
        ("tga_gray8_rle.tga", Some((17, 9, 0xcd9de1e6d6cc8229))),
        ("tga_graya16.tga", Some((17, 9, 0x4d4727dd2baa90c5))),
        ("tga_graya16_rle.tga", Some((17, 9, 0x4d4727dd2baa90c5))),
        ("tga_rgb15.tga", Some((17, 9, 0xb21c46995a666ff1))),
        ("tga_rgb15_rle.tga", Some((17, 9, 0xb21c46995a666ff1))),
        ("tga_rgb16.tga", Some((17, 9, 0xb21c46995a666ff1))),
        ("tga_rgb16_rle.tga", Some((17, 9, 0xb21c46995a666ff1))),
        ("tga_rgb24.tga", Some((17, 9, 0xdb472f1db2580949))),
        ("tga_rgb24_rle.tga", Some((17, 9, 0xdb472f1db2580949))),
        ("tga_rgba32.tga", Some((17, 9, 0xb9840f369dba4465))),
        ("tga_rgba32_noalphabits.tga", Some((17, 9, 0xb9840f369dba4465))),
        ("tga_rgba32_rle.tga", Some((17, 9, 0xb9840f369dba4465))),
        ("tga_rgba32_top.tga", Some((17, 9, 0xb9840f369dba4465))),
        ("tga_rgba32_top_rle.tga", Some((17, 9, 0xb9840f369dba4465))),
    ];

    #[test]
    fn lossless_corpus_matches_legacy_decoders_exactly() {
        let mut bad = Vec::new();
        for &(name, want) in LEGACY {
            let got = decode(&corpus(name)).map(|i| (i.w, i.h, fnv1a(&i.px)));
            if got != want {
                bad.push(format!("{name}: got {got:x?}, legacy {want:x?}"));
            }
        }
        assert!(
            bad.is_empty(),
            "{} mismatches:\n{}",
            bad.len(),
            bad.join("\n")
        );
    }

    #[test]
    fn jpeg_sizes_match_legacy() {
        for name in [
            "jpg_420.jpg",
            "jpg_422.jpg",
            "jpg_444.jpg",
            "jpg_gray.jpg",
            "jpg_optimized.jpg",
            "jpg_progressive.jpg",
        ] {
            let img = decode(&corpus(name)).unwrap_or_else(|| panic!("{name} failed"));
            assert_eq!(
                (img.w, img.h, img.px.len()),
                (29, 21, 29 * 21 * 4),
                "{name}"
            );
            assert!(
                img.px.chunks(4).all(|p| p[3] == 255),
                "{name}: JPEG is opaque"
            );
        }
    }

    #[test]
    fn bmp_palette_entries_stb_never_read_are_black() {
        // OS/2 BMP, 1 bpp, 2 palette entries: stb_image computes a palette size
        // of -2, reads no entry and indexes uninitialised memory; D-R7-5 makes
        // those entries black.
        let mut f = b"BM".to_vec();
        f.extend_from_slice(&[0; 8]);
        f.extend_from_slice(&(14u32 + 12 + 6).to_le_bytes());
        f.extend_from_slice(&[12, 0, 0, 0, 2, 0, 1, 0, 1, 0, 1, 0]);
        f.extend_from_slice(&[255, 255, 255, 9, 9, 9]);
        f.extend_from_slice(&[0b1000_0000, 0, 0, 0]);
        assert_eq!(decode(&f).unwrap().px, [0, 0, 0, 255].repeat(2));
    }

    #[test]
    fn huge_images_fail_instead_of_aborting() {
        // PNG 65535 x 65535, 16-bit grey: about 34 GB of output.
        let mut png = vec![0x89, b'P', b'N', b'G', 13, 10, 26, 10];
        let ihdr: Vec<u8> = [&[0u8, 0, 0, 13][..], b"IHDR", &[0, 0, 255, 255, 0, 0, 255, 255, 16, 0, 0, 0, 0], &[0, 0, 0, 0]].concat();
        png.extend_from_slice(&ihdr);
        assert!(decode(&png).is_none());
        // JPEG SOF0 65535 x 65535
        let jpg = [0xFF, 0xD8, 0xFF, 0xC0, 0, 11, 8, 255, 255, 255, 255, 1, 1, 0x11, 0, 0xFF, 0xD9];
        assert!(decode(&jpg).is_none());
    }

    #[test]
    fn png_pixel_values_are_bgra() {
        // png_1x1.png holds RGBA (1, 2, 3, 4).
        assert_eq!(decode(&corpus("png_1x1.png")).unwrap().px, vec![3, 2, 1, 4]);
    }

    #[test]
    fn ffi_returns_malloc_buffer_and_size() {
        let data = corpus("png_1x1.png");
        let (mut w, mut h) = (0, 0);
        // SAFETY: valid buffer and out-pointers; result is freed with free().
        unsafe {
            let p = qb_image_decode(data.as_ptr(), data.len() as i32, &mut w, &mut h);
            assert!(!p.is_null());
            assert_eq!((w, h), (1, 1));
            assert_eq!(std::slice::from_raw_parts(p, 4), &[3, 2, 1, 4]);
            unsafe extern "C" {
                fn free(p: *mut std::os::raw::c_void);
            }
            free(p as *mut _);
            assert!(qb_image_decode(std::ptr::null(), 0, &mut w, &mut h).is_null());
        }
    }
}
