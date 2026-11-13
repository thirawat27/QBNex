//! GUI programs (compiled without `$CONSOLE`) run with a window: PRINT and
//! graphics go to the screen, keys come from the window, closing it ends the
//! program. Every run here is headless (`QBNEX_HEADLESS=1`) and driven by an
//! input script (`QBNEX_INPUT_SCRIPT`), as the conformance and corpus runners
//! do; no window ever opens.
//!
//! `fixtures/window/*.qir` are `qb --ir` output of the `.bas` next to them.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("qb_exec_window_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn fixture(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/window").join(name);
    std::fs::read_to_string(p).unwrap()
}

/// Runs `qir` headless with `script`; returns (exit code, stdout). Panics after 30 s.
fn run(dir: &Path, qir: &str, script: &str) -> (Option<i32>, String) {
    let (code, out, _) = run_err(dir, qir, script);
    (code, out)
}

/// [`run`] that also returns stderr.
fn run_err(dir: &Path, qir: &str, script: &str) -> (Option<i32>, String, String) {
    std::fs::write(dir.join("prog.qir"), qir).unwrap();
    std::fs::write(dir.join("keys.input"), script).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_qb-exec"))
        .arg(dir.join("prog.qir"))
        .current_dir(dir)
        .env("QBNEX_HEADLESS", "1")
        .env("QBNEX_INPUT_SCRIPT", dir.join("keys.input"))
        .env("QBNEX_NO_ERROR_DIALOG", "1")
        .env_remove("QBNEX_FB_CAPTURE")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut out = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = out.read_to_string(&mut s);
        s
    });
    let mut err = child.stderr.take().unwrap();
    let err_reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = err.read_to_string(&mut s);
        s
    });
    let start = Instant::now();
    let status = loop {
        if let Some(st) = child.try_wait().unwrap() {
            break st;
        }
        if start.elapsed() > Duration::from_secs(30) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("program did not end within 30 s");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    (status.code(), reader.join().unwrap(), err_reader.join().unwrap())
}

const HELLO: &str = "QIR 1\nfunc main\nconst.str %0 x68656C6C6F\nconst.i32 %1 0\nrt PRINTS %1 %0\nrt PRINTNL %1\nend\nendfunc\n";
const BLANK: &str = "QIR 1\nfunc main\nend\nendfunc\n";

fn frame_line(out: &str) -> &str {
    out.lines().find(|l| l.starts_with("[frame ")).unwrap_or_else(|| panic!("no snapshot in {out:?}"))
}

#[test]
fn print_draws_on_the_window_and_end_waits_for_close() {
    let dir = tmpdir("print");
    let script = "wait 500\nsnapshot\nclose\n";
    let (code, out) = run(&dir, HELLO, script);
    assert_eq!(code, Some(0), "{out}");
    // nothing on stdout but the snapshot: the text is on the 640x400 SCREEN 0 window
    assert!(!out.contains("hello"), "{out}");
    let hello = frame_line(&out).to_string();
    assert!(hello.starts_with("[frame 640x400 "), "{hello}");
    let (_, blank) = run(&dir, BLANK, script);
    assert_ne!(frame_line(&blank), hello, "PRINT changed nothing on the screen");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn console_programs_keep_stdout() {
    let dir = tmpdir("console");
    let (code, out) = run(&dir, &HELLO.replace("QIR 1\n", "QIR 1\nmeta console only\n"), "wait 100\nsnapshot\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("hello"), "{out}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn input_reads_typed_keys_and_sleep_waits_for_a_key() {
    let dir = tmpdir("input");
    let script = "wait 400\ntype Bob\nkey enter\nwait 200\ntype 42\nkey enter\nwait 600\nkey space\nwait 400\nclose\n";
    let (code, out) = run(&dir, &fixture("winput.qir"), script);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(std::fs::read_to_string(dir.join("answer.txt")).unwrap().trim_end(), "Bob 42");
    assert_eq!(std::fs::read_to_string(dir.join("slept.txt")).unwrap().trim_end(), "ok");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sleep_without_a_key_waits_until_close() {
    let dir = tmpdir("sleep");
    let script = "wait 400\ntype Al\nkey enter\nwait 200\ntype 7\nkey enter\nwait 800\nclose\n";
    let (code, _) = run(&dir, &fixture("winput.qir"), script);
    assert_eq!(code, Some(0));
    assert!(dir.join("answer.txt").exists());
    assert!(!dir.join("slept.txt").exists(), "SLEEP returned without a key");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn graphics_screen_and_inkey() {
    let dir = tmpdir("gfx");
    let (code, out) = run(&dir, &fixture("wgfx.qir"), "wait 500\nsnapshot\nkey a\nwait 2000\nclose\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(frame_line(&out).starts_with("[frame 320x200 "), "{out}");
    assert_eq!(std::fs::read_to_string(dir.join("key.txt")).unwrap().trim(), "97");
    let _ = std::fs::remove_dir_all(&dir);
}

/// SUB _GL draws through a real OpenGL context (offscreen when headless): the
/// frame shows its glClear colour and its glBegin triangle over the software
/// screen. Skipped (passes) where no OpenGL context can be made.
#[test]
fn sub_gl_draws_clear_colour_and_triangle() {
    let dir = tmpdir("gl");
    let script = "wait 1000\nsnapshot\npixel 2 2\npixel 317 197\npixel 160 110\nclose\n";
    let (code, out, err) = run_err(&dir, &fixture("wgl.qir"), script);
    let _ = std::fs::remove_dir_all(&dir);
    if err.contains("SUB _GL is not run") {
        eprintln!("no OpenGL here, skipped: {err}");
        return;
    }
    assert_eq!(code, Some(0), "{out}{err}");
    assert!(frame_line(&out).starts_with("[frame 320x200 "), "{out}");
    assert!(out.contains("[pixel 2,2=FF0000FF]"), "clear colour in a corner: {out}{err}");
    assert!(out.contains("[pixel 317,197=FF0000FF]"), "clear colour in a corner: {out}{err}");
    assert!(out.contains("[pixel 160,110=FFFF0000]"), "red triangle in the middle: {out}{err}");
}

#[test]
fn close_ends_a_busy_loop() {
    let dir = tmpdir("loop");
    let prog = "QIR 1\nfunc main\nlabel L\nconst.i32 %0 1\njmp L\nendfunc\n";
    let t = Instant::now();
    let (code, _) = run(&dir, prog, "wait 300\nclose\n");
    assert_eq!(code, Some(0));
    assert!(t.elapsed() < Duration::from_secs(10));
    let _ = std::fs::remove_dir_all(&dir);
}

/// A close in a loop of short SOUND calls ends the program at once. SOUND
/// only queues the note, but the first sound statement opens the audio
/// device, which takes from a few hundred ms to a second; a close during
/// that open used to wait for it. The close comes 20 ms after the window
/// opens, so it lands inside the open. Without an audio device SOUND does
/// nothing and the loop is a plain busy loop.
#[test]
fn close_ends_a_sound_loop_at_once() {
    use std::io::{BufRead, BufReader};
    let dir = tmpdir("sound");
    let prog = "QIR 1\nfunc main\nlabel L\nconst.f64 %0 100\nconst.f64 %1 2\nrt SOUND %0 %1\njmp L\nendfunc\n";
    std::fs::write(dir.join("prog.qir"), prog).unwrap();
    // the snapshot line on stdout marks the moment of the close
    std::fs::write(dir.join("keys.input"), "wait 20\nsnapshot\nclose\n").unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_qb-exec"))
        .arg(dir.join("prog.qir"))
        .current_dir(&dir)
        .env("QBNEX_HEADLESS", "1")
        .env("QBNEX_INPUT_SCRIPT", dir.join("keys.input"))
        .env("QBNEX_NO_ERROR_DIALOG", "1")
        .env_remove("QBNEX_FB_CAPTURE")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let out = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        BufReader::new(out).lines().map_while(Result::ok).find(|l| l.starts_with("[frame ")).map(|_| Instant::now())
    });
    let start = Instant::now();
    let status = loop {
        if let Some(st) = child.try_wait().unwrap() {
            break st;
        }
        if start.elapsed() > Duration::from_secs(30) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("program did not end within 30 s");
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let ended = Instant::now();
    let closed = reader.join().unwrap().expect("no snapshot line before the close");
    assert_eq!(status.code(), Some(0));
    let took = ended.saturating_duration_since(closed);
    // about 70 ms here (mostly process teardown); waiting for the device took 0.4-1 s
    assert!(took < Duration::from_millis(400), "the program ended {took:?} after the close");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Q11: PRINT USING in a GUI program goes to the window like PRINT.
#[test]
fn print_using_draws_on_the_window() {
    let dir = tmpdir("using");
    let (code, out) = run(&dir, &fixture("q11_using.qir"), "wait 100\n");
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(out, "", "PRINT USING reached stdout");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Q12: the INT 33h mouse driver called through CALL ABSOLUTE (reset: AX=-1, BX=2;
/// status: BX buttons, CX/DX position).
#[test]
fn int33_mouse_through_call_absolute() {
    let dir = tmpdir("int33");
    let (code, out) = run(&dir, &fixture("q12_mouse.qir"), "wait 200\nmousemove 100 48\nwait 3000\n");
    assert_eq!(code, Some(0), "{out}");
    let t = std::fs::read_to_string(dir.join("mouse.txt")).unwrap();
    let lines: Vec<&str> = t.lines().map(str::trim).collect();
    // SCREEN 13: CX is twice the pixel x (libqb call_int)
    assert_eq!(lines, ["-1  2", "0  200  48"]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// _SCREENMOVE, _SCREENHIDE, _SCREENICON, _FULLSCREEN and the functions that read the
/// window back; after the first _EXIT the close box is a request the program reads.
#[test]
fn window_statements_and_exit_requests() {
    let dir = tmpdir("wstate");
    let (code, out) = run(&dir, &fixture("wstate.qir"), "wait 4000\nclose\n");
    assert_eq!(code, Some(0), "{out}");
    let t = std::fs::read_to_string(dir.join("state.txt")).unwrap();
    let lines: Vec<&str> = t.lines().map(str::trim_end).collect();
    assert_eq!(
        lines,
        [
            "exists-1",
            "desktop 1920  1080",
            "scaled 640  480",
            "at 30  40",
            // (1920 - 640) / 2, (1080 - 480) / 2
            "middle 640  300",
            "hidden 0 -1  0",
            "icon 0 -1",
            // off; square pixels, smooth; stretched (the default); off
            "full 0  0  2 -1  1  0  0",
            // _SCREENMOVE without a position
            "error 5",
            "exit 0",
            // the close box once, then nothing pending
            "request 1  0",
        ]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A resize, focus changes, a dropped file, the keyboard and the mouse as devices, mouse pipes.
#[test]
fn window_events_devices_and_mouse_pipes() {
    let dir = tmpdir("wevents");
    let script = "wait 1500\nresize 400 300\nwait 200\nfocus 0\nwait 200\nfocus 1\nwait 200\ndrop C:\\tmp\\one file.txt\nwait 200\n\
                  mousemove 319 199\nwait 200\nmousedown 1\nwait 200\nmouseup 1\nwait 200\nwheel -1\nwait 200\nkey s\nwait 200\nkey esc\n";
    let (code, out) = run(&dir, &fixture("wevents.qir"), script);
    assert_eq!(code, Some(0), "{out}");
    let t = std::fs::read_to_string(dir.join("events.txt")).unwrap();
    let lines: Vec<&str> = t.lines().map(str::trim_end).collect();
    assert_eq!(
        lines,
        [
            "drop-1  0",
            "devices 2 [KEYBOARD][BUTTON] [MOUSE][BUTTON][AXIS][WHEEL]",
            "last 512  3  2  3",
            // the headless stand-in has no native window
            "handle 0",
            // _FPS 0, an unknown pointer shape
            "error 5",
            "error 5",
            "resize 400  300",
            "focus 0",
            "focus-1",
            "file C:\\tmp\\one file.txt",
            // the bottom right pixel of the 320x200 page
            "axis 1  1",
            "button-1 -1",
            "button 0  1",
            "wheel-1",
            "key-1 -1",
            "key 0  1",
            // no further mouse event; the device stays selected
            "select 0 [MOUSE][BUTTON][AXIS][WHEEL]",
            // _MOUSEMOVE 10, 20 with a pipe open: nothing for the default queue until the pipe hands it on
            "default 0",
            "pipe 2  10  20  10  20",
            "handed on 2  10  20",
            "closed 30  40",
            // a closed pipe; _MOUSEMOVE outside the page
            "error 258",
            "error 5",
            "off 0",
        ]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// ON STRIG handlers run for the buttons of (scripted) game controllers; ALT+ENTER walks the
/// full screen modes `_ALLOWFULLSCREEN` allows.
#[test]
fn joystick_button_traps_and_alt_enter() {
    let dir = tmpdir("wtraps");
    let press = |pad: u32, button: u32| format!("pad {pad} {button} 1\nwait 60\npad {pad} {button} 0\nwait 60\n");
    let alt_enter = "keydown lalt\nwait 60\nkey enter\nwait 60\nkeyup lalt\nwait 200\n";
    let script = [
        "wait 1500\n",
        // joystick A button 1 twice, its button 2, joystick B button 1
        &press(0, 0),
        &press(0, 0),
        &press(0, 1),
        &press(1, 0),
        "wait 300\nkey a\nwait 300\n",
        &press(0, 0),
        &press(0, 0),
        "wait 300\nkey b\nwait 900\n",
        &press(0, 0),
        "wait 300\nkey c\nwait 300\n",
        alt_enter,
        "key d\nwait 300\n",
        alt_enter,
        "key e\nwait 300\n",
        alt_enter,
        "key f\nwait 300\n",
        alt_enter,
        "key g\n",
    ]
    .concat();
    let (code, out) = run(&dir, &fixture("wtraps.qir"), &script);
    assert_eq!(code, Some(0), "{out}");
    let t = std::fs::read_to_string(dir.join("traps.txt")).unwrap();
    let lines: Vec<&str> = t.lines().map(str::trim_end).collect();
    assert_eq!(
        lines,
        [
            "on 2  1  7",
            "stopped 2",
            "on again 3",
            // no handler, but STRIG still reads the button: not down now, pressed since the last call
            "off 3  0 -1",
            "all 1  0",
            "square smooth 2 -1",
            "window 0  0",
            // _ALLOWFULLSCREEN _OFF: the program gets the key
            "not allowed 0  1",
        ]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Q13: a CALL ABSOLUTE that fails gives its argument space back (no panic after many calls).
#[test]
fn failing_call_absolute_does_not_leak_argument_space() {
    let dir = tmpdir("absleak");
    let (code, out) = run(&dir, &fixture("q13_absleak.qir"), "");
    assert_eq!((code, out.trim()), (Some(0), "done"));
    let _ = std::fs::remove_dir_all(&dir);
}
