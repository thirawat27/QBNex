//! QBNex conformance runner.
//!
//! Every `cases/**/<name>.bas` is compiled with `qb`, executed, and its stdout is
//! compared with `<name>.out` (line endings normalised to `\n`). An optional
//! `<name>.in` is piped to stdin; `<name>.input` is an input script (keys/mouse)
//! replayed by the Rust runtime in headless mode. `--bless` rewrites the `.out` files from the
//! current compiler, which is how golden output is captured from a trusted build.
//!
//! A `<name>.xfail` file marks a known defect of the legacy implementation; its
//! first line is the reason. Such a case is expected to fail. If it starts
//! passing the run fails with `XPASS`, so the marker gets removed when the bug
//! is fixed. `--bless` never writes golden output for xfail cases.
//!
//! A `<name>.deviation` file marks an intentional difference from the legacy
//! runtime (a fixed legacy defect). Its `.out` is written by hand from QBasic
//! semantics, so `--bless` leaves it alone.
//!
//! Graphics: every program runs with `QBNEX_FB_CAPTURE` pointing at a scratch
//! file. The runtime appends one record per `_DISPLAY` call and one at program
//! end (graphics pages only); see `crates/runtime/core/src/gfx/capture.rs`. Each
//! record is summarised as one line (size, depth, mode, FNV-1a 64 hash of the
//! whole record). When records exist they are compared with `<name>.fb`, so the
//! golden framebuffer is pixel- and palette-exact. `--fb-dump <dir>` keeps the
//! raw captures (`<tag>.fbraw`) for debugging a mismatch.
//!
//! OpenGL (`SUB _GL`): on a host without an OpenGL context the runner reports
//! `qb-exec: SUB _GL is not run: no OpenGL context` on stderr; such a case is
//! reported as `[skip]` (not passed, not failed). `QBNEX_REQUIRE_GL=1` makes
//! it a failure instead, for hosts that must have OpenGL.
//!
//! The runner only uses `std` so it builds offline and on every CI host.

use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone)]
struct Options {
    qb: PathBuf,
    cases: PathBuf,
    /// `--work <dir>` names the parent; `main` replaces it with a fresh
    /// subdirectory, the only thing the runner creates and removes.
    work: PathBuf,
    bless: bool,
    filter: Option<String>,
    timeout: Duration,
    verbose: bool,
    fb_dump: Option<PathBuf>,
    /// Extra compiler flags, e.g. `--qb-arg --ir` for the R11 QIR backend.
    qb_args: Vec<String>,
    /// Differential mode: compile every case a second time with these flags
    /// (e.g. `--diff-arg --ir`) and compare the two runs with each other.
    diff_args: Option<Vec<String>>,
}

enum Outcome {
    Pass,
    Blessed,
    Fail(String),
    /// the host lacks something the case needs (OpenGL)
    Skip(String),
}

/// What qb-exec prints when SUB _GL cannot run (crates/engine/exec/src/vm.rs, gl_frame).
const NO_GL: &str = "SUB _GL is not run: no OpenGL context";
/// `run_program` error prefix for a run that hit [`NO_GL`].
const SKIP: &str = "\u{1}skip:";

fn require_gl() -> bool {
    env::var("QBNEX_REQUIRE_GL").is_ok_and(|v| v == "1")
}

fn usage() -> &'static str {
    "usage: conformance [--qb <path>] [--cases <dir>] [--work <dir>] [--filter <text>] \
     [--timeout <secs>] [--qb-arg <flag>]... [--diff-arg <flag>]... [--bless] [--verbose] [--fb-dump <dir>]"
}

fn repo_root() -> PathBuf {
    // tests/conformance/ lives two levels under the repository root.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn default_qb(root: &Path) -> PathBuf {
    if cfg!(windows) {
        root.join("qb.exe")
    } else {
        root.join("qb")
    }
}

fn parse_args() -> Result<Options, String> {
    let root = repo_root();
    let mut opts = Options {
        qb: default_qb(&root),
        cases: root.join("tests").join("conformance").join("cases"),
        // The run uses a fresh subdirectory of this (see `fresh_dir`).
        work: env::temp_dir(),
        bless: false,
        filter: None,
        timeout: Duration::from_secs(30),
        verbose: false,
        fb_dump: None,
        qb_args: Vec::new(),
        diff_args: None,
    };
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "--qb" => opts.qb = PathBuf::from(value("--qb")?),
            "--cases" => opts.cases = PathBuf::from(value("--cases")?),
            "--work" => opts.work = PathBuf::from(value("--work")?),
            "--filter" => opts.filter = Some(value("--filter")?),
            "--timeout" => {
                let secs: u64 = value("--timeout")?
                    .parse()
                    .map_err(|_| "--timeout expects whole seconds".to_string())?;
                opts.timeout = Duration::from_secs(secs);
            }
            "--fb-dump" => opts.fb_dump = Some(PathBuf::from(value("--fb-dump")?)),
            "--qb-arg" => opts.qb_args.push(value("--qb-arg")?),
            "--diff-arg" => opts.diff_args.get_or_insert_with(Vec::new).push(value("--diff-arg")?),
            "--bless" => opts.bless = true,
            "--verbose" | "-v" => opts.verbose = true,
            "--help" | "-h" => return Err(usage().to_string()),
            other => return Err(format!("unknown argument `{other}`\n{}", usage())),
        }
    }
    // A bare `--qb qb.exe` must mean the file in the current directory. Windows
    // process creation does not search the current directory when
    // NoDefaultCurrentDirectoryInExePath is set, so it would silently pick another
    // qb.exe from PATH (e.g. an installed QBNex release).
    if opts.qb.is_relative()
        && let Ok(cwd) = env::current_dir() {
            opts.qb = cwd.join(&opts.qb);
        }
    if !opts.qb.exists() {
        return Err(format!("qb compiler not found: {}", opts.qb.display()));
    }
    Ok(opts)
}

fn collect_cases(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_cases(&path, out)?;
        } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("bas")) {
            out.push(path);
        }
    }
    Ok(())
}

fn normalise(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
            i += 1;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

/// Exit code, stdout, stderr of a finished child.
type RunOutput = (Option<i32>, Vec<u8>, Vec<u8>);

/// Runs `cmd` with the given stdin, enforcing a timeout. Returns (exit code, stdout, stderr).
fn run_with_timeout(
    mut cmd: Command,
    stdin: Option<Vec<u8>>,
    timeout: Duration,
) -> Result<RunOutput, String> {
    cmd.stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("spawn failed: {e}"))?;

    let stdin_thread = match (stdin, child.stdin.take()) {
        (Some(data), Some(mut pipe)) => Some(thread::spawn(move || {
            let _ = pipe.write_all(&data);
        })),
        _ => None,
    };
    let mut stdout_pipe = child.stdout.take().ok_or("no stdout pipe")?;
    let mut stderr_pipe = child.stderr.take().ok_or("no stderr pipe")?;
    // Readers report over a channel (not `join`) so the wait for them can be bounded.
    let (out_tx, out_rx) = mpsc::channel::<(usize, Vec<u8>)>();
    let err_tx = out_tx.clone();
    thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout_pipe.read_to_end(&mut buf);
        let _ = out_tx.send((0, buf));
    });
    thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr_pipe.read_to_end(&mut buf);
        let _ = err_tx.send((1, buf));
    });

    let start = Instant::now();
    let status = loop {
        match child.try_wait().map_err(|e| format!("wait failed: {e}"))? {
            Some(status) => break status,
            None if start.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("timed out after {}s", timeout.as_secs()));
            }
            None => thread::sleep(Duration::from_millis(20)),
        }
    };
    // Not joined: a process the child started may still hold its stdin.
    drop(stdin_thread);
    // The pipes close when the child exits, unless a process it started keeps
    // them open: wait at most the rest of the timeout (2 s at least), then give
    // up on the reader threads instead of hanging the runner.
    let deadline = Instant::now() + timeout.saturating_sub(start.elapsed()).max(Duration::from_secs(2));
    let mut outs: [Option<Vec<u8>>; 2] = [None, None];
    while outs.iter().any(Option::is_none) {
        match out_rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok((i, buf)) => outs[i] = Some(buf),
            Err(_) => return Err("the program exited but its output pipes stayed open (a process it started holds them)".to_string()),
        }
    }
    let [stdout, stderr] = outs;
    Ok((status.code(), stdout.unwrap_or_default(), stderr.unwrap_or_default()))
}

fn first_difference(expected: &[u8], actual: &[u8]) -> String {
    let exp: Vec<&[u8]> = expected.split(|b| *b == b'\n').collect();
    let act: Vec<&[u8]> = actual.split(|b| *b == b'\n').collect();
    for i in 0..exp.len().max(act.len()) {
        let e = exp.get(i).copied();
        let a = act.get(i).copied();
        if e != a {
            let show = |l: Option<&[u8]>| match l {
                Some(bytes) => format!("{:?}", String::from_utf8_lossy(bytes)),
                None => "<missing>".to_string(),
            };
            return format!(
                "line {}:\n      expected {}\n      actual   {}",
                i + 1,
                show(e),
                show(a)
            );
        }
    }
    "outputs differ".to_string()
}

/// Compiles `case` with `qb_args` and runs it: (normalised stdout + exit marker, frame summary).
fn run_program(opts: &Options, case: &Path, qb_args: &[String]) -> Result<(Vec<u8>, Vec<u8>), String> {
    let stem = case.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let rel = case.strip_prefix(&opts.cases).unwrap_or(case);
    let tag: String = rel
        .with_extension("")
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let exe = opts
        .work
        .join(if cfg!(windows) { format!("{tag}.exe") } else { tag.clone() });
    let _ = fs::remove_file(&exe);

    // Compile from the repository root: qb resolves internal/ relative to itself.
    let mut compile = Command::new(&opts.qb);
    // Runtime errors print to stderr and fail instead of opening a modal dialog.
    compile.env("QBNEX_NO_ERROR_DIALOG", "1");
    compile
        .arg(case)
        .args(qb_args)
        .arg("-o")
        .arg(&exe)
        .arg("-q")
        .arg("-m")
        .current_dir(repo_root());
    let compile_out = match run_with_timeout(compile, None, Duration::from_secs(600)) {
        Ok(r) => r,
        Err(e) => return Err(format!("compile: {e}")),
    };
    if compile_out.0 != Some(0) || !exe.exists() {
        return Err(format!(
            "compile failed (exit {:?})\n{}",
            compile_out.0,
            String::from_utf8_lossy(&compile_out.1)
        ));
    }

    let stdin = fs::read(case.with_extension("in")).ok();
    // Programs run inside the scratch dir so file-I/O cases never touch the repo.
    let capture = opts.work.join(format!("{tag}.fbraw"));
    let _ = fs::remove_file(&capture);
    let mut run = Command::new(&exe);
    run.current_dir(&opts.work).env("QBNEX_FB_CAPTURE", &capture).env("QBNEX_NO_ERROR_DIALOG", "1");
    // GUI programs run without a window; `<name>.input` is replayed as keyboard/mouse
    // events (QBNEX_INPUT_SCRIPT, see crates/runtime/core/src/script.rs). The legacy
    // runtime ignores both variables.
    run.env("QBNEX_HEADLESS", "1");
    if let Ok(script) = case.with_extension("input").canonicalize() {
        run.env("QBNEX_INPUT_SCRIPT", script);
    }
    let (code, stdout, stderr) = match run_with_timeout(run, stdin, opts.timeout) {
        Ok(r) => r,
        Err(e) => return Err(format!("run: {e}")),
    };
    let _ = fs::remove_file(&exe);
    let raw = fs::read(&capture).unwrap_or_default();
    let _ = fs::remove_file(&capture);
    if let Some(dir) = &opts.fb_dump
        && !raw.is_empty() {
            let _ = fs::create_dir_all(dir);
            let _ = fs::write(dir.join(format!("{tag}.fbraw")), &raw);
        }
    let frames = match summarise_frames(&raw) {
        Ok(f) => f,
        Err(e) => return Err(format!("framebuffer capture: {e}")),
    };
    let mut actual = normalise(&stdout);
    if code != Some(0) {
        actual.extend_from_slice(format!("[exit {code:?}]\n").as_bytes());
    }
    if opts.verbose && !stderr.is_empty() {
        eprintln!("    stderr({stem}): {}", String::from_utf8_lossy(&stderr));
    }
    let err_text = String::from_utf8_lossy(&stderr);
    if let Some(line) = err_text.lines().find(|l| l.contains(NO_GL)) {
        return Err(if require_gl() { line.to_string() } else { format!("{SKIP}{line}") });
    }
    Ok((actual, frames))
}

/// Differential check: the same case under `qb_args` and `diff_args` must behave the same.
fn diff_case(opts: &Options, case: &Path, diff_args: &[String]) -> Outcome {
    let a = match run_program(opts, case, &opts.qb_args) {
        Ok(r) => r,
        Err(e) if e.starts_with(SKIP) => return Outcome::Skip(e[SKIP.len()..].to_string()),
        Err(e) => return Outcome::Fail(format!("reference: {e}")),
    };
    let b = match run_program(opts, case, diff_args) {
        Ok(r) => r,
        Err(e) if e.starts_with(SKIP) => return Outcome::Skip(e[SKIP.len()..].to_string()),
        Err(e) => return Outcome::Fail(format!("candidate: {e}")),
    };
    if a.0 != b.0 {
        return Outcome::Fail(format!("output differs, {}", first_difference(&a.0, &b.0)));
    }
    if a.1 != b.1 {
        return Outcome::Fail(format!("framebuffer differs, {}", first_difference(&a.1, &b.1)));
    }
    Outcome::Pass
}

fn run_case(opts: &Options, case: &Path) -> Outcome {
    if let Some(d) = &opts.diff_args {
        return diff_case(opts, case, d);
    }
    let (actual, frames) = match run_program(opts, case, &opts.qb_args) {
        Ok(r) => r,
        Err(e) if e.starts_with(SKIP) => return Outcome::Skip(e[SKIP.len()..].to_string()),
        Err(e) => return Outcome::Fail(e),
    };

    let expected_path = case.with_extension("out");
    let fb_path = case.with_extension("fb");
    if opts.bless {
        let fb_result = if frames.is_empty() {
            match fs::remove_file(&fb_path) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
                _ => Ok(()),
            }
        } else {
            fs::write(&fb_path, &frames)
        };
        return match fs::write(&expected_path, &actual).and(fb_result) {
            Ok(()) => Outcome::Blessed,
            Err(e) => Outcome::Fail(format!("bless write failed: {e}")),
        };
    }
    match fs::read(&expected_path) {
        Ok(expected) => {
            let expected = normalise(&expected);
            if expected != actual {
                return Outcome::Fail(first_difference(&expected, &actual));
            }
        }
        Err(_) => {
            return Outcome::Fail(format!(
                "missing golden file {} (run with --bless on a trusted build)",
                expected_path.display()
            ))
        }
    }
    let expected_frames = fs::read(&fb_path).map(|b| normalise(&b)).unwrap_or_default();
    if expected_frames == frames {
        Outcome::Pass
    } else if !fb_path.exists() {
        // No frame golden: the case only checks stdout. Cases whose frames
        // the legacy oracle cannot produce (R8 input scripts) have none.
        Outcome::Pass
    } else if expected_frames.is_empty() {
        Outcome::Fail(format!(
            "program captured framebuffer output but {} is missing (run with --bless on a trusted build)",
            fb_path.display()
        ))
    } else {
        Outcome::Fail(format!(
            "framebuffer differs, {}",
            first_difference(&expected_frames, &frames)
        ))
    }
}

/// FNV-1a, 64-bit.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Turns a raw capture (records written by the runtime, see the module docs)
/// into one text line per frame.
fn summarise_frames(raw: &[u8]) -> Result<Vec<u8>, String> {
    let u32_at = |pos: usize| -> Result<u32, String> {
        raw.get(pos..pos + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .ok_or_else(|| format!("truncated record at byte {pos}"))
    };
    let mut out = String::new();
    let mut pos = 0usize;
    let mut index = 0usize;
    while pos < raw.len() {
        let start = pos;
        if u32_at(pos)? != 0x4246_5142 {
            return Err(format!("bad record magic at byte {pos}"));
        }
        let kind = u32_at(pos + 4)?;
        let (w, h, bpp, mode) = (u32_at(pos + 8)?, u32_at(pos + 12)?, u32_at(pos + 16)?, u32_at(pos + 20)?);
        pos += 24 + (w as usize) * (h as usize) * (bpp as usize);
        let npal = u32_at(pos)?;
        pos += 4 + 4 * npal as usize;
        let record = raw
            .get(start..pos)
            .ok_or_else(|| format!("truncated record {index}"))?;
        out.push_str(&format!(
            "frame {index} kind={kind} {w}x{h} bpp={bpp} mode={mode} pal={npal} fnv={:016x}\n",
            fnv1a64(record)
        ));
        index += 1;
    }
    Ok(out.into_bytes())
}

/// A new directory `<prefix>_<pid>_<random>` inside `base`, created with
/// `create_dir` semantics (fails on an existing path or symlink, so nobody
/// can pre-plant it); private on Unix.
fn fresh_dir(base: &Path, prefix: &str) -> std::io::Result<PathBuf> {
    use std::hash::{BuildHasher, Hasher};
    let mut last = std::io::Error::other("no fresh work dir");
    for attempt in 0..16u32 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u32(std::process::id());
        h.write_u32(attempt);
        if let Ok(t) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            h.write_u128(t.as_nanos());
        }
        let dir = base.join(format!("{prefix}_{}_{:016x}", std::process::id(), h.finish()));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
        match builder.create(&dir) {
            Ok(()) => return Ok(dir),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => last = e,
            Err(e) => return Err(e),
        }
    }
    Err(last)
}

fn main() -> ExitCode {
    let mut opts = match parse_args() {
        Ok(o) => o,
        Err(msg) => {
            eprintln!("{msg}");
            return ExitCode::from(2);
        }
    };
    if !opts.qb.exists() {
        eprintln!("[FAIL] qb not found at {}", opts.qb.display());
        return ExitCode::from(2);
    }
    let mut cases = Vec::new();
    if let Err(e) = collect_cases(&opts.cases, &mut cases) {
        eprintln!("[FAIL] cannot read cases from {}: {e}", opts.cases.display());
        return ExitCode::from(2);
    }
    if let Some(f) = &opts.filter {
        cases.retain(|c| c.to_string_lossy().contains(f.as_str()));
    }
    if let Err(e) = fs::create_dir_all(&opts.work) {
        eprintln!("[FAIL] cannot create work dir {}: {e}", opts.work.display());
        return ExitCode::from(2);
    }
    // Work in (and later delete) only a new subdirectory: `--work .` must not
    // wipe the given folder.
    match fresh_dir(&opts.work, "qbnex_conformance") {
        Ok(dir) => opts.work = dir,
        Err(e) => {
            eprintln!("[FAIL] cannot create a work dir in {}: {e}", opts.work.display());
            return ExitCode::from(2);
        }
    }

    let (mut passed, mut blessed, mut xfailed, mut failed) = (0usize, 0usize, 0usize, Vec::new());
    let mut skipped = 0usize;
    for case in &cases {
        let rel = case.strip_prefix(&opts.cases).unwrap_or(case).display().to_string();
        let xfail_reason = fs::read_to_string(case.with_extension("xfail"))
            .ok()
            .map(|s| s.lines().next().unwrap_or("").trim().to_string());
        let start = Instant::now();
        if opts.bless && case.with_extension("deviation").exists() {
            println!("[keep]  {rel} (intentional deviation, hand-written golden)");
            continue;
        }
        if let Some(reason) = xfail_reason {
            let outcome = if opts.bless {
                // Never capture golden output from a known-broken run.
                let check = Options { bless: false, ..opts.clone() };
                run_case(&check, case)
            } else {
                run_case(&opts, case)
            };
            match outcome {
                Outcome::Fail(_) => {
                    xfailed += 1;
                    println!("[xfail] {rel} ({reason})");
                }
                Outcome::Pass | Outcome::Blessed => {
                    println!("[XPASS] {rel}\n    known defect no longer reproduces; delete the .xfail marker ({reason})");
                    failed.push(rel);
                }
                Outcome::Skip(why) => {
                    skipped += 1;
                    println!("[skip]  {rel} ({why})");
                }
            }
            continue;
        }
        match run_case(&opts, case) {
            Outcome::Pass => {
                passed += 1;
                println!("[ok]    {rel} ({:.1}s)", start.elapsed().as_secs_f32());
            }
            Outcome::Blessed => {
                blessed += 1;
                println!("[bless] {rel}");
            }
            Outcome::Fail(why) => {
                println!("[FAIL]  {rel}\n    {why}");
                failed.push(rel);
            }
            Outcome::Skip(why) => {
                skipped += 1;
                println!("[skip]  {rel} ({why})");
            }
        }
    }
    let _ = fs::remove_dir_all(&opts.work);

    println!(
        "\nconformance: {} cases, {passed} passed, {blessed} blessed, {xfailed} xfail, {skipped} skipped, {} failed",
        cases.len(),
        failed.len()
    );
    if failed.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
