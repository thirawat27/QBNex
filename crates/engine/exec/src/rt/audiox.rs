//! Sound builtins of the runtime, routed to the `qb-audio` C ABI
//! (the same functions libqb's audio_bridge.cpp forwards to).

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

/// Handles `name` if it is a sound builtin; `None` otherwise.
pub(crate) fn call<'a>(vm: &mut dyn Host<'a>, name: &str, a: &[V]) -> Option<R<Option<V>>> {
    if !(name.starts_with("_SND") || matches!(name, "SOUND" | "BEEP" | "PLAY" | "PLAY.f" | "PLAYEVENT" | "_VOICE" | "_ADSR" | "_WAVE" | "_MEMSOUND")) {
        return None;
    }
    if !vm.rt().snd_started {
        snd::qb_snd_init();
        vm.rt().snd_started = true;
    }
    let i = |k: usize| a[k].i() as i32;
    let f = |k: usize| a[k].f();
    let s = |k: usize| a[k].s();
    let some = |v: V| Some(Ok(Some(v)));
    let none = Some(Ok(None));
    // SAFETY (all unsafe calls below): pointers are live slices of the given lengths.
    match name {
        "SOUND" => Some(raise(snd::qb_snd_sound(f(0), f(1)))),
        "BEEP" => Some(raise(snd::qb_snd_sound(900.0, 5.0))),
        "PLAY" => {
            let dblock = vm.rt().cmem[super::builtins::DBLOCK..].as_ptr();
            Some(raise(unsafe { snd::qb_snd_play(s(0).as_ptr(), s(0).len() as i32, dblock) }))
        }
        "PLAY.f" => some(V::I(snd::qb_snd_play_count().into())),
        // PLAY ON|OFF|STOP (1..3); ON PLAY(n) GOSUB starts the queue polling
        "PLAYEVENT" => Some(raise(qb_runtime::events::play_trap().statement(i(0)).err().unwrap_or(0))),
        "_VOICE" => Some(raise(snd::qb_snd_voice(i(0)))),
        "_ADSR" => Some(raise(snd::qb_snd_adsr(f(0), f(1), f(2), f(3), i(4), i(5)))),
        "_WAVE" => Some(raise(unsafe { snd::qb_snd_wave(s(0).as_ptr(), s(0).len() as i32, i(1), i(2)) })),
        "_SNDRATE" => some(V::I(snd::qb_snd_rate().into())),
        "_SNDOPEN" => {
            let (n, r) = (s(0), s(1));
            some(V::I(unsafe { snd::qb_snd_open(n.as_ptr(), n.len() as i32, r.as_ptr(), r.len() as i32, i(2)) }.into()))
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
            unsafe { snd::qb_snd_play_file(s(0).as_ptr(), s(0).len() as i32, i(1), f(2), i(3)) };
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
            // audio_bridge.cpp func__memsound
            let (h, ch) = (i(1), i(2));
            let mut info = snd::QbMemSound { offset: 0, size: 0, element_size: 0, mem_type: 0, lock_offset: 0, lock_id: 0 };
            let ok = unsafe { snd::qb_snd_memsound(h, ch, &mut info) } != 0;
            let mut mb = MemBlock::default();
            if !ok {
                // SAFETY: reading the runtime's lock base pointer.
                mb.lock_offset = unsafe { mem::mem_lock_base } as isize;
                mb.lock_id = mem::MEM_LOCK_INVALID_ID as i64;
            } else {
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
