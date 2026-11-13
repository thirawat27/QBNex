//! Window, desktop, clipboard, console and keyboard-lock builtins (registry
//! callnames): `_SCREENMOVE`, `_SCREENX`, `_SCREENHIDE`, `_FULLSCREEN`,
//! `_RESIZE`, `_WINDOWHASFOCUS`, `_DROPPEDFILE$`, `_DESKTOPWIDTH`, `_CLIPBOARD$`,
//! `_EXIT`, `_CAPSLOCK`, `_KEYCLEAR`, `_DIR$`, `_SCREENCLICK`, `_CONSOLEINPUT`,
//! `LOCK`, `SHELL _HIDE`, ...
//!
//! A `$CONSOLE:ONLY` program and the browser runner have no window: there the
//! statements do nothing and the functions answer as QB64 does without one.

use super::builtins::Host;
use super::{e, R, V};
use qb_runtime::screen::{LockChange, LockKey};

/// BIOS keyboard buffer in conventional memory: head and tail pointers, and
/// the offset both hold when the buffer is empty.
const KEY_BUFFER_HEAD: usize = 0x41A;
const KEY_BUFFER_TAIL: usize = 0x41C;
const KEY_BUFFER_START: u8 = 30;

/// `_DROPPEDFILE$` without an index reads the dropped files one after the other: the last index read.
#[cfg(not(target_arch = "wasm32"))]
static DROP_INDEX: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);

/// `FRE` answers (bytes): string space, array space and stack of a program that is never short of memory.
const FREE_STRING_SPACE: i32 = 60_000;
const FREE_ARRAY_SPACE: i32 = 16 * 1024 * 1024;
const FREE_STACK: i32 = 30_000;

/// `_FPS` option bits (`passed`).
const FPS_AUTO: i64 = 1;
const FPS_VALUE: i64 = 2;

/// `SHELL _HIDE` / `SHELL _DONTWAIT` option bits (`passed`): the other of the
/// two options was written too, a command was given.
const SHELL_BOTH: i64 = 1;
const SHELL_COMMAND: i64 = 2;

/// `_MOUSESHOW` cursor shape names.
#[cfg(not(target_arch = "wasm32"))]
fn cursor_style(name: &[u8]) -> Option<i32> {
    use qb_runtime::window::cursor as c;
    Some(match name.to_ascii_uppercase().as_slice() {
        b"DEFAULT" => c::DEFAULT,
        b"LINK" => c::LINK,
        b"TEXT" => c::TEXT,
        b"CROSSHAIR" => c::CROSSHAIR,
        b"VERTICAL" => c::VERTICAL,
        b"HORIZONTAL" => c::HORIZONTAL,
        b"TOPLEFT_BOTTOMRIGHT" => c::TOPLEFT_BOTTOMRIGHT,
        b"TOPRIGHT_BOTTOMLEFT" => c::TOPRIGHT_BOTTOMLEFT,
        b"WAIT" => c::WAIT,
        b"HELP" => c::HELP,
        b"CYCLE" | b"MOVE" => c::MOVE,
        _ => return None,
    })
}

/// `SHELL _HIDE` (`hide`) and `SHELL _DONTWAIT` statements: (command, passed).
fn shell_option(vm: &mut dyn Host<'_>, hide: bool, cmd: &[u8], passed: i64) -> R<Option<V>> {
    use qb_runtime::shell::{run, Mode};
    let given = passed & SHELL_COMMAND != 0 && !cmd.is_empty();
    let both = passed & SHELL_BOTH != 0;
    // a hidden shell cannot wait for the user
    if (hide || both) && !given {
        return e(5);
    }
    vm.rt().flush();
    let mode = Mode { show: !(hide || both), wait: hide && !both, use_console: false };
    // SHELL _DONTWAIT alone opens a command prompt
    let prompt: &[u8] = if cfg!(windows) { b"cmd" } else { b"sh" };
    run(if given { cmd } else { prompt }, mode);
    Ok(None)
}

/// `_SCREENMOVE` option bits (`passed`).
const MOVE_MIDDLE: i64 = 1;
const MOVE_TO: i64 = 2;

fn lock_key(name: &str) -> LockKey {
    if name.ends_with("capslock") {
        LockKey::Caps
    } else if name.ends_with("numlock") {
        LockKey::Num
    } else {
        LockKey::Scroll
    }
}

/// True when the program shows a real or headless window (not the browser runner).
fn has_window(vm: &mut dyn Host<'_>) -> bool {
    cfg!(not(target_arch = "wasm32")) && vm.rt().window
}

#[cfg(not(target_arch = "wasm32"))]
mod win {
    use qb_runtime::window as w;

    pub fn position() -> (i32, i32) {
        let (mut x, mut y) = (0, 0);
        // SAFETY: two valid out pointers.
        unsafe { w::qb_window_position(&mut x, &mut y) };
        (x, y)
    }

    pub fn size() -> (i32, i32) {
        let (mut x, mut y) = (0, 0);
        // SAFETY: two valid out pointers.
        unsafe { w::qb_window_size(&mut x, &mut y) };
        (x, y)
    }

    /// The desktop as the window's event loop reports it.
    pub fn desktop() -> (i32, i32) {
        let (mut x, mut y) = (0, 0);
        // SAFETY: two valid out pointers.
        unsafe { w::qb_desktop_size(&mut x, &mut y) };
        (x, y)
    }
}

/// Size of the desktop: from the window's event loop, or measured directly for
/// a console program.
fn desktop(vm: &mut dyn Host<'_>) -> (i32, i32) {
    #[cfg(not(target_arch = "wasm32"))]
    if has_window(vm) {
        return win::desktop();
    }
    let _ = vm;
    qb_runtime::screen::desktop()
}

/// Handles the window and system builtins; `None` for any other name.
pub(crate) fn call(vm: &mut dyn Host<'_>, name: &str, a: &[V]) -> Option<R<Option<V>>> {
    let i = |k: usize| a[k].i();
    let int = |v: i32| Ok(Some(V::I(i64::from(v))));
    let none = Ok(None);
    Some(match name {
        "func__clipboard" => Ok(Some(V::S(qb_runtime::clipboard::get_text()))),
        "sub__clipboard" => {
            qb_runtime::clipboard::set_text(a[0].s());
            none
        }
        "func__environcount" => int(std::env::vars_os().count() as i32),
        "func__dir" => Ok(Some(V::S(qb_runtime::fsops::special_dir(a[0].s())))),
        "func_screenwidth" => int(desktop(vm).0),
        "func_screenheight" => int(desktop(vm).1),
        "func__capslock" | "func__numlock" | "func__scrolllock" => int(qb_runtime::screen::lock_key_state(lock_key(name))),
        "sub__capslock" | "sub__numlock" | "sub__scrolllock" => {
            qb_runtime::screen::set_lock_key(lock_key(name), LockChange::from_option(i(0) as i32));
            none
        }
        // _KEYCLEAR [buffer]: 1 INKEY$, 2 _KEYHIT, 3 INP(&H60); all of them without an argument
        "sub__keyclear" => {
            let (buf, passed) = (i(0), i(1) != 0);
            if passed && !(1..=3).contains(&buf) {
                return Some(e(5));
            }
            let all = !passed;
            let rt = vm.rt();
            // keys that arrived but were not applied yet belong to the buffers being cleared
            rt.input.pump(&mut rt.cmem);
            if all || buf == 1 {
                rt.cmem[KEY_BUFFER_HEAD] = KEY_BUFFER_START;
                rt.cmem[KEY_BUFFER_HEAD + 1] = 0;
                rt.cmem[KEY_BUFFER_TAIL] = KEY_BUFFER_START;
                rt.cmem[KEY_BUFFER_TAIL + 1] = 0;
            }
            if all || buf == 2 {
                qb_runtime::keybuf::qb_keyhit_clear();
            }
            if all || buf == 3 {
                rt.input.clear_port60();
            }
            qb_runtime::console::stdio::flush_input();
            none
        }
        // _EXIT: the close requests since the last call (1 = close box); from the
        // first call on they no longer end the program
        "func__exit" => {
            #[cfg(not(target_arch = "wasm32"))]
            if has_window(vm) {
                return Some(int(super::window::take_exit_request()));
            }
            int(0)
        }
        "func__screenx" | "func__screeny" => {
            #[cfg(not(target_arch = "wasm32"))]
            if has_window(vm) {
                let (x, y) = win::position();
                return Some(int(if name == "func__screenx" { x } else { y }));
            }
            int(0)
        }
        // _SCREENMOVE {x, y | _MIDDLE}
        "sub__screenmove" => {
            let passed = i(2);
            if passed != MOVE_MIDDLE && passed != MOVE_TO {
                return Some(e(5));
            }
            #[cfg(not(target_arch = "wasm32"))]
            if has_window(vm) && super::window::fullscreen().0 == 0 {
                let (x, y) = if passed == MOVE_TO {
                    (i(0) as i32, i(1) as i32)
                } else {
                    let (dw, dh) = win::desktop();
                    let (ww, wh) = win::size();
                    ((dw - ww) / 2, (dh - wh) / 2)
                };
                qb_runtime::window::qb_window_set_position(x, y);
            }
            none
        }
        "func_windowexists" => {
            #[cfg(not(target_arch = "wasm32"))]
            if has_window(vm) {
                return Some(int(-qb_runtime::window::qb_window_exists()));
            }
            int(-1)
        }
        "sub_screenicon" => {
            #[cfg(not(target_arch = "wasm32"))]
            if has_window(vm) {
                qb_runtime::window::qb_window_minimize();
            }
            none
        }
        // _SCREENICON: -1 while the window is minimized (an error for a hidden screen)
        "func_screenicon" => {
            if vm.rt().display.screen_hide {
                return Some(e(5));
            }
            #[cfg(not(target_arch = "wasm32"))]
            if has_window(vm) {
                return Some(int(-qb_runtime::window::qb_window_minimized()));
            }
            int(0)
        }
        "sub__screenhide" | "sub__screenshow" => {
            let hide = name == "sub__screenhide";
            vm.rt().display.screen_hide = hide;
            #[cfg(not(target_arch = "wasm32"))]
            if has_window(vm) {
                qb_runtime::window::qb_window_show(i32::from(!hide));
                if !hide {
                    super::window::present(&vm.rt().display);
                }
            }
            none
        }
        "func__screenhide" => int(-i32::from(vm.rt().display.screen_hide)),
        // _FULLSCREEN [{_OFF|_STRETCH|_SQUAREPIXELS|OFF}][, _SMOOTH]; no option = stretched
        "sub__fullscreen" => {
            #[cfg(not(target_arch = "wasm32"))]
            if has_window(vm) {
                let mode = match i(0) {
                    1 | 4 => 0,
                    3 => 2,
                    _ => 1,
                };
                super::window::set_fullscreen(mode, i(1) & 1 != 0);
            }
            none
        }
        "func__fullscreen" | "func__fullscreensmooth" => {
            #[cfg(not(target_arch = "wasm32"))]
            if has_window(vm) {
                let (mode, smooth) = super::window::fullscreen();
                return Some(int(if name == "func__fullscreen" { mode } else { -i32::from(smooth) }));
            }
            int(0)
        }
        "func__scaledwidth" | "func__scaledheight" => {
            #[cfg(not(target_arch = "wasm32"))]
            if has_window(vm) {
                let (w, h) = super::window::scaled_size();
                return Some(int(if name == "func__scaledwidth" { w } else { h }));
            }
            int(0)
        }
        // _RESIZE [ON|OFF][, _STRETCH|_SMOOTH]: (1 on, 2 off; 1 stretch, 2 smooth)
        "sub__resize" => {
            #[cfg(not(target_arch = "wasm32"))]
            if has_window(vm) {
                let allow = match i(0) {
                    1 => Some(true),
                    2 => Some(false),
                    _ => None,
                };
                super::window::set_resize(allow, i(1) as i32);
            }
            none
        }
        "func__resize" | "func__resizewidth" | "func__resizeheight" => {
            #[cfg(not(target_arch = "wasm32"))]
            if has_window(vm) {
                return Some(int(match name {
                    "func__resize" => -i32::from(super::window::take_resize()),
                    "func__resizewidth" => super::window::resize_size().0,
                    _ => super::window::resize_size().1,
                }));
            }
            int(0)
        }
        "func__hasfocus" => {
            #[cfg(not(target_arch = "wasm32"))]
            if has_window(vm) {
                return Some(int(-i32::from(super::window::focused())));
            }
            int(-1)
        }
        // _WINDOWHANDLE: of the program window, of the console window for a console program
        "func__handle" => {
            #[cfg(not(target_arch = "wasm32"))]
            if has_window(vm) {
                let hidden = vm.rt().display.screen_hide;
                return Some(Ok(Some(V::I(super::window::handle(hidden)))));
            }
            Ok(Some(V::I(qb_runtime::console::stdio::console_window() as i64)))
        }
        // _ACCEPTFILEDROP [ON|OFF] (1, 2; on without an option)
        "sub__filedrop" => {
            #[cfg(not(target_arch = "wasm32"))]
            super::window::accept_drop(i(0) != 2);
            none
        }
        "func__filedrop" => {
            #[cfg(not(target_arch = "wasm32"))]
            if has_window(vm) {
                return Some(int(-i32::from(super::window::accepts_drop())));
            }
            int(0)
        }
        "sub__finishdrop" | "func__totaldroppedfiles" | "func__droppedfile" => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                use std::sync::atomic::Ordering::Relaxed;
                let mut files = super::window::dropped();
                return Some(match name {
                    "sub__finishdrop" => {
                        files.clear();
                        DROP_INDEX.store(-1, Relaxed);
                        none
                    }
                    "func__totaldroppedfiles" => int(files.len() as i32),
                    // _DROPPEDFILE$[(n)]: without n the next file; reading past the
                    // last one that way ends the drop
                    _ => {
                        let (n, given) = (i(0), i(1) != 0);
                        let index = if given { n - 1 } else { i64::from(DROP_INDEX.load(Relaxed)) + 1 };
                        let file = usize::try_from(index).ok().and_then(|k| files.get(k)).cloned();
                        let last = index + 1 >= files.len() as i64;
                        if !given && (file.is_none() || last) {
                            files.clear();
                        }
                        DROP_INDEX.store(if file.is_none() || (!given && last) { -1 } else { index as i32 }, Relaxed);
                        Ok(Some(V::S(file.unwrap_or_default())))
                    }
                });
            }
            #[cfg(target_arch = "wasm32")]
            if name == "func__droppedfile" { Ok(Some(V::S(Vec::new()))) } else if name == "sub__finishdrop" { none } else { int(0) }
        }
        // _FPS n | _AUTO: how often the window shows the screen
        "sub__fps" => {
            let (fps, passed) = (a[0].f(), i(1));
            if passed != FPS_AUTO && passed != FPS_VALUE {
                return Some(e(5));
            }
            if passed == FPS_VALUE && (fps.is_nan() || fps < 1.0) {
                return Some(e(5));
            }
            #[cfg(not(target_arch = "wasm32"))]
            if let Err(c) = super::window::set_fps((passed == FPS_VALUE).then_some(fps)) {
                return Some(e(c));
            }
            none
        }
        // _MOUSESHOW [shape$], _MOUSEHIDE: the pointer over the program window
        "sub__mouseshow" | "sub__mousehide" => {
            #[cfg(not(target_arch = "wasm32"))]
            {
            let style = if name == "sub__mousehide" {
                qb_runtime::window::cursor::HIDDEN
            } else if i(1) == 0 {
                vm.rt().mouse_cursor
            } else {
                match cursor_style(a[0].s()) {
                    Some(s) => s,
                    None => return Some(e(5)),
                }
            };
            if name == "sub__mouseshow" {
                vm.rt().mouse_cursor = style;
            }
            if has_window(vm) && !vm.rt().display.screen_hide {
                qb_runtime::window::qb_window_set_cursor(style);
            }
            }
            none
        }
        // _CONSOLE ON|OFF: the console window of a $CONSOLE program that also has a screen
        "sub__console" => {
            let rt = vm.rt();
            if rt.console_on && !rt.console_only {
                qb_runtime::console::stdio::console_show(i(0) == 1);
            }
            none
        }
        // _ALLOWFULLSCREEN: which modes ALT+ENTER may switch to; the runner has no
        // ALT+ENTER switch, so there is nothing to restrict
        "sub__allowfullscreen" => none,
        "sub_lock" | "sub_unlock" => {
            let err = qb_runtime::fileio::lock(i(0) as i32, i(1), i(2), i(3) as i32, name == "sub_lock");
            if err != 0 { e(err) } else { none }
        }
        "sub__screenclick" => {
            qb_runtime::screen::qb_screenclick(i(0) as i32, i(1) as i32, i(2) as i32, i(3) as i32);
            none
        }
        "sub__screenprint" => {
            let s = a[0].s();
            // SAFETY: a live slice of the given length.
            unsafe { qb_runtime::screen::qb_screenprint(s.as_ptr(), s.len() as i32) };
            none
        }
        // _MAPUNICODE unicode TO ascii: the Unicode character a code page 437 byte stands for
        "sub__mapunicode" | "func__mapunicode" => {
            let set = name == "sub__mapunicode";
            let (unicode, ascii) = if set { (i(0), i(1)) } else { (0, i(0)) };
            if !(0..=65535).contains(&unicode) || !(0..=255).contains(&ascii) {
                return Some(e(5));
            }
            // SAFETY: one element of the table, read and written on the program thread only.
            unsafe {
                let slot = core::ptr::addr_of_mut!(qb_runtime::console::utf8::codepage437_to_unicode16).cast::<u16>().add(ascii as usize);
                if set {
                    slot.write(unicode as u16);
                    none
                } else {
                    int(i32::from(slot.read()))
                }
            }
        }
        "func__cinp" => int(qb_runtime::console::stdio::cinp(i(0) as i32, i(1) as i32)),
        "func__getconsoleinput" => {
            vm.rt().flush();
            int(qb_runtime::console::stdio::console_input())
        }
        "sub__console_cursor" => {
            qb_runtime::console::stdio::console_cursor(i(0) as i32, i(1) as i32, i(2) as i32);
            none
        }
        "sub__consolefont" => {
            if vm.rt().console_on {
                qb_runtime::console::stdio::console_font(a[0].s(), i(1) as i32);
            }
            none
        }
        "sub_shell2" => shell_option(vm, true, a[0].s(), i(1)),
        "sub_shell3" => shell_option(vm, false, a[0].s(), i(1)),
        // SHELL alone: a command prompt the user leaves with EXIT
        "SHELLI" => {
            let rt = vm.rt();
            rt.flush();
            qb_runtime::shell::interactive(rt.console_on || !rt.window);
            none
        }
        // FRE(n): QuickBASIC's free memory. A program only compares it with what it
        // needs, so fixed, generous sizes: string space, array space (-1), stack (-2)
        "func_fre" | "func_setmem" => int(match (name, i(0)) {
            ("func_fre", -2) => FREE_STACK,
            ("func_fre", -1) | ("func_setmem", _) => FREE_ARRAY_SPACE,
            _ => FREE_STRING_SPACE,
        }),
        "func_fileattr" => match qb_runtime::fileio::fileattr(i(0) as i32, i(1) as i32) {
            (v, 0) => int(v),
            (_, err) => e(err),
        },
        // IOCTL sends a control string to a device driver: files have none (error 5
        // for an open file, as QuickBASIC answers for a file that is not a device)
        "func_ioctl" | "sub_ioctl" => match qb_runtime::fileio::fileattr(i(0) as i32, 1) {
            (_, 0) => e(5),
            (_, err) => e(err),
        },
        // TRON / TROFF print line numbers in the QuickBASIC editor only
        "sub_tron" | "sub_troff" => none,
        // light pen, serial port and user events never happen here
        "func_pen" => {
            if (0..=9).contains(&i(0)) { int(0) } else { e(5) }
        }
        "sub_pen" | "sub_uevent" => none,
        "sub_com" => {
            if (1..=2).contains(&i(0)) { none } else { e(5) }
        }
        // _CONSOLETITLE: the title of the console window of a $CONSOLE program
        "sub__consoletitle" => {
            if vm.rt().console_on {
                qb_runtime::console::stdio::console_title(a[0].s());
            }
            none
        }
        _ => return None,
    })
}
