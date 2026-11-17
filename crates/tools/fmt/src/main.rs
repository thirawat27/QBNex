//! `qb-fmt`: formats QBasic/QB64/QBNex source files.

use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use qb_fmt::{KeywordCase, LineEnding, Options, format_bytes};

const USAGE: &str = "\
usage: qb-fmt [options] [file.bas ...]

With no files, reads standard input and writes standard output; `-` among
the files is standard input too. With files, writes the formatted text to
standard output unless --write or --check is given.

options:
  -w, --write              rewrite the files in place
  --check                  write nothing; list the files that would change
                           and exit 1 if there are any
  --keyword-case <c>       upper (default), lower or preserve
  --indent <n>             spaces per level (default 4)
  --tabs                   indent with tabs
  --line-ending <e>        auto (default), lf or crlf
  -h, --help               show this help

Exit code: 0 done, 1 --check found a file to change, 2 an error (a file that
cannot be read or written; the other files are still done).";

fn main() -> ExitCode {
    // (the system's own strings: a file name that is not Unicode is still a file)
    let mut args: Vec<OsString> = std::env::args_os().skip(1).collect();
    // qb.cmd / qbnex.cmd pass "fmt" on (see there)
    if std::env::var("QBNEX_SUBCOMMAND").is_ok_and(|s| s.eq_ignore_ascii_case("fmt")) && args.first().is_some_and(|a| a.eq_ignore_ascii_case("fmt")) {
        args.remove(0);
    }
    ExitCode::from(run(args, &mut io::stdin().lock(), &mut io::stdout().lock(), &mut io::stderr()))
}

/// The whole command; returns the exit code.
fn run(args: Vec<OsString>, stdin: &mut dyn Read, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let mut opts = Options::default();
    let mut write = false;
    let mut check = false;
    let mut files: Vec<OsString> = Vec::new();
    let mut args = args.into_iter();
    while let Some(a) = args.next() {
        let word = a.to_str().map(str::to_string);
        let mut value = || args.next().and_then(|v| v.into_string().ok());
        match word.as_deref() {
            Some("-h" | "--help") => {
                return if writeln!(out, "{USAGE}").and_then(|()| out.flush()).is_ok() { 0 } else { 2 };
            }
            Some("-w" | "--write") => write = true,
            Some("--check") => check = true,
            Some("--tabs") => opts.indent = "\t".into(),
            Some("--keyword-case") => {
                opts.keyword_case = match value().as_deref() {
                    Some("upper") => KeywordCase::Upper,
                    Some("lower") => KeywordCase::Lower,
                    Some("preserve") => KeywordCase::Preserve,
                    _ => return usage_error(err, "--keyword-case needs upper, lower or preserve"),
                }
            }
            Some("--indent") => match value().and_then(|n| n.parse::<usize>().ok()) {
                Some(n) if n <= 16 => opts.indent = " ".repeat(n),
                _ => return usage_error(err, "--indent needs a number from 0 to 16"),
            },
            Some("--line-ending") => {
                opts.line_ending = match value().as_deref() {
                    Some("auto") => LineEnding::Auto,
                    Some("lf") => LineEnding::Lf,
                    Some("crlf") => LineEnding::CrLf,
                    _ => return usage_error(err, "--line-ending needs auto, lf or crlf"),
                }
            }
            Some(s) if s.starts_with('-') && s != "-" => return usage_error(err, &format!("unknown option {s}")),
            _ => files.push(a),
        }
    }
    if check && write {
        return usage_error(err, "--check and --write cannot be used together (--check writes nothing)");
    }
    // with no files, standard input (whose name is not listed by --check)
    let named = !files.is_empty();
    if !named {
        files.push("-".into());
    }

    let mut failed = false;
    let mut changed = false;
    for f in &files {
        let is_stdin = f.to_str() == Some("-");
        let name = Path::new(f).display().to_string();
        let input = if is_stdin {
            let mut b = Vec::new();
            stdin.read_to_end(&mut b).map(|_| b)
        } else {
            std::fs::read(f)
        };
        let input = match input {
            Ok(b) => b,
            Err(e) => {
                // reported, and the other files are still done
                let what = if is_stdin { "reading standard input".to_string() } else { name };
                let _ = writeln!(err, "qb-fmt: {what}: {e}");
                failed = true;
                continue;
            }
        };
        let formatted = format_bytes(&input, &opts);
        let differs = formatted != input;
        changed |= differs;
        if check {
            if differs && (named || !is_stdin) && writeln!(out, "{name}").is_err() {
                return stdout_failed(err);
            }
        } else if write && !is_stdin {
            if differs && let Err(e) = write_atomic(Path::new(f), &formatted) {
                let _ = writeln!(err, "qb-fmt: {name}: {e}");
                failed = true;
            }
        } else if out.write_all(&formatted).is_err() {
            // (standard input has no file to rewrite: with --write it goes to standard output)
            return stdout_failed(err);
        }
    }
    if out.flush().is_err() {
        return stdout_failed(err);
    }
    if failed {
        2
    } else if check && changed {
        1
    } else {
        0
    }
}

fn stdout_failed(err: &mut dyn Write) -> u8 {
    let _ = writeln!(err, "qb-fmt: cannot write to standard output");
    2
}

fn usage_error(err: &mut dyn Write, msg: &str) -> u8 {
    let _ = writeln!(err, "qb-fmt: {msg}\n\n{USAGE}");
    2
}

/// Replaces the file at `path` with `data` so that nothing ever sees it half
/// written and a failure leaves the old text: the text goes to a new file in
/// the same folder, which is then renamed over the original. The original's
/// permissions are kept, and a symbolic link keeps pointing at the file it
/// names (that file is the one replaced).
fn write_atomic(path: &Path, data: &[u8]) -> io::Result<()> {
    let target = if std::fs::symlink_metadata(path)?.file_type().is_symlink() { std::fs::canonicalize(path)? } else { path.to_path_buf() };
    // A file that cannot be written is not replaced either (the folder may
    // allow a rename where the file itself is read-only).
    drop(std::fs::OpenOptions::new().write(true).open(&target)?);
    let perms = std::fs::metadata(&target)?.permissions();
    let dir = match target.parent() {
        Some(d) if !d.as_os_str().is_empty() => d.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let name = target.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut last = io::Error::other("no free name for a temporary file");
    for k in 0..100u32 {
        let tmp = dir.join(format!(".{name}.qb-fmt-{}-{k}.tmp", std::process::id()));
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&tmp) {
            Ok(mut file) => {
                let r = file
                    .write_all(data)
                    .and_then(|()| file.sync_all())
                    .and_then(|()| file.set_permissions(perms.clone()))
                    .and_then(|()| {
                        drop(file);
                        std::fs::rename(&tmp, &target)
                    });
                if r.is_err() {
                    let _ = std::fs::remove_file(&tmp);
                }
                return r;
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => last = e,
            Err(e) => return Err(e),
        }
    }
    Err(last)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MESSY: &str = "for i=1 to 3\nprint i\nnext\n";

    fn tidy() -> Vec<u8> {
        format_bytes(MESSY.as_bytes(), &Options::default())
    }

    fn work_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("qb-fmt-unit-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// (exit code, standard output, standard error)
    fn qb_fmt(args: &[&OsString], stdin: &str) -> (u8, String, String) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run(args.iter().map(|a| (*a).clone()).collect(), &mut stdin.as_bytes(), &mut out, &mut err);
        (code, String::from_utf8(out).unwrap(), String::from_utf8(err).unwrap())
    }

    fn os(s: &str) -> OsString {
        OsString::from(s)
    }

    #[test]
    fn the_sample_needs_formatting() {
        assert_ne!(tidy(), MESSY.as_bytes());
    }

    /// A file that cannot be read is reported; the others are still done and
    /// the exit code is 2. --check lists every file that would change.
    #[test]
    fn missing_file_does_not_stop_the_batch() {
        let dir = work_dir("batch");
        let (a, b, c) = (dir.join("a.bas"), dir.join("b.bas"), dir.join("c.bas"));
        std::fs::write(&a, MESSY).unwrap();
        std::fs::write(&b, MESSY).unwrap();
        std::fs::write(&c, tidy()).unwrap();
        let missing = dir.join("missing.bas");
        let (a, b, c, missing) = (a.into_os_string(), b.into_os_string(), c.into_os_string(), missing.into_os_string());

        let (code, out, err) = qb_fmt(&[&os("--check"), &a, &missing, &b, &c], "");
        assert_eq!(code, 2, "{err}");
        let listed: Vec<&str> = out.lines().collect();
        assert_eq!(listed, vec![Path::new(&a).display().to_string(), Path::new(&b).display().to_string()]);
        assert!(err.contains("missing.bas"), "{err}");

        let (code, _, _) = qb_fmt(&[&os("--check"), &a, &b, &c], "");
        assert_eq!(code, 1);
        let (code, out, _) = qb_fmt(&[&os("--check"), &c], "");
        assert_eq!((code, out.as_str()), (0, ""));

        // --write goes on past the missing file too
        let (code, _, err) = qb_fmt(&[&os("-w"), &missing, &a], "");
        assert_eq!(code, 2, "{err}");
        assert_eq!(std::fs::read(&a).unwrap(), tidy());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn check_with_write_is_an_error() {
        let dir = work_dir("checkwrite");
        let a = dir.join("a.bas");
        std::fs::write(&a, MESSY).unwrap();
        let (code, _, err) = qb_fmt(&[&os("--check"), &os("--write"), &a.clone().into_os_string()], "");
        assert_eq!(code, 2);
        assert!(err.contains("--check and --write"), "{err}");
        assert_eq!(std::fs::read_to_string(&a).unwrap(), MESSY, "the file was changed");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dash_is_standard_input() {
        let (code, out, _) = qb_fmt(&[&os("-")], MESSY);
        assert_eq!((code, out.as_bytes()), (0, &tidy()[..]));
        let (code, out, _) = qb_fmt(&[], MESSY);
        assert_eq!((code, out.as_bytes()), (0, &tidy()[..]));
        // --check: an explicit `-` is listed, the implicit one is not
        assert_eq!(qb_fmt(&[&os("--check"), &os("-")], MESSY), (1, "-\n".to_string(), String::new()));
        assert_eq!(qb_fmt(&[&os("--check")], MESSY).0, 1);
        assert_eq!(qb_fmt(&[&os("--check")], MESSY).1, "");
        // --write: standard input has no file, so the text goes to standard output
        let (code, out, _) = qb_fmt(&[&os("-w"), &os("-")], MESSY);
        assert_eq!((code, out.as_bytes()), (0, &tidy()[..]));
    }

    /// The rewrite leaves no temporary file behind, keeps the permissions,
    /// and leaves a file that needs no change untouched.
    #[test]
    fn write_replaces_the_file_whole() {
        let dir = work_dir("atomic");
        let a = dir.join("a.bas");
        std::fs::write(&a, MESSY).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&a, std::fs::Permissions::from_mode(0o640)).unwrap();
        }
        let (code, _, err) = qb_fmt(&[&os("--write"), &a.clone().into_os_string()], "");
        assert_eq!(code, 0, "{err}");
        assert_eq!(std::fs::read(&a).unwrap(), tidy());
        let names: Vec<String> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(names, vec!["a.bas".to_string()]);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&a).unwrap().permissions().mode() & 0o777, 0o640);
        }
        // a read-only file is refused, not replaced
        let mut ro = std::fs::metadata(&a).unwrap().permissions();
        std::fs::write(&a, MESSY).unwrap();
        ro.set_readonly(true);
        std::fs::set_permissions(&a, ro.clone()).unwrap();
        let (code, _, err) = qb_fmt(&[&os("-w"), &a.clone().into_os_string()], "");
        // (an administrator on Unix may write a read-only file anyway)
        if code == 2 {
            assert!(err.contains("a.bas"), "{err}");
            assert_eq!(std::fs::read_to_string(&a).unwrap(), MESSY);
        }
        #[allow(clippy::permissions_set_readonly_false)]
        ro.set_readonly(false);
        std::fs::set_permissions(&a, ro).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    struct Broken;

    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
    }

    #[test]
    fn failed_standard_output_is_exit_2() {
        let mut err = Vec::new();
        assert_eq!(run(vec![os("-")], &mut MESSY.as_bytes(), &mut Broken, &mut err), 2);
        assert!(String::from_utf8(err).unwrap().contains("standard output"));
        assert_eq!(run(vec![os("--help")], &mut "".as_bytes(), &mut Broken, &mut Vec::<u8>::new()), 2);
    }
}
