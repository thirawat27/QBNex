//! `qbrt` — the QBNex runtime, ported from `internal/c/libqb.cpp` to Rust one
//! subsystem at a time.
//!
//! The BASIC compiler still emits C++ that is compiled together with
//! `internal/c/qbx.cpp` and `libqb.cpp`. Each ported subsystem exports the
//! exact C symbol that libqb.cpp used to define, declared in
//! `include/qbrt.h`. The legacy C++ stays available behind a
//! `QBNEX_LEGACY_<SUBSYSTEM>` define so both implementations can be compared.
//!
//! Rules for every module (see docs/rust-port/port-plan.md, Revision 2):
//! - Behaviour is ported bug-for-bug; deviations need a conformance note.
//! - Exported functions never unwind across the FFI boundary (panic = abort).
//! - Pointers coming from C may be unaligned (they often point into string
//!   buffers), so reads and writes go through byte copies.

#![deny(unsafe_op_in_unsafe_fn)]

pub mod mbf;
pub mod mem;
pub mod numfmt;
pub mod rnd;
pub mod round;
pub mod timer;
pub mod val;

/// Version of the C ABI exposed by this crate. Bumped whenever a declaration in
/// `include/qbrt.h` changes incompatibly.
pub const QBRT_ABI_VERSION: u32 = 1;

/// Returns [`QBRT_ABI_VERSION`]; lets libqb.cpp assert it links the runtime it
/// was compiled against.
#[no_mangle]
pub extern "C" fn qbrt_abi_version() -> u32 {
    QBRT_ABI_VERSION
}
