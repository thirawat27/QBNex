//! Where QBNex is installed and how it is made reachable: PATH or links, the
//! applications menu, the desktop, the file manager's menu for `.bas` files and the
//! uninstall entry.
//!
//! With the environment variable [`SANDBOX`] set to a folder, everything outside the
//! install folder goes below that folder (and, on Windows, below a registry key of
//! its own) instead of the user's settings: the installer's own tests install for
//! real without touching the machine they run on.

use std::path::{Path, PathBuf};

pub use imp::*;

use crate::engine::Options;

/// Environment variable naming a folder that stands in for the user's settings.
pub const SANDBOX: &str = "QBNEX_SETUP_SANDBOX";

fn sandbox() -> Option<PathBuf> {
    std::env::var_os(SANDBOX).filter(|v| !v.is_empty()).map(PathBuf::from)
}

/// The existing folder `path` is in or will be created in.
fn existing_ancestor(path: &Path) -> Option<&Path> {
    path.ancestors().find(|p| p.is_dir())
}

#[cfg(windows)]
mod imp {
    use std::os::windows::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::ptr::{null, null_mut};

    use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, STGM_READ};
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    use windows::core::{HSTRING, Interface};
    use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    use windows_sys::Win32::System::Console::{FreeConsole, GetConsoleProcessList};
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_DWORD, REG_EXPAND_SZ, REG_OPTION_NON_VOLATILE, REG_SZ,
        RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE};

    use super::{Options, existing_ancestor, sandbox};

    const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\QBNex";
    /// The entry in the menu Explorer shows for a `.bas` file, whatever program opens it.
    const BAS_VERB_KEY: &str = r"Software\Classes\SystemFileAssociations\.bas\shell\QBNexCompile";
    const ENVIRONMENT_KEY: &str = "Environment";
    /// Registry key that stands in for HKCU when [`super::SANDBOX`] is set.
    const SANDBOX_KEY: &str = r"Software\QBNexSetupSandbox";
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const MENU_FOLDER: &str = "QBNex";
    const PROMPT_LINK: &str = "QBNex Command Prompt.lnk";
    const README_LINK: &str = "QBNex README.lnk";
    const UNINSTALL_LINK: &str = "Uninstall QBNex.lnk";

    pub fn setup_name() -> &'static str {
        "qbnex-setup.exe"
    }

    pub fn compiler_name() -> &'static str {
        "qb.exe"
    }

    /// Files the installer adds to the bundle's on this system.
    pub fn extra_files() -> &'static [(&'static str, &'static [u8])] {
        &[]
    }

    /// The `.bas` menu entry exists on this system.
    pub fn can_associate() -> bool {
        true
    }

    pub fn default_dir() -> Result<PathBuf, String> {
        let base = std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA is not set")?;
        Ok(PathBuf::from(base).join("Programs").join("QBNex"))
    }

    /// Free bytes on the drive `path` is (or will be) on; None when it cannot be told.
    pub fn free_space(path: &Path) -> Option<u64> {
        let p = wide(&existing_ancestor(path)?.to_string_lossy());
        let mut free = 0u64;
        // SAFETY: NUL-terminated path and a valid out pointer.
        (unsafe { GetDiskFreeSpaceExW(p.as_ptr(), &mut free, null_mut(), null_mut()) } != 0).then_some(free)
    }

    /// Closes the console Windows opened for this program when it was started from
    /// Explorer, so only the setup window shows. A console the program was started
    /// from (a terminal, a script) is shared with it and stays.
    pub fn leave_own_console() {
        let mut ids = [0u32; 2];
        // SAFETY: the buffer holds two ids; the call returns how many processes share the console.
        if unsafe { GetConsoleProcessList(ids.as_mut_ptr(), 2) } == 1 {
            // SAFETY: no arguments; this process stops using its console.
            unsafe { FreeConsole() };
        }
    }

    /// True when a window can be shown.
    pub fn has_display() -> bool {
        true
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// The path below HKCU that `path` stands for.
    fn key_path(path: &str) -> String {
        if sandbox().is_some() { format!(r"{SANDBOX_KEY}\{path}") } else { path.to_string() }
    }

    /// An open HKCU key, closed on drop.
    struct Key(HKEY);

    impl Drop for Key {
        fn drop(&mut self) {
            // SAFETY: the handle came from RegCreateKeyExW/RegOpenKeyExW and is closed once.
            unsafe { RegCloseKey(self.0) };
        }
    }

    impl Key {
        fn create(path: &str) -> Result<Key, String> {
            let path = key_path(path);
            let mut h: HKEY = null_mut();
            let p = wide(&path);
            // SAFETY: valid NUL-terminated strings and out pointers.
            let r = unsafe {
                RegCreateKeyExW(HKEY_CURRENT_USER, p.as_ptr(), 0, null(), REG_OPTION_NON_VOLATILE, KEY_READ | KEY_WRITE, null(), &mut h, null_mut())
            };
            if r == ERROR_SUCCESS { Ok(Key(h)) } else { Err(format!("cannot open HKCU\\{path} (error {r})")) }
        }

        fn open(path: &str) -> Option<Key> {
            let mut h: HKEY = null_mut();
            let p = wide(&key_path(path));
            // SAFETY: as above.
            let r = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, p.as_ptr(), 0, KEY_READ | KEY_WRITE, &mut h) };
            (r == ERROR_SUCCESS).then_some(Key(h))
        }

        /// Removes the key and everything below it; a missing key is not an error.
        fn delete(path: &str) {
            let p = wide(&key_path(path));
            // SAFETY: NUL-terminated subkey under HKCU.
            unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, p.as_ptr()) };
        }

        /// A string value and whether it is REG_EXPAND_SZ; `Ok(None)` only when the value
        /// does not exist. A value that exists but cannot be read is an error: a caller
        /// that rewrites the value must not take it for empty.
        fn get_str(&self, name: &str) -> Result<Option<(String, bool)>, String> {
            let n = wide(name);
            let mut ty = 0u32;
            let mut len = 0u32;
            // SAFETY: size query with a null buffer.
            let r = unsafe { RegQueryValueExW(self.0, n.as_ptr(), null(), &mut ty, null_mut(), &mut len) };
            if r == ERROR_FILE_NOT_FOUND {
                return Ok(None);
            }
            if r != ERROR_SUCCESS {
                return Err(format!("cannot read registry value {name} (error {r})"));
            }
            if ty != REG_SZ && ty != REG_EXPAND_SZ {
                return Err(format!("registry value {name} is not a string (type {ty})"));
            }
            let mut buf = vec![0u16; (len as usize).div_ceil(2) + 1];
            let mut bytes = (buf.len() * 2) as u32;
            // SAFETY: `buf` holds `bytes` bytes.
            let r = unsafe { RegQueryValueExW(self.0, n.as_ptr(), null(), &mut ty, buf.as_mut_ptr().cast(), &mut bytes) };
            if r != ERROR_SUCCESS {
                return Err(format!("cannot read registry value {name} (error {r})"));
            }
            let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            Ok(Some((String::from_utf16_lossy(&buf[..end]), ty == REG_EXPAND_SZ)))
        }

        fn set_str(&self, name: &str, value: &str, expand: bool) -> Result<(), String> {
            let n = wide(name);
            let v = wide(value);
            let ty = if expand { REG_EXPAND_SZ } else { REG_SZ };
            // SAFETY: `v` is a NUL-terminated UTF-16 buffer of the given byte length.
            let r = unsafe { RegSetValueExW(self.0, n.as_ptr(), 0, ty, v.as_ptr().cast(), (v.len() * 2) as u32) };
            if r == ERROR_SUCCESS { Ok(()) } else { Err(format!("cannot write registry value {name} (error {r})")) }
        }

        fn set_dword(&self, name: &str, value: u32) -> Result<(), String> {
            let n = wide(name);
            let b = value.to_le_bytes();
            // SAFETY: 4-byte buffer.
            let r = unsafe { RegSetValueExW(self.0, n.as_ptr(), 0, REG_DWORD, b.as_ptr(), 4) };
            if r == ERROR_SUCCESS { Ok(()) } else { Err(format!("cannot write registry value {name} (error {r})")) }
        }
    }

    fn same_dir(a: &str, b: &Path) -> bool {
        let norm = |s: &str| s.trim().trim_end_matches(['\\', '/']).to_lowercase();
        norm(a) == norm(&b.to_string_lossy())
    }

    /// Tells Explorer and new consoles that the user environment changed.
    fn broadcast_environment() {
        if sandbox().is_some() {
            return;
        }
        let env = wide("Environment");
        let mut result = 0usize;
        // SAFETY: broadcast with a NUL-terminated string; the timeout bounds the wait.
        unsafe {
            SendMessageTimeoutW(HWND_BROADCAST, WM_SETTINGCHANGE, 0, env.as_ptr() as isize, SMTO_ABORTIFHUNG, 3000, &mut result);
        }
    }

    fn edit_user_path(dir: &Path, add: bool) -> Result<(), String> {
        let key = Key::create(ENVIRONMENT_KEY)?;
        // an unreadable Path stops here: writing it back as "empty plus our folder"
        // would erase every entry the user had
        let (old, expand) = key.get_str("Path")?.unwrap_or((String::new(), true));
        let d = dir.to_string_lossy().into_owned();
        let mut parts: Vec<&str> = old.split(';').filter(|p| !p.trim().is_empty()).collect();
        let present = parts.iter().any(|p| same_dir(p, dir));
        if add == present {
            return Ok(());
        }
        if add {
            parts.push(&d);
        } else {
            parts.retain(|p| !same_dir(p, dir));
        }
        key.set_str("Path", &parts.join(";"), expand)?;
        broadcast_environment();
        Ok(())
    }

    /// The user's Start menu programs folder and desktop.
    fn shell_folders() -> Result<(PathBuf, PathBuf), String> {
        if let Some(root) = sandbox() {
            return Ok((root.join("menu"), root.join("desktop")));
        }
        let appdata = std::env::var_os("APPDATA").ok_or("APPDATA is not set")?;
        let profile = std::env::var_os("USERPROFILE").ok_or("USERPROFILE is not set")?;
        let menu = PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs");
        // a desktop moved elsewhere (OneDrive) is named in the registry
        let shell = Key::open(r"Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders");
        let moved = shell.and_then(|k| k.get_str("Desktop").ok().flatten()).map(|(v, _)| expand_profile(&v, &profile.to_string_lossy()));
        let desktop = moved.map(PathBuf::from).filter(|p| p.is_dir()).unwrap_or_else(|| PathBuf::from(&profile).join("Desktop"));
        Ok((menu, desktop))
    }

    fn expand_profile(value: &str, profile: &str) -> String {
        let lower = value.to_ascii_lowercase();
        match lower.find("%userprofile%") {
            Some(at) => format!("{}{profile}{}", &value[..at], &value[at + "%userprofile%".len()..]),
            None => value.to_string(),
        }
    }

    /// COM on this thread for as long as the value lives.
    struct Com(bool);

    impl Com {
        fn start() -> Com {
            // SAFETY: plain initialisation; a thread that already has COM reports so and is left alone.
            Com(unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok())
        }
    }

    impl Drop for Com {
        fn drop(&mut self) {
            if self.0 {
                // SAFETY: balances the successful CoInitializeEx above.
                unsafe { CoUninitialize() };
            }
        }
    }

    /// Writes the shortcut `link` to `target`; its working folder is the install folder,
    /// which is also how an uninstall recognises the shortcut as its own.
    fn write_link(link: &Path, target: &str, arguments: &str, dir: &Path, icon: &Path, description: &str) -> Result<(), String> {
        let _com = Com::start();
        let fail = |e: windows::core::Error| format!("cannot create {}: {e}", link.display());
        // SAFETY: COM calls on interfaces this function owns, with strings that outlive them.
        unsafe {
            let shell: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).map_err(fail)?;
            shell.SetPath(&HSTRING::from(target)).map_err(fail)?;
            shell.SetArguments(&HSTRING::from(arguments)).map_err(fail)?;
            shell.SetWorkingDirectory(&HSTRING::from(dir.as_os_str())).map_err(fail)?;
            shell.SetIconLocation(&HSTRING::from(icon.as_os_str()), 0).map_err(fail)?;
            shell.SetDescription(&HSTRING::from(description)).map_err(fail)?;
            let file: IPersistFile = shell.cast().map_err(fail)?;
            file.Save(&HSTRING::from(link.as_os_str()), true).map_err(fail)
        }
    }

    /// The working folder stored in the shortcut `link`.
    fn link_dir(link: &Path) -> Option<String> {
        let _com = Com::start();
        let mut buf = [0u16; 1024];
        // SAFETY: as in `write_link`; the buffer is the size passed.
        unsafe {
            let shell: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
            let file: IPersistFile = shell.cast().ok()?;
            file.Load(&HSTRING::from(link.as_os_str()), STGM_READ).ok()?;
            shell.GetWorkingDirectory(&mut buf).ok()?;
        }
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some(String::from_utf16_lossy(&buf[..end]))
    }

    fn prompt_link(folder: &Path, dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(folder).map_err(|e| format!("cannot create {}: {e}", folder.display()))?;
        let d = dir.display();
        // a prompt that finds `qb` even when PATH was left alone
        let arguments = format!("/k \"set \"PATH={d};%PATH%\" && cd /d \"%USERPROFILE%\" && qb --version\"");
        write_link(&folder.join(PROMPT_LINK), "cmd.exe", &arguments, dir, &dir.join(compiler_name()), "A command prompt with the QBNex compiler")
    }

    fn create_menu(dir: &Path) -> Result<(), String> {
        let folder = shell_folders()?.0.join(MENU_FOLDER);
        prompt_link(&folder, dir)?;
        let readme = dir.join("README.md");
        if readme.is_file() {
            write_link(&folder.join(README_LINK), &readme.to_string_lossy(), "", dir, &dir.join(compiler_name()), "About QBNex")?;
        }
        let setup = dir.join(setup_name());
        write_link(&folder.join(UNINSTALL_LINK), &setup.to_string_lossy(), "--uninstall", dir, &setup, "Remove QBNex")
    }

    /// Removes the shortcuts in `folder` that belong to the install in `dir`.
    fn remove_links(folder: &Path, names: &[&str], dir: &Path) {
        for name in names {
            let link = folder.join(name);
            if link.is_file() && link_dir(&link).is_some_and(|d| same_dir(&d, dir)) {
                let _ = std::fs::remove_file(link);
            }
        }
    }

    fn associate(dir: &Path) -> Result<(), String> {
        let qb = dir.join(compiler_name());
        let verb = Key::create(BAS_VERB_KEY)?;
        verb.set_str("", "Compile with QBNex", false)?;
        verb.set_str("Icon", &qb.to_string_lossy(), false)?;
        // the prompt stays open so the compiler's messages can be read
        Key::create(&format!(r"{BAS_VERB_KEY}\command"))?.set_str("", &format!("cmd.exe /k \"\"{}\" \"%1\"\"", qb.display()), false)
    }

    fn unassociate(dir: &Path) {
        let command = Key::open(&format!(r"{BAS_VERB_KEY}\command")).and_then(|k| k.get_str("").ok().flatten());
        let qb = dir.join(compiler_name()).to_string_lossy().to_lowercase();
        if command.is_some_and(|(c, _)| c.to_lowercase().contains(&qb)) {
            Key::delete(BAS_VERB_KEY);
        }
    }

    pub fn register(o: &Options, version: &str, size_kb: u64) -> Result<(), String> {
        let dir = &o.dir;
        if o.add_path {
            edit_user_path(dir, true)?;
        }
        if o.menu {
            create_menu(dir)?;
        }
        if o.desktop {
            prompt_link(&shell_folders()?.1, dir)?;
        }
        if o.associate {
            associate(dir)?;
        }
        let d = dir.to_string_lossy();
        let setup = dir.join(setup_name());
        let k = Key::create(UNINSTALL_KEY)?;
        k.set_str("DisplayName", "QBNex", false)?;
        k.set_str("DisplayVersion", version, false)?;
        k.set_str("Publisher", "QBNex", false)?;
        k.set_str("URLInfoAbout", "https://github.com/thirawat27/QBNex", false)?;
        k.set_str("InstallLocation", &d, false)?;
        k.set_str("DisplayIcon", &dir.join(compiler_name()).to_string_lossy(), false)?;
        k.set_str("UninstallString", &format!("\"{}\" --uninstall", setup.display()), false)?;
        k.set_str("QuietUninstallString", &format!("\"{}\" --uninstall --yes", setup.display()), false)?;
        k.set_dword("NoModify", 1)?;
        k.set_dword("NoRepair", 1)?;
        k.set_dword("EstimatedSize", u32::try_from(size_kb).unwrap_or(u32::MAX))?;
        Ok(())
    }

    /// Undoes [`register`] for the install in `dir`; what belongs to another install stays.
    pub fn unregister(dir: &Path) -> Result<(), String> {
        if Key::open(ENVIRONMENT_KEY).is_some() {
            edit_user_path(dir, false)?;
        }
        if let Ok((menu, desktop)) = shell_folders() {
            let folder = menu.join(MENU_FOLDER);
            remove_links(&folder, &[PROMPT_LINK, README_LINK, UNINSTALL_LINK], dir);
            let _ = std::fs::remove_dir(folder); // only succeeds when empty
            remove_links(&desktop, &[PROMPT_LINK], dir);
        }
        unassociate(dir);
        let location = Key::open(UNINSTALL_KEY).and_then(|k| k.get_str("InstallLocation").ok().flatten());
        if location.is_none_or(|(l, _)| same_dir(&l, dir)) {
            Key::delete(UNINSTALL_KEY);
        }
        Ok(())
    }

    pub fn path_hint(_dir: &Path) -> String {
        "Open a new terminal and run: qb --version".into()
    }

    /// Shows `path` (a file or a folder) with the program the system uses for it.
    pub fn open(path: &Path) {
        let null = std::process::Stdio::null;
        let _ = std::process::Command::new("explorer").arg(path).stdin(null()).stdout(null()).stderr(null()).spawn();
    }

    /// Removes the (now empty) install folder. A running program cannot delete its own
    /// file on Windows, so when the uninstaller runs from that folder a hidden `cmd`
    /// deletes it (and the folder, only if empty) after this process has exited.
    pub fn remove_install_dir(dir: &Path) {
        let me = std::env::current_exe().ok();
        let inside = me.as_ref().and_then(|m| m.parent()).is_some_and(|p| p == dir);
        if let (true, Some(me)) = (inside, me) {
            let cmd = format!("/c ping 127.0.0.1 -n 3 >nul & del /f /q \"{}\" & rmdir \"{}\"", me.display(), dir.display());
            // the setup window has no console: the helper must not inherit its handles
            let null = std::process::Stdio::null;
            let _ = std::process::Command::new("cmd").raw_arg(cmd).creation_flags(CREATE_NO_WINDOW).stdin(null()).stdout(null()).stderr(null()).spawn();
        } else {
            let _ = std::fs::remove_dir(dir);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn a_missing_value_is_none_and_an_unreadable_one_is_an_error() {
            let path = format!(r"Software\QBNexSetupTest-{}", std::process::id());
            let key = Key::create(&path).unwrap();
            assert_eq!(key.get_str("Missing"), Ok(None));
            key.set_str("Text", "a;b", true).unwrap();
            assert_eq!(key.get_str("Text"), Ok(Some(("a;b".to_string(), true))));
            // not a string: must not read as "no value" (the PATH editor would then
            // replace the user's whole PATH)
            key.set_dword("Number", 1).unwrap();
            assert!(key.get_str("Number").is_err());
            drop(key);
            Key::delete(&path);
        }

        #[test]
        fn a_moved_desktop_is_expanded() {
            assert_eq!(expand_profile(r"%USERPROFILE%\OneDrive\Desktop", r"C:\Users\me"), r"C:\Users\me\OneDrive\Desktop");
            assert_eq!(expand_profile(r"D:\Desk", r"C:\Users\me"), r"D:\Desk");
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::{Path, PathBuf};

    use super::{Options, existing_ancestor, sandbox};

    /// Commands linked into `~/.local/bin`.
    const COMMANDS: [&str; 2] = ["qb", "qbnex"];
    const ICON: &str = "qbnex.png";
    const TERMINAL_NAME: &str = "QBNex Terminal";
    /// Menu entry that opens a terminal with `qb` on PATH.
    const MENU_ENTRY: &str = "qbnex.desktop";
    /// Entry the file manager offers for `.bas` files.
    const COMPILE_ENTRY: &str = "qbnex-compile.desktop";
    const MIME_PACKAGE: &str = "qbnex.xml";

    pub fn setup_name() -> &'static str {
        "qbnex-setup"
    }

    pub fn compiler_name() -> &'static str {
        "qb"
    }

    /// Files the installer adds to the bundle's on this system: the icon of the menu entries.
    pub fn extra_files() -> &'static [(&'static str, &'static [u8])] {
        &[(ICON, include_bytes!("../../../../assets/icons/linux/QBNex.png"))]
    }

    /// The `.bas` menu entry exists on this system. macOS ties file types to
    /// application bundles, and QBNex is a command line compiler without one.
    pub fn can_associate() -> bool {
        cfg!(not(target_os = "macos"))
    }

    fn home() -> Result<PathBuf, String> {
        if let Some(root) = sandbox() {
            return Ok(root);
        }
        std::env::var_os("HOME").map(PathBuf::from).ok_or_else(|| "HOME is not set".to_string())
    }

    fn data_home() -> Result<PathBuf, String> {
        match std::env::var_os("XDG_DATA_HOME") {
            Some(d) if !d.is_empty() && sandbox().is_none() => Ok(PathBuf::from(d)),
            _ => Ok(home()?.join(".local").join("share")),
        }
    }

    pub fn default_dir() -> Result<PathBuf, String> {
        Ok(data_home()?.join("qbnex"))
    }

    fn bin_dir() -> Result<PathBuf, String> {
        Ok(home()?.join(".local").join("bin"))
    }

    /// Free bytes on the file system `path` is (or will be) on; None when it cannot be told.
    pub fn free_space(path: &Path) -> Option<u64> {
        use std::os::unix::ffi::OsStrExt;
        let c = std::ffi::CString::new(existing_ancestor(path)?.as_os_str().as_bytes()).ok()?;
        // SAFETY: a NUL-terminated path and a zeroed structure the call fills in.
        unsafe {
            let mut s: libc::statvfs = std::mem::zeroed();
            #[allow(clippy::unnecessary_cast)] // the field types differ between systems
            (libc::statvfs(c.as_ptr(), &mut s) == 0).then(|| (s.f_bavail as u64).saturating_mul(s.f_frsize as u64))
        }
    }

    /// Nothing to do: a program started from a file manager has no console here.
    pub fn leave_own_console() {}

    /// True when a window can be shown.
    pub fn has_display() -> bool {
        cfg!(target_os = "macos") || ["WAYLAND_DISPLAY", "DISPLAY"].iter().any(|v| std::env::var_os(v).is_some_and(|s| !s.is_empty()))
    }

    /// `p` with symlinks resolved; when it does not exist, without `.` and trailing slashes.
    fn canon(p: &Path) -> PathBuf {
        p.canonicalize().unwrap_or_else(|_| p.components().collect())
    }

    /// True when `link` is a symlink whose target lies in `dir` (or no longer exists).
    /// A relative target is relative to the link's folder, not to the current one, and
    /// the two paths may spell the same folder differently (a symlinked home, a
    /// trailing slash).
    fn ours(link: &Path, dir: &Path) -> bool {
        match std::fs::read_link(link) {
            Ok(t) => {
                let t = if t.is_absolute() { t } else { link.parent().unwrap_or(Path::new(".")).join(t) };
                t.starts_with(dir) || canon(&t).starts_with(canon(dir)) || !t.exists()
            }
            Err(_) => false,
        }
    }

    fn link_commands(dir: &Path) -> Result<(), String> {
        let bin = bin_dir()?;
        std::fs::create_dir_all(&bin).map_err(|e| format!("cannot create {}: {e}", bin.display()))?;
        for name in COMMANDS {
            let target = dir.join(name);
            if !target.is_file() {
                continue;
            }
            let link = bin.join(name);
            if std::fs::symlink_metadata(&link).is_ok() {
                if !ours(&link, dir) {
                    eprintln!("note: {} exists and is not QBNex's; left as is", link.display());
                    continue;
                }
                let _ = std::fs::remove_file(&link);
            }
            std::os::unix::fs::symlink(&target, &link).map_err(|e| format!("cannot link {}: {e}", link.display()))?;
        }
        Ok(())
    }

    /// `text` inside single quotes for `sh`.
    fn quoted(text: &str) -> String {
        format!("'{}'", text.replace('\'', "'\\''"))
    }

    /// A value of a desktop entry's `Exec` key that runs `script` with `sh -c`: the
    /// entry's own quoting needs `"`, `` ` ``, `$` and `\` escaped inside double quotes.
    fn exec_sh(script: &str, arguments: &str) -> String {
        let mut escaped = String::new();
        for c in script.chars() {
            if matches!(c, '"' | '`' | '$' | '\\') {
                escaped.push('\\');
            }
            escaped.push(c);
        }
        format!("sh -c \"{escaped}\" sh{arguments}")
    }

    /// A shell line that puts the install folder first on PATH.
    fn with_path(dir: &Path) -> String {
        format!("PATH={}:\"$PATH\"; export PATH", quoted(&dir.to_string_lossy()))
    }

    fn write_executable(path: &Path, text: &str) -> Result<(), String> {
        use std::os::unix::fs::PermissionsExt;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
        }
        std::fs::write(path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())
    }

    /// The menu entry of the terminal: a desktop entry, or on macOS a script Finder runs in Terminal.
    fn terminal_entry(dir: &Path) -> String {
        if cfg!(target_os = "macos") {
            return format!("#!/bin/sh\n# QBNex {}\n{}\ncd \"$HOME\"\nqb --version\nexec \"${{SHELL:-/bin/sh}}\"\n", dir.display(), with_path(dir));
        }
        let script = format!("{}; cd \"$HOME\"; qb --version; exec \"${{SHELL:-sh}}\"", with_path(dir));
        format!(
            "[Desktop Entry]\nType=Application\nName={TERMINAL_NAME}\nComment=A terminal with the QBNex BASIC compiler\nExec={}\nIcon={}\nTerminal=true\nCategories=Development;\nX-QBNex-Dir={}\n",
            exec_sh(&script, ""),
            dir.join(ICON).display(),
            dir.display()
        )
    }

    fn compile_entry(dir: &Path) -> String {
        // the terminal stays open so the compiler's messages can be read
        let script = format!("{} \"$1\"; printf '\\nPress Enter to close.'; read x", quoted(&dir.join(compiler_name()).to_string_lossy()));
        format!(
            "[Desktop Entry]\nType=Application\nName=Compile with QBNex\nExec={}\nIcon={}\nTerminal=true\nNoDisplay=true\nMimeType=text/x-basic;\nX-QBNex-Dir={}\n",
            exec_sh(&script, " %f"),
            dir.join(ICON).display(),
            dir.display()
        )
    }

    const MIME_XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<mime-info xmlns=\"http://www.freedesktop.org/standards/shared-mime-info\">\n  <mime-type type=\"text/x-basic\">\n    <comment>BASIC source</comment>\n    <sub-class-of type=\"text/plain\"/>\n    <glob pattern=\"*.bas\"/>\n    <glob pattern=\"*.BAS\"/>\n  </mime-type>\n</mime-info>\n";

    /// Where the terminal entry goes in the applications menu and on the desktop.
    fn entry_paths() -> Result<(PathBuf, PathBuf), String> {
        let home = home()?;
        if cfg!(target_os = "macos") {
            let name = format!("{TERMINAL_NAME}.command");
            Ok((home.join("Applications").join(&name), home.join("Desktop").join(name)))
        } else {
            Ok((data_home()?.join("applications").join(MENU_ENTRY), home.join("Desktop").join(MENU_ENTRY)))
        }
    }

    /// Asks the desktop to read its caches again; a desktop without these tools reads the files itself.
    fn refresh_desktop() {
        if sandbox().is_some() {
            return;
        }
        if let Ok(data) = data_home() {
            for (tool, folder) in [("update-mime-database", "mime"), ("update-desktop-database", "applications")] {
                let _ = std::process::Command::new(tool).arg(data.join(folder)).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status();
            }
        }
    }

    pub fn register(o: &Options, _version: &str, _size_kb: u64) -> Result<(), String> {
        let dir = &o.dir;
        if o.add_path {
            link_commands(dir)?;
        }
        let (menu, desktop) = entry_paths()?;
        if o.menu {
            write_executable(&menu, &terminal_entry(dir))?;
        }
        if o.desktop {
            write_executable(&desktop, &terminal_entry(dir))?;
        }
        if o.associate && can_associate() {
            let data = data_home()?;
            write_executable(&data.join("applications").join(COMPILE_ENTRY), &compile_entry(dir))?;
            let package = data.join("mime").join("packages");
            std::fs::create_dir_all(&package).map_err(|e| format!("cannot create {}: {e}", package.display()))?;
            std::fs::write(package.join(MIME_PACKAGE), MIME_XML).map_err(|e| format!("cannot write {MIME_PACKAGE}: {e}"))?;
        }
        if o.menu || o.associate {
            refresh_desktop();
        }
        Ok(())
    }

    /// True when the menu entry or script `path` was written for the install in `dir`.
    fn entry_of(path: &Path, dir: &Path) -> bool {
        let Ok(text) = std::fs::read_to_string(path) else {
            return false;
        };
        let d = dir.display().to_string();
        let want = canon(dir);
        text.lines()
            .filter_map(|l| l.strip_prefix("X-QBNex-Dir=").or(l.strip_prefix("# QBNex ")))
            .any(|v| v == d || canon(Path::new(v)) == want)
    }

    /// Undoes [`register`] for the install in `dir`; what belongs to another install stays.
    pub fn unregister(dir: &Path) -> Result<(), String> {
        let bin = bin_dir()?;
        for name in COMMANDS {
            let link = bin.join(name);
            if ours(&link, dir) {
                let _ = std::fs::remove_file(&link);
            }
        }
        let (menu, desktop) = entry_paths()?;
        let data = data_home()?;
        let compile = data.join("applications").join(COMPILE_ENTRY);
        let mut changed = false;
        if entry_of(&compile, dir) {
            let _ = std::fs::remove_file(&compile);
            let _ = std::fs::remove_file(data.join("mime").join("packages").join(MIME_PACKAGE));
            changed = true;
        }
        for entry in [menu, desktop] {
            if entry_of(&entry, dir) {
                let _ = std::fs::remove_file(&entry);
                changed = true;
            }
        }
        if changed {
            refresh_desktop();
        }
        Ok(())
    }

    pub fn path_hint(_dir: &Path) -> String {
        let bin = match bin_dir() {
            Ok(b) => b,
            Err(e) => return e,
        };
        let on_path = std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d == bin));
        if on_path {
            "Run: qb --version".into()
        } else {
            format!(
                "{} is not on PATH. Add it (for example in ~/.profile: export PATH=\"$HOME/.local/bin:$PATH\"), then run: qb --version",
                bin.display()
            )
        }
    }

    /// Shows `path` (a file or a folder) with the program the system uses for it.
    pub fn open(path: &Path) {
        let tool = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
        let _ = std::process::Command::new(tool).arg(path).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
    }

    pub fn remove_install_dir(dir: &Path) {
        if let Ok(me) = std::env::current_exe()
            && me.parent() == Some(dir)
        {
            let _ = std::fs::remove_file(&me);
        }
        let _ = std::fs::remove_dir(dir);
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn desktop_entries_quote_the_install_folder() {
            let dir = Path::new("/opt/my qb's");
            let entry = terminal_entry(dir);
            if cfg!(not(target_os = "macos")) {
                assert!(entry.contains("Exec=sh -c \"PATH='/opt/my qb'\\\\''s':\\\"\\$PATH\\\"; export PATH;"), "{entry}");
                assert!(compile_entry(dir).contains("\\\"\\$1\\\"; printf"));
                assert!(compile_entry(dir).ends_with("X-QBNex-Dir=/opt/my qb's\n"));
            }
            assert_eq!(quoted("a'b"), "'a'\\''b'");
        }

        #[test]
        fn an_entry_is_recognised_by_its_install_folder() {
            let root = std::env::temp_dir().join(format!("qbnex-setup-entry-{}", std::process::id()));
            std::fs::create_dir_all(&root).unwrap();
            let file = root.join("x.desktop");
            std::fs::write(&file, terminal_entry(Path::new("/opt/qbnex"))).unwrap();
            assert!(entry_of(&file, Path::new("/opt/qbnex")));
            assert!(!entry_of(&file, Path::new("/opt/qb")));
            std::fs::remove_dir_all(&root).unwrap();
        }
    }
}
