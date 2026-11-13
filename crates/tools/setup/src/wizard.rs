//! The pages of the setup window and the rules for moving between them, without
//! any drawing: what a page shows comes from here, and so does every check.

use std::path::{Path, PathBuf};

use crate::engine::{CANCELLED, Options};
use crate::text::{Key, fill};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Welcome,
    License,
    Folder,
    Tasks,
    Ready,
    /// Asks before an uninstall.
    Remove,
    /// Installing or removing; no way back.
    Working,
    Finished,
}

/// What the wizard was started for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Install,
    Uninstall,
}

/// What the checks of the folder page need to know about the machine.
pub trait Machine {
    fn free_space(&self, dir: &Path) -> Option<u64>;
    /// The version installed in `dir` (empty when it is not recorded), None when there is no install.
    fn installed_version(&self, dir: &Path) -> Option<String>;
    fn is_file(&self, dir: &Path) -> bool;
    /// True when `dir` exists and holds something.
    fn has_content(&self, dir: &Path) -> bool;
    /// Tries to write where `dir` is or will be created.
    fn can_write(&self, dir: &Path) -> Result<(), String>;
}

/// Why Next did not move on.
#[derive(Clone, Debug, PartialEq)]
pub enum Stop {
    /// Something to correct.
    Problem(String),
    /// A question; [`Wizard::confirm`] answers yes.
    Question(String),
}

pub struct Wizard {
    pub mode: Mode,
    pub page: Page,
    pub accepted: bool,
    /// The folder as typed.
    pub dir: String,
    pub add_path: bool,
    pub menu: bool,
    pub desktop: bool,
    pub associate: bool,
    /// The `.bas` task exists on this system.
    pub can_associate: bool,
    /// Bytes the install needs.
    pub required: u64,
    /// How the install or uninstall ended.
    pub outcome: Option<Result<(), String>>,
    /// A folder with other files in it the user agreed to use.
    agreed_dir: Option<String>,
}

const MB: u64 = 1024 * 1024;

impl Wizard {
    pub fn new(mode: Mode, options: &Options, required: u64, can_associate: bool) -> Wizard {
        Wizard {
            mode,
            page: if mode == Mode::Install { Page::Welcome } else { Page::Remove },
            accepted: false,
            dir: options.dir.to_string_lossy().into_owned(),
            add_path: options.add_path,
            menu: options.menu,
            desktop: options.desktop,
            associate: options.associate && can_associate,
            can_associate,
            required,
            outcome: None,
            agreed_dir: None,
        }
    }

    pub fn text(&self, key: Key) -> &'static str {
        key.text()
    }

    pub fn folder(&self) -> PathBuf {
        PathBuf::from(self.dir.trim())
    }

    pub fn options(&self) -> Options {
        Options { dir: self.folder(), add_path: self.add_path, menu: self.menu, desktop: self.desktop, associate: self.associate && self.can_associate }
    }

    pub fn can_back(&self) -> bool {
        matches!(self.page, Page::License | Page::Folder | Page::Tasks | Page::Ready)
    }

    pub fn can_next(&self) -> bool {
        match self.page {
            Page::License => self.accepted,
            Page::Working => false,
            _ => true,
        }
    }

    /// Cancel is offered until the work is done; while working it stops an install
    /// (an uninstall cannot be stopped half way).
    pub fn can_cancel(&self) -> bool {
        match self.page {
            Page::Finished => false,
            Page::Working => self.mode == Mode::Install,
            _ => true,
        }
    }

    pub fn next_label(&self) -> Key {
        match self.page {
            Page::Ready => Key::Install,
            Page::Remove => Key::Remove,
            Page::Finished => Key::Finish,
            _ => Key::Next,
        }
    }

    pub fn back(&mut self) {
        self.page = match self.page {
            Page::License => Page::Welcome,
            Page::Folder => Page::License,
            Page::Tasks => Page::Folder,
            Page::Ready => Page::Tasks,
            other => other,
        };
    }

    /// Moves to the next page when the current one is in order. Reaching
    /// [`Page::Working`] means the caller starts the work now.
    pub fn next(&mut self, machine: &dyn Machine) -> Result<(), Stop> {
        self.page = match self.page {
            Page::Welcome => Page::License,
            Page::License if self.accepted => Page::Folder,
            Page::License => return Ok(()),
            Page::Folder => {
                self.check_folder(machine)?;
                Page::Tasks
            }
            Page::Tasks => Page::Ready,
            Page::Ready | Page::Remove => Page::Working,
            Page::Working | Page::Finished => return Ok(()),
        };
        Ok(())
    }

    /// Yes to the question [`Wizard::next`] stopped with.
    pub fn confirm(&mut self) {
        self.agreed_dir = Some(self.dir.trim().to_string());
    }

    /// The work ended.
    pub fn finish(&mut self, outcome: Result<(), String>) {
        self.outcome = Some(outcome);
        self.page = Page::Finished;
    }

    pub fn cancelled(&self) -> bool {
        self.outcome.as_ref().is_some_and(|o| o.as_ref().is_err_and(|e| e == CANCELLED))
    }

    fn check_folder(&self, machine: &dyn Machine) -> Result<(), Stop> {
        let problem = |key: Key| Stop::Problem(self.text(key).to_string());
        let typed = self.dir.trim();
        if typed.is_empty() {
            return Err(problem(Key::FolderEmpty));
        }
        let dir = self.folder();
        if !dir.is_absolute() {
            let example = if cfg!(windows) { r"C:\QBNex" } else { "/opt/qbnex" };
            return Err(Stop::Problem(fill(self.text(Key::FolderRelative), example)));
        }
        if machine.is_file(&dir) {
            return Err(problem(Key::FolderIsFile));
        }
        if let Err(e) = machine.can_write(&dir) {
            return Err(Stop::Problem(fill(self.text(Key::FolderNoWrite), &e)));
        }
        if machine.free_space(&dir).is_some_and(|free| free < self.required) {
            return Err(problem(Key::FolderNoSpace));
        }
        let upgrade = machine.installed_version(&dir).is_some();
        if !upgrade && machine.has_content(&dir) && self.agreed_dir.as_deref() != Some(typed) {
            return Err(Stop::Question(self.text(Key::FolderInUse).to_string()));
        }
        Ok(())
    }

    /// The lines below the folder box: space needed and free, and what is installed there.
    pub fn folder_notes(&self, machine: &dyn Machine) -> Vec<String> {
        let mb = |bytes: u64| bytes.div_ceil(MB).to_string();
        let mut notes = vec![fill(self.text(Key::SpaceRequired), &mb(self.required))];
        let dir = self.folder();
        if !dir.is_absolute() {
            return notes;
        }
        if let Some(free) = machine.free_space(&dir) {
            notes.push(fill(self.text(Key::SpaceFree), &(free / MB).to_string()));
        }
        match machine.installed_version(&dir) {
            Some(v) if v.is_empty() => notes.push(self.text(Key::UpgradeUnknown).to_string()),
            Some(v) => notes.push(fill(self.text(Key::Upgrade), &v)),
            None => {}
        }
        notes
    }

    /// The tasks that are on, as the Ready page lists them.
    pub fn chosen_tasks(&self) -> Vec<&'static str> {
        let tasks = [
            (self.add_path, Key::TaskPath),
            (self.menu, Key::TaskMenu),
            (self.desktop, Key::TaskDesktop),
            (self.associate && self.can_associate, Key::TaskAssociate),
        ];
        tasks.iter().filter(|(on, _)| *on).map(|(_, key)| self.text(*key)).collect()
    }
}

/// The machine the installer runs on.
pub struct ThisMachine;

impl Machine for ThisMachine {
    fn free_space(&self, dir: &Path) -> Option<u64> {
        crate::platform::free_space(dir)
    }

    fn installed_version(&self, dir: &Path) -> Option<String> {
        crate::engine::installed_version(dir)
    }

    fn is_file(&self, dir: &Path) -> bool {
        dir.is_file()
    }

    fn has_content(&self, dir: &Path) -> bool {
        std::fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_some())
    }

    fn can_write(&self, dir: &Path) -> Result<(), String> {
        // the folder may not exist yet: what counts is the folder it will be created in
        let Some(base) = dir.ancestors().find(|p| p.is_dir()) else {
            return Err(format!("{} does not exist", dir.ancestors().last().unwrap_or(dir).display()));
        };
        let probe = base.join(format!(".qbnex-setup-probe-{}", std::process::id()));
        std::fs::write(&probe, b"").map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(probe);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        free: Option<u64>,
        installed: Option<&'static str>,
        file: bool,
        content: bool,
        write: Result<(), String>,
    }

    impl Default for Fake {
        fn default() -> Fake {
            Fake { free: Some(500 * MB), installed: None, file: false, content: false, write: Ok(()) }
        }
    }

    impl Machine for Fake {
        fn free_space(&self, _: &Path) -> Option<u64> {
            self.free
        }
        fn installed_version(&self, _: &Path) -> Option<String> {
            self.installed.map(str::to_string)
        }
        fn is_file(&self, _: &Path) -> bool {
            self.file
        }
        fn has_content(&self, _: &Path) -> bool {
            self.content
        }
        fn can_write(&self, _: &Path) -> Result<(), String> {
            self.write.clone()
        }
    }

    fn full_path() -> PathBuf {
        PathBuf::from(if cfg!(windows) { r"C:\Apps\QBNex" } else { "/opt/qbnex" })
    }

    fn wizard() -> Wizard {
        Wizard::new(Mode::Install, &Options::new(full_path()), 40 * MB, true)
    }

    fn at_folder() -> Wizard {
        let mut w = wizard();
        w.next(&Fake::default()).unwrap();
        w.accepted = true;
        w.next(&Fake::default()).unwrap();
        assert_eq!(w.page, Page::Folder);
        w
    }

    #[test]
    fn the_pages_follow_each_other_and_the_license_must_be_accepted() {
        let ok = Fake::default();
        let mut w = wizard();
        assert!(!w.can_back() && w.can_next() && w.can_cancel());
        w.next(&ok).unwrap();
        assert_eq!(w.page, Page::License);
        assert!(!w.can_next());
        w.next(&ok).unwrap();
        assert_eq!(w.page, Page::License, "Next does nothing until the agreement is accepted");
        w.accepted = true;
        for page in [Page::Folder, Page::Tasks, Page::Ready] {
            w.next(&ok).unwrap();
            assert_eq!(w.page, page);
        }
        assert_eq!(w.next_label(), Key::Install);
        w.back();
        assert_eq!(w.page, Page::Tasks);
        w.next(&ok).unwrap();
        w.next(&ok).unwrap();
        assert_eq!(w.page, Page::Working);
        assert!(!w.can_back() && !w.can_next() && w.can_cancel());
        w.back();
        assert_eq!(w.page, Page::Working, "no way back while installing");
        w.finish(Err(CANCELLED.into()));
        assert!(w.cancelled() && !w.can_cancel() && w.page == Page::Finished);
        assert_eq!(w.next_label(), Key::Finish);
    }

    #[test]
    fn the_folder_is_checked_before_moving_on() {
        let problem = |w: &mut Wizard, m: &Fake| match w.next(m) {
            Err(Stop::Problem(text)) => text,
            other => panic!("{other:?}"),
        };
        let mut w = at_folder();
        w.dir = "  ".into();
        assert_eq!(problem(&mut w, &Fake::default()), Key::FolderEmpty.text());
        w.dir = "relative/folder".into();
        assert!(problem(&mut w, &Fake::default()).starts_with("The folder must be a full path"));
        w.dir = full_path().to_string_lossy().into_owned();
        assert_eq!(problem(&mut w, &Fake { file: true, ..Fake::default() }), Key::FolderIsFile.text());
        assert!(problem(&mut w, &Fake { write: Err("access denied".into()), ..Fake::default() }).ends_with("access denied"));
        assert_eq!(problem(&mut w, &Fake { free: Some(39 * MB), ..Fake::default() }), Key::FolderNoSpace.text());
        assert_eq!(w.page, Page::Folder);
        // free space that cannot be told does not stop the install
        w.next(&Fake { free: None, ..Fake::default() }).unwrap();
        assert_eq!(w.page, Page::Tasks);
    }

    #[test]
    fn a_folder_with_other_files_needs_a_yes_and_an_install_in_it_does_not() {
        let used = Fake { content: true, ..Fake::default() };
        let mut w = at_folder();
        assert!(matches!(w.next(&used), Err(Stop::Question(_))));
        assert_eq!(w.page, Page::Folder);
        w.confirm();
        w.next(&used).unwrap();
        assert_eq!(w.page, Page::Tasks);
        // the yes was for that folder only
        w.back();
        w.dir.push('2');
        assert!(matches!(w.next(&used), Err(Stop::Question(_))));

        let mut w = at_folder();
        w.next(&Fake { content: true, installed: Some("1.0.0"), ..Fake::default() }).unwrap();
        assert_eq!(w.page, Page::Tasks);
    }

    #[test]
    fn notes_and_tasks_follow_the_choices() {
        let mut w = at_folder();
        let notes = w.folder_notes(&Fake { installed: Some("1.0.0"), ..Fake::default() });
        assert_eq!(notes, ["Space required: 40 MB", "Space available: 500 MB", "QBNex 1.0.0 is installed in this folder and will be replaced."]);
        assert_eq!(w.folder_notes(&Fake { installed: Some(""), ..Fake::default() })[2], Key::UpgradeUnknown.text());
        w.dir = "x".into();
        assert_eq!(w.folder_notes(&Fake::default()).len(), 1);

        w.dir = format!("  {}  ", full_path().display());
        w.add_path = false;
        w.desktop = true;
        w.associate = true;
        assert_eq!(w.options(), Options { dir: full_path(), add_path: false, menu: true, desktop: true, associate: true });
        assert_eq!(w.chosen_tasks(), [Key::TaskMenu.text(), Key::TaskDesktop.text(), Key::TaskAssociate.text()]);
        w.can_associate = false;
        assert!(!w.options().associate && w.chosen_tasks().len() == 2);
    }

    #[test]
    fn an_uninstall_asks_once_and_cannot_be_stopped() {
        let mut w = Wizard::new(Mode::Uninstall, &Options::new(full_path()), 0, true);
        assert_eq!((w.page, w.next_label()), (Page::Remove, Key::Remove));
        assert!(w.can_cancel() && !w.can_back());
        w.next(&Fake::default()).unwrap();
        assert_eq!(w.page, Page::Working);
        assert!(!w.can_cancel());
        w.finish(Ok(()));
        assert!(!w.cancelled());
    }
}
