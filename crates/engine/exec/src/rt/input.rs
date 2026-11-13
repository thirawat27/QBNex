//! Keyboard and mouse input of the VM: INKEY$, _KEYHIT, _KEYDOWN, _MOUSEINPUT,
//! _MOUSEX/_MOUSEY, _MOUSEBUTTON, _MOUSEWHEEL.
//!
//! Events come from qb-runtime's window layer (an input script replayed by
//! the headless window, `QBNEX_INPUT_SCRIPT`) through its callbacks; they are
//! queued and applied on the program thread when the program asks for input.
//! The key path is a port of libqb.cpp `keydown` / `keyup` (BIOS INKEY$
//! buffer in conventional memory, _KEYHIT ring and held keys in qb-runtime
//! keybuf) and the mouse path of window_bridge.cpp (one message queue);
//! CTRL+BREAK, PAUSE and ALT+ENTER are not handled here.
//!
//! Mouse messages go to a queue the program reads with _MOUSEINPUT; a mouse
//! pipe (_MOUSEPIPEOPEN) is a further queue put in front of it. The same
//! events, key scan codes and the game controllers feed the devices of
//! _DEVICEINPUT (rt/devices.rs).

// browser build: the window callbacks that feed events are not compiled in
#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

use super::devices::{self, Devices};
use super::scancodes::SCANCODE_LOOKUP;
use super::{e, R};
use qb_runtime::console::{keyboard, utf8};
use qb_runtime::keybuf;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

const QBK: u32 = 200_000;
const VK: u32 = 100_000;
const UC: u32 = 1_073_741_824;
const QBK_SCROLL_LOCK_MODE: u32 = 11;
const QBK_INSERT_MODE: u32 = 12;
const QBK_CHR0: u32 = 13;
const KP0: u32 = 256;
const KP9: u32 = 265;
const KP_PERIOD: u32 = 266;
const KP_DIVIDE: u32 = 267;
const KP_MULTIPLY: u32 = 268;
const KP_MINUS: u32 = 269;
const KP_PLUS: u32 = 270;
const KP_ENTER: u32 = 271;
const NUMLOCK: u32 = 300;
const CAPSLOCK: u32 = 301;
const SCROLLOCK: u32 = 302;
const RSHIFT: u32 = 303;
const LSHIFT: u32 = 304;
const RCTRL: u32 = 305;
const LCTRL: u32 = 306;
const RALT: u32 = 307;
const LALT: u32 = 308;

#[derive(Clone, Copy)]
enum Ev {
    Key(i32, u32),
    Button(i32, bool, i32, i32),
    Move(i32, i32),
    Wheel(i32, i32, i32),
    /// raw pointer movement (not bound to the window)
    Relative(i32, i32),
}

static EVENTS: Mutex<VecDeque<Ev>> = Mutex::new(VecDeque::new());
static STARTED: AtomicBool = AtomicBool::new(false);

/// Queued events beyond which consecutive mouse moves are merged (a GUI
/// program that never reads input would otherwise grow the queue forever).
const MOVE_BACKLOG: usize = 256;

fn push(e: Ev) {
    let mut q = EVENTS.lock().unwrap_or_else(|p| p.into_inner());
    if matches!(e, Ev::Move(..)) && q.len() >= MOVE_BACKLOG
        && let Some(last) = q.back_mut()
            && matches!(last, Ev::Move(..)) {
                *last = e;
                return;
            }
    q.push_back(e);
}
/// Key presses seen so far (SLEEP ends at the next one).
pub(crate) static KEY_PRESSES: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

pub(crate) extern "C" fn cb_key(action: i32, code: u32) {
    if action == 1 || action == 3 {
        KEY_PRESSES.fetch_add(1, Ordering::Relaxed);
    }
    push(Ev::Key(action, code));
}
pub(crate) extern "C" fn cb_button(b: i32, down: i32, x: i32, y: i32) {
    push(Ev::Button(b, down != 0, x, y));
}
pub(crate) extern "C" fn cb_move(x: i32, y: i32) {
    push(Ev::Move(x, y));
}
pub(crate) extern "C" fn cb_wheel(dir: i32, x: i32, y: i32) {
    push(Ev::Wheel(dir, x, y));
}
pub(crate) extern "C" fn cb_relative(dx: i32, dy: i32) {
    push(Ev::Relative(dx, dy));
}

/// Size of the page the mouse moves over (the mouse axes of `_DEVICEINPUT` are relative to it).
static PAGE_W: AtomicI32 = AtomicI32::new(640);
static PAGE_H: AtomicI32 = AtomicI32::new(400);

/// The shown page changed its pixel size.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) fn set_page_size(w: i32, h: i32) {
    PAGE_W.store(w, Ordering::Relaxed);
    PAGE_H.store(h, Ordering::Relaxed);
}

/// True when pointer movement is taken from the difference of two positions:
/// a real Windows window reports it as raw input instead ([`Ev::Relative`]).
fn movement_from_positions() -> bool {
    #[cfg(all(windows, not(target_arch = "wasm32")))]
    {
        qb_runtime::window::qb_window_headless() != 0
    }
    #[cfg(not(all(windows, not(target_arch = "wasm32"))))]
    {
        true
    }
}

/// Starts the headless window with the input script (once), when one is set.
pub(crate) fn start() {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    if std::env::var_os("QBNEX_INPUT_SCRIPT").is_none_or(|s| s.is_empty()) {
        return;
    }
    // a GUI program's window (rt/window.rs) already replays the script on the main thread
    #[cfg(not(target_arch = "wasm32"))]
    if super::window::active() {
        return;
    }
    // browser build: no window layer and no threads; the page feeds keys instead
    #[cfg(not(target_arch = "wasm32"))]
    start_window();
}

#[cfg(not(target_arch = "wasm32"))]
fn start_window() {
    // no window here: the window layer must run headless (it then only replays the script)
    // SAFETY: set before the window thread starts; no other thread reads the environment yet.
    unsafe { std::env::set_var("QBNEX_HEADLESS", "1") };
    let cb = qb_runtime::window::Callbacks {
        key: Some(cb_key),
        mouse_button: Some(cb_button),
        mouse_move: Some(cb_move),
        mouse_wheel: Some(cb_wheel),
        mouse_relative: None,
        resize: None,
        close: None,
        focus: None,
        file_drop: None,
        created: None,
    };
    std::thread::spawn(move || {
        // SAFETY: valid callbacks, null title.
        unsafe { qb_runtime::window::qb_window_run(&cb, 640, 400, core::ptr::null(), 0, 1) };
    });
}

/// Browser build: the page's keyboard and mouse events, in the
/// layout of `crates/web/www/qbnex.js` (`[kind, a, b, c, d, mods]`):
/// 1 key (a pressed, b repeat, c key id, d character or virtual key; mods
/// 1 shift, 2 ctrl, 4 alt), 2 button (a button 0-2, b down, c x, d y),
/// 3 move (a x, b y), 4 wheel (a direction, b x, c y). Keys go through the
/// same translation as the window layer (`qb_runtime::keymap`).
#[cfg(target_arch = "wasm32")]
fn web_events() {
    use qb_runtime::keymap::{self, Key, KeyAction, KeyInput};
    // at most one ring's worth per pump, so a flooding page cannot stall the program
    for _ in 0..256 {
        let Some([kind, a, b, c, d, mods]) = qb_runtime::systime::web::poll_event() else { break };
        match kind {
            1 => {
                let key = match c {
                    0 => match char::from_u32(d as u32) {
                        Some(ch) => Key::Char(ch),
                        None => continue,
                    },
                    1..=12 => Key::F(c as u8),
                    20 => Key::PageUp,
                    21 => Key::PageDown,
                    22 => Key::Home,
                    23 => Key::End,
                    24 => Key::Left,
                    25 => Key::Up,
                    26 => Key::Right,
                    27 => Key::Down,
                    28 => Key::Insert,
                    29 => Key::Delete,
                    30 => Key::LCtrl,
                    31 => Key::RCtrl,
                    32 => Key::LShift,
                    33 => Key::RShift,
                    34 => Key::LAlt,
                    35 => Key::RAlt,
                    36 => Key::Pause,
                    37 => Key::Cancel,
                    38 => Key::Enter,
                    39 => Key::Escape,
                    40 => Key::Tab,
                    41 => Key::Backspace,
                    _ => Key::Other(d as u32),
                };
                let ev = KeyInput { pressed: a != 0, repeat: b != 0, key, shift: mods & 1 != 0, ctrl: mods & 2 != 0, alt: mods & 4 != 0 };
                for act in keymap::translate(&ev) {
                    match act {
                        KeyAction::Down(x) => push(Ev::Key(1, x)),
                        KeyAction::Up(x) => push(Ev::Key(2, x)),
                        KeyAction::DownUnicode(x) => push(Ev::Key(3, x)),
                        KeyAction::UpUnicode(x) => push(Ev::Key(4, x)),
                    }
                }
            }
            2 => push(Ev::Button(a, b != 0, c, d)),
            3 => push(Ev::Move(a, b)),
            4 => push(Ev::Wheel(a, b, c)),
            _ => {}
        }
    }
}

#[derive(Clone, Copy, Default)]
struct MouseMsg {
    x: i32,
    y: i32,
    movement_x: i32,
    movement_y: i32,
    buttons: u32,
}

/// Messages a mouse queue holds; the oldest unread one is dropped beyond it.
const MOUSE_QUEUE: usize = 65536;
/// Handle of the queue every program has (_MOUSEINPUT without a pipe).
const DEFAULT_QUEUE: usize = 0;

/// A ring of mouse messages: `current` is the one `_MOUSEX` and the others read,
/// `last` the newest.
struct MouseQueue {
    msgs: Vec<MouseMsg>,
    current: usize,
    last: usize,
    /// the queue `_MOUSEINPUTPIPE` passes a message on to
    child: Option<usize>,
    open: bool,
}

impl MouseQueue {
    fn new(child: Option<usize>) -> MouseQueue {
        MouseQueue { msgs: vec![MouseMsg::default(); MOUSE_QUEUE], current: 0, last: 0, child, open: true }
    }
    /// Appends the message `make` builds from the newest one.
    fn add(&mut self, make: impl FnOnce(MouseMsg) -> MouseMsg) {
        let n = self.msgs.len();
        let i = (self.last + 1) % n;
        if i == self.current {
            self.current = (i + 1) % n;
        }
        self.msgs[i] = make(self.msgs[self.last]);
        self.last = i;
    }
    fn message(&self) -> MouseMsg {
        self.msgs[self.current]
    }
}

/// Key and mouse state of the program (libqb globals).
pub(crate) struct Input {
    keyup_mask: Vec<u32>,
    insert_held: bool,
    scroll_lock_held: bool,
    glyph: bool,
    port60h: Vec<u8>,
    port60_last: u8,
    /// mouse queues by handle - 1; [`DEFAULT_QUEUE`] always exists
    queues: Vec<MouseQueue>,
    /// the queue new messages go to: the default one, or the pipe opened last
    first: usize,
    pub(crate) devices: Devices,
}

impl Default for Input {
    fn default() -> Self {
        Input {
            keyup_mask: Vec::new(),
            insert_held: false,
            scroll_lock_held: false,
            glyph: false,
            port60h: Vec::new(),
            port60_last: 0,
            queues: vec![MouseQueue::new(None)],
            first: DEFAULT_QUEUE,
            devices: Devices::default(),
        }
    }
}

fn held(x: u32) -> bool {
    keybuf::qb_keyheld(x) != 0
}

fn sc(i: usize) -> i32 {
    SCANCODE_LOOKUP.get(i).copied().unwrap_or(0)
}

impl Input {
    /// Applies the queued events (program thread).
    pub(crate) fn pump(&mut self, cmem: &mut [u8]) {
        start();
        #[cfg(target_arch = "wasm32")]
        web_events();
        if cmem[0x41A] == 0 && cmem[0x41C] == 0 {
            // SAFETY: BIOS data area inside cmem.
            unsafe { keyboard::clear(cmem[0x400..].as_mut_ptr()) };
        }
        let evs: Vec<Ev> = EVENTS.lock().unwrap_or_else(|p| p.into_inner()).drain(..).collect();
        for e in evs {
            match e {
                Ev::Key(1, x) => self.keydown(cmem, x),
                Ev::Key(2, x) => self.keyup(cmem, x),
                Ev::Key(3, x) => self.keydown_unicode(cmem, x),
                Ev::Key(4, x) => self.keyup_unicode(cmem, x),
                Ev::Key(..) => {}
                Ev::Button(b, down, x, y) => {
                    let n = b + 1;
                    if down {
                        self.button_down(n, x, y)
                    } else {
                        self.button_up(n, x, y)
                    }
                }
                Ev::Move(x, y) => {
                    let deltas = movement_from_positions();
                    let q = &mut self.queues[self.first];
                    q.add(|m| {
                        let (dx, dy) = if deltas { (x - m.x, y - m.y) } else { (0, 0) };
                        MouseMsg { x, y, movement_x: dx, movement_y: dy, ..m }
                    });
                    // a second message ends the movement
                    q.add(|m| MouseMsg { movement_x: 0, movement_y: 0, ..m });
                    let (w, h) = (PAGE_W.load(Ordering::Relaxed), PAGE_H.load(Ordering::Relaxed));
                    let axis = |v: i32, n: i32| if n > 1 { v.clamp(0, n - 1) as f32 / (n - 1) as f32 * 2.0 - 1.0 } else { 0.0 };
                    self.devices.axes(devices::MOUSE, &[(0, axis(x, w)), (1, axis(y, h))]);
                }
                Ev::Relative(dx, dy) => {
                    let q = &mut self.queues[self.first];
                    q.add(|m| MouseMsg { movement_x: dx, movement_y: dy, ..m });
                    q.add(|m| MouseMsg { movement_x: 0, movement_y: 0, ..m });
                    self.devices.wheel(devices::MOUSE, &[(0, dx as f32), (1, dy as f32)]);
                }
                Ev::Wheel(dir, x, y) => {
                    let n = if dir > 0 { 4 } else { 5 };
                    self.button_down(n, x, y);
                    self.button_up(n, x, y);
                    self.devices.wheel(devices::MOUSE, &[(devices::SCROLL_WHEEL, if dir > 0 { -1.0 } else { 1.0 })]);
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if self.devices.controllers {
            pads::poll(&mut self.devices);
        }
    }

    /// Buttons 1 left, 2 middle, 3 right; 4 and 5 are the wheel steps.
    fn button_down(&mut self, button: i32, x: i32, y: i32) {
        self.queues[self.first].add(|m| MouseMsg { x, y, movement_x: 0, movement_y: 0, buttons: m.buttons | (1 << (button - 1)) });
        if (1..=3).contains(&button) {
            self.devices.button(devices::MOUSE, button as usize - 1, true);
        }
    }
    fn button_up(&mut self, button: i32, x: i32, y: i32) {
        self.queues[self.first].add(|m| MouseMsg { x, y, movement_x: 0, movement_y: 0, buttons: m.buttons & !(1 << (button - 1)) });
        if (1..=3).contains(&button) {
            self.devices.button(devices::MOUSE, button as usize - 1, false);
        }
    }

    /// The queue of a mouse function: the default one, or the pipe `context` names
    /// (error 258 for a handle that is not an open queue).
    pub(crate) fn queue(&self, context: i64, passed: bool) -> R<usize> {
        if !passed {
            return Ok(DEFAULT_QUEUE);
        }
        match usize::try_from(context - 1).ok().filter(|&q| self.queues.get(q).is_some_and(|q| q.open)) {
            Some(q) => Ok(q),
            None => e(258),
        }
    }

    /// `_MOUSEPIPEOPEN`: a new queue that gets the mouse messages before the
    /// queues opened earlier; its handle.
    pub(crate) fn pipe_open(&mut self) -> i32 {
        self.queues.push(MouseQueue::new(Some(self.first)));
        self.first = self.queues.len() - 1;
        self.queues.len() as i32
    }

    /// `_MOUSEINPUTPIPE`: hands the current message of pipe `q` on to the queue behind it.
    pub(crate) fn pipe_pass(&mut self, q: usize) -> R<()> {
        let Some(child) = self.queues[q].child else { return e(5) };
        let msg = self.queues[q].message();
        self.queues[child].add(|_| msg);
        Ok(())
    }

    /// `_MOUSEPIPECLOSE`: removes pipe `q`; the queue behind it gets the messages again.
    pub(crate) fn pipe_close(&mut self, q: usize) -> R<()> {
        let Some(child) = self.queues[q].child else { return e(5) };
        if self.first == q {
            self.first = child;
        }
        for other in self.queues.iter_mut().filter(|o| o.child == Some(q)) {
            other.child = Some(child);
        }
        let closed = &mut self.queues[q];
        closed.open = false;
        closed.msgs = Vec::new();
        Ok(())
    }

    /// `func__mouseinput`
    pub(crate) fn mouseinput(&mut self, q: usize) -> bool {
        let q = &mut self.queues[q];
        if q.current == q.last {
            return false;
        }
        q.current = (q.current + 1) % q.msgs.len();
        true
    }
    /// The most recent mouse message: (x, y, buttons) (INT 33h status).
    pub(crate) fn latest_mouse(&self) -> (i32, i32, u32) {
        // (with a mouse pipe open the newest message is in the pipe)
        let q = &self.queues[self.first];
        let m = q.msgs[q.last];
        (m.x, m.y, m.buttons)
    }
    pub(crate) fn mouse_xy(&self, q: usize) -> (i32, i32) {
        let m = self.queues[q].message();
        (m.x, m.y)
    }
    /// `_MOUSEMOVEMENTX`, `_MOUSEMOVEMENTY`
    pub(crate) fn mouse_movement(&self, q: usize) -> (i32, i32) {
        let m = self.queues[q].message();
        (m.movement_x, m.movement_y)
    }
    /// `func__mousebutton` (buttons 2 and 3 swapped as libqb)
    pub(crate) fn mousebutton(&self, q: usize, i: i32) -> bool {
        let i = match i {
            2 => 3,
            3 => 2,
            n => n,
        };
        if !(1..=3).contains(&i) {
            return false;
        }
        self.queues[q].message().buttons & (1 << (i - 1)) != 0
    }
    /// `func__mousewheel`
    pub(crate) fn mousewheel(&self, q: usize) -> i32 {
        let x = self.queues[q].message().buttons;
        if x & 24 == 24 {
            0
        } else if x & 8 != 0 {
            -1
        } else if x & 16 != 0 {
            1
        } else {
            0
        }
    }

    /// INP(&H60): the oldest queued scancode event; with none queued, the last one read.
    /// `_KEYCLEAR 3`: forgets the scan codes INP(&H60) has not read yet.
    pub(crate) fn clear_port60(&mut self) {
        self.port60h.clear();
    }

    pub(crate) fn port60(&mut self) -> u8 {
        if self.port60h.is_empty() {
            return self.port60_last;
        }
        self.port60_last = self.port60h.remove(0);
        self.port60_last
    }

    fn scancodedown(&mut self, s: i32) {
        self.devices.button(devices::KEYBOARD, s as usize & 255, true);
        let s = s as u8;
        if self.port60h.last() == Some(&s) {
            return;
        }
        if self.port60h.len() == 256 {
            self.port60h.remove(0);
        }
        self.port60h.push(s);
    }
    fn scancodeup(&mut self, s: i32) {
        self.devices.button(devices::KEYBOARD, s as usize & 255, false);
        let s = (s + 128) as u8;
        if self.port60h.last() == Some(&s) {
            return;
        }
        if self.port60h.len() == 256 {
            self.port60h.remove(0);
        }
        self.port60h.push(s);
    }

    fn update_shift_state(cmem: &mut [u8]) {
        let mut b = [0u8; 3];
        // SAFETY: three writable bytes.
        unsafe { keybuf::qb_shift_state(b.as_mut_ptr()) };
        cmem[0x417] = b[0];
        cmem[0x418] = b[1];
        cmem[0x496] = b[2];
    }

    fn keydown_unicode(&mut self, cmem: &mut [u8], x: u32) {
        self.glyph = true;
        if x <= 127 {
            return self.keydown(cmem, x);
        }
        let x2 = utf8::unicode_to_cp437(x);
        if x2 != 0 {
            return self.keydown(cmem, x2);
        }
        if (0xFF01..=0xFF5E).contains(&x) {
            return self.keydown(cmem, x - 0xFF01 + 0x21);
        }
        if x == 0x3000 {
            return self.keydown(cmem, 32);
        }
        self.keydown(cmem, x | UC);
    }
    fn keyup_unicode(&mut self, cmem: &mut [u8], x: u32) {
        if x <= 127 {
            return self.keyup(cmem, x);
        }
        let x2 = utf8::unicode_to_cp437(x);
        if x2 != 0 {
            return self.keyup(cmem, x2);
        }
        if (0xFF01..=0xFF5E).contains(&x) {
            return self.keyup(cmem, x - 0xFF01 + 0x21);
        }
        if x == 0x3000 {
            return self.keyup(cmem, 32);
        }
        self.keyup(cmem, x | UC);
    }

    /// numpad keys reverted to their plain codes, and the numpad flag of _KEYHIT
    fn plain(x: u32) -> (u32, i64) {
        if (VK + KP0..=VK + KP_ENTER).contains(&x) {
            let x2 = if (VK + KP0..=VK + KP9).contains(&x) {
                x - (VK + KP0) + 48
            } else {
                match x - VK {
                    KP_PERIOD => 46,
                    KP_DIVIDE => 47,
                    KP_MULTIPLY => 42,
                    KP_MINUS => 45,
                    KP_PLUS => 43,
                    _ => 13,
                }
            };
            return (x2, 1 << 32);
        }
        if (QBK..=QBK + 10).contains(&x) {
            let k = [82, 79, 80, 81, 75, 76, 77, 71, 72, 73, 83][(x - QBK) as usize];
            return (k << 8, 1 << 32);
        }
        (x, 0)
    }

    fn modifier_scancode(x: u32) -> Option<i32> {
        Some(match x.checked_sub(VK)? {
            LSHIFT => 42,
            RSHIFT => 54,
            LALT | RALT => 56,
            LCTRL | RCTRL => 29,
            NUMLOCK => 69,
            CAPSLOCK => 58,
            SCROLLOCK => 70,
            _ => return None,
        })
    }

    fn keyup(&mut self, cmem: &mut [u8], mut x: u32) {
        if x == 0 {
            x = QBK + QBK_CHR0;
        }
        keybuf::qb_keyheld_remove(x);
        let (x2, numpad) = Self::plain(x);
        if let Some(p) = self.keyup_mask.iter().position(|&m| m == x) {
            self.keyup_mask[p] = 0;
            return;
        }
        keybuf::qb_keyhit_push(i64::from(-(x2 as i32)) | numpad);
        if x <= 255 {
            if sc(x as usize * 10 + 2) != 0 {
                self.scancodeup(sc(x as usize * 10 + 1));
            }
            return;
        }
        let mut xx = x;
        let mut numpadkey = false;
        if (VK + KP0..=VK + KP_ENTER).contains(&x) {
            xx = (x - (VK + KP0) + 256) * 256;
            numpadkey = true;
        } else if (QBK..=QBK + (KP_PERIOD - KP0)).contains(&x) {
            xx = (x - QBK + 256) * 256;
            numpadkey = true;
        }
        if numpadkey || xx <= 65535 {
            let r = ((xx >> 8) + 256) as usize;
            if sc(r * 10 + 2) != 0 {
                self.scancodeup(sc(r * 10 + 1));
            }
            if xx == 0x5200 {
                Self::update_shift_state(cmem);
            }
            return;
        }
        if let Some(s) = Self::modifier_scancode(x) {
            self.scancodeup(s);
            Self::update_shift_state(cmem);
        }
    }

    /// ON KEY(1..14, 30, 31): F1-F10, the arrows (also on the keypad), F11, F12
    /// (libqb.cpp keydown, priority 2). Marks the slot; true = the key was trapped.
    fn onkey_trap(x: u32, x2: u32) -> bool {
        let keys = qb_runtime::events::onkeys();
        for (i, k) in keys.iter_mut().enumerate() {
            if k.active == 0 || k.id == 0 {
                continue;
            }
            let (code, alt) = match i {
                1..=10 => ((58 + i as u32) << 8, 0),
                11 => (72 << 8, VK + KP0 + 8),
                12 => (75 << 8, VK + KP0 + 4),
                13 => (77 << 8, VK + KP0 + 6),
                14 => (80 << 8, VK + KP0 + 2),
                30 => (133 << 8, 0),
                31 => (134 << 8, 0),
                _ => continue,
            };
            if x2 == code || (alt != 0 && x == alt) {
                if k.active == 1 {
                    k.state = k.state.saturating_add(1);
                } else {
                    k.state = 1;
                }
                return true;
            }
        }
        false
    }

    fn keydown(&mut self, cmem: &mut [u8], mut x: u32) {
        if x == 0 {
            x = QBK + QBK_CHR0;
        }
        let glyph = std::mem::take(&mut self.glyph);
        #[cfg(not(target_arch = "wasm32"))]
        if x == 13 && (held(VK + LALT) || held(VK + RALT)) && super::window::alt_enter() {
            // ALT+ENTER switched the full screen mode: the program sees neither the key nor its key-up
            match self.keyup_mask.iter().position(|&m| m == 0) {
                Some(p) => self.keyup_mask[p] = x,
                None if self.keyup_mask.len() < 256 => self.keyup_mask.push(x),
                None => {}
            }
            return;
        }
        if x == 0x5200 {
            self.insert_held = held(0x5200);
        }
        if x == VK + SCROLLOCK {
            self.scroll_lock_held = held(VK + SCROLLOCK);
        }
        keybuf::qb_keyheld_add(x, 0);
        let (x2, numpad) = Self::plain(x);
        keybuf::qb_keyhit_push(i64::from(x2) | numpad);
        if Self::onkey_trap(x, x2) {
            // the trapped key does not reach the keyboard buffer, nor its key-up
            match self.keyup_mask.iter().position(|&m| m == 0) {
                Some(p) => self.keyup_mask[p] = x,
                None if self.keyup_mask.len() < 256 => self.keyup_mask.push(x),
                None => {}
            }
            return;
        }
        let bios = &mut cmem[0x400..];
        let (shift, ctrl, alt) = (held(VK + LSHIFT) || held(VK + RSHIFT), held(VK + LCTRL) || held(VK + RCTRL), held(VK + LALT) || held(VK + RALT));
        let mut x = x;
        if x == QBK + QBK_CHR0 {
            x = 0;
        }
        if x <= 255 {
            let mut b1 = x as i32;
            let mut b2 = sc(x as usize * 10 + 1);
            if b2 != 0 {
                self.scancodedown(b2);
                let mut o = 0;
                if shift {
                    o = 1;
                }
                if ctrl {
                    o = 2;
                }
                if alt {
                    o = 3;
                }
                if glyph && !held(VK + LALT) && !held(VK + RCTRL) && held(VK + LCTRL) && held(VK + RALT) {
                    o = 0; // alt-gr combination
                }
                let z = sc(x as usize * 10 + 2 + o);
                if z == 0 {
                    return;
                }
                if z & 0xFF00 != 0 {
                    b1 = 0;
                    b2 = z >> 8;
                } else {
                    b1 = z;
                }
            }
            // SAFETY: bios = cmem + 0x400 (BIOS data area).
            unsafe { keyboard::push(bios.as_mut_ptr(), b1 as u8, b2 as u8) };
            return;
        }
        let mut numlock = false;
        let mut numpadkey = false;
        if (VK + KP0..=VK + KP_ENTER).contains(&x) {
            numlock = (VK + KP0..=VK + KP_PERIOD).contains(&x);
            x = (x - (VK + KP0) + 256) * 256;
            numpadkey = true;
        } else if (QBK..=QBK + (KP_PERIOD - KP0)).contains(&x) {
            x = (x - QBK + 256) * 256;
            numpadkey = true;
        }
        if numpadkey || x <= 65535 {
            let mut b1 = 0;
            let mut b2 = (x >> 8) as i32;
            let r = ((x >> 8) + 256) as usize;
            if sc(r * 10 + 2) != 0 {
                self.scancodedown(sc(r * 10 + 1));
                if x == 0x5200 && !self.insert_held && !alt && !shift && !ctrl {
                    if held(QBK + QBK_INSERT_MODE) {
                        keybuf::qb_keyheld_remove(QBK + QBK_INSERT_MODE);
                    } else {
                        keybuf::qb_keyheld_add(QBK + QBK_INSERT_MODE, 0);
                    }
                    Self::update_shift_state(cmem);
                }
                let mut o = 0;
                if shift {
                    o = 1;
                }
                if numlock {
                    o = 4;
                }
                if numlock && shift {
                    o = 7;
                }
                if ctrl {
                    o = 2;
                }
                if alt {
                    o = 3;
                }
                let z = sc(r * 10 + 2 + o);
                if z == 0 {
                    return;
                }
                if z & 0xFF00 != 0 {
                    b1 = 0;
                    b2 = z >> 8;
                } else {
                    b1 = z;
                    b2 = sc(r * 10 + 1);
                }
            }
            let bios = &mut cmem[0x400..];
            // SAFETY: as above.
            unsafe { keyboard::push(bios.as_mut_ptr(), b1 as u8, b2 as u8) };
            return;
        }
        if let Some(s) = Self::modifier_scancode(x) {
            self.scancodedown(s);
            if x == VK + SCROLLOCK && !self.scroll_lock_held && !ctrl {
                if held(QBK + QBK_SCROLL_LOCK_MODE) {
                    keybuf::qb_keyheld_remove(QBK + QBK_SCROLL_LOCK_MODE);
                } else {
                    keybuf::qb_keyheld_add(QBK + QBK_SCROLL_LOCK_MODE, 0);
                }
            }
            Self::update_shift_state(cmem);
        }
    }

    /// `qbs_inkey`: next key of the BIOS buffer
    pub(crate) fn inkey(&mut self, cmem: &mut [u8]) -> Vec<u8> {
        self.pump(cmem);
        // SAFETY: cmem + 0x400 is the BIOS data area.
        unsafe { keyboard::pop(cmem[0x400..].as_mut_ptr()) }.unwrap_or_default()
    }
}

/// Game controllers: qb-runtime reports them through C callbacks; the events
/// wait here until the program thread applies them to the devices.
#[cfg(not(target_arch = "wasm32"))]
mod pads {
    use super::Devices;
    use qb_runtime::gamepad::{qb_gamepad_poll, Callbacks};
    use std::ffi::{c_char, CStr};
    use std::sync::Mutex;

    enum Pad {
        Attached(u32, String, i32, i32, i32, i32),
        Removed(u32),
        Button(u32, i32, bool),
        Axis(u32, i32, f32),
    }

    static EVENTS: Mutex<Vec<Pad>> = Mutex::new(Vec::new());

    fn push(p: Pad) {
        EVENTS.lock().unwrap_or_else(|p| p.into_inner()).push(p);
    }
    extern "C" fn attached(id: u32, name: *const c_char, vendor: i32, product: i32, axes: i32, buttons: i32) {
        let name = if name.is_null() {
            String::new()
        } else {
            // SAFETY: qb-runtime passes a NUL-terminated name that lives for this call.
            unsafe { CStr::from_ptr(name) }.to_string_lossy().into_owned()
        };
        push(Pad::Attached(id, name, vendor, product, axes, buttons));
    }
    extern "C" fn removed(id: u32) {
        push(Pad::Removed(id));
    }
    extern "C" fn button(id: u32, button: i32, down: i32) {
        push(Pad::Button(id, button, down != 0));
    }
    extern "C" fn axis(id: u32, axis: i32, value: f32) {
        push(Pad::Axis(id, axis, value));
    }

    /// Reads the controllers (program thread) and applies what changed.
    pub(super) fn poll(devices: &mut Devices) {
        let cb = Callbacks { attached: Some(attached), removed: Some(removed), button: Some(button), axis: Some(axis) };
        // SAFETY: a valid callback table.
        unsafe { qb_gamepad_poll(&cb) };
        let events = std::mem::take(&mut *EVENTS.lock().unwrap_or_else(|p| p.into_inner()));
        for ev in events {
            match ev {
                Pad::Attached(id, name, vendor, product, axes, buttons) => devices.attach(id, &name, vendor, product, axes.max(0) as usize, buttons.max(0) as usize),
                Pad::Removed(id) => devices.detach(id),
                Pad::Button(id, b, down) if b >= 0 => devices.pad_button(id, b as usize, down),
                Pad::Axis(id, a, v) if a >= 0 => devices.pad_axis(id, a as usize, v),
                Pad::Button(..) | Pad::Axis(..) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{cb_button, cb_move, Ev, EVENTS, MOVE_BACKLOG};

    #[test]
    fn unread_mouse_moves_are_merged_past_the_backlog() {
        EVENTS.lock().unwrap_or_else(|p| p.into_inner()).clear();
        for x in 0..10_000 {
            cb_move(x, 1);
        }
        cb_button(1, 1, 5, 5);
        cb_move(7, 7);
        let q: Vec<Ev> = EVENTS.lock().unwrap_or_else(|p| p.into_inner()).drain(..).collect();
        assert_eq!(q.len(), MOVE_BACKLOG + 2);
        assert!(matches!(q[MOVE_BACKLOG - 1], Ev::Move(9999, 1)));
        assert!(matches!(q[MOVE_BACKLOG], Ev::Button(1, true, 5, 5)));
        assert!(matches!(q[MOVE_BACKLOG + 1], Ev::Move(7, 7)));
    }
}
