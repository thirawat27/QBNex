//! Keyboard translation: OS key events -> the QBNex key codes that
//! `keydown()`/`keyup()` in libqb.cpp consume.
//!
//! Legacy path (Windows): freeglut turned `WM_KEYDOWN`/`WM_CHAR`/`WM_KEYUP`
//! into GLUT callbacks, and `GLUT_key_special`/`GLUT_key_ascii` in libqb.cpp
//! turned those into `keydown_vk`/`keydown_ascii`/`keydown_unicode` calls.
//! This module keeps that two-stage shape:
//!
//! * stage 2 ([`glut_special`], [`glut_ascii`]) is a verbatim port of the two
//!   libqb.cpp functions and is checked by `tests/diff/diff_keymap.cpp`;
//! * stage 1 ([`translate`]) replaces the freeglut Win32 message translation.
//!   Its input is an OS-neutral [`KeyInput`] built by the window layer (or by
//!   an input script).

/// Offsets used by libqb.cpp for key codes.
pub const VK: u32 = 100_000;
pub const QBK: u32 = 200_000;
pub const UC: u32 = 1_073_741_824;

/// `QBVK_*` values from libqb.cpp (only those used by the translation).
pub mod qbvk {
    pub const KP0: u32 = 256;
    pub const KP_PERIOD: u32 = 266;
    pub const KP_DIVIDE: u32 = 267;
    pub const KP_MULTIPLY: u32 = 268;
    pub const KP_MINUS: u32 = 269;
    pub const KP_PLUS: u32 = 270;
    pub const KP_ENTER: u32 = 271;
    pub const F15: u32 = 296;
    pub const NUMLOCK: u32 = 300;
    pub const CAPSLOCK: u32 = 301;
    pub const SCROLLOCK: u32 = 302;
    pub const RSHIFT: u32 = 303;
    pub const LSHIFT: u32 = 304;
    pub const RCTRL: u32 = 305;
    pub const LCTRL: u32 = 306;
    pub const RALT: u32 = 307;
    pub const LALT: u32 = 308;
    pub const MODE: u32 = 313;
    pub const SYSREQ: u32 = 317;
    pub const BREAK: u32 = 318;
    pub const PAUSE: u32 = 19;
}

/// GLUT special key codes (freeglut_std.h / freeglut_ext.h).
pub mod glut {
    pub const KEY_F1: i32 = 0x0001;
    pub const KEY_LEFT: i32 = 0x0064;
    pub const KEY_UP: i32 = 0x0065;
    pub const KEY_RIGHT: i32 = 0x0066;
    pub const KEY_DOWN: i32 = 0x0067;
    pub const KEY_PAGE_UP: i32 = 0x0068;
    pub const KEY_PAGE_DOWN: i32 = 0x0069;
    pub const KEY_HOME: i32 = 0x006A;
    pub const KEY_END: i32 = 0x006B;
    pub const KEY_INSERT: i32 = 0x006C;
    pub const KEY_SHIFT_L: i32 = 0x0070;
    pub const KEY_SHIFT_R: i32 = 0x0071;
    pub const KEY_CTRL_L: i32 = 0x0072;
    pub const KEY_CTRL_R: i32 = 0x0073;
    pub const KEY_ALT_L: i32 = 0x0074;
    pub const KEY_ALT_R: i32 = 0x0075;
}

/// One call into libqb.cpp's key handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    /// `keydown(x)` (legacy `keydown_vk`/`keydown_ascii`).
    Down(u32),
    /// `keyup(x)`.
    Up(u32),
    /// `keydown_unicode(x)`.
    DownUnicode(u32),
    /// `keyup_unicode(x)`.
    UpUnicode(u32),
}

/// Keys the window layer distinguishes. Everything that produces text is
/// [`Key::Char`]; keys without text that legacy still reported on release are
/// [`Key::Other`] carrying their Windows virtual-key code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    F(u8),
    PageUp,
    PageDown,
    Home,
    End,
    Left,
    Up,
    Right,
    Down,
    Insert,
    Delete,
    LCtrl,
    RCtrl,
    LShift,
    RShift,
    LAlt,
    RAlt,
    Pause,
    /// Ctrl+Break (`VK_CANCEL`).
    Cancel,
    Enter,
    Escape,
    Tab,
    Backspace,
    /// A key that produces this character (shift/AltGr applied, Ctrl not).
    Char(char),
    /// A key without text, identified by its Windows virtual-key code.
    Other(u32),
}

/// An OS key event in neutral form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyInput {
    pub pressed: bool,
    /// Auto-repeat of a held key.
    pub repeat: bool,
    pub key: Key,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

/// Port of `GLUT_key_special` (libqb.cpp), CORE_FREEGLUT branch.
pub fn glut_special(key: i32, down: bool) -> Option<KeyAction> {
    let vk: u32 = match key {
        1..=10 => 0x3B00 + ((key as u32 - 1) << 8), // F1..F10 = 0x3B00..0x4400
        11 => 0x8500,
        12 => 0x8600,
        glut::KEY_LEFT => 0x4B00,
        glut::KEY_UP => 0x4800,
        glut::KEY_RIGHT => 0x4D00,
        glut::KEY_DOWN => 0x5000,
        glut::KEY_PAGE_UP => 0x4900,
        glut::KEY_PAGE_DOWN => 0x5100,
        glut::KEY_HOME => 0x4700,
        glut::KEY_END => 0x4F00,
        glut::KEY_INSERT => 0x5200,
        glut::KEY_SHIFT_L => VK + qbvk::LSHIFT,
        glut::KEY_SHIFT_R => VK + qbvk::RSHIFT,
        glut::KEY_CTRL_L => VK + qbvk::LCTRL,
        glut::KEY_CTRL_R => VK + qbvk::RCTRL,
        glut::KEY_ALT_L => VK + qbvk::LALT,
        glut::KEY_ALT_R => VK + qbvk::RALT,
        _ => return None,
    };
    Some(if down {
        KeyAction::Down(vk)
    } else {
        KeyAction::Up(vk)
    })
}

/// Port of `GLUT_key_ascii` (libqb.cpp). `key` is the `unsigned char` freeglut
/// passed; `ctrl`/`shift` come from `glutGetModifiers()`. `macos` enables the
/// DEL/backspace swap of the macOS build.
pub fn glut_ascii(key: i32, ctrl: bool, shift: bool, down: bool, macos: bool) -> KeyAction {
    let mut key = key;
    // Is CTRL down? If so, unencode the character (applying shift; caps lock assumed off).
    if ctrl {
        if key == 10 {
            key = 13;
        } else if (1..=26).contains(&key) {
            key = if shift { key - 1 + 65 } else { key - 1 + 97 };
        }
    }
    if macos {
        // swap DEL and backspace keys
        if key == 8 {
            key = 127;
        } else if key == 127 {
            key = 8;
        }
    }
    let code = if key == 127 { 0x5300 } else { key as u32 };
    if down {
        KeyAction::Down(code)
    } else {
        KeyAction::Up(code)
    }
}

/// The `WM_CHAR` code (or the `ToAscii` result on release) Windows produced
/// for a character key, given the Ctrl state.
fn char_code(c: char, ctrl: bool) -> u32 {
    let c = c as u32;
    if ctrl {
        match c {
            0x41..=0x5A | 0x61..=0x7A => return c & 0x1F,
            0x5B => return 27, // [
            0x5C => return 28, //             0x5D => return 29, // ]
            _ => {}
        }
    }
    c
}

/// Replaces freeglut's Win32 keyboard translation: returns the libqb.cpp
/// calls legacy made for this event, in order.
pub fn translate(ev: &KeyInput) -> Vec<KeyAction> {
    // No DEL/backspace swap here, on any OS: legacy macOS swapped because GLUT
    // reported the backspace key as 0x7F, but `Key::Backspace` / `Key::Delete`
    // are already the physical keys (mapped to 8 / 127 below).
    let macos = false;
    let down = ev.pressed;
    let ascii = |code: u32| {
        vec![glut_ascii(
            (code & 0xFF) as i32,
            ev.ctrl,
            ev.shift,
            down,
            macos,
        )]
    };
    let special = |code: i32| glut_special(code, down).into_iter().collect::<Vec<_>>();
    match ev.key {
        // freeglut polled the modifier keys: one down, one up, no repeats
        Key::LCtrl | Key::RCtrl | Key::LShift | Key::RShift | Key::LAlt | Key::RAlt
            if down && ev.repeat =>
        {
            Vec::new()
        }
        Key::LShift => special(glut::KEY_SHIFT_L),
        Key::RShift => special(glut::KEY_SHIFT_R),
        Key::LCtrl => special(glut::KEY_CTRL_L),
        Key::RCtrl => special(glut::KEY_CTRL_R),
        Key::LAlt => special(glut::KEY_ALT_L),
        Key::RAlt => special(glut::KEY_ALT_R),
        Key::F(n) if (1..=12).contains(&n) => special(n as i32),
        Key::F(_) => Vec::new(),
        Key::PageUp => special(glut::KEY_PAGE_UP),
        Key::PageDown => special(glut::KEY_PAGE_DOWN),
        Key::Home => special(glut::KEY_HOME),
        Key::End => special(glut::KEY_END),
        Key::Left => special(glut::KEY_LEFT),
        Key::Up => special(glut::KEY_UP),
        Key::Right => special(glut::KEY_RIGHT),
        Key::Down => special(glut::KEY_DOWN),
        Key::Insert => special(glut::KEY_INSERT),
        Key::Delete => ascii(127),
        Key::Pause | Key::Cancel => {
            let vk = VK
                + if ev.key == Key::Pause {
                    qbvk::PAUSE
                } else {
                    qbvk::BREAK
                };
            vec![if down {
                KeyAction::Down(vk)
            } else {
                KeyAction::Up(vk)
            }]
        }
        Key::Enter => ascii(if ev.ctrl { 10 } else { 13 }),
        Key::Escape => ascii(27),
        Key::Tab => ascii(9),
        Key::Backspace => ascii(if ev.ctrl { 127 } else { 8 }),
        Key::Char(c) if (c as u32) > 0x7F => {
            // WM_CHAR above 0x7F: unicode down+up at once, nothing on release
            if down {
                vec![
                    KeyAction::DownUnicode(c as u32),
                    KeyAction::UpUnicode(c as u32),
                ]
            } else {
                Vec::new()
            }
        }
        Key::Char(c) => ascii(char_code(c, ev.ctrl)),
        Key::Other(vk) => {
            if down {
                Vec::new()
            } else {
                ascii(vk)
            }
        }
    }
}

fn action_out(a: KeyAction, out_code: *mut u32) -> i32 {
    let (k, c) = match a {
        KeyAction::Down(c) => (1, c),
        KeyAction::Up(c) => (2, c),
        KeyAction::DownUnicode(c) => (3, c),
        KeyAction::UpUnicode(c) => (4, c),
    };
    // SAFETY: callers pass a writable pointer (checked non-null).
    if !out_code.is_null() {
        unsafe { out_code.write_unaligned(c) };
    }
    k
}

/// [`glut_special`] for the differential test: returns 0 (no call),
/// 1 (keydown) or 2 (keyup) and writes the code.
///
/// # Safety
/// `out_code` is null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_keymap_glut_special(key: i32, down: i32, out_code: *mut u32) -> i32 {
    glut_special(key, down != 0).map_or(0, |a| action_out(a, out_code))
}

/// [`glut_ascii`] for the differential test; `mods` uses GLUT bits
/// (1 shift, 2 ctrl, 4 alt).
///
/// # Safety
/// `out_code` is null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_keymap_glut_ascii(
    key: i32,
    mods: i32,
    down: i32,
    macos: i32,
    out_code: *mut u32,
) -> i32 {
    action_out(glut_ascii(key, mods & 2 != 0, mods & 1 != 0, down != 0, macos != 0), out_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(key: Key) -> KeyInput {
        KeyInput {
            pressed: true,
            repeat: false,
            key,
            shift: false,
            ctrl: false,
            alt: false,
        }
    }
    fn release(key: Key) -> KeyInput {
        KeyInput {
            pressed: false,
            ..press(key)
        }
    }

    #[test]
    fn special_keys_map_to_bios_codes() {
        assert_eq!(
            glut_special(glut::KEY_F1, true),
            Some(KeyAction::Down(0x3B00))
        );
        assert_eq!(glut_special(10, true), Some(KeyAction::Down(0x4400))); // F10
        assert_eq!(glut_special(11, false), Some(KeyAction::Up(0x8500))); // F11
        assert_eq!(glut_special(12, true), Some(KeyAction::Down(0x8600))); // F12
        assert_eq!(
            glut_special(glut::KEY_LEFT, true),
            Some(KeyAction::Down(0x4B00))
        );
        assert_eq!(
            glut_special(glut::KEY_UP, true),
            Some(KeyAction::Down(0x4800))
        );
        assert_eq!(
            glut_special(glut::KEY_RIGHT, true),
            Some(KeyAction::Down(0x4D00))
        );
        assert_eq!(
            glut_special(glut::KEY_DOWN, true),
            Some(KeyAction::Down(0x5000))
        );
        assert_eq!(
            glut_special(glut::KEY_PAGE_UP, true),
            Some(KeyAction::Down(0x4900))
        );
        assert_eq!(
            glut_special(glut::KEY_PAGE_DOWN, true),
            Some(KeyAction::Down(0x5100))
        );
        assert_eq!(
            glut_special(glut::KEY_HOME, true),
            Some(KeyAction::Down(0x4700))
        );
        assert_eq!(
            glut_special(glut::KEY_END, true),
            Some(KeyAction::Down(0x4F00))
        );
        assert_eq!(
            glut_special(glut::KEY_INSERT, true),
            Some(KeyAction::Down(0x5200))
        );
        assert_eq!(
            glut_special(glut::KEY_SHIFT_L, true),
            Some(KeyAction::Down(VK + qbvk::LSHIFT))
        );
        assert_eq!(
            glut_special(glut::KEY_ALT_R, false),
            Some(KeyAction::Up(VK + qbvk::RALT))
        );
        assert_eq!(glut_special(0, true), None);
        assert_eq!(glut_special(13, true), None);
    }

    #[test]
    fn ascii_ctrl_letters_are_decoded() {
        assert_eq!(
            glut_ascii(65, false, true, true, false),
            KeyAction::Down(65)
        );
        assert_eq!(glut_ascii(1, true, false, true, false), KeyAction::Down(97));
        assert_eq!(glut_ascii(1, true, true, true, false), KeyAction::Down(65));
        assert_eq!(
            glut_ascii(26, true, false, false, false),
            KeyAction::Up(122)
        );
        assert_eq!(
            glut_ascii(10, true, false, true, false),
            KeyAction::Down(13)
        );
        assert_eq!(
            glut_ascii(10, false, false, true, false),
            KeyAction::Down(10)
        );
        assert_eq!(
            glut_ascii(127, false, false, true, false),
            KeyAction::Down(0x5300)
        );
        assert_eq!(
            glut_ascii(8, false, false, true, true),
            KeyAction::Down(0x5300)
        );
        assert_eq!(glut_ascii(127, false, false, false, true), KeyAction::Up(8));
    }

    #[test]
    fn printable_key_press_and_release() {
        assert_eq!(translate(&press(Key::Char('a'))), vec![KeyAction::Down(97)]);
        assert_eq!(translate(&release(Key::Char('a'))), vec![KeyAction::Up(97)]);
        let mut ctrl_a = press(Key::Char('a'));
        ctrl_a.ctrl = true;
        assert_eq!(translate(&ctrl_a), vec![KeyAction::Down(97)]);
    }

    #[test]
    fn non_ascii_text_is_unicode_down_up() {
        assert_eq!(
            translate(&press(Key::Char('\u{e9}'))),
            vec![KeyAction::DownUnicode(0xe9), KeyAction::UpUnicode(0xe9)]
        );
        assert_eq!(translate(&release(Key::Char('\u{e9}'))), vec![]);
    }

    #[test]
    fn named_keys() {
        assert_eq!(translate(&press(Key::Enter)), vec![KeyAction::Down(13)]);
        assert_eq!(translate(&release(Key::Escape)), vec![KeyAction::Up(27)]);
        assert_eq!(
            translate(&press(Key::Delete)),
            vec![KeyAction::Down(0x5300)]
        );
        assert_eq!(translate(&press(Key::F(5))), vec![KeyAction::Down(0x3F00)]);
        assert_eq!(
            translate(&press(Key::Pause)),
            vec![KeyAction::Down(VK + qbvk::PAUSE)]
        );
        assert_eq!(
            translate(&release(Key::Cancel)),
            vec![KeyAction::Up(VK + qbvk::BREAK)]
        );
        let mut ctrl_bs = press(Key::Backspace);
        ctrl_bs.ctrl = true;
        assert_eq!(translate(&ctrl_bs), vec![KeyAction::Down(0x5300)]);
        let mut ctrl_enter = press(Key::Enter);
        ctrl_enter.ctrl = true;
        assert_eq!(translate(&ctrl_enter), vec![KeyAction::Down(13)]);
    }

    #[test]
    fn modifiers_do_not_repeat() {
        assert_eq!(
            translate(&press(Key::LShift)),
            vec![KeyAction::Down(VK + qbvk::LSHIFT)]
        );
        let mut rep = press(Key::LShift);
        rep.repeat = true;
        assert_eq!(translate(&rep), vec![]);
        // other keys repeat
        let mut rep_a = press(Key::Char('a'));
        rep_a.repeat = true;
        assert_eq!(translate(&rep_a), vec![KeyAction::Down(97)]);
    }

    #[test]
    fn keys_without_text_report_vk_on_release_only() {
        // legacy: WM_KEYUP default branch passed (char)wParam when ToAscii failed
        assert_eq!(translate(&press(Key::Other(0x14))), vec![]);
        assert_eq!(
            translate(&release(Key::Other(0x14))),
            vec![KeyAction::Up(0x14)]
        );
    }
}
