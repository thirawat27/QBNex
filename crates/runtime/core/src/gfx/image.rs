//! Image registry and surface setup (QB64's `imgframe`, `imgnew`,
//! `imgrevert`, `newimg`, `freeimg`, `grow_img_registry`).

use super::palette::restorepalette;
use super::ImgStruct;

/// Per-mode values of `imgframe`/`imgrevert`; `false` for an unknown mode.
fn apply_mode_defaults(im: &mut ImgStruct, mode: u16) -> bool {
    im.bytes_per_pixel = 1;
    im.font = 16;
    im.color = 15;
    im.print_mode = 3;
    im.draw_ta = 0.0;
    im.draw_scale = 1.0;
    match mode {
        0 => {
            im.bits_per_pixel = 16;
            im.bytes_per_pixel = 2;
            im.color = 7;
            im.text = 1;
            im.cursor_show = 0;
            im.cursor_firstvalue = 4;
            im.cursor_lastvalue = 4;
        }
        1 => {
            im.bits_per_pixel = 2;
            im.font = 8;
            im.color = 3;
        }
        2 => {
            im.bits_per_pixel = 1;
            im.font = 8; // stretched from 8 to 16 by the display code
            im.color = 1;
        }
        7 | 8 => {
            im.bits_per_pixel = 4;
            im.font = 8;
        }
        9 => {
            im.bits_per_pixel = 4;
            im.font = 14;
        }
        10 => {
            im.bits_per_pixel = 2;
            im.font = 14;
            im.color = 3;
        }
        11 => {
            im.bits_per_pixel = 1;
            im.color = 1;
        }
        12 => im.bits_per_pixel = 4,
        13 => {
            im.bits_per_pixel = 8;
            im.font = 8;
        }
        256 => im.bits_per_pixel = 8,
        32 => {
            im.bits_per_pixel = 32;
            im.bytes_per_pixel = 4;
            im.color = 0xFFFF_FFFF;
            im.background_color = 0xFF00_0000;
        }
        _ => return false,
    }
    im.draw_color = im.color;
    true
}

/// The "calculatable values" block shared by `imgframe` and `imgrevert`.
fn apply_derived(im: &mut ImgStruct, mode: u16) {
    im.mask = if im.bits_per_pixel < 32 { (1u32 << im.bits_per_pixel) - 1 } else { 0xFFFF_FFFF };
    im.cursor_x = 1;
    im.cursor_y = 1;
    im.top_row = 1;
    let mut bottom = if mode != 0 {
        (i32::from(im.height) / im.font as i32) as i16
    } else {
        im.height as i16
    };
    bottom = bottom.wrapping_sub(1);
    if bottom <= 0 {
        bottom = 1;
    }
    im.bottom_row = bottom;
    if mode == 0 {
        return;
    }
    im.x = (f64::from(im.width) / 2.0) as f32;
    im.y = (f64::from(im.height) / 2.0) as f32;
    im.view_x2 = i32::from(im.width) - 1;
    im.view_y2 = i32::from(im.height) - 1;
    im.scaling_x = 1.0;
    im.scaling_y = 1.0;
    im.window_x2 = im.view_x2 as f32;
    im.window_y2 = im.view_y2 as f32;
}

/// Property setup of `imgframe(o, w, h, mode)` once the palette pointer (non
/// 32-bit modes) is attached; returns false for an unknown mode. Does not
/// touch pixels, flags or the text-cell side tables.
pub fn setup_new_image(im: &mut ImgStruct, w: u16, h: u16, mode: u16) -> bool {
    im.width = w;
    im.height = h;
    im.compatible_mode = mode;
    if !apply_mode_defaults(im, mode) {
        return false;
    }
    if mode != 32 {
        if !im.pal.is_null() {
            // SAFETY: palettes hold 256 entries.
            unsafe { restorepalette(im) };
        }
        im.transparent_color = -1;
    }
    apply_derived(im, mode);
    true
}

/// `imgrevert`: back to the defaults of the surface's mode, pixels cleared.
/// The caller re-initialises text cells for text surfaces.
///
/// # Safety
/// `im` must be a valid surface.
pub unsafe fn imgrevert(im: &mut ImgStruct) {
    let mode = im.compatible_mode;
    im.background_color = 0;
    apply_mode_defaults(im, mode);
    if mode != 32 {
        // SAFETY: non-32-bit surfaces own a 256-entry palette.
        unsafe { restorepalette(im) };
        im.transparent_color = -1;
    }
    apply_derived(im, mode);
    if mode == 0 {
        return;
    }
    let n = usize::from(im.width) * usize::from(im.height) * usize::from(im.bytes_per_pixel);
    // SAFETY: the surface owns n bytes.
    unsafe { core::ptr::write_bytes(im.offset, 0, n) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::testutil::TestImage;

    #[test]
    fn screen13_defaults() {
        let t = TestImage::new(320, 200, 13);
        let im = &t.img;
        assert_eq!((im.width, im.height, im.compatible_mode), (320, 200, 13));
        assert_eq!((im.bytes_per_pixel, im.bits_per_pixel, im.mask), (1, 8, 255));
        assert_eq!((im.font, im.color, im.background_color, im.draw_color), (8, 15, 0, 15));
        assert_eq!((im.x, im.y), (160.0, 100.0));
        assert_eq!((im.view_x2, im.view_y2, im.window_x2, im.window_y2), (319, 199, 319.0, 199.0));
        assert_eq!((im.scaling_x, im.scaling_y), (1.0, 1.0));
        assert_eq!((im.top_row, im.bottom_row, im.cursor_x, im.cursor_y), (1, 24, 1, 1));
        assert_eq!((im.print_mode, im.transparent_color, im.draw_scale), (3, -1, 1.0));
        assert_eq!(t.palette[1], 0xFF00_00AA);
        assert_eq!(t.palette[255], crate::gfx::data::palette_256[255]);
    }

    #[test]
    fn per_mode_settings() {
        let cases: [(u16, u8, u32, u32); 9] = [
            (1, 2, 8, 3),
            (2, 1, 8, 1),
            (7, 4, 8, 15),
            (8, 4, 8, 15),
            (9, 4, 14, 15),
            (10, 2, 14, 3),
            (11, 1, 16, 1),
            (12, 4, 16, 15),
            (256, 8, 16, 15),
        ];
        for (mode, bits, font, color) in cases {
            let t = TestImage::new(64, 48, mode);
            assert_eq!((t.img.bits_per_pixel, t.img.font, t.img.color), (bits, font, color), "mode {mode}");
            assert_eq!(t.img.mask, (1 << bits) - 1);
        }
        let t = TestImage::new(64, 48, 9);
        assert_eq!(t.palette[6], crate::gfx::data::palette_64[20]);
        assert_eq!(t.palette[8], crate::gfx::data::palette_64[56]);
        let t = TestImage::new(64, 48, 10);
        assert_eq!(&t.palette[4..8], &[0, 4, 6, 8]);
    }

    #[test]
    fn text_and_32bit() {
        let t = TestImage::new(80, 25, 0);
        assert_eq!((t.img.text, t.img.bytes_per_pixel, t.img.bits_per_pixel, t.img.color), (1, 2, 16, 7));
        assert_eq!((t.img.cursor_firstvalue, t.img.cursor_lastvalue, t.img.bottom_row), (4, 4, 24));
        assert_eq!(t.img.view_x2, 0); // text surfaces skip the graphics setup
        let t = TestImage::new(10, 10, 32);
        assert_eq!((t.img.bytes_per_pixel, t.img.mask, t.img.color, t.img.background_color), (4, 0xFFFF_FFFF, 0xFFFF_FFFF, 0xFF00_0000));
        assert_eq!(t.img.transparent_color, 0); // not set for 32-bit
    }

    #[test]
    fn unknown_mode_is_rejected() {
        let mut im = ImgStruct::default();
        assert!(!setup_new_image(&mut im, 10, 10, 5));
    }
}
