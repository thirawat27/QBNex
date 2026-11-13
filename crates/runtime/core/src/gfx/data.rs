#![allow(non_upper_case_globals)]

//! Built-in graphics data: the default 256-color and EGA palettes and the
//! 8x8 / 8x16 bitmap fonts (QB64's tables). Font bitmaps hold one byte per
//! pixel (0 or 255), `[char][row][col]`.

const fn argb_palette<const N: usize>(raw: &[u8]) -> [u32; N] {
    let mut out = [0u32; N];
    let mut i = 0;
    while i < N {
        out[i] = (raw[i * 4] as u32
            | (raw[i * 4 + 1] as u32) << 8
            | (raw[i * 4 + 2] as u32) << 16
            | (raw[i * 4 + 3] as u32) << 24)
            | 0xFF00_0000;
        i += 1;
    }
    out
}

/// Default VGA 256-color palette (ARGB, alpha forced to 255 as QB64 does).
pub static palette_256: [u32; 256] = argb_palette::<256>(include_bytes!("data/pal256.bin"));

/// Default EGA 64-color palette.
pub static palette_64: [u32; 64] = argb_palette::<64>(include_bytes!("data/pal64.bin"));


/// 8x8 font, `[char][row][col]`.
pub static charset8x8: [u8; 256 * 8 * 8] = *include_bytes!("data/charset8.bin");

/// 8x16 font, `[char][row][col]` (the 8x14 font is rows 1..=14 of it).
pub static charset8x16: [u8; 256 * 16 * 8] = *include_bytes!("data/charset16.bin");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_palettes() {
        // VGA: 0 black, 1 blue, 4 red, 14 yellow, 15 white
        assert_eq!(palette_256[0], 0xFF00_0000);
        assert_eq!(palette_256[1], 0xFF00_00AA);
        assert_eq!(palette_256[4], 0xFFAA_0000);
        assert_eq!(palette_256[14], 0xFFFF_FF55);
        assert_eq!(palette_256[15], 0xFFFF_FFFF);
        // EGA: 63 white, 20 brown (default color 6 in SCREEN 9)
        assert_eq!(palette_64[63], 0xFFFF_FFFF);
        assert_eq!(palette_64[20], 0xFFAA_5500);
    }

    #[test]
    fn fonts_are_byte_per_pixel() {
        assert!(charset8x8.iter().all(|b| *b == 0 || *b == 255));
        // 'A' has ink, space has none
        let a = &charset8x16[65 * 128..66 * 128];
        assert!(a.iter().any(|b| *b != 0));
        assert!(charset8x16[32 * 128..33 * 128].iter().all(|b| *b == 0));
        assert!(charset8x8[32 * 64..33 * 64].iter().all(|b| *b == 0));
    }
}
