//! `qbnex-setup` — the QBNex installer.
//!
//! A release installer is this program with the release bundle (a zip of the folder
//! `.github/scripts/portable_bundle.*` stages) appended, followed by a 16-byte trailer:
//! the zip's length (u64, little endian) and the magic `QBNXSETP`. The same trick the
//! runner `qb-exec` uses for a program's QIR.
//!
//! ```text
//! qbnex-setup                      install (asks first when run from a terminal)
//! qbnex-setup --yes [--dir D]      install without questions
//! qbnex-setup --uninstall          remove an installation (run from the install folder)
//! qbnex-setup --pack DIR --out F   make an installer F that carries DIR
//! ```
//!
//! Per-user install, no administrator rights: Windows `%LOCALAPPDATA%\Programs\QBNex`
//! (user PATH, "Apps & features" entry); Linux/macOS `~/.local/share/qbnex` with `qb`
//! and `qbnex` links in `~/.local/bin`.

mod payload;
mod platform;

use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Lists every file the installer wrote, relative to the install folder, so an upgrade
/// or uninstall removes exactly those (and leaves files the user added).
const MANIFEST: &str = ".qbnex-install";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Default)]
struct Args {
    yes: bool,
    uninstall: bool,
    no_path: bool,
    dir: Option<PathBuf>,
    pack: Option<PathBuf>,
    out: Option<PathBuf>,
}

fn usage() -> &'static str {
    "QBNex setup\n\
     \n\
     qbnex-setup                      install QBNex for the current user\n\
     qbnex-setup --yes [--dir <dir>]  install without asking\n\
     qbnex-setup --no-path            do not add QBNex to PATH\n\
     qbnex-setup --uninstall [--yes]  remove QBNex (run the copy in the install folder)\n\
     qbnex-setup --pack <dir> --out <file>   build an installer that carries <dir>\n\
     qbnex-setup --version | --help\n"
}

fn parse(mut it: impl Iterator<Item = String>) -> Result<Option<Args>, String> {
    let mut a = Args::default();
    while let Some(s) = it.next() {
        let mut value = |name: &str| it.next().map(PathBuf::from).ok_or(format!("{name} needs a value"));
        match s.as_str() {
            "-y" | "--yes" | "/S" | "/s" => a.yes = true,
            "--uninstall" => a.uninstall = true,
            "--no-path" => a.no_path = true,
            "--dir" => a.dir = Some(value("--dir")?),
            "--pack" => a.pack = Some(value("--pack")?),
            "--out" => a.out = Some(value("--out")?),
            "-h" | "--help" => {
                print!("{}", usage());
                return Ok(None);
            }
            "-v" | "--version" => {
                println!("qbnex-setup {VERSION}");
                return Ok(None);
            }
            other => return Err(format!("unknown argument: {other}\n\n{}", usage())),
        }
    }
    Ok(Some(a))
}

fn main() -> ExitCode {
    let args = match parse(std::env::args().skip(1)) {
        Ok(Some(a)) => a,
        Ok(None) => return ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let interactive = !args.yes && io::stdin().is_terminal();
    let result = if let Some(src) = &args.pack {
        match &args.out {
            Some(out) => payload::pack(src, out).map(|n| println!("wrote {} ({n} files)", out.display())),
            None => Err("--pack needs --out <file>".into()),
        }
    } else if args.uninstall {
        uninstall(&args, interactive)
    } else {
        install(&args, interactive)
    };
    let code = match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("qbnex-setup: {e}");
            ExitCode::FAILURE
        }
    };
    // Started by double-clicking on Windows: keep the console open to show the result.
    if interactive && cfg!(windows) {
        pause();
    }
    code
}

fn ask(question: &str) -> bool {
    print!("{question} [Y/n] ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    if io::stdin().lock().read_line(&mut line).is_err() {
        return false;
    }
    let t = line.trim();
    t.is_empty() || t.eq_ignore_ascii_case("y") || t.eq_ignore_ascii_case("yes")
}

fn pause() {
    print!("Press Enter to close.");
    let _ = io::stdout().flush();
    let mut line = String::new();
    let _ = io::stdin().lock().read_line(&mut line);
}

fn install(args: &Args, interactive: bool) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("cannot find this program: {e}"))?;
    let image = std::fs::read(&exe).map_err(|e| format!("cannot read {}: {e}", exe.display()))?;
    let Some((stub, zip)) = payload::split(&image) else {
        return Err("this copy of qbnex-setup carries no QBNex files (it is the uninstaller or a plain build); download the release installer".into());
    };
    let dir = match &args.dir {
        Some(d) => d.clone(),
        None => platform::default_dir()?,
    };
    // PATH entries and links must not depend on the folder setup was started from
    let dir = std::path::absolute(&dir).map_err(|e| format!("bad install folder {}: {e}", dir.display()))?;
    println!("QBNex {VERSION} setup");
    println!("Install folder: {}", dir.display());
    if interactive && !ask("Install QBNex here?") {
        return Err("cancelled".into());
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    remove_listed(&dir)?;
    let files = payload::extract(zip, &dir)?;
    // The uninstaller is this program without the bundle.
    let me = platform::setup_name();
    write_file(&dir.join(me), stub, true)?;
    let mut manifest = files;
    manifest.push(me.to_string());
    manifest.sort();
    let text: String = manifest.iter().map(|f| format!("{f}\n")).collect();
    write_file(&dir.join(MANIFEST), text.as_bytes(), false)?;

    let size_kb = dir_size(&dir) / 1024;
    platform::register(&dir, VERSION, size_kb, !args.no_path)?;
    println!("QBNex {VERSION} is installed in {}.", dir.display());
    if args.no_path {
        println!("PATH was not changed; run {} directly.", dir.join(platform::compiler_name()).display());
    } else {
        println!("{}", platform::path_hint(&dir));
    }
    Ok(())
}

fn uninstall(args: &Args, interactive: bool) -> Result<(), String> {
    let dir = match &args.dir {
        Some(d) => d.clone(),
        None => {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let here = exe.parent().map(Path::to_path_buf).unwrap_or_default();
            if here.join(MANIFEST).is_file() { here } else { platform::default_dir()? }
        }
    };
    if !dir.join(MANIFEST).is_file() {
        return Err(format!("no QBNex installation found in {}", dir.display()));
    }
    if interactive && !ask(&format!("Remove QBNex from {}?", dir.display())) {
        return Err("cancelled".into());
    }
    platform::unregister(&dir)?;
    remove_listed(&dir)?;
    // the compiler's work folder (build files, settings)
    let work = dir.join("target");
    if work.is_dir() && !is_link(&work) {
        let _ = std::fs::remove_dir_all(&work);
    }
    platform::remove_install_dir(&dir);
    println!("QBNex was removed from {}.", dir.display());
    Ok(())
}

/// Removes the files an earlier install listed in its manifest, then empty folders.
fn remove_listed(dir: &Path) -> Result<(), String> {
    let Ok(text) = std::fs::read_to_string(dir.join(MANIFEST)) else {
        return Ok(());
    };
    let me = std::env::current_exe().ok().and_then(|p| p.canonicalize().ok());
    let mut parents = Vec::new();
    for rel in text.lines().filter(|l| !l.is_empty()) {
        let Some(path) = payload::safe_join(dir, rel) else {
            continue;
        };
        // a running uninstaller cannot delete itself; platform::remove_install_dir does
        if me.is_some() && path.canonicalize().ok() == me {
            continue;
        }
        if path.is_file() || is_link(&path) {
            std::fs::remove_file(&path).map_err(|e| format!("cannot remove {}: {e}", path.display()))?;
        }
        let mut p = path.parent();
        while let Some(d) = p {
            if d == dir {
                break;
            }
            parents.push(d.to_path_buf());
            p = d.parent();
        }
    }
    parents.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    parents.dedup();
    for d in parents {
        let _ = std::fs::remove_dir(&d); // only succeeds when empty
    }
    let _ = std::fs::remove_file(dir.join(MANIFEST));
    Ok(())
}

fn is_link(p: &Path) -> bool {
    std::fs::symlink_metadata(p).map(|m| m.file_type().is_symlink()).unwrap_or(false)
}

fn write_file(path: &Path, bytes: &[u8], executable: bool) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    #[cfg(unix)]
    if executable {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }
    #[cfg(not(unix))]
    let _ = executable;
    Ok(())
}

fn dir_size(dir: &Path) -> u64 {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return 0;
    };
    rd.flatten()
        .map(|e| match e.file_type() {
            Ok(t) if t.is_dir() => dir_size(&e.path()),
            Ok(t) if t.is_file() => e.metadata().map(|m| m.len()).unwrap_or(0),
            _ => 0,
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Result<Option<Args>, String> {
        parse(v.iter().map(|s| s.to_string()))
    }

    #[test]
    fn parses_flags() {
        let a = args(&["--yes", "--dir", "x", "--no-path"]).unwrap().unwrap();
        assert!(a.yes && a.no_path && !a.uninstall);
        assert_eq!(a.dir, Some(PathBuf::from("x")));
        assert!(args(&["--dir"]).is_err());
        assert!(args(&["--bogus"]).is_err());
        assert!(args(&["--version"]).unwrap().is_none());
    }

    #[test]
    fn upgrade_removes_only_listed_files() {
        let dir = std::env::temp_dir().join(format!("qbnex-setup-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("compiler/stdlib")).unwrap();
        std::fs::write(dir.join("compiler/stdlib/a.bas"), "x").unwrap();
        std::fs::write(dir.join("mine.bas"), "user file").unwrap();
        std::fs::write(dir.join(MANIFEST), "compiler/stdlib/a.bas\n../outside.txt\n").unwrap();
        remove_listed(&dir).unwrap();
        assert!(!dir.join("compiler").exists(), "listed file and its empty folders go");
        assert!(dir.join("mine.bas").is_file(), "files the user added stay");
        assert!(!dir.join(MANIFEST).exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
