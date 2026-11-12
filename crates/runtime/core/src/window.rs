//! The program window: winit event loop on the main thread, software frames
//! presented with softbuffer (or, with the `gpu` feature, through wgpu when an
//! adapter is available; see `gpu_present`). Replaces freeglut + GLEW + the GL
//! half of `libqb/gui.cpp`.
//!
//! Thread model:
//! * main thread: [`qb_window_run`] owns the event loop and the window;
//! * any thread: `qb_window_*` setters queue a [`Cmd`] and wake the loop;
//!   getters read atomics updated by the loop;
//! * frames: the thread that builds a display frame calls
//!   [`qb_window_present`]; the loop scales the latest one into the window.
//!
//! OS input is translated (see `keymap`) and delivered through the C
//! callbacks in [`Callbacks`]; libqb.cpp guards its key/mouse state with
//! `qb_input_lock`.
//!
//! `QBNEX_HEADLESS=1` runs without a window (CI). `QBNEX_INPUT_SCRIPT=<file>`
//! replays scripted input (see `script`).

use std::ffi::{c_char, c_void, CStr, CString};
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{
    DeviceEvent, DeviceId, ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent,
};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key as WKey, KeyCode, ModifiersState, NamedKey, PhysicalKey};
use winit::window::{CursorIcon, Fullscreen, Icon, Window, WindowId};

use crate::keymap::{self, Key, KeyAction, KeyInput};
use crate::raster::{self, Source, Target};
use crate::script;

/// C callbacks into libqb.cpp. Any entry may be null.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Callbacks {
    /// action: 1 keydown, 2 keyup, 3 keydown_unicode, 4 keyup_unicode.
    pub key: Option<extern "C" fn(i32, u32)>,
    /// GLUT button number (0 left, 1 middle, 2 right), 1 = down, window x/y.
    pub mouse_button: Option<extern "C" fn(i32, i32, i32, i32)>,
    pub mouse_move: Option<extern "C" fn(i32, i32)>,
    /// GLUT wheel direction (+1 forward/up, -1 back/down), window x/y.
    pub mouse_wheel: Option<extern "C" fn(i32, i32, i32)>,
    pub mouse_relative: Option<extern "C" fn(i32, i32)>,
    /// New client size after an OS or program resize.
    pub resize: Option<extern "C" fn(i32, i32)>,
    pub close: Option<extern "C" fn()>,
    pub focus: Option<extern "C" fn(i32)>,
    /// UTF-8 path of a file dropped on the window.
    pub file_drop: Option<extern "C" fn(*const c_char)>,
    /// Native window handle (HWND on Windows, else null) once created.
    pub created: Option<extern "C" fn(*mut c_void)>,
}

static CALLBACKS: OnceLock<Callbacks> = OnceLock::new();

fn cb() -> Callbacks {
    *CALLBACKS.get().unwrap_or(&Callbacks {
        key: None,
        mouse_button: None,
        mouse_move: None,
        mouse_wheel: None,
        mouse_relative: None,
        resize: None,
        close: None,
        focus: None,
        file_drop: None,
        created: None,
    })
}

/// Cursor shapes for [`qb_window_set_cursor`] (`_MOUSESHOW` styles).
pub mod cursor {
    pub const HIDDEN: i32 = 0;
    pub const DEFAULT: i32 = 1;
    pub const LINK: i32 = 2;
    pub const TEXT: i32 = 3;
    pub const CROSSHAIR: i32 = 4;
    pub const VERTICAL: i32 = 5;
    pub const HORIZONTAL: i32 = 6;
    pub const TOPLEFT_BOTTOMRIGHT: i32 = 7;
    pub const TOPRIGHT_BOTTOMLEFT: i32 = 8;
    pub const WAIT: i32 = 9;
    pub const HELP: i32 = 10;
    pub const MOVE: i32 = 11;
}

enum Cmd {
    Create,
    Title(String),
    Size(i32, i32),
    Fullscreen(bool),
    Position(i32, i32),
    Visible(bool),
    Minimize,
    Cursor(i32),
    Warp(i32, i32),
    Icon(Vec<u32>, u32, u32),
}

struct Frame {
    px: Vec<u32>,
    w: usize,
    h: usize,
    full_screen: i32,
    fullscreen_smooth: bool,
    resize_auto: i32,
    serial: u64,
}

static FRAME: Mutex<Frame> = Mutex::new(Frame {
    px: Vec::new(),
    w: 0,
    h: 0,
    full_screen: 0,
    fullscreen_smooth: false,
    resize_auto: 0,
    serial: 0,
});
static CMDS: Mutex<Vec<Cmd>> = Mutex::new(Vec::new());
static PROXY: Mutex<Option<EventLoopProxy<()>>> = Mutex::new(None);

static EXISTS: AtomicBool = AtomicBool::new(false);
static HEADLESS: AtomicBool = AtomicBool::new(false);
static WIN_W: AtomicI32 = AtomicI32::new(640);
static WIN_H: AtomicI32 = AtomicI32::new(400);
static POS_X: AtomicI32 = AtomicI32::new(0);
static POS_Y: AtomicI32 = AtomicI32::new(0);
static DESK_W: AtomicI32 = AtomicI32::new(0);
static DESK_H: AtomicI32 = AtomicI32::new(0);
static MINIMIZED: AtomicBool = AtomicBool::new(false);

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn push(cmd: Cmd) {
    lock(&CMDS).push(cmd);
    wake();
}

fn wake() {
    if let Some(p) = lock(&PROXY).as_ref() {
        let _ = p.send_event(());
    }
}

fn headless_requested() -> bool {
    std::env::var("QBNEX_HEADLESS")
        .map(|v| !v.is_empty() && v != "0")
        .unwrap_or(false)
}

/// FNV-1a over the frame pixels (for `snapshot` in input scripts).
fn frame_hash(px: &[u32]) -> u32 {
    let mut h: u32 = 0x811C_9DC5;
    for p in px {
        for b in p.to_le_bytes() {
            h ^= b as u32;
            h = h.wrapping_mul(0x0100_0193);
        }
    }
    h
}

// ---- key translation from winit ----------------------------------------------

fn named(k: &NamedKey) -> Option<Key> {
    Some(match k {
        NamedKey::F1 => Key::F(1),
        NamedKey::F2 => Key::F(2),
        NamedKey::F3 => Key::F(3),
        NamedKey::F4 => Key::F(4),
        NamedKey::F5 => Key::F(5),
        NamedKey::F6 => Key::F(6),
        NamedKey::F7 => Key::F(7),
        NamedKey::F8 => Key::F(8),
        NamedKey::F9 => Key::F(9),
        NamedKey::F10 => Key::F(10),
        NamedKey::F11 => Key::F(11),
        NamedKey::F12 => Key::F(12),
        NamedKey::PageUp => Key::PageUp,
        NamedKey::PageDown => Key::PageDown,
        NamedKey::Home => Key::Home,
        NamedKey::End => Key::End,
        NamedKey::ArrowLeft => Key::Left,
        NamedKey::ArrowUp => Key::Up,
        NamedKey::ArrowRight => Key::Right,
        NamedKey::ArrowDown => Key::Down,
        NamedKey::Insert => Key::Insert,
        NamedKey::Delete => Key::Delete,
        NamedKey::Pause => Key::Pause,
        NamedKey::Cancel => Key::Cancel,
        NamedKey::Enter => Key::Enter,
        NamedKey::Escape => Key::Escape,
        NamedKey::Tab => Key::Tab,
        NamedKey::Backspace => Key::Backspace,
        NamedKey::Space => Key::Char(' '),
        NamedKey::CapsLock => Key::Other(0x14),
        NamedKey::NumLock => Key::Other(0x90),
        NamedKey::ScrollLock => Key::Other(0x91),
        NamedKey::Super => Key::Other(0x5B),
        NamedKey::ContextMenu => Key::Other(0x5D),
        NamedKey::PrintScreen => Key::Other(0x2C),
        _ => return None,
    })
}

fn to_key(ev: &KeyEvent) -> Option<Key> {
    // modifiers by physical key, so left/right are distinguished
    if let PhysicalKey::Code(code) = ev.physical_key {
        match code {
            KeyCode::ShiftLeft => return Some(Key::LShift),
            KeyCode::ShiftRight => return Some(Key::RShift),
            KeyCode::ControlLeft => return Some(Key::LCtrl),
            KeyCode::ControlRight => return Some(Key::RCtrl),
            KeyCode::AltLeft => return Some(Key::LAlt),
            KeyCode::AltRight => return Some(Key::RAlt),
            _ => {}
        }
    }
    match &ev.logical_key {
        WKey::Named(n) => named(n),
        WKey::Character(s) => s.chars().next().map(Key::Char),
        _ => None,
    }
}

fn deliver(actions: &[KeyAction]) {
    if let Some(f) = cb().key {
        for a in actions {
            match *a {
                KeyAction::Down(x) => f(1, x),
                KeyAction::Up(x) => f(2, x),
                KeyAction::DownUnicode(x) => f(3, x),
                KeyAction::UpUnicode(x) => f(4, x),
            }
        }
    }
}

// ---- event loop --------------------------------------------------------------

/// How frames reach the window.
enum Presenter {
    Soft(softbuffer::Surface<Arc<Window>, Arc<Window>>),
    #[cfg(feature = "gpu")]
    Gpu(Box<crate::gpu_present::Present>),
}

struct Win {
    window: Arc<Window>,
    presenter: Presenter,
    last_serial: u64,
}

/// The GPU presenter when the feature is on and an adapter/surface works, else softbuffer.
fn presenter(window: &Arc<Window>) -> Option<Presenter> {
    #[cfg(feature = "gpu")]
    match crate::gpu_present::Present::new(window.clone()) {
        Ok(p) => {
            if std::env::var("QB_GPU").is_ok_and(|v| v == "verbose") {
                eprintln!("qb-runtime: presenting through the GPU ({})", p.adapter_name());
            }
            return Some(Presenter::Gpu(Box::new(p)));
        }
        Err(e) => {
            if std::env::var("QB_GPU").is_ok_and(|v| v == "verbose") {
                eprintln!("qb-runtime: {e}; using the software presenter");
            }
        }
    }
    let context = match softbuffer::Context::new(window.clone()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("qb-runtime: softbuffer context: {e}");
            return None;
        }
    };
    match softbuffer::Surface::new(&context, window.clone()) {
        Ok(s) => Some(Presenter::Soft(s)),
        Err(e) => {
            eprintln!("qb-runtime: softbuffer surface: {e}");
            None
        }
    }
}

struct App {
    win: Option<Win>,
    title: String,
    init_w: u32,
    init_h: u32,
    resizable: bool,
    start_hidden: bool,
    hidden: bool,
    mods: ModifiersState,
    cursor: (i32, i32),
    wheel_accum: f64,
    focused: bool,
    fullscreen: bool,
}

impl App {
    fn create(&mut self, el: &ActiveEventLoop) {
        if self.win.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title(self.title.clone())
            .with_inner_size(PhysicalSize::new(self.init_w, self.init_h))
            .with_resizable(self.resizable)
            .with_visible(!self.hidden);
        let window = match el.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                eprintln!("qb-runtime: cannot create window: {e}");
                return;
            }
        };
        let Some(presenter) = presenter(&window) else { return };
        let size = window.inner_size();
        WIN_W.store(size.width as i32, Ordering::SeqCst);
        WIN_H.store(size.height as i32, Ordering::SeqCst);
        if let Ok(p) = window.outer_position() {
            POS_X.store(p.x, Ordering::SeqCst);
            POS_Y.store(p.y, Ordering::SeqCst);
        }
        let handle = native_handle(&window);
        crate::gl::set_window(window.clone());
        self.win = Some(Win {
            window,
            presenter,
            last_serial: u64::MAX,
        });
        EXISTS.store(true, Ordering::SeqCst);
        if let Some(f) = cb().created {
            f(handle);
        }
        if let Some(f) = cb().resize {
            f(size.width as i32, size.height as i32);
        }
        start_script();
    }

    fn run_cmds(&mut self, el: &ActiveEventLoop) {
        let cmds: Vec<Cmd> = std::mem::take(&mut *lock(&CMDS));
        for c in cmds {
            if let Cmd::Create = c {
                self.create(el);
                continue;
            }
            match c {
                Cmd::Title(t) => {
                    if let Some(w) = &self.win {
                        w.window.set_title(&t);
                    }
                    self.title = t;
                }
                Cmd::Visible(v) => {
                    self.hidden = !v;
                    if v && self.win.is_none() {
                        self.create(el);
                    }
                    if let Some(w) = &self.win {
                        w.window.set_visible(v);
                    }
                }
                _ => {
                    let Some(w) = &self.win else { continue };
                    let window = &w.window;
                    match c {
                        Cmd::Size(x, y) if x > 0 && y > 0 => {
                            if let Some(s) =
                                window.request_inner_size(PhysicalSize::new(x as u32, y as u32))
                            {
                                self.on_resize(s.width, s.height);
                            }
                        }
                        Cmd::Fullscreen(on) => {
                            self.fullscreen = on;
                            window.set_fullscreen(on.then_some(Fullscreen::Borderless(None)));
                        }
                        Cmd::Position(x, y) => {
                            window.set_outer_position(PhysicalPosition::new(x, y));
                            POS_X.store(x, Ordering::SeqCst);
                            POS_Y.store(y, Ordering::SeqCst);
                        }
                        Cmd::Minimize => {
                            window.set_minimized(true);
                            MINIMIZED.store(true, Ordering::SeqCst);
                        }
                        Cmd::Cursor(style) => {
                            if style == cursor::HIDDEN {
                                window.set_cursor_visible(false);
                            } else {
                                window.set_cursor_visible(true);
                                window.set_cursor(cursor_icon(style));
                            }
                        }
                        Cmd::Warp(x, y) => {
                            let _ = window.set_cursor_position(PhysicalPosition::new(x, y));
                        }
                        Cmd::Icon(px, iw, ih) => {
                            let rgba: Vec<u8> = px
                                .iter()
                                .flat_map(|p| {
                                    let [b, g, r, a] = p.to_le_bytes();
                                    [r, g, b, a]
                                })
                                .collect();
                            if let Ok(icon) = Icon::from_rgba(rgba, iw, ih) {
                                window.set_window_icon(Some(icon));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        // new frame?
        if let Some(w) = &self.win
            && lock(&FRAME).serial != w.last_serial {
                w.window.request_redraw();
            }
    }

    fn on_resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 {
            return; // minimised
        }
        let changed = (WIN_W.swap(w as i32, Ordering::SeqCst) != w as i32)
            | (WIN_H.swap(h as i32, Ordering::SeqCst) != h as i32);
        if changed
            && let Some(f) = cb().resize {
                f(w as i32, h as i32);
            }
        if let Some(win) = &mut self.win {
            win.last_serial = u64::MAX;
            win.window.request_redraw();
        }
    }

    fn redraw(&mut self) {
        let Some(win) = &mut self.win else { return };
        if crate::gl::presenting() {
            // SUB _GL: the program thread draws and swaps through OpenGL (crate::gl)
            return;
        }
        let size = win.window.inner_size();
        let (Some(nw), Some(nh)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        else {
            return;
        };
        // one arm without the `gpu` feature, two with it
        #[allow(clippy::infallible_destructuring_match)]
        let surface = match &mut win.presenter {
            Presenter::Soft(s) => s,
            #[cfg(feature = "gpu")]
            Presenter::Gpu(p) => {
                let frame = lock(&FRAME);
                let l = raster::layout(
                    frame.w as i32,
                    frame.h as i32,
                    size.width as i32,
                    size.height as i32,
                    frame.full_screen,
                    frame.fullscreen_smooth,
                    frame.resize_auto,
                );
                if p.redraw(nw.get(), nh.get(), &frame.px, frame.w, frame.h, &l) {
                    win.last_serial = frame.serial;
                }
                return;
            }
        };
        if surface.resize(nw, nh).is_err() {
            return;
        }
        let Ok(mut buffer) = surface.buffer_mut() else {
            return;
        };
        let frame = lock(&FRAME);
        let l = raster::layout(
            frame.w as i32,
            frame.h as i32,
            size.width as i32,
            size.height as i32,
            frame.full_screen,
            frame.fullscreen_smooth,
            frame.resize_auto,
        );
        raster::scale_to_window(
            &mut Target {
                px: &mut buffer,
                w: size.width as usize,
                h: size.height as usize,
            },
            Source {
                px: &frame.px,
                w: frame.w,
                h: frame.h,
            },
            &l,
        );
        win.last_serial = frame.serial;
        drop(frame);
        let _ = buffer.present();
    }
}

fn cursor_icon(style: i32) -> CursorIcon {
    match style {
        cursor::LINK => CursorIcon::Pointer,
        cursor::TEXT => CursorIcon::Text,
        cursor::CROSSHAIR => CursorIcon::Crosshair,
        cursor::VERTICAL => CursorIcon::NsResize,
        cursor::HORIZONTAL => CursorIcon::EwResize,
        cursor::TOPLEFT_BOTTOMRIGHT => CursorIcon::NwseResize,
        cursor::TOPRIGHT_BOTTOMLEFT => CursorIcon::NeswResize,
        cursor::WAIT => CursorIcon::Wait,
        cursor::HELP => CursorIcon::Help,
        cursor::MOVE => CursorIcon::Move,
        _ => CursorIcon::Default,
    }
}

#[cfg(windows)]
fn native_handle(w: &Window) -> *mut c_void {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match w.window_handle().map(|h| h.as_raw()) {
        Ok(RawWindowHandle::Win32(h)) => h.hwnd.get() as *mut c_void,
        _ => std::ptr::null_mut(),
    }
}

#[cfg(not(windows))]
fn native_handle(_w: &Window) -> *mut c_void {
    std::ptr::null_mut()
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if let Some(m) = el
            .primary_monitor()
            .or_else(|| el.available_monitors().next())
        {
            let s = m.size();
            DESK_W.store(s.width as i32, Ordering::SeqCst);
            DESK_H.store(s.height as i32, Ordering::SeqCst);
        }
        if !self.start_hidden {
            self.create(el);
        }
        self.run_cmds(el);
    }

    fn user_event(&mut self, el: &ActiveEventLoop, _: ()) {
        self.run_cmds(el);
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        self.run_cmds(el);
        if let Some(w) = &self.win {
            MINIMIZED.store(w.window.is_minimized().unwrap_or(false), Ordering::SeqCst);
        }
        el.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(16),
        ));
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event
            && self.focused
                && let Some(f) = cb().mouse_relative {
                    f(delta.0 as i32, delta.1 as i32);
                }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                if let Some(f) = cb().close {
                    f();
                }
            }
            WindowEvent::Resized(s) => self.on_resize(s.width, s.height),
            WindowEvent::Moved(p) => {
                POS_X.store(p.x, Ordering::SeqCst);
                POS_Y.store(p.y, Ordering::SeqCst);
            }
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::Focused(f) => {
                self.focused = f;
                if let Some(cbf) = cb().focus {
                    cbf(f as i32);
                }
            }
            WindowEvent::ModifiersChanged(m) => self.mods = m.state(),
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => {
                let pressed = event.state == ElementState::Pressed;
                if is_synthetic && pressed {
                    return;
                }
                if let Some(key) = to_key(&event) {
                    let input = KeyInput {
                        pressed,
                        repeat: event.repeat,
                        key,
                        shift: self.mods.shift_key(),
                        ctrl: self.mods.control_key(),
                        alt: self.mods.alt_key(),
                    };
                    deliver(&keymap::translate(&input));
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as i32, position.y as i32);
                if let Some(f) = cb().mouse_move {
                    f(self.cursor.0, self.cursor.1);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let b = match button {
                    MouseButton::Left => 0,
                    MouseButton::Middle => 1,
                    MouseButton::Right => 2,
                    _ => return,
                };
                if let Some(f) = cb().mouse_button {
                    f(
                        b,
                        (state == ElementState::Pressed) as i32,
                        self.cursor.0,
                        self.cursor.1,
                    );
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // freeglut counted WHEEL_DELTA (120) units per notch
                self.wheel_accum += match delta {
                    MouseScrollDelta::LineDelta(_, y) => y as f64 * 120.0,
                    MouseScrollDelta::PixelDelta(p) => p.y,
                };
                while self.wheel_accum.abs() >= 120.0 {
                    let dir = if self.wheel_accum > 0.0 { 1 } else { -1 };
                    if let Some(f) = cb().mouse_wheel {
                        f(dir, self.cursor.0, self.cursor.1);
                    }
                    self.wheel_accum -= 120.0 * dir as f64;
                }
            }
            WindowEvent::DroppedFile(path) => {
                if let (Some(f), Ok(c)) = (
                    cb().file_drop,
                    CString::new(path.to_string_lossy().as_bytes()),
                ) {
                    f(c.as_ptr());
                }
            }
            _ => {}
        }
        let _ = el;
    }
}

// ---- input script ------------------------------------------------------------

static SCRIPT_STARTED: AtomicBool = AtomicBool::new(false);

fn start_script() {
    let Ok(path) = std::env::var("QBNEX_INPUT_SCRIPT") else {
        return;
    };
    if path.is_empty() || SCRIPT_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    let cmds = match std::fs::read_to_string(&path)
        .map_err(|e| e.to_string())
        .and_then(|t| script::parse(&t))
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("qb-runtime: QBNEX_INPUT_SCRIPT {path}: {e}");
            return;
        }
    };
    std::thread::spawn(move || run_script(cmds));
}

fn run_script(cmds: Vec<script::Cmd>) {
    use script::Cmd as S;
    let (mut shift, mut ctrl, mut alt) = (false, false, false);
    let mut pos = (0, 0);
    for c in cmds {
        match c {
            S::Wait(ms) => std::thread::sleep(Duration::from_millis(ms)),
            S::KeyDown(k) | S::KeyUp(k) => {
                let pressed = matches!(c, S::KeyDown(_));
                match k {
                    Key::LShift | Key::RShift => shift = pressed,
                    Key::LCtrl | Key::RCtrl => ctrl = pressed,
                    Key::LAlt | Key::RAlt => alt = pressed,
                    _ => {}
                }
                let input = KeyInput {
                    pressed,
                    repeat: false,
                    key: k,
                    shift,
                    ctrl,
                    alt,
                };
                deliver(&keymap::translate(&input));
            }
            S::MouseMove(x, y) => {
                pos = (x, y);
                if let Some(f) = cb().mouse_move {
                    f(x, y);
                }
            }
            S::MouseDown(b) | S::MouseUp(b) => {
                let glut = match b {
                    1 => 0,
                    2 => 2,
                    3 => 1,
                    _ => continue,
                };
                if let Some(f) = cb().mouse_button {
                    f(glut, matches!(c, S::MouseDown(_)) as i32, pos.0, pos.1);
                }
            }
            S::Wheel(n) => {
                if let Some(f) = cb().mouse_wheel {
                    for _ in 0..n.unsigned_abs() {
                        f(if n < 0 { 1 } else { -1 }, pos.0, pos.1);
                    }
                }
            }
            S::Resize(w, h) => {
                if HEADLESS.load(Ordering::SeqCst) {
                    WIN_W.store(w, Ordering::SeqCst);
                    WIN_H.store(h, Ordering::SeqCst);
                    if let Some(f) = cb().resize {
                        f(w, h);
                    }
                } else {
                    push(Cmd::Size(w, h));
                }
            }
            S::Close => {
                if let Some(f) = cb().close {
                    f();
                }
            }
            S::Snapshot => {
                let f = lock(&FRAME);
                use std::io::Write;
                let mut out = std::io::stdout().lock();
                let _ = writeln!(
                    out,
                    "[frame {}x{} fnv1a={:08x}]",
                    f.w,
                    f.h,
                    frame_hash(&f.px)
                );
                let _ = out.flush();
            }
        }
    }
}

// ---- C ABI -------------------------------------------------------------------

/// Runs the window system on the calling (main) thread. Never returns in
/// windowed mode unless the event loop cannot start; the program ends with
/// `exit()` from another thread. Returns 1 when it fell back to headless.
///
/// # Safety
/// `callbacks` must point to a valid struct; `title` is null or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_window_run(
    callbacks: *const Callbacks,
    width: i32,
    height: i32,
    title: *const c_char,
    resizable: i32,
    start_hidden: i32,
) -> i32 {
    if !callbacks.is_null() {
        // SAFETY: caller passes a valid struct.
        let _ = CALLBACKS.set(unsafe { *callbacks });
    }
    let title = if title.is_null() {
        "Untitled".to_string()
    } else {
        // SAFETY: caller passes a NUL-terminated string.
        unsafe { CStr::from_ptr(title) }
            .to_string_lossy()
            .into_owned()
    };
    WIN_W.store(width, Ordering::SeqCst);
    WIN_H.store(height, Ordering::SeqCst);
    let event_loop = if headless_requested() {
        None
    } else {
        EventLoop::new().ok()
    };
    let Some(event_loop) = event_loop else {
        return run_headless();
    };
    *lock(&PROXY) = Some(event_loop.create_proxy());
    let mut app = App {
        win: None,
        title,
        init_w: width.max(1) as u32,
        init_h: height.max(1) as u32,
        resizable: resizable != 0,
        start_hidden: start_hidden != 0,
        hidden: start_hidden != 0,
        mods: ModifiersState::empty(),
        cursor: (0, 0),
        wheel_accum: 0.0,
        focused: false,
        fullscreen: false,
    };
    let _ = event_loop.run_app(&mut app);
    0
}

fn run_headless() -> i32 {
    HEADLESS.store(true, Ordering::SeqCst);
    DESK_W.store(1920, Ordering::SeqCst);
    DESK_H.store(1080, Ordering::SeqCst);
    EXISTS.store(true, Ordering::SeqCst);
    if let Some(f) = cb().created {
        f(std::ptr::null_mut());
    }
    start_script();
    loop {
        // commands have no effect without a window, except sizes
        for c in std::mem::take(&mut *lock(&CMDS)) {
            match c {
                Cmd::Size(w, h) if w > 0 && h > 0 => {
                    let changed = (WIN_W.swap(w, Ordering::SeqCst) != w)
                        | (WIN_H.swap(h, Ordering::SeqCst) != h);
                    if changed
                        && let Some(f) = cb().resize {
                            f(w, h);
                        }
                }
                Cmd::Position(x, y) => {
                    POS_X.store(x, Ordering::SeqCst);
                    POS_Y.store(y, Ordering::SeqCst);
                }
                Cmd::Minimize => MINIMIZED.store(true, Ordering::SeqCst),
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(16));
    }
}

/// 1 when running without a window (`QBNEX_HEADLESS` or no display).
#[unsafe(no_mangle)]
pub extern "C" fn qb_window_headless() -> i32 {
    (HEADLESS.load(Ordering::SeqCst) || headless_requested()) as i32
}

/// 1 once the window (or the headless stand-in) exists.
#[unsafe(no_mangle)]
pub extern "C" fn qb_window_exists() -> i32 {
    EXISTS.load(Ordering::SeqCst) as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_window_create() {
    push(Cmd::Create);
}

/// Hands a composed frame (`0xAARRGGBB`, `w*h`) to the window. The data is copied.
///
/// # Safety
/// `px` must point to `w*h` readable `u32`s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_window_present(
    px: *const u32,
    w: i32,
    h: i32,
    full_screen: i32,
    fullscreen_smooth: i32,
    resize_auto: i32,
) {
    if px.is_null() || w <= 0 || h <= 0 {
        return;
    }
    let n = w as usize * h as usize;
    {
        let mut f = lock(&FRAME);
        f.px.resize(n, 0);
        // SAFETY: caller guarantees n readable pixels.
        unsafe { std::ptr::copy_nonoverlapping(px, f.px.as_mut_ptr(), n) };
        f.w = w as usize;
        f.h = h as usize;
        f.full_screen = full_screen;
        f.fullscreen_smooth = fullscreen_smooth != 0;
        f.resize_auto = resize_auto;
        f.serial = f.serial.wrapping_add(1);
    }
    if HEADLESS.load(Ordering::SeqCst) {
        // the headless "window" always matches the frame
        let (ow, oh) = (
            WIN_W.swap(w, Ordering::SeqCst),
            WIN_H.swap(h, Ordering::SeqCst),
        );
        if (ow, oh) != (w, h)
            && let Some(f) = cb().resize {
                f(w, h);
            }
    } else {
        wake();
    }
}

/// # Safety
/// `title` is null or NUL-terminated (bytes are taken as UTF-8, invalid bytes as CP-1252-ish lossy).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_window_set_title(title: *const c_char) {
    let t = if title.is_null() {
        String::new()
    } else {
        // SAFETY: caller passes a NUL-terminated string.
        let bytes = unsafe { CStr::from_ptr(title) }.to_bytes();
        match std::str::from_utf8(bytes) {
            Ok(s) => s.to_string(),
            Err(_) => bytes.iter().map(|&b| b as char).collect(),
        }
    };
    push(Cmd::Title(t));
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_window_set_size(w: i32, h: i32) {
    push(Cmd::Size(w, h));
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_window_set_fullscreen(on: i32) {
    push(Cmd::Fullscreen(on != 0));
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_window_set_position(x: i32, y: i32) {
    push(Cmd::Position(x, y));
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_window_show(visible: i32) {
    push(Cmd::Visible(visible != 0));
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_window_minimize() {
    push(Cmd::Minimize);
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_window_minimized() -> i32 {
    MINIMIZED.load(Ordering::SeqCst) as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_window_set_cursor(style: i32) {
    push(Cmd::Cursor(style));
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_window_warp_mouse(x: i32, y: i32) {
    push(Cmd::Warp(x, y));
}

/// # Safety
/// `px` must point to `w*h` readable `u32`s (`0xAARRGGBB`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_window_set_icon(px: *const u32, w: i32, h: i32) {
    if px.is_null() || w <= 0 || h <= 0 {
        return;
    }
    // SAFETY: caller guarantees w*h readable pixels.
    let v = unsafe { std::slice::from_raw_parts(px, w as usize * h as usize) }.to_vec();
    push(Cmd::Icon(v, w as u32, h as u32));
}

/// # Safety
/// `w`/`h` must be valid for writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_window_size(w: *mut i32, h: *mut i32) {
    // SAFETY: caller passes writable pointers.
    unsafe {
        *w = WIN_W.load(Ordering::SeqCst);
        *h = WIN_H.load(Ordering::SeqCst);
    }
}

/// # Safety
/// `x`/`y` must be valid for writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_window_position(x: *mut i32, y: *mut i32) {
    // SAFETY: caller passes writable pointers.
    unsafe {
        *x = POS_X.load(Ordering::SeqCst);
        *y = POS_Y.load(Ordering::SeqCst);
    }
}

/// Desktop size; waits up to 2 s for the event loop to report it.
///
/// # Safety
/// `w`/`h` must be valid for writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_desktop_size(w: *mut i32, h: *mut i32) {
    let start = Instant::now();
    while DESK_W.load(Ordering::SeqCst) == 0 && start.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(5));
    }
    // SAFETY: caller passes writable pointers.
    unsafe {
        *w = DESK_W.load(Ordering::SeqCst);
        *h = DESK_H.load(Ordering::SeqCst);
    }
}

/// `layout()` for libqb.cpp (`_SCALEDWIDTH`, mouse coordinate mapping).
/// `out` receives x1,y1,x2,y2,scaled_w,scaled_h,smooth,letterbox; `scale` x/y.
///
/// # Safety
/// `out` must hold 8 `i32`s and `scale` 2 `f32`s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_layout(
    screen_w: i32,
    screen_h: i32,
    window_w: i32,
    window_h: i32,
    full_screen: i32,
    fullscreen_smooth: i32,
    resize_auto: i32,
    out: *mut i32,
    scale: *mut f32,
) {
    let l = raster::layout(
        screen_w,
        screen_h,
        window_w,
        window_h,
        full_screen,
        fullscreen_smooth != 0,
        resize_auto,
    );
    let v = [
        l.x1,
        l.y1,
        l.x2,
        l.y2,
        l.scaled_w,
        l.scaled_h,
        l.smooth as i32,
        l.letterbox,
    ];
    // SAFETY: caller provides the documented buffers.
    unsafe {
        std::ptr::copy_nonoverlapping(v.as_ptr(), out, 8);
        *scale = l.x_scale;
        *scale.add(1) = l.y_scale;
    }
}

// ---- hardware images (software) ----------------------------------------------

/// `hardware_img_put` in software.
///
/// # Safety
/// `dst` holds `dw*dh` pixels, `src` holds `sw*sh` pixels.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_hw_put(
    dst: *mut u32,
    dw: i32,
    dh: i32,
    src: *const u32,
    sw: i32,
    sh: i32,
    d: *const i32,
    s: *const i32,
    use_alpha: i32,
    smooth: i32,
) {
    if dst.is_null() || src.is_null() || dw <= 0 || dh <= 0 || sw <= 0 || sh <= 0 {
        return;
    }
    // SAFETY: sizes as documented by the caller.
    let (dp, sp, d4, s4) = unsafe {
        (
            std::slice::from_raw_parts_mut(dst, (dw * dh) as usize),
            std::slice::from_raw_parts(src, (sw * sh) as usize),
            *(d as *const [i32; 4]),
            *(s as *const [i32; 4]),
        )
    };
    raster::put(
        &mut Target {
            px: dp,
            w: dw as usize,
            h: dh as usize,
        },
        Source {
            px: sp,
            w: sw as usize,
            h: sh as usize,
        },
        d4,
        s4,
        use_alpha != 0,
        smooth != 0,
    );
}

/// `hardware_img_tri2d` in software. `d`/`s` hold x1,y1,x2,y2,x3,y3.
///
/// # Safety
/// As [`qb_hw_put`]; `d` and `s` hold 6 floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_hw_tri2d(
    dst: *mut u32,
    dw: i32,
    dh: i32,
    src: *const u32,
    sw: i32,
    sh: i32,
    d: *const f32,
    s: *const f32,
    use_alpha: i32,
    smooth: i32,
) {
    if dst.is_null() || src.is_null() || dw <= 0 || dh <= 0 || sw <= 0 || sh <= 0 {
        return;
    }
    // SAFETY: sizes as documented by the caller.
    let (dp, sp, d6, s6) = unsafe {
        (
            std::slice::from_raw_parts_mut(dst, (dw * dh) as usize),
            std::slice::from_raw_parts(src, (sw * sh) as usize),
            *(d as *const [f32; 6]),
            *(s as *const [f32; 6]),
        )
    };
    let p = |a: [f32; 6]| [(a[0], a[1]), (a[2], a[3]), (a[4], a[5])];
    raster::tri2d(
        &mut Target {
            px: dp,
            w: dw as usize,
            h: dh as usize,
        },
        Source {
            px: sp,
            w: sw as usize,
            h: sh as usize,
        },
        p(d6),
        p(s6),
        use_alpha != 0,
        smooth != 0,
    );
}

/// `hardware_img_tri3d` in software. `d` holds x,y,z * 3, `s` x,y * 3;
/// `depth` holds `dw*dh` floats (initialise to +infinity).
///
/// # Safety
/// As [`qb_hw_put`], plus `depth`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_hw_tri3d(
    dst: *mut u32,
    dw: i32,
    dh: i32,
    depth: *mut f32,
    src: *const u32,
    sw: i32,
    sh: i32,
    d: *const f32,
    s: *const f32,
    use_alpha: i32,
    smooth: i32,
    cull: i32,
    depth_mode: i32,
) {
    if dst.is_null() || src.is_null() || depth.is_null() || dw <= 0 || dh <= 0 || sw <= 0 || sh <= 0
    {
        return;
    }
    let n = (dw * dh) as usize;
    // SAFETY: sizes as documented by the caller.
    let (dp, zp, sp, d9, s6) = unsafe {
        (
            std::slice::from_raw_parts_mut(dst, n),
            std::slice::from_raw_parts_mut(depth, n),
            std::slice::from_raw_parts(src, (sw * sh) as usize),
            *(d as *const [f32; 9]),
            *(s as *const [f32; 6]),
        )
    };
    raster::tri3d(
        &mut Target {
            px: dp,
            w: dw as usize,
            h: dh as usize,
        },
        zp,
        Source {
            px: sp,
            w: sw as usize,
            h: sh as usize,
        },
        [
            (d9[0], d9[1], d9[2]),
            (d9[3], d9[4], d9[5]),
            (d9[6], d9[7], d9[8]),
        ],
        [(s6[0], s6[1]), (s6[2], s6[3]), (s6[4], s6[5])],
        use_alpha != 0,
        smooth != 0,
        cull,
        depth_mode,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_hash_is_fnv1a() {
        assert_eq!(frame_hash(&[]), 0x811C_9DC5);
        assert_ne!(frame_hash(&[1]), frame_hash(&[2]));
    }

    #[test]
    fn named_keys_map() {
        assert_eq!(named(&NamedKey::F11), Some(Key::F(11)));
        assert_eq!(named(&NamedKey::Space), Some(Key::Char(' ')));
        assert_eq!(named(&NamedKey::CapsLock), Some(Key::Other(0x14)));
        assert_eq!(named(&NamedKey::MediaPlay), None);
    }
}
