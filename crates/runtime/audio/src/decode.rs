//! Sound file decoding for `_SNDOPEN` (legacy: miniaudio + dr_wav, dr_flac,
//! dr_mp3, stb_vorbis, libxmp-lite, tinysoundfont, RADv2).
//!
//! - WAV / FLAC / MP3 / Ogg Vorbis: symphonia (pure Rust).
//! - MOD / XM / S3M / IT: xmrs + xmrsplayer (pure Rust), rendered once to
//!   16-bit stereo (song played to its end, loops not repeated, capped at 30 minutes).
//! - MIDI (tinysoundfont + AWE32 ROM) and Reality AdLib Tracker (RADv2) are
//!   dropped: `_SNDOPEN` returns -1 for them (documented deviation).
//!
//! Like the legacy resource manager, data is converted to the device sample
//! rate at load time and kept in the decoder's native sample format so
//! `_MEMSOUND` sees the same element type: WAV keeps its integer format
//! (u8/s16/s24/s32) or f32, everything else is f32 (tracker modules s16).
//! Resampling is linear interpolation (legacy used miniaudio's linear
//! resampler with a 4th-order low-pass filter; output differs slightly when
//! the file rate differs from the device rate), done packet by packet as the
//! file is decoded. A sound decoded into memory is limited to
//! [`MAX_DECODED_SECONDS`] and [`MAX_DECODED_BYTES`]; a streamed one
//! ([`crate::stream`]) is decoded while it plays and has no limit.

use std::fs::File;
use std::path::Path;

use symphonia::core::audio::GenericAudioBufferRef;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleFormat {
    U8,
    S16,
    S24,
    S32,
    F32,
}

impl SampleFormat {
    pub fn bytes(self) -> usize {
        match self {
            SampleFormat::U8 => 1,
            SampleFormat::S16 => 2,
            SampleFormat::S24 => 3,
            SampleFormat::S32 | SampleFormat::F32 => 4,
        }
    }
}

/// Decoded PCM: interleaved, native sample format, at the device rate.
#[derive(Debug)]
pub struct Pcm {
    pub format: SampleFormat,
    pub channels: u16,
    pub frames: usize,
    pub data: Vec<u8>,
}

impl Pcm {
    pub fn from_f32(format: SampleFormat, channels: u16, samples: &[f32]) -> Pcm {
        let mut data = Vec::with_capacity(samples.len() * format.bytes());
        encode_samples(format, samples, &mut data);
        Pcm { format, channels, frames: samples.len() / channels.max(1) as usize, data }
    }

    pub fn sample(&self, index: usize) -> f32 {
        sample_at(self.format, &self.data, index)
    }

    /// Frame `i` as stereo (mono is duplicated, extra channels are dropped).
    pub fn stereo(&self, i: usize) -> (f32, f32) {
        let ch = self.channels as usize;
        if ch == 1 {
            let s = self.sample(i);
            (s, s)
        } else {
            (self.sample(i * ch), self.sample(i * ch + 1))
        }
    }

    pub fn to_f32(&self) -> Vec<f32> {
        (0..self.frames * self.channels as usize).map(|i| self.sample(i)).collect()
    }

    pub fn bytes_per_frame(&self) -> usize {
        self.format.bytes() * self.channels as usize
    }
}

/// Appends `samples` to `data` in `format` (the conversion of [`Pcm::from_f32`]).
fn encode_samples(format: SampleFormat, samples: &[f32], data: &mut Vec<u8>) {
    for &s in samples {
        let c = s.clamp(-1.0, 1.0);
        match format {
            SampleFormat::U8 => data.push(((c * 127.0).round() as i32 + 128).clamp(0, 255) as u8),
            SampleFormat::S16 => data.extend_from_slice(&((c * 32767.0).round() as i16).to_le_bytes()),
            SampleFormat::S24 => {
                let v = (c as f64 * 8388607.0).round() as i32;
                data.extend_from_slice(&v.to_le_bytes()[0..3]);
            }
            SampleFormat::S32 => data.extend_from_slice(&((c as f64 * 2147483647.0).round() as i32).to_le_bytes()),
            SampleFormat::F32 => data.extend_from_slice(&s.to_le_bytes()),
        }
    }
}

/// Sample `index` of interleaved `format` data, as f32 in -1..1.
fn sample_at(format: SampleFormat, b: &[u8], index: usize) -> f32 {
    match format {
        SampleFormat::U8 => (b[index] as f32 - 128.0) / 128.0,
        SampleFormat::S16 => i16::from_le_bytes([b[2 * index], b[2 * index + 1]]) as f32 / 32768.0,
        SampleFormat::S24 => {
            let i = 3 * index;
            let v = i32::from_le_bytes([0, b[i], b[i + 1], b[i + 2]]) >> 8;
            v as f32 / 8388608.0
        }
        SampleFormat::S32 => i32::from_le_bytes(le4(&b[4 * index..])) as f32 / 2147483648.0,
        SampleFormat::F32 => f32::from_le_bytes(le4(&b[4 * index..])),
    }
}

/// A sample as it plays once stored in `format`: what a sound decoded into
/// memory holds, so a streamed sound sounds the same.
pub(crate) fn stored(format: SampleFormat, s: f32) -> f32 {
    if format == SampleFormat::F32 {
        return s;
    }
    let mut b = Vec::with_capacity(4);
    encode_samples(format, &[s], &mut b);
    sample_at(format, &b, 0)
}

/// Appends the samples of interleaved `format` bytes to `out` as f32.
pub(crate) fn native_to_f32(format: SampleFormat, b: &[u8], out: &mut Vec<f32>) {
    let n = b.len() / format.bytes();
    out.extend((0..n).map(|i| sample_at(format, b, i)));
}

/// Sample rates accepted for resampling; a file claiming a rate outside this
/// range is rejected (a rate of 1 would ask for a buffer of hundreds of GB).
const RESAMPLE_RATES: core::ops::RangeInclusive<u32> = 1000..=768_000;

/// Longest sound decoded into memory (`_SNDOPEN` without "STREAM"), in
/// seconds at the device rate: a longer file fails to open (-1). Streamed
/// sounds ("STREAM", `_SNDPLAYFILE`) have no limit.
pub const MAX_DECODED_SECONDS: u64 = 4 * 60 * 60;
/// Most bytes of samples a sound decoded into memory may hold (a file with
/// many channels reaches it before [`MAX_DECODED_SECONDS`]).
pub const MAX_DECODED_BYTES: u64 = 4 << 30;

/// Linear-interpolation resampler for interleaved f32 that takes its input
/// in pieces: output frame `o` is input position `o * from / to`, between
/// the two input frames around it, exactly as when the whole input is
/// resampled at once (`floor(frames * to / from)` frames in all). Only the
/// input frames still needed are kept.
pub struct Resampler {
    ch: usize,
    from: u64,
    to: u64,
    step: f64,
    /// Input frames from frame `base` on.
    buf: Vec<f32>,
    base: u64,
    /// Input frames pushed so far (counted from 0, also after `restart`).
    total: u64,
    /// The next output frame.
    next: u64,
}

impl Resampler {
    /// None when a rate is outside [`RESAMPLE_RATES`].
    pub fn new(channels: usize, from: u32, to: u32) -> Option<Self> {
        if channels == 0 || !RESAMPLE_RATES.contains(&from) || !RESAMPLE_RATES.contains(&to) {
            return None;
        }
        Some(Resampler { ch: channels, from: from.into(), to: to.into(), step: from as f64 / to as f64, buf: Vec::new(), base: 0, total: 0, next: 0 })
    }

    /// Output frames for `n` input frames.
    pub fn frames_out(&self, n: u64) -> u64 {
        n.saturating_mul(self.to) / self.from
    }

    /// Takes the next input frames and appends the output frames they complete.
    pub fn push(&mut self, input: &[f32], out: &mut Vec<f32>) {
        self.buf.extend_from_slice(input);
        self.total += (input.len() / self.ch) as u64;
        // (no more frames than the input so far has: the count only grows)
        let limit = self.frames_out(self.total);
        while self.next < limit {
            let pos = self.next as f64 * self.step;
            let i0 = pos as u64;
            if i0 + 1 >= self.total {
                break;
            }
            self.emit(pos, i0, i0 + 1, out);
            self.next += 1;
        }
        let keep = ((self.next as f64 * self.step) as u64).min(self.total.saturating_sub(1));
        if keep > self.base {
            let n = ((keep - self.base) as usize * self.ch).min(self.buf.len());
            self.buf.drain(..n);
            self.base = keep;
        }
    }

    /// The input has ended: appends the last output frames (the final input
    /// frame repeats past the end).
    pub fn finish(&mut self, out: &mut Vec<f32>) {
        if self.total > self.base {
            let last = self.total - 1;
            let limit = self.frames_out(self.total);
            while self.next < limit {
                let pos = self.next as f64 * self.step;
                let i0 = (pos as u64).min(last);
                self.emit(pos, i0, (i0 + 1).min(last), out);
                self.next += 1;
            }
        }
        self.buf.clear();
        self.base = self.total;
    }

    /// Continues with output frame `at` (a seek): returns the input frame the
    /// next input must start at.
    pub fn restart(&mut self, at: u64) -> u64 {
        let i = (at as f64 * self.step) as u64;
        self.next = at;
        self.base = i;
        self.total = i;
        self.buf.clear();
        i
    }

    fn emit(&self, pos: f64, i0: u64, i1: u64, out: &mut Vec<f32>) {
        let frac = (pos - i0 as f64) as f32;
        let a = (i0 - self.base) as usize * self.ch;
        let b = (i1 - self.base) as usize * self.ch;
        for c in 0..self.ch {
            let (x, y) = (self.buf[a + c], self.buf[b + c]);
            out.push(x + (y - x) * frac);
        }
    }
}

/// Resamples a whole buffer (see [`Resampler`]). None when a rate is outside
/// [`RESAMPLE_RATES`], the result would be over [`MAX_DECODED_SECONDS`] or
/// it cannot be allocated.
pub fn resample(input: &[f32], channels: usize, from: u32, to: u32) -> Option<Vec<f32>> {
    if from == to || channels == 0 {
        return Some(input.to_vec());
    }
    let mut r = Resampler::new(channels, from, to)?;
    let frames = r.frames_out((input.len() / channels) as u64);
    if frames > MAX_DECODED_SECONDS * u64::from(to) {
        return None;
    }
    let mut out = Vec::new();
    out.try_reserve_exact(usize::try_from(frames).ok()?.checked_mul(channels)?).ok()?;
    r.push(input, &mut out);
    r.finish(&mut out);
    Some(out)
}

/// The samples of a sound decoded into memory, growing up to its limit
/// ([`MAX_DECODED_SECONDS`], [`MAX_DECODED_BYTES`]) with fallible
/// allocations: a file too long or too large for memory fails to open
/// instead of aborting the program.
struct Collector {
    format: SampleFormat,
    data: Vec<u8>,
    max_bytes: u64,
}

impl Collector {
    fn new(format: SampleFormat, channels: u16, rate: u32) -> Self {
        let frame = (format.bytes() * channels.max(1) as usize) as u64;
        let max_bytes = (MAX_DECODED_SECONDS * u64::from(rate)).saturating_mul(frame).min(MAX_DECODED_BYTES);
        Collector { format, data: Vec::new(), max_bytes }
    }

    fn room(&mut self, add: usize) -> Option<()> {
        if (self.data.len() + add) as u64 > self.max_bytes {
            return None;
        }
        self.data.try_reserve(add).ok()
    }

    fn push_native(&mut self, b: &[u8]) -> Option<()> {
        self.room(b.len())?;
        self.data.extend_from_slice(b);
        Some(())
    }

    fn push_f32(&mut self, s: &[f32]) -> Option<()> {
        self.room(s.len() * self.format.bytes())?;
        encode_samples(self.format, s, &mut self.data);
        Some(())
    }
}

/// A symphonia reader and decoder for one file, packet by packet. The sample
/// format, channels and rate are those of the first decoded packet.
pub(crate) struct SymDecoder {
    reader: Box<dyn symphonia::core::formats::FormatReader>,
    decoder: Box<dyn symphonia::core::codecs::audio::AudioDecoder>,
    track_id: u32,
    is_pcm: bool,
    pub format: Option<SampleFormat>,
    pub channels: u16,
    pub rate: u32,
    /// Frames of the track when the container says (file rate).
    pub num_frames: Option<u64>,
    /// The last decoded packet, interleaved, in `format`.
    pub pkt: Vec<u8>,
    /// Frames of the next packets to drop (an accurate seek lands before its target).
    skip: u64,
    /// Time base of the track's timestamps (numerator, denominator), if not 1 / rate.
    time_base: Option<(u64, u64)>,
}

/// Timestamp units of `tb` (seconds per unit) to frames at `rate`, and back.
fn ts_to_frames(tb: Option<(u64, u64)>, ts: u64, rate: u32) -> u64 {
    match tb {
        Some((n, d)) => (u128::from(ts) * u128::from(n) * u128::from(rate) / u128::from(d)).min(u128::from(u64::MAX)) as u64,
        None => ts,
    }
}
fn frames_to_ts(tb: Option<(u64, u64)>, frames: u64, rate: u32) -> u64 {
    match tb {
        Some((n, d)) if rate > 0 => (u128::from(frames) * u128::from(d) / (u128::from(n) * u128::from(rate))).min(u128::from(u64::MAX)) as u64,
        _ => frames,
    }
}

fn track_time_base(track: &symphonia::core::formats::Track, rate: u32) -> Option<(u64, u64)> {
    let tb = track.time_base?;
    let (n, d) = (u64::from(tb.numer.get()), u64::from(tb.denom.get()));
    (!(n == 1 && d == u64::from(rate))).then_some((n, d))
}

fn probe(path: &Path) -> Option<Box<dyn symphonia::core::formats::FormatReader>> {
    let file = File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    symphonia::default::get_probe()
        .probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())
        .ok()
}

impl SymDecoder {
    pub fn open(path: &Path) -> Option<Self> {
        let reader = probe(path)?;
        let track = reader.default_track(TrackType::Audio)?;
        let track_id = track.id;
        let num_frames = track.num_frames;
        let params = track.codec_params.as_ref()?.audio()?.clone();
        let time_base = track_time_base(track, params.sample_rate.unwrap_or(0));
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(&params, &AudioDecoderOptions::default())
            .ok()?;
        let is_pcm = symphonia::default::get_codecs()
            .get_audio_decoder(params.codec)
            .map(|d| d.codec.info.short_name.starts_with("pcm"))
            .unwrap_or(false);
        Some(SymDecoder { reader, decoder, track_id, is_pcm, format: None, channels: 0, rate: params.sample_rate.unwrap_or(0), num_frames, pkt: Vec::new(), skip: 0, time_base })
    }

    /// Decodes the next packet into `pkt`. False at the end of the track, at
    /// an error that ends it, or when the channel count changes.
    pub fn next(&mut self) -> bool {
        loop {
            let Ok(Some(packet)) = self.reader.next_packet() else { return false };
            if packet.track_id != self.track_id {
                continue;
            }
            let buf = match self.decoder.decode(&packet) {
                Ok(b) => b,
                Err(SymError::DecodeError(_)) => continue,
                Err(_) => return false,
            };
            let spec = buf.spec().clone();
            if self.format.is_none() {
                self.channels = spec.channels().count() as u16;
                self.rate = spec.rate();
                self.format = Some(if self.is_pcm {
                    match &buf {
                        GenericAudioBufferRef::U8(_) => SampleFormat::U8,
                        GenericAudioBufferRef::S16(_) => SampleFormat::S16,
                        GenericAudioBufferRef::S24(_) => SampleFormat::S24,
                        GenericAudioBufferRef::S32(_) => SampleFormat::S32,
                        _ => SampleFormat::F32,
                    }
                } else {
                    SampleFormat::F32
                });
            }
            if spec.channels().count() as u16 != self.channels {
                return false;
            }
            let Some(fmt) = self.format else { return false };
            self.pkt.clear();
            let native = &mut self.pkt;
            match fmt {
                SampleFormat::U8 => {
                    let mut v: Vec<u8> = Vec::new();
                    buf.copy_to_vec_interleaved(&mut v);
                    native.extend_from_slice(&v);
                }
                SampleFormat::S16 => {
                    let mut v: Vec<i16> = Vec::new();
                    buf.copy_to_vec_interleaved(&mut v);
                    for s in &v {
                        native.extend_from_slice(&s.to_le_bytes());
                    }
                }
                SampleFormat::S24 => {
                    let mut v: Vec<i32> = Vec::new();
                    buf.copy_to_vec_interleaved(&mut v);
                    for s in &v {
                        native.extend_from_slice(&s.to_le_bytes()[1..4]);
                    }
                }
                SampleFormat::S32 => {
                    let mut v: Vec<i32> = Vec::new();
                    buf.copy_to_vec_interleaved(&mut v);
                    for s in &v {
                        native.extend_from_slice(&s.to_le_bytes());
                    }
                }
                SampleFormat::F32 => {
                    let mut v: Vec<f32> = Vec::new();
                    buf.copy_to_vec_interleaved(&mut v);
                    for s in &v {
                        native.extend_from_slice(&s.to_le_bytes());
                    }
                }
            }
            if self.skip > 0 {
                let frame = fmt.bytes() * self.channels.max(1) as usize;
                let drop = (self.skip as usize).min(self.pkt.len() / frame);
                self.pkt.drain(..drop * frame);
                self.skip -= drop as u64;
                if self.pkt.is_empty() {
                    continue;
                }
            }
            return true;
        }
    }

    /// Drops the next `n` decoded frames.
    pub fn skip_frames(&mut self, n: u64) {
        self.skip = n;
    }

    /// Goes to file frame `frame` (the next packets start there). False when
    /// the file cannot seek: the caller decodes from the start and skips.
    pub fn seek(&mut self, frame: u64) -> bool {
        use symphonia::core::formats::{SeekMode, SeekTo};
        use symphonia::core::units::Timestamp;
        let Ok(ts) = i64::try_from(frames_to_ts(self.time_base, frame, self.rate)) else { return false };
        let to = SeekTo::Timestamp { ts: Timestamp::new(ts), track_id: self.track_id };
        match self.reader.seek(SeekMode::Accurate, to) {
            Ok(at) => {
                self.decoder.reset();
                let behind = at.required_ts.get().saturating_sub(at.actual_ts.get()).max(0) as u64;
                self.skip = ts_to_frames(self.time_base, behind, self.rate);
                true
            }
            Err(_) => false,
        }
    }

    /// Frames of the track (file rate): the container's count, else the
    /// durations of its packets (read without decoding them).
    pub fn length(path: &Path) -> Option<u64> {
        let mut reader = probe(path)?;
        let track = reader.default_track(TrackType::Audio)?;
        if let Some(n) = track.num_frames {
            return Some(n);
        }
        let id = track.id;
        let rate = track.codec_params.as_ref()?.audio()?.sample_rate.unwrap_or(0);
        let tb = track_time_base(track, rate);
        let mut n = 0u64;
        while let Ok(Some(p)) = reader.next_packet() {
            if p.track_id == id {
                n = n.saturating_add(p.dur.get());
            }
        }
        Some(ts_to_frames(tb, n, rate))
    }
}

fn decode_symphonia(path: &Path, device_rate: u32) -> Option<Pcm> {
    let mut d = SymDecoder::open(path)?;
    let mut col: Option<Collector> = None;
    let mut rs: Option<Resampler> = None;
    let mut any = false;
    let (mut f, mut out) = (Vec::new(), Vec::new());
    // Decoded packet by packet: resampled as it comes (no copies of the whole sound)
    // and stored in the file's sample format.
    while d.next() {
        let fmt = d.format?;
        let c = col.get_or_insert_with(|| Collector::new(fmt, d.channels, device_rate));
        any |= !d.pkt.is_empty();
        if d.rate == device_rate {
            c.push_native(&d.pkt)?;
            continue;
        }
        if rs.is_none() {
            rs = Some(Resampler::new(d.channels as usize, d.rate, device_rate)?);
        }
        let r = rs.as_mut()?;
        f.clear();
        native_to_f32(fmt, &d.pkt, &mut f);
        out.clear();
        r.push(&f, &mut out);
        c.push_f32(&out)?;
    }
    let format = d.format?;
    let mut c = col?;
    if d.channels == 0 || !any {
        return None;
    }
    if let Some(r) = rs.as_mut() {
        out.clear();
        r.finish(&mut out);
        c.push_f32(&out)?;
    }
    let channels = d.channels;
    Some(Pcm { format, channels, frames: c.data.len() / (format.bytes() * channels as usize), data: c.data })
}

const MODULE_EXTENSIONS: [&str; 4] = ["mod", "xm", "s3m", "it"];
const MODULE_MAX_SECONDS: usize = 30 * 60;

fn decode_module(path: &Path, device_rate: u32) -> Option<Pcm> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    if !MODULE_EXTENSIONS.contains(&ext.as_str()) || device_rate == 0 {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let module = xmrs::prelude::Module::load(&bytes).ok()?;
    let mut player = xmrsplayer::prelude::XmrsPlayer::new(&module, device_rate, 0);
    player.set_max_loop_count(1);
    let cap = MODULE_MAX_SECONDS * device_rate as usize * 2;
    let mut data = Vec::new();
    for s in player.by_ref().take(cap) {
        data.extend_from_slice(&s.to_le_bytes());
    }
    if data.len() < 4 {
        return None;
    }
    let frames = data.len() / 4;
    data.truncate(frames * 4);
    Some(Pcm { format: SampleFormat::S16, channels: 2, frames, data })
}

/// Loads a sound file and converts it to `device_rate`. None = unsupported or unreadable.
pub fn load_file(path: &Path, device_rate: u32) -> Option<Pcm> {
    decode_symphonia(path, device_rate).or_else(|| decode_module(path, device_rate))
}

/// First 4 bytes of `b` (callers index inside the sample buffer).
fn le4(b: &[u8]) -> [u8; 4] {
    [b[0], b[1], b[2], b[3]]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(rate: u32, channels: u16, bits: u16, samples: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        let block = channels * bits / 8;
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&(36 + samples.len() as u32).to_le_bytes());
        v.extend_from_slice(b"WAVEfmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&channels.to_le_bytes());
        v.extend_from_slice(&rate.to_le_bytes());
        v.extend_from_slice(&(rate * block as u32).to_le_bytes());
        v.extend_from_slice(&block.to_le_bytes());
        v.extend_from_slice(&bits.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&(samples.len() as u32).to_le_bytes());
        v.extend_from_slice(samples);
        v
    }

    fn tmp(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("qb_audio_{}_{}", std::process::id(), name));
        std::fs::write(&p, bytes).unwrap();
        p
    }

    #[test]
    fn wav_s16_kept_native_at_same_rate() {
        let samples: Vec<u8> = [0i16, 1000, -1000, 32767].iter().flat_map(|s| s.to_le_bytes()).collect();
        let p = tmp("s16.wav", &wav(8000, 2, 16, &samples));
        let pcm = load_file(&p, 8000).expect("decode");
        assert_eq!(pcm.format, SampleFormat::S16);
        assert_eq!(pcm.channels, 2);
        assert_eq!(pcm.frames, 2);
        assert_eq!(pcm.data, samples);
        assert_eq!(pcm.stereo(1), (-1000.0 / 32768.0, 32767.0 / 32768.0));
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn wav_u8_mono_resampled() {
        let p = tmp("u8.wav", &wav(4000, 1, 8, &[128, 255, 0, 128]));
        let pcm = load_file(&p, 8000).expect("decode");
        assert_eq!(pcm.format, SampleFormat::U8);
        assert_eq!(pcm.channels, 1);
        assert_eq!(pcm.frames, 8);
        assert_eq!(pcm.stereo(0).0, pcm.stereo(0).1);
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn missing_and_garbage_files_fail() {
        assert!(load_file(Path::new("definitely_missing_qb_runtime.wav"), 48000).is_none());
        let p = tmp("junk.wav", b"not a sound file at all");
        assert!(load_file(&p, 48000).is_none());
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn decoded_size_is_capped() {
        let c = Collector::new(SampleFormat::S16, 2, 48000);
        assert_eq!(c.max_bytes, MAX_DECODED_SECONDS * 48000 * 4);
        assert_eq!(Collector::new(SampleFormat::F32, 64, 768_000).max_bytes, MAX_DECODED_BYTES);
        let mut c = Collector { format: SampleFormat::S16, data: Vec::new(), max_bytes: 6 };
        assert!(c.push_native(&[1, 2, 3, 4]).is_some());
        assert!(c.push_f32(&[0.5]).is_some());
        assert!(c.push_f32(&[0.5]).is_none());
        assert_eq!(c.data.len(), 6);
    }

    /// Resampling in pieces gives what resampling at once gives.
    #[test]
    fn resampler_in_pieces() {
        let input: Vec<f32> = (0..2 * 1001).map(|i| (i as f32 * 0.37).sin()).collect();
        for (from, to) in [(8000, 44100), (44100, 8000), (22050, 48000)] {
            let whole = resample(&input, 2, from, to).unwrap();
            let mut r = Resampler::new(2, from, to).unwrap();
            let mut out = Vec::new();
            for piece in input.chunks(2 * 37) {
                r.push(piece, &mut out);
            }
            r.finish(&mut out);
            assert_eq!(out, whole, "{from} {to}");
        }
    }

    #[test]
    fn linear_resample() {
        assert_eq!(resample(&[0.0, 1.0], 1, 1000, 2000), Some(vec![0.0, 0.5, 1.0, 1.0]));
        assert_eq!(resample(&[0.0, 1.0, 2.0, 3.0], 1, 2000, 1000), Some(vec![0.0, 2.0]));
    }

    #[test]
    fn implausible_rates_are_rejected() {
        // rate 1 used to size the output as in_frames * device_rate
        assert_eq!(resample(&[0.0; 8], 1, 1, 48000), None);
        assert_eq!(resample(&[0.0; 8], 2, 48000, 0), None);
        assert_eq!(resample(&[0.0; 8], 1, 1000, 768_001), None);
        assert_eq!(resample(&[0.5; 8], 1, 7, 7), Some(vec![0.5; 8]));
        let p = tmp("rate1.wav", &wav(1, 1, 8, &[128, 255, 0, 128]));
        assert!(load_file(&p, 48000).is_none());
        std::fs::remove_file(p).ok();
        // the same data at a real rate still loads
        let p = tmp("rate1000.wav", &wav(1000, 1, 8, &[128, 255, 0, 128]));
        assert_eq!(load_file(&p, 48000).expect("decode").frames, 192);
        std::fs::remove_file(p).ok();
    }

    /// `_SNDOPEN` of a damaged or foreign file must fail (handle 0), never panic: a
    /// panic aborts the program instead of letting it go on without the sound.
    #[test]
    fn damaged_sound_files_never_panic() {
        let mut seed: u64 = 0x5EED_AD10_0000_0001;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let samples: Vec<u8> = (0..400u32).map(|i| (i * 37 % 256) as u8).collect();
        let bases = [wav(22050, 1, 8, &samples), wav(44100, 2, 16, &samples), wav(8000, 1, 16, &samples[..2])];
        let runs = if cfg!(debug_assertions) { 60 } else { 300 };
        let mut bad = Vec::new();
        for k in 0..runs {
            let mut v = bases[k % bases.len()].clone();
            for _ in 0..1 + next() % 4 {
                let i = (next() % v.len() as u64) as usize;
                match next() % 4 {
                    0 => v[i] ^= 1 << (next() % 8),
                    1 => v[i] = [0, 0xFF, 0x7F, 0x80][(next() % 4) as usize],
                    2 => v.truncate(i),
                    _ if i + 4 <= v.len() => v[i..i + 4].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0x7F]),
                    _ => {}
                }
                if v.is_empty() {
                    break;
                }
            }
            // the same bytes as a sound file and as a tracker module (other decoder)
            for ext in ["wav", "mod", "xm", "s3m"] {
                let p = tmp(&format!("fuzz{k}.{ext}"), &v);
                let r = std::panic::catch_unwind(|| {
                    let _ = load_file(&p, 48000);
                });
                std::fs::remove_file(&p).ok();
                if r.is_err() {
                    bad.push(format!("{k}.{ext} len {}", v.len()));
                }
            }
        }
        for k in 0..runs {
            let n = (next() % 64) as usize;
            let input: Vec<f32> = (0..n).map(|_| f32::from_bits(next() as u32)).collect();
            let ch = (next() % 9) as usize;
            let (from, to) = ((next() % 200_000) as u32, (next() % 200_000) as u32);
            if std::panic::catch_unwind(|| resample(&input, ch, from, to)).is_err() {
                bad.push(format!("resample {k}: {n} samples, {ch} channels, {from} -> {to}"));
            }
        }
        assert!(bad.is_empty(), "{} panics: {:?}", bad.len(), &bad[..bad.len().min(10)]);
    }
}
