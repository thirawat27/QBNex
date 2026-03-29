use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct Note {
    pub frequency: f32,
    pub duration_ms: u32,
}

#[derive(Debug, Clone)]
pub struct ParsedMelody {
    pub notes: Vec<Note>,
    pub background: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Articulation {
    Normal,
    Legato,
    Staccato,
}

pub struct SoundSynth {
    _sample_rate: u32,
    notes: VecDeque<Note>,
    is_playing: bool,
}

impl SoundSynth {
    pub fn new() -> Self {
        Self {
            _sample_rate: 44100,
            notes: VecDeque::new(),
            is_playing: false,
        }
    }

    pub fn play_note(&mut self, frequency: f32, duration_ms: u32) {
        self.notes.push_back(Note {
            frequency,
            duration_ms,
        });
        self.is_playing = true;
    }

    pub fn play_melody(&mut self, mml: &str) {
        let notes = self.parse_mml(mml).notes;

        for note in notes {
            self.play_note(note.frequency, note.duration_ms);
        }
    }

    pub fn parse_melody(&self, mml: &str) -> ParsedMelody {
        self.parse_mml(mml)
    }

    pub fn drain_notes(&mut self) -> Vec<Note> {
        let drained = self.notes.drain(..).collect();
        self.is_playing = false;
        drained
    }

    fn parse_mml(&self, mml: &str) -> ParsedMelody {
        let mut notes = Vec::new();
        let mut tempo: u32 = 120;
        let mut octave: u32 = 4;
        let mut default_length: u32 = 4;
        let mut articulation = Articulation::Normal;
        let mut background = false;

        let chars: Vec<char> = mml.chars().collect();
        let mut i = 0usize;

        while i < chars.len() {
            let ch = chars[i].to_ascii_uppercase();

            match ch {
                ' ' | '\t' | '\r' | '\n' | ';' => {
                    i += 1;
                }
                'T' => {
                    i += 1;
                    if let Some(value) = Self::parse_number(&chars, &mut i) {
                        tempo = value.max(1);
                    }
                }
                'O' => {
                    i += 1;
                    if let Some(value) = Self::parse_number(&chars, &mut i) {
                        octave = value.clamp(1, 8);
                    }
                }
                'L' => {
                    i += 1;
                    if let Some(value) = Self::parse_number(&chars, &mut i) {
                        default_length = value.max(1);
                    }
                }
                'M' => {
                    i += 1;
                    if i < chars.len() {
                        match chars[i].to_ascii_uppercase() {
                            'L' => articulation = Articulation::Legato,
                            'N' => articulation = Articulation::Normal,
                            'S' => articulation = Articulation::Staccato,
                            'B' => background = true,
                            'F' => background = false,
                            _ => {
                                continue;
                            }
                        }
                        i += 1;
                    }
                }
                'A' | 'B' | 'C' | 'D' | 'E' | 'F' | 'G' => {
                    let mut frequency = self.get_note_frequency(ch, octave);
                    i += 1;

                    if i < chars.len() {
                        match chars[i] {
                            '#' | '+' => {
                                frequency *= 1.059463;
                                i += 1;
                            }
                            '-' => {
                                frequency /= 1.059463;
                                i += 1;
                            }
                            _ => {}
                        }
                    }

                    let note_length = Self::parse_note_length(&chars, &mut i, default_length);
                    let duration_ms =
                        Self::parse_dotted_duration(tempo, note_length, &chars, &mut i);
                    Self::push_note_with_articulation(
                        &mut notes,
                        frequency,
                        duration_ms,
                        articulation,
                    );
                }
                'N' => {
                    i += 1;
                    let note_number = Self::parse_number(&chars, &mut i).unwrap_or(0);
                    let duration_ms =
                        Self::parse_dotted_duration(tempo, default_length, &chars, &mut i);
                    if note_number == 0 {
                        Self::push_rest(&mut notes, duration_ms);
                    } else {
                        Self::push_note_with_articulation(
                            &mut notes,
                            self.get_note_number_frequency(note_number),
                            duration_ms,
                            articulation,
                        );
                    }
                }
                'P' => {
                    i += 1;
                    let pause_length = match Self::parse_number(&chars, &mut i) {
                        Some(0) => 0,
                        Some(value) => value,
                        None => default_length,
                    };
                    let duration_ms =
                        Self::parse_dotted_duration(tempo, pause_length, &chars, &mut i);
                    Self::push_rest(&mut notes, duration_ms);
                }
                'R' => {
                    i += 1;
                    let rest_length = Self::parse_note_length(&chars, &mut i, default_length);
                    let duration_ms =
                        Self::parse_dotted_duration(tempo, rest_length, &chars, &mut i);
                    Self::push_rest(&mut notes, duration_ms);
                }
                '>' => {
                    if octave < 8 {
                        octave += 1;
                    }
                    i += 1;
                }
                '<' => {
                    if octave > 1 {
                        octave -= 1;
                    }
                    i += 1;
                }
                _ => {
                    i += 1;
                }
            }
        }

        ParsedMelody { notes, background }
    }

    fn parse_number(chars: &[char], index: &mut usize) -> Option<u32> {
        let start = *index;
        let mut digits = String::new();
        while *index < chars.len() && chars[*index].is_ascii_digit() {
            digits.push(chars[*index]);
            *index += 1;
        }
        if *index == start {
            None
        } else {
            digits.parse::<u32>().ok()
        }
    }

    fn parse_note_length(chars: &[char], index: &mut usize, default_length: u32) -> u32 {
        match Self::parse_number(chars, index) {
            Some(0) | None => default_length,
            Some(value) => value,
        }
    }

    fn parse_dotted_duration(tempo: u32, length: u32, chars: &[char], index: &mut usize) -> u32 {
        let base_duration = Self::duration_ms(tempo, length);
        let mut total = base_duration as f64;
        let mut extra = base_duration as f64 / 2.0;
        while *index < chars.len() && chars[*index] == '.' {
            total += extra;
            extra /= 2.0;
            *index += 1;
        }
        total.round() as u32
    }

    fn duration_ms(tempo: u32, length: u32) -> u32 {
        if length == 0 {
            return 0;
        }
        240000 / (tempo.max(1) * length.max(1))
    }

    fn push_rest(notes: &mut Vec<Note>, duration_ms: u32) {
        if duration_ms == 0 {
            return;
        }
        notes.push(Note {
            frequency: 0.0,
            duration_ms,
        });
    }

    fn push_note_with_articulation(
        notes: &mut Vec<Note>,
        frequency: f32,
        duration_ms: u32,
        articulation: Articulation,
    ) {
        if duration_ms == 0 {
            return;
        }

        let sound_duration = match articulation {
            Articulation::Legato => duration_ms,
            Articulation::Normal => duration_ms.saturating_mul(7) / 8,
            Articulation::Staccato => duration_ms.saturating_mul(3) / 4,
        };
        let sound_duration = sound_duration.min(duration_ms);
        let rest_duration = duration_ms.saturating_sub(sound_duration);

        if sound_duration > 0 {
            notes.push(Note {
                frequency,
                duration_ms: sound_duration,
            });
        }
        if rest_duration > 0 {
            Self::push_rest(notes, rest_duration);
        }
    }

    fn get_note_frequency(&self, note: char, octave: u32) -> f32 {
        let note_val = match note.to_uppercase().next() {
            Some('C') => 0,
            Some('D') => 2,
            Some('E') => 4,
            Some('F') => 5,
            Some('G') => 7,
            Some('A') => 9,
            Some('B') => 11,
            _ => 0,
        };

        let semitones = (octave - 1) * 12 + note_val;
        440.0 * 2.0_f32.powf((semitones as f32 - 9.0) / 12.0)
    }

    fn get_note_number_frequency(&self, note_number: u32) -> f32 {
        let semitones = note_number.saturating_sub(1);
        440.0 * 2.0_f32.powf((semitones as f32 - 9.0) / 12.0)
    }

    pub fn beep(&mut self) {
        self.play_note(440.0, 200);
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing && !self.notes.is_empty()
    }

    pub fn stop(&mut self) {
        self.notes.clear();
        self.is_playing = false;
    }
}

impl Default for SoundSynth {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::SoundSynth;

    #[test]
    fn drain_notes_preserves_parsed_play_durations() {
        let mut synth = SoundSynth::new();
        synth.play_melody("T240MLL8C");

        let notes = synth.drain_notes();
        assert_eq!(notes.len(), 1);
        assert!(notes[0].frequency > 0.0);
        assert_eq!(notes[0].duration_ms, 125);
        assert!(!synth.is_playing());
    }

    #[test]
    fn music_mode_prefixes_do_not_create_spurious_notes() {
        let mut synth = SoundSynth::new();
        synth.play_melody("MFT240L64C");

        let notes = synth.drain_notes();
        assert_eq!(notes.iter().filter(|note| note.frequency > 0.0).count(), 1);
        assert_eq!(notes.iter().map(|note| note.duration_ms).sum::<u32>(), 15);
    }

    #[test]
    fn play_supports_pause_note_numbers_and_multiple_dots() {
        let mut synth = SoundSynth::new();
        synth.play_melody("T120MLP8N1..");

        let notes = synth.drain_notes();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].frequency, 0.0);
        assert_eq!(notes[0].duration_ms, 250);
        assert!(notes[1].frequency > 0.0);
        assert_eq!(notes[1].duration_ms, 875);
    }

    #[test]
    fn play_articulation_commands_split_note_and_rest_durations() {
        let mut synth = SoundSynth::new();
        synth.play_melody("T120L4MNCMLCMSC");

        let notes = synth.drain_notes();
        let timeline: Vec<(bool, u32)> = notes
            .iter()
            .map(|note| (note.frequency > 0.0, note.duration_ms))
            .collect();
        assert_eq!(
            timeline,
            vec![
                (true, 437),
                (false, 63),
                (true, 500),
                (true, 375),
                (false, 125),
            ]
        );
    }

    #[test]
    fn parse_melody_reports_background_mode() {
        let synth = SoundSynth::new();
        assert!(!synth.parse_melody("MFT120L8C").background);
        assert!(synth.parse_melody("MBT120L8C").background);
    }

    #[test]
    fn beep_enqueues_short_note() {
        let mut synth = SoundSynth::new();
        synth.beep();

        let notes = synth.drain_notes();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].duration_ms, 200);
        assert!(notes[0].frequency > 0.0);
    }
}
