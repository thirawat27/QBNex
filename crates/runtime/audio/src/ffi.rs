//! The audio runtime's entry points, one per statement or function. The
//! caller raises BASIC errors and manages `_MEM` locks.
//!
//! Functions that can raise a BASIC error return the error number (0 = none).
//! Strings are byte slices and may contain NUL bytes.

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


fn err(r: Result<(), i32>) -> i32 {
    r.err().unwrap_or(0)
}

/// Reads PLAY "=" + VARPTR$ variables from the program's DBLOCK (the 64K
/// data segment VARPTR$ offsets point into). Bytes past its end read as 0.
struct DBlock<'a>(&'a [u8]);
impl VarReader for DBlock<'_> {
    fn read(&self, offset: u16) -> [u8; 10] {
        let mut r = [0u8; 10];
        let at = usize::from(offset).min(self.0.len());
        let src = &self.0[at..self.0.len().min(at + 10)];
        r[..src.len()].copy_from_slice(src);
        r
    }
}

// ---- lifecycle ---------------------------------------------------------------

/// `snd_init`. Opens the default output device; without one, the silent
/// device ([`Device::open_null`]) plays the mix against the clock, so notes
/// take their real time. `QB_AUDIO_DEVICE=null` uses the silent device
/// directly; `QB_AUDIO_DEVICE=none` starts no engine at all (sound
/// statements do nothing and never wait: fast, deterministic test runs).
pub fn qb_snd_init() {
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

/// `snd_un_init`: stops the device and the engine.
pub fn qb_snd_un_init() {
    // Stop the device first and without the engine lock: its callback takes it.
    let dev = DEVICE.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(d) = dev {
        d.close();
    }
    engine().stop();
}

// ---- SOUND / BEEP / PLAY / synth ---------------------------------------------

pub fn qb_snd_sound(frequency: f64, ticks: f64) -> i32 {
    err(engine().sound(frequency, ticks))
}

/// `PLAY s$`; `dblock` is the program's DBLOCK (for `"=" + VARPTR$(v)`).
pub fn qb_snd_play(s: &[u8], dblock: &[u8]) -> i32 {
    err(engine().play(s, &DBlock(dblock)))
}

pub fn qb_snd_play_count() -> i32 {
    engine().play_remaining()
}

/// 1 while PLAY is in music-foreground mode (`MF`, the default): SOUND and
/// PLAY then wait until their notes have played. 0 after `PLAY "MB"` or
/// without an audio engine.
pub fn qb_snd_foreground() -> i32 {
    let e = engine();
    i32::from(e.initialized && e.synth.play.mb == 0.0)
}

/// Notes in the background music queue (the `ON PLAY(n)` event count).
pub fn qb_snd_play_notes() -> i32 {
    engine().play_notes_remaining()
}

pub fn qb_snd_voice(voice: i32) -> i32 {
    err(engine().synth.set_voice(voice))
}

pub fn qb_snd_adsr(attack: f64, decay: f64, sustain: f64, release: f64, voice: i32, passed: i32) -> i32 {
    err(engine().synth.adsr(attack, decay, sustain, release, voice, passed))
}

pub fn qb_snd_wave(spec: &[u8], voice: i32, passed: i32) -> i32 {
    err(engine().synth.wave(spec, voice, passed))
}

// ---- _SND* handles -----------------------------------------------------------

pub fn qb_snd_rate() -> i32 {
    engine().rate as i32
}

fn path_from(name: &[u8]) -> PathBuf {
    // QB64 handed a C string to its decoder: stop at the first NUL.
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

pub fn qb_snd_open(name: &[u8], req: &[u8], passed: i32) -> i32 {
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

pub fn qb_snd_close(h: i32) {
    engine().close(h)
}

/// `_MEM` locks of `_MEMSOUND` blocks whose sound was freed since the last call;
/// the caller invalidates them, so `_MEMEXISTS` of such a block is false.
pub fn qb_snd_take_released_locks() -> Vec<usize> {
    std::mem::take(&mut engine().released_locks)
}

pub fn qb_snd_copy(h: i32) -> i32 {
    engine().copy(h)
}

pub fn qb_snd_play_handle(h: i32) {
    engine().play_sound(h)
}

pub fn qb_snd_play_copy(h: i32, volume: f64, passed: i32) {
    engine().play_copy(h, volume, passed)
}

pub fn qb_snd_play_file(name: &[u8], _sync: i32, volume: f64, passed: i32) {
    let h = open(name, FLAG_STREAM);
    engine().play_file(h, volume, passed)
}

pub fn qb_snd_pause(h: i32) {
    engine().pause(h)
}

pub fn qb_snd_playing(h: i32) -> i32 {
    engine().playing(h)
}

pub fn qb_snd_paused(h: i32) -> i32 {
    engine().paused(h)
}

pub fn qb_snd_vol(h: i32, volume: f32) {
    engine().set_volume(h, volume)
}

pub fn qb_snd_loop(h: i32) {
    engine().loop_sound(h)
}

pub fn qb_snd_bal(h: i32, x: f64, y: f64, z: f64, _channel: i32, passed: i32) {
    engine().balance(h, x, y, z, passed)
}

pub fn qb_snd_len(h: i32) -> f64 {
    engine().length(h)
}

pub fn qb_snd_getpos(h: i32) -> f64 {
    engine().get_pos(h)
}

pub fn qb_snd_setpos(h: i32, seconds: f64) {
    engine().set_pos(h, seconds)
}

pub fn qb_snd_limit(h: i32, seconds: f64) {
    engine().limit(h, seconds)
}

pub fn qb_snd_stop(h: i32) {
    engine().stop_sound(h)
}

pub fn qb_snd_open_raw() -> i32 {
    engine().open_raw()
}

pub fn qb_snd_raw(left: f32, right: f32, h: i32, passed: i32) {
    engine().raw(left, right, h, passed)
}

pub fn qb_snd_raw_done(h: i32, passed: i32) {
    engine().raw_done(h, passed != 0)
}

pub fn qb_snd_raw_len(h: i32, passed: i32) -> f64 {
    engine().raw_len(h, passed != 0)
}

/// `_MEMSOUND` description returned by [`qb_snd_memsound`].
pub struct QbMemSound {
    pub offset: usize,
    pub size: usize,
    pub element_size: usize,
    pub mem_type: i32,
    /// 0 when no lock exists yet: the caller creates one and calls `qb_snd_memsound_set_lock`.
    pub lock_offset: usize,
    pub lock_id: i64,
}

/// The sample buffer of sound `h` (channel `target_channel`), `None` on failure.
pub fn qb_snd_memsound(h: i32, target_channel: i32) -> Option<QbMemSound> {
    let m = engine().mem_sound(h, target_channel)?;
    let (lock_offset, lock_id) = m.lock.unwrap_or((0, 0));
    Some(QbMemSound { offset: m.offset, size: m.size, element_size: m.element_size, mem_type: m.mem_type, lock_offset, lock_id })
}

pub fn qb_snd_memsound_set_lock(h: i32, lock_offset: usize, lock_id: i64) {
    engine().set_mem_lock(h, lock_offset, lock_id)
}
