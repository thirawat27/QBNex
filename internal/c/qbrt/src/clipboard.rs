//! `_CLIPBOARD$` (read/write) and `_CLIPBOARDIMAGE` (read/write).
//!
//! Windows keeps the legacy formats byte for byte: text is `CF_TEXT` (raw
//! bytes, read up to the first NUL), images are `CF_BITMAP`. Linux and macOS
//! use `arboard`; the system clipboard there is UTF-8, so text that is not
//! valid UTF-8 is stored with U+FFFD replacements (deviation D-07). When no
//! system clipboard is reachable (no display), a clipboard private to the
//! process is used instead, like legacy's `internal_clipboard`.
//!
//! Images cross the C ABI as top-down BGRA pixels (`0xAARRGGBB` words);
//! images read from the clipboard always have alpha 255.

/// An image as top-down `0xAARRGGBB` pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub width: i32,
    pub height: i32,
    pub pixels: Vec<u32>,
}

pub use platform::{get_image, get_text, set_image, set_text};

#[cfg(windows)]
mod platform {
    use super::Image;
    use std::mem::{size_of, zeroed};
    use std::ptr::null_mut;
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::System::DataExchange::*;
    use windows_sys::Win32::System::Memory::*;
    use windows_sys::Win32::System::Ole::{CF_BITMAP, CF_TEXT};

    /// Puts `text` on the clipboard as `CF_TEXT` (legacy `sub__clipboard`).
    pub fn set_text(text: &[u8]) {
        // SAFETY: the global block is allocated with room for the text and a
        // NUL, filled while locked and handed to the clipboard.
        unsafe {
            if OpenClipboard(null_mut()) == 0 {
                return;
            }
            EmptyClipboard();
            let h = GlobalAlloc(GMEM_MOVEABLE, text.len() + 1);
            if !h.is_null() {
                let p = GlobalLock(h).cast::<u8>();
                if !p.is_null() {
                    std::ptr::copy_nonoverlapping(text.as_ptr(), p, text.len());
                    *p.add(text.len()) = 0;
                    GlobalUnlock(h);
                    SetClipboardData(u32::from(CF_TEXT), h);
                }
            }
            CloseClipboard();
        }
    }

    /// `CF_TEXT` up to the first NUL (legacy `func__clipboard`).
    pub fn get_text() -> Vec<u8> {
        let mut out = Vec::new();
        // SAFETY: the clipboard block is only read while locked; CF_TEXT data
        // is NUL-terminated.
        unsafe {
            if OpenClipboard(null_mut()) == 0 {
                return out;
            }
            if IsClipboardFormatAvailable(u32::from(CF_TEXT)) != 0 {
                let h = GetClipboardData(u32::from(CF_TEXT));
                if !h.is_null() {
                    let p = GlobalLock(h).cast::<std::ffi::c_char>();
                    if !p.is_null() {
                        out = std::ffi::CStr::from_ptr(p).to_bytes().to_vec();
                        GlobalUnlock(h);
                    }
                }
            }
            CloseClipboard();
        }
        out
    }

    fn header(w: i32, h: i32) -> BITMAPINFO {
        // SAFETY: BITMAPINFO is plain data.
        let mut bi: BITMAPINFO = unsafe { zeroed() };
        bi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        bi.bmiHeader.biWidth = w;
        bi.bmiHeader.biHeight = h; // negative: top-down
        bi.bmiHeader.biPlanes = 1;
        bi.bmiHeader.biBitCount = 32;
        bi.bmiHeader.biCompression = BI_RGB;
        bi
    }

    /// Puts a screen-compatible `CF_BITMAP` copy of the image on the
    /// clipboard (legacy `sub__clipboardimage`).
    pub fn set_image(img: &Image) {
        let (w, h) = (img.width, img.height);
        // SAFETY: GDI objects are created and released here; the DIB section
        // holds w*h 32-bit pixels. The bitmap given to the clipboard belongs
        // to the system afterwards and is not deleted.
        unsafe {
            let hdc = GetDC(null_mut());
            let bi = header(w, -h);
            let mut bits: *mut std::ffi::c_void = null_mut();
            let dib = CreateDIBSection(hdc, &bi, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
            if dib.is_null() || bits.is_null() {
                ReleaseDC(null_mut(), hdc);
                return;
            }
            std::ptr::copy_nonoverlapping(img.pixels.as_ptr(), bits.cast::<u32>(), img.pixels.len());
            let copy = CreateCompatibleBitmap(hdc, w, h);
            let dst = CreateCompatibleDC(hdc);
            let src = CreateCompatibleDC(hdc);
            let old_dst = SelectObject(dst, copy);
            let old_src = SelectObject(src, dib);
            BitBlt(dst, 0, 0, w, h, src, 0, 0, SRCCOPY);
            SelectObject(dst, old_dst);
            SelectObject(src, old_src);
            DeleteDC(dst);
            DeleteDC(src);
            ReleaseDC(null_mut(), hdc);
            DeleteObject(dib);
            if OpenClipboard(null_mut()) != 0 {
                EmptyClipboard();
                if SetClipboardData(u32::from(CF_BITMAP), copy).is_null() {
                    DeleteObject(copy);
                }
                CloseClipboard();
            } else {
                DeleteObject(copy);
            }
        }
    }

    /// The `CF_BITMAP` image as 32-bit pixels with alpha 255 (legacy
    /// `func__clipboardimage`).
    pub fn get_image() -> Option<Image> {
        // SAFETY: the clipboard-owned bitmap is only read while the clipboard
        // is open; GetDIBits writes at most w*h pixels.
        unsafe {
            if OpenClipboard(null_mut()) == 0 {
                return None;
            }
            let mut result = None;
            if IsClipboardFormatAvailable(u32::from(CF_BITMAP)) != 0 {
                let bitmap = GetClipboardData(u32::from(CF_BITMAP));
                let mut info: BITMAP = zeroed();
                GetObjectW(bitmap, size_of::<BITMAP>() as i32, (&mut info as *mut BITMAP).cast());
                let (w, h) = (info.bmWidth, info.bmHeight);
                if w > 0 && h > 0 {
                    let mut px = vec![0u32; w as usize * h as usize];
                    let mut bi = header(w, -h);
                    let hdc = GetDC(null_mut());
                    GetDIBits(hdc, bitmap, 0, h as u32, px.as_mut_ptr().cast(), &mut bi, DIB_RGB_COLORS);
                    ReleaseDC(null_mut(), hdc);
                    for p in &mut px {
                        *p |= 0xFF00_0000;
                    }
                    result = Some(Image { width: w, height: h, pixels: px });
                }
            }
            CloseClipboard();
            result
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use super::Image;
    use std::sync::Mutex;

    enum Backend {
        System(arboard::Clipboard),
        /// No display: a clipboard private to this process.
        Private { text: Vec<u8>, image: Option<Image> },
    }

    static CLIP: Mutex<Option<Backend>> = Mutex::new(None);

    fn with<T>(f: impl FnOnce(&mut Backend) -> T) -> T {
        let mut guard = CLIP.lock().unwrap_or_else(|e| e.into_inner());
        let backend = guard.get_or_insert_with(|| match arboard::Clipboard::new() {
            Ok(c) => Backend::System(c),
            Err(_) => Backend::Private { text: Vec::new(), image: None },
        });
        f(backend)
    }

    fn until_nul(b: &[u8]) -> &[u8] {
        &b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())]
    }

    pub fn set_text(text: &[u8]) {
        with(|b| match b {
            Backend::System(c) => {
                let _ = c.set_text(String::from_utf8_lossy(text).into_owned());
            }
            Backend::Private { text: t, .. } => *t = text.to_vec(),
        })
    }

    pub fn get_text() -> Vec<u8> {
        with(|b| match b {
            Backend::System(c) => c.get_text().map(|s| until_nul(s.as_bytes()).to_vec()).unwrap_or_default(),
            Backend::Private { text, .. } => until_nul(text).to_vec(),
        })
    }

    pub fn set_image(img: &Image) {
        with(|b| match b {
            Backend::System(c) => {
                let bytes: Vec<u8> = img
                    .pixels
                    .iter()
                    .flat_map(|&p| [(p >> 16) as u8, (p >> 8) as u8, p as u8, (p >> 24) as u8])
                    .collect();
                let _ = c.set_image(arboard::ImageData {
                    width: img.width as usize,
                    height: img.height as usize,
                    bytes: bytes.into(),
                });
            }
            Backend::Private { image, .. } => *image = Some(img.clone()),
        })
    }

    pub fn get_image() -> Option<Image> {
        with(|b| match b {
            Backend::System(c) => {
                let data = c.get_image().ok()?;
                let pixels = data
                    .bytes
                    .chunks_exact(4)
                    .map(|p| 0xFF00_0000 | u32::from(p[0]) << 16 | u32::from(p[1]) << 8 | u32::from(p[2]))
                    .collect();
                Some(Image { width: data.width as i32, height: data.height as i32, pixels })
            }
            Backend::Private { image, .. } => image.clone().map(|mut i| {
                i.pixels.iter_mut().for_each(|p| *p |= 0xFF00_0000);
                i
            }),
        })
    }
}


// --- C ABI (include/qbrt.h) ---------------------------------------------------

/// # Safety
/// `text` must be readable for `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn qbrt_clipboard_set_text(text: *const u8, len: i32) {
    let text = if text.is_null() || len <= 0 {
        &[][..]
    } else {
        // SAFETY: caller guarantees `text` covers `len` bytes.
        unsafe { std::slice::from_raw_parts(text, len as usize) }
    };
    set_text(text);
}

/// Returns the clipboard text (free with `qbrt_free_bytes`) and its length,
/// or NULL with length 0.
///
/// # Safety
/// `len` must be writable.
#[no_mangle]
pub unsafe extern "C" fn qbrt_clipboard_get_text(len: *mut i64) -> *mut u8 {
    let t = get_text();
    // SAFETY: caller guarantees `len` is writable.
    unsafe { len.write_unaligned(t.len() as i64) };
    crate::pixels::bytes_into_raw(t)
}

/// # Safety
/// `pixels` must be readable for `w * h` values.
#[no_mangle]
pub unsafe extern "C" fn qbrt_clipboard_set_image(pixels: *const u32, w: i32, h: i32) {
    if pixels.is_null() || w <= 0 || h <= 0 {
        return;
    }
    // SAFETY: caller guarantees `w * h` readable pixels.
    let px = unsafe { std::slice::from_raw_parts(pixels, w as usize * h as usize) };
    set_image(&Image { width: w, height: h, pixels: px.to_vec() });
}

/// Returns the clipboard image (free with `qbrt_free_pixels`, `w * h`
/// pixels) or NULL when there is none.
///
/// # Safety
/// `w` and `h` must be writable.
#[no_mangle]
pub unsafe extern "C" fn qbrt_clipboard_get_image(w: *mut i32, h: *mut i32) -> *mut u32 {
    let img = get_image().filter(|i| i.width > 0 && i.height > 0);
    let (iw, ih, px) = img.map_or((0, 0, Vec::new()), |i| (i.width, i.height, i.pixels));
    // SAFETY: caller guarantees writable outputs.
    unsafe {
        w.write_unaligned(iw);
        h.write_unaligned(ih);
    }
    crate::pixels::into_raw(px)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One test touches the real clipboard so nothing runs concurrently with
    /// it. The user's text is put back afterwards. Without a desktop session
    /// (CI service accounts, headless Linux) the private clipboard is used,
    /// and the same assertions hold.
    #[test]
    fn clipboard_round_trips() {
        let saved = get_text();

        set_text(b"QBNex clipboard test");
        assert_eq!(get_text(), b"QBNex clipboard test");
        set_text(b"");
        assert_eq!(get_text(), b"");
        set_text(b"line 1\r\nline 2\t!");
        assert_eq!(get_text(), b"line 1\r\nline 2\t!");
        set_text(b"ab\0cd");
        assert_eq!(get_text(), b"ab", "text ends at the first NUL");
        #[cfg(windows)]
        {
            set_text(b"caf\xe9 \xa9");
            assert_eq!(get_text(), b"caf\xe9 \xa9", "CF_TEXT keeps bytes");
        }

        let pixels: Vec<u32> = (0..12u32).map(|i| 0xFF00_0000 | (i * 0x0010_2030)).collect();
        let img = Image { width: 4, height: 3, pixels };
        set_image(&img);
        if let Some(got) = get_image() {
            assert_eq!(got, img);
            assert_eq!(get_image(), Some(img.clone()), "a second read still works");
        } else {
            eprintln!("clipboard image unavailable here; skipped image assertions");
        }
        set_image(&Image { width: 1, height: 1, pixels: vec![0x0012_3456] });
        if let Some(got) = get_image() {
            assert_eq!(got.pixels, [0xFF12_3456], "alpha is forced to 255");
        }

        set_text(&saved);
    }
}
