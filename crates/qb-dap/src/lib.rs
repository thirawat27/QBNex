//! Debug Adapter Protocol server for QBNex programs on the qb-exec VM.
//!
//! The adapter ([`adapter`]) speaks DAP on stdio. On launch it gets the
//! program's QIR (from `qb <file.bas> --ir`, or a given `.qir`) and starts
//! itself again as the debuggee ([`debuggee`]): a child process whose
//! stdin/stdout are the program's, running the VM with qb-exec's `debug`
//! hook and reporting stops over a loopback TCP connection.
//!
//! Supported: line breakpoints in the launched file, pause, continue, step
//! in/over/out (statement granularity), call stack of SUB/FUNCTION frames,
//! variables (scalars, strings, arrays, TYPEs) per frame plus globals,
//! evaluate/hover for `name`, `name.field`, `name(i, j)`.

pub mod adapter;
pub mod debuggee;
pub mod names;
pub mod protocol;
pub mod snapshot;
