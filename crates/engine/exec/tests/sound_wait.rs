//! SOUND/PLAY timing against the silent audio device (`QB_AUDIO_DEVICE=null`):
//! in music-foreground mode (the QBasic default) a statement returns once its
//! notes have played, after `PLAY "MB"` it returns at once, and the music
//! queue drains in real time so `ON PLAY(n)` fires. `QB_AUDIO_DEVICE=none`
//! starts no engine and never waits.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn hex(s: &str) -> String {
    let mut h = String::from("x");
    for b in s.bytes() {
        h.push_str(&format!("{b:02x}"));
    }
    h
}

/// Runs `prog` with `QB_AUDIO_DEVICE=device`; returns (stdout, elapsed).
fn run(name: &str, device: &str, prog: &str) -> (String, Duration) {
    let dir = std::env::temp_dir().join(format!("qb-sound-wait-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let qir = dir.join(format!("{name}.qir"));
    std::fs::write(&qir, prog).unwrap();
    let t = Instant::now();
    let o = Command::new(env!("CARGO_BIN_EXE_qb-exec"))
        .arg(&qir)
        .env("QB_AUDIO_DEVICE", device)
        .env("QBNEX_NO_ERROR_DIALOG", "1")
        .env("QBNEX_HEADLESS", "1")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let elapsed = t.elapsed();
    let _ = std::fs::remove_file(&qir);
    assert!(o.status.success(), "{name}: {}", String::from_utf8_lossy(&o.stderr));
    (String::from_utf8_lossy(&o.stdout).replace("\r\n", "\n"), elapsed)
}

/// `[PLAY mode$:] SOUND 440, 18.2` (one second of sound).
fn one_second_sound(mode: &str) -> String {
    let play = if mode.is_empty() { String::new() } else { format!("const.str %2 {}\nrt PLAY %2\n", hex(mode)) };
    format!("QIR 1\nmeta console only\nfunc main\n{play}const.f64 %0 440\nconst.f64 %1 18.2\nrt SOUND %0 %1\nend\nendfunc\n")
}

#[test]
fn sound_waits_for_its_note_in_foreground_mode() {
    let (_, t) = run("mf", "null", &one_second_sound(""));
    assert!(t >= Duration::from_millis(950), "SOUND returned after {t:?}");
    assert!(t < Duration::from_secs(5), "SOUND returned after {t:?}");
}

#[test]
fn sound_queues_in_background_mode() {
    let (_, t) = run("mb", "null", &one_second_sound("MB"));
    assert!(t < Duration::from_millis(900), "SOUND returned after {t:?}");
}

#[test]
fn no_engine_never_waits() {
    let (_, t) = run("none", "none", &one_second_sound(""));
    assert!(t < Duration::from_millis(900), "SOUND returned after {t:?}");
}

/// `ON PLAY(2) GOSUB refill: PLAY ON: PLAY "MB T255 L16 CDEFG"`, then wait
/// (5 s guard) for the handler and print its flag.
#[test]
fn on_play_fires_on_the_silent_device() {
    let prog = format!(
        "QIR 1
meta console only
global $__LONG_FIRED i32
func main
const.i32 %0 2
onplay %0 L_REFILL
const.i32 %1 1
rt PLAYEVENT %1
const.str %2 {}
rt PLAY %2
rt %3 TIMER
label ~L1
load.i32 %4 $__LONG_FIRED
jnz %4 ~DONE
rt %5 TIMER
sub.f64 %6 %5 %3
const.f64 %7 5.0E+0
gt.f64 %8 %6 %7
jz %8 ~L1
label ~DONE
load.i32 %9 $__LONG_FIRED
cvt.i64.i32 %10 %9
rt %11 STR$.i64 %10
const.i32 %12 0
rt PRINTS %12 %11
rt PRINTNL %12
end
label L_REFILL
const.i32 %13 -1
store.i32 $__LONG_FIRED %13
gret
endfunc
",
        hex("MB T255 L16 CDEFG")
    );
    let (out, t) = run("onplay", "null", &prog);
    assert_eq!(out, "-1\n");
    assert!(t < Duration::from_secs(4), "handler ran after {t:?}");
}
