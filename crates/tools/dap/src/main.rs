//! `qb-dap`: debug adapter on stdio. `qb-dap --debuggee <qir> <port>` is the
//! internal child mode that runs the program (`QBNEX_DAP_TOKEN` in its
//! environment, `--program-dir <dir>`: the folder of the `.bas` file,
//! `--exit-file <file>`: where the program's exit code goes, and after `--`
//! the program's own argv: [program, args...]).

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    // (the system's own strings: an argument or folder that is not Unicode must
    // not stop the debuggee, which passes them on to the program)
    let mut args: Vec<OsString> = std::env::args_os().skip(1).collect();
    // the debuggee token is for this process only: the program (ENVIRON$) and
    // what it starts (SHELL) do not see it
    let token = std::env::var(qb_dap::adapter::TOKEN_VAR).unwrap_or_default();
    // SAFETY: no other thread exists yet, so changing the environment cannot race.
    unsafe { std::env::remove_var(qb_dap::adapter::TOKEN_VAR) };
    // qb.cmd / qbnex.cmd pass "dap" on (see there)
    if std::env::var("QBNEX_SUBCOMMAND").is_ok_and(|s| s.eq_ignore_ascii_case("dap")) && args.first().is_some_and(|a| a.eq_ignore_ascii_case("dap")) {
        args.remove(0);
    }
    match args.first().and_then(|a| a.to_str()) {
        Some("-h" | "--help") => {
            println!("usage: qb-dap\n\nDebug adapter (Debug Adapter Protocol) for QBNex programs on standard input/output.\nAn editor starts it; it is not used from a terminal.");
            return ExitCode::SUCCESS;
        }
        Some("-V" | "--version") => {
            println!("qb-dap {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Some("--debuggee") => {}
        _ => return exit_code(qb_dap::adapter::run_stdio()),
    }
    // The program's own arguments follow `--`: options are looked for only
    // before it, so a program argument such as `--no-debug` stays the program's.
    let (opts, program_args) = match args.iter().position(|a| a.to_str() == Some("--")) {
        Some(k) => (&args[..k], Some(args[k + 1..].to_vec())),
        None => (&args[..], None),
    };
    let (Some(qir), Some(port)) = (opts.get(1), opts.get(2).and_then(|p| p.to_str()?.parse::<u16>().ok())) else {
        eprintln!("usage: qb-dap --debuggee <program.qir> <port> [--program-dir <dir>] [--exit-file <file>] [--no-debug] [--stop-on-entry] [-- <program> <args>...]");
        return ExitCode::from(2);
    };
    let flag = |f: &str| opts.iter().any(|a| a.to_str() == Some(f));
    let value = |f: &str| opts.iter().position(|a| a.to_str() == Some(f)).and_then(|k| opts.get(k + 1));
    if let Some(dir) = value("--program-dir") {
        qb_ffi::set_program_dir(Some(Path::new(dir)));
    }
    let o = qb_dap::debuggee::Options {
        qir: PathBuf::from(qir),
        port,
        token,
        no_debug: flag("--no-debug"),
        stop_on_entry: flag("--stop-on-entry"),
        program_args,
        exit_file: value("--exit-file").map(PathBuf::from),
    };
    // (the real exit code, -1 or 300 too, reaches the adapter through the exit file)
    exit_code(qb_dap::debuggee::main(&o))
}

fn exit_code(code: i32) -> ExitCode {
    ExitCode::from(u8::try_from(code.clamp(0, 255)).unwrap_or(1))
}
