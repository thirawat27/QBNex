//! `qb-exec` executable.
//!
//! - Packaged program: the compiler copies this executable and appends
//!   `<QIR text><u64 LE length>QBNEXIR0`. At start-up the runner reads its own
//!   file and runs the payload.
//! - Development: `qb-exec program.qir` runs a QIR file directly.

use std::io::{BufWriter, Write};

const MAGIC: &[u8; 8] = b"QBNEXIR0";

fn payload(exe: &[u8]) -> Option<&[u8]> {
    let n = exe.len();
    if n < 16 || &exe[n - 8..] != MAGIC {
        return None;
    }
    let len = u64::from_le_bytes(exe[n - 16..n - 8].try_into().ok()?) as usize;
    exe.get(n - 16 - len..n - 16)
}

fn main() {
    let exe = std::env::current_exe().and_then(std::fs::read).unwrap_or_default();
    let src = match payload(&exe) {
        Some(p) => p.to_vec(),
        None => match std::env::args_os().nth(1) {
            Some(path) => std::fs::read(&path).unwrap_or_else(|e| {
                eprintln!("qb-exec: cannot read {}: {e}", path.to_string_lossy());
                std::process::exit(2)
            }),
            None => {
                eprintln!("usage: qb-exec <program.qir>");
                std::process::exit(2)
            }
        },
    };
    let src = String::from_utf8_lossy(&src);
    let stdout = std::io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let result = qb_exec::run_ir(&src, &mut out);
    let _ = out.flush();
    if let Err(e) = result {
        if e.code < 0 {
            eprintln!("qb-exec: invalid program: {}", e.msg);
        } else {
            eprintln!("Unhandled error #{} on line {}", e.code, e.line);
        }
        std::process::exit(1);
    }
}
