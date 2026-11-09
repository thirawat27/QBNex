#![allow(non_upper_case_globals)]

//! Built-in graphics data that libqb.cpp embedded as byte arrays and copied
//! into globals at start-up: the default 256-color and EGA palettes, the
//! SCREEN 10 pseudo-color table and the 8x8 / 8x16 bitmap fonts.
//!
//! The globals keep their C names and types so the remaining C++ (window
//! presentation, text rendering) links against them unchanged:
//! `uint32 palette_256[256]`, `uint32 palette_64[64]`, `uint32 pal_mode10[2][9]`,
//! `uint8 charset8x8[256][8][8]`, `uint8 charset8x16[256][16][8]`.
//! Font bitmaps hold one byte per pixel (0 or 255).

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

/// Default VGA 256-color palette (ARGB, alpha forced to 255 as in legacy init).
#[no_mangle]
pub static palette_256: [u32; 256] = argb_palette::<256>(include_bytes!("data/pal256.bin"));

/// Default EGA 64-color palette.
#[no_mangle]
pub static palette_64: [u32; 64] = argb_palette::<64>(include_bytes!("data/pal64.bin"));

/// SCREEN 10 pseudo colors: `[blink phase][0..8]` (no alpha, as in legacy).
#[no_mangle]
pub static pal_mode10: [[u32; 9]; 2] = [
    [0, 0, 0, 0x808080, 0x808080, 0x808080, 0xFFFFFF, 0xFFFFFF, 0xFFFFFF],
    [0, 0x808080, 0xFFFFFF, 0, 0x808080, 0xFFFFFF, 0, 0x808080, 0xFFFFFF],
];

/// 8x8 font, `[char][row][col]`.
#[no_mangle]
pub static charset8x8: [u8; 256 * 8 * 8] = *include_bytes!("data/charset8.bin");

/// 8x16 font, `[char][row][col]` (the 8x14 font is rows 1..=14 of it).
#[no_mangle]
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
