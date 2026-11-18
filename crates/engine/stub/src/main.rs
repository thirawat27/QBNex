//! `qb-stub`: the first bytes of a compiled QBNex program.
//!
//! The file is this launcher, the runner `qb-exec` packed with brotli, then the
//! program (layout: crate `qb-payload`). The launcher unpacks the runner once
//! into a cache folder, named after the runner build, and starts it there with
//! the program file in `QBNEX_PACKED_EXE`; the runner reads the program from that
//! file. Later starts find the runner in the cache and unpack nothing.
//!
//! Linux and macOS: the runner replaces the launcher (`exec`), so the program
//! keeps the process, its signals and its exit status. Windows: the runner is a
//! child process in a job that ends it with the launcher; the launcher passes on
//! its exit code, leaves Ctrl+C to it and gives it the right to come to the front.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

use qb_payload::PackedRunner;

/// Must match `qb_exec::payload::PACKED_EXE_VAR`.
const PACKED_EXE_VAR: &str = "QBNEX_PACKED_EXE";

/// Reports a launcher failure (a message box for a Windows GUI program, which has
/// no console to print to) and ends with status 2.
fn fail(gui: bool, msg: &str) -> ! {
    let text = format!("This QBNex program cannot start: {msg}");
    eprintln!("{text}");
    if gui {
        message_box(&text);
    }
    exit(2)
}

#[cfg(windows)]
fn message_box(text: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
    let w: Vec<u16> = text.encode_utf16().chain([0]).collect();
    let title: Vec<u16> = "QBNex".encode_utf16().chain([0]).collect();
    // SAFETY: two NUL-terminated UTF-16 strings; no owner window.
    unsafe { MessageBoxW(std::ptr::null_mut(), w.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR) };
}

#[cfg(not(windows))]
fn message_box(_: &str) {}

fn read_at(f: &mut File, at: u64, len: usize) -> std::io::Result<Vec<u8>> {
    let mut b = vec![0u8; len];
    f.seek(SeekFrom::Start(at))?;
    f.read_exact(&mut b)?;
    Ok(b)
}

/// The folder where unpacked runners are kept, per user.
fn cache_root() -> Option<PathBuf> {
    let var = |n: &str| std::env::var_os(n).map(PathBuf::from).filter(|p| p.is_absolute());
    if cfg!(windows) {
        var("LOCALAPPDATA").map(|d| d.join("QBNex").join("runners"))
    } else if cfg!(target_os = "macos") {
        var("HOME").map(|h| h.join("Library").join("Caches").join("QBNex").join("runners"))
    } else {
        var("XDG_CACHE_HOME").or_else(|| var("HOME").map(|h| h.join(".cache"))).map(|d| d.join("qbnex").join("runners"))
    }
}

/// A new folder of this process only, for when the cache cannot be written.
fn private_dir() -> Option<PathBuf> {
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let dir = std::env::temp_dir().join(format!("qbnex-runner-{}-{stamp:x}", std::process::id()));
    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut b = std::fs::DirBuilder::new();
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut b, 0o700);
    b.create(&dir).ok().map(|()| dir)
}

/// The runner, unpacked from `f` (filtered back, checked against its id and
/// given the launcher's Windows subsystem).
fn unpack(f: &mut File, r: &PackedRunner, subsystem: Option<u16>) -> Result<Vec<u8>, String> {
    let packed_len = usize::try_from(r.packed_len).map_err(|_| "damaged program file")?;
    let packed = read_at(f, r.at, packed_len).map_err(|e| format!("cannot read the program file: {e}"))?;
    let mut runner = Vec::with_capacity(usize::try_from(r.runner_len).unwrap_or(0));
    brotli_decompressor::BrotliDecompress(&mut &packed[..], &mut runner).map_err(|_| "damaged program file")?;
    r.filter.apply(&mut runner, false);
    if runner.len() as u64 != r.runner_len || qb_payload::runner_id(&runner) != r.id {
        return Err("damaged program file".into());
    }
    if let Some(s) = subsystem {
        qb_payload::set_pe_subsystem(&mut runner, s);
    }
    Ok(runner)
}

/// Writes `runner` as `dir/name`: through a file of this process, renamed into
/// place, so another program starting at the same moment never runs half a file.
fn store(dir: &Path, name: &str, runner: &[u8]) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(name);
    let tmp = dir.join(format!("{name}.{}.tmp", std::process::id()));
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut o, 0o755);
    let written = o.open(&tmp).and_then(|mut w| w.write_all(runner));
    if let Err(e) = written {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    match std::fs::rename(&tmp, &path) {
        Ok(()) => Ok(path),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            // another program put the same runner there first (Windows refuses to
            // replace a runner that is running)
            if std::fs::metadata(&path).is_ok_and(|m| m.len() == runner.len() as u64) { Ok(path) } else { Err(e) }
        }
    }
}

fn main() {
    let me = std::env::current_exe().unwrap_or_else(|e| fail(false, &format!("cannot find its own file: {e}")));
    let mut f = File::open(&me).unwrap_or_else(|e| fail(false, &format!("cannot open {}: {e}", me.display())));
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let head = read_at(&mut f, 0, len.min(4096) as usize).unwrap_or_default();
    let subsystem = qb_payload::pe_subsystem(&head);
    let gui = subsystem == Some(qb_payload::SUBSYSTEM_GUI);
    let tail_len = len.min(qb_payload::PROGRAM_TRAILER_MAX as u64);
    let tail = read_at(&mut f, len - tail_len, tail_len as usize).unwrap_or_default();
    let Some(span) = qb_payload::program_span(&tail, len) else { fail(gui, "it carries no program") };
    let front = span.start();
    let runner_tail = front.checked_sub(qb_payload::RUNNER_TRAILER as u64).and_then(|at| read_at(&mut f, at, qb_payload::RUNNER_TRAILER).ok());
    let Some(packed) = runner_tail.and_then(|t| qb_payload::packed_runner(&t, front)) else { fail(gui, "it carries no runner") };

    // one file per runner build and Windows subsystem (a window program's runner
    // is a GUI executable, so no console window opens next to it)
    let name = format!("qb-exec-{:016x}{}{}", packed.id, if gui { "-gui" } else { "" }, std::env::consts::EXE_SUFFIX);
    let cached = cache_root().map(|d| d.join(&name)).filter(|p| std::fs::metadata(p).is_ok_and(|m| m.len() == packed.runner_len));
    let mut private = None;
    let runner = match cached {
        Some(p) => p,
        None => {
            let bytes = unpack(&mut f, &packed, subsystem).unwrap_or_else(|e| fail(gui, &e));
            match cache_root().map(|d| store(&d, &name, &bytes)) {
                Some(Ok(p)) => p,
                // no writable cache: a folder of this run only
                _ => {
                    let dir = private_dir().unwrap_or_else(|| fail(gui, "cannot write the runner to a cache or temporary folder"));
                    let p = store(&dir, &name, &bytes).unwrap_or_else(|e| fail(gui, &format!("cannot write the runner: {e}")));
                    private = Some(dir);
                    p
                }
            }
        }
    };
    drop(f);
    launch(&runner, &me, gui, private.as_deref())
}

#[cfg(unix)]
fn launch(runner: &Path, me: &Path, gui: bool, _private: Option<&Path>) -> ! {
    use std::os::unix::process::CommandExt;
    let mut args = std::env::args_os();
    let arg0 = args.next().unwrap_or_else(|| me.as_os_str().to_owned());
    let e = Command::new(runner).arg0(arg0).args(args).env(PACKED_EXE_VAR, me).exec();
    fail(gui, &format!("cannot start {}: {e}", runner.display()))
}

#[cfg(windows)]
fn launch(runner: &Path, me: &Path, gui: bool, private: Option<&Path>) -> ! {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::{FALSE, TRUE};
    use windows_sys::core::BOOL;
    use windows_sys::Win32::System::Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT, SetConsoleCtrlHandler};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation, SetInformationJobObject,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{AllowSetForegroundWindow, MSG, PM_NOREMOVE, PeekMessageW};

    // Ctrl+C and Ctrl+Break are the program's: the launcher waits for its end. A
    // handler, not SetConsoleCtrlHandler(NULL): ignoring would be inherited.
    unsafe extern "system" fn keep_running(event: u32) -> BOOL {
        if event == CTRL_C_EVENT || event == CTRL_BREAK_EVENT { TRUE } else { FALSE }
    }
    // SAFETY: a handler with the documented signature.
    unsafe { SetConsoleCtrlHandler(Some(keep_running), TRUE) };

    let mut child = Command::new(runner)
        .args(std::env::args_os().skip(1))
        .env(PACKED_EXE_VAR, me)
        .spawn()
        .unwrap_or_else(|e| fail(gui, &format!("cannot start {}: {e}", runner.display())));
    // SAFETY: Win32 calls on a job created here and the child's own handle.
    unsafe {
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if !job.is_null() {
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            // the runner ends with the launcher (killed from the task manager); what the
            // program starts itself stays out of the job and lives on
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK;
            let size = std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32;
            if SetInformationJobObject(job, JobObjectExtendedLimitInformation, (&raw const info).cast(), size) != 0 {
                AssignProcessToJobObject(job, child.as_raw_handle());
            }
        }
        // started from Explorer, the launcher may bring a window to the front: the
        // runner opens the window
        AllowSetForegroundWindow(child.id());
        if gui {
            // a GUI process that reads no messages keeps the "starting" cursor for
            // seconds: one look at the empty queue ends it
            let mut msg: MSG = std::mem::zeroed();
            PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);
        }
    }
    let status = child.wait();
    if let Some(dir) = private {
        let _ = std::fs::remove_dir_all(dir);
    }
    match status {
        Ok(s) => exit(s.code().unwrap_or(1)),
        Err(e) => fail(gui, &format!("lost the program: {e}")),
    }
}
