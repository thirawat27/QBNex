//! Tiering policy: interpret first, compile when something gets hot.
//!
//! The VM calls [`Tiering::tick`] at SUB/FUNCTION entry (`pc = ENTRY_CALL`)
//! and at loop headers (back-edge targets, `pc` = block pc). When a counter
//! reaches the threshold the whole module is compiled once (its functions
//! compile in parallel; compiling only the hot ones would need calls from
//! native code into functions that are not compiled yet). From then on `tick` returns the native
//! code and the VM enters it at that point; with [`Mode::Eager`] the module
//! is compiled before the first statement runs (`qb --jit`).
//!
//! A big module (more than [`BACKGROUND_INSTS`] instructions) is compiled on
//! another thread in Hot mode: the VM keeps interpreting, and `tick` returns
//! the native code once it is ready, so a large program never stalls at its
//! first hot loop.

use crate::codegen::{compile, Compiled, Target};
use crate::mir::{FuncId, Module};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, TryRecvError};

/// Modules with more instructions than this compile in the background (Hot mode).
pub const BACKGROUND_INSTS: usize = 50_000;

/// Compiled code handed from the compiler thread to the VM thread.
struct Ready(Compiled);
// SAFETY: the code memory and function pointers are process-wide; the value is moved to
// the VM thread once and used only there.
unsafe impl Send for Ready {}

/// Instructions in `m` (what the compile time grows with).
fn size(m: &Module) -> usize {
    m.funcs.iter().flat_map(|f| &f.blocks).map(|b| b.insts.len() + 1).sum()
}

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
    /// background compilation in progress
    pending: Option<Receiver<Result<Ready, String>>>,
}

impl Tiering {
    pub fn new(mode: Mode, threshold: u32) -> Tiering {
        Tiering { mode, threshold: threshold.max(1), counts: HashMap::new(), compiled: None, failed: None, pending: None }
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

    /// Installs code compiled ahead of time: every tick returns it at once.
    pub fn preload(&mut self, c: Compiled) {
        self.compiled = Some(c);
        if self.mode == Mode::Off {
            self.mode = Mode::Hot;
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

    /// Hot mode: compiles `m` now when it is small, else on another thread;
    /// returns the code once it exists.
    fn ensure_hot(&mut self, m: &Module) -> Option<&Compiled> {
        if self.compiled.is_none() && self.failed.is_none() {
            match &self.pending {
                Some(rx) => match rx.try_recv() {
                    Ok(Ok(Ready(c))) => self.compiled = Some(c),
                    Ok(Err(e)) => self.failed = Some(e),
                    Err(TryRecvError::Empty) => return None,
                    Err(TryRecvError::Disconnected) => self.failed = Some("JIT compiler thread stopped".into()),
                },
                None if size(m) <= BACKGROUND_INSTS => return self.ensure(m),
                None => {
                    let (tx, rx) = std::sync::mpsc::channel();
                    let job = m.clone();
                    let spawned = std::thread::Builder::new().name("qb-jit".into()).spawn(move || {
                        let _ = tx.send(compile(&job, Target::Host).map(Ready).map_err(|e| e.0));
                    });
                    match spawned {
                        Ok(_) => self.pending = Some(rx),
                        Err(_) => return self.ensure(m),
                    }
                    return None;
                }
            }
            self.pending = None;
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
                if self.pending.is_some() {
                    return self.ensure_hot(m);
                }
                let c = self.counts.entry((func, pc)).or_insert(0);
                *c += 1;
                if *c >= self.threshold {
                    self.ensure_hot(m)
                } else {
                    None
                }
            }
        }
    }
}
