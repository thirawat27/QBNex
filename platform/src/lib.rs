//! # HAL Layer
//!
//! Hardware Abstraction Layer providing cross-platform file I/O, graphics,
//! and sound support for QBasic programs.
//!
//! ## Example
//!
//! ```rust
//! use hal_layer::FileIO;
//!
//! let mut file_io = FileIO::new();
//! // file_io.open("test.txt", FileMode::Output).unwrap();
//! ```

pub mod file_io;
pub mod sound_synth;
pub mod vga_graphics;

pub use file_io::FileIO;
pub use sound_synth::SoundSynth;
pub use vga_graphics::VGAGraphics;
