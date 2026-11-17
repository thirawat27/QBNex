//! `qb-lsp`: QBasic/QB64/QBNex language server on standard input/output.

fn main() -> std::process::ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    // qb.cmd / qbnex.cmd pass "lsp" on (see there)
    if std::env::var("QBNEX_SUBCOMMAND").is_ok_and(|s| s.eq_ignore_ascii_case("lsp")) && args.first().is_some_and(|a| a.eq_ignore_ascii_case("lsp")) {
        args.remove(0);
    }
    // other arguments are ignored: editors add their own (--stdio)
    match args.first().map(String::as_str) {
        Some("-h" | "--help") => {
            println!("usage: qb-lsp\n\nLanguage server for QBasic/QB64/QBNex on standard input/output.\nAn editor starts it; it is not used from a terminal.");
            return std::process::ExitCode::SUCCESS;
        }
        Some("-V" | "--version") => {
            println!("qb-lsp {}", env!("CARGO_PKG_VERSION"));
            return std::process::ExitCode::SUCCESS;
        }
        _ => {}
    }
    match qb_lsp::run_stdio() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        // the LSP specification: exit code 1 when `exit` came without `shutdown`
        Err(e) if e.is::<qb_lsp::ExitWithoutShutdown>() => std::process::ExitCode::FAILURE,
        // lsp-server's words for standard input that ended before an exit message
        Err(e) if e.to_string() == "disconnected channel" => {
            eprintln!("qb-lsp: the input ended without an exit message (an editor starts qb-lsp; see qb-lsp --help)");
            std::process::ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("qb-lsp: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
