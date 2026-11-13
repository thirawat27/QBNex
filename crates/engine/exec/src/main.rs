//! `qb-exec` executable.
//!
//! - Packaged program: the compiler copies this executable and appends
//!   `<QIR text><u64 LE length>QBNEXIR0`. At start-up the runner reads its own
//!   file and runs the payload (VM + JIT).
//! - Native program (D15): `qb-exec build program.qir -o program.exe` appends
//!   `<QIR text><native image><u64 QIR length><u64 image length>QBNEXAO1`;
//!   the machine code was generated at build time, so nothing is compiled
//!   when the program starts.
//! - Native executable (D15 final, default when the native kit is present):
//!   `qb-exec build program.qir -o program.exe` compiles the program into an
//!   object file and links it with rust-lld and the `qb-native` runtime
//!   library into an ordinary executable (`qb_link::toolchain`). The two
//!   payload forms above are the fallback (`--payload`).
//! - `qb-exec native-kit ...` builds the kit (see `qb_link::toolchain`).
//! - Development: `qb-exec program.qir` runs a QIR file directly.

use std::io::{BufWriter, Write};

// program stdout (CR LF line ends on Windows), shared with qb-native
use qb_exec::Stdout;

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
        let qir = exe.get(end.checked_sub(il)?.checked_sub(ql)?..end - il)?;
        return Some(Payload::Native(qir, img));
    }
    if n < 16 || &exe[n - 8..] != MAGIC {
        return None;
    }
    let len = le(&exe[n - 16..n - 8])?;
    // a length that does not fit inside the file is not a payload
    exe.get((n - 16).checked_sub(len)?..n - 16).map(Payload::Ir)
}

fn read(path: &std::ffi::OsStr) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("qb-exec: cannot read {}: {e}", path.to_string_lossy());
        std::process::exit(2)
    })
}

fn opt<'a>(args: &'a [std::ffi::OsString], name: &str) -> Option<&'a std::ffi::OsString> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1))
}

/// `qb-exec native-kit --staticlib <lib> --lld <rust-lld> -o <kit dir>` plus
/// `[--system <dir>]` (Windows: the DLLs, default System32) or
/// `--libs "<rustc native-static-libs>" [--sdk <dir>]` (Linux, macOS).
fn native_kit(args: &[std::ffi::OsString]) -> ! {
    let (Some(lib), Some(lld), Some(out)) = (opt(args, "--staticlib"), opt(args, "--lld"), opt(args, "-o")) else {
        eprintln!(
            "usage: qb-exec native-kit --staticlib <qb_native.lib | libqb_native.a> --lld <rust-lld> -o <kit dir>\n  \
             Windows: [--system <dir>]   Linux/macOS: --libs \"<native-static-libs>\" [--sdk <macOS SDK>]"
        );
        std::process::exit(2)
    };
    let r = if cfg!(windows) {
        let sys = opt(args, "--system").map(std::path::PathBuf::from).unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into())).join("System32")
        });
        qb_link::toolchain::make_kit_windows(lib.as_ref(), lld.as_ref(), &sys, out.as_ref())
    } else {
        let libs = opt(args, "--libs").map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let sdk = opt(args, "--sdk").map(std::path::PathBuf::from).or_else(|| std::env::var_os("SDKROOT").map(Into::into)).or_else(|| {
            let o = std::process::Command::new("xcrun").args(["--sdk", "macosx", "--show-sdk-path"]).output().ok()?;
            let p = String::from_utf8_lossy(&o.stdout).trim().to_string();
            (o.status.success() && !p.is_empty()).then(|| p.into())
        });
        qb_link::toolchain::make_kit_unix(lib.as_ref(), lld.as_ref(), &libs, sdk.as_deref(), out.as_ref())
    };
    match r {
        Ok(s) => {
            println!("{s}");
            std::process::exit(0)
        }
        Err(e) => {
            eprintln!("qb-exec: {e}");
            std::process::exit(1)
        }
    }
}

/// `qb-exec build <program.qir> -o <out.exe> [--native-link | --payload]`
///
/// Default: a standard executable linked by rust-lld with the native kit
/// when the kit is present (`--native-link` requires it), else the runner
/// with the program appended (`--payload` forces that). A GUI program (no
/// `meta console`) always gets the runner: native executables are console-only.
fn build(args: &[std::ffi::OsString]) -> ! {
    let (Some(input), Some(out)) = (args.first(), opt(args, "-o")) else {
        eprintln!("usage: qb-exec build <program.qir> -o <program.exe> [--native-link | --payload]");
        std::process::exit(2)
    };
    let qir = read(input);
    let mut force_link = args.iter().any(|a| a == "--native-link");
    let mut payload_only = args.iter().any(|a| a == "--payload");
    if qb_exec::wants_window(&String::from_utf8_lossy(&qir)) {
        // native executables are console-only: a GUI program (no $CONSOLE)
        // needs the runner, which opens its window
        if force_link {
            eprintln!("qb-exec: GUI program (no $CONSOLE): native executables are console-only; writing the runner with the program");
        }
        (force_link, payload_only) = (false, true);
    }
    let me_path = std::env::current_exe().ok();
    let kit = if payload_only { None } else { qb_link::toolchain::find_kit(me_path.as_deref()) };
    if force_link && kit.is_none() {
        eprintln!("qb-exec: --native-link: no native kit (set QBNEX_NATIVE_KIT or install native/ next to qb-exec)");
        std::process::exit(2)
    }
    if let Some(kit) = kit {
        let obj = qb_exec::build_native_object(&String::from_utf8_lossy(&qir)).unwrap_or_else(|e| {
            eprintln!("qb-exec: cannot build: {}", e.msg);
            std::process::exit(1)
        });
        match qb_link::toolchain::link_native(&kit, &obj, out.as_ref()) {
            Ok(()) => std::process::exit(0),
            Err(e) if force_link => {
                eprintln!("qb-exec: {e}");
                std::process::exit(1)
            }
            Err(e) => eprintln!("qb-exec: {e}; writing the runner with the program instead"),
        }
    }
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
    let text = String::from_utf8_lossy(&qir);
    if qb_exec::wants_window(&text) && !qb_exec::wants_console(&text) {
        windows_gui_subsystem(&mut exe);
    }
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

/// Marks a Windows executable as a GUI program, so starting it opens no console
/// window next to its own (a program with `$CONSOLE` keeps the console). Other
/// executable formats are left as they are.
fn windows_gui_subsystem(exe: &mut [u8]) {
    const PE_OFFSET_AT: usize = 0x3C;
    /// From the PE signature: signature (4), file header (20), then the
    /// subsystem field 68 bytes into the optional header.
    const SUBSYSTEM_FROM_PE: usize = 4 + 20 + 68;
    const CONSOLE: u16 = 3;
    const GUI: u16 = 2;
    if exe.len() < PE_OFFSET_AT + 4 || &exe[..2] != b"MZ" {
        return;
    }
    let pe = u32::from_le_bytes([exe[PE_OFFSET_AT], exe[PE_OFFSET_AT + 1], exe[PE_OFFSET_AT + 2], exe[PE_OFFSET_AT + 3]]) as usize;
    let Some(at) = pe.checked_add(SUBSYSTEM_FROM_PE).filter(|&at| at + 2 <= exe.len()) else { return };
    if exe.get(pe..pe + 4) == Some(b"PE\0\0") && u16::from_le_bytes([exe[at], exe[at + 1]]) == CONSOLE {
        exe[at..at + 2].copy_from_slice(&GUI.to_le_bytes());
    }
}

fn main() {
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    // QBNEX_OWN_CONSOLE=1 tells the program that its console window was opened for
    // it (started from Explorer, not from a terminal) and closes when it ends; qb
    // uses it to explain itself instead of flashing its help. Set or cleared here,
    // so a program started from such a program gets its own answer.
    // SAFETY: no other thread exists yet, so changing the environment cannot race.
    unsafe {
        if qb_runtime::console::stdio::console_is_own() {
            std::env::set_var("QBNEX_OWN_CONSOLE", "1");
        } else {
            std::env::remove_var("QBNEX_OWN_CONSOLE");
        }
    }
    let exe = std::env::current_exe().and_then(std::fs::read).unwrap_or_default();
    let packaged = payload(&exe);
    if packaged.is_none() && args.first().is_some_and(|a| a == "build") {
        build(&args[1..]);
    }
    if packaged.is_none() && args.first().is_some_and(|a| a == "native-kit") {
        native_kit(&args[1..]);
    }
    let (src, image) = match packaged {
        Some(Payload::Ir(p)) => (p.to_vec(), None),
        Some(Payload::Native(q, i)) => (q.to_vec(), Some(i.to_vec())),
        None => match args.first() {
            Some(path) => {
                // `qb-exec prog.qir a b`: the program sees [prog.qir, a, b]
                qb_exec::set_program_args(args.iter().map(|a| a.to_string_lossy().into_owned()).collect());
                (read(path), None)
            }
            None => {
                eprintln!("usage: qb-exec <program.qir> | qb-exec build <program.qir> -o <program.exe>");
                std::process::exit(2)
            }
        },
    };
    let src = String::from_utf8_lossy_owned(src);
    if qb_exec::wants_window(&src) {
        // GUI program (no $CONSOLE): the window layer needs the main thread,
        // the program runs on its own thread and ends the process
        let hidden = qb_exec::screen_hidden(&src);
        let resize = qb_exec::window_resize(&src);
        let program = std::thread::Builder::new()
            .name("qb-program".into())
            .stack_size(64 << 20)
            .spawn(move || {
                // stdout stays unlocked: the window thread writes `snapshot` lines of input scripts
                let mut out = Stdout(BufWriter::new(std::io::stdout()));
                let result = qb_exec::run_program_window(&src, image.as_deref(), &mut out);
                let _ = out.flush();
                finish(result)
            })
            .unwrap_or_else(|e| {
                eprintln!("qb-exec: cannot start the program thread: {e}");
                std::process::exit(2)
            });
        qb_exec::run_window_main("Untitled", hidden, resize);
        let _ = program.join();
        std::process::exit(1);
    }
    let stdout = std::io::stdout();
    let mut out = Stdout(BufWriter::new(stdout.lock()));
    let result = qb_exec::run_program(&src, image.as_deref(), &mut out);
    let _ = out.flush();
    finish(result)
}

/// Exit status of a finished program (the error report goes to stderr).
fn finish(result: Result<i32, qb_exec::RunError>) -> ! {
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
