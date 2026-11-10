//! Sound file decoding for `_SNDOPEN` (legacy: miniaudio + dr_wav, dr_flac,
//! dr_mp3, stb_vorbis, libxmp-lite, tinysoundfont, RADv2).
//!
//! - WAV / FLAC / MP3 / Ogg Vorbis: symphonia (pure Rust).
//! - MOD / XM / S3M / IT: xmrs + xmrsplayer (pure Rust), rendered once to
//!   16-bit stereo (song played to its end, loops not repeated, capped at 30 minutes).
//! - MIDI (tinysoundfont + AWE32 ROM) and Reality AdLib Tracker (RADv2) are
//!   dropped: `_SNDOPEN` returns -1 for them (deviation D-63).
//!
//! Like the legacy resource manager, data is converted to the device sample
//! rate at load time and kept in the decoder's native sample format so
//! `_MEMSOUND` sees the same element type: WAV keeps its integer format
//! (u8/s16/s24/s32) or f32, everything else is f32 (tracker modules s16).
//! Resampling is linear interpolation (legacy used miniaudio's linear
//! resampler with a 4th-order low-pass filter; output differs slightly when
//! the file rate differs from the device rate).

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
        Pcm { format, channels, frames: samples.len() / channels.max(1) as usize, data }
    }

    pub fn sample(&self, index: usize) -> f32 {
        let b = &self.data;
        match self.format {
            SampleFormat::U8 => (b[index] as f32 - 128.0) / 128.0,
            SampleFormat::S16 => i16::from_le_bytes([b[2 * index], b[2 * index + 1]]) as f32 / 32768.0,
            SampleFormat::S24 => {
                let i = 3 * index;
                let v = i32::from_le_bytes([0, b[i], b[i + 1], b[i + 2]]) >> 8;
                v as f32 / 8388608.0
            }
            SampleFormat::S32 => i32::from_le_bytes(b[4 * index..4 * index + 4].try_into().unwrap()) as f32 / 2147483648.0,
            SampleFormat::F32 => f32::from_le_bytes(b[4 * index..4 * index + 4].try_into().unwrap()),
        }
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

/// Linear-interpolation resampler for interleaved f32.
pub fn resample(input: &[f32], channels: usize, from: u32, to: u32) -> Vec<f32> {
    if from == to || from == 0 || to == 0 || channels == 0 {
        return input.to_vec();
    }
    let in_frames = input.len() / channels;
    if in_frames == 0 {
        return Vec::new();
    }
    let out_frames = ((in_frames as u64 * to as u64) / from as u64) as usize;
    let mut out = Vec::with_capacity(out_frames * channels);
    let step = from as f64 / to as f64;
    for o in 0..out_frames {
        let pos = o as f64 * step;
        let i0 = (pos as usize).min(in_frames - 1);
        let i1 = (i0 + 1).min(in_frames - 1);
        let frac = (pos - i0 as f64) as f32;
        for c in 0..channels {
            let a = input[i0 * channels + c];
            let b = input[i1 * channels + c];
            out.push(a + (b - a) * frac);
        }
    }
    out
}

fn finish(format: SampleFormat, channels: u16, native: Vec<u8>, file_rate: u32, device_rate: u32) -> Pcm {
    let pcm = Pcm { format, channels, frames: native.len() / (format.bytes() * channels.max(1) as usize), data: native };
    if file_rate == device_rate {
        return pcm;
    }
    let f = resample(&pcm.to_f32(), channels as usize, file_rate, device_rate);
    Pcm::from_f32(format, channels, &f)
}

fn decode_symphonia(path: &Path, device_rate: u32) -> Option<Pcm> {
    let file = File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let mut reader = symphonia::default::get_probe()
        .probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())
        .ok()?;
    let track = reader.default_track(TrackType::Audio)?;
    let track_id = track.id;
    let params = track.codec_params.as_ref()?.audio()?.clone();
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())
        .ok()?;
    let is_pcm = symphonia::default::get_codecs()
        .get_audio_decoder(params.codec)
        .map(|d| d.codec.info.short_name.starts_with("pcm"))
        .unwrap_or(false);

    let mut format: Option<SampleFormat> = None;
    let mut channels = 0u16;
    let mut rate = params.sample_rate.unwrap_or(0);
    let mut native: Vec<u8> = Vec::new();

    while let Ok(Some(packet)) = reader.next_packet() {
        if packet.track_id != track_id {
            continue;
        }
        let buf = match decoder.decode(&packet) {
            Ok(b) => b,
            Err(SymError::DecodeError(_)) => continue,
            Err(_) => break,
        };
        let spec = buf.spec().clone();
        if format.is_none() {
            channels = spec.channels().count() as u16;
            rate = spec.rate();
            format = Some(if is_pcm {
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
        if spec.channels().count() as u16 != channels {
            break;
        }
        match format.unwrap() {
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
    }
    let format = format?;
    if channels == 0 || native.is_empty() {
        return None;
    }
    Some(finish(format, channels, native, rate, device_rate))
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
    fn linear_resample() {
        assert_eq!(resample(&[0.0, 1.0], 1, 1, 2), vec![0.0, 0.5, 1.0, 1.0]);
        assert_eq!(resample(&[0.0, 1.0, 2.0, 3.0], 1, 2, 1), vec![0.0, 2.0]);
    }
}
