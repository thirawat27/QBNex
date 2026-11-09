//! Sound handles and the mixer (legacy `AudioEngine`, `SoundHandle` and the
//! `_SND*` functions in audio.cpp, plus the parts of miniaudio's `ma_sound`
//! they relied on: play/stop/loop/seek, volume, pan, stop time, at-end).
//!
//! All state lives in one [`Engine`] behind one mutex (see [`crate::ffi`]):
//! the BASIC thread, libqb's main-loop thread (`snd_mainloop`) and the device
//! callback ([`Engine::render`]) all lock it, so there is no data race
//! between `_SND*` calls and playback (legacy debt F3).

use std::sync::Arc;

use crate::decode::Pcm;
use crate::mml::{Sink, Synth};
use crate::queue::RawQueue;
use crate::synth::VOICE_COUNT;

pub const INVALID_SOUND_HANDLE: i32 = -1;
pub const QB_TRUE: i32 = -1;
pub const QB_FALSE: i32 = 0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundKind {
    None,
    Static,
    Raw,
}

/// miniaudio flags that legacy stored per handle and that change behaviour.
pub const FLAG_STREAM: u32 = 1;
pub const FLAG_DECODE: u32 = 2;

/// A playing instance of decoded data (`ma_sound`).
pub struct Playback {
    pub data: Arc<Pcm>,
    pub cursor: usize,
    pub playing: bool,
    pub looping: bool,
    pub at_end: bool,
    pub volume: f32,
    pub pan: f32,
    /// `Some(x, y, z)` when `_SNDBAL` set a 3-D position.
    pub position: Option<[f32; 3]>,
    /// Absolute engine time (frames) at which playback stops (`_SNDLIMIT`).
    pub stop_time: Option<u64>,
}

impl Playback {
    fn new(data: Arc<Pcm>) -> Self {
        Playback { data, cursor: 0, playing: false, looping: false, at_end: false, volume: 1.0, pan: 0.0, position: None, stop_time: None }
    }

    /// `ma_sound_start`: a sound at its end restarts from the beginning.
    fn start(&mut self) {
        if self.at_end {
            self.seek(0);
        }
        self.playing = true;
    }

    fn seek(&mut self, frame: usize) {
        self.cursor = frame.min(self.data.frames);
        self.at_end = false;
    }
}

pub struct Handle {
    pub used: bool,
    pub kind: SoundKind,
    pub auto_kill: bool,
    pub flags: u32,
    pub sound: Option<Playback>,
    pub raw: Option<RawQueue>,
    /// `(lock_offset, lock_id)` of the libqb mem lock created by `_MEMSOUND`.
    pub mem_lock: Option<(usize, i64)>,
}

impl Handle {
    fn new() -> Self {
        Handle { used: false, kind: SoundKind::None, auto_kill: false, flags: 0, sound: None, raw: None, mem_lock: None }
    }
}

/// Result of `_MEMSOUND` before libqb attaches a mem lock.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MemSound {
    pub offset: usize,
    pub size: usize,
    pub element_size: usize,
    pub mem_type: i32,
    pub lock: Option<(usize, i64)>,
}

pub struct Engine {
    pub initialized: bool,
    pub init_failed: bool,
    pub rate: u32,
    pub synth: Synth,
    pub voices: Vec<RawQueue>,
    pub handles: Vec<Handle>,
    lowest_free: usize,
    snd_internal: i32,
    snd_internal_raw: i32,
    /// Frames rendered since init (miniaudio engine time).
    pub time: u64,
    /// Mem locks released by freed handles, for libqb to free (`free_mem_lock`).
    pub released_locks: Vec<usize>,
}

struct VoiceSink<'a>(&'a mut [RawQueue]);
impl Sink for VoiceSink<'_> {
    fn send(&mut self, voice: usize, frames: &[i16]) {
        if let Some(q) = self.0.get_mut(voice) {
            q.push_i16(frames);
        }
    }
}

/// miniaudio `ma_pan_mode_pan` on a stereo frame.
fn apply_pan(pan: f32, l: f32, r: f32) -> (f32, f32) {
    let pan = pan.clamp(-1.0, 1.0);
    if pan > 0.0 {
        (l * (1.0 - pan), r + l * pan)
    } else if pan < 0.0 {
        (l + r * -pan, r * (1.0 + pan))
    } else {
        (l, r)
    }
}

/// 3-D position to (gain, pan): inverse distance (min 1, rolloff 1) and the
/// x direction as stereo pan. Approximates miniaudio's spatializer (D-64).
fn spatial(pos: [f32; 3]) -> (f32, f32) {
    let d = (pos[0] * pos[0] + pos[1] * pos[1] + pos[2] * pos[2]).sqrt();
    let gain = if d > 1.0 { 1.0 / d } else { 1.0 };
    let pan = if d > 0.0 { pos[0] / d } else { 0.0 };
    (gain, pan)
}

impl Default for Engine {
    fn default() -> Self {
        Engine {
            initialized: false,
            init_failed: false,
            rate: 0,
            synth: Synth::default(),
            voices: Vec::new(),
            handles: Vec::new(),
            lowest_free: 0,
            snd_internal: INVALID_SOUND_HANDLE,
            snd_internal_raw: INVALID_SOUND_HANDLE,
            time: 0,
            released_locks: Vec::new(),
        }
    }
}

impl Engine {
    /// Brings the engine up at `rate` (the device rate, or any rate offline).
    pub fn start(&mut self, rate: u32) {
        self.rate = rate;
        self.initialized = true;
        self.voices = (0..VOICE_COUNT).map(|_| RawQueue::new(rate)).collect();
        // Reserve handle 0 so user-visible handles stay strictly positive.
        self.snd_internal = self.allocate();
        self.synth.current_voice = 0;
        self.synth.next_voice = 0;
        self.synth.voice_pinned = false;
    }

    /// `snd_un_init`: frees every handle; synth settings are kept, as in legacy.
    pub fn stop(&mut self) {
        if !self.initialized {
            return;
        }
        self.voices.clear();
        for v in self.synth.voices.iter_mut() {
            v.custom_wave = Vec::new();
        }
        for h in 0..self.handles.len() {
            self.free(h as i32);
        }
        self.handles.clear();
        self.lowest_free = 0;
        self.snd_internal = INVALID_SOUND_HANDLE;
        self.snd_internal_raw = INVALID_SOUND_HANDLE;
        self.initialized = false;
    }

    // ---- handle table (AllocateSoundHandle / FreeSoundHandle) --------------

    fn allocate(&mut self) -> i32 {
        if !self.initialized {
            return INVALID_SOUND_HANDLE;
        }
        let n = self.handles.len();
        let mut h = (self.lowest_free..n).find(|&h| !self.handles[h].used).unwrap_or(n);
        if h >= n {
            h = (0..n).find(|&h| !self.handles[h].used).unwrap_or(n);
        }
        if h >= n {
            self.handles.push(Handle::new());
            h = self.handles.len() - 1;
        }
        let hd = &mut self.handles[h];
        *hd = Handle::new();
        hd.used = true;
        self.lowest_free = h + 1;
        h as i32
    }

    fn free(&mut self, handle: i32) {
        if !self.initialized || handle < 0 || handle as usize >= self.handles.len() || !self.handles[handle as usize].used {
            return;
        }
        let h = &mut self.handles[handle as usize];
        h.sound = None;
        h.raw = None;
        if let Some((off, _)) = h.mem_lock.take() {
            self.released_locks.push(off);
        }
        h.used = false;
        h.kind = SoundKind::None;
        if (handle as usize) < self.lowest_free {
            self.lowest_free = handle as usize;
        }
    }

    /// IS_SOUND_HANDLE_VALID
    fn valid(&self, h: i32) -> bool {
        h > 0 && (h as usize) < self.handles.len() && self.handles[h as usize].used && !self.handles[h as usize].auto_kill
    }

    fn valid_kind(&self, h: i32, kind: SoundKind) -> bool {
        self.initialized && self.valid(h) && self.handles[h as usize].kind == kind
    }

    fn sound_mut(&mut self, h: i32) -> Option<&mut Playback> {
        if self.valid_kind(h, SoundKind::Static) {
            self.handles[h as usize].sound.as_mut()
        } else {
            None
        }
    }

    // ---- synth -------------------------------------------------------------

    pub fn sound(&mut self, frequency: f64, ticks: f64) -> Result<(), i32> {
        let (rate, init) = (self.rate, self.initialized);
        let mut sink = VoiceSink(&mut self.voices);
        self.synth.sound(rate, init, frequency, ticks, &mut sink)
    }

    pub fn play(&mut self, s: &[u8], vars: &dyn crate::mml::VarReader) -> Result<(), i32> {
        let (rate, init) = (self.rate, self.initialized);
        let mut sink = VoiceSink(&mut self.voices);
        self.synth.play(rate, init, s, vars, &mut sink)
    }

    /// `PLAY(n)`: frames left to play on the busiest voice.
    pub fn play_remaining(&mut self) -> i32 {
        if !self.initialized {
            return 0;
        }
        let mut m = 0i32;
        for q in &mut self.voices {
            q.force_last();
            m = m.max(q.frames_remaining() as i32);
        }
        m
    }

    // ---- static sounds -----------------------------------------------------

    /// `_SNDOPEN` once the file has been decoded (decoding happens outside the lock).
    pub fn open_decoded(&mut self, pcm: Option<Pcm>, flags: u32) -> i32 {
        if !self.initialized {
            return INVALID_SOUND_HANDLE;
        }
        let h = self.allocate();
        if h < 1 {
            return INVALID_SOUND_HANDLE;
        }
        let hd = &mut self.handles[h as usize];
        hd.kind = SoundKind::Static;
        hd.flags = flags;
        match pcm {
            Some(p) => {
                hd.sound = Some(Playback::new(Arc::new(p)));
                h
            }
            None => {
                hd.used = false;
                INVALID_SOUND_HANDLE
            }
        }
    }

    pub fn close(&mut self, h: i32) {
        if self.initialized && self.valid(h) {
            self.raw_done(h, true);
            self.handles[h as usize].auto_kill = true;
        }
    }

    pub fn copy(&mut self, src: i32) -> i32 {
        if !self.valid_kind(src, SoundKind::Static) {
            return INVALID_SOUND_HANDLE;
        }
        let flags = self.handles[src as usize].flags;
        let data = self.handles[src as usize].sound.as_ref().map(|s| s.data.clone());
        let dst = self.allocate();
        if dst < 1 {
            return INVALID_SOUND_HANDLE;
        }
        // ma_sound_init_copy cannot copy streamed sounds
        match data {
            Some(d) if flags & FLAG_STREAM == 0 => {
                let hd = &mut self.handles[dst as usize];
                hd.kind = SoundKind::Static;
                hd.flags = flags;
                hd.sound = Some(Playback::new(d));
                dst
            }
            _ => {
                self.handles[dst as usize].used = false;
                INVALID_SOUND_HANDLE
            }
        }
    }

    pub fn play_sound(&mut self, h: i32) {
        if let Some(s) = self.sound_mut(h) {
            if s.playing && (!s.looping || s.at_end) {
                s.seek(0);
            }
            s.start();
            s.looping = false;
        }
    }

    pub fn loop_sound(&mut self, h: i32) {
        if let Some(s) = self.sound_mut(h) {
            if s.playing && (!s.looping || s.at_end) {
                s.seek(0);
            }
            s.start();
            s.looping = true;
        }
    }

    pub fn play_copy(&mut self, src: i32, volume: f64, passed: i32) {
        let dst = self.copy(src);
        if dst > 0 {
            if passed != 0 {
                if let Some(s) = self.handles[dst as usize].sound.as_mut() {
                    s.volume = volume as f32;
                }
            }
            self.play_sound(dst);
            self.handles[dst as usize].auto_kill = true;
        }
    }

    /// `_SNDPLAYFILE` after `open_decoded(.., FLAG_STREAM)`.
    pub fn play_file(&mut self, h: i32, volume: f64, passed: i32) {
        if h > 0 {
            if passed & 2 != 0 {
                if let Some(s) = self.handles[h as usize].sound.as_mut() {
                    s.volume = volume as f32;
                }
            }
            self.play_sound(h);
            self.handles[h as usize].auto_kill = true;
        }
    }

    pub fn pause(&mut self, h: i32) {
        if let Some(s) = self.sound_mut(h) {
            s.playing = false;
        }
    }

    pub fn playing(&mut self, h: i32) -> i32 {
        match self.sound_mut(h) {
            Some(s) if s.playing => QB_TRUE,
            _ => QB_FALSE,
        }
    }

    pub fn paused(&mut self, h: i32) -> i32 {
        match self.sound_mut(h) {
            Some(s) if !s.playing && (s.looping || !s.at_end) => QB_TRUE,
            _ => QB_FALSE,
        }
    }

    pub fn set_volume(&mut self, h: i32, v: f32) {
        if self.initialized && self.valid(h) {
            let hd = &mut self.handles[h as usize];
            match hd.kind {
                SoundKind::Static => {
                    if let Some(s) = hd.sound.as_mut() {
                        s.volume = v;
                    }
                }
                SoundKind::Raw => {
                    if let Some(q) = hd.raw.as_mut() {
                        q.volume = v;
                    }
                }
                SoundKind::None => {}
            }
        }
    }

    /// `_SNDBAL h, x, y, z, channel`; passed: 1 = x, 2 = y, 4 = z.
    pub fn balance(&mut self, h: i32, x: f64, y: f64, z: f64, passed: i32) {
        if !(self.initialized && self.valid(h)) {
            return;
        }
        let hd = &mut self.handles[h as usize];
        let spatial_req = passed & 2 != 0 || passed & 4 != 0;
        match hd.kind {
            SoundKind::Static => {
                let Some(s) = hd.sound.as_mut() else { return };
                if spatial_req {
                    let cur = s.position.unwrap_or([0.0; 3]);
                    let nx = if passed & 1 != 0 { x as f32 } else { cur[0] };
                    let ny = if passed & 2 != 0 { y as f32 } else { cur[1] };
                    let nz = if passed & 4 != 0 { z as f32 } else { cur[2] };
                    s.position = Some([nx, ny, nz]);
                } else {
                    s.position = None;
                    s.pan = x as f32;
                }
            }
            SoundKind::Raw => {
                // Raw streams keep only stereo pan (3-D positions map to their pan).
                if let Some(q) = hd.raw.as_mut() {
                    q.pan = if spatial_req { spatial([x as f32, y as f32, z as f32]).1 } else { x as f32 };
                }
            }
            SoundKind::None => {}
        }
    }

    /// `_SNDLEN` in seconds (single precision, as legacy).
    pub fn length(&mut self, h: i32) -> f64 {
        let rate = self.rate;
        match self.sound_mut(h) {
            Some(s) if rate > 0 => (s.data.frames as f32 / rate as f32) as f64,
            _ => 0.0,
        }
    }

    pub fn get_pos(&mut self, h: i32) -> f64 {
        let rate = self.rate;
        match self.sound_mut(h) {
            Some(s) if rate > 0 => (s.cursor as f32 / rate as f32) as f64,
            _ => 0.0,
        }
    }

    pub fn set_pos(&mut self, h: i32, seconds: f64) {
        let rate = self.rate;
        if let Some(s) = self.sound_mut(h) {
            if rate == 0 {
                return;
            }
            let len_secs = s.data.frames as f32 / rate as f32;
            if seconds > len_secs as f64 {
                s.playing = false;
                return;
            }
            let frame = (s.data.frames as f64 * (seconds / len_secs as f64)) as u64;
            s.seek(frame as usize);
        }
    }

    /// `_SNDLIMIT`: miniaudio's stop time is absolute engine time, kept as is.
    pub fn limit(&mut self, h: i32, seconds: f64) {
        let rate = self.rate as u64;
        if let Some(s) = self.sound_mut(h) {
            let ms = (seconds * 1000.0) as u64;
            s.stop_time = Some(ms * rate / 1000);
        }
    }

    pub fn stop_sound(&mut self, h: i32) {
        if let Some(s) = self.sound_mut(h) {
            s.playing = false;
            s.seek(0);
        }
    }

    // ---- raw streams -------------------------------------------------------

    pub fn open_raw(&mut self) -> i32 {
        if !self.initialized {
            return INVALID_SOUND_HANDLE;
        }
        let h = self.allocate();
        if h < 1 {
            return INVALID_SOUND_HANDLE;
        }
        let rate = self.rate;
        let hd = &mut self.handles[h as usize];
        hd.kind = SoundKind::Raw;
        hd.raw = Some(RawQueue::new(rate));
        h
    }

    /// `_SNDRAW l, r, h`; passed: 1 = right, 2 = handle.
    pub fn raw(&mut self, left: f32, mut right: f32, mut h: i32, passed: i32) {
        if passed & 2 == 0 {
            if self.snd_internal_raw < 1 {
                self.snd_internal_raw = self.open_raw();
            }
            h = self.snd_internal_raw;
        }
        if self.valid_kind(h, SoundKind::Raw) {
            if passed & 1 == 0 {
                right = left;
            }
            if let Some(q) = self.handles[h as usize].raw.as_mut() {
                q.push(left, right);
            }
        }
    }

    pub fn raw_done(&mut self, mut h: i32, passed: bool) {
        if !passed {
            h = self.snd_internal_raw;
        }
        if self.valid_kind(h, SoundKind::Raw) {
            if let Some(q) = self.handles[h as usize].raw.as_mut() {
                q.force_last();
            }
        }
    }

    pub fn raw_len(&mut self, mut h: i32, passed: bool) -> f64 {
        if !passed {
            h = self.snd_internal_raw;
        }
        let rate = self.rate;
        if self.valid_kind(h, SoundKind::Raw) {
            if let Some(q) = self.handles[h as usize].raw.as_mut() {
                if q.block_count() > 1 {
                    q.force_last();
                }
                let f = q.frames_remaining();
                return if f == 0 || rate == 0 { 0.0 } else { f as f64 / rate as f64 };
            }
        }
        0.0
    }

    // ---- _MEMSOUND ---------------------------------------------------------

    /// None = error (libqb returns an empty _MEM). `target_channel` must be 0 or 1.
    pub fn mem_sound(&mut self, h: i32, target_channel: i32) -> Option<MemSound> {
        if !self.valid_kind(h, SoundKind::Static) {
            return None;
        }
        let hd = &self.handles[h as usize];
        if hd.flags & FLAG_STREAM != 0 || hd.flags & FLAG_DECODE == 0 {
            return None;
        }
        if target_channel != 0 && target_channel != 1 {
            return None;
        }
        let s = hd.sound.as_ref()?;
        let pcm = &s.data;
        let mut mem_type = if pcm.format == crate::decode::SampleFormat::F32 { 4 } else { 1 };
        if pcm.format == crate::decode::SampleFormat::U8 {
            mem_type |= 2;
        }
        Some(MemSound {
            offset: pcm.data.as_ptr() as usize,
            size: pcm.frames * pcm.bytes_per_frame(),
            element_size: pcm.bytes_per_frame(),
            mem_type,
            lock: hd.mem_lock,
        })
    }

    pub fn set_mem_lock(&mut self, h: i32, offset: usize, id: i64) {
        if h > 0 && (h as usize) < self.handles.len() && self.handles[h as usize].used {
            self.handles[h as usize].mem_lock = Some((offset, id));
        }
    }

    // ---- main loop and mixer -----------------------------------------------

    /// `snd_mainloop`: disposes auto-kill handles that finished.
    pub fn housekeeping(&mut self) {
        if !self.initialized {
            return;
        }
        for h in 0..self.handles.len() {
            let hd = &self.handles[h];
            if !hd.used || !hd.auto_kill {
                continue;
            }
            let done = match hd.kind {
                SoundKind::Static => !hd.sound.as_ref().map(|s| s.playing).unwrap_or(false),
                SoundKind::Raw => hd.raw.as_ref().map(|q| q.frames_remaining() == 0).unwrap_or(true),
                SoundKind::None => false,
            };
            if done {
                self.free(h as i32);
            }
        }
    }

    /// Mixes `out.len() / 2` stereo frames (device callback or offline).
    pub fn render(&mut self, out: &mut [f32]) {
        for x in out.iter_mut() {
            *x = 0.0;
        }
        if !self.initialized {
            return;
        }
        let frames = out.len() / 2;
        for q in self.voices.iter_mut() {
            mix_queue(q, out);
        }
        for h in self.handles.iter_mut() {
            if !h.used {
                continue;
            }
            if let Some(q) = h.raw.as_mut() {
                mix_queue(q, out);
            }
            if let Some(s) = h.sound.as_mut() {
                mix_sound(s, out, self.time);
            }
        }
        self.time += frames as u64;
        for x in out.iter_mut() {
            *x = x.clamp(-1.0, 1.0);
        }
    }
}

fn mix_queue(q: &mut RawQueue, out: &mut [f32]) {
    let (vol, pan) = (q.volume, q.pan);
    for f in out.as_chunks_mut::<2>().0 {
        match q.next_frame() {
            Some((l, r)) => {
                let (l, r) = apply_pan(pan, l * vol, r * vol);
                f[0] += l;
                f[1] += r;
            }
            None => break,
        }
    }
}

fn mix_sound(s: &mut Playback, out: &mut [f32], time: u64) {
    if !s.playing {
        return;
    }
    let (gain, pan) = match s.position {
        Some(p) => spatial(p),
        None => (1.0, s.pan),
    };
    let vol = s.volume * gain;
    for (k, f) in out.as_chunks_mut::<2>().0.iter_mut().enumerate() {
        if let Some(t) = s.stop_time {
            if time + k as u64 >= t {
                s.playing = false;
                s.stop_time = None;
                return;
            }
        }
        if s.cursor >= s.data.frames {
            if s.looping && s.data.frames > 0 {
                s.cursor = 0;
            } else {
                s.playing = false;
                s.at_end = true;
                return;
            }
        }
        let (l, r) = s.data.stereo(s.cursor);
        let (l, r) = apply_pan(pan, l * vol, r * vol);
        f[0] += l;
        f[1] += r;
        s.cursor += 1;
    }
    if s.cursor >= s.data.frames && !s.looping {
        s.playing = false;
        s.at_end = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::SampleFormat;

    fn engine() -> Engine {
        let mut e = Engine::default();
        e.start(1000);
        e
    }

    fn pcm(frames: usize) -> Pcm {
        let s: Vec<f32> = (0..frames).map(|i| i as f32 / 100.0).collect();
        Pcm::from_f32(SampleFormat::F32, 1, &s)
    }

    #[test]
    fn handles_start_at_one_and_recycle() {
        let mut e = engine();
        let a = e.open_decoded(Some(pcm(10)), FLAG_DECODE);
        let b = e.open_decoded(Some(pcm(10)), FLAG_DECODE);
        assert_eq!((a, b), (1, 2));
        assert_eq!(e.open_decoded(None, FLAG_DECODE), INVALID_SOUND_HANDLE);
        e.close(a);
        assert_eq!(e.playing(a), QB_FALSE);
        e.housekeeping();
        assert_eq!(e.open_raw(), 1);
        assert_eq!(e.open_decoded(Some(pcm(1)), 0), 3);
    }

    #[test]
    fn uninitialised_engine_rejects_everything() {
        let mut e = Engine::default();
        assert_eq!(e.open_decoded(Some(pcm(4)), 0), INVALID_SOUND_HANDLE);
        assert_eq!(e.open_raw(), INVALID_SOUND_HANDLE);
        assert_eq!(e.length(1), 0.0);
        assert_eq!(e.play_remaining(), 0);
        assert_eq!(e.sound(440.0, 1.0), Ok(()));
    }

    #[test]
    fn play_render_end_and_restart() {
        let mut e = engine();
        let h = e.open_decoded(Some(pcm(5)), FLAG_DECODE);
        assert_eq!(e.length(h), 0.005f32 as f64);
        e.play_sound(h);
        assert_eq!(e.playing(h), QB_TRUE);
        let mut out = vec![0.0f32; 16];
        e.render(&mut out);
        assert_eq!(out[2], 0.01);
        assert_eq!(out[3], 0.01);
        assert_eq!(out[10], 0.0);
        assert_eq!(e.playing(h), QB_FALSE);
        assert_eq!(e.paused(h), QB_FALSE); // at end
        e.play_sound(h);
        assert_eq!(e.get_pos(h), 0.0);
    }

    #[test]
    fn loop_pause_stop_and_positions() {
        let mut e = engine();
        let h = e.open_decoded(Some(pcm(4)), FLAG_DECODE);
        e.loop_sound(h);
        let mut out = vec![0.0f32; 20];
        e.render(&mut out);
        assert_eq!(out[8], 0.0); // wrapped to frame 0
        assert_eq!(out[10], 0.01);
        assert_eq!(e.playing(h), QB_TRUE);
        e.pause(h);
        assert_eq!(e.paused(h), QB_TRUE);
        e.stop_sound(h);
        assert_eq!(e.get_pos(h), 0.0);
        e.set_pos(h, 0.0025);
        assert_eq!(e.get_pos(h), 0.002f32 as f64);
        e.play_sound(h);
        e.set_pos(h, 1.0); // beyond the end stops
        assert_eq!(e.playing(h), QB_FALSE);
    }

    #[test]
    fn volume_pan_and_limit() {
        let mut e = engine();
        let h = e.open_decoded(Some(pcm(100)), FLAG_DECODE);
        e.set_volume(h, 0.5);
        e.balance(h, 1.0, 0.0, 0.0, 1);
        e.limit(h, 0.003); // stop at engine frame 3
        e.play_sound(h);
        let mut out = vec![0.0f32; 12];
        e.render(&mut out);
        assert_eq!(&out[0..6], &[0.0, 0.0, 0.0, 0.01, 0.0, 0.02]);
        assert_eq!(&out[6..], &[0.0; 6]);
        assert_eq!(e.playing(h), QB_FALSE);
    }

    #[test]
    fn copies_streams_and_memsound() {
        let mut e = engine();
        let s = e.open_decoded(Some(pcm(8)), FLAG_STREAM);
        assert_eq!(e.copy(s), INVALID_SOUND_HANDLE);
        assert_eq!(e.mem_sound(s, 0), None);
        let d = e.open_decoded(Some(pcm(8)), FLAG_DECODE);
        let c = e.copy(d);
        assert!(c > d);
        let m = e.mem_sound(d, 0).unwrap();
        assert_eq!((m.size, m.element_size, m.mem_type, m.lock), (32, 4, 4, None));
        assert_eq!(e.mem_sound(d, 2), None);
        e.set_mem_lock(d, 1234, 7);
        assert_eq!(e.mem_sound(d, 1).unwrap().lock, Some((1234, 7)));
        e.close(d);
        e.housekeeping();
        assert_eq!(e.released_locks, vec![1234]);
    }

    #[test]
    fn raw_stream_default_handle_and_length() {
        let mut e = engine(); // 1000 Hz: 62 frames per block
        for _ in 0..100 {
            e.raw(0.25, 0.0, 0, 0);
        }
        assert_eq!(e.raw_len(0, false), 0.1); // > 1 block: last block forced
        let mut out = vec![0.0f32; 300];
        e.render(&mut out);
        assert_eq!(out[0], 0.25);
        assert_eq!(out[1], 0.25);
        assert_eq!(e.raw_len(0, false), 0.0);
        let h = e.open_raw();
        e.raw(0.5, -0.5, h, 3);
        e.raw_done(h, true);
        e.close(h);
        e.render(&mut out);
        assert_eq!((out[0], out[1]), (0.5, -0.5));
        e.housekeeping();
        assert!(!e.handles[h as usize].used);
    }

    #[test]
    fn synth_voices_are_mixed_and_drain() {
        let mut e = engine();
        e.sound(440.0, 18.2).unwrap();
        assert_eq!(e.play_remaining(), 1000);
        let mut out = vec![0.0f32; 2000];
        e.render(&mut out);
        assert!(out.iter().any(|&x| x != 0.0));
        assert_eq!(e.play_remaining(), 0);
    }

    #[test]
    fn pan_law() {
        assert_eq!(apply_pan(0.5, 1.0, 1.0), (0.5, 1.5));
        assert_eq!(apply_pan(-1.0, 1.0, 1.0), (2.0, 0.0));
        assert_eq!(spatial([2.0, 0.0, 0.0]), (0.5, 1.0));
    }
}
