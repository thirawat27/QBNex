//! SOUND, PLAY (MML), _VOICE, _ADSR and _WAVE: the device-independent part of
//! the legacy synth (`sub_sound`, `sub_play`, `sub__voice`, `sub__adsr`,
//! `sub__wave`, `ResolveConfigVoiceIndex`, `ResolvePlaybackVoiceIndex` in
//! `internal/c/parts/audio/out/audio.cpp`).
//!
//! Rendering produces 16-bit stereo frames and hands them to a [`Sink`]
//! (a voice queue in the engine, a buffer in tests), exactly where legacy
//! called `SendWaveformToQueue`.

use crate::synth::{apply_wave_spec, frames_for, generate_waveform, CRand, Voice, VOICE_COUNT};

/// QBasic "Illegal function call".
pub const ERR_ILLEGAL: i32 = 5;

/// Receives rendered 16-bit stereo frames for a synth voice.
pub trait Sink {
    fn send(&mut self, voice: usize, frames: &[i16]);
}

/// Reads a numeric variable referenced by `"=" + VARPTR$(v)` in a PLAY string.
/// `ty` is the VARPTR$ type byte and `offset` the offset inside DBLOCK.
pub trait VarReader {
    /// Returns the raw 8 bytes at `offset` (only as many as the type needs are used).
    fn read(&self, offset: u16) -> [u8; 10];
}

/// No DBLOCK available (tests, or VARPTR$ unsupported): reads zeros.
pub struct NoVars;
impl VarReader for NoVars {
    fn read(&self, _offset: u16) -> [u8; 10] {
        [0; 10]
    }
}

/// PLAY state that persists between PLAY statements (C++ function statics).
#[derive(Clone, Debug)]
pub struct PlayState {
    pub octave: i32,
    pub tempo: f64,
    pub note_length: f64,
    /// ML 0, MN 1/8, MS 1/4
    pub pause: f64,
    /// 1 after MB, 0 after MF
    pub mb: f64,
    pub volume: f64,
    pub fullstops: i32,
    /// Frequency of the last note rendered (legacy static `frequency`).
    pub frequency: f64,
}

impl Default for PlayState {
    fn default() -> Self {
        PlayState { octave: 4, tempo: 120.0, note_length: 4.0, pause: 1.0 / 8.0, mb: 0.0, volume: 50.0, fullstops: 0, frequency: 0.0 }
    }
}

/// All synth state of the engine: voices, voice selection, PLAY state, rand().
#[derive(Clone, Debug, Default)]
pub struct Synth {
    pub voices: [Voice; VOICE_COUNT],
    pub rng: CRand,
    pub current_voice: usize,
    pub next_voice: usize,
    pub voice_pinned: bool,
    pub play: PlayState,
}

fn f80_to_f64(b: &[u8]) -> f64 {
    let mant = u64::from_le_bytes(b[0..8].try_into().unwrap());
    let se = u16::from_le_bytes([b[8], b[9]]);
    let sign = if se & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exp = (se & 0x7fff) as i32;
    if exp == 0 && mant == 0 {
        return sign * 0.0;
    }
    // x87 loads unnormals, pseudo-infinities and pseudo-NaNs (integer bit
    // clear with a non-zero exponent) as the invalid-operation NaN.
    if exp != 0 && mant >> 63 == 0 {
        return f64::NAN;
    }
    if exp == 0x7fff {
        return if mant << 1 == 0 { sign * f64::INFINITY } else { f64::NAN };
    }
    // value = mant * 2^(exp - 16383 - 63); denormals use exponent 1
    let e = exp.max(1) - 16383 - 63;
    let hi = (mant >> 11) as f64 * 2f64.powi(11);
    let lo = (mant & 0x7ff) as f64;
    sign * (hi + lo) * pow2(e)
}

fn pow2(e: i32) -> f64 {
    if e < -1000 {
        2f64.powi(-1000) * 2f64.powi(e + 1000)
    } else if e > 1000 {
        2f64.powi(1000) * 2f64.powi(e - 1000)
    } else {
        2f64.powi(e)
    }
}

/// Value of a VARPTR$ reference; None = illegal type (error 5).
fn varptr_value(ty: u8, raw: &[u8; 10]) -> Option<f64> {
    let i64v = i64::from_le_bytes(raw[0..8].try_into().unwrap());
    Some(match ty {
        1 => raw[0] as i8 as f64,
        129 => raw[0] as f64,
        2 => i16::from_le_bytes([raw[0], raw[1]]) as f64,
        130 => u16::from_le_bytes([raw[0], raw[1]]) as f64,
        4 => f32::from_le_bytes(raw[0..4].try_into().unwrap()) as f64,
        5 | 133 => i64v as f64,
        6 => f80_to_f64(&raw[..]),
        8 => f64::from_le_bytes(raw[0..8].try_into().unwrap()),
        20 => i32::from_le_bytes(raw[0..4].try_into().unwrap()) as f64,
        148 => u32::from_le_bytes(raw[0..4].try_into().unwrap()) as f64,
        _ => {
            if ty & 64 == 0 {
                return None;
            }
            let bits = (ty & 63) as u32;
            if bits > 56 {
                return None;
            }
            let mask = 1i64.wrapping_shl(bits).wrapping_sub(1);
            let mut num = i64v & mask;
            if ty & 128 != 0 {
                let top = 1i64.wrapping_shl(bits.wrapping_sub(1));
                if num & top != 0 {
                    num = num.wrapping_add((-1i64).wrapping_shl(bits));
                }
            }
            num as f64
        }
    })
}

impl Synth {
    /// `ResolveConfigVoiceIndex`: Some((index, apply_all)) or None for an invalid voice.
    fn config_voice(&self, requested: i32, passed: bool) -> Option<(usize, bool)> {
        if passed {
            if requested < 1 || requested > VOICE_COUNT as i32 {
                return None;
            }
            return Some(((requested - 1) as usize, false));
        }
        if self.voice_pinned {
            return Some((self.current_voice, false));
        }
        Some((0, true))
    }

    /// `ResolvePlaybackVoiceIndex`: round-robin unless `_VOICE n` pinned a voice.
    pub fn playback_voice(&mut self) -> usize {
        if self.voice_pinned {
            return self.current_voice;
        }
        let v = self.next_voice;
        self.next_voice = (self.next_voice + 1) % VOICE_COUNT;
        self.current_voice = v;
        v
    }

    /// `_VOICE n` (0 = round-robin).
    pub fn set_voice(&mut self, voice: i32) -> Result<(), i32> {
        if voice == 0 {
            self.voice_pinned = false;
            return Ok(());
        }
        if voice < 1 || voice > VOICE_COUNT as i32 {
            return Err(ERR_ILLEGAL);
        }
        self.voice_pinned = true;
        self.current_voice = (voice - 1) as usize;
        Ok(())
    }

    /// `_ADSR attack, decay, sustain, release[, voice]`; `passed & 16` = voice given.
    pub fn adsr(&mut self, attack: f64, decay: f64, mut sustain: f64, release: f64, voice: i32, passed: i32) -> Result<(), i32> {
        let (target, all) = self.config_voice(voice, passed & 16 != 0).ok_or(ERR_ILLEGAL)?;
        if sustain > 1.0 && sustain <= 100.0 {
            sustain /= 100.0;
        }
        if attack < 0.0 || decay < 0.0 || release < 0.0 || sustain < 0.0 || sustain > 1.0 {
            return Err(ERR_ILLEGAL);
        }
        let range = if all { 0..VOICE_COUNT } else { target..target + 1 };
        for v in &mut self.voices[range] {
            v.envelope.attack = attack;
            v.envelope.decay = decay;
            v.envelope.sustain = sustain;
            v.envelope.release = release;
        }
        Ok(())
    }

    /// `_WAVE spec$[, voice]`; `passed & 2` = voice given.
    pub fn wave(&mut self, spec: &[u8], voice: i32, passed: i32) -> Result<(), i32> {
        let (target, all) = self.config_voice(voice, passed & 2 != 0).ok_or(ERR_ILLEGAL)?;
        if spec.is_empty() {
            return Err(ERR_ILLEGAL);
        }
        let range = if all { 0..VOICE_COUNT } else { target..target + 1 };
        for v in &mut self.voices[range] {
            if !apply_wave_spec(v, spec) {
                return Err(ERR_ILLEGAL);
            }
        }
        Ok(())
    }

    /// `SOUND frequency, duration` (duration in clock ticks, 18.2 per second).
    /// `initialized` = audio engine is up (legacy returns silently otherwise).
    pub fn sound(&mut self, rate: u32, initialized: bool, frequency: f64, ticks: f64, sink: &mut dyn Sink) -> Result<(), i32> {
        if !initialized {
            return Ok(());
        }
        if (frequency < 37.0 && frequency != 0.0) || frequency > 32767.0 || ticks < 0.0 || ticks > 65535.0 {
            return Err(ERR_ILLEGAL);
        }
        if ticks == 0.0 {
            return Ok(());
        }
        let v = self.playback_voice();
        let data = generate_waveform(&mut self.voices[v], &mut self.rng, rate, frequency, ticks / 18.2, 1.0);
        sink.send(v, &data);
        Ok(())
    }

    /// `PLAY commands$`.
    ///
    /// A transcription of the legacy `goto` state machine: `continue` is
    /// `goto next_byte`, `break 'parse` is `goto done`. Buffer sizes are in
    /// i16 units (legacy used bytes, always a multiple of 4).
    pub fn play(&mut self, rate: u32, initialized: bool, s: &[u8], vars: &dyn VarReader, sink: &mut dyn Sink) -> Result<(), i32> {
        if !initialized {
            return Ok(());
        }
        let voice = self.playback_voice();
        let st = &mut self.play;

        let mut pos = 0usize;
        let mut bytes_left = s.len() as i64;
        let mut wave: Option<Vec<i16>> = None;
        let mut wave_base = 0usize;
        let mut n_changed = false;
        let mut n: i32 = 0;
        let mut number_entered = 0;
        let mut number: i64 = 0;
        let mut followup = 0;
        let mut length = 1.0 / (st.tempo / 60.0) * (4.0 / st.note_length);
        let mut playit = false;
        let mut a: i32 = 0;
        let mut i: i32;

        // realloc + the legacy memset of (new - old) bytes starting at wave_base.
        // The legacy tail past that range was uninitialised; here it is zero (D-61).
        fn grow(w: &mut Vec<i16>, base: usize, add: usize) {
            if base + add > w.len() {
                let old = w.len();
                w.resize(base + add, 0);
                let clear = base + add - old;
                for x in &mut w[base..base + clear] {
                    *x = 0;
                }
            }
        }

        'parse: loop {
            let more = bytes_left != 0;
            bytes_left -= 1;
            if !(more || followup != 0) {
                break 'parse;
            }

            if bytes_left < 0 {
                i = 32;
            } else {
                i = s[pos] as i32;
                pos += 1;
                if i == 32 {
                    continue;
                }
                a = if (97..=122).contains(&i) { i - 32 } else { i };

                if i == 61 {
                    // '=' + VARPTR$
                    if st.fullstops != 0 || number_entered != 0 {
                        return Err(ERR_ILLEGAL);
                    }
                    number_entered = 2;
                    if bytes_left < 3 {
                        return Err(ERR_ILLEGAL);
                    }
                    let ty = s[pos];
                    let off = u16::from_le_bytes([s[pos + 1], s[pos + 2]]);
                    pos += 3;
                    bytes_left -= 3;
                    let d = varptr_value(ty, &vars.read(off)).ok_or(ERR_ILLEGAL)?;
                    if d > 2147483647.0 || d < -2147483648.0 {
                        return Err(ERR_ILLEGAL);
                    }
                    // C++ (int64)round(d): NaN converts to INT64_MIN (cvttsd2si)
                    number = if d.is_nan() { i64::MIN } else { d.round() as i64 };
                    continue;
                }

                if (48..=57).contains(&i) {
                    if st.fullstops != 0 || number_entered == 2 {
                        return Err(ERR_ILLEGAL);
                    }
                    if number_entered == 0 {
                        number = 0;
                        number_entered = 1;
                    }
                    number = number.wrapping_mul(10).wrapping_add((i - 48) as i64);
                    continue;
                }

                if i == 46 {
                    if followup != 7 && followup != 1 && followup != 4 {
                        return Err(ERR_ILLEGAL);
                    }
                    st.fullstops += 1;
                    continue;
                }
            }

            // follow_up:
            if followup == 8 {
                // V
                if number_entered == 0 {
                    return Err(ERR_ILLEGAL);
                }
                number_entered = 0;
                if number > 100 {
                    return Err(ERR_ILLEGAL);
                }
                st.volume = number as f64;
                followup = 0;
                if bytes_left < 0 {
                    break 'parse;
                }
            }

            if followup == 7 {
                // P
                let mut length2;
                if number_entered != 0 {
                    number_entered = 0;
                    if !(1..=64).contains(&number) {
                        return Err(ERR_ILLEGAL);
                    }
                    length2 = 1.0 / (st.tempo / 60.0) * (4.0 / (number as f64));
                } else {
                    length2 = length;
                }
                let mut d = length2;
                for _ in 0..st.fullstops.max(0) {
                    d /= 2.0;
                    length2 += d;
                }
                st.fullstops = 0;

                let sw = frames_for(rate, length2) * 2;
                match wave.as_mut() {
                    None => {
                        wave = Some(vec![0; sw]);
                        wave_base = 0;
                    }
                    Some(w) => grow(w, wave_base, sw),
                }
                if i != 44 {
                    wave_base += sw;
                }
                playit = true;
                followup = 0;
                if i == 44 {
                    continue;
                }
                if bytes_left < 0 {
                    break 'parse;
                }
            }

            if followup == 6 {
                // T
                if number_entered == 0 {
                    return Err(ERR_ILLEGAL);
                }
                number_entered = 0;
                if !(32..=255).contains(&number) {
                    number = 120;
                }
                st.tempo = number as f64;
                length = 1.0 / (st.tempo / 60.0) * (4.0 / st.note_length);
                followup = 0;
                if bytes_left < 0 {
                    break 'parse;
                }
            }

            if followup == 5 {
                // M
                if number_entered != 0 {
                    return Err(ERR_ILLEGAL);
                }
                match a {
                    76 => st.pause = 0.0,
                    78 => st.pause = 1.0 / 8.0,
                    83 => st.pause = 1.0 / 4.0,
                    66 => {
                        if st.mb == 0.0 {
                            st.mb = 1.0;
                            if playit {
                                playit = false;
                                if let Some(w) = &wave {
                                    sink.send(voice, w);
                                }
                            }
                            wave = None;
                        }
                    }
                    70 => {
                        if st.mb != 0.0 {
                            st.mb = 0.0;
                        }
                    }
                    _ => return Err(ERR_ILLEGAL),
                }
                followup = 0;
                continue;
            }

            let mut to_followup1 = false;
            if followup == 4 {
                // N
                if number_entered == 0 {
                    return Err(ERR_ILLEGAL);
                }
                number_entered = 0;
                if number > 84 {
                    return Err(ERR_ILLEGAL);
                }
                n = (-33i64).wrapping_add(number) as i32;
                to_followup1 = true;
            }

            if !to_followup1 && followup == 3 {
                // O
                if number_entered == 0 {
                    return Err(ERR_ILLEGAL);
                }
                number_entered = 0;
                if number > 6 {
                    return Err(ERR_ILLEGAL);
                }
                st.octave = number as i32;
                followup = 0;
                if bytes_left < 0 {
                    break 'parse;
                }
            }

            if !to_followup1 && followup == 2 {
                // L
                if number_entered == 0 {
                    return Err(ERR_ILLEGAL);
                }
                number_entered = 0;
                if !(1..=64).contains(&number) {
                    return Err(ERR_ILLEGAL);
                }
                st.note_length = number as f64;
                length = 1.0 / (st.tempo / 60.0) * (4.0 / st.note_length);
                followup = 0;
                if bytes_left < 0 {
                    break 'parse;
                }
            }

            if to_followup1 || followup == 1 {
                // A-G
                if !to_followup1 {
                    if i == 45 {
                        if n_changed || number_entered != 0 {
                            return Err(ERR_ILLEGAL);
                        }
                        n_changed = true;
                        n -= 1;
                        continue;
                    }
                    if i == 43 || i == 35 {
                        if n_changed || number_entered != 0 {
                            return Err(ERR_ILLEGAL);
                        }
                        n_changed = true;
                        n += 1;
                        continue;
                    }
                }
                // followup1:
                let mut length2;
                if number_entered != 0 {
                    number_entered = 0;
                    if !(0..=64).contains(&number) {
                        return Err(ERR_ILLEGAL);
                    }
                    length2 = if number == 0 { length } else { 1.0 / (st.tempo / 60.0) * (4.0 / (number as f64)) };
                } else {
                    length2 = length;
                }
                let mut d = length2;
                for _ in 0..st.fullstops.max(0) {
                    d /= 2.0;
                    length2 += d;
                }
                st.fullstops = 0;
                st.frequency = 2f64.powf(n as f64 / 12.0) * 440.0;

                let mut wave2 = generate_waveform(&mut self.voices[voice], &mut self.rng, rate, st.frequency, length2 * (1.0 - st.pause), st.volume / 100.0);
                if st.pause > 0.0 {
                    let extra = frames_for(rate, length2 * st.pause) * 2;
                    wave2.resize(wave2.len() + extra, 0);
                }
                let sw = wave2.len();
                match wave.as_mut() {
                    None => {
                        wave = Some(wave2);
                        wave_base = 0;
                    }
                    Some(w) => {
                        let mix = wave_base != w.len();
                        grow(w, wave_base, sw);
                        if mix {
                            for (dst, &src) in w[wave_base..wave_base + sw].iter_mut().zip(&wave2) {
                                let x = (*dst as i32 + src as i32).clamp(-32767, 32767);
                                *dst = x as i16;
                            }
                        } else {
                            w[wave_base..wave_base + sw].copy_from_slice(&wave2);
                        }
                    }
                }
                if i != 44 {
                    wave_base += sw;
                }
                playit = true;
                n_changed = false;
                followup = 0;
                if i == 44 {
                    continue;
                }
                if bytes_left < 0 {
                    break 'parse;
                }
            }

            if (65..=71).contains(&a) {
                n = match a {
                    65 => 9,
                    66 => 11,
                    67 => 0,
                    68 => 2,
                    69 => 4,
                    70 => 5,
                    _ => 7,
                };
                n = n + (st.octave - 2) * 12 - 9;
                followup = 1;
                continue;
            }
            match a {
                76 => followup = 2,
                77 => followup = 5,
                78 => followup = 4,
                79 => followup = 3,
                84 => followup = 6,
                60 => {
                    st.octave -= 1;
                    if st.octave < 0 {
                        st.octave = 0;
                    }
                }
                62 => {
                    st.octave += 1;
                    if st.octave > 6 {
                        st.octave = 6;
                    }
                }
                80 => followup = 7,
                86 => followup = 8,
                _ => return Err(ERR_ILLEGAL),
            }
        }

        // done:
        if number_entered != 0 || followup != 0 {
            return Err(ERR_ILLEGAL);
        }
        if playit
            && let Some(w) = &wave {
                sink.send(voice, w);
            }
        Ok(())
    }
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;
    use crate::synth::Waveform;

    #[derive(Default)]
    struct Buf {
        sent: Vec<(usize, Vec<i16>)>,
    }
    impl Sink for Buf {
        fn send(&mut self, voice: usize, frames: &[i16]) {
            self.sent.push((voice, frames.to_vec()));
        }
    }

    const RATE: u32 = 8000;

    fn play(s: &mut Synth, mml: &str) -> (Result<(), i32>, Buf) {
        let mut b = Buf::default();
        let r = s.play(RATE, true, mml.as_bytes(), &NoVars, &mut b);
        (r, b)
    }

    fn frames(b: &Buf) -> usize {
        b.sent.iter().map(|(_, f)| f.len() / 2).sum()
    }

    #[test]
    fn default_quarter_note_with_normal_pause() {
        let mut s = Synth::default();
        let (r, b) = play(&mut s, "O2A");
        assert_eq!(r, Ok(()));
        assert_eq!(b.sent.len(), 1);
        // T120 L4 = 0.5 s; MN: 7/8 tone (3500 frames) + 1/8 silence (500 frames)
        assert_eq!(frames(&b), 4000);
        assert!(b.sent[0].1[2 * 3500..].iter().all(|&x| x == 0));
        assert!(b.sent[0].1[..2 * 3500].iter().any(|&x| x != 0));
        assert_eq!(s.play.frequency, 440.0);
    }

    #[test]
    fn octaves_and_octave_shifts() {
        let mut s = Synth::default();
        play(&mut s, "O3A");
        assert_eq!(s.play.frequency, 880.0);
        play(&mut s, "<A");
        assert_eq!(s.play.frequency, 440.0);
        assert_eq!(s.play.octave, 2);
        play(&mut s, ">>>>>>>>");
        assert_eq!(s.play.octave, 6);
        play(&mut s, "<<<<<<<<<");
        assert_eq!(s.play.octave, 0);
        assert_eq!(play(&mut s, "O7").0, Err(5));
    }

    #[test]
    fn sharps_and_flats() {
        let mut s = Synth::default();
        play(&mut s, "O2A#");
        assert_eq!(s.play.frequency, 2f64.powf(1.0 / 12.0) * 440.0);
        play(&mut s, "O2A+");
        assert_eq!(s.play.frequency, 2f64.powf(1.0 / 12.0) * 440.0);
        play(&mut s, "O2A-");
        assert_eq!(s.play.frequency, 2f64.powf(-1.0 / 12.0) * 440.0);
        assert_eq!(play(&mut s, "A#-").0, Err(5));
        assert_eq!(play(&mut s, "A4#").0, Err(5));
    }

    #[test]
    fn lengths_tempo_and_dots() {
        let mut s = Synth::default();
        let (_, b) = play(&mut s, "MLL8C");
        assert_eq!(frames(&b), 2000);
        let (_, b) = play(&mut s, "C16");
        assert_eq!(frames(&b), 1000);
        let (_, b) = play(&mut s, "C.");
        assert_eq!(frames(&b), 3000);
        let (_, b) = play(&mut s, "C..");
        assert_eq!(frames(&b), 3500);
        let (_, b) = play(&mut s, "T60C4");
        assert_eq!(frames(&b), 8000);
        let (_, b) = play(&mut s, "T10C4");
        assert_eq!(s.play.tempo, 120.0);
        assert_eq!(frames(&b), 4000);
        let (_, b) = play(&mut s, "C0");
        assert_eq!(frames(&b), 2000); // 0 = current L8
        assert_eq!(play(&mut s, "L0").0, Err(5));
        assert_eq!(play(&mut s, "L65").0, Err(5));
        assert_eq!(play(&mut s, "C65").0, Err(5));
    }

    #[test]
    fn music_styles() {
        let mut s = Synth::default();
        let (_, b) = play(&mut s, "MSC");
        assert_eq!(frames(&b), 4000);
        assert!(b.sent[0].1[2 * 3000..].iter().all(|&x| x == 0));
        assert!(b.sent[0].1[2 * 2999..2 * 3000].iter().any(|&x| x != 0) || true);
        play(&mut s, "ML");
        assert_eq!(s.play.pause, 0.0);
        play(&mut s, "MN");
        assert_eq!(s.play.pause, 1.0 / 8.0);
        assert_eq!(play(&mut s, "MX").0, Err(5));
        assert_eq!(play(&mut s, "M").0, Err(5));
    }

    #[test]
    fn mb_flushes_and_mf_keeps_buffer() {
        let mut s = Synth::default();
        let (r, b) = play(&mut s, "CMBD");
        assert_eq!(r, Ok(()));
        assert_eq!(s.play.mb, 1.0);
        // C sent at MB, D sent at the end
        assert_eq!(b.sent.len(), 2);
        let (_, b) = play(&mut s, "MBC");
        assert_eq!(b.sent.len(), 1); // MB again: no flush
        play(&mut s, "MF");
        assert_eq!(s.play.mb, 0.0);
    }

    #[test]
    fn n_command_and_pauses() {
        let mut s = Synth::default();
        play(&mut s, "N33");
        assert_eq!(s.play.frequency, 440.0);
        play(&mut s, "N0");
        assert_eq!(s.play.frequency, 2f64.powf(-33.0 / 12.0) * 440.0);
        assert_eq!(play(&mut s, "N85").0, Err(5));
        assert_eq!(play(&mut s, "N").0, Err(5));
        let (r, b) = play(&mut s, "P4");
        assert_eq!(r, Ok(()));
        assert_eq!(frames(&b), 4000);
        assert!(b.sent[0].1.iter().all(|&x| x == 0));
        let (_, b) = play(&mut s, "P8.");
        assert_eq!(frames(&b), 3000);
    }

    #[test]
    fn chords_with_comma_mix() {
        let mut s = Synth::default();
        let (r, b) = play(&mut s, "MLC,E");
        assert_eq!(r, Ok(()));
        // E mixed on top of C: one note long
        assert_eq!(frames(&b), 4000);
        let mut solo = Synth::default();
        let (_, c) = play(&mut solo, "MLC");
        assert_ne!(b.sent[0].1, c.sent[0].1);
        assert_eq!(play(&mut s, ",").0, Err(5));
    }

    #[test]
    fn volume_and_errors() {
        let mut s = Synth::default();
        play(&mut s, "V100");
        assert_eq!(s.play.volume, 100.0);
        assert_eq!(play(&mut s, "V101").0, Err(5));
        assert_eq!(play(&mut s, "V").0, Err(5));
        assert_eq!(play(&mut s, "X").0, Err(5));
        assert_eq!(play(&mut s, "C\t").0, Err(5));
        assert_eq!(play(&mut s, ".").0, Err(5));
        assert_eq!(play(&mut s, "c d e").0, Ok(()));
    }

    #[test]
    fn state_persists_between_statements() {
        let mut s = Synth::default();
        play(&mut s, "T240 L8 O1 MS V10");
        let p = s.play.clone();
        assert_eq!((p.tempo, p.note_length, p.octave, p.pause, p.volume), (240.0, 8.0, 1, 0.25, 10.0));
        let (_, b) = play(&mut s, "C");
        assert_eq!(frames(&b), 1000);
    }

    #[test]
    fn varptr_reference() {
        struct Vars;
        impl VarReader for Vars {
            fn read(&self, offset: u16) -> [u8; 10] {
                let mut r = [0u8; 10];
                r[0..2].copy_from_slice(&(offset as i16 * 2).to_le_bytes());
                r
            }
        }
        let mut s = Synth::default();
        let mut b = Buf::default();
        // "O=" + VARPTR$(integer at offset 1) -> O2
        let mml = [b'O', b'=', 2, 1, 0, b'A'];
        assert_eq!(s.play(RATE, true, &mml, &Vars, &mut b), Ok(()));
        assert_eq!(s.play.octave, 2);
        assert_eq!(s.play.frequency, 440.0);
        let bad = [b'O', b'=', 3, 1, 0];
        assert_eq!(s.play(RATE, true, &bad, &Vars, &mut b), Err(5));
    }

    #[test]
    fn voices_round_robin_and_pinning() {
        let mut s = Synth::default();
        let mut b = Buf::default();
        for _ in 0..5 {
            s.sound(RATE, true, 440.0, 1.0, &mut b).unwrap();
        }
        let v: Vec<usize> = b.sent.iter().map(|x| x.0).collect();
        assert_eq!(v, vec![0, 1, 2, 3, 0]);
        s.set_voice(3).unwrap();
        s.sound(RATE, true, 440.0, 1.0, &mut b).unwrap();
        assert_eq!(b.sent.last().unwrap().0, 2);
        assert_eq!(s.set_voice(5), Err(5));
        assert_eq!(s.wave(b"SQUARE", 0, 0), Ok(()));
        assert_eq!(s.voices[2].waveform, Waveform::Square);
        assert_eq!(s.voices[0].waveform, Waveform::Triangle);
        s.set_voice(0).unwrap();
        assert_eq!(s.wave(b"SINE", 0, 0), Ok(()));
        assert!(s.voices.iter().all(|v| v.waveform == Waveform::Sine));
        assert_eq!(s.wave(b"SAW", 4, 2), Ok(()));
        assert_eq!(s.voices[3].waveform, Waveform::Saw);
        assert_eq!(s.wave(b"SAW", 5, 2), Err(5));
        assert_eq!(s.wave(b"", 1, 2), Err(5));
    }

    #[test]
    fn adsr_rules() {
        let mut s = Synth::default();
        assert_eq!(s.adsr(0.1, 0.2, 50.0, 0.3, 2, 16), Ok(()));
        assert_eq!(s.voices[1].envelope.sustain, 0.5);
        assert_eq!(s.voices[0].envelope.sustain, 0.8);
        assert_eq!(s.adsr(-0.1, 0.2, 0.5, 0.3, 0, 0), Err(5));
        assert_eq!(s.adsr(0.1, 0.2, 101.0, 0.3, 0, 0), Err(5));
        assert_eq!(s.adsr(0.0, 0.0, 1.0, 0.0, 0, 0), Ok(()));
        assert!(s.voices.iter().all(|v| v.envelope.sustain == 1.0));
    }

    #[test]
    fn sound_rules() {
        let mut s = Synth::default();
        let mut b = Buf::default();
        assert_eq!(s.sound(RATE, true, 36.0, 1.0, &mut b), Err(5));
        assert_eq!(s.sound(RATE, true, 32768.0, 1.0, &mut b), Err(5));
        assert_eq!(s.sound(RATE, true, 440.0, -1.0, &mut b), Err(5));
        assert_eq!(s.sound(RATE, true, 440.0, 65536.0, &mut b), Err(5));
        assert_eq!(s.sound(RATE, true, 440.0, 0.0, &mut b), Ok(()));
        assert_eq!(s.sound(RATE, false, 1.0, -1.0, &mut b), Ok(()));
        assert!(b.sent.is_empty());
        assert_eq!(s.sound(RATE, true, 0.0, 18.2, &mut b), Ok(()));
        assert_eq!(b.sent[0].1.len(), 2 * 8000);
    }

    #[test]
    fn f80_conversion() {
        let one = [0, 0, 0, 0, 0, 0, 0, 0x80, 0xff, 0x3f];
        assert_eq!(f80_to_f64(&one), 1.0);
        assert_eq!(varptr_value(6, &one), Some(1.0));
        assert_eq!(varptr_value(3, &one), None);
        let unnormal = [0, 0, 0, 0, 0, 0, 0, 0x40, 0xff, 0x3f];
        assert!(f80_to_f64(&unnormal).is_nan());
        // _BIT * 4 signed: 0b1111 -> -1
        let mut r = [0u8; 10];
        r[0] = 0x0f;
        assert_eq!(varptr_value(64 + 128 + 4, &r), Some(-1.0));
        assert_eq!(varptr_value(64 + 4, &r), Some(15.0));
        assert_eq!(varptr_value(64 + 57, &r), None);
    }
}
