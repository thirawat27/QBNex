use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct Note {
    pub frequency: f32,
    pub duration_ms: u32,
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
        let notes = self.parse_mml(mml);

        for note in notes {
            self.play_note(note.frequency, note.duration_ms);
        }
    }

    fn parse_mml(&self, mml: &str) -> Vec<Note> {
        let mut notes = Vec::new();
        let mut tempo = 120;
        let mut octave = 4;
        let mut duration = 4;

        let chars: Vec<char> = mml.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let ch = chars[i];

            match ch.to_uppercase().next() {
                Some('T') => {
                    i += 1;
                    let mut num_str = String::new();
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        num_str.push(chars[i]);
                        i += 1;
                    }
                    if let Ok(t) = num_str.parse::<u32>() {
                        tempo = t;
                    }
                    continue;
                }
                Some('O') => {
                    i += 1;
                    if i < chars.len() && chars[i].is_ascii_digit() {
                        octave = chars[i].to_digit(10).unwrap_or(4);
                        i += 1;
                    }
                    continue;
                }
                Some('L') => {
                    i += 1;
                    let mut num_str = String::new();
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        num_str.push(chars[i]);
                        i += 1;
                    }
                    if let Ok(d) = num_str.parse::<u32>() {
                        duration = d;
                    }
                    continue;
                }
                Some('A') | Some('B') | Some('C') | Some('D') | Some('E') | Some('F')
                | Some('G') => {
                    let base_freq = self.get_note_frequency(ch, octave);
                    let note_len = 240000 / (tempo * duration);

                    if i + 1 < chars.len() && chars[i + 1] == '#' {
                        let sharp_freq = self.get_note_frequency_sharp(ch, octave);
                        notes.push(Note {
                            frequency: sharp_freq,
                            duration_ms: note_len,
                        });
                        i += 2;
                    } else if i + 1 < chars.len() && chars[i + 1] == '-' {
                        let flat_freq = self.get_note_frequency_flat(ch, octave);
                        notes.push(Note {
                            frequency: flat_freq,
                            duration_ms: note_len,
                        });
                        i += 2;
                    } else {
                        notes.push(Note {
                            frequency: base_freq,
                            duration_ms: note_len,
                        });
                        i += 1;
                    }

                    if i < chars.len() {
                        let next_ch = chars[i];
                        if next_ch.is_ascii_digit() {
                            let mut num_str = String::new();
                            while i < chars.len() && chars[i].is_ascii_digit() {
                                num_str.push(chars[i]);
                                i += 1;
                            }
                            if let Ok(d) = num_str.parse::<u32>() {
                                let note_len = 240000 / (tempo * d);
                                if let Some(note) = notes.last_mut() {
                                    note.duration_ms = note_len;
                                }
                            }
                            continue;
                        } else if next_ch == '.' {
                            if let Some(note) = notes.last_mut() {
                                note.duration_ms = (note.duration_ms as f32 * 1.5) as u32;
                            }
                            i += 1;
                        }
                    }
                    continue;
                }
                Some('R') => {
                    let note_len = 240000 / (tempo * duration);
                    notes.push(Note {
                        frequency: 0.0,
                        duration_ms: note_len,
                    });
                    i += 1;
                    continue;
                }
                Some('>') => {
                    if octave < 8 {
                        octave += 1;
                    }
                    i += 1;
                    continue;
                }
                Some('<') => {
                    if octave > 1 {
                        octave -= 1;
                    }
                    i += 1;
                    continue;
                }
                _ => {
                    i += 1;
                }
            }
        }

        notes
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

    fn get_note_frequency_sharp(&self, note: char, octave: u32) -> f32 {
        self.get_note_frequency(note, octave) * 1.059463
    }

    fn get_note_frequency_flat(&self, note: char, octave: u32) -> f32 {
        self.get_note_frequency(note, octave) / 1.059463
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
