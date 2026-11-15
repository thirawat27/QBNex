//! Raw sample-frame queue (legacy `SampleFrameBlockQueue`): the buffer behind
//! `_SNDRAW` handles and the four synth voices.
//!
//! Frames are pushed in blocks of `rate >> 4` stereo frames. A block is only
//! played once it is full or "forced" (`_SNDRAWDONE`, end of a PLAY/SOUND),
//! as in legacy. Unlike legacy, the mixer pulls frames directly from the queue
//! under the engine lock instead of `snd_mainloop` copying blocks into a
//! ping-pong buffer that the device thread reads without synchronisation.

use std::collections::VecDeque;

use crate::synth::NoteGen;

struct Block {
    samples: Vec<f32>, // interleaved L R, len = frames written * 2
    force: bool,
}

/// What plays after the blocks once a long note was queued: the note,
/// generated a block at a time, and the frames queued behind it.
enum Later {
    Note(Box<NoteGen>),
    Frames(Vec<i16>),
}

pub struct RawQueue {
    block_frames: usize,
    blocks: VecDeque<Block>,
    later: VecDeque<Later>,
    /// The block being generated from a note.
    scratch: Vec<i16>,
    /// Frames of the front block already played.
    read: usize,
    /// Frames pushed and not yet played (also those in `later`).
    queued: usize,
    /// Frames pushed since the queue was made (also those in `later`).
    pushed: u64,
    /// Frame positions (counted like `pushed`) where PLAY notes end.
    note_ends: VecDeque<u64>,
    pub volume: f32,
    pub pan: f32,
}

impl RawQueue {
    pub fn new(rate: u32) -> Self {
        RawQueue { block_frames: ((rate >> 4) as usize).max(1), blocks: VecDeque::new(), later: VecDeque::new(), scratch: Vec::new(), read: 0, queued: 0, pushed: 0, note_ends: VecDeque::new(), volume: 1.0, pan: 0.0 }
    }

    pub fn block_frames(&self) -> usize {
        self.block_frames
    }

    pub fn push(&mut self, l: f32, r: f32) {
        self.push_block(l, r);
        self.queued += 1;
        self.pushed += 1;
    }

    fn push_block(&mut self, l: f32, r: f32) {
        let need_new = match self.blocks.back() {
            Some(b) => b.samples.len() >= self.block_frames * 2,
            None => true,
        };
        if need_new {
            self.blocks.push_back(Block { samples: Vec::with_capacity(self.block_frames * 2), force: false });
        }
        let Some(b) = self.blocks.back_mut() else { return };
        b.samples.push(l);
        b.samples.push(r);
    }

    /// Records where PLAY notes end, in frames from the start of the next push.
    pub fn mark_note_ends(&mut self, ends: &[usize]) {
        let base = self.pushed;
        self.note_ends.extend(ends.iter().map(|&e| base + e as u64));
    }

    /// PLAY notes queued and not fully played (the `ON PLAY(n)` count).
    pub fn notes_remaining(&mut self) -> usize {
        let played = self.pushed - self.queued as u64;
        while self.note_ends.front().is_some_and(|&e| e <= played) {
            self.note_ends.pop_front();
        }
        self.note_ends.len()
    }

    /// Pushes 16-bit stereo frames (legacy `SendWaveformToQueue`) and forces
    /// the last block so a short sound plays without waiting for more data.
    pub fn push_i16(&mut self, frames: &[i16]) {
        if !self.later.is_empty() {
            let n = frames.len() / 2;
            if n == 0 {
                return;
            }
            self.later.push_back(Later::Frames(frames[..n * 2].to_vec()));
            self.queued += n;
            self.pushed += n as u64;
            return;
        }
        for f in frames.as_chunks::<2>().0 {
            self.push(f[0] as f32 / 32768.0, f[1] as f32 / 32768.0);
        }
        self.force_last();
    }

    /// Queues a long note: it is generated a block at a time as it plays.
    pub fn push_note(&mut self, note: NoteGen) {
        let n = note.remaining();
        self.later.push_back(Later::Note(Box::new(note)));
        self.queued += n;
        self.pushed += n as u64;
    }

    /// The blocks ran out: the next block comes from `later`.
    fn refill(&mut self) {
        let Some(front) = self.later.front_mut() else { return };
        match front {
            Later::Note(g) => {
                self.scratch.resize(self.block_frames * 2, 0);
                let n = g.fill(&mut self.scratch);
                if g.remaining() == 0 {
                    self.later.pop_front();
                }
                let scratch = std::mem::take(&mut self.scratch);
                for f in scratch[..n * 2].as_chunks::<2>().0 {
                    self.push_block(f[0] as f32 / 32768.0, f[1] as f32 / 32768.0);
                }
                self.scratch = scratch;
            }
            Later::Frames(_) => {
                let Some(Later::Frames(v)) = self.later.pop_front() else { return };
                for f in v.as_chunks::<2>().0 {
                    self.push_block(f[0] as f32 / 32768.0, f[1] as f32 / 32768.0);
                }
            }
        }
        self.force_last();
    }

    pub fn force_last(&mut self) {
        if let Some(b) = self.blocks.back_mut() {
            b.force = true;
        }
    }

    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    /// Frames still to be played.
    pub fn frames_remaining(&self) -> usize {
        self.queued
    }

    fn front_ready(&self) -> bool {
        match self.blocks.front() {
            Some(b) => b.force || b.samples.len() >= self.block_frames * 2,
            None => false,
        }
    }

    /// Pops the next frame if the front block may play; None means silence.
    pub fn next_frame(&mut self) -> Option<(f32, f32)> {
        if self.blocks.is_empty() {
            self.refill();
        }
        if !self.front_ready() {
            return None;
        }
        let b = self.blocks.front()?;
        let frame = (b.samples[2 * self.read], b.samples[2 * self.read + 1]);
        self.read += 1;
        self.queued -= 1;
        if self.read * 2 >= b.samples.len() {
            self.blocks.pop_front();
            self.read = 0;
        }
        Some(frame)
    }

    /// Drains up to `max` frames without mixing (tests and offline rendering).
    pub fn drain(&mut self, max: usize) -> Vec<f32> {
        let mut out = Vec::new();
        while out.len() < max * 2 {
            match self.next_frame() {
                Some((l, r)) => {
                    out.push(l);
                    out.push(r);
                }
                None => break,
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_play_only_when_full_or_forced() {
        let mut q = RawQueue::new(160); // 10 frames per block
        assert_eq!(q.block_frames(), 10);
        for i in 0..15 {
            q.push(i as f32, -(i as f32));
        }
        assert_eq!(q.block_count(), 2);
        assert_eq!(q.frames_remaining(), 15);
        let first = q.drain(100);
        assert_eq!(first.len(), 20); // second block not full yet
        assert_eq!(first[2], 1.0);
        assert_eq!(first[3], -1.0);
        assert_eq!(q.next_frame(), None);
        q.force_last();
        assert_eq!(q.drain(100).len(), 10);
        assert_eq!(q.frames_remaining(), 0);
        assert_eq!(q.block_count(), 0);
    }

    #[test]
    fn i16_frames_are_scaled_and_forced() {
        let mut q = RawQueue::new(48000);
        q.push_i16(&[16384, -32768, 0, 32767]);
        assert_eq!(q.drain(10), vec![0.5, -1.0, 0.0, 32767.0 / 32768.0]);
    }

    /// A long note plays from blocks generated as it goes, and what is
    /// queued after it plays after it.
    #[test]
    fn long_notes_play_in_order() {
        use crate::synth::{generate_waveform, LegacyRand, NoteGen, Voice};
        let mut q = RawQueue::new(160); // 10 frames per block
        let v = Voice::default();
        let whole = generate_waveform(&mut v.clone(), &mut LegacyRand::default(), 160, 50.0, 0.4, 1.0);
        q.push_i16(&[1000, 1000]);
        q.push_note(NoteGen::new(v, LegacyRand::default(), 160, 50.0, 0.4, 1.0));
        q.mark_note_ends(&[1]);
        q.push_i16(&[-1000, -1000]);
        assert_eq!(q.frames_remaining(), 1 + 64 + 1);
        assert_eq!(q.notes_remaining(), 1);
        let out = q.drain(1000);
        assert_eq!(out.len(), 2 * 66);
        assert_eq!(out[0], 1000.0 / 32768.0);
        let note: Vec<f32> = whole.iter().map(|&x| x as f32 / 32768.0).collect();
        assert_eq!(&out[2..2 + 128], &note[..]);
        assert_eq!(out[130], -1000.0 / 32768.0);
        assert_eq!((q.frames_remaining(), q.notes_remaining()), (0, 0));
    }

    #[test]
    fn note_ends_count_down_as_frames_play() {
        let mut q = RawQueue::new(160);
        q.mark_note_ends(&[2, 5]);
        q.push_i16(&[0; 10]); // 5 frames: two notes
        assert_eq!(q.notes_remaining(), 2);
        q.drain(2);
        assert_eq!(q.notes_remaining(), 1);
        q.drain(3);
        assert_eq!(q.notes_remaining(), 0);
    }
}
