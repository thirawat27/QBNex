//! QBNex synth voices: ADSR envelope, waveforms and the 16-bit stereo
//! waveform generator used by SOUND, BEEP and PLAY.
//!
//! Ported bug-for-bug from `internal/c/parts/audio/out/audio.cpp`
//! (`EnvelopeAmplitude`, `SampleCustomWave`, `NextLfsrNoise`,
//! `ParseCustomWaveSpec`, `ApplyWaveSpecToVoice`, `SampleSynthWave`,
//! `GenerateWaveform`, `WaveformBufferSize`). Arithmetic keeps the legacy
//! float/double mix and evaluation order so output matches sample for sample
//! (see tests/diff_audio.cpp), which is why some clippy suggestions
//! (clamp, partial_cmp) are not applied: they change NaN handling.
#![allow(clippy::manual_clamp, clippy::neg_cmp_op_on_partial_ord, clippy::excessive_precision)]

/// Number of synth voices (`_VOICE 1..4`).
pub const VOICE_COUNT: usize = 4;
// Legacy QBNEX_PI = 3.14159265358979323846 is exactly std::f64::consts::PI.
use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Envelope {
    pub attack: f64,
    pub decay: f64,
    pub sustain: f64,
    pub release: f64,
}

impl Default for Envelope {
    fn default() -> Self {
        Envelope { attack: 0.01, decay: 0.05, sustain: 0.80, release: 0.06 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Waveform {
    Triangle,
    Sine,
    Square,
    Saw,
    WhiteNoise,
    PinkNoise,
    BrownNoise,
    LfsrNoise,
    Custom,
}

/// The C runtime `rand()` used by the legacy noise generators: the MSVCRT/UCRT
/// LCG (`RAND_MAX` = 32767, seed 1). Using it on every platform makes noise
/// deterministic and identical to the legacy Windows build (a deviation on
/// Linux/macOS, where glibc/libc `rand()` differs).
#[derive(Clone, Debug)]
pub struct CRand {
    state: u32,
}

pub const RAND_MAX: i32 = 32767;

impl Default for CRand {
    fn default() -> Self {
        CRand { state: 1 }
    }
}

impl CRand {
    pub fn rand(&mut self) -> i32 {
        self.state = self.state.wrapping_mul(214013).wrapping_add(2531011);
        ((self.state >> 16) & 0x7fff) as i32
    }

    fn white(&mut self) -> f64 {
        (self.rand() as f64 / RAND_MAX as f64) * 2.0 - 1.0
    }
}

#[derive(Clone, Debug)]
pub struct Voice {
    pub envelope: Envelope,
    pub waveform: Waveform,
    pub custom_wave: Vec<f32>,
    pub lfsr: u32,
}

impl Default for Voice {
    fn default() -> Self {
        Voice { envelope: Envelope::default(), waveform: Waveform::Triangle, custom_wave: Vec::new(), lfsr: 0xACE1 }
    }
}

fn clamp_unit(v: f64) -> f64 {
    if v < 0.0 {
        0.0
    } else if v > 1.0 {
        1.0
    } else {
        v
    }
}

fn max0(v: f64) -> f64 {
    // std::max(0.0, v): returns 0.0 when v is NaN (comparison 0.0 < NaN is false)
    if 0.0 < v {
        v
    } else {
        0.0
    }
}

pub fn envelope_amplitude(env: &Envelope, total_length: f64, mut t: f64) -> f32 {
    let mut attack = max0(env.attack);
    let mut decay = max0(env.decay);
    let sustain = clamp_unit(env.sustain);
    let mut release = max0(env.release);
    let mut sustain_duration = 0.0;
    let shaped_total = attack + decay + release;

    if total_length <= 0.0 {
        return 0.0;
    }
    if shaped_total > total_length && shaped_total > 0.0 {
        let scale = total_length / shaped_total;
        attack *= scale;
        decay *= scale;
        release *= scale;
    } else {
        sustain_duration = total_length - shaped_total;
    }

    if attack > 0.0 && t < attack {
        return (t / attack) as f32;
    }
    t -= attack;
    if decay > 0.0 && t < decay {
        return (1.0 - (1.0 - sustain) * (t / decay)) as f32;
    }
    t -= decay;
    if t < sustain_duration {
        return sustain as f32;
    }
    t -= sustain_duration;
    if release > 0.0 && t < release {
        return (sustain * (1.0 - (t / release))) as f32;
    }
    0.0
}

pub fn sample_custom_wave(wave: &[f32], phase: f64) -> f32 {
    if wave.is_empty() {
        return 0.0;
    }
    if wave.len() == 1 {
        return wave[0];
    }
    let wrapped = phase - phase.floor();
    let scaled = wrapped * wave.len() as f64;
    let i0 = (scaled as usize) % wave.len();
    let i1 = (i0 + 1) % wave.len();
    let fraction = scaled - scaled.floor();
    // float + (float - float) * double, evaluated in double
    (wave[i0] as f64 + (wave[i1] - wave[i0]) as f64 * fraction) as f32
}

pub fn next_lfsr(lfsr: &mut u32) -> f32 {
    let bit = (*lfsr ^ (*lfsr >> 2) ^ (*lfsr >> 3) ^ (*lfsr >> 5)) & 1;
    *lfsr = (*lfsr >> 1) | (bit << 15);
    if *lfsr & 1 != 0 {
        1.0
    } else {
        -1.0
    }
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

fn trim(s: &[u8]) -> &[u8] {
    let mut a = 0;
    let mut b = s.len();
    while a < b && is_c_space(s[a]) {
        a += 1;
    }
    while b > a && is_c_space(s[b - 1]) {
        b -= 1;
    }
    &s[a..b]
}

/// `strtod` that must consume the whole (already trimmed) token.
/// Supports what C strtod accepts: optional sign, decimal with exponent,
/// hexadecimal floats (`0x1.8p3`), `inf`/`infinity`, `nan`/`nan(...)`.
pub fn strtod_full(tok: &[u8]) -> Option<f64> {
    // C strtod skips leading whitespace (tokens are trimmed already).
    let s = std::str::from_utf8(tok).ok()?;
    let (neg, body) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let lower = body.to_ascii_lowercase();
    let v = if lower == "inf" || lower == "infinity" {
        f64::INFINITY
    } else if lower == "nan" || (lower.starts_with("nan(") && lower.ends_with(')') && lower[4..lower.len() - 1].bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')) {
        f64::NAN
    } else if let Some(hex) = lower.strip_prefix("0x") {
        parse_hex_float(hex)?
    } else {
        let b = lower.as_bytes();
        let mut i = 0;
        let mut digits = 0;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
            digits += 1;
        }
        if i < b.len() && b[i] == b'.' {
            i += 1;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
                digits += 1;
            }
        }
        if digits == 0 {
            return None;
        }
        if i < b.len() && b[i] == b'e' {
            let mut j = i + 1;
            if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
                j += 1;
            }
            let start = j;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            if j > start {
                i = j;
            }
        }
        if i != b.len() {
            return None;
        }
        lower.parse::<f64>().ok()?
    };
    Some(if neg { -v } else { v })
}

fn parse_hex_float(s: &str) -> Option<f64> {
    let b = s.as_bytes();
    let mut i = 0;
    let mut mant: f64 = 0.0;
    let mut exp: i64 = 0;
    let mut digits = 0;
    while i < b.len() && b[i].is_ascii_hexdigit() {
        mant = mant * 16.0 + (b[i] as char).to_digit(16)? as f64;
        i += 1;
        digits += 1;
    }
    if i < b.len() && b[i] == b'.' {
        i += 1;
        while i < b.len() && b[i].is_ascii_hexdigit() {
            mant = mant * 16.0 + (b[i] as char).to_digit(16)? as f64;
            exp -= 4;
            i += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return None;
    }
    if i < b.len() && b[i] == b'p' {
        let mut j = i + 1;
        let mut eneg = false;
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
            eneg = b[j] == b'-';
            j += 1;
        }
        let start = j;
        let mut e: i64 = 0;
        while j < b.len() && b[j].is_ascii_digit() {
            e = (e * 10 + (b[j] - b'0') as i64).min(100_000);
            j += 1;
        }
        if j > start {
            exp += if eneg { -e } else { e };
            i = j;
        }
    }
    if i != b.len() {
        return None;
    }
    Some(mant * 2f64.powi(exp.clamp(-2000, 2000) as i32))
}

pub fn parse_custom_wave_spec(spec: &[u8]) -> Option<Vec<f32>> {
    let text = match spec.iter().position(|&c| c == b':') {
        Some(p) => &spec[p + 1..],
        None => spec,
    };
    let mut out = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let mut end = start;
        while end < text.len() && text[end] != b',' && text[end] != b';' {
            end += 1;
        }
        let tok = trim(&text[start..end]);
        if !tok.is_empty() {
            let mut v = strtod_full(tok)?;
            if v > 1.0 || v < -1.0 {
                v /= 32768.0;
            }
            if v > 1.0 {
                v = 1.0;
            }
            if v < -1.0 {
                v = -1.0;
            }
            out.push(v as f32);
        }
        start = end + 1;
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// `_WAVE spec` for one voice. Returns false for an invalid spec (error 5).
pub fn apply_wave_spec(voice: &mut Voice, spec: &[u8]) -> bool {
    let n: Vec<u8> = trim(spec).iter().map(|c| c.to_ascii_uppercase()).collect();
    let simple = match &n[..] {
        b"TRIANGLE" => Some(Waveform::Triangle),
        b"SINE" => Some(Waveform::Sine),
        b"SQUARE" => Some(Waveform::Square),
        b"SAW" | b"SAWTOOTH" => Some(Waveform::Saw),
        b"NOISE" | b"WHITE" | b"WHITE NOISE" => Some(Waveform::WhiteNoise),
        b"PINK" | b"PINK NOISE" => Some(Waveform::PinkNoise),
        b"BROWN" | b"BROWNIAN" | b"BROWN NOISE" | b"BROWNIAN NOISE" => Some(Waveform::BrownNoise),
        b"LFSR" | b"LFSR NOISE" => Some(Waveform::LfsrNoise),
        _ => None,
    };
    if let Some(w) = simple {
        voice.waveform = w;
        voice.custom_wave = Vec::new();
        if w == Waveform::LfsrNoise && voice.lfsr == 0 {
            voice.lfsr = 0xACE1;
        }
        return true;
    }
    if n.starts_with(b"CUSTOM") || n.contains(&b',') || n.contains(&b';') {
        // Note: the untrimmed, original-case spec is parsed (as legacy does).
        return match parse_custom_wave_spec(spec) {
            Some(w) => {
                voice.waveform = Waveform::Custom;
                voice.custom_wave = w;
                true
            }
            None => false,
        };
    }
    false
}

/// Per-waveform state that lives for one generated note.
#[derive(Default)]
struct NoteState {
    brown: f32,
    pink0: f32,
    pink1: f32,
    pink2: f32,
}

fn sample_wave(voice: &mut Voice, rng: &mut CRand, phase: f64, st: &mut NoteState) -> f32 {
    match voice.waveform {
        Waveform::Sine => (phase * PI * 2.0).sin() as f32,
        Waveform::Square => {
            if phase < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        Waveform::Saw => (phase * 2.0 - 1.0) as f32,
        Waveform::WhiteNoise => rng.white() as f32,
        Waveform::PinkNoise => {
            let white = rng.white() as f32;
            st.pink0 = 0.99765f32 * st.pink0 + white * 0.0990460f32;
            st.pink1 = 0.96300f32 * st.pink1 + white * 0.2965164f32;
            st.pink2 = 0.57000f32 * st.pink2 + white * 1.0526913f32;
            let mut pink = st.pink0 + st.pink1 + st.pink2 + white * 0.1848f32;
            if pink > 1.0 {
                pink = 1.0;
            }
            if pink < -1.0 {
                pink = -1.0;
            }
            pink
        }
        Waveform::BrownNoise => {
            let white = rng.white() as f32;
            st.brown += white * 0.08f32;
            if st.brown > 1.0 {
                st.brown = 1.0;
            }
            if st.brown < -1.0 {
                st.brown = -1.0;
            }
            st.brown
        }
        Waveform::LfsrNoise => next_lfsr(&mut voice.lfsr),
        Waveform::Custom => sample_custom_wave(&voice.custom_wave, phase),
        Waveform::Triangle => (1.0 - 4.0 * (phase - 0.5).abs()) as f32,
    }
}

/// x86 `cvttsd2si` (C/C++ double -> int32 cast): truncates toward zero;
/// NaN and out-of-range values give 0x80000000 instead of saturating.
pub fn cvtt_i32(x: f64) -> i32 {
    if x.is_nan() || x >= 2147483648.0 || x <= -2147483649.0 {
        i32::MIN
    } else {
        x as i32
    }
}

/// `(int32)(length * rate)`, at least 1 frame.
pub fn frames_for(rate: u32, length: f64) -> usize {
    let s = (length * rate as f64) as i32;
    if s == 0 {
        1
    } else {
        s.max(0) as usize
    }
}

/// Generates `frames_for(rate, length)` interleaved 16-bit stereo frames.
/// Frequencies of 20000 Hz and above give silence (QuickBASIC behaviour).
pub fn generate_waveform(voice: &mut Voice, rng: &mut CRand, rate: u32, frequency: f64, length: f64, volume: f64) -> Vec<i16> {
    let frames = frames_for(rate, length);
    let mut out = vec![0i16; frames * 2];
    if !(frequency < 20000.0) {
        return out;
    }
    let volume_multiplier = volume * 32767.0;
    let phase_inc = if rate > 0 { frequency / rate as f64 } else { 0.0 };
    let noise = matches!(voice.waveform, Waveform::WhiteNoise | Waveform::PinkNoise | Waveform::BrownNoise | Waveform::LfsrNoise);
    let mut st = NoteState::default();
    let mut phase = 0.0f64;
    for i in 0..frames {
        let t = i as f64 / rate as f64;
        let env = envelope_amplitude(&voice.envelope, length, t);
        let mut w = 0.0f32;
        if frequency > 0.0 || noise {
            w = sample_wave(voice, rng, phase - phase.floor(), &mut st);
        }
        // (float * float) * double, converted like the C++ cast to int16
        let x = cvtt_i32((w * env) as f64 * volume_multiplier) as i16;
        out[2 * i] = x;
        out[2 * i + 1] = x;
        phase += phase_inc;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_default_shape() {
        let e = Envelope::default();
        assert_eq!(envelope_amplitude(&e, 1.0, 0.005), 0.5);
        assert!((envelope_amplitude(&e, 1.0, 0.03) - 0.92).abs() < 1e-6);
        assert_eq!(envelope_amplitude(&e, 1.0, 0.5), 0.8);
        assert!((envelope_amplitude(&e, 1.0, 0.97) - 0.4).abs() < 1e-6);
        assert_eq!(envelope_amplitude(&e, 1.0, 1.5), 0.0);
        assert_eq!(envelope_amplitude(&e, 0.0, 0.0), 0.0);
    }

    #[test]
    fn envelope_scaled_when_note_is_short() {
        // a+d+r = 0.12 > 0.06: everything scaled by 0.5, no sustain
        let e = Envelope::default();
        assert_eq!(envelope_amplitude(&e, 0.06, 0.0025), 0.5);
    }

    #[test]
    fn lfsr_sequence() {
        let mut l = 0xACE1u32;
        let v: Vec<f32> = (0..4).map(|_| next_lfsr(&mut l)).collect();
        // 0xACE1: taps 0,2,3,5 -> bit = 1^0^0^1 = 0 -> 0x5670 ...
        assert_eq!(v[0], -1.0);
        assert_eq!(l & 0xffff, l);
    }

    #[test]
    fn crand_matches_msvcrt() {
        let mut r = CRand::default();
        assert_eq!([r.rand(), r.rand(), r.rand()], [41, 18467, 6334]);
    }

    #[test]
    fn custom_wave_parsing() {
        assert_eq!(parse_custom_wave_spec(b"CUSTOM:0,1,0,-1"), Some(vec![0.0, 1.0, 0.0, -1.0]));
        assert_eq!(parse_custom_wave_spec(b"16384; -32768"), Some(vec![0.5, -1.0]));
        assert_eq!(parse_custom_wave_spec(b"CUSTOM:"), None);
        assert_eq!(parse_custom_wave_spec(b"1,x"), None);
        assert_eq!(parse_custom_wave_spec(b"0x1p-1, .25, , 1e0"), Some(vec![0.5, 0.25, 1.0]));
    }

    #[test]
    fn wave_spec_names() {
        let mut v = Voice::default();
        assert!(apply_wave_spec(&mut v, b"  sawtooth "));
        assert_eq!(v.waveform, Waveform::Saw);
        assert!(apply_wave_spec(&mut v, b"Brownian Noise"));
        assert_eq!(v.waveform, Waveform::BrownNoise);
        assert!(apply_wave_spec(&mut v, b"custom:0,1"));
        assert_eq!(v.waveform, Waveform::Custom);
        assert_eq!(v.custom_wave, vec![0.0, 1.0]);
        assert!(!apply_wave_spec(&mut v, b"BOGUS"));
        assert_eq!(v.waveform, Waveform::Custom);
        v.lfsr = 0;
        assert!(apply_wave_spec(&mut v, b"lfsr"));
        assert_eq!(v.lfsr, 0xACE1);
    }

    #[test]
    fn custom_wave_interpolates() {
        let w = [0.0f32, 1.0];
        assert_eq!(sample_custom_wave(&w, 0.25), 0.5);
        assert_eq!(sample_custom_wave(&w, 0.75), 0.5);
        assert_eq!(sample_custom_wave(&[0.3], 0.9), 0.3);
    }

    #[test]
    fn waveform_lengths_and_silence() {
        let mut v = Voice::default();
        let mut r = CRand::default();
        assert_eq!(generate_waveform(&mut v, &mut r, 8000, 440.0, 0.5, 1.0).len(), 8000);
        assert_eq!(generate_waveform(&mut v, &mut r, 8000, 440.0, 0.0, 1.0).len(), 2);
        assert!(generate_waveform(&mut v, &mut r, 8000, 20000.0, 0.1, 1.0).iter().all(|&s| s == 0));
        assert!(generate_waveform(&mut v, &mut r, 8000, 0.0, 0.1, 1.0).iter().all(|&s| s == 0));
    }

    #[test]
    fn huge_volume_wraps_like_cvttsd2si() {
        assert_eq!(cvtt_i32(3e9), i32::MIN);
        assert_eq!(cvtt_i32(f64::NAN), i32::MIN);
        assert_eq!(cvtt_i32(-2147483648.5), -2147483648);
        assert_eq!(cvtt_i32(-7.9), -7);
        let mut v = Voice { waveform: Waveform::Square, envelope: Envelope { attack: 0.0, decay: 0.0, sustain: 1.0, release: 0.0 }, ..Voice::default() };
        let w = generate_waveform(&mut v, &mut CRand::default(), 8000, 1000.0, 0.01, 1e6);
        assert_eq!(w[0], 0); // 32767e6 does not fit in int32: low 16 bits of 0x80000000
    }

    #[test]
    fn square_wave_full_volume() {
        let mut v = Voice { waveform: Waveform::Square, envelope: Envelope { attack: 0.0, decay: 0.0, sustain: 1.0, release: 0.0 }, ..Voice::default() };
        let mut r = CRand::default();
        let w = generate_waveform(&mut v, &mut r, 8000, 1000.0, 0.01, 1.0);
        // 8 samples per period: 4 high, 4 low
        assert_eq!(&w[0..16], &[32767, 32767, 32767, 32767, 32767, 32767, 32767, 32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767]);
        assert_eq!(w[0], w[1]);
    }
}
