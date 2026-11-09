//! Printer output for `_PRINTIMAGE` and buffered `LPRINT` (Windows only, D8).
//!
//! Replaces `internal/c/libqb/printer.cpp`. The page flow is written against
//! the [`Device`] trait so it can be checked call by call against a verbatim
//! copy of the legacy code (`tests/diff/diff_printer.cpp`) without a printer.
//! Scaling the image to the page size stays in libqb.cpp (graphics engine):
//!
//! 1. [`begin`] opens the default printer, starts a document and a page and
//!    returns the printable size;
//! 2. the caller scales the image to that size (32-bit, top-down);
//! 3. [`finish`] sends the rows and closes the page and document, or
//!    [`abort`] closes them when the caller could not make the image.

/// The GDI calls the legacy printer code makes, in order.
pub trait Device {
    /// `GetDefaultPrinter`; legacy ignored failures (empty name here).
    fn default_printer(&mut self) -> Vec<u8>;
    /// `CreateDC("WINSPOOL", name)`; false when it returns NULL.
    fn create_dc(&mut self, name: &[u8]) -> bool;
    fn start_doc(&mut self) -> i32;
    fn start_page(&mut self) -> i32;
    /// `GetDeviceCaps(HORZRES)`, `GetDeviceCaps(VERTRES)`.
    fn page_size(&mut self) -> (i32, i32);
    /// `SetDIBitsToDevice` of one row at `y`.
    fn put_row(&mut self, y: i32, width: i32, height: i32, row: &[u32]);
    fn end_page(&mut self) -> i32;
    fn end_doc(&mut self) -> i32;
    fn delete_dc(&mut self);
}

/// Opens a page. Returns its size in pixels, or `None` (nothing to clean up).
pub fn begin(d: &mut impl Device) -> Option<(i32, i32)> {
    let name = d.default_printer();
    if !d.create_dc(&name) {
        return None;
    }
    if d.start_doc() <= 0 {
        d.delete_dc();
        return None;
    }
    if d.start_page() <= 0 {
        d.end_doc();
        d.delete_dc();
        return None;
    }
    Some(d.page_size())
}

/// Sends a `width` x `height` top-down image and ends the page and document.
pub fn finish(d: &mut impl Device, width: i32, height: i32, pixels: &[u32]) {
    let w = width.max(0) as usize;
    for y in 0..height.max(0) {
        let row = &pixels[y as usize * w..][..w];
        d.put_row(y, width, height, row);
    }
    if d.end_page() <= 0 {
        d.end_doc();
        d.delete_dc();
        return;
    }
    if d.end_doc() <= 0 {
        d.delete_dc();
        return;
    }
    d.delete_dc();
}

/// Ends the document without printing (the caller could not build the image).
pub fn abort(d: &mut impl Device) {
    d.end_doc();
    d.delete_dc();
}

#[cfg(windows)]
pub mod gdi {
    //! The real printer.
    use super::Device;
    use std::mem::{size_of, zeroed};
    use std::ptr::null;
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::Graphics::Gdi as gdi32;
    use windows_sys::Win32::Graphics::Printing::GetDefaultPrinterA;
    use windows_sys::Win32::Storage::Xps::*;

    pub struct Gdi {
        pub dc: HDC,
    }

    impl Device for Gdi {
        fn default_printer(&mut self) -> Vec<u8> {
            let mut buf = vec![0u8; 65536];
            let mut len = buf.len() as u32;
            // SAFETY: `buf` holds `len` bytes.
            if unsafe { GetDefaultPrinterA(buf.as_mut_ptr(), &mut len) } == 0 {
                return vec![0];
            }
            let end = buf.iter().position(|&b| b == 0).unwrap_or(0);
            buf.truncate(end + 1);
            buf
        }
        fn create_dc(&mut self, name: &[u8]) -> bool {
            // SAFETY: both strings are NUL-terminated.
            self.dc = unsafe { CreateDCA(c"WINSPOOL".as_ptr().cast(), name.as_ptr(), null(), null()) };
            !self.dc.is_null()
        }
        fn start_doc(&mut self) -> i32 {
            // SAFETY: DOCINFOA is plain data; the name is NUL-terminated.
            unsafe {
                let mut di: DOCINFOA = zeroed();
                di.cbSize = size_of::<DOCINFOA>() as i32;
                di.lpszDocName = c"Document".as_ptr().cast();
                StartDocA(self.dc, &di)
            }
        }
        fn start_page(&mut self) -> i32 {
            // SAFETY: valid DC.
            unsafe { StartPage(self.dc) }
        }
        fn page_size(&mut self) -> (i32, i32) {
            // SAFETY: valid DC.
            unsafe { (GetDeviceCaps(self.dc, gdi32::HORZRES as i32), GetDeviceCaps(self.dc, gdi32::VERTRES as i32)) }
        }
        fn put_row(&mut self, y: i32, width: i32, height: i32, row: &[u32]) {
            // SAFETY: `row` holds `width` pixels; the header describes them.
            unsafe {
                let mut bi: BITMAPINFO = zeroed();
                bi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
                bi.bmiHeader.biWidth = width;
                bi.bmiHeader.biHeight = height;
                bi.bmiHeader.biPlanes = 1;
                bi.bmiHeader.biBitCount = 32;
                bi.bmiHeader.biCompression = BI_RGB;
                SetDIBitsToDevice(self.dc, 0, y, width as u32, 1, 0, 0, 0, 1, row.as_ptr().cast(), &bi, DIB_RGB_COLORS);
            }
        }
        fn end_page(&mut self) -> i32 {
            // SAFETY: valid DC.
            unsafe { EndPage(self.dc) }
        }
        fn end_doc(&mut self) -> i32 {
            // SAFETY: valid DC.
            unsafe { EndDoc(self.dc) }
        }
        fn delete_dc(&mut self) {
            // SAFETY: valid DC, deleted once.
            unsafe { DeleteDC(self.dc) };
        }
    }
}

// --- C ABI (include/qb_runtime.h) ---------------------------------------------------

/// Opens a printer page. Returns a handle and the page size, or NULL (no
/// printer, or not Windows).
///
/// # Safety
/// `w` and `h` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_printer_begin(w: *mut i32, h: *mut i32) -> *mut std::ffi::c_void {
    #[cfg(windows)]
    {
        let mut d = Box::new(gdi::Gdi { dc: std::ptr::null_mut() });
        if let Some((pw, ph)) = begin(d.as_mut()) {
            // SAFETY: caller guarantees writable outputs.
            unsafe {
                w.write_unaligned(pw);
                h.write_unaligned(ph);
            }
            return Box::into_raw(d).cast();
        }
    }
    let _ = (w, h);
    std::ptr::null_mut()
}

/// Prints `w * h` top-down pixels (the size from `qb_printer_begin`) and
/// closes the handle.
///
/// # Safety
/// `p` must come from `qb_printer_begin`; `pixels` readable for `w * h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_printer_finish(p: *mut std::ffi::c_void, pixels: *const u32, w: i32, h: i32) {
    #[cfg(windows)]
    if !p.is_null() {
        // SAFETY: ownership comes back from `qb_printer_begin`.
        let mut d = unsafe { Box::from_raw(p.cast::<gdi::Gdi>()) };
        let n = (w.max(0) as usize) * (h.max(0) as usize);
        // SAFETY: caller guarantees the pixel count.
        let px = if n == 0 { &[][..] } else { unsafe { std::slice::from_raw_parts(pixels, n) } };
        finish(d.as_mut(), w, h, px);
    }
    let _ = (p, pixels, w, h);
}

/// Closes a handle from `qb_printer_begin` without printing.
///
/// # Safety
/// `p` must come from `qb_printer_begin`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_printer_abort(p: *mut std::ffi::c_void) {
    #[cfg(windows)]
    if !p.is_null() {
        // SAFETY: ownership comes back from `qb_printer_begin`.
        abort(unsafe { Box::from_raw(p.cast::<gdi::Gdi>()) }.as_mut());
    }
    let _ = p;
}

/// Call trace for the differential test. Codes: 1 GetDefaultPrinter,
/// 2 CreateDC, 3 StartDoc, 4 StartPage, 5 GetDeviceCaps (twice),
/// 6 SetDIBitsToDevice, 7 EndPage, 8 EndDoc, 9 DeleteDC.
#[derive(Default)]
pub struct Trace {
    pub calls: Vec<i32>,
    /// Bit `n` set: call code `n` fails.
    pub fail: u32,
    pub size: (i32, i32),
}

impl Trace {
    fn rc(&mut self, code: i32) -> i32 {
        self.calls.push(code);
        if self.fail & (1 << code) != 0 {
            0
        } else {
            1
        }
    }
}

impl Device for Trace {
    fn default_printer(&mut self) -> Vec<u8> {
        self.calls.push(1);
        b"printer\0".to_vec()
    }
    fn create_dc(&mut self, _name: &[u8]) -> bool {
        self.rc(2) != 0
    }
    fn start_doc(&mut self) -> i32 {
        self.rc(3)
    }
    fn start_page(&mut self) -> i32 {
        self.rc(4)
    }
    fn page_size(&mut self) -> (i32, i32) {
        self.calls.extend([5, 5]);
        self.size
    }
    fn put_row(&mut self, _y: i32, _w: i32, _h: i32, _row: &[u32]) {
        self.calls.push(6);
    }
    fn end_page(&mut self) -> i32 {
        self.rc(7)
    }
    fn end_doc(&mut self) -> i32 {
        self.rc(8)
    }
    fn delete_dc(&mut self) {
        self.calls.push(9);
    }
}

/// Runs the Rust flow on a [`Trace`] the way libqb's `sub__printimage` does
/// (`image_ok == 0`: the scaled image could not be created) and writes the
/// call codes into `out`. Returns the number of calls.
///
/// # Safety
/// `out` must be writable for `cap` values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_test_printer_trace(fail: u32, page_w: i32, page_h: i32, image_ok: i32, out: *mut i32, cap: i32) -> i32 {
    let mut t = Trace { fail, size: (page_w, page_h), ..Trace::default() };
    if let Some((w, h)) = begin(&mut t) {
        if image_ok != 0 {
            let px = vec![0u32; (w.max(0) as usize) * (h.max(0) as usize)];
            finish(&mut t, w, h, &px);
        } else {
            abort(&mut t);
        }
    }
    for (i, c) in t.calls.iter().take(cap.max(0) as usize).enumerate() {
        // SAFETY: caller guarantees `cap` writable values.
        unsafe { out.add(i).write_unaligned(*c) };
    }
    t.calls.len() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(fail: u32, image_ok: bool) -> Vec<i32> {
        let mut t = Trace { fail, size: (3, 2), ..Trace::default() };
        if let Some((w, h)) = begin(&mut t) {
            assert_eq!((w, h), (3, 2));
            if image_ok {
                finish(&mut t, w, h, &[0; 6]);
            } else {
                abort(&mut t);
            }
        }
        t.calls
    }

    #[test]
    fn prints_every_row_then_closes() {
        assert_eq!(run(0, true), [1, 2, 3, 4, 5, 5, 6, 6, 7, 8, 9]);
    }

    #[test]
    fn cleans_up_after_each_failure() {
        assert_eq!(run(1 << 2, true), [1, 2]);
        assert_eq!(run(1 << 3, true), [1, 2, 3, 9]);
        assert_eq!(run(1 << 4, true), [1, 2, 3, 4, 8, 9]);
        assert_eq!(run(0, false), [1, 2, 3, 4, 5, 5, 8, 9]);
        assert_eq!(run(1 << 7, true), [1, 2, 3, 4, 5, 5, 6, 6, 7, 8, 9]);
        assert_eq!(run(1 << 8, true), [1, 2, 3, 4, 5, 5, 6, 6, 7, 8, 9]);
    }

    #[test]
    fn c_abi_trace_matches() {
        let mut out = [0i32; 32];
        let n = unsafe { qb_test_printer_trace(0, 3, 2, 1, out.as_mut_ptr(), 32) };
        assert_eq!(&out[..n as usize], [1, 2, 3, 4, 5, 5, 6, 6, 7, 8, 9]);
    }
}
