//! SCREEN statement decisions (legacy `qbg_screen` in libqb.cpp).
//!
//! The orchestration (display lock, page table, freeing old surfaces) stays
//! in C++ for now because it touches window state owned by R8. What lives
//! here is the part that decides *what* to build: which mode numbers are
//! legal, the size of each graphics mode, the font override that keeps an
//! 80x43/80x60 text layout, and the SCREEN 0 layout with its page
//! granularity in `cmem` (segment &HB800).

/// `SCREEN n` with `n >= 0`: modes 3..6 and anything above 13 are rejected
/// with error 5 by the legacy code.
pub fn mode_valid(mode: i32) -> bool {
    !(3..=6).contains(&mode) && mode <= 13
}

/// Size of a graphics mode and the font override (`0` = mode default, `8` =
/// switch to the 8x8 font because the previous screen was 80x60 / 80x43).
/// `None` for mode 0, custom screens and unsupported modes.
pub fn graphics_geometry(mode: i32, prev_w: i32, prev_h: i32) -> Option<(i32, i32, i32)> {
    let keep = |h: i32| if prev_w == 80 && prev_h == h { 8 } else { 0 };
    Some(match mode {
        13 => (320, 200, 0),
        12 | 11 => (640, 480, keep(60)),
        10 | 9 => (640, 350, keep(43)),
        8 | 2 => (640, 200, 0),
        7 | 1 => (320, 200, 0),
        _ => return None,
    })
}

/// SCREEN 0 layout: columns, rows, font, bytes per page in `cmem` and the
/// number of pages (at most 8) that live in `cmem` after &HB800.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextLayout {
    pub width: i32,
    pub height: i32,
    pub font: i32,
    pub granularity: i32,
    pub cmem_pages: i32,
}

/// Pending WIDTH/font overrides (`-1` = not set). The legacy code consumes
/// only the ones a branch uses; the others stay pending (bug-for-bug).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Overrides {
    pub width: i32,
    pub height: i32,
    pub font: i32,
}

pub fn text_layout(prev_w: i32, prev_h: i32, ov: &mut Overrides) -> TextLayout {
    let (x, y, mut f);
    if ov.width != -1 && ov.height != -1 && ov.font != -1 {
        (x, y, f) = (ov.width, ov.height, ov.font);
        *ov = Overrides { width: -1, height: -1, font: -1 };
    } else if ov.width != -1 {
        x = ov.width;
        ov.width = -1;
        let (yy, ff) = match prev_h {
            43 => (43, 14),
            50 => (50, 8),
            _ => (25, 16),
        };
        y = yy;
        f = if x == 40 { ff + 1 } else { ff };
    } else if ov.height != -1 {
        y = ov.height;
        ov.height = -1;
        f = match y {
            43 => 14,
            50 => 8,
            _ => 16,
        };
        if prev_w == 40 {
            f += 1;
            x = 40;
        } else {
            x = 80;
        }
    } else {
        (x, y, f) = match (prev_w, prev_h) {
            (80, 50) => (80, 50, 8),
            (40, 50) => (40, 50, 9),
            (80, 43) => (80, 43, 8),
            (40, 43) => (40, 43, 9),
            (40, 25) => (40, 25, 17),
            _ => (80, 25, 16),
        };
    }
    let mut granularity = x.wrapping_mul(y).wrapping_mul(2);
    // D-08: legacy wrote `(f=(16+1))` and `(f=16)` (assignments), forcing
    // fonts 17/16 on any 40x25/80x25 screen. Every WIDTH path passes those
    // fonts anyway, so the comparison changes nothing reachable today.
    if x == 40 && y == 25 && f == 17 {
        granularity = 2048;
    }
    if x == 80 && y == 25 && f == 16 {
        granularity = 4096;
    }
    // Widths/heights are validated by WIDTH, so granularity is positive;
    // a zero would be a division by zero (crash) in the legacy code.
    let cmem_pages = if granularity > 0 { (65536 / granularity).min(8) } else { 8 };
    TextLayout { width: x, height: y, font: f, granularity, cmem_pages }
}

/// C ABI: 1 when `SCREEN mode` (mode >= 0) is legal.
#[no_mangle]
pub extern "C" fn qbrt_gfx_screen_mode_valid(mode: i32) -> i32 {
    mode_valid(mode) as i32
}

/// C ABI: `out = {width, height, font override}`; returns 0 when `mode` is
/// not a graphics mode.
///
/// # Safety
/// `out` must point to 3 writable `int32`.
#[no_mangle]
pub unsafe extern "C" fn qbrt_gfx_screen_geometry(mode: i32, prev_w: i32, prev_h: i32, out: *mut i32) -> i32 {
    match graphics_geometry(mode, prev_w, prev_h) {
        Some((w, h, f)) => {
            let out = unsafe { std::slice::from_raw_parts_mut(out, 3) };
            out.copy_from_slice(&[w, h, f]);
            1
        }
        None => 0,
    }
}

/// C ABI: SCREEN 0 layout. The three override pointers are the legacy
/// `sub_screen_*` globals and are reset to -1 where the legacy code did.
/// `out = {width, height, font, granularity, cmem pages}`.
///
/// # Safety
/// All pointers must be valid; `out` must hold 5 `int32`.
#[no_mangle]
pub unsafe extern "C" fn qbrt_gfx_screen0_layout(
    prev_w: i32,
    prev_h: i32,
    width: *mut i32,
    height: *mut i32,
    font: *mut i32,
    out: *mut i32,
) {
    unsafe {
        let mut ov = Overrides { width: *width, height: *height, font: *font };
        let l = text_layout(prev_w, prev_h, &mut ov);
        *width = ov.width;
        *height = ov.height;
        *font = ov.font;
        let out = std::slice::from_raw_parts_mut(out, 5);
        out.copy_from_slice(&[l.width, l.height, l.font, l.granularity, l.cmem_pages]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Overrides = Overrides { width: -1, height: -1, font: -1 };

    fn layout(pw: i32, ph: i32, ov: Overrides) -> (TextLayout, Overrides) {
        let mut o = ov;
        (text_layout(pw, ph, &mut o), o)
    }

    #[test]
    fn valid_modes_match_legacy() {
        let ok: Vec<i32> = (0..=20).filter(|&m| mode_valid(m)).collect();
        assert_eq!(ok, vec![0, 1, 2, 7, 8, 9, 10, 11, 12, 13]);
    }

    #[test]
    fn graphics_sizes() {
        assert_eq!(graphics_geometry(13, 0, 0), Some((320, 200, 0)));
        assert_eq!(graphics_geometry(12, 80, 25), Some((640, 480, 0)));
        assert_eq!(graphics_geometry(12, 80, 60), Some((640, 480, 8)));
        assert_eq!(graphics_geometry(11, 80, 60), Some((640, 480, 8)));
        assert_eq!(graphics_geometry(9, 80, 43), Some((640, 350, 8)));
        assert_eq!(graphics_geometry(10, 80, 60), Some((640, 350, 0)));
        assert_eq!(graphics_geometry(8, 80, 43), Some((640, 200, 0)));
        assert_eq!(graphics_geometry(7, 0, 0), Some((320, 200, 0)));
        assert_eq!(graphics_geometry(2, 0, 0), Some((640, 200, 0)));
        assert_eq!(graphics_geometry(1, 0, 0), Some((320, 200, 0)));
        assert_eq!(graphics_geometry(0, 0, 0), None);
        assert_eq!(graphics_geometry(4, 0, 0), None);
    }

    #[test]
    fn screen0_defaults_follow_previous_size() {
        let l = |pw, ph| layout(pw, ph, NONE).0;
        assert_eq!(l(0, 0), TextLayout { width: 80, height: 25, font: 16, granularity: 4096, cmem_pages: 8 });
        assert_eq!(l(40, 25), TextLayout { width: 40, height: 25, font: 17, granularity: 2048, cmem_pages: 8 });
        assert_eq!(l(80, 50), TextLayout { width: 80, height: 50, font: 8, granularity: 8000, cmem_pages: 8 });
        assert_eq!(l(80, 43), TextLayout { width: 80, height: 43, font: 8, granularity: 6880, cmem_pages: 8 });
        assert_eq!(l(40, 43), TextLayout { width: 40, height: 43, font: 9, granularity: 3440, cmem_pages: 8 });
        // 640x480 graphics screen in characters: not a text size, so default
        assert_eq!(l(80, 30).width, 80);
    }

    #[test]
    fn screen0_width_override() {
        let (l, o) = layout(80, 43, Overrides { width: 40, height: -1, font: 14 });
        assert_eq!((l.width, l.height, l.font), (40, 43, 15));
        // only the width is consumed: the stale font override stays pending
        assert_eq!(o, Overrides { width: -1, height: -1, font: 14 });
    }

    #[test]
    fn screen0_height_override() {
        let (l, o) = layout(40, 25, Overrides { width: -1, height: 50, font: -1 });
        assert_eq!((l.width, l.height, l.font, l.granularity), (40, 50, 9, 4000));
        assert_eq!(o, NONE);
    }

    #[test]
    fn screen0_full_override_keeps_requested_font() {
        // D-08: legacy `(f=(16+1))` / `(f=16)` overwrote the font; the fixed
        // granularities apply only to the standard 40x25/80x25 fonts
        let (l, o) = layout(80, 25, Overrides { width: 40, height: 25, font: 8 });
        assert_eq!((l.font, l.granularity), (8, 2000));
        assert_eq!(o, NONE);
        let (l, _) = layout(80, 25, Overrides { width: 80, height: 25, font: 8 });
        assert_eq!((l.font, l.granularity), (8, 4000));
        let (l, _) = layout(80, 25, Overrides { width: 40, height: 25, font: 17 });
        assert_eq!((l.font, l.granularity), (17, 2048));
        let (l, _) = layout(40, 25, Overrides { width: 80, height: 25, font: 16 });
        assert_eq!((l.font, l.granularity), (16, 4096));
        let (l, _) = layout(80, 25, Overrides { width: 100, height: 60, font: 8 });
        assert_eq!((l.font, l.granularity, l.cmem_pages), (8, 12000, 5));
        let (l, _) = layout(80, 25, Overrides { width: 400, height: 200, font: 16 });
        assert_eq!(l.cmem_pages, 0);
    }
}
