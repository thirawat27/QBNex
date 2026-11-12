//! `SUB _GL` (QB64 OpenGL): an OpenGL context on the program window.
//!
//! The runner calls [`begin_frame`] on its program thread at each display
//! frame, then the program's `SUB _GL` (whose `_gl*` statements are plain
//! OpenGL 1.1 calls resolved through [`proc_address`]), then [`end_frame`],
//! which swaps buffers. While a program draws through OpenGL the window layer
//! stops presenting software frames ([`presenting`]); the software screen is
//! drawn first, behind the OpenGL output (`_GLRENDER _BEHIND`, the default).
//!
//! The context lives on the program thread. Without a window (`QBNEX_HEADLESS`)
//! it is made on a hidden window that is never shown, so `SUB _GL` still runs
//! and [`read_pixels`] can read what it drew.
//!
//! Pure Rust: glutin (WGL on Windows, EGL/GLX loaded at run time on Linux,
//! CGL on macOS). No GL headers or C libraries at build time.

use std::cell::RefCell;
use std::ffi::{c_void, CString};
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use glutin::config::{ConfigTemplateBuilder, GlConfig};
use glutin::context::{ContextAttributesBuilder, GlProfile, NotCurrentGlContext, PossiblyCurrentContext};
use glutin::display::{Display, DisplayApiPreference, GlDisplay};
use glutin::surface::{GlSurface, Surface, SurfaceAttributesBuilder, WindowSurface};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use winit::window::Window;

/// A window and its raw handles, taken on the window's own thread (winit
/// refuses them elsewhere); the handles stay valid while `window` lives.
#[derive(Clone)]
struct Hidden {
    window: Arc<Window>,
    wh: RawWindowHandle,
    dh: RawDisplayHandle,
}
// SAFETY: the raw handles are plain identifiers of a window kept alive by `window`.
unsafe impl Send for Hidden {}

static WINDOW: Mutex<Option<Hidden>> = Mutex::new(None);
static PRESENTING: AtomicBool = AtomicBool::new(false);
/// `_GLRENDER`: 1 = `_BEHIND` (default), 2 = `_ONTOP`, 3 = `_ONLY`
static RENDER: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(1);

/// `_GLRENDER _BEHIND|_ONTOP|_ONLY` (1, 2, 3): where the software screen goes.
pub fn set_render(mode: i32) {
    RENDER.store(mode, Ordering::Relaxed);
}

/// The program window (set by the window layer, on its thread, when it is created).
pub fn set_window(w: Arc<Window>) {
    let (Ok(wh), Ok(dh)) = (w.window_handle().map(|h| h.as_raw()), w.display_handle().map(|h| h.as_raw())) else { return };
    *WINDOW.lock().unwrap_or_else(|e| e.into_inner()) = Some(Hidden { window: w, wh, dh });
}

/// True once frames go to the window through OpenGL (the window layer then
/// leaves the window alone).
pub fn presenting() -> bool {
    PRESENTING.load(Ordering::Relaxed)
}

const GL_COLOR_BUFFER_BIT: u32 = 0x4000;
const GL_DEPTH_BUFFER_BIT: u32 = 0x100;
const GL_PROJECTION: u32 = 0x1701;
const GL_MODELVIEW: u32 = 0x1700;
const GL_UNSIGNED_BYTE: u32 = 0x1401;
const GL_BGRA_EXT: u32 = 0x80E1;
const GL_RGBA: u32 = 0x1908;
const GL_UNPACK_ALIGNMENT: u32 = 0x0CF5;
const GL_PACK_ALIGNMENT: u32 = 0x0D05;

/// The few GL 1.1 entry points the runtime itself needs.
struct Fns {
    viewport: extern "system" fn(i32, i32, i32, i32),
    clear_color: extern "system" fn(f32, f32, f32, f32),
    clear: extern "system" fn(u32),
    matrix_mode: extern "system" fn(u32),
    load_identity: extern "system" fn(),
    raster_pos2f: extern "system" fn(f32, f32),
    pixel_zoom: extern "system" fn(f32, f32),
    pixel_storei: extern "system" fn(u32, i32),
    draw_pixels: extern "system" fn(i32, i32, u32, u32, *const c_void),
    read_pixels: extern "system" fn(i32, i32, i32, i32, u32, u32, *mut c_void),
    finish: extern "system" fn(),
    frustum: extern "system" fn(f64, f64, f64, f64, f64, f64),
    enable: extern "system" fn(u32),
    blend_func: extern "system" fn(u32, u32),
    push_attrib: extern "system" fn(u32),
    pop_attrib: extern "system" fn(),
}

const GL_BLEND: u32 = 0x0BE2;
const GL_SRC_ALPHA: u32 = 0x0302;
const GL_ONE_MINUS_SRC_ALPHA: u32 = 0x0303;
const GL_ALL_ATTRIB_BITS: u32 = 0x000F_FFFF;

const GL_FRAMEBUFFER: u32 = 0x8D40;
const GL_RENDERBUFFER: u32 = 0x8D41;
const GL_RGBA8: u32 = 0x8058;
const GL_DEPTH24_STENCIL8: u32 = 0x88F0;
const GL_COLOR_ATTACHMENT0: u32 = 0x8CE0;
const GL_DEPTH_STENCIL_ATTACHMENT: u32 = 0x821A;

/// Framebuffer objects (GL 3.0 / ARB_framebuffer_object): a hidden window's
/// own pixels are undefined (pixel ownership), so a headless run draws into
/// an offscreen framebuffer of the screen's size instead.
struct Fbo {
    gen_framebuffers: extern "system" fn(i32, *mut u32),
    bind_framebuffer: extern "system" fn(u32, u32),
    gen_renderbuffers: extern "system" fn(i32, *mut u32),
    bind_renderbuffer: extern "system" fn(u32, u32),
    renderbuffer_storage: extern "system" fn(u32, u32, i32, i32),
    framebuffer_renderbuffer: extern "system" fn(u32, u32, u32, u32),
    fb: u32,
    color: u32,
    depth: u32,
}

impl Fbo {
    fn new(display: &Display) -> Result<Fbo, String> {
        let mut f = Fbo {
            gen_framebuffers: load(display, "glGenFramebuffers")?,
            bind_framebuffer: load(display, "glBindFramebuffer")?,
            gen_renderbuffers: load(display, "glGenRenderbuffers")?,
            bind_renderbuffer: load(display, "glBindRenderbuffer")?,
            renderbuffer_storage: load(display, "glRenderbufferStorage")?,
            framebuffer_renderbuffer: load(display, "glFramebufferRenderbuffer")?,
            fb: 0,
            color: 0,
            depth: 0,
        };
        (f.gen_framebuffers)(1, &mut f.fb);
        (f.gen_renderbuffers)(1, &mut f.color);
        (f.gen_renderbuffers)(1, &mut f.depth);
        Ok(f)
    }

    /// Binds the framebuffer with storage for `w` x `h`.
    fn bind(&self, w: u32, h: u32, resize: bool) {
        (self.bind_framebuffer)(GL_FRAMEBUFFER, self.fb);
        if resize {
            (self.bind_renderbuffer)(GL_RENDERBUFFER, self.color);
            (self.renderbuffer_storage)(GL_RENDERBUFFER, GL_RGBA8, w as i32, h as i32);
            (self.bind_renderbuffer)(GL_RENDERBUFFER, self.depth);
            (self.renderbuffer_storage)(GL_RENDERBUFFER, GL_DEPTH24_STENCIL8, w as i32, h as i32);
            (self.bind_renderbuffer)(GL_RENDERBUFFER, 0);
            (self.framebuffer_renderbuffer)(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_RENDERBUFFER, self.color);
            (self.framebuffer_renderbuffer)(GL_FRAMEBUFFER, GL_DEPTH_STENCIL_ATTACHMENT, GL_RENDERBUFFER, self.depth);
        }
    }
}

struct Gl {
    display: Display,
    surface: Surface<WindowSurface>,
    context: PossiblyCurrentContext,
    window: Arc<Window>,
    /// the hidden window of a headless run (its event loop thread keeps it alive)
    own: bool,
    fns: Fns,
    /// drawing target of a hidden window (headless)
    fbo: Option<Fbo>,
    size: (u32, u32),
}

thread_local! {
    static GL: RefCell<Option<Gl>> = const { RefCell::new(None) };
    static IN_FRAME: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// `_GLRENDER _ONTOP`: the software screen of this frame, drawn after SUB _GL
    static ONTOP: RefCell<Option<(Vec<u32>, usize, usize)>> = const { RefCell::new(None) };
}

/// Draws `px` (0xAARRGGBB rows, top first) over the whole drawable with fresh
/// matrices; `blend`: by its alpha (the software screen over GL output).
fn draw_soft(f: &Fns, px: &[u32], sw: usize, sh: usize, w: u32, h: u32, blend: bool) {
    if sw == 0 || sh == 0 || px.len() < sw * sh {
        return;
    }
    // GL rows go bottom-up: flip, then draw from the bottom-left corner
    let mut up = Vec::with_capacity(sw * sh);
    for y in (0..sh).rev() {
        up.extend_from_slice(&px[y * sw..(y + 1) * sw]);
    }
    (f.push_attrib)(GL_ALL_ATTRIB_BITS);
    (f.matrix_mode)(GL_PROJECTION);
    (f.load_identity)();
    (f.matrix_mode)(GL_MODELVIEW);
    (f.load_identity)();
    if blend {
        (f.enable)(GL_BLEND);
        (f.blend_func)(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
    }
    (f.pixel_storei)(GL_UNPACK_ALIGNMENT, 4);
    (f.raster_pos2f)(-1.0, -1.0);
    (f.pixel_zoom)(w as f32 / sw as f32, h as f32 / sh as f32);
    (f.draw_pixels)(sw as i32, sh as i32, GL_BGRA_EXT, GL_UNSIGNED_BYTE, up.as_ptr().cast());
    (f.pixel_zoom)(1.0, 1.0);
    (f.pop_attrib)();
}

/// A hidden window for a run without a visible one: its own event loop on a
/// thread of its own (winit allows that on Windows and X11/Wayland).
fn hidden_window() -> Result<Hidden, String> {
    use std::sync::mpsc;
    use winit::application::ApplicationHandler;
    use winit::event::WindowEvent;
    use winit::event_loop::{ActiveEventLoop, EventLoop};
    use winit::window::WindowId;

    struct H(Option<mpsc::Sender<Result<Hidden, String>>>);
    impl ApplicationHandler for H {
        fn resumed(&mut self, el: &ActiveEventLoop) {
            if let Some(tx) = self.0.take() {
                let attrs = Window::default_attributes()
                    .with_title("QBNex _GL")
                    .with_inner_size(winit::dpi::PhysicalSize::new(640u32, 400u32))
                    .with_visible(false);
                let r = el.create_window(attrs).map_err(|e| e.to_string()).and_then(|w| {
                    // winit hands out the raw handles only on the window's own thread
                    let wh = w.window_handle().map_err(|e| e.to_string())?.as_raw();
                    let dh = w.display_handle().map_err(|e| e.to_string())?.as_raw();
                    Ok(Hidden { window: Arc::new(w), wh, dh })
                });
                let _ = tx.send(r);
            }
        }
        fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
    }

    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("qb-gl-window".into())
        .spawn(move || {
            let mut b = EventLoop::builder();
            #[cfg(windows)]
            {
                use winit::platform::windows::EventLoopBuilderExtWindows;
                b.with_any_thread(true);
            }
            #[cfg(all(unix, not(target_os = "macos")))]
            {
                use winit::platform::x11::EventLoopBuilderExtX11;
                b.with_any_thread(true);
            }
            match b.build() {
                Ok(el) => {
                    let _ = el.run_app(&mut H(Some(tx)));
                }
                Err(e) => {
                    let _ = tx.send(Err(e.to_string()));
                }
            }
        })
        .map_err(|e| e.to_string())?;
    rx.recv().map_err(|e| e.to_string())?
}

fn load<T: Copy>(display: &Display, name: &str) -> Result<T, String> {
    let c = CString::new(name).map_err(|e| e.to_string())?;
    let p = display.get_proc_address(&c);
    if p.is_null() {
        return Err(format!("OpenGL function {name} not found"));
    }
    assert_eq!(std::mem::size_of::<T>(), std::mem::size_of::<*const c_void>());
    // SAFETY: `T` is the extern "system" fn type of this GL entry point.
    Ok(unsafe { std::mem::transmute_copy::<*const c_void, T>(&p) })
}

fn create() -> Result<Gl, String> {
    if std::env::var("QB_GL").is_ok_and(|v| v == "off") {
        return Err("disabled by QB_GL=off".into());
    }
    let shared = WINDOW.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let (hw, own) = match shared {
        Some(w) => (w, false),
        None => (hidden_window()?, true),
    };
    let (window, rwh, rdh) = (hw.window, hw.wh, hw.dh);
    #[cfg(windows)]
    let pref = DisplayApiPreference::Wgl(Some(rwh));
    #[cfg(target_os = "macos")]
    let pref = DisplayApiPreference::Cgl;
    #[cfg(all(unix, not(target_os = "macos")))]
    let pref = DisplayApiPreference::EglThenGlx(Box::new(winit::platform::x11::register_xlib_error_hook));
    // SAFETY: the handles belong to a live window (kept alive in `Gl`).
    let display = unsafe { Display::new(rdh, pref) }.map_err(|e| e.to_string())?;
    let template = ConfigTemplateBuilder::new().compatible_with_native_window(rwh).with_depth_size(24).with_stencil_size(8).build();
    // SAFETY: as above.
    let config = unsafe { display.find_configs(template) }
        .map_err(|e| e.to_string())?
        // a driver's config before the generic (GL 1.1 software) one, then the deepest depth buffer
        .max_by_key(|c| (c.hardware_accelerated(), c.depth_size()))
        .ok_or("no OpenGL configuration")?;
    let attrs = ContextAttributesBuilder::new().with_profile(GlProfile::Compatibility).build(Some(rwh));
    // SAFETY: as above.
    let ctx = unsafe { display.create_context(&config, &attrs) }.map_err(|e| e.to_string())?;
    let size = window.inner_size();
    let (w, h) = (non_zero(size.width), non_zero(size.height));
    let sattrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(rwh, w, h);
    // SAFETY: as above.
    let surface = unsafe { display.create_window_surface(&config, &sattrs) }.map_err(|e| e.to_string())?;
    let context = ctx.make_current(&surface).map_err(|e| e.to_string())?;
    let fns = Fns {
        viewport: load(&display, "glViewport")?,
        clear_color: load(&display, "glClearColor")?,
        clear: load(&display, "glClear")?,
        matrix_mode: load(&display, "glMatrixMode")?,
        load_identity: load(&display, "glLoadIdentity")?,
        raster_pos2f: load(&display, "glRasterPos2f")?,
        pixel_zoom: load(&display, "glPixelZoom")?,
        pixel_storei: load(&display, "glPixelStorei")?,
        draw_pixels: load(&display, "glDrawPixels")?,
        read_pixels: load(&display, "glReadPixels")?,
        finish: load(&display, "glFinish")?,
        frustum: load(&display, "glFrustum")?,
        enable: load(&display, "glEnable")?,
        blend_func: load(&display, "glBlendFunc")?,
        push_attrib: load(&display, "glPushAttrib")?,
        pop_attrib: load(&display, "glPopAttrib")?,
    };
    let fbo = if own { Some(Fbo::new(&display)?) } else { None };
    // (0, 0): the framebuffer gets its storage at the first frame
    let size = if own { (0, 0) } else { (size.width.max(1), size.height.max(1)) };
    Ok(Gl { display, surface, context, window, own, fns, fbo, size })
}

/// A window dimension as the non-zero size GL surfaces need (a minimised window
/// reports 0).
fn non_zero(v: u32) -> NonZeroU32 {
    NonZeroU32::new(v).unwrap_or(NonZeroU32::MIN)
}

/// Starts a display frame for `SUB _GL`: makes the context current (creating
/// it on first use), clears, and draws the software screen `soft` (0xAARRGGBB,
/// `w` x `h`) stretched over the window. Returns the drawable size.
pub fn begin_frame(soft: Option<(&[u32], usize, usize)>) -> Result<(u32, u32), String> {
    GL.with(|g| {
        let mut g = g.borrow_mut();
        if g.is_none() {
            *g = Some(create()?);
        }
        let gl = g.as_mut().expect("created above");
        let (w, h) = if let Some(fbo) = &gl.fbo {
            // a headless run: an offscreen framebuffer of the screen's size
            let (w, h) = soft.map_or((640, 400), |(_, w, h)| (w.max(1) as u32, h.max(1) as u32));
            fbo.bind(w, h, (w, h) != gl.size);
            gl.size = (w, h);
            (w, h)
        } else {
            PRESENTING.store(true, Ordering::Relaxed);
            let size = gl.window.inner_size();
            let (w, h) = (size.width.max(1), size.height.max(1));
            if (w, h) != gl.size {
                gl.surface.resize(&gl.context, non_zero(w), non_zero(h));
                gl.size = (w, h);
            }
            (w, h)
        };
        let f = &gl.fns;
        (f.viewport)(0, 0, w as i32, h as i32);
        (f.clear_color)(0.0, 0.0, 0.0, 1.0);
        (f.clear)(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
        (f.matrix_mode)(GL_PROJECTION);
        (f.load_identity)();
        (f.matrix_mode)(GL_MODELVIEW);
        (f.load_identity)();
        if let Some((px, sw, sh)) = soft {
            match RENDER.load(Ordering::Relaxed) {
                2 => ONTOP.with(|o| *o.borrow_mut() = Some((px.to_vec(), sw, sh))),
                3 => {}
                _ => draw_soft(f, px, sw, sh, w, h, false),
            }
        }
        IN_FRAME.with(|c| c.set(true));
        Ok((w, h))
    })
}

/// Ends the frame begun by [`begin_frame`] and shows it.
pub fn end_frame() {
    IN_FRAME.with(|c| c.set(false));
    GL.with(|g| {
        if let Some(gl) = g.borrow().as_ref() {
            if let Some((px, sw, sh)) = ONTOP.with(|o| o.borrow_mut().take()) {
                draw_soft(&gl.fns, &px, sw, sh, gl.size.0, gl.size.1, true);
            }
            if !gl.own {
                let _ = gl.surface.swap_buffers(&gl.context);
            }
        }
    });
}

/// True when this thread's context draws into an offscreen framebuffer (a
/// hidden window: headless, or no program window); its frames are not shown
/// by OpenGL, so the window layer gets them through [`read_pixels`].
pub fn offscreen() -> bool {
    GL.with(|g| g.borrow().as_ref().is_some_and(|gl| gl.own))
}

/// True between [`begin_frame`] and [`end_frame`] on this thread: `_gl*`
/// statements are allowed (outside `SUB _GL` they raise error 270, as in QB64).
pub fn in_frame() -> bool {
    IN_FRAME.with(|c| c.get())
}

/// Address of an OpenGL function (or `gluPerspective`, which the runtime
/// provides), null when unknown or before the first frame.
pub fn proc_address(name: &str) -> *const c_void {
    if name == "gluPerspective" {
        return glu_perspective as *const c_void;
    }
    GL.with(|g| {
        let g = g.borrow();
        let Some(gl) = g.as_ref() else { return std::ptr::null() };
        match CString::new(name) {
            Ok(c) => gl.display.get_proc_address(&c),
            Err(_) => std::ptr::null(),
        }
    })
}

/// `gluPerspective` (GLU is not part of OpenGL): the glFrustum it stands for.
extern "system" fn glu_perspective(fovy: f64, aspect: f64, near: f64, far: f64) {
    let fh = (fovy / 360.0 * std::f64::consts::PI).tan() * near;
    let fw = fh * aspect;
    GL.with(|g| {
        if let Some(gl) = g.borrow().as_ref() {
            (gl.fns.frustum)(-fw, fw, -fh, fh, near, far);
        }
    });
}

/// The back buffer as 0xAARRGGBB rows, top row first (after the program drew
/// into it, before [`end_frame`]); for the frame capture and tests.
pub fn read_pixels() -> Option<(Vec<u32>, usize, usize)> {
    GL.with(|g| {
        let g = g.borrow();
        let gl = g.as_ref()?;
        let (w, h) = (gl.size.0 as usize, gl.size.1 as usize);
        let mut rgba = vec![0u8; w * h * 4];
        (gl.fns.finish)();
        (gl.fns.pixel_storei)(GL_PACK_ALIGNMENT, 1);
        (gl.fns.read_pixels)(0, 0, w as i32, h as i32, GL_RGBA, GL_UNSIGNED_BYTE, rgba.as_mut_ptr().cast());
        let mut out = vec![0u32; w * h];
        for y in 0..h {
            let src = &rgba[(h - 1 - y) * w * 4..(h - y) * w * 4];
            for x in 0..w {
                let p = &src[x * 4..x * 4 + 4];
                out[y * w + x] = 0xFF00_0000 | (u32::from(p[0]) << 16) | (u32::from(p[1]) << 8) | u32::from(p[2]);
            }
        }
        Some((out, w, h))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A hidden window's context: the software screen goes behind, a GL call
    /// through `proc_address` draws over it, gluPerspective resolves, and
    /// nothing is shown. Skipped (passes) on machines without OpenGL.
    #[test]
    fn hidden_context_draws_software_screen_and_gl_calls() {
        let soft = vec![0xFF00_FF00u32; 64 * 40];
        if let Err(e) = begin_frame(Some((&soft, 64, 40))) {
            eprintln!("no OpenGL here, skipped: {e}");
            return;
        }
        assert!(in_frame());
        assert!(!presenting(), "a hidden window does not take over presentation");
        let (px, w, h) = read_pixels().expect("pixels");
        assert_eq!((w, h), (64, 40), "the offscreen framebuffer has the screen's size");
        assert_eq!(px[(h / 2) * w + w / 2] & 0xFF_FFFF, 0x00FF00, "software screen behind");
        type Scissor = extern "system" fn(i32, i32, i32, i32);
        type Enable = extern "system" fn(u32);
        type ClearColor = extern "system" fn(f32, f32, f32, f32);
        type Clear = extern "system" fn(u32);
        let get = |n: &str| {
            let p = proc_address(n);
            assert!(!p.is_null(), "{n}");
            p
        };
        // SAFETY: GL 1.1 entry points with these signatures.
        let (cc, cl, en, sc): (ClearColor, Clear, Enable, Scissor) = unsafe {
            (
                std::mem::transmute::<*const c_void, ClearColor>(get("glClearColor")),
                std::mem::transmute::<*const c_void, Clear>(get("glClear")),
                std::mem::transmute::<*const c_void, Enable>(get("glEnable")),
                std::mem::transmute::<*const c_void, Scissor>(get("glScissor")),
            )
        };
        sc(0, 0, (w / 2) as i32, h as i32);
        en(0x0C11); // GL_SCISSOR_TEST
        cc(0.0, 0.0, 1.0, 1.0);
        cl(GL_COLOR_BUFFER_BIT);
        let (px, w, h) = read_pixels().expect("pixels");
        assert_eq!(px[(h / 2) * w + w / 4] & 0xFF_FFFF, 0x0000FF, "left half cleared to blue");
        assert_eq!(px[(h / 2) * w + 3 * w / 4] & 0xFF_FFFF, 0x00FF00, "right half kept");
        assert!(!proc_address("gluPerspective").is_null());
        end_frame();
        assert!(!in_frame());

        // _GLRENDER _ONTOP: the software screen goes over the GL output by its alpha
        type Disable = extern "system" fn(u32);
        // SAFETY: GL 1.1 entry point.
        let dis: Disable = unsafe { std::mem::transmute::<*const c_void, Disable>(get("glDisable")) };
        set_render(2);
        let mut soft = vec![0x0000_FF00u32; 64 * 40]; // transparent green
        for y in 0..40 {
            for x in 0..32 {
                soft[y * 64 + x] = 0xFF00_FF00; // opaque green on the left
            }
        }
        begin_frame(Some((&soft, 64, 40))).expect("frame");
        dis(0x0C11);
        cc(0.0, 0.0, 1.0, 1.0);
        cl(GL_COLOR_BUFFER_BIT);
        end_frame(); // a hidden window: no swap, the framebuffer keeps the frame
        let (px, w, h) = read_pixels().expect("pixels");
        assert_eq!(px[(h / 2) * w + w / 4] & 0xFF_FFFF, 0x00FF00, "opaque software pixel on top");
        assert_eq!(px[(h / 2) * w + 3 * w / 4] & 0xFF_FFFF, 0x0000FF, "transparent software pixel shows GL");

        // _GLRENDER _ONLY: no software screen
        set_render(3);
        begin_frame(Some((&soft, 64, 40))).expect("frame");
        let (px, w, h) = read_pixels().expect("pixels");
        assert_eq!(px[(h / 2) * w + w / 4] & 0xFF_FFFF, 0, "nothing behind");
        end_frame();
        set_render(1);
    }
}
