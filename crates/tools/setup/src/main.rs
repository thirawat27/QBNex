//! `qbnex-setup` — the QBNex installer.
//!
//! A release installer is this program with the release bundle (a zip of the folder
//! `.github/scripts/portable_bundle.*` stages) appended, followed by a 16-byte trailer:
//! the zip's length (u64, little endian) and the magic `QBNXSETP`. The same trick the
//! runner `qb-exec` uses for a program's QIR.
//!
//! ```text
//! qbnex-setup                      the setup wizard (a window; in a terminal without
//!                                  a display it asks there instead)
//! qbnex-setup --yes [--dir D]      install without questions
//! qbnex-setup --uninstall          remove an installation (run from the install folder)
//! qbnex-setup --pack DIR --out F   make an installer F that carries DIR
//! ```
//!
//! Per-user install, no administrator rights: Windows `%LOCALAPPDATA%\Programs\QBNex`
//! (user PATH, Start menu, "Apps & features" entry); Linux/macOS `~/.local/share/qbnex`
//! with `qb` and `qbnex` links in `~/.local/bin` and an entry in the applications menu.

mod engine;
#[cfg(feature = "gui")]
mod gui;
mod payload;
mod platform;
#[cfg(any(feature = "gui", test))]
mod text;
#[cfg(any(feature = "gui", test))]
mod wizard;

use std::io::{self, BufRead, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use engine::{Bundle, Event, Options, Progress, VERSION};

#[derive(Default)]
struct Args {
    yes: bool,
    uninstall: bool,
    no_path: bool,
    no_menu: bool,
    desktop: bool,
    associate: bool,
    /// Ask in the terminal even where a window could be shown.
    console: bool,
    /// Show the window even when not started from a terminal or a file manager.
    gui: bool,
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
     qbnex-setup --no-menu            do not add QBNex to the applications menu\n\
     qbnex-setup --desktop            create a desktop shortcut\n\
     qbnex-setup --associate          add \"Compile with QBNex\" to the menu of .bas files\n\
     qbnex-setup --console | --gui    ask in the terminal | in the setup window\n\
     qbnex-setup --uninstall [--yes]  remove QBNex (run the copy in the install folder)\n\
     qbnex-setup --pack <dir> --out <file>   build an installer that carries <dir>\n\
     qbnex-setup --version | --help\n"
}

fn parse(mut it: impl Iterator<Item = String>) -> Result<Option<Args>, String> {
    let mut a = Args::default();
    while let Some(s) = it.next() {
        let mut value = |name: &str| it.next().map(PathBuf::from).ok_or(format!("{name} needs a value"));
        match s.as_str() {
            // the switches other Windows installers use for a silent install
            "-y" | "--yes" | "/S" | "/s" | "/SILENT" | "/VERYSILENT" => a.yes = true,
            "--uninstall" => a.uninstall = true,
            "--no-path" => a.no_path = true,
            "--no-menu" => a.no_menu = true,
            "--desktop" => a.desktop = true,
            "--associate" => a.associate = true,
            "--console" => a.console = true,
            "--gui" => a.gui = true,
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
            other => match other.strip_prefix("/DIR=") {
                Some(dir) if !dir.is_empty() => a.dir = Some(PathBuf::from(dir.trim_matches('"'))),
                _ => return Err(format!("unknown argument: {other}\n\n{}", usage())),
            },
        }
    }
    Ok(Some(a))
}

impl Args {
    /// What to install where, from the switches.
    fn options(&self) -> Result<Options, String> {
        let dir = match &self.dir {
            Some(d) => d.clone(),
            None => platform::default_dir()?,
        };
        // PATH entries and links must not depend on the folder setup was started from
        let dir = std::path::absolute(&dir).map_err(|e| format!("bad install folder {}: {e}", dir.display()))?;
        Ok(Options { dir, add_path: !self.no_path, menu: !self.no_menu, desktop: self.desktop, associate: self.associate && platform::can_associate() })
    }

    /// True when the questions are asked in the setup window.
    fn wants_window(&self) -> bool {
        if !cfg!(feature = "gui") || self.yes || self.console || self.pack.is_some() || !platform::has_display() {
            return false;
        }
        let terminal = io::stdin().is_terminal();
        // Started from a file manager there is no terminal at all. A script that
        // pipes or redirects the installer gets no window: it could not answer it.
        let from_file_manager = cfg!(unix) && !terminal && !io::stdout().is_terminal() && !io::stderr().is_terminal();
        self.gui || terminal || from_file_manager
    }
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
    if let Some(src) = &args.pack {
        // a build step: it never asks and never waits for Enter
        let result = match &args.out {
            Some(out) => payload::pack(src, out).map(|n| println!("wrote {} ({n} files)", out.display())),
            None => Err("--pack needs --out <file>".into()),
        };
        return report(result);
    }
    #[cfg(feature = "gui")]
    if args.wants_window() {
        match window(&args) {
            Ok(result) => return report(result),
            // no window after all (no display server answers): the terminal it is
            Err(e) if io::stdin().is_terminal() => eprintln!("qbnex-setup: no setup window ({e}); asking here"),
            Err(e) => return report(Err(format!("cannot open the setup window: {e}"))),
        }
    }
    let interactive = !args.yes && io::stdin().is_terminal();
    let code = report(if args.uninstall { uninstall(&args, interactive) } else { install(&args, interactive) });
    // Started by double-clicking on Windows: keep the console open to show the result.
    if interactive && cfg!(windows) {
        pause();
    }
    code
}

fn report(result: Result<(), String>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("qbnex-setup: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Runs the wizard; the outer error means no window could be opened.
#[cfg(feature = "gui")]
fn window(args: &Args) -> Result<Result<(), String>, String> {
    let (mode, options, bundle) = if args.uninstall {
        let dir = match engine::find_install(args.dir.as_deref()) {
            Ok(dir) => dir,
            Err(e) => return Ok(Err(e)),
        };
        (wizard::Mode::Uninstall, Options::new(dir), None)
    } else {
        let options = match args.options() {
            Ok(o) => o,
            Err(e) => return Ok(Err(e)),
        };
        // without a bundle the window still opens, to say so where it can be read
        (wizard::Mode::Install, options, Bundle::open().ok())
    };
    // a watchdog for the wizard that drives itself: a test must end
    if std::env::var_os(gui::SELF_DRIVE).is_some() {
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_secs(180));
            eprintln!("qbnex-setup: the self-driven wizard did not finish");
            std::process::exit(3);
        });
    }
    platform::leave_own_console();
    gui::run(gui::Start { mode, options, bundle })
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

/// The terminal shows results, not progress.
struct Quiet;

impl Progress for Quiet {
    fn event(&mut self, _: Event) {}
}

fn install(args: &Args, interactive: bool) -> Result<(), String> {
    let bundle = Bundle::open()?;
    let options = args.options()?;
    let dir = &options.dir;
    println!("QBNex {VERSION} setup");
    println!("Install folder: {}", dir.display());
    if interactive && !ask("Install QBNex here?") {
        return Err("cancelled".into());
    }
    engine::install(&options, &bundle, &mut Quiet)?;
    println!("QBNex {VERSION} is installed in {}.", dir.display());
    if args.no_path {
        println!("PATH was not changed; run {} directly.", dir.join(platform::compiler_name()).display());
    } else {
        println!("{}", platform::path_hint(dir));
    }
    Ok(())
}

fn uninstall(args: &Args, interactive: bool) -> Result<(), String> {
    let dir = engine::find_install(args.dir.as_deref())?;
    if interactive && !ask(&format!("Remove QBNex from {}?", dir.display())) {
        return Err("cancelled".into());
    }
    engine::uninstall(&dir, &mut Quiet)?;
    platform::remove_install_dir(&dir);
    println!("QBNex was removed from {}.", dir.display());
    Ok(())
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
        assert!(a.yes && a.no_path && !a.uninstall && !a.no_menu && !a.desktop && !a.associate);
        assert_eq!(a.dir, Some(PathBuf::from("x")));
        assert!(args(&["--dir"]).is_err());
        assert!(args(&["--bogus"]).is_err());
        assert!(args(&["--version"]).unwrap().is_none());

        let a = args(&["--no-menu", "--desktop", "--associate", "--console"]).unwrap().unwrap();
        assert!(a.no_menu && a.desktop && a.associate && a.console && !a.yes);
    }

    #[test]
    fn the_switches_of_other_installers_mean_a_silent_install() {
        for switch in ["/S", "/SILENT", "/VERYSILENT"] {
            assert!(args(&[switch]).unwrap().unwrap().yes, "{switch}");
        }
        let a = args(&["/VERYSILENT", r#"/DIR="C:\My QBNex""#]).unwrap().unwrap();
        assert_eq!(a.dir, Some(PathBuf::from(r"C:\My QBNex")));
        assert!(args(&["/DIR="]).is_err());
    }

    #[test]
    fn the_switches_choose_what_is_installed_and_never_a_window_when_silent() {
        let a = args(&["--yes", "--dir", "x", "--no-path", "--no-menu", "--desktop"]).unwrap().unwrap();
        let o = a.options().unwrap();
        assert!(o.dir.is_absolute() && o.dir.ends_with("x"));
        assert!(!o.add_path && !o.menu && o.desktop && !o.associate);
        assert!(!a.wants_window());
        let defaults = args(&["--dir", "x"]).unwrap().unwrap().options().unwrap();
        assert!(defaults.add_path && defaults.menu && !defaults.desktop && !defaults.associate);
        for silent in [&["--console"][..], &["--gui", "--yes"], &["--pack", "d", "--out", "f", "--gui"]] {
            assert!(!args(silent).unwrap().unwrap().wants_window(), "{silent:?}");
        }
    }
}
