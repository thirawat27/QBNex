//! C ABI of the audio runtime. Called by the thin shims in
//! `internal/c/libqb/audio_bridge.cpp` (compiled into libqb when
//! DEPENDENCY_AUDIO_OUT is defined), which keep the legacy `sub_*`/`func_*`
//! signatures, raise BASIC errors and manage `_MEM` locks.
//!
//! Functions that can raise a BASIC error return the error number (0 = none).
//! Strings are passed as (pointer, length) and may contain NUL bytes.

use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};

use crate::decode::load_file;
use crate::device::Device;
use crate::engine::{Engine, FLAG_DECODE, FLAG_STREAM, INVALID_SOUND_HANDLE};
use crate::mml::VarReader;

static ENGINE: LazyLock<Mutex<Engine>> = LazyLock::new(|| Mutex::new(Engine::default()));
static DEVICE: Mutex<Option<Device>> = Mutex::new(None);

fn engine() -> MutexGuard<'static, Engine> {
    ENGINE.lock().unwrap_or_else(|e| e.into_inner())
}

unsafe fn bytes<'a>(ptr: *const u8, len: i32) -> &'a [u8] {
    if ptr.is_null() || len <= 0 {
        &[]
    } else {
        // SAFETY: the caller passes a qbs buffer of `len` bytes.
        unsafe { std::slice::from_raw_parts(ptr, len as usize) }
    }
}

fn err(r: Result<(), i32>) -> i32 {
    r.err().unwrap_or(0)
}

/// Reads PLAY "=" + VARPTR$ variables from libqb's DBLOCK.
struct DBlock(*const u8);
impl VarReader for DBlock {
    fn read(&self, offset: u16) -> [u8; 10] {
        let mut r = [0u8; 10];
        if !self.0.is_null() {
            // SAFETY: VARPTR$ offsets point into the 64K DBLOCK owned by libqb
            // (legacy dereferenced dblock + offset the same way).
            unsafe { std::ptr::copy_nonoverlapping(self.0.add(offset as usize), r.as_mut_ptr(), 10) };
        }
        r
    }
}

// ---- lifecycle ---------------------------------------------------------------

/// `snd_init`. Opens the default output device; without one, the silent
/// device ([`Device::open_null`]) plays the mix against the clock, so notes
/// take their real time. `QB_AUDIO_DEVICE=null` uses the silent device
/// directly; `QB_AUDIO_DEVICE=none` starts no engine at all (sound
/// statements do nothing and never wait: fast, deterministic test runs).
#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_init() {
    {
        let e = engine();
        if e.initialized || e.init_failed {
            return;
        }
    }
    let choice = std::env::var("QB_AUDIO_DEVICE").unwrap_or_default();
    let render: crate::device::RenderFn = Arc::new(|buf: &mut [f32]| engine().render(buf));
    let dev = if choice.eq_ignore_ascii_case("none") {
        None
    } else if choice.eq_ignore_ascii_case("null") {
        Device::open_null(render)
    } else {
        Device::open(render.clone()).or_else(|| Device::open_null(render))
    };
    match dev {
        Some(d) => {
            engine().start(d.rate);
            *DEVICE.lock().unwrap_or_else(|e| e.into_inner()) = Some(d);
        }
        None => engine().init_failed = true,
    }
}

/// `snd_un_init`. Afterwards call `qb_snd_take_released_lock` until it returns 0.
#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_un_init() {
    // Stop the device first and without the engine lock: its callback takes it.
    let dev = DEVICE.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(d) = dev {
        d.close();
    }
    engine().stop();
}

/// `snd_mainloop`: disposes finished auto-kill sounds.
#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_mainloop() {
    engine().housekeeping();
}

/// Pops a `_MEMSOUND` lock released by a freed handle (0 = none left).
#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_take_released_lock() -> usize {
    engine().released_locks.pop().unwrap_or(0)
}

// ---- SOUND / BEEP / PLAY / synth ---------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_sound(frequency: f64, ticks: f64) -> i32 {
    err(engine().sound(frequency, ticks))
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_beep() -> i32 {
    err(engine().sound(900.0, 5.0))
}

/// # Safety
/// `s` points to `len` bytes; `dblock` is libqb's DBLOCK (or null).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_snd_play(s: *const u8, len: i32, dblock: *const u8) -> i32 {
    let s = unsafe { bytes(s, len) };
    err(engine().play(s, &DBlock(dblock)))
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_play_count() -> i32 {
    engine().play_remaining()
}

/// 1 while PLAY is in music-foreground mode (`MF`, the default): SOUND and
/// PLAY then wait until their notes have played. 0 after `PLAY "MB"` or
/// without an audio engine.
#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_foreground() -> i32 {
    let e = engine();
    i32::from(e.initialized && e.synth.play.mb == 0.0)
}

/// Notes in the background music queue (the `ON PLAY(n)` event count).
#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_play_notes() -> i32 {
    engine().play_notes_remaining()
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_voice(voice: i32) -> i32 {
    err(engine().synth.set_voice(voice))
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_adsr(attack: f64, decay: f64, sustain: f64, release: f64, voice: i32, passed: i32) -> i32 {
    err(engine().synth.adsr(attack, decay, sustain, release, voice, passed))
}

/// # Safety
/// `spec` points to `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_snd_wave(spec: *const u8, len: i32, voice: i32, passed: i32) -> i32 {
    let s = unsafe { bytes(spec, len) };
    err(engine().synth.wave(s, voice, passed))
}

// ---- _SND* handles -----------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_rate() -> i32 {
    engine().rate as i32
}

fn path_from(name: &[u8]) -> PathBuf {
    // The legacy code handed a C string to miniaudio: stop at the first NUL.
    let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
    PathBuf::from(String::from_utf8_lossy(&name[..end]).into_owned())
}

fn open(name: &[u8], flags: u32) -> i32 {
    let rate = {
        let e = engine();
        if !e.initialized {
            return INVALID_SOUND_HANDLE;
        }
        e.rate
    };
    if name.is_empty() {
        return INVALID_SOUND_HANDLE;
    }
    // Decode without holding the lock so playback continues meanwhile.
    let pcm = load_file(&path_from(name), rate);
    engine().open_decoded(pcm, flags)
}

/// # Safety
/// `name`/`req` point to `name_len`/`req_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_snd_open(name: *const u8, name_len: i32, req: *const u8, req_len: i32, passed: i32) -> i32 {
    let name = unsafe { bytes(name, name_len) };
    let req = unsafe { bytes(req, req_len) };
    let flags = if passed != 0 && !req.is_empty() {
        let up: Vec<u8> = req.to_ascii_uppercase();
        if up.windows(6).any(|w| w == b"STREAM") {
            FLAG_STREAM
        } else {
            0
        }
    } else {
        FLAG_DECODE
    };
    open(name, flags)
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_close(h: i32) {
    engine().close(h)
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_copy(h: i32) -> i32 {
    engine().copy(h)
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_play_handle(h: i32) {
    engine().play_sound(h)
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_play_copy(h: i32, volume: f64, passed: i32) {
    engine().play_copy(h, volume, passed)
}

/// # Safety
/// `name` points to `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_snd_play_file(name: *const u8, len: i32, _sync: i32, volume: f64, passed: i32) {
    let name = unsafe { bytes(name, len) };
    let h = open(name, FLAG_STREAM);
    engine().play_file(h, volume, passed)
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_pause(h: i32) {
    engine().pause(h)
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_playing(h: i32) -> i32 {
    engine().playing(h)
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_paused(h: i32) -> i32 {
    engine().paused(h)
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_vol(h: i32, volume: f32) {
    engine().set_volume(h, volume)
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_loop(h: i32) {
    engine().loop_sound(h)
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_bal(h: i32, x: f64, y: f64, z: f64, _channel: i32, passed: i32) {
    engine().balance(h, x, y, z, passed)
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_len(h: i32) -> f64 {
    engine().length(h)
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_getpos(h: i32) -> f64 {
    engine().get_pos(h)
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_setpos(h: i32, seconds: f64) {
    engine().set_pos(h, seconds)
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_limit(h: i32, seconds: f64) {
    engine().limit(h, seconds)
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_stop(h: i32) {
    engine().stop_sound(h)
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_open_raw() -> i32 {
    engine().open_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_raw(left: f32, right: f32, h: i32, passed: i32) {
    engine().raw(left, right, h, passed)
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_raw_done(h: i32, passed: i32) {
    engine().raw_done(h, passed != 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_raw_len(h: i32, passed: i32) -> f64 {
    engine().raw_len(h, passed != 0)
}

/// `_MEMSOUND` description filled by [`qb_snd_memsound`].
#[repr(C)]
pub struct QbMemSound {
    pub offset: usize,
    pub size: usize,
    pub element_size: usize,
    pub mem_type: i32,
    /// 0 when no lock exists yet: libqb creates one and calls `qb_snd_memsound_set_lock`.
    pub lock_offset: usize,
    pub lock_id: i64,
}

/// Returns 1 and fills `out` on success, 0 on failure.
///
/// # Safety
/// `out` must point to a writable QbMemSound.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_snd_memsound(h: i32, target_channel: i32, out: *mut QbMemSound) -> i32 {
    let Some(m) = engine().mem_sound(h, target_channel) else { return 0 };
    let (lock_offset, lock_id) = m.lock.unwrap_or((0, 0));
    let r = QbMemSound { offset: m.offset, size: m.size, element_size: m.element_size, mem_type: m.mem_type, lock_offset, lock_id };
    // SAFETY: out is valid per the contract.
    unsafe { out.write(r) };
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn qb_snd_memsound_set_lock(h: i32, lock_offset: usize, lock_id: i64) {
    engine().set_mem_lock(h, lock_offset, lock_id)
}

// ---- offline synth (differential tests) --------------------------------------

/// A device-less engine at `rate` for rendering SOUND/PLAY offline.
#[unsafe(no_mangle)]
pub extern "C" fn qb_synth_new(rate: u32) -> *mut Engine {
    let mut e = Engine::default();
    e.start(rate);
    Box::into_raw(Box::new(e))
}

/// # Safety
/// `e` comes from `qb_synth_new` and is not used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_synth_free(e: *mut Engine) {
    if !e.is_null() {
        drop(unsafe { Box::from_raw(e) });
    }
}

/// # Safety
/// `e` comes from `qb_synth_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_synth_sound(e: *mut Engine, frequency: f64, ticks: f64) -> i32 {
    err(unsafe { &mut *e }.sound(frequency, ticks))
}

/// # Safety
/// `e` comes from `qb_synth_new`; `s` points to `len` bytes; `dblock` may be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_synth_play(e: *mut Engine, s: *const u8, len: i32, dblock: *const u8) -> i32 {
    let s = unsafe { bytes(s, len) };
    err(unsafe { &mut *e }.play(s, &DBlock(dblock)))
}

/// # Safety
/// `e` comes from `qb_synth_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_synth_voice(e: *mut Engine, voice: i32) -> i32 {
    err(unsafe { &mut *e }.synth.set_voice(voice))
}

/// # Safety
/// `e` comes from `qb_synth_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_synth_adsr(e: *mut Engine, a: f64, d: f64, s: f64, r: f64, voice: i32, passed: i32) -> i32 {
    err(unsafe { &mut *e }.synth.adsr(a, d, s, r, voice, passed))
}

/// # Safety
/// `e` comes from `qb_synth_new`; `spec` points to `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_synth_wave(e: *mut Engine, spec: *const u8, len: i32, voice: i32, passed: i32) -> i32 {
    let s = unsafe { bytes(spec, len) };
    err(unsafe { &mut *e }.synth.wave(s, voice, passed))
}

/// Moves up to `max_frames` queued stereo frames of `voice` into `out`; returns the count.
///
/// # Safety
/// `e` comes from `qb_synth_new`; `out` has room for `2 * max_frames` floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_synth_drain(e: *mut Engine, voice: i32, out: *mut f32, max_frames: i32) -> i32 {
    let e = unsafe { &mut *e };
    let Some(q) = e.voices.get_mut(voice as usize) else { return 0 };
    let v = q.drain(max_frames.max(0) as usize);
    // SAFETY: room for 2 * max_frames floats per the contract.
    unsafe { std::ptr::copy_nonoverlapping(v.as_ptr(), out, v.len()) };
    (v.len() / 2) as i32
}

/// Frames queued on `voice` (offline engine).
///
/// # Safety
/// `e` comes from `qb_synth_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_synth_queued(e: *mut Engine, voice: i32) -> i32 {
    unsafe { &*e }.voices.get(voice as usize).map(|q| q.frames_remaining() as i32).unwrap_or(0)
}
