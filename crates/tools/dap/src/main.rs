//! `qb-dap`: debug adapter on stdio. `qb-dap --debuggee <qir> <port>` is the
//! internal child mode that runs the program (`QBNEX_DAP_TOKEN` in its
//! environment, `--program-dir <dir>`: the folder of the `.bas` file).

fn main() -> std::process::ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    // the debuggee token is for this process only: the program (ENVIRON$) and
    // what it starts (SHELL) do not see it
    let token = std::env::var(qb_dap::adapter::TOKEN_VAR).unwrap_or_default();
    // SAFETY: no other thread exists yet, so changing the environment cannot race.
    unsafe { std::env::remove_var(qb_dap::adapter::TOKEN_VAR) };
    // qb.cmd / qbnex.cmd pass "dap" on (see there)
    if std::env::var("QBNEX_SUBCOMMAND").is_ok_and(|s| s.eq_ignore_ascii_case("dap")) && args.first().is_some_and(|a| a.eq_ignore_ascii_case("dap")) {
        args.remove(0);
    }
    let code = if args.first().map(String::as_str) == Some("--debuggee") {
        let (Some(qir), Some(port)) = (args.get(1), args.get(2).and_then(|p| p.parse().ok())) else {
            eprintln!("usage: qb-dap --debuggee <program.qir> <port> [--no-debug] [--stop-on-entry]");
            return std::process::ExitCode::from(2);
        };
        let flag = |f: &str| args.iter().any(|a| a == f);
        if let Some(k) = args.iter().position(|a| a == "--program-dir") {
            qb_ffi::set_program_dir(args.get(k + 1).map(std::path::Path::new));
        }
        qb_dap::debuggee::main(qir, port, &token, flag("--no-debug"), flag("--stop-on-entry"))
    } else {
        qb_dap::adapter::run_stdio()
    };
    std::process::ExitCode::from(u8::try_from(code.clamp(0, 255)).unwrap_or(1))
}
