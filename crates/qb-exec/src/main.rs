//! `qb-exec` executable.
//!
//! - Packaged program: the compiler copies this executable and appends
//!   `<QIR text><u64 LE length>QBNEXIR0`. At start-up the runner reads its own
//!   file and runs the payload (VM + JIT).
//! - Native program (D15): `qb-exec build program.qir -o program.exe` appends
//!   `<QIR text><native image><u64 QIR length><u64 image length>QBNEXAO1`;
//!   the machine code was generated at build time, so nothing is compiled
//!   when the program starts.
//! - Development: `qb-exec program.qir` runs a QIR file directly.

use std::io::{BufWriter, Write};

const MAGIC: &[u8; 8] = b"QBNEXIR0";
const MAGIC_NATIVE: &[u8; 8] = b"QBNEXAO1";

enum Payload<'a> {
    Ir(&'a [u8]),
    Native(&'a [u8], &'a [u8]),
}

fn le(b: &[u8]) -> Option<usize> {
    Some(u64::from_le_bytes(b.try_into().ok()?) as usize)
}

fn payload(exe: &[u8]) -> Option<Payload<'_>> {
    let n = exe.len();
    if n >= 24 && &exe[n - 8..] == MAGIC_NATIVE {
        let (ql, il) = (le(&exe[n - 24..n - 16])?, le(&exe[n - 16..n - 8])?);
        let end = n - 24;
        let img = exe.get(end.checked_sub(il)?..end)?;
        let qir = exe.get(end - il - ql..end - il)?;
        return Some(Payload::Native(qir, img));
    }
    if n < 16 || &exe[n - 8..] != MAGIC {
        return None;
    }
    let len = le(&exe[n - 16..n - 8])?;
    exe.get(n - 16 - len..n - 16).map(Payload::Ir)
}

fn read(path: &std::ffi::OsStr) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("qb-exec: cannot read {}: {e}", path.to_string_lossy());
        std::process::exit(2)
    })
}

/// `qb-exec build <program.qir> -o <out.exe>`
fn build(args: &[std::ffi::OsString]) -> ! {
    let (Some(input), Some(out)) = (args.first(), args.iter().position(|a| a == "-o").and_then(|i| args.get(i + 1))) else {
        eprintln!("usage: qb-exec build <program.qir> -o <program.exe>");
        std::process::exit(2)
    };
    let qir = read(input);
    let image = qb_exec::build_native(&String::from_utf8_lossy(&qir)).unwrap_or_else(|e| {
        eprintln!("qb-exec: cannot build: {}", e.msg);
        std::process::exit(1)
    });
    let me = std::env::current_exe().map(|p| read(p.as_os_str())).unwrap_or_default();
    let base = match payload(&me) {
        // Building from an already packaged runner: strip its payload.
        Some(Payload::Ir(p)) => &me[..me.len() - 16 - p.len()],
        Some(Payload::Native(q, i)) => &me[..me.len() - 24 - q.len() - i.len()],
        None => &me[..],
    };
    let mut exe = base.to_vec();
    exe.extend_from_slice(&qir);
    exe.extend_from_slice(&image);
    exe.extend_from_slice(&(qir.len() as u64).to_le_bytes());
    exe.extend_from_slice(&(image.len() as u64).to_le_bytes());
    exe.extend_from_slice(MAGIC_NATIVE);
    if let Err(e) = std::fs::write(out, exe) {
        eprintln!("qb-exec: cannot write {}: {e}", out.to_string_lossy());
        std::process::exit(2)
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(out, std::fs::Permissions::from_mode(0o755));
    }
    std::process::exit(0)
}

fn main() {
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    let exe = std::env::current_exe().and_then(std::fs::read).unwrap_or_default();
    let packaged = payload(&exe);
    if packaged.is_none() && args.first().is_some_and(|a| a == "build") {
        build(&args[1..]);
    }
    let (src, image) = match packaged {
        Some(Payload::Ir(p)) => (p.to_vec(), None),
        Some(Payload::Native(q, i)) => (q.to_vec(), Some(i.to_vec())),
        None => match args.first() {
            Some(path) => (read(path), None),
            None => {
                eprintln!("usage: qb-exec <program.qir> | qb-exec build <program.qir> -o <program.exe>");
                std::process::exit(2)
            }
        },
    };
    let src = String::from_utf8_lossy(&src);
    let stdout = std::io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let result = qb_exec::run_program(&src, image.as_deref(), &mut out);
    let _ = out.flush();
    match result {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            if e.code < 0 {
                eprintln!("qb-exec: invalid program: {}", e.msg);
            } else if e.msg != "reported" {
                eprintln!("Unhandled error #{} on line {}", e.code, e.line);
            }
            std::process::exit(1);
        }
    }
}
