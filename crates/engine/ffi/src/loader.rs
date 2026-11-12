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
/// way the legacy compiler did: `path/`, `name`, version (`name:1.0` or
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

/// Files the legacy compiler probed for `DECLARE DYNAMIC LIBRARY "spec"`, in order.
/// `source_dir` replaces a leading `./`.
#[must_use]
pub fn library_candidates(spec: &str, source_dir: Option<&Path>) -> Vec<PathBuf> {
    candidates_for(spec, source_dir, Os::NATIVE)
}

fn candidates_for(spec: &str, source_dir: Option<&Path>, os: Os) -> Vec<PathBuf> {
    let s = parse_spec(spec, source_dir, os);
    let (p, x, ver) = (&s.path, &s.name, &s.version);
    let mut out: Vec<String> = Vec::new();
    match os {
        Os::Windows => {
            let sys = std::env::var("SYSTEMROOT").unwrap_or_else(|_| "C:\\Windows".into());
            out.push(format!("{p}{x}.dll"));
            out.push(format!("{sys}\\System32\\{p}{x}.dll"));
            out.push(format!("{x}.dll"));
            out.push(format!("{sys}\\System32\\{x}.dll"));
        }
        Os::Linux | Os::MacOs => {
            let mac = os == Os::MacOs;
            for prefix in [p.as_str(), ""] {
                let dirs: Vec<String> = if prefix.is_empty() {
                    vec![String::new(), "/usr/lib64/".into(), "/usr/lib/".into()]
                } else {
                    vec![prefix.to_string(), format!("/usr/lib64/{prefix}"), format!("/usr/lib/{prefix}")]
                };
                for (k, d) in dirs.iter().enumerate() {
                    if mac && k != 1 {
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

/// The C header the legacy compiler would have used for `DECLARE LIBRARY
/// "spec"` (`name.h`, then `name.hpp`; next to the path, then bare, and in
/// `/usr/include` on Linux/macOS). `Some` means the declaration needs a C
/// compiler: report [`header_error`].
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

/// Compile error for a `DECLARE LIBRARY` that names a C header.
#[must_use]
pub fn header_error(spec: &str, header: &Path) -> String {
    format!(
        "DECLARE LIBRARY \"{spec}\" uses the C header {}, which needs a C compiler; \
         QBNex does not support C headers. Declare the functions \
         without a header, or use DECLARE DYNAMIC LIBRARY",
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
    /// exist, then the plain file name through the OS search path.
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
            if c.is_file() || (cfg!(target_vendor = "apple") && c.is_absolute()) {
                match Library::open_path(&text) {
                    Ok(l) => return Ok(l),
                    Err(e) => last = e,
                }
            }
            tried.push(text);
        }
        // Fall back to the loader's own search (System32, LD_LIBRARY_PATH, ...).
        for c in &cands {
            if c.parent().is_some_and(|p| !p.as_os_str().is_empty()) {
                continue;
            }
            let text = c.display().to_string();
            match Library::open_path(&text) {
                Ok(l) => return Ok(l),
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
/// order: the program itself, then the C runtime and system libraries the
/// legacy build linked.
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
/// [`default_modules`]. On Windows a POSIX name the C runtime only exports
/// with a leading underscore (`getpid` -> `_getpid`, as MinGW's `oldnames`
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

#[cfg(windows)]
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

    /// A system module by name, searched in System32 only (no application
    /// directory, current directory or PATH: no DLL planting).
    pub fn open_system(name: &str) -> Result<*mut c_void, String> {
        const LOAD_LIBRARY_SEARCH_SYSTEM32: u32 = 0x0000_0800;
        let w: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        // SAFETY: NUL-terminated wide string; the reserved handle argument is null.
        let h = unsafe { LoadLibraryExW(w.as_ptr(), core::ptr::null_mut(), LOAD_LIBRARY_SEARCH_SYSTEM32) };
        if h.is_null() {
            // SAFETY: plain thread-local error code.
            return Err(format!("Windows error {}", unsafe { GetLastError() }));
        }
        Ok(h)
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

#[cfg(unix)]
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

/// No dynamic libraries (wasm32 in a browser, qb-web): every DECLARE LIBRARY
/// load fails with an ordinary error and no symbol is found.
#[cfg(not(any(windows, unix)))]
mod sys {
    use std::ffi::c_void;

    pub fn open(_path: &str) -> Result<*mut c_void, String> {
        Err("dynamic libraries are not available on this platform".to_string())
    }

    pub fn open_system(path: &str) -> Result<*mut c_void, String> {
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

    #[test]
    fn candidate_order() {
        let w = s(&candidates_for("dir/foo", None, Os::Windows));
        assert_eq!(w[0], "dir\\foo.dll");
        assert!(w[1].ends_with("\\System32\\dir\\foo.dll"));
        assert_eq!(w[2], "foo.dll");
        let l = s(&candidates_for("z:1", None, Os::Linux));
        assert_eq!(&l[..4], ["libz.so.1", "libz.so", "/usr/lib64/libz.so.1", "/usr/lib64/libz.so"]);
        let m = s(&candidates_for("png", None, Os::MacOs));
        assert_eq!(m[0], "libpng.dylib");
        assert!(m.contains(&"/usr/lib/libpng.dylib".to_string()));
        assert!(!m.contains(&"/usr/lib64/libpng.dylib".to_string()));
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
        assert!(msg.contains("does not support C headers") && msg.contains("helper.h") && msg.contains("DECLARE DYNAMIC LIBRARY"));
        assert!(find_header(&format!("{}/nothing", dir.display()), None).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
