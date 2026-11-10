//! Raw sample-frame queue (legacy `SampleFrameBlockQueue`): the buffer behind
//! `_SNDRAW` handles and the four synth voices.
//!
//! Frames are pushed in blocks of `rate >> 4` stereo frames. A block is only
//! played once it is full or "forced" (`_SNDRAWDONE`, end of a PLAY/SOUND),
//! as in legacy. Unlike legacy, the mixer pulls frames directly from the queue
//! under the engine lock instead of `snd_mainloop` copying blocks into a
//! ping-pong buffer that the device thread reads without synchronisation.

use std::collections::VecDeque;

struct Block {
    samples: Vec<f32>, // interleaved L R, len = frames written * 2
    force: bool,
}

pub struct RawQueue {
    block_frames: usize,
    blocks: VecDeque<Block>,
    /// Frames of the front block already played.
    read: usize,
    /// Frames pushed and not yet played.
    queued: usize,
    pub volume: f32,
    pub pan: f32,
}

impl RawQueue {
    pub fn new(rate: u32) -> Self {
        RawQueue { block_frames: ((rate >> 4) as usize).max(1), blocks: VecDeque::new(), read: 0, queued: 0, volume: 1.0, pan: 0.0 }
    }

    pub fn block_frames(&self) -> usize {
        self.block_frames
    }

    pub fn push(&mut self, l: f32, r: f32) {
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
        self.queued += 1;
    }

    /// Pushes 16-bit stereo frames (legacy `SendWaveformToQueue`) and forces
    /// the last block so a short sound plays without waiting for more data.
    pub fn push_i16(&mut self, frames: &[i16]) {
        for f in frames.as_chunks::<2>().0 {
            self.push(f[0] as f32 / 32768.0, f[1] as f32 / 32768.0);
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
}
