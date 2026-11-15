//! Finding and loading libraries and symbols at run time.

use std::ffi::c_void;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Target operating system family for path rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    Windows,
    Linux,
    MacOs,
}

impl Os {
    pub const NATIVE: Os = if cfg!(windows) {
        Os::Windows
    } else if cfg!(target_vendor = "apple") {
        Os::MacOs
    } else {
        Os::Linux
    };
}

/// A library name as written in `DECLARE [DYNAMIC] LIBRARY "spec"`, split the
/// way QB64 does: `path/`, `name`, version (`name:1.0` or
/// `name.1.0` gives `1.0`).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Spec {
    path: String,
    name: String,
    version: String,
}

fn is_uint(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn parse_spec(spec: &str, source_dir: Option<&Path>, os: Os) -> Spec {
    let mut x = spec.replace("\\\\", "\\");
    // Strip ".N" / ":N" version parts from the end.
    let mut v = String::new();
    loop {
        let bytes = x.as_bytes();
        let mut cut = None;
        for z in (0..bytes.len()).rev() {
            let c = bytes[z];
            if c == b'\\' || c == b'/' {
                break;
            }
            if c == b'.' || c == b':' {
                if is_uint(&x[z + 1..]) {
                    cut = Some((z, c == b':'));
                }
                break;
            }
        }
        match cut {
            Some((z, colon)) => {
                let part = x[z + 1..].to_string();
                v = if v.is_empty() { part } else { format!("{part}.{v}") };
                x.truncate(z);
                if colon {
                    break;
                }
            }
            None => break,
        }
    }
    let sep = if os == Os::Windows { '\\' } else { '/' };
    let x = if os == Os::Windows { x.replace('/', "\\") } else { x.replace('\\', "/") };
    let (mut path, name) = match x.rfind(['/', '\\']) {
        Some(z) => (x[..=z].to_string(), x[z + 1..].to_string()),
        None => (String::new(), x),
    };
    if path == "./" || path == ".\\" {
        path = source_dir.map(|d| d.display().to_string()).unwrap_or_default();
        if !path.is_empty() && !path.ends_with(sep) {
            path.push(sep);
        }
    }
    Spec { path, name, version: v }
}

/// Where a name without a path is looked for, besides the fixed system folders.
#[derive(Clone, Debug, Default)]
struct SearchEnv {
    /// The program's own folder ([`program_dir`]).
    program_dir: Option<String>,
    /// Windows: `%SYSTEMROOT%`.
    system_root: String,
    /// Absolute folders of `PATH` (Windows) or `DYLD_FALLBACK_LIBRARY_PATH` (macOS).
    path: Vec<String>,
}

static PROGRAM_DIR: OnceLock<std::sync::Mutex<Option<PathBuf>>> = OnceLock::new();

/// Sets the folder searched first for a library named without a path. A runner
/// whose program is not the executable itself calls this (`qb-exec prog.qir`:
/// the folder of `prog.qir`; the debugger: the folder of the `.bas` file).
/// A relative folder is taken relative to the current directory now (`""`, the
/// folder of `prog.qir` given without one, is the current directory), so a
/// later CHDIR does not move it.
pub fn set_program_dir(dir: Option<&Path>) {
    let dir = dir.map(|d| if d.as_os_str().is_empty() { Path::new(".") } else { d }).and_then(|d| std::path::absolute(d).ok());
    let m = PROGRAM_DIR.get_or_init(|| std::sync::Mutex::new(None));
    *m.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = dir;
}

/// The folder searched first for `DECLARE DYNAMIC LIBRARY "name"` without a
/// path: the one given to [`set_program_dir`], else the executable's folder.
#[must_use]
pub fn program_dir() -> Option<PathBuf> {
    if let Some(m) = PROGRAM_DIR.get() {
        if let Some(d) = m.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone() {
            return Some(d);
        }
    }
    std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf))
}

/// A folder of a search-path variable that does not depend on the current
/// directory: `.`, `bin` or an empty entry would make a library planted in
/// whatever folder the program is started from load first.
fn absolute_dir(d: &str, os: Os) -> bool {
    let b = d.as_bytes();
    match os {
        // C:\..., C:/..., \\server\share\..., \... (root of the current drive)
        Os::Windows => (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && matches!(b[2], b'\\' | b'/')) || matches!(b.first(), Some(b'\\' | b'/')),
        Os::Linux | Os::MacOs => b.first() == Some(&b'/'),
    }
}

impl SearchEnv {
    fn native() -> SearchEnv {
        let os = Os::NATIVE;
        let var = match os {
            Os::Windows => "PATH",
            Os::MacOs => "DYLD_FALLBACK_LIBRARY_PATH",
            Os::Linux => "",
        };
        let path = if var.is_empty() {
            Vec::new()
        } else {
            let sep = if os == Os::Windows { ';' } else { ':' };
            let v = std::env::var(var).unwrap_or_default();
            v.split(sep).map(|d| d.trim().trim_matches('"').to_string()).filter(|d| absolute_dir(d, os)).collect()
        };
        SearchEnv {
            program_dir: program_dir().map(|d| d.display().to_string()),
            system_root: std::env::var("SYSTEMROOT").unwrap_or_else(|_| "C:\\Windows".into()),
            path,
        }
    }
}

/// Files probed for `DECLARE DYNAMIC LIBRARY "spec"`, in order.
/// `source_dir` replaces a leading `./`.
///
/// A name with a path (`lib/name`, `C:\libs\name`) is looked for there first,
/// as in QB64. A name without a path, and the name part of a path that has no
/// such file, is looked for in the program's own folder ([`program_dir`]) and
/// the system folders, never in the current directory (QB64 looked there
/// first, so a same-named file in a download or shared folder the program was
/// started from was loaded instead of the intended library).
#[must_use]
pub fn library_candidates(spec: &str, source_dir: Option<&Path>) -> Vec<PathBuf> {
    candidates_for(spec, source_dir, Os::NATIVE, &SearchEnv::native())
}

fn candidates_for(spec: &str, source_dir: Option<&Path>, os: Os, env: &SearchEnv) -> Vec<PathBuf> {
    let s = parse_spec(spec, source_dir, os);
    let (p, x, ver) = (&s.path, &s.name, &s.version);
    let sep = if os == Os::Windows { '\\' } else { '/' };
    let pd = env.program_dir.as_ref().map(|d| {
        let mut d = d.clone();
        if !d.ends_with(['/', '\\']) {
            d.push(sep);
        }
        d
    });
    let mut out: Vec<String> = Vec::new();
    match os {
        Os::Windows => {
            let sys = env.system_root.trim_end_matches(['\\', '/']);
            if !p.is_empty() {
                out.push(format!("{p}{x}.dll"));
                out.push(format!("{sys}\\System32\\{p}{x}.dll"));
            }
            // the order LoadLibrary searches (program folder, System32, Windows,
            // PATH) without the current directory
            if let Some(pd) = &pd {
                out.push(format!("{pd}{x}.dll"));
            }
            out.push(format!("{sys}\\System32\\{x}.dll"));
            out.push(format!("{sys}\\{x}.dll"));
            for d in &env.path {
                out.push(format!("{}\\{x}.dll", d.trim_end_matches(['\\', '/'])));
            }
        }
        Os::Linux | Os::MacOs => {
            let mac = os == Os::MacOs;
            let mut blocks: Vec<Vec<String>> = Vec::new();
            if !p.is_empty() {
                blocks.push(vec![p.clone(), format!("/usr/lib64/{p}"), format!("/usr/lib/{p}")]);
            }
            let mut bare: Vec<String> = pd.iter().cloned().collect();
            bare.push("/usr/lib64/".into());
            bare.push("/usr/lib/".into());
            if mac {
                // what dlopen of a bare name found after /usr/lib (its search of the
                // current directory comes before these, so it is not used)
                bare.extend(env.path.iter().map(|d| format!("{}/", d.trim_end_matches('/'))));
                bare.push("/usr/local/lib/".into());
            }
            blocks.push(bare);
            for dirs in &blocks {
                for d in dirs {
                    // no .dylib in /usr/lib64 (QB64's order)
                    if mac && !d.starts_with("/usr/lib64/") {
                        if !ver.is_empty() {
                            out.push(format!("{d}lib{x}.{ver}.dylib"));
                        }
                        out.push(format!("{d}lib{x}.dylib"));
                    }
                    if !ver.is_empty() {
                        out.push(format!("{d}lib{x}.so.{ver}"));
                    }
                    out.push(format!("{d}lib{x}.so"));
                }
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    out.into_iter().filter(|c| seen.insert(c.clone())).map(PathBuf::from).collect()
}

/// Names handed to the system loader's own search when no candidate file
/// loads: Windows (restricted to the executable's folder and System32, so
/// API-set and side-by-side names still resolve) and Linux (`LD_LIBRARY_PATH`,
/// the `ld.so` cache: no current directory). None on macOS, where dlopen of
/// a bare name searches the current directory.
fn search_names(spec: &str, os: Os) -> Vec<String> {
    let s = parse_spec(spec, None, os);
    let (x, ver) = (&s.name, &s.version);
    match os {
        Os::Windows => vec![format!("{x}.dll")],
        Os::Linux => {
            let mut v = Vec::new();
            if !ver.is_empty() {
                v.push(format!("lib{x}.so.{ver}"));
            }
            v.push(format!("lib{x}.so"));
            v
        }
        Os::MacOs => Vec::new(),
    }
}

/// The header file QB64 would use for `DECLARE LIBRARY
/// "spec"` (`name.h`, then `name.hpp`; next to the path, then bare, and in
/// `/usr/include` on Linux/macOS). `Some` means the declaration names a
/// header file instead of a library: report [`header_error`].
#[must_use]
pub fn find_header(spec: &str, source_dir: Option<&Path>) -> Option<PathBuf> {
    header_candidates(spec, source_dir, Os::NATIVE).into_iter().find(|p| p.is_file())
}

fn header_candidates(spec: &str, source_dir: Option<&Path>, os: Os) -> Vec<PathBuf> {
    let s = parse_spec(spec, source_dir, os);
    let (p, x) = (&s.path, &s.name);
    let mut out = vec![format!("{p}{x}.h"), format!("{p}{x}.hpp")];
    if os != Os::Windows {
        out.push(format!("/usr/include/{p}{x}.h"));
        out.push(format!("/usr/include/{p}{x}.hpp"));
    }
    out.push(format!("{x}.h"));
    out.push(format!("{x}.hpp"));
    if os != Os::Windows {
        out.push(format!("/usr/include/{x}.h"));
        out.push(format!("/usr/include/{x}.hpp"));
    }
    out.into_iter().map(PathBuf::from).collect()
}

/// Compile error for a `DECLARE LIBRARY` that names a header file: the text the
/// BASIC compiler reports (compiler/qbnex.bas).
#[must_use]
pub fn header_error(spec: &str, header: &Path) -> String {
    format!(
        "DECLARE LIBRARY \"{spec}\" names the header file {}: header files are not \
         supported. Build the functions into a DLL (.so, .dylib) and declare that library",
        header.display()
    )
}

/// Why a library could not be loaded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryError {
    pub spec: String,
    pub tried: Vec<String>,
    pub os_error: String,
}

impl fmt::Display for LibraryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot load library \"{}\" (tried {})", self.spec, self.tried.join(", "))?;
        if !self.os_error.is_empty() {
            write!(f, ": {}", self.os_error)?;
        }
        Ok(())
    }
}

impl std::error::Error for LibraryError {}

/// A loaded library. Dropping it unloads it (`FreeLibrary` / `dlclose`).
#[derive(Debug)]
pub struct Library {
    handle: *mut c_void,
    path: String,
    owned: bool,
}

// SAFETY: module handles are process-wide and thread-safe in both APIs.
unsafe impl Send for Library {}
// SAFETY: as above; symbol lookup does not mutate the handle.
unsafe impl Sync for Library {}

impl Library {
    /// Loads a library file (or a name the OS loader searches for).
    ///
    /// # Errors
    /// The loader's message when the file cannot be loaded.
    pub fn open_path(path: &str) -> Result<Library, String> {
        let h = sys::open(path)?;
        Ok(Library { handle: h, path: path.to_string(), owned: true })
    }

    /// `DECLARE DYNAMIC LIBRARY "spec"`: tries [`library_candidates`] that
    /// exist, then the plain file name through the system loader's search
    /// (without the current directory).
    ///
    /// # Errors
    /// [`LibraryError`] with every path tried when nothing loads.
    pub fn open_dynamic(spec: &str, source_dir: Option<&Path>) -> Result<Library, LibraryError> {
        let cands = library_candidates(spec, source_dir);
        let mut tried = Vec::new();
        let mut last = String::new();
        for c in &cands {
            let text = c.display().to_string();
            // macOS 11+: system libraries (/usr/lib/libz.dylib, ...) exist only in
            // the dyld shared cache, not on disk, yet dlopen of that path works.
            // Every candidate has a folder: a name without one would be searched for.
            if c.parent().is_some_and(|p| !p.as_os_str().is_empty()) && (c.is_file() || (cfg!(target_vendor = "apple") && c.is_absolute())) {
                // A relative candidate (an explicit `lib/name`) is this file under the
                // current directory: LoadLibraryW would look for a relative path under
                // every folder of its search order instead.
                let load = if c.is_relative() { std::path::absolute(c).map_or_else(|_| text.clone(), |a| a.display().to_string()) } else { text.clone() };
                match sys::open(&load) {
                    Ok(h) => return Ok(Library { handle: h, path: text, owned: true }),
                    Err(e) => last = e,
                }
            }
            tried.push(text);
        }
        for name in search_names(spec, Os::NATIVE) {
            match sys::open_search(&name) {
                Ok(h) => return Ok(Library { handle: h, path: name, owned: true }),
                Err(e) => last = e,
            }
        }
        Err(LibraryError { spec: spec.to_string(), tried, os_error: last })
    }

    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// `GetProcAddress` / `dlsym`.
    #[must_use]
    pub fn symbol(&self, name: &str) -> Option<*const c_void> {
        sys::symbol(self.handle, name)
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        if self.owned {
            sys::close(self.handle);
        }
    }
}

/// Modules searched by a header-less `DECLARE LIBRARY` without a name, in
/// order: the program itself, then the system runtime and system libraries a
/// QB64 program links.
#[must_use]
pub fn default_modules() -> &'static [&'static str] {
    if cfg!(windows) {
        &[
            "ucrtbase.dll",
            "msvcrt.dll",
            "kernel32.dll",
            "user32.dll",
            "gdi32.dll",
            "advapi32.dll",
            "shell32.dll",
            "ole32.dll",
            "oleaut32.dll",
            "comdlg32.dll",
            "winmm.dll",
            "ws2_32.dll",
            "ntdll.dll",
        ]
    } else if cfg!(target_vendor = "apple") {
        &["libSystem.B.dylib"]
    } else {
        &["libc.so.6", "libm.so.6", "libdl.so.2", "libpthread.so.0"]
    }
}

struct Defaults(Vec<Library>);

fn defaults() -> &'static Defaults {
    static D: OnceLock<Defaults> = OnceLock::new();
    D.get_or_init(|| {
        let mut v = Vec::new();
        if let Some(h) = sys::program() {
            v.push(Library { handle: h, path: "<program>".into(), owned: false });
        }
        for m in default_modules() {
            // built-in system modules come from the system directory only, never
            // from a same-named DLL planted next to the program
            if let Ok(h) = sys::open_system(m) {
                v.push(Library { handle: h, path: (*m).to_string(), owned: true });
            }
        }
        Defaults(v)
    })
}

/// Header-less `DECLARE LIBRARY`: looks `name` up in the program and the
/// [`default_modules`]. On Windows a POSIX name the system runtime only exports
/// with a leading underscore (`getpid` -> `_getpid`, as the legacy `oldnames` aliases
/// mapped it) is found too.
#[must_use]
pub fn find_default_symbol(name: &str) -> Option<*const c_void> {
    if let Some(p) = sys::global_symbol(name) {
        return Some(p);
    }
    let d = defaults();
    for l in &d.0 {
        if let Some(p) = l.symbol(name) {
            return Some(p);
        }
    }
    if cfg!(windows) && !name.starts_with('_') {
        let alt = format!("_{name}");
        for l in &d.0 {
            if let Some(p) = l.symbol(&alt) {
                return Some(p);
            }
        }
    }
    None
}

// (only where a call thunk exists: elsewhere a library that loaded could not be called)
#[cfg(all(windows, any(target_arch = "x86_64", target_arch = "aarch64")))]
mod sys {
    use std::ffi::c_void;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryW(name: *const u16) -> *mut c_void;
        fn LoadLibraryExW(name: *const u16, file: *mut c_void, flags: u32) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
        fn FreeLibrary(module: *mut c_void) -> i32;
        fn GetModuleHandleW(name: *const u16) -> *mut c_void;
        fn GetLastError() -> u32;
    }

    pub fn open(path: &str) -> Result<*mut c_void, String> {
        let w: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
        // SAFETY: NUL-terminated wide string.
        let h = unsafe { LoadLibraryW(w.as_ptr()) };
        if h.is_null() {
            // SAFETY: plain thread-local error code.
            return Err(format!("Windows error {}", unsafe { GetLastError() }));
        }
        Ok(h)
    }

    const LOAD_LIBRARY_SEARCH_APPLICATION_DIR: u32 = 0x0000_0200;
    const LOAD_LIBRARY_SEARCH_SYSTEM32: u32 = 0x0000_0800;

    fn load_ex(name: &str, flags: u32) -> Result<*mut c_void, String> {
        let w: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        // SAFETY: NUL-terminated wide string; the reserved handle argument is null.
        let h = unsafe { LoadLibraryExW(w.as_ptr(), core::ptr::null_mut(), flags) };
        if h.is_null() {
            // SAFETY: plain thread-local error code.
            return Err(format!("Windows error {}", unsafe { GetLastError() }));
        }
        Ok(h)
    }

    /// A system module by name, searched in System32 only (no application
    /// directory, current directory or PATH: no DLL planting).
    pub fn open_system(name: &str) -> Result<*mut c_void, String> {
        load_ex(name, LOAD_LIBRARY_SEARCH_SYSTEM32)
    }

    /// A library by name through the loader's search, limited to the
    /// executable's folder and System32 (LoadLibraryW would also search the
    /// current directory).
    pub fn open_search(name: &str) -> Result<*mut c_void, String> {
        load_ex(name, LOAD_LIBRARY_SEARCH_APPLICATION_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32)
    }

    pub fn symbol(h: *mut c_void, name: &str) -> Option<*const c_void> {
        if name.as_bytes().contains(&0) {
            return None;
        }
        let c: Vec<u8> = name.bytes().chain(Some(0)).collect();
        // SAFETY: valid module handle, NUL-terminated name.
        let p = unsafe { GetProcAddress(h, c.as_ptr()) };
        (!p.is_null()).then_some(p.cast_const())
    }

    pub fn close(h: *mut c_void) {
        // SAFETY: handle came from LoadLibraryW.
        unsafe { FreeLibrary(h) };
    }

    pub fn program() -> Option<*mut c_void> {
        // SAFETY: NULL asks for the executable's own module.
        let h = unsafe { GetModuleHandleW(core::ptr::null()) };
        (!h.is_null()).then_some(h)
    }

    pub fn global_symbol(_name: &str) -> Option<*const c_void> {
        None
    }
}

#[cfg(all(unix, any(target_arch = "x86_64", target_arch = "aarch64")))]
mod sys {
    use std::ffi::{CStr, CString, c_char, c_int, c_void};

    #[cfg_attr(all(target_os = "linux", target_env = "gnu"), link(name = "dl"))]
    unsafe extern "C" {
        fn dlopen(filename: *const c_char, flag: c_int) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
        fn dlclose(handle: *mut c_void) -> c_int;
        fn dlerror() -> *mut c_char;
    }

    const RTLD_NOW: c_int = 2;
    #[cfg(target_vendor = "apple")]
    const RTLD_GLOBAL: c_int = 8;
    #[cfg(not(target_vendor = "apple"))]
    const RTLD_GLOBAL: c_int = 0x100;
    #[cfg(target_vendor = "apple")]
    const RTLD_DEFAULT: *mut c_void = -2isize as *mut c_void;
    #[cfg(not(target_vendor = "apple"))]
    const RTLD_DEFAULT: *mut c_void = core::ptr::null_mut();

    fn last_error() -> String {
        // SAFETY: dlerror returns NULL or a NUL-terminated message.
        let e = unsafe { dlerror() };
        if e.is_null() {
            String::new()
        } else {
            // SAFETY: see above.
            unsafe { CStr::from_ptr(e) }.to_string_lossy().into_owned()
        }
    }

    pub fn open(path: &str) -> Result<*mut c_void, String> {
        let c = CString::new(path).map_err(|_| "NUL in library name".to_string())?;
        // SAFETY: NUL-terminated path.
        let h = unsafe { dlopen(c.as_ptr(), RTLD_NOW | RTLD_GLOBAL) };
        if h.is_null() {
            return Err(last_error());
        }
        Ok(h)
    }

    /// A system library by name (the dynamic loader's own search).
    pub fn open_system(name: &str) -> Result<*mut c_void, String> {
        open(name)
    }

    /// A library by name through the dynamic loader's search (no slash in
    /// `name`: glibc and musl never look in the current directory then).
    pub fn open_search(name: &str) -> Result<*mut c_void, String> {
        open(name)
    }

    pub fn symbol(h: *mut c_void, name: &str) -> Option<*const c_void> {
        let c = CString::new(name).ok()?;
        // SAFETY: valid handle (or RTLD_DEFAULT), NUL-terminated name.
        let p = unsafe { dlsym(h, c.as_ptr()) };
        (!p.is_null()).then_some(p as *const c_void)
    }

    pub fn close(h: *mut c_void) {
        // SAFETY: handle came from dlopen.
        unsafe { dlclose(h) };
    }

    pub fn program() -> Option<*mut c_void> {
        // SAFETY: NULL opens the main program.
        let h = unsafe { dlopen(core::ptr::null(), RTLD_NOW) };
        (!h.is_null()).then_some(h)
    }

    pub fn global_symbol(name: &str) -> Option<*const c_void> {
        symbol(RTLD_DEFAULT, name)
    }
}

/// No dynamic libraries (wasm32 in a browser, qb-web; an architecture without a
/// call thunk, where a call would silently return 0): every DECLARE LIBRARY
/// load fails with an ordinary error and no symbol is found.
#[cfg(not(all(any(windows, unix), any(target_arch = "x86_64", target_arch = "aarch64"))))]
mod sys {
    use std::ffi::c_void;

    pub fn open(_path: &str) -> Result<*mut c_void, String> {
        Err("dynamic libraries are not available on this platform".to_string())
    }

    pub fn open_system(path: &str) -> Result<*mut c_void, String> {
        open(path)
    }

    pub fn open_search(path: &str) -> Result<*mut c_void, String> {
        open(path)
    }

    pub fn symbol(_h: *mut c_void, _name: &str) -> Option<*const c_void> {
        None
    }

    pub fn close(_h: *mut c_void) {}

    pub fn program() -> Option<*mut c_void> {
        None
    }

    pub fn global_symbol(_name: &str) -> Option<*const c_void> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[PathBuf]) -> Vec<String> {
        v.iter().map(|p| p.display().to_string()).collect()
    }

    #[test]
    fn spec_parsing_like_the_legacy_compiler() {
        let w = parse_spec("libs\\\\sub/foo:1.2", None, Os::Windows);
        assert_eq!((w.path.as_str(), w.name.as_str(), w.version.as_str()), ("libs\\sub\\", "foo", "1.2"));
        let l = parse_spec("SDL.1.2", None, Os::Linux);
        assert_eq!((l.name.as_str(), l.version.as_str()), ("SDL", "1.2"));
        let d = parse_spec("./mine", Some(Path::new("/src/prog")), Os::Linux);
        assert_eq!(d.path, "/src/prog/");
        let n = parse_spec("v2.dll.x", None, Os::Linux);
        assert_eq!((n.name.as_str(), n.version.as_str()), ("v2.dll.x", ""));
    }

    #[test]
    fn default_modules_load_as_system_modules() {
        // Windows: from System32 only (LoadLibraryExW + LOAD_LIBRARY_SEARCH_SYSTEM32)
        let first = default_modules()[0];
        let h = sys::open_system(first).expect("first default module");
        sys::close(h);
        assert!(find_default_symbol("strlen").is_some());
        if cfg!(windows) {
            assert!(sys::open_system("qb_no_such_module.dll").is_err());
        }
    }

    fn win_env() -> SearchEnv {
        SearchEnv { program_dir: Some("C:\\Games\\prog".into()), system_root: "C:\\Windows".into(), path: vec!["C:\\Tools\\bin\\".into()] }
    }

    fn unix_env() -> SearchEnv {
        SearchEnv { program_dir: Some("/opt/prog".into()), system_root: String::new(), path: Vec::new() }
    }

    #[test]
    fn candidate_order_windows() {
        let w = s(&candidates_for("dir/foo", None, Os::Windows, &win_env()));
        assert_eq!(
            w,
            [
                "dir\\foo.dll",
                "C:\\Windows\\System32\\dir\\foo.dll",
                "C:\\Games\\prog\\foo.dll",
                "C:\\Windows\\System32\\foo.dll",
                "C:\\Windows\\foo.dll",
                "C:\\Tools\\bin\\foo.dll",
            ]
        );
        // a name without a path: the program's folder first, then the system folders
        let b = s(&candidates_for("user32", None, Os::Windows, &win_env()));
        assert_eq!(b, ["C:\\Games\\prog\\user32.dll", "C:\\Windows\\System32\\user32.dll", "C:\\Windows\\user32.dll", "C:\\Tools\\bin\\user32.dll"]);
        assert_eq!(search_names("user32", Os::Windows), ["user32.dll"]);
    }

    #[test]
    fn candidate_order_unix() {
        let l = s(&candidates_for("z:1", None, Os::Linux, &unix_env()));
        assert_eq!(l, ["/opt/prog/libz.so.1", "/opt/prog/libz.so", "/usr/lib64/libz.so.1", "/usr/lib64/libz.so", "/usr/lib/libz.so.1", "/usr/lib/libz.so"]);
        assert_eq!(search_names("z:1", Os::Linux), ["libz.so.1", "libz.so"]);
        let p = s(&candidates_for("sub/foo", None, Os::Linux, &unix_env()));
        assert_eq!(&p[..4], ["sub/libfoo.so", "/usr/lib64/sub/libfoo.so", "/usr/lib/sub/libfoo.so", "/opt/prog/libfoo.so"]);
        let mut menv = unix_env();
        menv.path = vec!["/Users/me/lib".into()];
        let m = s(&candidates_for("png", None, Os::MacOs, &menv));
        assert_eq!(m[0], "/opt/prog/libpng.dylib");
        assert!(m.contains(&"/usr/lib/libpng.dylib".to_string()));
        assert!(!m.contains(&"/usr/lib64/libpng.dylib".to_string()));
        let at = |c: &str| m.iter().position(|x| x == c).unwrap();
        assert!(at("/usr/lib/libpng.dylib") < at("/Users/me/lib/libpng.dylib"));
        assert!(at("/Users/me/lib/libpng.dylib") < at("/usr/local/lib/libpng.dylib"));
        // dlopen of a bare name searches the current directory on macOS: never used
        assert!(search_names("png", Os::MacOs).is_empty());
    }

    #[test]
    fn no_candidate_depends_on_the_current_directory() {
        for os in [Os::Windows, Os::Linux, Os::MacOs] {
            let env = if os == Os::Windows { win_env() } else { unix_env() };
            for spec in ["foo", "foo:2", "foo.1.2"] {
                for c in s(&candidates_for(spec, None, os, &env)) {
                    assert!(absolute_dir(&c, os), "{os:?} {spec}: {c}");
                }
            }
            // without a known program folder the bare name is still not tried as is
            let none = SearchEnv { program_dir: None, ..env.clone() };
            for c in s(&candidates_for("foo", None, os, &none)) {
                assert!(absolute_dir(&c, os), "{os:?}: {c}");
            }
        }
        // ./name is the source folder (the compiler passes it as an absolute path)
        let d = s(&candidates_for("./mine", Some(Path::new("/src/prog")), Os::Linux, &unix_env()));
        assert_eq!(d[0], "/src/prog/libmine.so");
    }

    #[test]
    fn search_path_entries_relative_to_the_current_directory_are_skipped() {
        assert!(absolute_dir("C:\\Tools", Os::Windows));
        assert!(absolute_dir("c:/tools", Os::Windows));
        assert!(absolute_dir("\\\\server\\share", Os::Windows));
        for rel in ["", ".", "bin", "C:tools", "..\\x"] {
            assert!(!absolute_dir(rel, Os::Windows), "{rel}");
        }
        assert!(absolute_dir("/usr/local/lib", Os::MacOs));
        assert!(!absolute_dir("lib", Os::MacOs));
        assert!(!absolute_dir("", Os::Linux));
    }

    #[test]
    fn program_dir_override() {
        // (the only test that sets it: tests run in parallel in one process)
        let here = std::env::temp_dir();
        set_program_dir(Some(&here));
        assert_eq!(program_dir().as_deref(), Some(here.as_path()));
        set_program_dir(Some(Path::new("")));
        assert_eq!(program_dir(), std::env::current_dir().ok());
        set_program_dir(None);
        let exe = std::env::current_exe().unwrap();
        assert_eq!(program_dir().as_deref(), exe.parent());
    }

    #[test]
    fn header_detection_and_message() {
        let h = s(&header_candidates("inc/foo", None, Os::Linux));
        assert_eq!(&h[..3], ["inc/foo.h", "inc/foo.hpp", "/usr/include/inc/foo.h"]);
        let dir = std::env::temp_dir().join(format!("qb_ffi_hdr_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("helper.h"), "int helper(void);").unwrap();
        let spec = format!("{}/helper", dir.display());
        let found = find_header(&spec, None).expect("header found");
        assert!(found.ends_with("helper.h"));
        let msg = header_error("helper", &found);
        assert!(msg.contains("header files are not supported") && msg.contains("helper.h") && msg.contains("declare that library"), "{msg}");
        assert!(find_header(&format!("{}/nothing", dir.display()), None).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
