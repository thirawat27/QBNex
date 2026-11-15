//! Streamed sounds (`_SNDOPEN(f$, "STREAM")`, `_SNDPLAYFILE`): a thread per
//! sound decodes the file while it plays, a little ahead of the mixer, into a
//! short buffer of stereo frames at the device rate. Opening reads only the
//! file's headers and its first packets, so the program does not wait for a
//! whole file to decode and memory does not grow with the length of the file.
//!
//! The mixer ([`crate::engine::Engine::render`]) takes frames under the
//! stream's own lock, which the decoder thread holds only to add a chunk;
//! the decoder never takes the engine lock. Seeking (`_SNDSETPOS`, a restart)
//! drops what is buffered and tells the thread where to go on; looping makes
//! it start the file again when it reaches the end.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

use crate::decode::{native_to_f32, stored, Resampler, SampleFormat, SymDecoder};

/// Frames the decoder keeps ready: half a second.
fn buffer_frames(rate: u32) -> usize {
    (rate as usize / 2).max(1024)
}

/// Frames decoded before `open` returns, so playback starts at once.
fn prefill_frames(rate: u32) -> usize {
    (rate as usize / 10).max(256)
}

struct Chunk {
    /// Position (device frames) of the first frame.
    pos: u64,
    /// Interleaved left, right.
    data: Vec<f32>,
    /// Frames already played.
    read: usize,
    /// The pass through the file (a loop starts the next one).
    lap: u64,
}

#[derive(Default)]
struct State {
    chunks: VecDeque<Chunk>,
    buffered: usize,
    /// Bumped by every seek: chunks decoded for an older one are dropped.
    generation: u64,
    seek: Option<u64>,
    looping: bool,
    /// The pass the decoder is in, and the one the mixer read last.
    lap: u64,
    read_lap: u64,
    /// The decoder reached the end of the file (and does not loop).
    ended: bool,
    /// The sound was freed: the thread ends.
    closed: bool,
}

struct Shared {
    state: Mutex<State>,
    wake: Condvar,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// A streamed sound. Dropping it ends its decoder thread.
pub struct Stream {
    shared: Arc<Shared>,
    /// Length in device frames (0 when the file does not tell).
    pub frames: usize,
}

/// What the mixer got from [`Stream::read`].
pub struct Read {
    /// Frames written to the start of the output.
    pub frames: usize,
    /// Position after the last frame read (unchanged when nothing was read).
    pub pos: Option<u64>,
    /// Nothing more will come: the end of the file was played.
    pub ended: bool,
}

/// The decoding side, owned by the thread.
struct Feeder {
    path: PathBuf,
    dec: SymDecoder,
    format: SampleFormat,
    channels: usize,
    device_rate: u32,
    resampler: Option<Resampler>,
    /// Position (device frames) of the next frame produced.
    pos: u64,
    /// The decoder holds a packet not converted yet.
    pending: bool,
    /// Frames produced since the last seek, and whether that seek went to
    /// the start: a loop over a file that gives no frames ends instead of
    /// starting it again forever.
    produced: u64,
    from_start: bool,
    f: Vec<f32>,
    out: Vec<f32>,
}

impl Feeder {
    /// The next chunk of stereo frames; None at the end of the file.
    fn produce(&mut self) -> Option<Chunk> {
        loop {
            if !self.pending && !self.dec.next() {
                // the resampler's last frames, then the end
                let mut tail = Vec::new();
                if let Some(r) = self.resampler.as_mut() {
                    r.finish(&mut tail);
                }
                if tail.is_empty() {
                    return None;
                }
                return Some(self.chunk(tail));
            }
            self.pending = false;
            self.f.clear();
            native_to_f32(self.format, &self.dec.pkt, &mut self.f);
            let frames = match self.resampler.as_mut() {
                Some(r) => {
                    self.out.clear();
                    r.push(&self.f, &mut self.out);
                    std::mem::take(&mut self.out)
                }
                None => std::mem::take(&mut self.f),
            };
            if !frames.is_empty() {
                return Some(self.chunk(frames));
            }
        }
    }

    /// Stereo chunk at `pos` from interleaved frames of the file's channels,
    /// with the precision a decoded sound of the same file has.
    fn chunk(&mut self, frames: Vec<f32>) -> Chunk {
        let ch = self.channels;
        let n = frames.len() / ch;
        let mut data = Vec::with_capacity(n * 2);
        // (resampled frames are stored in the file's format by a decoded sound;
        // frames at the file's rate are the file's own samples)
        let format = if self.resampler.is_some() { self.format } else { SampleFormat::F32 };
        for f in frames.chunks_exact(ch) {
            let l = stored(format, f[0]);
            let r = if ch == 1 { l } else { stored(format, f[1]) };
            data.push(l);
            data.push(r);
        }
        let c = Chunk { pos: self.pos, data, read: 0, lap: 0 };
        self.pos += n as u64;
        self.produced += n as u64;
        c
    }

    /// Goes to device frame `at`.
    fn seek(&mut self, at: u64) {
        self.pending = false;
        self.pos = at;
        self.produced = 0;
        self.from_start = at == 0;
        let file_frame = match self.resampler.as_mut() {
            Some(r) => r.restart(at),
            None => at,
        };
        if file_frame == 0 || !self.dec.seek(file_frame) {
            // from the start of the file, dropping the frames before the target
            if let Some(d) = SymDecoder::open(&self.path) {
                self.dec = d;
                self.dec.skip_frames(file_frame);
            }
        }
    }
}

impl Stream {
    /// Opens `path` for streaming at `device_rate`. None when symphonia cannot
    /// decode it (tracker modules are decoded whole, see `decode`).
    pub fn open(path: &Path, device_rate: u32) -> Option<Stream> {
        let mut dec = SymDecoder::open(path)?;
        if !dec.next() {
            return None;
        }
        let format = dec.format?;
        let channels = dec.channels as usize;
        if channels == 0 || device_rate == 0 {
            return None;
        }
        let resampler = if dec.rate == device_rate { None } else { Some(Resampler::new(channels, dec.rate, device_rate)?) };
        let file_frames = dec.num_frames.or_else(|| SymDecoder::length(path)).unwrap_or(0);
        let frames = match &resampler {
            Some(r) => r.frames_out(file_frames),
            None => file_frames,
        };
        let mut feeder = Feeder { path: path.to_path_buf(), dec, format, channels, device_rate, resampler, pos: 0, pending: true, produced: 0, from_start: true, f: Vec::new(), out: Vec::new() };
        let shared = Arc::new(Shared { state: Mutex::new(State::default()), wake: Condvar::new() });
        {
            let mut st = shared.lock();
            while st.buffered < prefill_frames(device_rate) {
                match feeder.produce() {
                    Some(c) => {
                        st.buffered += c.data.len() / 2;
                        st.chunks.push_back(c);
                    }
                    None => {
                        st.ended = true;
                        break;
                    }
                }
            }
        }
        let s2 = shared.clone();
        std::thread::Builder::new()
            .name("qb-audio-stream".into())
            .stack_size(256 * 1024)
            .spawn(move || run(&s2, feeder))
            .ok()?;
        Some(Stream { shared, frames: usize::try_from(frames).unwrap_or(usize::MAX) })
    }

    /// Copies up to `out.len() / 2` frames into `out` (stereo).
    pub fn read(&self, out: &mut [f32]) -> Read {
        let mut guard = self.shared.lock();
        let st = &mut *guard;
        let want = out.len() / 2;
        let mut n = 0;
        let mut pos = None;
        while n < want {
            let Some(c) = st.chunks.front_mut() else { break };
            let avail = c.data.len() / 2 - c.read;
            let k = avail.min(want - n);
            out[2 * n..2 * (n + k)].copy_from_slice(&c.data[2 * c.read..2 * (c.read + k)]);
            c.read += k;
            n += k;
            pos = Some(c.pos + c.read as u64);
            st.read_lap = c.lap;
            if c.read * 2 >= c.data.len() {
                st.chunks.pop_front();
            }
        }
        st.buffered -= n;
        let ended = st.ended && st.chunks.is_empty();
        if n > 0 {
            self.shared.wake.notify_one();
        }
        Read { frames: n, pos, ended }
    }

    /// Plays from device frame `at` on: what is buffered is dropped.
    pub fn seek(&self, at: u64) {
        let mut st = self.shared.lock();
        st.chunks.clear();
        st.buffered = 0;
        st.generation += 1;
        st.seek = Some(at);
        st.ended = false;
        st.read_lap = st.lap;
        self.shared.wake.notify_one();
    }

    /// `_SNDLOOP` and its end: at the end of the file the decoder starts it again.
    pub fn set_looping(&self, on: bool) {
        let mut st = self.shared.lock();
        st.looping = on;
        if on && st.ended {
            // the end was decoded already: go on from the start after what is buffered
            st.ended = false;
            st.seek_after_buffer();
        } else if !on && (st.lap > st.read_lap || st.seek == Some(u64::MAX)) {
            // the next pass was started already: what was decoded of it is
            // dropped and the sound ends with the pass that plays (`_SNDCLOSE`
            // of a looping sound)
            let lap = st.read_lap;
            st.chunks.retain(|c| c.lap == lap);
            st.buffered = st.chunks.iter().map(|c| c.data.len() / 2 - c.read).sum();
            st.seek = None;
            st.ended = true;
            st.lap = lap;
            st.generation += 1;
        }
        self.shared.wake.notify_one();
    }
}

impl State {
    /// The decoder continues from frame 0 after the frames still buffered.
    fn seek_after_buffer(&mut self) {
        self.seek = Some(u64::MAX);
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        self.shared.lock().closed = true;
        self.shared.wake.notify_one();
    }
}

fn run(shared: &Shared, mut feeder: Feeder) {
    let cap = buffer_frames(feeder.device_rate);
    loop {
        let generation;
        {
            let mut st = shared.lock();
            loop {
                if st.closed {
                    return;
                }
                if let Some(at) = st.seek.take() {
                    // (u64::MAX: a loop that starts again after what is buffered)
                    let at = if at == u64::MAX {
                        st.lap += 1;
                        0
                    } else {
                        at
                    };
                    drop(st);
                    feeder.seek(at);
                    st = shared.lock();
                    continue;
                }
                if !st.ended && st.buffered < cap {
                    generation = st.generation;
                    break;
                }
                st = shared.wake.wait(st).unwrap_or_else(|e| e.into_inner());
            }
        }
        let chunk = feeder.produce();
        let mut st = shared.lock();
        if st.generation != generation || st.seek.is_some() {
            continue; // a seek came while decoding
        }
        match chunk {
            Some(mut c) => {
                c.lap = st.lap;
                st.buffered += c.data.len() / 2;
                st.chunks.push_back(c);
            }
            None if st.looping && !(feeder.from_start && feeder.produced == 0) => st.seek_after_buffer(),
            None => st.ended = true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::load_file;

    fn wav(rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
        let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let block = channels * 2;
        let mut v = b"RIFF".to_vec();
        v.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        v.extend_from_slice(b"WAVEfmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&channels.to_le_bytes());
        v.extend_from_slice(&rate.to_le_bytes());
        v.extend_from_slice(&(rate * block as u32).to_le_bytes());
        v.extend_from_slice(&block.to_le_bytes());
        v.extend_from_slice(&16u16.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&(data.len() as u32).to_le_bytes());
        v.extend_from_slice(&data);
        v
    }

    fn tmp(name: &str, bytes: &[u8]) -> PathBuf {
        let p = std::env::temp_dir().join(format!("qb_stream_{}_{}", std::process::id(), name));
        std::fs::write(&p, bytes).unwrap();
        p
    }

    /// Reads `n` frames, waiting for the decoder; stops early at the end.
    fn read_frames(s: &Stream, n: usize) -> (Vec<f32>, bool) {
        let mut out = Vec::new();
        let mut buf = vec![0f32; 2 * 997];
        for _ in 0..20_000 {
            let want = (n - out.len() / 2).min(997);
            if want == 0 {
                return (out, false);
            }
            let r = s.read(&mut buf[..2 * want]);
            out.extend_from_slice(&buf[..2 * r.frames]);
            if r.ended {
                return (out, true);
            }
            if r.frames == 0 {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        panic!("the stream stalled");
    }

    fn decoded_stereo(p: &Path, rate: u32) -> Vec<f32> {
        let pcm = load_file(p, rate).unwrap();
        (0..pcm.frames).flat_map(|i| <[f32; 2]>::from(pcm.stereo(i))).collect()
    }

    fn samples(n: usize) -> Vec<i16> {
        (0..n).map(|i| ((i * 7919) % 65536) as i16).collect()
    }

    /// A streamed file plays the frames the same file decoded into memory
    /// holds, at the file's rate and resampled.
    #[test]
    fn stream_matches_decoded_sound() {
        let p = tmp("match.wav", &wav(8000, 2, &samples(2 * 30_000)));
        for rate in [8000, 11025, 44100] {
            let s = Stream::open(&p, rate).unwrap();
            let whole = decoded_stereo(&p, rate);
            assert_eq!(s.frames, whole.len() / 2, "{rate}");
            let (got, ended) = read_frames(&s, usize::MAX);
            assert!(ended);
            assert_eq!(got.len(), whole.len(), "{rate}");
            assert!(got == whole, "{rate}");
        }
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn seek_loop_and_end_of_loop() {
        let p = tmp("loop.wav", &wav(8000, 1, &samples(10_000)));
        let whole = decoded_stereo(&p, 8000);
        let s = Stream::open(&p, 8000).unwrap();
        // seek
        s.seek(9000);
        let (got, ended) = read_frames(&s, usize::MAX);
        assert!(ended);
        assert_eq!(got, whole[2 * 9000..]);
        // loop: the start follows the end
        s.seek(0);
        s.set_looping(true);
        let (got, ended) = read_frames(&s, 25_000);
        assert!(!ended);
        assert_eq!(got[..2 * 10_000], whole[..]);
        assert_eq!(got[2 * 10_000..2 * 20_000], whole[..]);
        assert_eq!(got[2 * 20_000..], whole[..2 * 5000]);
        // the loop ends with the pass that plays, also when the next one was decoded
        std::thread::sleep(std::time::Duration::from_millis(50));
        let (_, _) = read_frames(&s, 4990);
        std::thread::sleep(std::time::Duration::from_millis(50));
        s.set_looping(false);
        let (rest, ended) = read_frames(&s, usize::MAX);
        assert!(ended);
        assert_eq!(rest, whole[2 * 9990..]);
        std::fs::remove_file(p).ok();
    }
}
