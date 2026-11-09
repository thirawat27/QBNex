//! `_SCREENIMAGE`, `_SCREENCLICK` and `_SCREENPRINT`: desktop capture and
//! synthetic mouse/keyboard input.
//!
//! The event sequences are computed by pure functions (`win_*`, `mac_*`,
//! `screen_region`) that reproduce legacy libqb.cpp exactly, quirks included,
//! and are compared with a verbatim copy of the legacy code in
//! `tests/diff_screen.cpp`. Thin platform layers send them:
//!
//! | | Windows | macOS | Linux |
//! | --- | --- | --- | --- |
//! | `_SCREENIMAGE` | GDI `BitBlt` + `GetDIBits` | blank image of the screen size | blank image |
//! | `_SCREENCLICK` | `SendInput` | CoreGraphics mouse events | nothing |
//! | `_SCREENPRINT` | `SendInput` + `VkKeyScanA` | CoreGraphics key events | nothing |
//!
//! These are the legacy platform gaps; they are kept as they were.

/// `INPUT_MOUSE` flags (winuser.h).
pub const MOUSEEVENTF_MOVE: u32 = 0x0001;
pub const MOUSEEVENTF_LEFTDOWN: u32 = 0x0002;
pub const MOUSEEVENTF_LEFTUP: u32 = 0x0004;
pub const MOUSEEVENTF_RIGHTDOWN: u32 = 0x0008;
pub const MOUSEEVENTF_RIGHTUP: u32 = 0x0010;
pub const MOUSEEVENTF_MIDDLEDOWN: u32 = 0x0020;
pub const MOUSEEVENTF_MIDDLEUP: u32 = 0x0040;
pub const MOUSEEVENTF_ABSOLUTE: u32 = 0x8000;

/// Windows virtual keys used by `_SCREENPRINT`.
pub const VK_BACK: u16 = 0x08;
pub const VK_TAB: u16 = 0x09;
pub const VK_RETURN: u16 = 0x0D;
pub const VK_SHIFT: u16 = 0x10;
pub const VK_CONTROL: u16 = 0x11;
pub const VK_PRIOR: u16 = 0x21;
pub const VK_NEXT: u16 = 0x22;
pub const VK_END: u16 = 0x23;
pub const VK_HOME: u16 = 0x24;
pub const VK_LEFT: u16 = 0x25;
pub const VK_UP: u16 = 0x26;
pub const VK_RIGHT: u16 = 0x27;
pub const VK_DOWN: u16 = 0x28;
pub const VK_INSERT: u16 = 0x2D;
pub const VK_DELETE: u16 = 0x2E;

/// One `SendInput` mouse record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mouse {
    pub flags: u32,
    pub dx: i32,
    pub dy: i32,
}

/// One `SendInput` keyboard record (`KEYEVENTF_KEYUP` when `up`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WinKey {
    pub vk: u16,
    pub up: bool,
}

/// CoreGraphics event flags set with `CGEventSetFlags`.
pub const CG_FLAG_SHIFT: u64 = 0x0002_0000;
pub const CG_FLAG_COMMAND: u64 = 0x0010_0000;

/// One `CGEventCreateKeyboardEvent` + `CGEventPost` (flags 0: not set).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacKey {
    pub code: u16,
    pub down: bool,
    pub flags: u64,
}

/// One CoreGraphics left-button event at a point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacClick {
    pub x: i32,
    pub y: i32,
    pub down: bool,
}

pub const MACVK_TAB: u16 = 0x30;
pub const MACVK_COMMAND: u16 = 0x37;
pub const MACVK_SHIFT: u16 = 0x38;
pub const MACVK_HELP: u16 = 0x72;
pub const MACVK_HOME: u16 = 0x73;
pub const MACVK_PAGE_UP: u16 = 0x74;
pub const MACVK_FORWARD_DELETE: u16 = 0x75;
pub const MACVK_END: u16 = 0x77;
pub const MACVK_PAGE_DOWN: u16 = 0x79;
pub const MACVK_LEFT_ARROW: u16 = 0x7B;
pub const MACVK_RIGHT_ARROW: u16 = 0x7C;
pub const MACVK_DOWN_ARROW: u16 = 0x7D;
pub const MACVK_UP_ARROW: u16 = 0x7E;

/// Legacy `ASCII_TO_MACVK`: key code in the high byte, ASCII value in the low
/// 7 bits (0 = no key), bit 7 = SHIFT needed.
#[rustfmt::skip]
pub static ASCII_TO_MACVK: [u16; 256] = [
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x3308, 0x3009, 0x0000, 0x0000, 0x0000, 0x240D, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x351B, 0x0000, 0x0000, 0x0000, 0x0000,
    0x3120, 0x12A1, 0x27A2, 0x14A3, 0x15A4, 0x17A5, 0x1AA6, 0x2727,
    0x19A8, 0x1DA9, 0x1CAA, 0x18AB, 0x2B2C, 0x1B2D, 0x2F2E, 0x2C2F,
    0x1D30, 0x1231, 0x1332, 0x1433, 0x1534, 0x1735, 0x1636, 0x1A37,
    0x1C38, 0x1939, 0x29BA, 0x293B, 0x2BBC, 0x183D, 0x2FBE, 0x2CBF,
    0x13C0, 0x00C1, 0x0BC2, 0x08C3, 0x02C4, 0x0EC5, 0x03C6, 0x05C7,
    0x04C8, 0x22C9, 0x26CA, 0x28CB, 0x25CC, 0x2ECD, 0x2DCE, 0x1FCF,
    0x23D0, 0x0CD1, 0x0FD2, 0x01D3, 0x11D4, 0x20D5, 0x09D6, 0x0DD7,
    0x07D8, 0x10D9, 0x06DA, 0x215B, 0x2A5C, 0x1E5D, 0x16DE, 0x1BDF,
    0x3260, 0x0061, 0x0B62, 0x0863, 0x0264, 0x0E65, 0x0366, 0x0567,
    0x0468, 0x2269, 0x266A, 0x286B, 0x256C, 0x2E6D, 0x2D6E, 0x1F6F,
    0x2370, 0x0C71, 0x0F72, 0x0173, 0x1174, 0x2075, 0x0976, 0x0D77,
    0x0778, 0x1079, 0x067A, 0x21FB, 0x2AFC, 0x1EFD, 0x32FE, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
];

/// `_SCREENIMAGE` capture rectangle `(x, y, width, height)` on a screen of
/// `screen_w` x `screen_h` pixels. Only the far edges are clipped when a
/// region is given (legacy behaviour).
pub fn screen_region(screen_w: i32, screen_h: i32, x1: i32, y1: i32, x2: i32, y2: i32, passed: bool) -> (i32, i32, i32, i32) {
    if !passed {
        return (0, 0, screen_w, screen_h);
    }
    let (x1, y1) = (x1.max(0), y1.max(0));
    let (x2, y2) = (x2.min(screen_w - 1), y2.min(screen_h - 1));
    (x1, y1, x2 - x1 + 1, y2 - y1 + 1)
}

/// `_SCREENCLICK x, y[, button]` on Windows.
pub fn win_screenclick(x: i32, y: i32, button: i32, passed: bool, screen_w: i32, screen_h: i32) -> Vec<Mouse> {
    // Legacy: fx = x * (65535.0f / width), stored into a LONG (truncation).
    let scale = |v: i32, size: i32| (f64::from(v) * (65535.0f64 / f64::from(size))) as i32;
    let m = |flags| Mouse { flags, dx: 0, dy: 0 };
    let mut ev = vec![
        Mouse { flags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE, dx: scale(x, screen_w), dy: scale(y, screen_h) },
        m(MOUSEEVENTF_LEFTDOWN),
    ];
    if passed {
        let (down, up) = match button {
            1 => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
            2 => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
            3 => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
            _ => (0, 0),
        };
        ev.extend([m(down), m(up)]);
    } else {
        ev.extend([m(MOUSEEVENTF_LEFTDOWN), m(MOUSEEVENTF_LEFTUP)]);
    }
    ev
}

/// `_SCREENCLICK` on macOS: left button down and up (the button is ignored).
pub fn mac_screenclick(x: i32, y: i32) -> Vec<MacClick> {
    vec![MacClick { x, y, down: true }, MacClick { x, y, down: false }]
}

/// `_SCREENPRINT text$` on Windows. `vk_key_scan` is `VkKeyScanA`.
pub fn win_screenprint(txt: &[u8], vk_key_scan: impl Fn(u8) -> i16) -> Vec<WinKey> {
    let mut ev = Vec::new();
    let tap = |ev: &mut Vec<WinKey>, vk| ev.extend([WinKey { vk, up: false }, WinKey { vk, up: true }]);
    let held = |ev: &mut Vec<WinKey>, modifier, vk| {
        ev.push(WinKey { vk: modifier, up: false });
        tap(ev, vk);
        ev.push(WinKey { vk: modifier, up: true });
    };
    let mut i = 0;
    while i < txt.len() {
        let c = txt[i];
        match c {
            9 => tap(&mut ev, VK_TAB),
            8 => tap(&mut ev, VK_BACK),
            13 => tap(&mut ev, VK_RETURN),
            1..=26 => held(&mut ev, VK_CONTROL, (vk_key_scan(64 + c) & 255) as u16),
            0 => {
                if i + 1 < txt.len() {
                    i += 1;
                    let vk = match txt[i] {
                        15 => {
                            held(&mut ev, VK_SHIFT, VK_TAB);
                            None
                        }
                        75 => Some(VK_LEFT),
                        77 => Some(VK_RIGHT),
                        72 => Some(VK_UP),
                        80 => Some(VK_DOWN),
                        82 => Some(VK_INSERT),
                        71 => Some(VK_HOME),
                        83 => Some(VK_DELETE),
                        79 => Some(VK_END),
                        81 => Some(VK_NEXT),
                        73 => Some(VK_PRIOR),
                        _ => None,
                    };
                    if let Some(vk) = vk {
                        tap(&mut ev, vk);
                    }
                }
            }
            32..=126 => {
                let x = vk_key_scan(c);
                let vk = (x & 255) as u16;
                if (x >> 8) & 1 != 0 {
                    held(&mut ev, VK_SHIFT, vk);
                } else {
                    tap(&mut ev, vk);
                }
            }
            _ => {}
        }
        i += 1;
    }
    ev
}

/// `_SCREENPRINT text$` on macOS.
pub fn mac_screenprint(txt: &[u8]) -> Vec<MacKey> {
    let mut ev = Vec::new();
    let tap = |ev: &mut Vec<MacKey>, code| {
        ev.extend([MacKey { code, down: true, flags: 0 }, MacKey { code, down: false, flags: 0 }]);
    };
    let held = |ev: &mut Vec<MacKey>, modifier, code, flags| {
        ev.extend([
            MacKey { code: modifier, down: true, flags: 0 },
            MacKey { code, down: true, flags },
            MacKey { code, down: false, flags: 0 },
            MacKey { code: modifier, down: false, flags: 0 },
        ]);
    };
    let mut i = 0;
    while i < txt.len() {
        let c = txt[i];
        if (1..=26).contains(&c) && c != 8 && c != 9 && c != 13 {
            // COMMAND replaces CONTROL on macOS.
            held(&mut ev, MACVK_COMMAND, ASCII_TO_MACVK[usize::from(c) + 96] >> 8, CG_FLAG_COMMAND);
        } else if c == 0 {
            if i + 1 < txt.len() {
                i += 1;
                let code = match txt[i] {
                    15 => {
                        held(&mut ev, MACVK_SHIFT, MACVK_TAB, CG_FLAG_SHIFT);
                        None
                    }
                    75 => Some(MACVK_LEFT_ARROW),
                    77 => Some(MACVK_RIGHT_ARROW),
                    72 => Some(MACVK_UP_ARROW),
                    80 => Some(MACVK_DOWN_ARROW),
                    82 => Some(MACVK_HELP),
                    71 => Some(MACVK_HOME),
                    83 => Some(MACVK_FORWARD_DELETE),
                    79 => Some(MACVK_END),
                    81 => Some(MACVK_PAGE_DOWN),
                    73 => Some(MACVK_PAGE_UP),
                    _ => None,
                };
                if let Some(code) = code {
                    tap(&mut ev, code);
                }
            }
        } else {
            let x = ASCII_TO_MACVK[usize::from(c)];
            if x & 127 != 0 {
                if x & 128 != 0 {
                    held(&mut ev, MACVK_SHIFT, x >> 8, CG_FLAG_SHIFT);
                } else {
                    tap(&mut ev, x >> 8);
                }
            }
        }
        i += 1;
    }
    ev
}


#[cfg(windows)]
mod platform {
    use super::*;
    use std::mem::{size_of, zeroed};
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetDesktopWindow, GetWindowRect};

    pub fn desktop_size() -> (i32, i32) {
        // SAFETY: plain Win32 calls with a local RECT.
        unsafe {
            let mut r: RECT = zeroed();
            GetWindowRect(GetDesktopWindow(), &mut r);
            (r.right - r.left, r.bottom - r.top)
        }
    }

    fn send(input: &INPUT) {
        // SAFETY: one fully initialised INPUT record.
        unsafe { SendInput(1, input, size_of::<INPUT>() as i32) };
    }

    pub fn send_mouse(events: &[Mouse]) {
        for e in events {
            // SAFETY: INPUT is plain data; zero is a valid bit pattern.
            let mut input: INPUT = unsafe { zeroed() };
            input.r#type = INPUT_MOUSE;
            input.Anonymous.mi.dwFlags = e.flags;
            input.Anonymous.mi.dx = e.dx;
            input.Anonymous.mi.dy = e.dy;
            send(&input);
        }
    }

    pub fn send_keys(events: &[WinKey]) {
        for e in events {
            // SAFETY: INPUT is plain data; zero is a valid bit pattern.
            let mut input: INPUT = unsafe { zeroed() };
            input.r#type = INPUT_KEYBOARD;
            input.Anonymous.ki.wVk = e.vk;
            input.Anonymous.ki.dwFlags = if e.up { KEYEVENTF_KEYUP } else { 0 };
            send(&input);
        }
    }

    pub fn vk_key_scan(c: u8) -> i16 {
        // SAFETY: plain Win32 call.
        unsafe { VkKeyScanA(c as i8) }
    }

    /// Captures a desktop rectangle as top-down BGRA pixels with alpha 255.
    pub fn capture(x: i32, y: i32, w: i32, h: i32) -> Vec<u32> {
        let mut px = vec![0u32; (w as usize) * (h as usize)];
        // SAFETY: GDI objects are created and released locally; GetDIBits
        // writes at most w*h 32-bit pixels into `px`.
        unsafe {
            let hdc = GetDC(std::ptr::null_mut());
            let mem = CreateCompatibleDC(hdc);
            let bitmap = CreateCompatibleBitmap(hdc, w, h);
            let old = SelectObject(mem, bitmap);
            BitBlt(mem, 0, 0, w, h, hdc, x, y, SRCCOPY);
            SelectObject(mem, old);
            let mut bi: BITMAPINFO = zeroed();
            bi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
            bi.bmiHeader.biWidth = w;
            bi.bmiHeader.biHeight = -h; // top-down
            bi.bmiHeader.biPlanes = 1;
            bi.bmiHeader.biBitCount = 32;
            bi.bmiHeader.biCompression = BI_RGB;
            GetDIBits(mem, bitmap, 0, h as u32, px.as_mut_ptr().cast(), &mut bi, DIB_RGB_COLORS);
            DeleteObject(bitmap);
            DeleteDC(mem);
            ReleaseDC(std::ptr::null_mut(), hdc);
        }
        for p in &mut px {
            *p |= 0xFF00_0000; // legacy: _SETALPHA 255
        }
        px
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use std::ffi::c_void;

    #[repr(C)]
    struct CGPoint {
        x: f64,
        y: f64,
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventSourceCreate(state: i32) -> *mut c_void;
        fn CGEventCreateKeyboardEvent(src: *mut c_void, key: u16, down: bool) -> *mut c_void;
        fn CGEventCreateMouseEvent(src: *mut c_void, kind: u32, at: CGPoint, button: u32) -> *mut c_void;
        fn CGEventSetFlags(e: *mut c_void, flags: u64);
        fn CGEventPost(tap: u32, e: *mut c_void);
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFRelease(cf: *const c_void);
    }

    const COMBINED_SESSION_STATE: i32 = 0;
    const HID_EVENT_TAP: u32 = 0;
    const ANNOTATED_SESSION_EVENT_TAP: u32 = 2;
    const LEFT_MOUSE_DOWN: u32 = 1;
    const LEFT_MOUSE_UP: u32 = 2;

    pub fn send_keys(events: &[MacKey]) {
        // SAFETY: CoreGraphics objects are created, posted and released here.
        unsafe {
            let es = CGEventSourceCreate(COMBINED_SESSION_STATE);
            for k in events {
                let e = CGEventCreateKeyboardEvent(es, k.code, k.down);
                if k.flags != 0 {
                    CGEventSetFlags(e, k.flags);
                }
                CGEventPost(ANNOTATED_SESSION_EVENT_TAP, e);
                CFRelease(e);
            }
            if !es.is_null() {
                CFRelease(es);
            }
        }
    }

    pub fn send_clicks(events: &[MacClick]) {
        // SAFETY: as above.
        unsafe {
            for c in events {
                let kind = if c.down { LEFT_MOUSE_DOWN } else { LEFT_MOUSE_UP };
                let e = CGEventCreateMouseEvent(std::ptr::null_mut(), kind, CGPoint { x: c.x as f64, y: c.y as f64 }, 0);
                CGEventPost(HID_EVENT_TAP, e);
                CFRelease(e);
            }
        }
    }
}

// --- C ABI (include/qbrt.h) ---------------------------------------------------

/// `_SCREENIMAGE`: returns top-down BGRA pixels (free with `qbrt_free_pixels`)
/// and their size, or NULL where capture is not supported (the caller then
/// makes a blank image of the screen size). A NULL result with a non-positive
/// size means "invalid region".
///
/// # Safety
/// `w` and `h` must be writable.
#[no_mangle]
pub unsafe extern "C" fn qbrt_screenimage(x1: i32, y1: i32, x2: i32, y2: i32, passed: i32, w: *mut i32, h: *mut i32) -> *mut u32 {
    #[cfg(windows)]
    {
        use std::sync::OnceLock;
        // Legacy measured the desktop once, on the first call.
        static SIZE: OnceLock<(i32, i32)> = OnceLock::new();
        let (sw, sh) = *SIZE.get_or_init(platform::desktop_size);
        let (x, y, cw, ch) = screen_region(sw, sh, x1, y1, x2, y2, passed != 0);
        // SAFETY: caller guarantees writable outputs.
        unsafe {
            w.write_unaligned(cw);
            h.write_unaligned(ch);
        }
        if cw <= 0 || ch <= 0 {
            return std::ptr::null_mut();
        }
        crate::pixels::into_raw(platform::capture(x, y, cw, ch))
    }
    #[cfg(not(windows))]
    {
        let _ = (x1, y1, x2, y2, passed);
        // SAFETY: caller guarantees writable outputs.
        unsafe {
            w.write_unaligned(1);
            h.write_unaligned(1);
        }
        std::ptr::null_mut()
    }
}

/// `_SCREENCLICK x, y[, button]`.
#[no_mangle]
pub extern "C" fn qbrt_screenclick(x: i32, y: i32, button: i32, passed: i32) {
    #[cfg(windows)]
    {
        let (sw, sh) = platform::desktop_size();
        platform::send_mouse(&win_screenclick(x, y, button, passed != 0, sw, sh));
    }
    #[cfg(target_os = "macos")]
    {
        let _ = (button, passed);
        platform::send_clicks(&mac_screenclick(x, y));
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    let _ = (x, y, button, passed);
}

/// `_SCREENPRINT text$`.
///
/// # Safety
/// `txt` must be readable for `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn qbrt_screenprint(txt: *const u8, len: i32) {
    let txt = if txt.is_null() || len <= 0 {
        &[][..]
    } else {
        // SAFETY: caller guarantees `txt` covers `len` bytes.
        unsafe { std::slice::from_raw_parts(txt, len as usize) }
    };
    #[cfg(windows)]
    platform::send_keys(&win_screenprint(txt, platform::vk_key_scan));
    #[cfg(target_os = "macos")]
    platform::send_keys(&mac_screenprint(txt));
    #[cfg(not(any(windows, target_os = "macos")))]
    let _ = txt;
}

// --- Test hooks for the differential harness (tests/diff_screen.cpp) ----------

/// Writes `screen_region` into `out[4]`.
///
/// # Safety
/// `out` must be writable for 4 `int32_t`.
#[no_mangle]
pub unsafe extern "C" fn qbrt_test_screen_region(sw: i32, sh: i32, x1: i32, y1: i32, x2: i32, y2: i32, passed: i32, out: *mut i32) {
    let (a, b, c, d) = screen_region(sw, sh, x1, y1, x2, y2, passed != 0);
    for (i, v) in [a, b, c, d].into_iter().enumerate() {
        // SAFETY: caller guarantees 4 writable values.
        unsafe { out.add(i).write_unaligned(v) };
    }
}

/// Writes `win_screenclick` as `flags, dx, dy` triples into `out` (room for
/// `cap` triples) and returns the event count.
///
/// # Safety
/// `out` must be writable for `3 * cap` `int32_t`.
#[no_mangle]
pub unsafe extern "C" fn qbrt_test_win_screenclick(x: i32, y: i32, button: i32, passed: i32, sw: i32, sh: i32, out: *mut i32, cap: i32) -> i32 {
    let ev = win_screenclick(x, y, button, passed != 0, sw, sh);
    for (i, e) in ev.iter().take(cap.max(0) as usize).enumerate() {
        // SAFETY: caller guarantees room for `cap` triples.
        unsafe {
            out.add(3 * i).write_unaligned(e.flags as i32);
            out.add(3 * i + 1).write_unaligned(e.dx);
            out.add(3 * i + 2).write_unaligned(e.dy);
        }
    }
    ev.len() as i32
}

/// Writes `win_screenprint` (with `VkKeyScanA` replaced by the table
/// `scan[256]`) as `vk | up << 16` into `out` and returns the event count.
///
/// # Safety
/// `txt` readable for `len` bytes, `scan` for 256 values, `out` writable for `cap`.
#[no_mangle]
pub unsafe extern "C" fn qbrt_test_win_screenprint(txt: *const u8, len: i32, scan: *const i16, out: *mut i32, cap: i32) -> i32 {
    // SAFETY: caller guarantees the buffers.
    let (txt, scan) = unsafe { (std::slice::from_raw_parts(txt, len.max(0) as usize), std::slice::from_raw_parts(scan, 256)) };
    let ev = win_screenprint(txt, |c| scan[c as usize]);
    for (i, e) in ev.iter().take(cap.max(0) as usize).enumerate() {
        // SAFETY: caller guarantees room for `cap` values.
        unsafe { out.add(i).write_unaligned(i32::from(e.vk) | (i32::from(e.up) << 16)) };
    }
    ev.len() as i32
}

/// Writes `mac_screenprint` as `code, down, flags_low32` triples into `out`
/// (room for `cap` triples) and returns the event count.
///
/// # Safety
/// `txt` readable for `len` bytes, `out` writable for `3 * cap`.
#[no_mangle]
pub unsafe extern "C" fn qbrt_test_mac_screenprint(txt: *const u8, len: i32, out: *mut i32, cap: i32) -> i32 {
    // SAFETY: caller guarantees the buffers.
    let txt = unsafe { std::slice::from_raw_parts(txt, len.max(0) as usize) };
    let ev = mac_screenprint(txt);
    for (i, e) in ev.iter().take(cap.max(0) as usize).enumerate() {
        // SAFETY: caller guarantees room for `cap` triples.
        unsafe {
            out.add(3 * i).write_unaligned(i32::from(e.code));
            out.add(3 * i + 1).write_unaligned(i32::from(e.down));
            out.add(3 * i + 2).write_unaligned(e.flags as i32);
        }
    }
    ev.len() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(vk: u16, up: bool) -> WinKey {
        WinKey { vk, up }
    }

    fn tap(vk: u16) -> [WinKey; 2] {
        [k(vk, false), k(vk, true)]
    }

    /// US layout: letters need SHIFT when upper case; '!' is SHIFT+'1'.
    fn us_scan(c: u8) -> i16 {
        match c {
            b'a'..=b'z' => i16::from(c - 32),
            b'A'..=b'Z' => 0x100 | i16::from(c),
            b'0'..=b'9' | b' ' => i16::from(c),
            b'!' => 0x100 | 0x31,
            b'~' => -1, // no key for it
            _ => 0xBA,
        }
    }

    #[test]
    fn region_defaults_to_whole_screen() {
        assert_eq!(screen_region(1920, 1080, 5, 5, 6, 6, false), (0, 0, 1920, 1080));
    }

    #[test]
    fn region_clips_only_far_edges() {
        assert_eq!(screen_region(800, 600, 10, 20, 109, 219, true), (10, 20, 100, 200));
        assert_eq!(screen_region(800, 600, -5, -5, 2000, 2000, true), (0, 0, 800, 600));
        assert_eq!(screen_region(800, 600, 100, 100, 50, 50, true), (100, 100, -49, -49));
        assert_eq!(screen_region(800, 600, 900, 0, 1000, 0, true), (900, 0, -100, 1));
    }

    #[test]
    fn click_sends_move_then_buttons() {
        let mv = Mouse { flags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE, dx: 32767, dy: 32767 };
        let m = |flags| Mouse { flags, dx: 0, dy: 0 };
        // Legacy always presses the left button once before the requested one.
        assert_eq!(
            win_screenclick(400, 300, 0, false, 800, 600),
            [mv, m(MOUSEEVENTF_LEFTDOWN), m(MOUSEEVENTF_LEFTDOWN), m(MOUSEEVENTF_LEFTUP)]
        );
        assert_eq!(
            win_screenclick(400, 300, 2, true, 800, 600),
            [mv, m(MOUSEEVENTF_LEFTDOWN), m(MOUSEEVENTF_RIGHTDOWN), m(MOUSEEVENTF_RIGHTUP)]
        );
        assert_eq!(
            win_screenclick(400, 300, 3, true, 800, 600)[2..],
            [m(MOUSEEVENTF_MIDDLEDOWN), m(MOUSEEVENTF_MIDDLEUP)]
        );
        assert_eq!(win_screenclick(400, 300, 7, true, 800, 600)[2..], [m(0), m(0)]);
        assert_eq!(win_screenclick(1, 1, 1, true, 3, 3)[0], Mouse { dx: 21845, dy: 21845, ..mv });
        assert_eq!(win_screenclick(-1, 10_000, 1, true, 1024, 768)[0], Mouse { dx: -63, dy: 853_320, ..mv });
    }

    #[test]
    fn mac_click_is_left_down_up() {
        assert_eq!(
            mac_screenclick(3, 4),
            [MacClick { x: 3, y: 4, down: true }, MacClick { x: 3, y: 4, down: false }]
        );
    }

    #[test]
    fn win_print_plain_and_shifted_characters() {
        assert_eq!(win_screenprint(b"a", us_scan), tap(b'A' as u16));
        assert_eq!(
            win_screenprint(b"A!", us_scan),
            [
                k(VK_SHIFT, false), k(b'A' as u16, false), k(b'A' as u16, true), k(VK_SHIFT, true),
                k(VK_SHIFT, false), k(0x31, false), k(0x31, true), k(VK_SHIFT, true),
            ]
        );
        // VkKeyScan = -1: legacy still sends SHIFT and key 255.
        assert_eq!(win_screenprint(b"~", us_scan), [k(VK_SHIFT, false), k(255, false), k(255, true), k(VK_SHIFT, true)]);
        // 127..255 and other control characters are skipped.
        assert_eq!(win_screenprint(&[127, 128, 200, 255, 27, 31], us_scan), []);
    }

    #[test]
    fn win_print_control_characters() {
        assert_eq!(win_screenprint(b"\t\x08\r", us_scan), [tap(VK_TAB), tap(VK_BACK), tap(VK_RETURN)].concat());
        // CTRL+C (3): VkKeyScan(64 + 3) & 255, SHIFT bit ignored.
        assert_eq!(
            win_screenprint(&[3], us_scan),
            [k(VK_CONTROL, false), k(b'C' as u16, false), k(b'C' as u16, true), k(VK_CONTROL, true)]
        );
        assert_eq!(win_screenprint(&[26], us_scan)[1], k(b'Z' as u16, false));
    }

    #[test]
    fn win_print_extended_keys() {
        let ext = |c: u8| win_screenprint(&[0, c], us_scan);
        assert_eq!(ext(75), tap(VK_LEFT));
        assert_eq!(ext(77), tap(VK_RIGHT));
        assert_eq!(ext(72), tap(VK_UP));
        assert_eq!(ext(80), tap(VK_DOWN));
        assert_eq!(ext(82), tap(VK_INSERT));
        assert_eq!(ext(71), tap(VK_HOME));
        assert_eq!(ext(83), tap(VK_DELETE));
        assert_eq!(ext(79), tap(VK_END));
        assert_eq!(ext(81), tap(VK_NEXT));
        assert_eq!(ext(73), tap(VK_PRIOR));
        assert_eq!(ext(15), [k(VK_SHIFT, false), k(VK_TAB, false), k(VK_TAB, true), k(VK_SHIFT, true)]);
        assert_eq!(ext(59), []); // F1: not implemented in legacy
        assert_eq!(win_screenprint(&[0], us_scan), []); // trailing CHR$(0)
        assert_eq!(win_screenprint(&[0, 75, b'a'], us_scan), [tap(VK_LEFT), tap(b'A' as u16)].concat());
    }

    fn mk(code: u16, down: bool, flags: u64) -> MacKey {
        MacKey { code, down, flags }
    }

    #[test]
    fn mac_print_uses_table_and_shift() {
        assert_eq!(mac_screenprint(b"a"), [mk(0x00, true, 0), mk(0x00, false, 0)]);
        assert_eq!(
            mac_screenprint(b"A"),
            [mk(MACVK_SHIFT, true, 0), mk(0x00, true, CG_FLAG_SHIFT), mk(0x00, false, 0), mk(MACVK_SHIFT, false, 0)]
        );
        assert_eq!(mac_screenprint(b"\t"), [mk(MACVK_TAB, true, 0), mk(MACVK_TAB, false, 0)]);
        assert_eq!(mac_screenprint(&[200]), []);
    }

    #[test]
    fn mac_print_command_and_extended() {
        // CTRL+C becomes COMMAND+C (key code of 'c' = 0x08).
        assert_eq!(
            mac_screenprint(&[3]),
            [mk(MACVK_COMMAND, true, 0), mk(0x08, true, CG_FLAG_COMMAND), mk(0x08, false, 0), mk(MACVK_COMMAND, false, 0)]
        );
        assert_eq!(mac_screenprint(&[0, 75]), [mk(MACVK_LEFT_ARROW, true, 0), mk(MACVK_LEFT_ARROW, false, 0)]);
        assert_eq!(mac_screenprint(&[0, 83]), [mk(MACVK_FORWARD_DELETE, true, 0), mk(MACVK_FORWARD_DELETE, false, 0)]);
        assert_eq!(
            mac_screenprint(&[0, 15]),
            [mk(MACVK_SHIFT, true, 0), mk(MACVK_TAB, true, CG_FLAG_SHIFT), mk(MACVK_TAB, false, 0), mk(MACVK_SHIFT, false, 0)]
        );
        assert_eq!(mac_screenprint(&[0]), []);
    }
}
