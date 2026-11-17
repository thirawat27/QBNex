//! Installing and removing QBNex, the same for the console and the window.
//!
//! An install never leaves a half-written folder behind. The files of the version
//! being replaced are moved into [`BACKUP`] first; when anything fails, or the user
//! cancels, the new files are deleted and the old ones moved back. A file of the
//! user's with the name of a file QBNex installs is not deleted: it is kept under
//! another name ([`KEPT_SUFFIX`]).

use std::io::Write;
use std::mem::DropGuard;
use std::path::{Path, PathBuf};

use crate::{payload, platform};

/// Lists every file the installer wrote, relative to the install folder, so an upgrade
/// or uninstall removes exactly those (and leaves files the user added). Lines that
/// start with `#` are notes; the first one names the installed version.
pub const MANIFEST: &str = ".qbnex-install";
/// Holds the files of the previous version while a new one is written. An install
/// starts only when it is gone, so what is in it belongs to one install.
const BACKUP: &str = ".qbnex-backup";
/// File in the backup folder that names the install it belongs to; written before
/// the first file is moved in.
const BACKUP_ID: &str = ".id";
/// File in the backup folder of an install that worked, holding the name in
/// [`BACKUP_ID`]: what is left of the folder is rubbish to delete, not a previous
/// version to bring back. A mark with another name is not this install's and is
/// not taken for its success.
const BACKUP_DONE: &str = ".done";
/// File in the backup folder listing the user's files the install renamed to keep
/// them, so a killed install can give them their names back.
const BACKUP_KEPT: &str = ".kept";
/// The bookkeeping files of the backup folder, in the order they are removed: the
/// mark of a finished install goes last, so a half-cleared folder still has it.
const BACKUP_MARKS: [&str; 3] = [BACKUP_ID, BACKUP_KEPT, BACKUP_DONE];
/// Files of finished installs that could not be deleted because they were still
/// open. Never brought back; deleted by the next run once they are closed.
const LEFTOVER: &str = ".qbnex-leftover";
/// Added to the name of a file of the user's that has the name of a file QBNex
/// installs (`README.md` becomes `README.md.before-qbnex`).
const KEPT_SUFFIX: &str = ".before-qbnex";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub use payload::CANCELLED;

/// What an install that worked has to tell the user.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Report {
    /// Files of the user's that had the name of a file QBNex installs: the name and
    /// the name the user's copy is kept under.
    pub kept: Vec<(String, String)>,
    /// Things about this machine the user should fix or know (another `qb` first on PATH).
    pub warnings: Vec<String>,
}

impl Report {
    /// The report as lines of text, the same in the terminal and in the setup window.
    pub fn lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if !self.kept.is_empty() {
            lines.push("Files of yours with the name of a QBNex file were kept under a new name:".to_string());
            lines.extend(self.kept.iter().map(|(name, kept)| format!("  {name} -> {kept}")));
        }
        lines.extend(self.warnings.iter().map(|w| format!("Warning: {w}")));
        lines
    }
}

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

/// A name no other install shares: the process and the time it started.
fn transaction_id() -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{}-{nanos}", std::process::id())
}

/// True when `path` names anything, a broken link included.
fn exists(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

/// The files moved aside, kept and written by one install, so it can be undone.
struct Transaction<'a> {
    dir: &'a Path,
    /// The name written into the backup folder ([`BACKUP_ID`], [`BACKUP_DONE`]).
    id: String,
    moved: Vec<String>,
    /// Files of the user's renamed in place: the name and the name they are kept under.
    kept: Vec<(String, String)>,
    written: Vec<String>,
}

impl<'a> Transaction<'a> {
    /// Starts an install in `dir`. The transaction rolls back when it is dropped, so
    /// every way out of the install (an error, a cancel) undoes it, unless it is taken
    /// out of the guard and committed.
    fn begin(dir: &'a Path) -> DropGuard<Transaction<'a>, impl FnOnce(Transaction<'a>)> {
        let tx = Transaction { dir, id: transaction_id(), moved: Vec::new(), kept: Vec::new(), written: Vec::new() };
        DropGuard::new(tx, Transaction::roll_back)
    }

    fn backup(&self) -> PathBuf {
        self.dir.join(BACKUP)
    }

    /// The backup folder, made with this install's name in it before anything else
    /// goes there.
    fn open_backup(&self) -> Result<PathBuf, String> {
        let backup = self.backup();
        let id = backup.join(BACKUP_ID);
        if !id.is_file() {
            std::fs::create_dir_all(&backup).map_err(|e| format!("cannot create {}: {e}", backup.display()))?;
            std::fs::write(&id, &self.id).map_err(|e| format!("cannot write {}: {e}", id.display()))?;
        }
        Ok(backup)
    }

    /// Moves a file of the previous version into the backup folder.
    fn set_aside(&mut self, rel: &str) -> Result<(), String> {
        let Some(from) = payload::safe_join(self.dir, rel) else {
            return Ok(());
        };
        if !from.is_file() && !is_link(&from) {
            return Ok(());
        }
        let to = self.open_backup()?.join(rel);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
        }
        std::fs::rename(&from, &to)
            .map_err(|e| format!("{} is in use or cannot be replaced; close running QBNex programs and try again ({e})", from.display()))?;
        self.moved.push(rel.to_string());
        Ok(())
    }

    /// Renames a file of the user's that has the name of a file this install writes,
    /// so it is not lost (it is not QBNex's: no manifest lists it).
    fn keep(&mut self, rel: &str) -> Result<(), String> {
        let Some(from) = payload::safe_join(self.dir, rel) else {
            return Ok(());
        };
        if !from.is_file() && !is_link(&from) {
            return Ok(());
        }
        let kept = kept_name(self.dir, rel);
        let Some(to) = payload::safe_join(self.dir, &kept) else {
            return Ok(());
        };
        // noted before the rename: an install killed after it gives the file its name back
        let note = self.open_backup()?.join(BACKUP_KEPT);
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&note)
            .and_then(|mut f| f.write_all(format!("{rel}\0{kept}\0").as_bytes()))
            .map_err(|e| format!("cannot write {}: {e}", note.display()))?;
        std::fs::rename(&from, &to)
            .map_err(|e| format!("{} is in use or cannot be renamed to {kept}; close the program that uses it and try again ({e})", from.display()))?;
        self.kept.push((rel.to_string(), kept));
        Ok(())
    }

    /// The install worked: the previous version goes. Returns the user's files that
    /// were kept under a new name.
    fn commit(self) -> Vec<(String, String)> {
        let backup = self.backup();
        if backup.is_dir() {
            // A file of the previous version that is still open (a running program, a
            // virus scanner) cannot be deleted on Windows. The mark tells the next run
            // that this install was finished: without it, such a file was taken for
            // a killed install and moved back over the new version.
            let _ = std::fs::write(backup.join(BACKUP_DONE), &self.id);
            discard_backup(self.dir);
        }
        prune_empty_dirs(self.dir, &self.moved);
        // a copy that is the same as the file installed in its place keeps nothing
        let mut kept = self.kept;
        kept.retain(|(rel, name)| match (payload::safe_join(self.dir, rel), payload::safe_join(self.dir, name)) {
            (Some(new), Some(old)) if same_content(&new, &old) => std::fs::remove_file(old).is_err(),
            _ => true,
        });
        kept
    }

    /// The install failed: the new files go and the previous version comes back.
    fn roll_back(self) {
        for rel in &self.written {
            if let Some(path) = payload::safe_join(self.dir, rel) {
                let _ = std::fs::remove_file(path);
            }
        }
        prune_empty_dirs(self.dir, &self.written);
        give_back(self.dir, &self.kept);
        restore(self.dir, &self.moved);
        let _ = std::fs::remove_dir(self.dir); // a folder this install created, now empty
    }
}

/// The name a file of the user's at `rel` is kept under: `rel` with [`KEPT_SUFFIX`],
/// numbered when that name is taken (`README.md.before-qbnex-2`).
fn kept_name(dir: &Path, rel: &str) -> String {
    let mut n = 1;
    loop {
        let name = if n == 1 { format!("{rel}{KEPT_SUFFIX}") } else { format!("{rel}{KEPT_SUFFIX}-{n}") };
        match payload::safe_join(dir, &name) {
            Some(path) if exists(&path) => n += 1,
            _ => return name,
        }
    }
}

/// True when the two files hold the same bytes.
fn same_content(a: &Path, b: &Path) -> bool {
    match (std::fs::metadata(a), std::fs::metadata(b)) {
        (Ok(ma), Ok(mb)) if ma.is_file() && mb.is_file() && ma.len() == mb.len() => {
            matches!((std::fs::read(a), std::fs::read(b)), (Ok(x), Ok(y)) if x == y)
        }
        _ => false,
    }
}

/// The user's files a stopped install renamed: the name and the name kept under.
fn read_kept(backup: &Path) -> Vec<(String, String)> {
    let text = std::fs::read_to_string(backup.join(BACKUP_KEPT)).unwrap_or_default();
    let parts: Vec<&str> = text.split('\0').collect();
    parts.as_chunks::<2>().0.iter().map(|[a, b]| (a.to_string(), b.to_string())).collect()
}

/// Gives the user's files renamed by [`Transaction::keep`] their names back, in
/// place of what the install wrote there.
fn give_back(dir: &Path, kept: &[(String, String)]) {
    for (rel, name) in kept.iter().rev() {
        let (Some(to), Some(from)) = (payload::safe_join(dir, rel), payload::safe_join(dir, name)) else {
            continue;
        };
        // not renamed yet (the install stopped between the note and the rename): the
        // file at `to` is still the user's
        if !exists(&from) {
            continue;
        }
        let _ = std::fs::remove_file(&to);
        let _ = std::fs::rename(&from, &to);
    }
}

/// Moves `files` from the backup folder back into `dir`. The backup folder goes only
/// when every file came back: one that did not stays there for the next run to bring
/// back. Returns whether everything came back.
fn restore(dir: &Path, files: &[String]) -> bool {
    let backup = dir.join(BACKUP);
    let mut all = true;
    for rel in files {
        let (Some(to), from) = (payload::safe_join(dir, rel), backup.join(rel)) else {
            continue;
        };
        if let Some(parent) = to.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if std::fs::rename(&from, &to).is_err() && exists(&from) {
            all = false;
        }
    }
    if all {
        let _ = std::fs::remove_dir_all(backup);
    }
    all
}

/// The files in the backup folder, without its bookkeeping files.
fn backup_files(backup: &Path) -> Vec<String> {
    let mut files = Vec::new();
    files_under(backup, "", &mut files);
    files.retain(|f| !BACKUP_MARKS.contains(&f.as_str()));
    files
}

/// True when the backup folder is what a finished install left: its mark carries
/// the name of the install that made the folder.
fn finished(backup: &Path) -> bool {
    let Ok(done) = std::fs::read_to_string(backup.join(BACKUP_DONE)) else {
        return false;
    };
    match std::fs::read_to_string(backup.join(BACKUP_ID)) {
        Ok(id) => !id.is_empty() && id == done,
        // A mark and no name: the name is removed before the mark once the folder is
        // cleared, and setups that wrote no name wrote the mark only when finished.
        Err(_) => true,
    }
}

/// Deletes the files left in the backup folder of a finished install. One that is
/// still open and cannot be deleted is moved to [`LEFTOVER`], so the backup folder
/// is empty for the next install; the mark goes last, once nothing else is left.
/// Returns the files that could neither be deleted nor moved.
fn discard_backup(dir: &Path) -> Vec<String> {
    let backup = dir.join(BACKUP);
    let files = backup_files(&backup);
    let aside = dir.join(LEFTOVER).join(transaction_id());
    let mut stuck = Vec::new();
    for rel in &files {
        let from = backup.join(rel);
        if std::fs::remove_file(&from).is_ok() {
            continue;
        }
        let to = aside.join(rel);
        if let Some(parent) = to.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if std::fs::rename(&from, &to).is_err() {
            stuck.push(rel.clone());
        }
    }
    prune_empty_dirs(&backup, &files);
    if stuck.is_empty() {
        for mark in BACKUP_MARKS {
            let _ = std::fs::remove_file(backup.join(mark));
        }
        let _ = std::fs::remove_dir(&backup);
    }
    stuck
}

/// Deletes what [`discard_backup`] moved aside, as far as it is closed by now.
/// Returns the files that are still there.
fn clear_leftovers(dir: &Path) -> Vec<PathBuf> {
    let leftover = dir.join(LEFTOVER);
    if !leftover.is_dir() {
        return Vec::new();
    }
    let mut files = Vec::new();
    files_under(&leftover, "", &mut files);
    let stuck: Vec<PathBuf> = files.iter().map(|rel| leftover.join(rel)).filter(|p| std::fs::remove_file(p).is_err() && exists(p)).collect();
    if stuck.is_empty() {
        let _ = std::fs::remove_dir_all(&leftover);
    } else {
        prune_empty_dirs(&leftover, &files);
    }
    stuck
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
/// its place is replaced by the old one, so the folder matches the old manifest again,
/// and a file of the user's it renamed gets its name back. A backup folder with the
/// mark of a finished install is only cleared.
fn recover_interrupted(dir: &Path) {
    clear_leftovers(dir);
    let backup = dir.join(BACKUP);
    if !backup.is_dir() {
        return;
    }
    // what a finished install could not delete: try again, bring nothing back
    if finished(&backup) {
        discard_backup(dir);
        return;
    }
    give_back(dir, &read_kept(&backup));
    let files = backup_files(&backup);
    for rel in &files {
        if let Some(to) = payload::safe_join(dir, rel) {
            let _ = std::fs::remove_file(to);
        }
    }
    restore(dir, &files);
}

/// A new install starts with an empty backup folder, so everything in it is its own.
/// What an earlier install left there and could not be put back stops the install.
fn backup_is_clear(dir: &Path) -> Result<(), String> {
    let backup = dir.join(BACKUP);
    if !backup.is_dir() {
        return Ok(());
    }
    let files = backup_files(&backup);
    if files.is_empty() {
        let _ = std::fs::remove_dir_all(&backup);
        return Ok(());
    }
    Err(format!(
        "{} still holds files of an earlier install that could not be put back ({}); close running QBNex programs and run setup again",
        backup.display(),
        files.join(", ")
    ))
}

/// The compiler keeps its build files and settings in the folder `target` of the
/// install folder (`target/qb-temp`, `target/qb-config.ini`): where that folder
/// cannot be written, the installed compiler would fail on every program.
fn check_work_folder(dir: &Path) -> Result<(), String> {
    let work = dir.join("target");
    let created = !exists(&work);
    let result = std::fs::create_dir_all(&work).and_then(|()| {
        let probe = work.join(format!(".qbnex-setup-probe-{}", std::process::id()));
        std::fs::write(&probe, b"")?;
        std::fs::remove_file(&probe)
    });
    if created {
        let _ = std::fs::remove_dir(&work); // the compiler makes it again
    }
    let sep = std::path::MAIN_SEPARATOR;
    result.map_err(|e| {
        format!(
            "cannot write the compiler's work folder {} ({e}). The compiler writes target{sep}qb-temp and target{sep}qb-config.ini there; install into a folder you can write to",
            work.display()
        )
    })
}

/// Installs the bundle as `o` says and makes it reachable.
pub fn install(o: &Options, bundle: &Bundle, progress: &mut dyn Progress) -> Result<Report, String> {
    let upgrade = o.dir.join(MANIFEST).is_file();
    let result = install_with(o, bundle, progress, &mut |o, size_kb| platform::register(o, VERSION, size_kb));
    if result.is_err() && !upgrade {
        // registering may have stopped half way
        let _ = platform::unregister(&o.dir);
    }
    let mut report = result?;
    if o.add_path
        && let Some(warning) = platform::shadowing(&o.dir)
    {
        report.warnings.push(warning);
    }
    Ok(report)
}

/// [`install`] with the step that touches the user's settings passed in.
pub fn install_with(
    o: &Options, bundle: &Bundle, progress: &mut dyn Progress, register: &mut dyn FnMut(&Options, u64) -> Result<(), String>,
) -> Result<Report, String> {
    let dir = &o.dir;
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    recover_interrupted(dir);
    // before the transaction: rolling back would empty a backup folder it does not own
    backup_is_clear(dir)?;
    let mut tx = Transaction::begin(dir);
    check_work_folder(dir)?;
    write_files(o, bundle, progress, &mut tx)?;
    progress.event(Event::Step(Step::Register));
    register(o, bundle.installed_size() / 1024)?;
    let kept = DropGuard::dismiss(tx).commit();
    Ok(Report { kept, warnings: Vec::new() })
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

    // Installing over what is already there: a file this install writes that the
    // previous manifest does not list (a first install into a folder with files, a
    // copy unpacked by hand, an install killed before it wrote its manifest) is not
    // QBNex's to delete. It is renamed and stays; it gets its name back when the
    // install fails or is cancelled. A copy that turns out the same as the new file
    // goes at the commit, since nothing of the user's is lost with it.
    let mut mine = payload::names(bundle.zip())?;
    mine.push(platform::setup_name().to_string());
    mine.extend(platform::extra_files().iter().map(|(name, _)| name.to_string()));
    for rel in &mine {
        if progress.cancelled() {
            return Err(CANCELLED.into());
        }
        if !tx.moved.contains(rel) {
            tx.keep(rel)?;
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
    // The install stored a full path in PATH, the links and the uninstall entry, and
    // an uninstall finds its own by comparing with it: `--dir .` must match too.
    let dir = std::path::absolute(&dir).map_err(|e| format!("bad install folder {}: {e}", dir.display()))?;
    if dir.join(MANIFEST).is_file() { Ok(dir) } else { Err(format!("no QBNex installation found in {}", dir.display())) }
}

/// Removes the install in `dir` and what made it reachable. The folder itself, and
/// the uninstaller in it, go with [`platform::remove_install_dir`] when the caller
/// is about to exit: on Windows a running program cannot delete its own file.
pub fn uninstall(dir: &Path, progress: &mut dyn Progress) -> Result<(), String> {
    uninstall_with(dir, progress, &mut |dir| platform::unregister(dir), &mut |path| std::fs::remove_file(path))
}

/// [`uninstall`] with the step that touches the user's settings and the deleting of
/// a file passed in.
fn uninstall_with(
    dir: &Path, progress: &mut dyn Progress, unregister: &mut dyn FnMut(&Path) -> Result<(), String>,
    remove: &mut dyn FnMut(&Path) -> std::io::Result<()>,
) -> Result<(), String> {
    progress.event(Event::Step(Step::Remove));
    recover_interrupted(dir);
    // The files first. One that is still open (a running qb, the language server of
    // an editor) cannot be deleted on Windows: QBNex then stays registered, with its
    // uninstall entry and shortcut, so the uninstall can be run again once it is closed.
    let mut in_use = remove_listed_with(dir, remove);
    in_use.extend(clear_leftovers(dir));
    let backup = dir.join(BACKUP);
    in_use.extend(backup_files(&backup).iter().map(|rel| backup.join(rel)));
    if !in_use.is_empty() {
        let list: String = in_use.iter().map(|p| format!("\n  {}", p.display())).collect();
        return Err(format!(
            "these files are in use and were not removed:{list}\nClose the programs that use them (a running QBNex program, a terminal or editor that runs qb) and run the uninstall again. QBNex stays on PATH and in the menus until then."
        ));
    }
    remove_work_files(dir);
    unregister(dir)
}

/// Removes the files an install listed in its manifest, then empty folders. Returns
/// the files that could not be removed; the uninstaller and the manifest then stay,
/// so the uninstall can be run again.
#[cfg(test)]
fn remove_listed(dir: &Path) -> Vec<PathBuf> {
    remove_listed_with(dir, &mut |path| std::fs::remove_file(path))
}

fn remove_listed_with(dir: &Path, remove: &mut dyn FnMut(&Path) -> std::io::Result<()>) -> Vec<PathBuf> {
    let files = listed(dir);
    let me = std::env::current_exe().ok().and_then(|p| p.canonicalize().ok());
    let setup = platform::setup_name();
    let mut failed = Vec::new();
    // the uninstaller last: it goes only when everything else went
    let (last, rest): (Vec<&String>, Vec<&String>) = files.iter().partition(|rel| rel.as_str() == setup);
    for rel in rest.into_iter().chain(last) {
        if rel == setup && !failed.is_empty() {
            continue;
        }
        let Some(path) = payload::safe_join(dir, rel) else {
            continue;
        };
        // a running uninstaller cannot delete itself; platform::remove_install_dir does
        if me.is_some() && path.canonicalize().ok() == me {
            continue;
        }
        if (path.is_file() || is_link(&path))
            && let Err(e) = remove(&path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            failed.push(path);
        }
    }
    prune_empty_dirs(dir, &files);
    if failed.is_empty() {
        let _ = std::fs::remove_file(dir.join(MANIFEST));
    }
    failed
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
        assert!(!dir.join(BACKUP).exists() && !dir.join(LEFTOVER).exists());
        assert_eq!(std::fs::read(dir.join(platform::setup_name())).unwrap(), b"STUB");

        assert!(remove_listed(&dir).is_empty());
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

    fn read(dir: &Path, rel: &str) -> String {
        std::fs::read_to_string(dir.join(rel)).unwrap()
    }

    #[test]
    fn an_install_over_files_that_are_already_there_keeps_them_under_a_new_name() {
        let dir = temp("over");
        std::fs::create_dir_all(dir.join("licenses")).unwrap();
        std::fs::write(dir.join("README.md"), "old").unwrap();
        // the first new name is taken: the user's copy gets the next one
        std::fs::write(dir.join(format!("README.md{KEPT_SUFFIX}")), "older").unwrap();
        std::fs::write(dir.join("qb"), "old qb").unwrap();
        std::fs::write(dir.join("licenses/GPL.txt"), "my notes").unwrap();
        std::fs::write(dir.join("LICENSE"), "same").unwrap();
        std::fs::write(dir.join("main.bas"), "PRINT 1").unwrap();
        let o = Options::new(dir.clone());
        let v1 = bundle(&[("qb", "one"), ("README.md", "new"), ("licenses/GPL.txt", "gpl"), ("LICENSE", "same")]);
        let report = install_with(&o, &v1, &mut Recorder::default(), &mut |_, _| Ok(())).unwrap();
        assert_eq!(read(&dir, "README.md"), "new");
        assert_eq!(read(&dir, "qb"), "one");
        assert_eq!(read(&dir, &format!("README.md{KEPT_SUFFIX}")), "older");
        assert_eq!(read(&dir, &format!("README.md{KEPT_SUFFIX}-2")), "old");
        assert_eq!(read(&dir, &format!("qb{KEPT_SUFFIX}")), "old qb");
        assert_eq!(read(&dir, &format!("licenses/GPL.txt{KEPT_SUFFIX}")), "my notes");
        // the same bytes as the new file: nothing to keep
        assert!(!dir.join(format!("LICENSE{KEPT_SUFFIX}")).exists());
        let mut kept = report.kept.clone();
        kept.sort();
        assert_eq!(
            kept,
            [
                ("README.md".to_string(), format!("README.md{KEPT_SUFFIX}-2")),
                ("licenses/GPL.txt".to_string(), format!("licenses/GPL.txt{KEPT_SUFFIX}")),
                ("qb".to_string(), format!("qb{KEPT_SUFFIX}")),
            ]
        );
        assert!(report.lines().iter().any(|l| l.contains(&format!("README.md -> README.md{KEPT_SUFFIX}-2"))));
        // a file the bundle does not carry is not QBNex's: it stays
        assert!(dir.join("main.bas").is_file());
        assert!(!dir.join(BACKUP).exists());

        // the manifest lists the files now: an upgrade replaces them without keeping
        let v2 = bundle(&[("qb", "two"), ("README.md", "newer"), ("licenses/GPL.txt", "gpl"), ("LICENSE", "same")]);
        let report = install_with(&o, &v2, &mut Recorder::default(), &mut |_, _| Ok(())).unwrap();
        assert!(report.kept.is_empty(), "{report:?}");
        assert_eq!(read(&dir, "README.md"), "newer");
        assert!(!dir.join(format!("README.md{KEPT_SUFFIX}-3")).exists());

        // the kept copies are the user's: an uninstall leaves them
        assert!(remove_listed(&dir).is_empty());
        assert!(dir.join("main.bas").is_file());
        assert!(!dir.join("README.md").exists());
        assert_eq!(read(&dir, &format!("README.md{KEPT_SUFFIX}-2")), "old");
        assert_eq!(read(&dir, &format!("licenses/GPL.txt{KEPT_SUFFIX}")), "my notes");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_install_killed_after_keeping_a_file_gives_it_its_name_back() {
        let dir = temp("killedkeep");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("README.md"), "mine").unwrap();
        let before = tree(&dir);
        // what the killed install left: the note, the renamed file, the new file
        std::fs::create_dir_all(dir.join(BACKUP)).unwrap();
        std::fs::write(dir.join(BACKUP).join(BACKUP_ID), "1-2").unwrap();
        let kept = format!("README.md{KEPT_SUFFIX}");
        std::fs::write(dir.join(BACKUP).join(BACKUP_KEPT), format!("README.md\0{kept}\0")).unwrap();
        std::fs::rename(dir.join("README.md"), dir.join(&kept)).unwrap();
        std::fs::write(dir.join("README.md"), "new").unwrap();
        recover_interrupted(&dir);
        assert_eq!(tree(&dir), before);
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
    fn leftovers_of_a_finished_install_are_not_brought_back() {
        let dir = temp("leftover");
        let o = Options::new(dir.clone());
        let v2 = bundle(&[("qb", "two")]);
        install_with(&o, &v2, &mut Recorder::default(), &mut |_, _| Ok(())).unwrap();
        // an old file the finished upgrade could not delete (it was open), with the
        // mark of that upgrade; then the same as left by a setup that wrote no name
        for id in [Some("7-11"), None] {
            std::fs::create_dir_all(dir.join(BACKUP)).unwrap();
            std::fs::write(dir.join(BACKUP).join("qb"), "one").unwrap();
            if let Some(id) = id {
                std::fs::write(dir.join(BACKUP).join(BACKUP_ID), id).unwrap();
            }
            std::fs::write(dir.join(BACKUP).join(BACKUP_DONE), id.unwrap_or("")).unwrap();
            recover_interrupted(&dir);
            assert_eq!(read(&dir, "qb"), "two", "{id:?}");
            assert!(!dir.join(BACKUP).exists() && !dir.join(LEFTOVER).exists());
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_mark_of_another_install_is_not_taken_for_this_one() {
        let dir = temp("stale");
        let o = Options::new(dir.clone());
        let v1 = bundle(&[("qb", "one"), ("compiler/stdlib/a.bas", "a1")]);
        install_with(&o, &v1, &mut Recorder::default(), &mut |_, _| Ok(())).unwrap();
        let before = tree(&dir);
        // a killed upgrade whose backup folder holds the mark of an earlier, finished one
        std::fs::create_dir_all(dir.join(BACKUP).join("compiler/stdlib")).unwrap();
        std::fs::write(dir.join(BACKUP).join(BACKUP_ID), "2-2").unwrap();
        std::fs::write(dir.join(BACKUP).join(BACKUP_DONE), "1-1").unwrap();
        for rel in ["qb", "compiler/stdlib/a.bas", MANIFEST] {
            std::fs::rename(dir.join(rel), dir.join(BACKUP).join(rel)).unwrap();
        }
        std::fs::write(dir.join("qb"), "half").unwrap();
        recover_interrupted(&dir);
        assert_eq!(tree(&dir), before, "the previous version comes back");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_install_does_not_start_over_a_backup_it_cannot_clear() {
        let dir = temp("stuck");
        let o = Options::new(dir.clone());
        let v1 = bundle(&[("qb", "one")]);
        install_with(&o, &v1, &mut Recorder::default(), &mut |_, _| Ok(())).unwrap();
        // an old file of a killed upgrade that cannot go back: a folder holds its place
        std::fs::create_dir_all(dir.join(BACKUP)).unwrap();
        std::fs::write(dir.join(BACKUP).join("tool"), "old tool").unwrap();
        std::fs::create_dir_all(dir.join("tool").join("x")).unwrap();
        let v2 = bundle(&[("qb", "two")]);
        let e = install_with(&o, &v2, &mut Recorder::default(), &mut |_, _| Ok(())).unwrap_err();
        assert!(e.contains(BACKUP) && e.contains("tool"), "{e}");
        assert_eq!(read(&dir, "qb"), "one", "nothing was installed");
        assert_eq!(read(&dir, &format!("{BACKUP}/tool")), "old tool", "the old file is not lost");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_install_where_the_compiler_cannot_write_its_work_folder_fails() {
        let dir = temp("nowork");
        std::fs::create_dir_all(&dir).unwrap();
        // a file where the work folder must go
        std::fs::write(dir.join("target"), "not a folder").unwrap();
        let before = tree(&dir);
        let o = Options::new(dir.clone());
        let e = install_with(&o, &bundle(&[("qb", "one")]), &mut Recorder::default(), &mut |_, _| Ok(())).unwrap_err();
        assert!(e.contains("qb-temp") && e.contains("target"), "{e}");
        assert_eq!(tree(&dir), before);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn files_in_use_keep_the_install_registered_until_they_are_gone() {
        let dir = temp("inuse");
        let o = Options::new(dir.clone());
        let v1 = bundle(&[("qb", "one"), ("compiler/stdlib/a.bas", "a1")]);
        install_with(&o, &v1, &mut Recorder::default(), &mut |_, _| Ok(())).unwrap();
        let mut unregistered = false;
        let mut in_use = |p: &Path| if p.ends_with("qb") { Err(std::io::Error::other("in use")) } else { std::fs::remove_file(p) };
        let mut unregister = |_: &Path| {
            unregistered = true;
            Ok::<(), String>(())
        };
        let e = uninstall_with(&dir, &mut Recorder::default(), &mut unregister, &mut in_use).unwrap_err();
        assert!(e.contains(&dir.join("qb").display().to_string()), "{e}");
        assert!(!unregistered, "PATH, shortcuts and the uninstall entry stay");
        assert!(dir.join(MANIFEST).is_file() && dir.join(platform::setup_name()).is_file(), "the uninstall can run again");
        assert!(!dir.join("compiler/stdlib/a.bas").exists());

        // closed now: the uninstall finishes, and unregisters only after the files went
        let mut clean = false;
        let mut unregister = |d: &Path| {
            clean = !d.join("qb").exists() && !d.join(MANIFEST).exists();
            Ok::<(), String>(())
        };
        uninstall_with(&dir, &mut Recorder::default(), &mut unregister, &mut |p| std::fs::remove_file(p)).unwrap();
        assert!(clean);
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
        assert!(remove_listed(&dir.join("in")).is_empty());
        assert!(outside.is_file());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
