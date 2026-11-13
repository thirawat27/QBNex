//! Sound builtins of the runtime, routed to `qb_audio::ffi`.

use super::builtins::Host;
use super::{Stop, R, V};
use qb_audio::ffi as snd;
use qb_runtime::mem::{self, MemBlock};

fn raise(err: i32) -> R<Option<V>> {
    if err != 0 {
        return Err(Stop::Err(err));
    }
    Ok(None)
}

/// Opens the audio device for the first sound statement. Opening it can take
/// a second or more (the program thread waits for the device thread), so in
/// a window program the open runs on a helper thread while the program
/// thread keeps watching the close box: a close during the wait ends the
/// program at once instead of after the device is up.
fn start_audio(rt: &mut super::builtins::RtState<'_>) {
    #[cfg(not(target_arch = "wasm32"))]
    if rt.window {
        let Ok(open) = std::thread::Builder::new().name("qb-audio-open".into()).spawn(snd::qb_snd_init) else {
            snd::qb_snd_init();
            return;
        };
        while !open.is_finished() {
            if super::window::closed() {
                super::builtins::close_exit(rt);
            }
            qb_runtime::systime::sleep_ms(5);
        }
        let _ = open.join();
        return;
    }
    let _ = rt;
    snd::qb_snd_init();
}

/// Seconds of SOUND/PLAY audio that may wait in the voice queues in
/// music-background mode (`PLAY "MB"`). Like the QBasic music buffer it has
/// a limit: a statement that queues past it waits until the queue plays
/// down, so `DO: SOUND f, d: LOOP` does not grow the queue until memory runs
/// out. Programs that queue less never wait.
const QUEUE_LIMIT_SECONDS: i64 = 10;

/// Notes the QBasic background music buffer holds: a statement that queues past
/// it waits until the earlier notes have played, which is what keeps a program's
/// animation in step with its music.
const MUSIC_BUFFER_NOTES: i64 = 32;

/// A wait gives up when the queue has not moved for this long (an output
/// device that stopped pulling frames), so a program never hangs on it.
const STALL_MS: i64 = 2000;

/// Waits after SOUND/PLAY. In music-foreground mode (`MF`, the QBasic
/// default) the statement returns once its notes have played; in
/// music-background mode (`MB`) it returns at once unless more than
/// [`MUSIC_BUFFER_NOTES`] notes or [`QUEUE_LIMIT_SECONDS`] per voice are queued. Without an audio engine
/// (`QB_AUDIO_DEVICE=none`) nothing is queued and nothing waits. The browser
/// build only applies the background limit: there the page pulls the mix.
fn hold_queue(rt: &mut super::builtins::RtState<'_>) {
    let background = i64::from(snd::qb_snd_rate()) * QUEUE_LIMIT_SECONDS;
    #[cfg(not(target_arch = "wasm32"))]
    let limit = if snd::qb_snd_foreground() != 0 { 0 } else { background };
    #[cfg(target_arch = "wasm32")]
    let limit = background;
    let mut last = i64::MAX;
    let mut moved = qb_runtime::systime::ticks();
    loop {
        let queued = i64::from(snd::qb_snd_play_count());
        let notes = i64::from(snd::qb_snd_play_notes());
        if (queued <= limit && notes <= MUSIC_BUFFER_NOTES) || background <= 0 {
            break;
        }
        let now = qb_runtime::systime::ticks();
        if queued < last {
            last = queued;
            moved = now;
        } else if now.saturating_sub(moved) >= STALL_MS {
            break;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if super::window::closed() {
            super::builtins::close_exit(rt);
        }
        qb_runtime::systime::sleep_ms(5);
    }
    let _ = rt;
}

/// Handles `name` if it is a sound builtin; `None` otherwise.
pub(crate) fn call<'a>(vm: &mut dyn Host<'a>, name: &str, a: &[V]) -> Option<R<Option<V>>> {
    if !(name.starts_with("_SND") || matches!(name, "SOUND" | "BEEP" | "PLAY" | "PLAY.f" | "PLAYEVENT" | "_VOICE" | "_ADSR" | "_WAVE" | "_MEMSOUND")) {
        return None;
    }
    if !vm.rt().snd_started {
        start_audio(vm.rt());
        vm.rt().snd_started = true;
    }
    let i = |k: usize| a[k].i() as i32;
    let f = |k: usize| a[k].f();
    let s = |k: usize| a[k].s();
    let some = |v: V| Some(Ok(Some(v)));
    let none = Some(Ok(None));
    match name {
        "SOUND" | "BEEP" => {
            let e = if name == "BEEP" { snd::qb_snd_sound(900.0, 5.0) } else { snd::qb_snd_sound(f(0), f(1)) };
            hold_queue(vm.rt());
            Some(raise(e))
        }
        "PLAY" => {
            let e = snd::qb_snd_play(s(0), &vm.rt().cmem[super::builtins::DBLOCK..]);
            hold_queue(vm.rt());
            Some(raise(e))
        }
        "PLAY.f" => some(V::I(snd::qb_snd_play_count().into())),
        // PLAY ON|OFF|STOP (1..3); ON PLAY(n) GOSUB starts the queue polling
        "PLAYEVENT" => Some(raise(qb_runtime::events::play_trap().statement(i(0)).err().unwrap_or(0))),
        "_VOICE" => Some(raise(snd::qb_snd_voice(i(0)))),
        "_ADSR" => Some(raise(snd::qb_snd_adsr(f(0), f(1), f(2), f(3), i(4), i(5)))),
        "_WAVE" => Some(raise(snd::qb_snd_wave(s(0), i(1), i(2)))),
        "_SNDRATE" => some(V::I(snd::qb_snd_rate().into())),
        "_SNDOPEN" => {
            some(V::I(snd::qb_snd_open(s(0), s(1), i(2)).into()))
        }
        "_SNDCLOSE" => {
            snd::qb_snd_close(i(0));
            none
        }
        "_SNDCOPY" => some(V::I(snd::qb_snd_copy(i(0)).into())),
        "_SNDPLAY" => {
            snd::qb_snd_play_handle(i(0));
            none
        }
        "_SNDPLAYCOPY" => {
            snd::qb_snd_play_copy(i(0), f(1), i(2));
            none
        }
        "_SNDPLAYFILE" => {
            snd::qb_snd_play_file(s(0), i(1), f(2), i(3));
            none
        }
        "_SNDPAUSE" => {
            snd::qb_snd_pause(i(0));
            none
        }
        "_SNDPLAYING" => some(V::I(snd::qb_snd_playing(i(0)).into())),
        "_SNDPAUSED" => some(V::I(snd::qb_snd_paused(i(0)).into())),
        "_SNDVOL" => {
            snd::qb_snd_vol(i(0), f(1) as f32);
            none
        }
        "_SNDLOOP" => {
            snd::qb_snd_loop(i(0));
            none
        }
        "_SNDBAL" => {
            snd::qb_snd_bal(i(0), f(1), f(2), f(3), i(4), i(5));
            none
        }
        "_SNDLEN" => some(V::F(snd::qb_snd_len(i(0)))),
        "_SNDGETPOS" => some(V::F(snd::qb_snd_getpos(i(0)))),
        "_SNDSETPOS" => {
            snd::qb_snd_setpos(i(0), f(1));
            none
        }
        "_SNDLIMIT" => {
            snd::qb_snd_limit(i(0), f(1));
            none
        }
        "_SNDSTOP" => {
            snd::qb_snd_stop(i(0));
            none
        }
        "_SNDOPENRAW" => some(V::I(snd::qb_snd_open_raw().into())),
        "_SNDRAW" => {
            snd::qb_snd_raw(f(0) as f32, f(1) as f32, i(2), i(3));
            none
        }
        "_SNDRAWDONE" => {
            snd::qb_snd_raw_done(i(0), i(1));
            none
        }
        "_SNDRAWLEN" => some(V::F(snd::qb_snd_raw_len(i(0), i(1)))),
        "_MEMSOUND" => {
            // `_MEMSOUND`: a `_MEM` over the sound's sample buffer
            let (h, ch) = (i(1), i(2));
            let mut mb = MemBlock::default();
            if let Some(info) = snd::qb_snd_memsound(h, ch) {
                if info.lock_offset != 0 {
                    mb.lock_offset = info.lock_offset as isize;
                    mb.lock_id = info.lock_id;
                } else {
                    let l = mem::new_mem_lock();
                    // SAFETY: fresh lock.
                    unsafe { (*l).type_ = 5 };
                    mb.lock_offset = l as isize;
                    // SAFETY: fresh lock.
                    mb.lock_id = unsafe { (*l).id } as i64;
                    snd::qb_snd_memsound_set_lock(h, l as usize, mb.lock_id);
                }
                mb.type_ = info.mem_type as isize;
                mb.elementsize = info.element_size as isize;
                mb.offset = info.offset as isize;
                mb.size = info.size as isize;
                mb.sound = h;
            } else {
                mb.lock_offset = mem::mem_lock_base() as isize;
                mb.lock_id = mem::MEM_LOCK_INVALID_ID as i64;
            }
            let r = vm.bytes(a[0].p(), std::mem::size_of::<MemBlock>()).map(|b| {
                // SAFETY: the destination is a _MEM UDT (MemBlock layout).
                unsafe { *(b.as_mut_ptr() as *mut MemBlock) = mb };
                None
            });
            Some(r)
        }
        _ => None,
    }
}
