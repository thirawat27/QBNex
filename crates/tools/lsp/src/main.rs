//! `qb-lsp`: QBasic/QB64/QBNex language server on standard input/output.

fn main() -> std::process::ExitCode {
    match qb_lsp::run_stdio() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("qb-lsp: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
