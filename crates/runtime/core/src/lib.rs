//! `qb-runtime` — the QBNex runtime library (console, graphics, files, input,
//! networking, memory, number formatting) that the QIR runner `qb-exec` calls.
//!
//! Rules for every module:
//! - Behaviour matches QB64/QBasic bug-for-bug; every intentional difference
//!   has an ID in docs/dev/deviations.md and a conformance case.
//! - Raw pointers handed in by the runner may be unaligned (they often point
//!   into string buffers), so reads and writes go through byte copies.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod alloc;
pub mod clipboard;
pub mod compress;
pub mod console;
mod cmem;
pub mod error;
pub mod events;
pub mod fileio;
pub mod font;
pub mod fsops;
#[cfg(not(target_arch = "wasm32"))]
pub mod gamepad;
pub mod gfs;
pub mod gfx;
#[cfg(feature = "gpu")]
pub mod gpu_present;
#[cfg(all(test, feature = "gpu"))]
mod gpu_tests;
pub mod image;
pub mod keybuf;
pub mod keymap;
pub mod mathfn;
pub mod mbf;
pub mod mem;
pub mod net;
pub mod numfmt;
pub mod numparse;
pub mod printer;
pub mod program;
pub mod qbs;
pub mod rnd;
pub mod round;
pub mod scene3d;
pub mod screen;
pub mod shell;
pub mod systime;
pub mod timer;
pub mod val;
pub mod waits;
#[cfg(windows)]
pub mod win32;

// Audio runtime: kept in its own crate (own object file); linked into the runner.
extern crate qb_audio;
// Extension library: the Rust side of the stdlib's ext.* modules.
#[cfg(feature = "ext")]
extern crate qb_ext;
#[cfg(not(target_arch = "wasm32"))]
pub mod gl;
pub mod raster;
pub mod script;
#[cfg(not(target_arch = "wasm32"))]
pub mod window;

#[cfg(test)]
mod robustness_tests;
#[cfg(test)]
mod props_tests;
