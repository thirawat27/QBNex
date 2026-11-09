//! Tiering policy: interpret first, compile when something gets hot.
//!
//! The VM calls [`Tiering::tick`] at SUB/FUNCTION entry (`pc = ENTRY_CALL`)
//! and at loop headers (back-edge targets, `pc` = block pc). When a counter
//! reaches the threshold the whole module is compiled once (Cranelift
//! compiles a module in milliseconds; per-function compilation buys little
//! and complicates JIT-to-JIT calls). From then on `tick` returns the native
//! code and the VM enters it at that point; with [`Mode::Eager`] the module
//! is compiled before the first statement runs (`qb --jit`).

use crate::codegen::{compile, Compiled, Target};
use crate::mir::{FuncId, Module};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Never compile.
    Off,
    /// Compile after `threshold` ticks at one point.
    Hot,
    /// Compile everything up front.
    Eager,
}

pub const DEFAULT_THRESHOLD: u32 = 1000;

pub struct Tiering {
    pub mode: Mode,
    pub threshold: u32,
    counts: HashMap<(FuncId, u32), u32>,
    compiled: Option<Compiled>,
    /// Compilation failed once: stay in the VM.
    failed: Option<String>,
}

impl Tiering {
    pub fn new(mode: Mode, threshold: u32) -> Tiering {
        Tiering { mode, threshold: threshold.max(1), counts: HashMap::new(), compiled: None, failed: None }
    }

    /// Reads `QB_JIT` (`off`, `eager`, or a threshold number) — default: hot, 1000.
    pub fn from_env() -> Tiering {
        match std::env::var("QB_JIT").ok().as_deref() {
            Some("off") | Some("0") => Tiering::new(Mode::Off, DEFAULT_THRESHOLD),
            Some("eager") | Some("all") => Tiering::new(Mode::Eager, 1),
            Some(n) => Tiering::new(Mode::Hot, n.parse().unwrap_or(DEFAULT_THRESHOLD)),
            None => Tiering::new(Mode::Hot, DEFAULT_THRESHOLD),
        }
    }

    pub fn compiled(&self) -> Option<&Compiled> {
        self.compiled.as_ref()
    }

    pub fn failure(&self) -> Option<&str> {
        self.failed.as_deref()
    }

    fn ensure(&mut self, m: &Module) -> Option<&Compiled> {
        if self.compiled.is_none() && self.failed.is_none() {
            match compile(m, Target::Host) {
                Ok(c) => self.compiled = Some(c),
                Err(e) => self.failed = Some(e.0),
            }
        }
        self.compiled.as_ref()
    }

    /// Called before running anything (eager mode compiles here).
    pub fn start(&mut self, m: &Module) -> Option<&Compiled> {
        if self.mode == Mode::Eager {
            self.ensure(m)
        } else {
            None
        }
    }

    /// Counts one arrival at `(func, pc)`; returns native code to enter now.
    pub fn tick(&mut self, m: &Module, func: FuncId, pc: u32) -> Option<&Compiled> {
        match self.mode {
            Mode::Off => None,
            Mode::Eager => self.ensure(m),
            Mode::Hot => {
                if self.compiled.is_some() {
                    return self.compiled.as_ref();
                }
                let c = self.counts.entry((func, pc)).or_insert(0);
                *c += 1;
                if *c >= self.threshold {
                    self.ensure(m)
                } else {
                    None
                }
            }
        }
    }
}
