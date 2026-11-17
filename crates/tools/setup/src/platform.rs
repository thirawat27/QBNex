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

/// Something the user must know before installing into a folder.
#[derive(Clone, Debug, PartialEq)]
pub struct Caution {
    pub text: String,
    /// Install only after a yes (or `--yes`): the installed compiler would not work
    /// for the user as things are.
    pub ask: bool,
}

/// True when a terminal would run `path` as a command.
fn runnable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

/// The command a terminal runs: the first of `names` found in the first of `dirs`
/// that has one (within a folder `names` are tried in order, as PATHEXT orders them
/// on Windows).
fn first_on_path(dirs: &[PathBuf], names: &[String]) -> Option<PathBuf> {
    dirs.iter().find_map(|d| names.iter().map(|n| d.join(n)).find(|p| runnable(p)))
}

#[cfg(test)]
mod shared_tests {
    use super::*;

    #[test]
    fn the_first_folder_on_path_with_the_command_wins() {
        let root = std::env::temp_dir().join(format!("qbnex-setup-path-{}", std::process::id()));
        let (a, b, c) = (root.join("a"), root.join("b"), root.join("c"));
        for d in [&a, &b, &c] {
            std::fs::create_dir_all(d).unwrap();
        }
        let make = |p: &Path| {
            std::fs::write(p, "x").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
        };
        make(&b.join("qb.cmd"));
        make(&c.join("qb.exe"));
        make(&c.join("qb.cmd"));
        let names = ["qb.exe".to_string(), "qb.cmd".to_string()];
        let dirs = [a.clone(), b.clone(), c.clone()];
        assert_eq!(first_on_path(&dirs, &names), Some(b.join("qb.cmd")), "an earlier folder wins over a better name");
        assert_eq!(first_on_path(&dirs[2..], &names), Some(c.join("qb.exe")), "within a folder the names go in order");
        assert_eq!(first_on_path(&dirs[..1], &names), None);
        std::fs::remove_dir_all(&root).unwrap();
    }
}

#[cfg(windows)]
mod imp {
    use std::os::windows::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::ptr::{null, null_mut};

    use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, STGM_READ};
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    use windows::core::{HSTRING, Interface};
    use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, HANDLE};
    use windows_sys::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TokenElevation};
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    use windows_sys::Win32::System::Console::{FreeConsole, GetConsoleProcessList};
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE, REG_DWORD, REG_EXPAND_SZ, REG_OPTION_NON_VOLATILE, REG_SZ,
        RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE};

    use super::{Caution, Options, existing_ancestor, first_on_path, sandbox};

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

        /// A key below HKLM, for reading only.
        fn open_machine(path: &str) -> Option<Key> {
            let mut h: HKEY = null_mut();
            let p = wide(path);
            // SAFETY: as above.
            let r = unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, p.as_ptr(), 0, KEY_READ, &mut h) };
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

    /// `text` with each `%NAME%` that `lookup` knows replaced by its value, as Windows
    /// expands a REG_EXPAND_SZ value; an unknown name stays as written.
    fn expand_vars(text: &str, lookup: &dyn Fn(&str) -> Option<String>) -> String {
        let mut out = String::new();
        let mut rest = text;
        while let Some(start) = rest.find('%') {
            out.push_str(&rest[..start]);
            let after = &rest[start + 1..];
            match after.find('%').filter(|&end| end > 0).and_then(|end| Some((end, lookup(&after[..end])?))) {
                Some((end, value)) => {
                    out.push_str(&value);
                    rest = &after[end + 1..];
                }
                None => {
                    out.push('%');
                    rest = after;
                }
            }
        }
        out.push_str(rest);
        out
    }

    fn env_var(name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    /// A folder as Windows compares it: variables expanded, `/` read as `\`, `.` and
    /// `..` resolved, no trailing separator, letter case ignored.
    fn norm_dir(text: &str, lookup: &dyn Fn(&str) -> Option<String>) -> String {
        let s = expand_vars(text.trim().trim_matches('"'), lookup).replace('/', "\\");
        // only a full path: a relative PATH entry depends on the folder a program runs in
        let s = if Path::new(&s).is_absolute() { std::path::absolute(&s).map(|p| p.to_string_lossy().into_owned()).unwrap_or(s) } else { s };
        s.trim_end_matches('\\').to_lowercase()
    }

    /// True when the PATH entry, link folder or registry value `a` names the folder `b`,
    /// however either is spelled (`C:/x/`, `c:\X`, `%LOCALAPPDATA%\x`).
    fn same_dir(a: &str, b: &Path) -> bool {
        norm_dir(a, &env_var) == norm_dir(&b.to_string_lossy(), &env_var)
    }

    /// True when `dir` is the folder `base` or lies inside it.
    fn inside(dir: &Path, base: &Path) -> bool {
        let (d, b) = (norm_dir(&dir.to_string_lossy(), &env_var), norm_dir(&base.to_string_lossy(), &env_var));
        !b.is_empty() && (d == b || d.starts_with(&format!("{b}\\")))
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

    /// Text for the inside of double quotes on a cmd command line. cmd expands
    /// `%NAME%` even inside quotes, so each `%` is put outside them, escaped with a
    /// caret: `a%b` becomes `a"^%"b`. cmd removes the caret, and with the quotes
    /// around it the text stays one word.
    fn cmd_escape(text: &str) -> String {
        text.replace('%', "\"^%\"")
    }

    /// `text` as one double-quoted word on a cmd command line, never expanded.
    fn cmd_quoted(text: &str) -> String {
        format!("\"{}\"", cmd_escape(text))
    }

    fn prompt_link(folder: &Path, dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(folder).map_err(|e| format!("cannot create {}: {e}", folder.display()))?;
        let d = cmd_escape(&dir.to_string_lossy());
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
        // qb.exe started in a console of its own keeps it open itself (it shows the
        // result and waits for a key); through cmd, a `%` in the install folder's
        // name would be taken for a variable
        Key::create(&format!(r"{BAS_VERB_KEY}\command"))?.set_str("", &format!("\"{}\" \"%1\"", qb.display()), false)
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

    /// True when this process runs with administrator rights (an elevated token).
    fn elevated() -> bool {
        // the token of this process: a pseudo handle that needs no closing (GetCurrentProcessToken)
        let token = -4isize as HANDLE;
        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut len = 0u32;
        let size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;
        // SAFETY: the buffer is a TOKEN_ELEVATION of the size passed; `len` is a valid out pointer.
        let ok = unsafe { GetTokenInformation(token, TokenElevation, (&raw mut elevation).cast(), size, &mut len) };
        ok != 0 && elevation.TokenIsElevated != 0
    }

    /// Folders only administrators can write to.
    const SYSTEM_FOLDERS: [&str; 4] = ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432", "SystemRoot"];

    /// What an install into `dir` from an elevated setup must tell the user: PATH,
    /// the shortcuts and the uninstall entry go to the account that runs setup, and
    /// the compiler writes into the install folder as whoever runs it.
    pub fn caution(dir: &Path) -> Option<Caution> {
        if sandbox().is_some() || !elevated() {
            return None;
        }
        let profile = std::env::var_os("USERPROFILE").map(PathBuf::from);
        if profile.as_deref().is_some_and(|p| inside(dir, p)) {
            return None;
        }
        let account = std::env::var("USERNAME").unwrap_or_else(|_| "this account".into());
        let mut text = format!(
            "Setup runs as administrator ({account}). PATH, the Start menu shortcuts and the uninstall entry go to the account that runs setup ({account}), not to other accounts. The compiler writes target\\qb-temp and target\\qb-config.ini in {}, so that folder must stay writable for every account that runs qb.",
            dir.display()
        );
        let system = SYSTEM_FOLDERS.iter().filter_map(std::env::var_os).any(|base| inside(dir, Path::new(&base)));
        if system {
            text.push_str(" It is in a system folder that only administrators can write to: qb fails there unless it runs as administrator.");
        }
        Some(Caution { text, ask: system })
    }

    /// The folders of the PATH new terminals get: the system PATH, then the user's.
    fn terminal_path() -> Vec<PathBuf> {
        let value = |key: Option<Key>| key.and_then(|k| k.get_str("Path").ok().flatten()).map(|(v, _)| v).unwrap_or_default();
        let system = value(Key::open_machine(r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment"));
        let user = value(Key::open(ENVIRONMENT_KEY));
        format!("{system};{user}")
            .split(';')
            .map(|p| expand_vars(p.trim().trim_matches('"'), &env_var))
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
            .collect()
    }

    /// A warning when typing `qb` in a new terminal runs another program than the
    /// compiler in `dir`.
    pub fn shadowing(dir: &Path) -> Option<String> {
        if sandbox().is_some() {
            return None;
        }
        let extensions = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
        let names: Vec<String> = extensions.split(';').filter(|e| !e.is_empty()).map(|e| format!("qb{}", e.to_ascii_lowercase())).collect();
        let found = first_on_path(&terminal_path(), &names)?;
        let folder = found.parent()?;
        if same_dir(&folder.to_string_lossy(), dir) {
            return None;
        }
        Some(format!(
            "another qb runs first in new terminals: {} (the system PATH comes before the user PATH, and an earlier entry before a later one). Remove it, or move {} before {} in PATH.",
            found.display(),
            dir.display(),
            folder.display()
        ))
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
        // the same folder however it is spelled (--dir ., another letter case)
        let inside = me.as_ref().and_then(|m| m.parent()).is_some_and(|p| p == dir || (p.canonicalize().ok().is_some() && p.canonicalize().ok() == dir.canonicalize().ok()));
        if let (true, Some(me)) = (inside, me) {
            let cmd = format!(
                "/c ping 127.0.0.1 -n 3 >nul & del /f /q {} & rmdir {}",
                cmd_quoted(&me.to_string_lossy()),
                cmd_quoted(&dir.to_string_lossy())
            );
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

        #[test]
        fn a_percent_sign_on_a_cmd_line_is_put_outside_the_quotes() {
            assert_eq!(cmd_quoted(r"C:\Apps\QBNex"), r#""C:\Apps\QBNex""#);
            assert_eq!(cmd_quoted(r"C:\100%\%PATH%"), r#""C:\100"^%"\"^%"PATH"^%"""#);
            assert_eq!(cmd_escape("a%b"), r#"a"^%"b"#);
            assert_eq!(cmd_escape(""), "");
        }

        #[test]
        fn variables_are_expanded_and_unknown_ones_stay() {
            let lookup = |name: &str| (name == "LOCALAPPDATA").then(|| r"C:\Users\me\AppData\Local".to_string());
            assert_eq!(expand_vars(r"%LOCALAPPDATA%\Programs\QBNex", &lookup), r"C:\Users\me\AppData\Local\Programs\QBNex");
            assert_eq!(expand_vars(r"C:\%NOPE%\x", &lookup), r"C:\%NOPE%\x");
            assert_eq!(expand_vars("100%", &lookup), "100%");
            assert_eq!(expand_vars("%%LOCALAPPDATA%", &lookup), r"%C:\Users\me\AppData\Local");
        }

        #[test]
        fn a_folder_matches_however_it_is_spelled() {
            let dir = Path::new(r"C:\Users\x\QBNex");
            for spelled in [r"C:\Users\x\QBNex", r"c:\users\x\qbnex\", "C:/Users/x/QBNex", "C:/Users/x/QBNex/", r#""C:\Users\x\QBNex""#, r"C:\Users\x\.\QBNex", r"C:\Users\y\..\x\QBNex"] {
                assert!(same_dir(spelled, dir), "{spelled}");
            }
            assert!(!same_dir(r"C:\Users\x\QBNex2", dir));
            assert!(!same_dir("QBNex", dir), "a relative PATH entry is not the folder");
            assert!(inside(Path::new(r"C:\Users\x\QBNex\sub"), Path::new("c:/users/x")));
            assert!(inside(Path::new(r"C:\Users\x"), Path::new(r"C:\Users\x\")));
            assert!(!inside(Path::new(r"C:\Users\xy"), Path::new(r"C:\Users\x")));
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::{Path, PathBuf};

    use super::{Caution, Options, existing_ancestor, first_on_path, sandbox};

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
        if cfg!(target_os = "macos") {
            return true;
        }
        let set = |v: &str| std::env::var_os(v).is_some_and(|s| !s.is_empty());
        if !set("WAYLAND_DISPLAY") && !set("DISPLAY") {
            return false;
        }
        // the window toolkit aborts the installer when its keyboard library is missing
        #[cfg(all(feature = "gui", not(target_os = "macos")))]
        {
            let missing = if xkbcommon_dl::xkbcommon_option().is_none() {
                Some("libxkbcommon.so.0")
            } else if !set("WAYLAND_DISPLAY") && xkbcommon_dl::x11::xkbcommon_x11_option().is_none() {
                Some("libxkbcommon-x11.so.0")
            } else {
                None
            };
            if let Some(lib) = missing {
                eprintln!("qbnex-setup: no setup window: {lib} is not installed");
                return false;
            }
        }
        true
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

    /// `text` as the value of a desktop entry key of type string: a backslash, and
    /// the line breaks and tabs that would end or bend the line, take a backslash.
    fn desktop_string(text: &str) -> String {
        let mut out = String::new();
        for c in text.chars() {
            match c {
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\t' => out.push_str("\\t"),
                '\r' => out.push_str("\\r"),
                c => out.push(c),
            }
        }
        out
    }

    /// The text of a string value written by [`desktop_string`].
    fn desktop_unescape(value: &str) -> String {
        let mut out = String::new();
        let mut chars = value.chars();
        while let Some(c) = chars.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('s') => out.push(' '),
                Some(other) => out.push(other),
                None => out.push('\\'),
            }
        }
        out
    }

    /// A value of a desktop entry's `Exec` key that runs `script` with `sh -c`. The
    /// layers of the desktop entry rules, inside out: in a double-quoted argument `"`,
    /// `` ` ``, `$` and `\` take a backslash; a literal `%` is written `%%` (one `%`
    /// starts a field code such as `%f`); then the whole value is a string, where every
    /// backslash is doubled again (a literal `$` ends up as `\\$`, a backslash as `\\\\`).
    /// `arguments` is put after the script as it is (field codes such as ` %f`).
    fn exec_sh(script: &str, arguments: &str) -> String {
        let mut quoted = String::new();
        for c in script.chars() {
            if matches!(c, '"' | '`' | '$' | '\\') {
                quoted.push('\\');
            }
            if c == '%' {
                quoted.push('%');
            }
            quoted.push(c);
        }
        desktop_string(&format!("sh -c \"{quoted}\" sh{arguments}"))
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
            desktop_string(&dir.join(ICON).to_string_lossy()),
            desktop_string(&dir.to_string_lossy())
        )
    }

    fn compile_entry(dir: &Path) -> String {
        // the terminal stays open so the compiler's messages can be read; the program
        // is written next to the source, where the user looks for it
        let script = format!("{} \"$1\" -x; printf '\\nPress Enter to close.'; read x", quoted(&dir.join(compiler_name()).to_string_lossy()));
        format!(
            "[Desktop Entry]\nType=Application\nName=Compile with QBNex\nExec={}\nIcon={}\nTerminal=true\nNoDisplay=true\nMimeType=text/x-basic;\nX-QBNex-Dir={}\n",
            exec_sh(&script, " %f"),
            desktop_string(&dir.join(ICON).to_string_lossy()),
            desktop_string(&dir.to_string_lossy())
        )
    }

    const MIME_XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<mime-info xmlns=\"http://www.freedesktop.org/standards/shared-mime-info\">\n  <mime-type type=\"text/x-basic\">\n    <comment>BASIC source</comment>\n    <sub-class-of type=\"text/plain\"/>\n    <glob pattern=\"*.bas\"/>\n    <glob pattern=\"*.BAS\"/>\n  </mime-type>\n</mime-info>\n";

    /// The folder named by `key` in the text of `user-dirs.dirs`: a line such as
    /// `XDG_DESKTOP_DIR="$HOME/Desktop"` or `XDG_DESKTOP_DIR="/data/desk"`, its
    /// value in double quotes with the shell's backslash escapes. None when the key
    /// is not set or its value is neither form.
    fn user_dir(text: &str, key: &str, home: &Path) -> Option<PathBuf> {
        // the last line that sets it wins, as when the shell reads the file
        let value = text.lines().filter_map(|l| l.trim().strip_prefix(key)?.trim_start().strip_prefix('=')).next_back()?;
        let quoted = value.trim().strip_prefix('"')?.strip_suffix('"')?;
        let mut v = String::new();
        let mut chars = quoted.chars();
        while let Some(c) = chars.next() {
            v.push(if c == '\\' { chars.next().unwrap_or('\\') } else { c });
        }
        if v == "$HOME" {
            Some(home.to_path_buf())
        } else if let Some(rest) = v.strip_prefix("$HOME/") {
            Some(home.join(rest))
        } else if v.starts_with('/') {
            Some(PathBuf::from(v))
        } else {
            None
        }
    }

    /// The desktop folder: the one `user-dirs.dirs` names (it is often translated,
    /// `~/Schreibtisch`), else `~/Desktop`.
    fn desktop_dir(home: &Path) -> PathBuf {
        if cfg!(target_os = "macos") {
            return home.join("Desktop");
        }
        let config = match std::env::var_os("XDG_CONFIG_HOME") {
            Some(d) if !d.is_empty() && sandbox().is_none() => PathBuf::from(d),
            _ => home.join(".config"),
        };
        std::fs::read_to_string(config.join("user-dirs.dirs"))
            .ok()
            .and_then(|text| user_dir(&text, "XDG_DESKTOP_DIR", home))
            .unwrap_or_else(|| home.join("Desktop"))
    }

    /// Where the terminal entry goes in the applications menu and on the desktop.
    fn entry_paths() -> Result<(PathBuf, PathBuf), String> {
        let home = home()?;
        let desktop = desktop_dir(&home);
        if cfg!(target_os = "macos") {
            let name = format!("{TERMINAL_NAME}.command");
            Ok((home.join("Applications").join(&name), desktop.join(name)))
        } else {
            Ok((data_home()?.join("applications").join(MENU_ENTRY), desktop.join(MENU_ENTRY)))
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
            // A desktop that does not exist, or is the home folder itself (how
            // user-dirs.dirs turns the desktop off), gets no shortcut: making the
            // folder would only leave a stray Desktop in the home folder.
            let home = home()?;
            match desktop.parent() {
                Some(folder) if folder.is_dir() && folder != home.as_path() => write_executable(&desktop, &terminal_entry(dir))?,
                folder => eprintln!("note: no desktop folder ({}); no desktop shortcut was made", folder.unwrap_or(Path::new("?")).display()),
            }
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
        // the desktop entry holds the folder as a string value (escaped); the macOS
        // script, and entries of earlier setups, hold it as it is
        let mut values = text.lines().flat_map(|l| match l.strip_prefix("X-QBNex-Dir=") {
            Some(v) => vec![v.to_string(), desktop_unescape(v)],
            None => l.strip_prefix("# QBNex ").map(str::to_string).into_iter().collect(),
        });
        values.any(|v| v == d || canon(Path::new(&v)) == want)
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

    /// What an install into `dir` as root must tell the user: the links and menu
    /// entries go to root's own home, and the compiler writes into the install
    /// folder as whoever runs it. Asked for under /opt and /usr, where that is a
    /// system-wide folder other users cannot write to.
    pub fn caution(dir: &Path) -> Option<Caution> {
        // SAFETY: no arguments; reads the effective user id of this process.
        if sandbox().is_some() || unsafe { libc::geteuid() } != 0 {
            return None;
        }
        if !["/opt", "/usr"].iter().any(|base| dir.starts_with(base)) {
            return None;
        }
        let bin = bin_dir().map(|b| b.display().to_string()).unwrap_or_else(|_| "~/.local/bin".into());
        Some(Caution {
            text: format!(
                "Setup runs as root. The qb and qbnex links go to root's {bin} and the menu entries to root's account, not to other users. The compiler writes target/qb-temp and target/qb-config.ini in {}, so that folder must stay writable for every user who runs qb. Run setup as your own user to install QBNex for yourself.",
                dir.display()
            ),
            ask: true,
        })
    }

    /// A warning when typing `qb` in a terminal runs another program than the
    /// compiler in `dir`.
    pub fn shadowing(dir: &Path) -> Option<String> {
        if sandbox().is_some() {
            return None;
        }
        let path = std::env::var_os("PATH")?;
        let dirs: Vec<PathBuf> = std::env::split_paths(&path).filter(|d| !d.as_os_str().is_empty()).collect();
        let found = first_on_path(&dirs, &["qb".to_string()])?;
        if canon(&found) == canon(&dir.join(compiler_name())) {
            return None;
        }
        let bin = bin_dir().ok()?;
        let folder = found.parent()?;
        if folder == bin.as_path() {
            return Some(format!("{} is not QBNex's and was left as it is, so typing qb runs it. Remove it and run setup again.", found.display()));
        }
        if !dirs.contains(&bin) {
            return Some(format!(
                "typing qb runs {}: {}, which holds the qb link, is not on PATH. Put {} before {} in PATH.",
                found.display(),
                bin.display(),
                bin.display(),
                folder.display()
            ));
        }
        Some(format!(
            "another qb runs first: {} (its folder comes before {} in PATH). Remove it, or put {} before {} in PATH.",
            found.display(),
            bin.display(),
            bin.display(),
            folder.display()
        ))
    }

    /// Shows `path` (a file or a folder) with the program the system uses for it.
    pub fn open(path: &Path) {
        let tool = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
        let _ = std::process::Command::new(tool).arg(path).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
    }

    pub fn remove_install_dir(dir: &Path) {
        if let Ok(me) = std::env::current_exe()
            && me.parent().is_some_and(|p| p == dir || (p.canonicalize().ok().is_some() && p.canonicalize().ok() == dir.canonicalize().ok()))
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
                assert!(entry.contains(r#"Exec=sh -c "PATH='/opt/my qb'\\\\''s':\\"\\$PATH\\"; export PATH;"#), "{entry}");
                assert!(compile_entry(dir).contains(r#"\\"\\$1\\" -x; printf"#), "{}", compile_entry(dir));
                assert!(compile_entry(dir).ends_with("X-QBNex-Dir=/opt/my qb's\n"));
            }
            assert_eq!(quoted("a'b"), "'a'\\''b'");
        }

        /// Reads an `Exec` value the way a desktop does: the string escapes first,
        /// then field codes and the quoting of arguments. Panics on anything the
        /// desktop entry rules do not allow.
        fn decode_exec(value: &str) -> Vec<String> {
            let mut text = String::new();
            let mut chars = value.chars();
            while let Some(c) = chars.next() {
                if c != '\\' {
                    text.push(c);
                    continue;
                }
                match chars.next() {
                    Some('\\') => text.push('\\'),
                    Some('n') => text.push('\n'),
                    Some('t') => text.push('\t'),
                    Some('r') => text.push('\r'),
                    Some('s') => text.push(' '),
                    other => panic!("invalid string escape \\{other:?} in {value}"),
                }
            }
            let (mut args, mut arg, mut quoted, mut started) = (Vec::new(), String::new(), false, false);
            let mut chars = text.chars();
            while let Some(c) = chars.next() {
                match c {
                    '%' => match chars.next() {
                        Some('%') => arg.push('%'),
                        Some(code) => {
                            assert!(!quoted, "field code %{code} inside quotes: {value}");
                            arg.push('%');
                            arg.push(code);
                        }
                        None => panic!("a lone % ends {value}"),
                    },
                    '"' => quoted = !quoted,
                    '\\' if quoted => {
                        let next = chars.next().unwrap();
                        assert!(matches!(next, '"' | '`' | '$' | '\\'), "\\{next} inside quotes: {value}");
                        arg.push(next);
                    }
                    ' ' if !quoted => {
                        if started {
                            args.push(std::mem::take(&mut arg));
                        }
                        started = false;
                        continue;
                    }
                    c => {
                        assert!(!quoted || !matches!(c, '`' | '$'), "unescaped {c} inside quotes: {value}");
                        arg.push(c);
                    }
                }
                started = true;
            }
            assert!(!quoted, "unclosed quote: {value}");
            if started {
                args.push(arg);
            }
            args
        }

        #[test]
        fn the_exec_value_follows_the_desktop_entry_rules() {
            let dir = Path::new("/tmp/a %b $c \"d \\e f");
            let script = format!("{}; echo", with_path(dir));
            let exec = exec_sh(&script, " %f");
            assert_eq!(decode_exec(&exec), ["sh", "-c", script.as_str(), "sh", "%f"]);
            // the forms the rules give: %% for a percent sign, \\$ for a dollar sign,
            // four backslashes for one
            assert!(exec.contains(r#"'/tmp/a %%b \\$c \\"d \\\\e f'"#), "{exec}");
            if cfg!(not(target_os = "macos")) {
                let entry = compile_entry(dir);
                let line = |key: &str| entry.lines().find_map(|l| l.strip_prefix(key)).unwrap().to_string();
                let args = decode_exec(&line("Exec="));
                assert!(args[2].starts_with(&quoted(&dir.join(compiler_name()).to_string_lossy())), "{args:?}");
                assert_eq!(desktop_unescape(&line("X-QBNex-Dir=")), dir.to_string_lossy());
                assert_eq!(desktop_unescape(&line("Icon=")), dir.join(ICON).to_string_lossy());
            }
        }

        #[test]
        fn the_desktop_folder_comes_from_user_dirs() {
            let home = Path::new("/home/me");
            let text = "# XDG_DESKTOP_DIR=\"$HOME/Wrong\"\nXDG_DOWNLOAD_DIR=\"$HOME/Downloads\"\nXDG_DESKTOP_DIR=\"$HOME/Schreibtisch\"\n";
            assert_eq!(user_dir(text, "XDG_DESKTOP_DIR", home), Some(home.join("Schreibtisch")));
            assert_eq!(user_dir("XDG_DESKTOP_DIR=\"/data/my \\\"desk\\\"\"", "XDG_DESKTOP_DIR", home), Some(PathBuf::from("/data/my \"desk\"")));
            assert_eq!(user_dir("XDG_DESKTOP_DIR=\"$HOME\"", "XDG_DESKTOP_DIR", home), Some(home.to_path_buf()));
            assert_eq!(user_dir("XDG_DESKTOP_DIRS=\"$HOME/x\"", "XDG_DESKTOP_DIR", home), None);
            assert_eq!(user_dir("XDG_DESKTOP_DIR=Desktop", "XDG_DESKTOP_DIR", home), None);
            assert_eq!(user_dir("", "XDG_DESKTOP_DIR", home), None);
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
