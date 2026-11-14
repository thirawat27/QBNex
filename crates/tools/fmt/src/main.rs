//! `qb-fmt`: formats QBasic/QB64/QBNex source files.

use std::io::{Read, Write};
use std::process::ExitCode;

use qb_fmt::{KeywordCase, LineEnding, Options, format_bytes};

const USAGE: &str = "\
usage: qb-fmt [options] [file.bas ...]

With no files, reads standard input and writes standard output.
With files, writes the formatted text to standard output unless --write
or --check is given.

options:
  -w, --write              rewrite the files in place
  --check                  write nothing; exit 1 if a file would change
  --keyword-case <c>       upper (default), lower or preserve
  --indent <n>             spaces per level (default 4)
  --tabs                   indent with tabs
  --line-ending <e>        auto (default), lf or crlf
  -h, --help               show this help";

fn main() -> ExitCode {
    let mut opts = Options::default();
    let mut write = false;
    let mut check = false;
    let mut files = Vec::new();
    let mut args = std::env::args().skip(1).peekable();
    // qb.cmd / qbnex.cmd pass "fmt" on (see there)
    if std::env::var("QBNEX_SUBCOMMAND").is_ok_and(|s| s.eq_ignore_ascii_case("fmt")) && args.peek().is_some_and(|a| a.eq_ignore_ascii_case("fmt")) {
        args.next();
    }
    while let Some(a) = args.next() {
        match a.as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "-w" | "--write" => write = true,
            "--check" => check = true,
            "--tabs" => opts.indent = "\t".into(),
            "--keyword-case" => {
                opts.keyword_case = match args.next().as_deref() {
                    Some("upper") => KeywordCase::Upper,
                    Some("lower") => KeywordCase::Lower,
                    Some("preserve") => KeywordCase::Preserve,
                    _ => return usage_error("--keyword-case needs upper, lower or preserve"),
                }
            }
            "--indent" => match args.next().and_then(|n| n.parse::<usize>().ok()) {
                Some(n) if n <= 16 => opts.indent = " ".repeat(n),
                _ => return usage_error("--indent needs a number from 0 to 16"),
            },
            "--line-ending" => {
                opts.line_ending = match args.next().as_deref() {
                    Some("auto") => LineEnding::Auto,
                    Some("lf") => LineEnding::Lf,
                    Some("crlf") => LineEnding::CrLf,
                    _ => return usage_error("--line-ending needs auto, lf or crlf"),
                }
            }
            s if s.starts_with('-') && s != "-" => return usage_error(&format!("unknown option {s}")),
            _ => files.push(a),
        }
    }

    if files.is_empty() {
        let mut input = Vec::new();
        if let Err(e) = std::io::stdin().read_to_end(&mut input) {
            eprintln!("qb-fmt: reading standard input: {e}");
            return ExitCode::from(2);
        }
        let out = format_bytes(&input, &opts);
        if check {
            return if out == input { ExitCode::SUCCESS } else { ExitCode::from(1) };
        }
        let _ = std::io::stdout().write_all(&out);
        return ExitCode::SUCCESS;
    }

    let mut changed = false;
    for f in &files {
        let input = match std::fs::read(f) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("qb-fmt: {f}: {e}");
                return ExitCode::from(2);
            }
        };
        let out = format_bytes(&input, &opts);
        if out != input {
            changed = true;
        }
        if check {
            if out != input {
                println!("{f}");
            }
        } else if write {
            if out != input
                && let Err(e) = std::fs::write(f, &out)
            {
                eprintln!("qb-fmt: {f}: {e}");
                return ExitCode::from(2);
            }
        } else {
            let _ = std::io::stdout().write_all(&out);
        }
    }
    if check && changed { ExitCode::from(1) } else { ExitCode::SUCCESS }
}

fn usage_error(msg: &str) -> ExitCode {
    eprintln!("qb-fmt: {msg}\n\n{USAGE}");
    ExitCode::from(2)
}
