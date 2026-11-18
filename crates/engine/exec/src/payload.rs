//! The program a compiled executable carries (layout: crate `qb-payload`), with
//! the compressed QIR text inflated.

use std::borrow::Cow;

/// zlib never turns one byte into more than about 1032, so a QIR length beyond
/// that ratio is a damaged file, not a reason to allocate gigabytes.
const MAX_INFLATE_RATIO: usize = 1100;

pub enum Payload<'a> {
    /// QIR text.
    Ir(Cow<'a, [u8]>),
    /// QIR text and the native image built from it.
    Native(&'a [u8], &'a [u8]),
}

/// The program at the end of `exe` and the offset where it starts (the bytes in
/// front of it are the runner, or the launcher with the packed runner). `None`
/// when `exe` carries no program or a damaged one.
pub fn find(exe: &[u8]) -> Option<(Payload<'_>, usize)> {
    let (program, at) = qb_payload::program(exe)?;
    let payload = match program {
        qb_payload::Program::Ir(q) => Payload::Ir(Cow::Borrowed(q)),
        qb_payload::Program::Native { qir, image } => Payload::Native(qir, image),
        qb_payload::Program::IrZlib { stream, len } => {
            if len > stream.len().saturating_mul(MAX_INFLATE_RATIO) {
                return None;
            }
            let mut qir = vec![0u8; len];
            let (rc, got) = qb_runtime::compress::inflate(stream, &mut qir);
            if rc != qb_runtime::compress::Z_OK || got != len {
                return None;
            }
            Payload::Ir(Cow::Owned(qir))
        }
    };
    Some((payload, at))
}

/// Environment variable through which the launcher `qb-stub` names the program
/// file it was started as; the runner it starts lives in a cache folder.
pub const PACKED_EXE_VAR: &str = "QBNEX_PACKED_EXE";

/// The program file named by the launcher, taken out of the environment so that a
/// program this one starts does not see it.
///
/// # Safety
/// No other thread may read or write the environment at the same time (call it
/// first thing in `main`).
pub unsafe fn take_packed_exe() -> Option<std::path::PathBuf> {
    let v = std::env::var_os(PACKED_EXE_VAR)?;
    // SAFETY: the caller's contract.
    unsafe { std::env::remove_var(PACKED_EXE_VAR) };
    (!v.is_empty()).then(|| v.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compressed_qir_is_inflated() {
        let qir = b"QIR 1\nfunc main\n".repeat(200);
        let z = qb_runtime::compress::deflate(&qir);
        let mut exe = b"RUNNER".to_vec();
        exe.extend_from_slice(&z);
        exe.extend_from_slice(&(z.len() as u64).to_le_bytes());
        exe.extend_from_slice(&(qir.len() as u64).to_le_bytes());
        exe.extend_from_slice(qb_payload::MAGIC_IR_ZLIB);
        match find(&exe) {
            Some((Payload::Ir(q), 6)) => assert_eq!(&q[..], &qir[..]),
            _ => panic!("not found"),
        }
        // a length the stream cannot hold, and a wrong length
        let n = exe.len();
        exe[n - 16..n - 8].copy_from_slice(&(u64::MAX / 2).to_le_bytes());
        assert!(find(&exe).is_none());
        exe[n - 16..n - 8].copy_from_slice(&(qir.len() as u64 + 1).to_le_bytes());
        assert!(find(&exe).is_none());
    }
}
