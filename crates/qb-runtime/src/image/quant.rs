//! 32bpp -> 8bpp conversion used by `_LOADIMAGE(f$, 256)`: ordered dithering
//! into a 3-3-2 colour cube and nearest-colour palette remapping. Ported from
//! `image_convert_8bpp` / `image_remap_palette`.

/// Returns the 8bpp indices (None if they cannot be allocated) and fills
/// `palette` (BGRA, alpha 0xFF). The cube sums are `uint32_t` in C and wrap.
pub fn to_8bpp(bgra: &[u8], w: usize, h: usize, palette: &mut [u32; 256]) -> Option<Vec<u8>> {
    const BAYER: [i32; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];
    let clamp = |n: i32| n.clamp(0, 255) as u32;
    let mut cubes = [[0u32; 4]; 256]; // r, g, b sums and count
    let mut out = Vec::new();
    out.try_reserve_exact(w * h).ok()?;
    let mut src = bgra.as_chunks::<4>().0.iter();
    for y in 0..h {
        for x in 0..w {
            let p = src.next().unwrap_or(&[0; 4]);
            let t = BAYER[((y & 3) << 2) + (x & 3)] << 1;
            let b = clamp(p[0] as i32 + t);
            let g = clamp(p[1] as i32 + t);
            let r = clamp(p[2] as i32 + t);
            let k = (((r >> 5) << 5) + ((g >> 5) << 2) + (b >> 6)) as u8;
            out.push(k);
            let c = &mut cubes[k as usize];
            c[0] = c[0].wrapping_add(r);
            c[1] = c[1].wrapping_add(g);
            c[2] = c[2].wrapping_add(b);
            c[3] = c[3].wrapping_add(1);
        }
    }
    for (p, c) in palette.iter_mut().zip(cubes.iter()) {
        *p = match (
            c[0].checked_div(c[3]),
            c[1].checked_div(c[3]),
            c[2].checked_div(c[3]),
        ) {
            (Some(r), Some(g), Some(b)) => make_bgra(r, g, b),
            _ => make_bgra(0, 0, 0),
        };
    }
    Some(out)
}

/// `IMAGE_MAKE_BGRA(r, g, b, 0xFF)`
fn make_bgra(r: u32, g: u32, b: u32) -> u32 {
    (b & 0xFF) | (g & 0xFF) << 8 | (r & 0xFF) << 16 | 0xFF00_0000
}

/// Rewrites `pixels` from `src_pal` indices to the nearest `dst_pal` entries.
///
/// Legacy compared float `sqrt` distances with a strict `>`; the squared
/// integer distances order the same way (sqrt is monotonic and neighbouring
/// sums stay apart in float), and the start value is the black-white
/// distance, so a colour exactly that far away is never picked (index 0).
pub fn remap_palette(pixels: &mut [u8], src_pal: &[u32; 256], dst_pal: &[u32; 256]) {
    let rgb = |c: u32| {
        [
            (c >> 16 & 0xFF) as i32,
            (c >> 8 & 0xFF) as i32,
            (c & 0xFF) as i32,
        ]
    };
    let mut map = [0u8; 256];
    for (x, m) in map.iter_mut().enumerate() {
        let a = rgb(src_pal[x]);
        let mut best = 3 * 255 * 255;
        for (y, &d) in dst_pal.iter().enumerate() {
            let b = rgb(d);
            let dist = (0..3).map(|i| (b[i] - a[i]) * (b[i] - a[i])).sum::<i32>();
            if best > dist {
                best = dist;
                *m = y as u8;
            }
        }
    }
    for p in pixels.iter_mut() {
        *p = map[*p as usize];
    }
}

/// C ABI, see `qb_runtime.h`.
///
/// # Safety
/// `bgra` must hold `w*h*4` bytes and `palette_out` 256 writable entries.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_image_to_8bpp(
    bgra: *const u8,
    w: i32,
    h: i32,
    palette_out: *mut u32,
) -> *mut u8 {
    if bgra.is_null() || palette_out.is_null() || w < 0 || h < 0 {
        return std::ptr::null_mut();
    }
    let (w, h) = (w as usize, h as usize);
    // SAFETY: caller guarantees the sizes.
    let src = unsafe { std::slice::from_raw_parts(bgra, w * h * 4) };
    let mut pal = [0u32; 256];
    let Some(out) = to_8bpp(src, w, h, &mut pal) else {
        return std::ptr::null_mut();
    };
    // SAFETY: caller guarantees 256 entries; palette_out may be unaligned.
    unsafe {
        std::ptr::copy_nonoverlapping(pal.as_ptr() as *const u8, palette_out as *mut u8, 1024)
    };
    crate::cmem::to_c_heap(&out)
}

/// C ABI, see `qb_runtime.h`.
///
/// # Safety
/// `pixels` must hold `w*h` bytes; both palettes 256 entries.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_image_remap_palette(
    pixels: *mut u8,
    w: i32,
    h: i32,
    src_pal: *const u32,
    dst_pal: *const u32,
) {
    if pixels.is_null() || src_pal.is_null() || dst_pal.is_null() || w <= 0 || h <= 0 {
        return;
    }
    let mut a = [0u32; 256];
    let mut b = [0u32; 256];
    // SAFETY: caller guarantees the sizes; palettes may be unaligned.
    unsafe {
        std::ptr::copy_nonoverlapping(src_pal as *const u8, a.as_mut_ptr() as *mut u8, 1024);
        std::ptr::copy_nonoverlapping(dst_pal as *const u8, b.as_mut_ptr() as *mut u8, 1024);
        remap_palette(
            std::slice::from_raw_parts_mut(pixels, w as usize * h as usize),
            &a,
            &b,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dithers_into_332_cube_and_averages_palette() {
        // 2x1 BGRA: pure red, pure white. Bayer offsets at (0,0)=0 and (1,0)=16.
        let src = [0, 0, 255, 255, 255, 255, 255, 255];
        let mut pal = [0u32; 256];
        let idx = to_8bpp(&src, 2, 1, &mut pal).unwrap();
        assert_eq!(idx, vec![0b1110_0000, 0xFF]);
        assert_eq!(pal[0b1110_0000], 0xFFFF_0000);
        assert_eq!(pal[0xFF], 0xFFFF_FFFF);
        assert_eq!(pal[1], 0xFF00_0000);
        // dither pushes a mid gray (100) at x=1 (t=16) across a cube edge
        let mut pal = [0u32; 256];
        let idx = to_8bpp(&[100, 100, 100, 0, 100, 100, 100, 0], 2, 1, &mut pal).unwrap();
        assert_eq!(idx, vec![(3 << 5) + (3 << 2) + 1, (3 << 5) + (3 << 2) + 1]);
    }

    #[test]
    fn remap_picks_first_nearest() {
        let mut src = [0u32; 256];
        src[0] = 0xFF00_0000; // black
        src[1] = 0xFF10_2030;
        let mut dst = [0xFFFF_FFFFu32; 256]; // all white: exactly the max distance from black
        dst[7] = 0xFF11_2030;
        dst[9] = 0xFF11_2030;
        let mut px = [0u8, 1, 1];
        remap_palette(&mut px, &src, &dst);
        assert_eq!(px, [7, 7, 7]); // equal distances: the first index wins
    }
}
