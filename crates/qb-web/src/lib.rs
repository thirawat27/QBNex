//! `qb-web` — QBNex programs in a browser (R13-wasm).
//!
//! The same pipeline as `qb-exec` (QIR → bytecode → VM), compiled to
//! `wasm32-unknown-unknown`, without the JIT (wasm cannot run Cranelift
//! output). The page (`www/qbnex.js`) loads the `.wasm`, hands it a `.qir`
//! program and stdin text, and draws the screen:
//!
//! - **Screen.** The program runs on the SCREEN 0 page from its first
//!   statement, as in a window: PRINT, CLS, LOCATE, COLOR, SCREEN 13 and the
//!   other graphics statements draw on qb-runtime surfaces; [`screen`] turns
//!   the visible page into RGBA for a canvas (and text, for tests).
//! - **Host imports** (module `qbnex`, see `qb_runtime::systime::web`): the
//!   page supplies the clock (TIMER, `_DELAY`, `_LIMIT`), sleeping, and is
//!   told when `_DISPLAY` finished a frame.
//! - **INPUT** reads the stdin text given to [`qb_web_run`] and echoes what it
//!   consumed on the screen. There is no interactive keyboard yet: a wasm call
//!   cannot wait for the page's events.
//! - **Unsupported** host services return QBasic errors, never a trap: files
//!   and directories (no file system: error 75/53), TCP/HTTP, SHELL, the
//!   clipboard (page-private), sound (silent). Any other import the module
//!   needs is stubbed by the page with an error.
//!
//! C ABI: every export is `qb_web_*`; strings and buffers are (pointer,
//! length) pairs in wasm memory, allocated with [`qb_web_alloc`].

pub mod screen;

use std::io::Write;
use std::sync::Mutex;

/// Console output of the program (`$CONSOLE` / `_DEST _CONSOLE`, and what
/// PRINT writes before the screen exists).
static OUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());
/// Buffer the last text/frame query returned.
static BUF: Mutex<Vec<u8>> = Mutex::new(Vec::new());
static LAST: Mutex<Outcome> = Mutex::new(Outcome::new());

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// How a run ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// 0 = finished (END/SYSTEM), 1 = runtime error, 2 = the program did not load
    pub status: i32,
    /// `SYSTEM n`
    pub exit_code: i32,
    /// QBasic error number and source line (status 1); message (1 and 2)
    pub error: i32,
    pub line: u32,
    pub message: String,
    /// console output
    pub stdout: Vec<u8>,
    /// the visible page when the program stopped
    pub screen: Option<screen::Frame>,
}

impl Outcome {
    const fn new() -> Outcome {
        Outcome { status: 0, exit_code: 0, error: 0, line: 0, message: String::new(), stdout: Vec::new(), screen: None }
    }
}

struct WebOut;

impl Write for WebOut {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        lock(&OUT).extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Console output collected so far (the program's writer and qb-runtime's
/// stdout path); drained.
fn take_stdout() -> Vec<u8> {
    let mut v = std::mem::take(&mut *lock(&OUT));
    #[cfg(target_arch = "wasm32")]
    v.extend_from_slice(&qb_runtime::console::stdio::take_stdout());
    v.shrink_to_fit();
    v
}

/// Runs a QIR program to its end with `stdin` as the keyboard text.
pub fn run(src: &str, stdin: &[u8]) -> Outcome {
    lock(&OUT).clear();
    let m = match qb_exec::compile(src) {
        Ok(m) => m,
        Err(e) => {
            return Outcome { status: 2, error: e.code, message: e.msg, stdout: take_stdout(), ..Outcome::new() };
        }
    };
    let mut out = WebOut;
    let mut vm = qb_exec::Vm::new(&m, &mut out);
    vm.set_web_console(stdin);
    let r = vm.run_status();
    let screen = vm.screen_record().as_deref().and_then(screen::parse).map(|p| screen::render(&p));
    drop(vm);
    let stdout = take_stdout();
    match r {
        Ok(code) => Outcome { exit_code: code, stdout, screen, ..Outcome::new() },
        Err(e) => Outcome { status: 1, error: e.code, line: e.line, message: e.msg, stdout, screen, ..Outcome::new() },
    }
}

// --- C ABI (www/qbnex.js) ----------------------------------------------------

/// Allocates `n` bytes for the page to write into (freed by [`qb_web_free`]).
#[unsafe(no_mangle)]
pub extern "C" fn qb_web_alloc(n: usize) -> *mut u8 {
    // A boxed slice has exactly `n` bytes, which `qb_web_free` relies on
    // (R14: Vec::from_raw_parts needs the original capacity).
    Box::into_raw(vec![0u8; n.max(1)].into_boxed_slice()).cast()
}

/// # Safety
/// `p`/`n` from [`qb_web_alloc`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_web_free(p: *mut u8, n: usize) {
    if !p.is_null() {
        // SAFETY: allocated by qb_web_alloc with this length.
        drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(p, n.max(1))) });
    }
}

/// # Safety
/// `p` readable for `n` bytes (or null with 0).
unsafe fn bytes<'a>(p: *const u8, n: usize) -> &'a [u8] {
    if p.is_null() || n == 0 {
        &[]
    } else {
        // SAFETY: caller contract.
        unsafe { std::slice::from_raw_parts(p, n) }
    }
}

/// Runs a program (QIR text) with `stdin`; returns [`Outcome::status`].
/// Details: [`qb_web_exit_code`], [`qb_web_error`], [`qb_web_error_line`],
/// [`qb_web_text`], [`qb_web_frame`].
///
/// # Safety
/// Both buffers readable for their lengths.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_web_run(src: *const u8, src_len: usize, stdin: *const u8, stdin_len: usize) -> i32 {
    // SAFETY: caller contract.
    let (src, stdin) = unsafe { (bytes(src, src_len), bytes(stdin, stdin_len)) };
    let o = run(&String::from_utf8_lossy(src), stdin);
    let s = o.status;
    *lock(&LAST) = o;
    s
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_web_exit_code() -> i32 {
    lock(&LAST).exit_code
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_web_error() -> i32 {
    lock(&LAST).error
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_web_error_line() -> u32 {
    lock(&LAST).line
}

fn publish(v: Vec<u8>) -> usize {
    let mut b = lock(&BUF);
    *b = v;
    b.len()
}

/// Copies a text of the last run into the result buffer and returns its
/// length ([`qb_web_buf`] = its address): 0 console output, 1 error message,
/// 2 the characters of the final text screen (rows joined by LF).
#[unsafe(no_mangle)]
pub extern "C" fn qb_web_text(kind: i32) -> usize {
    let o = lock(&LAST);
    let v = match kind {
        0 => o.stdout.clone(),
        1 => o.message.clone().into_bytes(),
        2 => o.screen.as_ref().map(|f| f.text.clone().into_bytes()).unwrap_or_default(),
        _ => Vec::new(),
    };
    drop(o);
    publish(v)
}

/// Address of the result buffer (valid until the next query).
#[unsafe(no_mangle)]
pub extern "C" fn qb_web_buf() -> *const u8 {
    lock(&BUF).as_ptr()
}

static FRAME_SIZE: Mutex<(u32, u32)> = Mutex::new((0, 0));

/// Renders a screen into the result buffer as RGBA (`qb_web_buf`, width
/// [`qb_web_frame_width`], height [`qb_web_frame_height`]); returns 1, or 0
/// when there is none. `which`: 0 = the frame `_DISPLAY` just finished (call
/// it from the `qb_web_present` import, also while the program runs), 1 =
/// the screen when the last run stopped.
#[unsafe(no_mangle)]
pub extern "C" fn qb_web_frame(which: i32) -> i32 {
    let f = match which {
        #[cfg(target_arch = "wasm32")]
        0 => {
            let rec = lock(&qb_runtime::gfx::capture::WEB_FRAME).clone();
            screen::parse(&rec).map(|p| screen::render(&p))
        }
        1 => lock(&LAST).screen.clone(),
        _ => None,
    };
    let Some(f) = f else { return 0 };
    *lock(&FRAME_SIZE) = (f.width, f.height);
    publish(f.rgba);
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_web_frame_width() -> u32 {
    lock(&FRAME_SIZE).0
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_web_frame_height() -> u32 {
    lock(&FRAME_SIZE).1
}

/// Console output written so far, while the program runs (for a live
/// console pane; the final text is in [`qb_web_text`] 0).
#[unsafe(no_mangle)]
pub extern "C" fn qb_web_poll_stdout() -> usize {
    let v = lock(&OUT).clone();
    publish(v)
}
