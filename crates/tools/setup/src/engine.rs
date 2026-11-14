//! Installing and removing QBNex, the same for the console and the window.
//!
//! An install never leaves a half-written folder behind. The files of the version
//! being replaced are moved into [`BACKUP`] first; when anything fails, or the user
//! cancels, the new files are deleted and the old ones moved back.

use std::mem::DropGuard;
use std::path::{Path, PathBuf};

use crate::{payload, platform};

/// Lists every file the installer wrote, relative to the install folder, so an upgrade
/// or uninstall removes exactly those (and leaves files the user added). Lines that
/// start with `#` are notes; the first one names the installed version.
pub const MANIFEST: &str = ".qbnex-install";
/// Holds the files of the previous version while a new one is written.
const BACKUP: &str = ".qbnex-backup";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub use payload::CANCELLED;

/// What the user chose.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    pub dir: PathBuf,
    /// Make `qb` reachable from a terminal (PATH on Windows, links in `~/.local/bin`).
    pub add_path: bool,
    /// Entries in the applications menu.
    pub menu: bool,
    /// A shortcut on the desktop.
    pub desktop: bool,
    /// Offer QBNex for `.bas` files in the file manager.
    pub associate: bool,
}

impl Options {
    pub fn new(dir: PathBuf) -> Options {
        Options { dir, add_path: true, menu: true, desktop: false, associate: false }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Step {
    /// Setting the previous version aside.
    Prepare,
    Extract,
    Register,
    Remove,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Step(Step),
    /// A file is about to be written; `done` of `total` bytes are extracted.
    File { name: String, done: u64, total: u64 },
}

/// Receives the progress of an install or uninstall and can stop an install.
pub trait Progress {
    fn event(&mut self, event: Event);
    fn cancelled(&self) -> bool {
        false
    }
}

/// The installer image: the program and the bundle it carries.
pub struct Bundle {
    image: Vec<u8>,
    /// Where the zip starts and ends in `image`.
    zip: std::ops::Range<usize>,
    size: u64,
}

impl Bundle {
    /// The bundle appended to this program.
    pub fn open() -> Result<Bundle, String> {
        let exe = std::env::current_exe().map_err(|e| format!("cannot find this program: {e}"))?;
        let image = std::fs::read(&exe).map_err(|e| format!("cannot read {}: {e}", exe.display()))?;
        let Some((stub, zip)) = payload::split(&image) else {
            return Err("this copy of qbnex-setup carries no QBNex files (it is the uninstaller or a plain build); download the release installer".into());
        };
        let range = stub.len()..stub.len() + zip.len();
        Bundle::from_image(image, range)
    }

    fn from_image(image: Vec<u8>, zip: std::ops::Range<usize>) -> Result<Bundle, String> {
        let size = payload::unpacked_size(&image[zip.clone()])?;
        Ok(Bundle { image, zip, size })
    }

    #[cfg(test)]
    pub fn from_parts(stub: &[u8], zip: &[u8]) -> Bundle {
        let mut image = stub.to_vec();
        image.extend_from_slice(zip);
        Bundle::from_image(image, stub.len()..stub.len() + zip.len()).unwrap()
    }

    /// This program without the bundle: the uninstaller.
    fn stub(&self) -> &[u8] {
        &self.image[..self.zip.start]
    }

    fn zip(&self) -> &[u8] {
        &self.image[self.zip.clone()]
    }

    /// Bytes the installed files take, the uninstaller included.
    pub fn installed_size(&self) -> u64 {
        self.size + self.zip.start as u64
    }
}

/// The files an install listed in its manifest.
fn listed(dir: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(dir.join(MANIFEST)).unwrap_or_default();
    text.lines().filter(|l| !l.is_empty() && !l.starts_with('#')).map(str::to_string).collect()
}

/// The version installed in `dir`: `Some("")` for an install that did not record it.
pub fn installed_version(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join(MANIFEST)).ok()?;
    let first = text.lines().next().unwrap_or("");
    Some(first.strip_prefix("# QBNex ").unwrap_or("").trim().to_string())
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

/// Removes the folders `files` were in, deepest first, where they are now empty.
fn prune_empty_dirs(dir: &Path, files: &[String]) {
    let mut parents = Vec::new();
    for rel in files {
        let Some(path) = payload::safe_join(dir, rel) else {
            continue;
        };
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
}

/// The files moved aside and written by one install, so it can be undone.
struct Transaction<'a> {
    dir: &'a Path,
    moved: Vec<String>,
    written: Vec<String>,
}

impl<'a> Transaction<'a> {
    /// Starts an install in `dir`. The transaction rolls back when it is dropped, so
    /// every way out of the install (an error, a cancel) undoes it, unless it is taken
    /// out of the guard and committed.
    fn begin(dir: &'a Path) -> DropGuard<Transaction<'a>, impl FnOnce(Transaction<'a>)> {
        DropGuard::new(Transaction { dir, moved: Vec::new(), written: Vec::new() }, Transaction::roll_back)
    }

    fn backup(&self) -> PathBuf {
        self.dir.join(BACKUP)
    }

    /// Moves a file of the previous version into the backup folder.
    fn set_aside(&mut self, rel: &str) -> Result<(), String> {
        let Some(from) = payload::safe_join(self.dir, rel) else {
            return Ok(());
        };
        if !from.is_file() && !is_link(&from) {
            return Ok(());
        }
        let to = self.backup().join(rel);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
        }
        std::fs::rename(&from, &to)
            .map_err(|e| format!("{} is in use or cannot be replaced; close running QBNex programs and try again ({e})", from.display()))?;
        self.moved.push(rel.to_string());
        Ok(())
    }

    /// The install worked: the previous version goes.
    fn commit(self) {
        let _ = std::fs::remove_dir_all(self.backup());
        prune_empty_dirs(self.dir, &self.moved);
    }

    /// The install failed: the new files go and the previous version comes back.
    fn roll_back(self) {
        for rel in &self.written {
            if let Some(path) = payload::safe_join(self.dir, rel) {
                let _ = std::fs::remove_file(path);
            }
        }
        prune_empty_dirs(self.dir, &self.written);
        restore(self.dir, &self.moved);
        let _ = std::fs::remove_dir(self.dir); // a folder this install created, now empty
    }
}

/// Moves `files` from the backup folder back into `dir` and removes the folder.
fn restore(dir: &Path, files: &[String]) {
    let backup = dir.join(BACKUP);
    for rel in files {
        let (Some(to), from) = (payload::safe_join(dir, rel), backup.join(rel)) else {
            continue;
        };
        if let Some(parent) = to.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::rename(from, to);
    }
    let _ = std::fs::remove_dir_all(backup);
}

/// Every file under `dir`, relative and `/`-separated.
fn files_under(dir: &Path, prefix: &str, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let name = format!("{prefix}{}", e.file_name().to_string_lossy());
        match e.file_type() {
            Ok(t) if t.is_dir() => files_under(&e.path(), &format!("{name}/"), out),
            Ok(_) => out.push(name),
            Err(_) => {}
        }
    }
}

/// An install that was killed half way left its backup folder: the previous version
/// comes back before anything else is done. A file the dead install already wrote in
/// its place is replaced by the old one, so the folder matches the old manifest again.
fn recover_interrupted(dir: &Path) {
    let backup = dir.join(BACKUP);
    if !backup.is_dir() {
        return;
    }
    let mut files = Vec::new();
    files_under(&backup, "", &mut files);
    for rel in &files {
        if let Some(to) = payload::safe_join(dir, rel) {
            let _ = std::fs::remove_file(to);
        }
    }
    restore(dir, &files);
}

/// Installs the bundle as `o` says and makes it reachable.
pub fn install(o: &Options, bundle: &Bundle, progress: &mut dyn Progress) -> Result<(), String> {
    let upgrade = o.dir.join(MANIFEST).is_file();
    let result = install_with(o, bundle, progress, &mut |o, size_kb| platform::register(o, VERSION, size_kb));
    if result.is_err() && !upgrade {
        // registering may have stopped half way
        let _ = platform::unregister(&o.dir);
    }
    result
}

/// [`install`] with the step that touches the user's settings passed in.
pub fn install_with(
    o: &Options, bundle: &Bundle, progress: &mut dyn Progress, register: &mut dyn FnMut(&Options, u64) -> Result<(), String>,
) -> Result<(), String> {
    let dir = &o.dir;
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    recover_interrupted(dir);
    let mut tx = Transaction::begin(dir);
    write_files(o, bundle, progress, &mut tx)?;
    progress.event(Event::Step(Step::Register));
    register(o, bundle.installed_size() / 1024)?;
    DropGuard::dismiss(tx).commit();
    Ok(())
}

fn write_files(o: &Options, bundle: &Bundle, progress: &mut dyn Progress, tx: &mut Transaction<'_>) -> Result<(), String> {
    let dir = &o.dir;
    progress.event(Event::Step(Step::Prepare));
    let mut old = listed(dir);
    old.push(MANIFEST.to_string());
    for rel in &old {
        if progress.cancelled() {
            return Err(CANCELLED.into());
        }
        tx.set_aside(rel)?;
    }

    // Installing over what is already there: a file this install replaces that no
    // manifest lists (a first install into a folder with files, a copy unpacked by
    // hand, an install killed before it wrote its manifest) is set aside like a file
    // of the previous version. It comes back when the install fails or is cancelled,
    // and is replaced for good only when the install is committed.
    let mut mine = payload::names(bundle.zip())?;
    mine.push(platform::setup_name().to_string());
    mine.extend(platform::extra_files().iter().map(|(name, _)| name.to_string()));
    for rel in &mine {
        if progress.cancelled() {
            return Err(CANCELLED.into());
        }
        if !tx.moved.contains(rel) {
            tx.set_aside(rel)?;
        }
    }

    progress.event(Event::Step(Step::Extract));
    let total = bundle.installed_size();
    payload::extract(bundle.zip(), dir, &mut tx.written, &mut |name, done| {
        progress.event(Event::File { name: name.to_string(), done, total });
        !progress.cancelled()
    })?;

    // The uninstaller is this program without the bundle.
    let me = platform::setup_name();
    progress.event(Event::File { name: me.to_string(), done: total - bundle.stub().len() as u64, total });
    tx.written.push(me.to_string());
    write_file(&dir.join(me), bundle.stub(), true)?;
    for (name, bytes) in platform::extra_files() {
        tx.written.push(name.to_string());
        write_file(&dir.join(name), bytes, false)?;
    }

    let mut manifest = tx.written.clone();
    manifest.sort();
    manifest.dedup();
    let text: String = std::iter::once(format!("# QBNex {VERSION}\n")).chain(manifest.iter().map(|f| format!("{f}\n"))).collect();
    tx.written.push(MANIFEST.to_string());
    write_file(&dir.join(MANIFEST), text.as_bytes(), false)
}

/// The install folder an uninstall is about: `dir`, else the folder this program is
/// in when that is an install, else the default folder.
pub fn find_install(dir: Option<&Path>) -> Result<PathBuf, String> {
    let dir = match dir {
        Some(d) => d.to_path_buf(),
        None => {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let here = exe.parent().map(Path::to_path_buf).unwrap_or_default();
            if here.join(MANIFEST).is_file() { here } else { platform::default_dir()? }
        }
    };
    if dir.join(MANIFEST).is_file() { Ok(dir) } else { Err(format!("no QBNex installation found in {}", dir.display())) }
}

/// Removes the install in `dir` and what made it reachable. The folder itself, and
/// the uninstaller in it, go with [`platform::remove_install_dir`] when the caller
/// is about to exit: on Windows a running program cannot delete its own file.
pub fn uninstall(dir: &Path, progress: &mut dyn Progress) -> Result<(), String> {
    progress.event(Event::Step(Step::Remove));
    recover_interrupted(dir);
    platform::unregister(dir)?;
    remove_listed(dir)?;
    remove_work_files(dir);
    Ok(())
}

/// Removes the files an install listed in its manifest, then empty folders.
fn remove_listed(dir: &Path) -> Result<(), String> {
    let files = listed(dir);
    let me = std::env::current_exe().ok().and_then(|p| p.canonicalize().ok());
    for rel in &files {
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
    }
    prune_empty_dirs(dir, &files);
    let _ = std::fs::remove_file(dir.join(MANIFEST));
    Ok(())
}

/// Settings and version stamp the compiler keeps in its work folder `target`.
const WORK_FILES: [&str; 2] = ["qb-config.ini", "qb-version.txt"];
/// Prefix of the compiler's build folders in `target` (`qb-temp`, `qb-temp2`, ...).
const WORK_DIR_PREFIX: &str = "qb-temp";

/// Removes what the compiler wrote into its work folder `target`, then the folder
/// if that left it empty. Anything else in it is not QBNex's (the install folder can
/// be one the user also keeps a project in) and stays.
fn remove_work_files(dir: &Path) {
    let work = dir.join("target");
    if !work.is_dir() || is_link(&work) {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(&work) {
        for e in entries.flatten() {
            let path = e.path();
            let name = e.file_name();
            let name = name.to_string_lossy();
            if is_link(&path) {
                continue;
            }
            if path.is_dir() && name.starts_with(WORK_DIR_PREFIX) {
                let _ = std::fs::remove_dir_all(&path);
            } else if path.is_file() && WORK_FILES.contains(&name.as_ref()) {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
    let _ = std::fs::remove_dir(&work); // only succeeds when empty
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::*;

    fn bundle(files: &[(&str, &str)]) -> Bundle {
        let mut w = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, text) in files {
            w.start_file(*name, SimpleFileOptions::default()).unwrap();
            w.write_all(text.as_bytes()).unwrap();
        }
        Bundle::from_parts(b"STUB", &w.finish().unwrap().into_inner())
    }

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("qbnex-setup-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// Records the events and cancels when the file `stop_at` is reached.
    #[derive(Default)]
    struct Recorder {
        events: Vec<Event>,
        stop_at: Option<&'static str>,
        stop: bool,
    }

    impl Progress for Recorder {
        fn event(&mut self, event: Event) {
            if let (Event::File { name, .. }, Some(stop)) = (&event, self.stop_at)
                && name == stop
            {
                self.stop = true;
            }
            self.events.push(event);
        }

        fn cancelled(&self) -> bool {
            self.stop
        }
    }

    fn tree(dir: &Path) -> Vec<(String, String)> {
        let mut names = Vec::new();
        files_under(dir, "", &mut names);
        names.sort();
        // the menu icon some systems install is not text
        names.into_iter().map(|n| (n.clone(), String::from_utf8_lossy(&std::fs::read(dir.join(&n)).unwrap()).into_owned())).collect()
    }

    #[test]
    fn install_upgrade_and_uninstall() {
        let dir = temp("cycle");
        let o = Options::new(dir.clone());
        let mut sizes = Vec::new();
        let v1 = bundle(&[("qb", "one"), ("compiler/stdlib/old.bas", "old"), ("compiler/stdlib/keep.bas", "k1")]);
        let mut rec = Recorder::default();
        install_with(&o, &v1, &mut rec, &mut |_, kb| {
            sizes.push(kb);
            Ok(())
        })
        .unwrap();
        assert_eq!(rec.events[0], Event::Step(Step::Prepare));
        assert_eq!(rec.events.last(), Some(&Event::Step(Step::Register)));
        // the bytes reported before the last file never pass the total
        assert!(rec.events.iter().all(|e| !matches!(e, Event::File { done, total, .. } if done > total)));
        assert_eq!(installed_version(&dir).as_deref(), Some(VERSION));
        std::fs::write(dir.join("mine.bas"), "user file").unwrap();

        let v2 = bundle(&[("qb", "two"), ("compiler/stdlib/keep.bas", "k2"), ("compiler/support/new.bas", "new")]);
        install_with(&o, &v2, &mut Recorder::default(), &mut |_, _| Ok(())).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("qb")).unwrap(), "two");
        assert!(!dir.join("compiler/stdlib/old.bas").exists(), "a file the new version dropped goes");
        assert!(dir.join("compiler/support/new.bas").is_file());
        assert!(dir.join("mine.bas").is_file(), "files the user added stay");
        assert!(!dir.join(BACKUP).exists());
        assert_eq!(std::fs::read(dir.join(platform::setup_name())).unwrap(), b"STUB");

        remove_listed(&dir).unwrap();
        assert_eq!(tree(&dir), vec![("mine.bas".to_string(), "user file".to_string())]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_cancelled_or_failed_upgrade_restores_the_previous_version() {
        let dir = temp("rollback");
        let o = Options::new(dir.clone());
        let v1 = bundle(&[("qb", "one"), ("compiler/stdlib/a.bas", "a1"), ("compiler/stdlib/gone.bas", "g")]);
        install_with(&o, &v1, &mut Recorder::default(), &mut |_, _| Ok(())).unwrap();
        let before = tree(&dir);

        let v2 = bundle(&[("compiler/new/n.bas", "n"), ("compiler/stdlib/a.bas", "a2"), ("qb", "two")]);
        let mut rec = Recorder { stop_at: Some("qb"), ..Recorder::default() };
        assert_eq!(install_with(&o, &v2, &mut rec, &mut |_, _| Ok(())), Err(CANCELLED.to_string()));
        assert_eq!(tree(&dir), before, "cancelled while extracting");

        let failed = install_with(&o, &v2, &mut Recorder::default(), &mut |_, _| Err("no registry".into()));
        assert_eq!(failed, Err("no registry".to_string()));
        assert_eq!(tree(&dir), before, "failed after every file was written");

        // a folder where a file must go stops the extraction
        std::fs::create_dir_all(dir.join("compiler/new/n.bas")).unwrap();
        assert!(install_with(&o, &v2, &mut Recorder::default(), &mut |_, _| Ok(())).is_err());
        let _ = std::fs::remove_dir_all(dir.join("compiler/new"));
        assert_eq!(tree(&dir), before, "failed while extracting");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Cancels at the `n`-th question, before any file of the new version is written.
    struct CancelAt {
        asked: std::cell::Cell<u32>,
        n: u32,
        extracted: bool,
    }

    impl Progress for CancelAt {
        fn event(&mut self, event: Event) {
            self.extracted |= event == Event::Step(Step::Extract);
        }

        fn cancelled(&self) -> bool {
            self.asked.set(self.asked.get() + 1);
            self.asked.get() >= self.n
        }
    }

    #[test]
    fn an_error_while_setting_the_old_version_aside_puts_it_back() {
        let dir = temp("early");
        let o = Options::new(dir.clone());
        let v1 = bundle(&[("qb", "one"), ("compiler/stdlib/a.bas", "a1")]);
        install_with(&o, &v1, &mut Recorder::default(), &mut |_, _| Ok(())).unwrap();
        let before = tree(&dir);

        // the third question comes after two old files were moved into the backup
        let v2 = bundle(&[("qb", "two"), ("compiler/stdlib/a.bas", "a2")]);
        let mut early = CancelAt { asked: std::cell::Cell::new(0), n: 3, extracted: false };
        assert_eq!(install_with(&o, &v2, &mut early, &mut |_, _| Ok(())), Err(CANCELLED.to_string()));
        assert!(!early.extracted, "stopped while preparing");
        assert_eq!(tree(&dir), before);
        assert!(!dir.join(BACKUP).exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_failed_first_install_leaves_nothing() {
        let dir = temp("fresh");
        let o = Options::new(dir.clone());
        let v1 = bundle(&[("qb", "one"), ("compiler/stdlib/a.bas", "a1")]);
        let mut rec = Recorder { stop_at: Some("qb"), ..Recorder::default() };
        assert!(install_with(&o, &v1, &mut rec, &mut |_, _| Ok(())).is_err());
        assert!(!dir.exists());
    }

    #[test]
    fn an_install_over_files_that_are_already_there_replaces_them() {
        let dir = temp("over");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("README.md"), "old").unwrap();
        std::fs::write(dir.join("qb"), "old qb").unwrap();
        std::fs::write(dir.join("main.bas"), "PRINT 1").unwrap();
        let o = Options::new(dir.clone());
        let v1 = bundle(&[("qb", "one"), ("README.md", "new")]);
        install_with(&o, &v1, &mut Recorder::default(), &mut |_, _| Ok(())).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("README.md")).unwrap(), "new");
        assert_eq!(std::fs::read_to_string(dir.join("qb")).unwrap(), "one");
        // a file the bundle does not carry is not QBNex's: it stays, also after an uninstall
        assert!(dir.join("main.bas").is_file());
        assert!(!dir.join(BACKUP).exists());
        remove_listed(&dir).unwrap();
        assert!(dir.join("main.bas").is_file());
        assert!(!dir.join("README.md").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_failed_install_over_files_that_are_already_there_puts_them_back() {
        let dir = temp("overfail");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("README.md"), "old").unwrap();
        std::fs::write(dir.join("qb"), "old qb").unwrap();
        let before = tree(&dir);
        let o = Options::new(dir.clone());
        let v1 = bundle(&[("qb", "one"), ("README.md", "new")]);
        // the step that touches the user's settings fails after every file was written
        assert!(install_with(&o, &v1, &mut Recorder::default(), &mut |_, _| Err("no".into())).is_err());
        assert_eq!(tree(&dir), before);
        assert_eq!(std::fs::read_to_string(dir.join("README.md")).unwrap(), "old");
        assert!(!dir.join(BACKUP).exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_install_killed_half_way_is_undone_by_the_next_run() {
        let dir = temp("killed");
        let o = Options::new(dir.clone());
        let v1 = bundle(&[("qb", "one"), ("compiler/stdlib/a.bas", "a1")]);
        install_with(&o, &v1, &mut Recorder::default(), &mut |_, _| Ok(())).unwrap();
        let before = tree(&dir);
        // what a killed upgrade leaves: old files in the backup, one new file written
        std::fs::create_dir_all(dir.join(BACKUP).join("compiler/stdlib")).unwrap();
        for rel in ["qb", "compiler/stdlib/a.bas", MANIFEST] {
            std::fs::rename(dir.join(rel), dir.join(BACKUP).join(rel)).unwrap();
        }
        std::fs::write(dir.join("qb"), "half").unwrap();
        recover_interrupted(&dir);
        assert_eq!(tree(&dir), before);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn uninstall_removes_only_the_compilers_work_files() {
        let dir = temp("work");
        let work = dir.join("target");
        std::fs::create_dir_all(work.join("qb-temp2")).unwrap();
        std::fs::write(work.join("qb-temp2/program.qir"), "x").unwrap();
        std::fs::write(work.join("qb-config.ini"), "x").unwrap();
        remove_work_files(&dir);
        assert!(!work.exists(), "a work folder holding only compiler files goes");

        std::fs::create_dir_all(work.join("qb-temp")).unwrap();
        std::fs::create_dir_all(work.join("release")).unwrap();
        std::fs::write(work.join("release/mine.exe"), "user build").unwrap();
        remove_work_files(&dir);
        assert!(!work.join("qb-temp").exists());
        assert!(work.join("release/mine.exe").is_file(), "other content of target stays");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_manifest_line_that_leaves_the_folder_is_ignored() {
        let dir = temp("escape");
        std::fs::create_dir_all(dir.join("in")).unwrap();
        let outside = dir.join("outside.txt");
        std::fs::write(&outside, "x").unwrap();
        std::fs::write(dir.join("in").join(MANIFEST), "../outside.txt\n").unwrap();
        remove_listed(&dir.join("in")).unwrap();
        assert!(outside.is_file());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
