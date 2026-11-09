//! Software graphics (phase R5): the image registry, palettes, and every
//! drawing primitive that used to live in `libqb.cpp`.
//!
//! The data model is unchanged: every surface is an [`ImgStruct`] with the
//! exact `#[repr(C)]` layout of `img_struct` in `internal/c/common.h`, so the
//! C++ code that is not ported yet (text output, window presentation in
//! `gui.cpp`/freeglut, `_MEM`) keeps reading the same fields and pixel
//! buffers. Drawing entry points take the page they act on as a pointer
//! (`write_page`/`read_page` stay C++ globals) and return a QBasic error code
//! (0 = no error); the one-line shims in libqb.cpp raise it with `error()`.
//!
//! Everything is ported bug-for-bug, including float precision: legacy code
//! mixes `float` and `double` arithmetic and rounds through x87 `fistp`
//! (`qbr`, `qbr_float_to_long`), and the Rust code repeats each conversion.

#![allow(clippy::too_many_arguments)]

pub mod blend;
pub mod capture;
pub mod circle;
pub mod data;
pub mod draw;
pub mod image;
pub mod imgops;
pub mod line;
pub mod maptri;
pub mod paint;
pub mod palette;
pub mod point;
pub mod putimage;
pub mod registry;
pub mod text;
pub mod view;

use core::ffi::c_void;

/// Layout-identical mirror of `img_struct` (`internal/c/common.h`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ImgStruct {
    pub lock_offset: *mut c_void,
    pub lock_id: i64,
    pub valid: u8,
    pub text: u8,
    pub console: u8,
    pub width: u16,
    pub height: u16,
    pub bytes_per_pixel: u8,
    pub bits_per_pixel: u8,
    pub mask: u32,
    pub compatible_mode: u16,
    pub color: u32,
    pub background_color: u32,
    pub draw_color: u32,
    pub font: u32,
    pub top_row: i16,
    pub bottom_row: i16,
    pub cursor_x: i16,
    pub cursor_y: i16,
    pub cursor_show: u8,
    pub cursor_firstvalue: u8,
    pub cursor_lastvalue: u8,
    /// `union { uint8 *offset; uint32 *offset32; }`
    pub offset: *mut u8,
    pub flags: u32,
    pub pal: *mut u32,
    pub transparent_color: i32,
    pub alpha_disabled: u8,
    pub holding_cursor: u8,
    pub print_mode: u8,
    // BEGIN apm ("active page migration"): copied as one block on SCREEN page changes.
    pub apm_p1: u8,
    pub view_x1: i32,
    pub view_y1: i32,
    pub view_x2: i32,
    pub view_y2: i32,
    pub view_offset_x: i32,
    pub view_offset_y: i32,
    pub x: f32,
    pub y: f32,
    pub clipping_or_scaling: u8,
    pub scaling_x: f32,
    pub scaling_y: f32,
    pub scaling_offset_x: f32,
    pub scaling_offset_y: f32,
    pub window_x1: f32,
    pub window_y1: f32,
    pub window_x2: f32,
    pub window_y2: f32,
    pub draw_ta: f64,
    pub draw_scale: f64,
    pub apm_p2: u8,
    // END apm
}

impl Default for ImgStruct {
    fn default() -> Self {
        // SAFETY: every field is an integer, float or raw pointer; all-zero is
        // valid for each (this is what legacy `memset(&img[i],0,...)` produced).
        unsafe { core::mem::zeroed() }
    }
}

impl ImgStruct {
    #[inline]
    pub fn offset32(&self) -> *mut u32 {
        self.offset as *mut u32
    }
}

/// `img_struct.flags`
pub const IMG_FREEPAL: u32 = 1;
pub const IMG_SCREEN: u32 = 2;
pub const IMG_FREEMEM: u32 = 4;

/// QBasic "Illegal function call".
pub const ERR_ILLEGAL_FUNCTION_CALL: i32 = 5;
/// QBNex "Invalid handle".
pub const ERR_INVALID_HANDLE: i32 = 258;

/// `qbr(long double)` from libqb.cpp applied to a value that started as a
/// float/double expression (exact widening), truncated to `int32` the way the
/// legacy assignments did.
#[inline]
pub fn qbr_i32(v: f64) -> i32 {
    crate::round::qbr(crate::round::f80_from_f64(v)) as i32
}

/// `qbr(long double)` keeping the 64-bit result.
#[inline]
pub fn qbr_i64(v: f64) -> i64 {
    crate::round::qbr(crate::round::f80_from_f64(v))
}

/// C/C++ float-to-int conversion (`cvttsd2si`): truncates toward zero and
/// yields `INT_MIN` for NaN and out-of-range values (Rust `as` saturates).
#[inline]
pub fn cvt_i32(v: f64) -> i32 {
    if v.is_nan() || v >= 2147483648.0 || v <= -2147483649.0 {
        i32::MIN
    } else {
        v as i32
    }
}

/// `qbr_float_to_long(float)`: the argument is converted to `float` first.
#[inline]
pub fn qbr_f2l(v: f32) -> i32 {
    crate::round::fistp_i32(f64::from(v))
}

/// `qbr_double_to_long(double)`.
#[inline]
pub fn qbr_d2l(v: f64) -> i32 {
    crate::round::fistp_i32(v)
}

/// Converts a coordinate pair through the page's WINDOW/VIEW mapping exactly
/// like the repeated legacy block (float arithmetic, `qbr_float_to_long`).
#[inline]
pub fn map_point_f2l(im: &ImgStruct, x: f32, y: f32) -> (i32, i32) {
    if im.clipping_or_scaling != 0 {
        if im.clipping_or_scaling == 2 {
            (
                qbr_f2l(x * im.scaling_x + im.scaling_offset_x).wrapping_add(im.view_offset_x),
                qbr_f2l(y * im.scaling_y + im.scaling_offset_y).wrapping_add(im.view_offset_y),
            )
        } else {
            (qbr_f2l(x).wrapping_add(im.view_offset_x), qbr_f2l(y).wrapping_add(im.view_offset_y))
        }
    } else {
        (qbr_f2l(x), qbr_f2l(y))
    }
}

#[cfg(test)]
pub(crate) mod testutil {
    use super::*;

    /// An owned surface for unit tests: the pixel and palette buffers live in
    /// the struct so the raw pointers in `img` stay valid.
    pub struct TestImage {
        pub img: ImgStruct,
        pub pixels: Vec<u32>,
        pub palette: Vec<u32>,
    }

    impl TestImage {
        /// Creates a surface set up like `imgnew(w, h, mode)`.
        pub fn new(w: u16, h: u16, mode: u16) -> Box<TestImage> {
            let mut t = Box::new(TestImage {
                img: ImgStruct::default(),
                pixels: vec![0u32; (w as usize * h as usize * 4).div_ceil(4) + 1],
                palette: vec![0u32; 256],
            });
            t.img.offset = t.pixels.as_mut_ptr() as *mut u8;
            t.img.pal = if mode == 32 { core::ptr::null_mut() } else { t.palette.as_mut_ptr() };
            image::setup_new_image(&mut t.img, w, h, mode);
            t
        }
        pub fn p8(&self, x: usize, y: usize) -> u8 {
            let w = self.img.width as usize;
            // SAFETY: test images are fully allocated.
            unsafe { *self.img.offset.add(y * w + x) }
        }
        pub fn p32(&self, x: usize, y: usize) -> u32 {
            self.pixels[y * self.img.width as usize + x]
        }
        pub fn bytes(&self) -> &[u8] {
            let n = self.img.width as usize * self.img.height as usize * self.img.bytes_per_pixel as usize;
            // SAFETY: the buffer holds at least n bytes.
            unsafe { core::slice::from_raw_parts(self.img.offset, n) }
        }
        pub fn count_nonzero(&self) -> usize {
            self.bytes().iter().filter(|b| **b != 0).count()
        }
        pub fn ptr(&mut self) -> *mut ImgStruct {
            &mut self.img
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{offset_of, size_of};

    #[test]
    fn img_struct_layout_matches_common_h() {
        // Offsets of img_struct as compiled by g++ for x86-64 (checked again
        // with static_assert in tests/diff_gfx.cpp).
        if size_of::<usize>() != 8 {
            return;
        }
        assert_eq!(offset_of!(ImgStruct, valid), 16);
        assert_eq!(offset_of!(ImgStruct, width), 20);
        assert_eq!(offset_of!(ImgStruct, bytes_per_pixel), 24);
        assert_eq!(offset_of!(ImgStruct, mask), 28);
        assert_eq!(offset_of!(ImgStruct, compatible_mode), 32);
        assert_eq!(offset_of!(ImgStruct, color), 36);
        assert_eq!(offset_of!(ImgStruct, font), 48);
        assert_eq!(offset_of!(ImgStruct, top_row), 52);
        assert_eq!(offset_of!(ImgStruct, cursor_show), 60);
        assert_eq!(offset_of!(ImgStruct, offset), 64);
        assert_eq!(offset_of!(ImgStruct, flags), 72);
        assert_eq!(offset_of!(ImgStruct, pal), 80);
        assert_eq!(offset_of!(ImgStruct, transparent_color), 88);
        assert_eq!(offset_of!(ImgStruct, alpha_disabled), 92);
        assert_eq!(offset_of!(ImgStruct, apm_p1), 95);
        assert_eq!(offset_of!(ImgStruct, view_x1), 96);
        assert_eq!(offset_of!(ImgStruct, x), 120);
        assert_eq!(offset_of!(ImgStruct, clipping_or_scaling), 128);
        assert_eq!(offset_of!(ImgStruct, scaling_x), 132);
        assert_eq!(offset_of!(ImgStruct, window_y2), 160);
        assert_eq!(offset_of!(ImgStruct, draw_ta), 168);
        assert_eq!(offset_of!(ImgStruct, draw_scale), 176);
        assert_eq!(offset_of!(ImgStruct, apm_p2), 184);
        assert_eq!(size_of::<ImgStruct>(), 192);
    }

    #[test]
    fn map_point_applies_window_and_view() {
        let mut im = ImgStruct::default();
        assert_eq!(map_point_f2l(&im, 2.5, 3.5), (2, 4)); // round half even
        im.clipping_or_scaling = 1;
        im.view_offset_x = 10;
        im.view_offset_y = 20;
        assert_eq!(map_point_f2l(&im, 1.0, 1.0), (11, 21));
        im.clipping_or_scaling = 2;
        im.scaling_x = 2.0;
        im.scaling_y = -1.0;
        im.scaling_offset_x = 1.0;
        im.scaling_offset_y = 100.0;
        assert_eq!(map_point_f2l(&im, 3.0, 5.0), (17, 115));
    }
}
