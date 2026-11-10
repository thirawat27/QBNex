//! `qb-dap`: debug adapter on stdio. `qb-dap --debuggee <qir> <port>` is the
//! internal child mode that runs the program.

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = if args.first().map(String::as_str) == Some("--debuggee") {
        let (Some(qir), Some(port)) = (args.get(1), args.get(2).and_then(|p| p.parse().ok())) else {
            eprintln!("usage: qb-dap --debuggee <program.qir> <port> [--no-debug] [--stop-on-entry]");
            return std::process::ExitCode::from(2);
        };
        let flag = |f: &str| args.iter().any(|a| a == f);
        qb_dap::debuggee::main(qir, port, flag("--no-debug"), flag("--stop-on-entry"))
    } else {
        qb_dap::adapter::run_stdio()
    };
    std::process::ExitCode::from(u8::try_from(code.clamp(0, 255)).unwrap_or(1))
}
