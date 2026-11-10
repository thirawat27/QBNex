//-----------------------------------------------------------------------------------------------------
// QBNex audio: thin shims over the Rust audio runtime (crates/qb-audio).
//
// This file is #included into libqb.cpp. The engine that used to live in
// audio.cpp (miniaudio, stb_vorbis, libxmp-lite, tinysoundfont, RADv2) is
// gone; these functions keep the old signatures that generated code calls,
// convert qbs strings, raise BASIC errors and manage _MEM locks. Everything
// else happens in Rust (qb_audio.h).
//-----------------------------------------------------------------------------------------------------

#include "audio_bridge.h"

#ifndef DEPENDENCY_AUDIO_OUT
// Just stubs if no audio support is required
void snd_mainloop() { return; }
void snd_init() { return; }
void snd_un_init() { return; }
#else

#include "../../../crates/qb-runtime/include/qb_audio.h"

#define QB_INVALID_MEM_LOCK 1073741821
#define QB_MEM_TYPE_SOUND 5

static void qb_snd_free_released_locks() {
    uintptr_t lock;
    while ((lock = qb_snd_take_released_lock()) != 0)
        free_mem_lock((mem_lock *)lock);
}

static void qb_snd_raise(int32_t err) {
    if (err)
        error(err);
}

void snd_init() { qb_snd_init(); }

void snd_un_init() {
    qb_snd_un_init();
    qb_snd_free_released_locks();
}

void snd_mainloop() {
    qb_snd_mainloop();
    qb_snd_free_released_locks();
}

void sub_sound(double frequency, double lengthInClockTicks) {
    if (new_error)
        return;
    qb_snd_raise(qb_snd_sound(frequency, lengthInClockTicks));
}

void sub_beep() { sub_sound(900, 5); }

int32_t func_play(int32_t ignore) {
    (void)ignore;
    return qb_snd_play_count();
}

void sub_play(qbs *str) {
    if (new_error)
        return;
    qb_snd_raise(qb_snd_play(str->chr, str->len, (const uint8_t *)dblock));
}

void sub__voice(int32_t voice) { qb_snd_raise(qb_snd_voice(voice)); }

void sub__adsr(double attack, double decay, double sustain, double release, int32_t voice, int32_t passed) {
    qb_snd_raise(qb_snd_adsr(attack, decay, sustain, release, voice, passed));
}

void sub__wave(qbs *spec, int32_t voice, int32_t passed) {
    qb_snd_raise(qb_snd_wave(spec ? spec->chr : NULL, spec ? spec->len : 0, voice, passed));
}

int32_t func__sndrate() { return qb_snd_rate(); }

int32_t func__sndopen(qbs *fileName, qbs *requirements, int32_t passed) {
    return qb_snd_open(fileName->chr, fileName->len, passed && requirements ? requirements->chr : NULL, passed && requirements ? requirements->len : 0,
                         passed);
}

void sub__sndclose(int32_t handle) { qb_snd_close(handle); }
int32_t func__sndcopy(int32_t src_handle) { return qb_snd_copy(src_handle); }
void sub__sndplay(int32_t handle) { qb_snd_play_handle(handle); }
void sub__sndplaycopy(int32_t src_handle, double volume, int32_t passed) { qb_snd_play_copy(src_handle, volume, passed); }
void sub__sndplayfile(qbs *fileName, int32_t sync, double volume, int32_t passed) {
    qb_snd_play_file(fileName->chr, fileName->len, sync, volume, passed);
}
void sub__sndpause(int32_t handle) { qb_snd_pause(handle); }
int32_t func__sndplaying(int32_t handle) { return qb_snd_playing(handle); }
int32_t func__sndpaused(int32_t handle) { return qb_snd_paused(handle); }
void sub__sndvol(int32_t handle, float volume) { qb_snd_vol(handle, volume); }
void sub__sndloop(int32_t handle) { qb_snd_loop(handle); }
void sub__sndbal(int32_t handle, double x, double y, double z, int32_t channel, int32_t passed) { qb_snd_bal(handle, x, y, z, channel, passed); }
double func__sndlen(int32_t handle) { return qb_snd_len(handle); }
double func__sndgetpos(int32_t handle) { return qb_snd_getpos(handle); }
void sub__sndsetpos(int32_t handle, double seconds) { qb_snd_setpos(handle, seconds); }
void sub__sndlimit(int32_t handle, double limit) { qb_snd_limit(handle, limit); }
void sub__sndstop(int32_t handle) { qb_snd_stop(handle); }

int32_t func__sndopenraw() { return qb_snd_open_raw(); }
void sub__sndraw(float left, float right, int32_t handle, int32_t passed) { qb_snd_raw(left, right, handle, passed); }
void sub__sndrawdone(int32_t handle, int32_t passed) { qb_snd_raw_done(handle, passed); }
double func__sndrawlen(int32_t handle, int32_t passed) { return qb_snd_raw_len(handle, passed); }

mem_block func__memsound(int32_t handle, int32_t targetChannel) {
    static mem_block mb;
    qb_mem_sound info;

    if (!qb_snd_memsound(handle, targetChannel, &info)) {
        mb.offset = 0;
        mb.size = 0;
        mb.lock_offset = (ptrszint)mem_lock_base;
        mb.lock_id = QB_INVALID_MEM_LOCK;
        mb.type = 0;
        mb.elementsize = 0;
        mb.sound = 0;
        mb.image = 0;
        return mb;
    }

    if (info.lock_offset) {
        mb.lock_offset = (ptrszint)info.lock_offset;
        mb.lock_id = info.lock_id;
    } else {
        new_mem_lock();
        mem_lock_tmp->type = QB_MEM_TYPE_SOUND;
        mb.lock_offset = (ptrszint)mem_lock_tmp;
        mb.lock_id = mem_lock_id;
        qb_snd_memsound_set_lock(handle, (uintptr_t)mem_lock_tmp, (int64_t)mem_lock_id);
    }

    mb.type = info.mem_type;
    mb.elementsize = info.element_size;
    mb.offset = (ptrszint)info.offset;
    mb.size = info.size;
    mb.sound = handle;
    mb.image = 0;
    return mb;
}

#endif
