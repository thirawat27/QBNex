//! Where QBNex is installed and how it is made reachable (PATH, links, uninstall entry).

pub use imp::*;

#[cfg(windows)]
mod imp {
    use std::os::windows::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::ptr::{null, null_mut};

    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_DWORD, REG_EXPAND_SZ, REG_OPTION_NON_VOLATILE, REG_SZ,
        RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE};

    const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\QBNex";
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    pub fn setup_name() -> &'static str {
        "qbnex-setup.exe"
    }

    pub fn compiler_name() -> &'static str {
        "qb.exe"
    }

    pub fn default_dir() -> Result<PathBuf, String> {
        let base = std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA is not set")?;
        Ok(PathBuf::from(base).join("Programs").join("QBNex"))
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
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
            let mut h: HKEY = null_mut();
            let p = wide(path);
            // SAFETY: valid NUL-terminated strings and out pointers.
            let r = unsafe {
                RegCreateKeyExW(HKEY_CURRENT_USER, p.as_ptr(), 0, null(), REG_OPTION_NON_VOLATILE, KEY_READ | KEY_WRITE, null(), &mut h, null_mut())
            };
            if r == ERROR_SUCCESS { Ok(Key(h)) } else { Err(format!("cannot open HKCU\\{path} (error {r})")) }
        }

        fn open(path: &str) -> Option<Key> {
            let mut h: HKEY = null_mut();
            let p = wide(path);
            // SAFETY: as above.
            let r = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, p.as_ptr(), 0, KEY_READ | KEY_WRITE, &mut h) };
            (r == ERROR_SUCCESS).then_some(Key(h))
        }

        /// A string value and whether it is REG_EXPAND_SZ.
        fn get_str(&self, name: &str) -> Option<(String, bool)> {
            let n = wide(name);
            let mut ty = 0u32;
            let mut len = 0u32;
            // SAFETY: size query with a null buffer.
            let r = unsafe { RegQueryValueExW(self.0, n.as_ptr(), null(), &mut ty, null_mut(), &mut len) };
            if r != ERROR_SUCCESS || (ty != REG_SZ && ty != REG_EXPAND_SZ) {
                return None;
            }
            let mut buf = vec![0u16; (len as usize).div_ceil(2) + 1];
            let mut bytes = (buf.len() * 2) as u32;
            // SAFETY: `buf` holds `bytes` bytes.
            let r = unsafe { RegQueryValueExW(self.0, n.as_ptr(), null(), &mut ty, buf.as_mut_ptr().cast(), &mut bytes) };
            if r != ERROR_SUCCESS {
                return None;
            }
            let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            Some((String::from_utf16_lossy(&buf[..end]), ty == REG_EXPAND_SZ))
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
        let env = wide("Environment");
        let mut result = 0usize;
        // SAFETY: broadcast with a NUL-terminated string; the timeout bounds the wait.
        unsafe {
            SendMessageTimeoutW(HWND_BROADCAST, WM_SETTINGCHANGE, 0, env.as_ptr() as isize, SMTO_ABORTIFHUNG, 3000, &mut result);
        }
    }

    fn edit_user_path(dir: &Path, add: bool) -> Result<(), String> {
        let key = Key::create("Environment")?;
        let (old, expand) = key.get_str("Path").unwrap_or((String::new(), true));
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

    pub fn register(dir: &Path, version: &str, size_kb: u64, add_path: bool) -> Result<(), String> {
        if add_path {
            edit_user_path(dir, true)?;
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

    pub fn unregister(dir: &Path) -> Result<(), String> {
        if Key::open("Environment").is_some() {
            edit_user_path(dir, false)?;
        }
        let k = wide(UNINSTALL_KEY);
        // SAFETY: NUL-terminated subkey under HKCU; a missing key is not an error here.
        unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, k.as_ptr()) };
        Ok(())
    }

    pub fn path_hint(_dir: &Path) -> String {
        "Open a new terminal and run: qb --version".into()
    }

    /// Removes the (now empty) install folder. A running program cannot delete its own
    /// file on Windows, so when the uninstaller runs from that folder a hidden `cmd`
    /// deletes it (and the folder, only if empty) after this process has exited.
    pub fn remove_install_dir(dir: &Path) {
        let me = std::env::current_exe().ok();
        let inside = me.as_ref().and_then(|m| m.parent()).is_some_and(|p| p == dir);
        if let (true, Some(me)) = (inside, me) {
            let cmd = format!("/c ping 127.0.0.1 -n 3 >nul & del /f /q \"{}\" & rmdir \"{}\"", me.display(), dir.display());
            let _ = std::process::Command::new("cmd").raw_arg(cmd).creation_flags(CREATE_NO_WINDOW).spawn();
        } else {
            let _ = std::fs::remove_dir(dir);
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::{Path, PathBuf};

    /// Commands linked into `~/.local/bin`.
    const COMMANDS: [&str; 2] = ["qb", "qbnex"];

    pub fn setup_name() -> &'static str {
        "qbnex-setup"
    }

    pub fn compiler_name() -> &'static str {
        "qb"
    }

    fn home() -> Result<PathBuf, String> {
        std::env::var_os("HOME").map(PathBuf::from).ok_or_else(|| "HOME is not set".to_string())
    }

    pub fn default_dir() -> Result<PathBuf, String> {
        let data = match std::env::var_os("XDG_DATA_HOME") {
            Some(d) if !d.is_empty() => PathBuf::from(d),
            _ => home()?.join(".local").join("share"),
        };
        Ok(data.join("qbnex"))
    }

    fn bin_dir() -> Result<PathBuf, String> {
        Ok(home()?.join(".local").join("bin"))
    }

    /// True when `link` is a symlink whose target lies in `dir` (or no longer exists).
    fn ours(link: &Path, dir: &Path) -> bool {
        match std::fs::read_link(link) {
            Ok(t) => t.starts_with(dir) || !t.exists(),
            Err(_) => false,
        }
    }

    pub fn register(dir: &Path, _version: &str, _size_kb: u64, add_path: bool) -> Result<(), String> {
        if !add_path {
            return Ok(());
        }
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

    pub fn unregister(dir: &Path) -> Result<(), String> {
        let bin = bin_dir()?;
        for name in COMMANDS {
            let link = bin.join(name);
            if ours(&link, dir) {
                let _ = std::fs::remove_file(&link);
            }
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

    pub fn remove_install_dir(dir: &Path) {
        if let Ok(me) = std::env::current_exe()
            && me.parent() == Some(dir)
        {
            let _ = std::fs::remove_file(&me);
        }
        let _ = std::fs::remove_dir(dir);
    }
}
