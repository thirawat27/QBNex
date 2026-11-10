//! `qb-runtime-audio` — the QBNex audio runtime in Rust.
//!
//! Replaces `internal/c/parts/audio/{out,decode,conversion}` (audio.cpp,
//! miniaudio, stb_vorbis, minimp3, libxmp-lite, tinysoundfont, RADv2).
//!
//! - [`synth`] / [`mml`]: SOUND, BEEP, PLAY, `_VOICE`, `_ADSR`, `_WAVE`,
//!   device independent and rendered offline in tests.
//! - [`queue`]: the raw sample-frame queue behind `_SNDRAW` and the synth voices.
//! - [`engine`]: sound handles and the mixer; one mutex guards all state that
//!   the audio callback reads (fixes debt F3).
//! - [`decode`]: file decoding (symphonia, xmrsplayer).
//! - [`device`]: cpal output on its own thread.
//! - [`ffi`]: the `qb_snd_*` C ABI used by `internal/c/libqb/audio_bridge.cpp`.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod mml;
pub mod synth;
pub mod decode;
pub mod device;
pub mod engine;
pub mod ffi;
pub mod queue;
