// qbrt_audio.h — C ABI of the Rust audio runtime (internal/c/qbrt/audio).
//
// Replaces internal/c/parts/audio/out/audio.cpp + miniaudio. Called only by
// the shims in internal/c/parts/audio/out/src.c when DEPENDENCY_AUDIO_OUT is
// defined, so console-only programs never reference these symbols and do not
// link the audio code. Functions returning int32_t "err" give a QBasic error
// number (0 = none). Strings are (pointer, length) pairs.
#ifndef QBNEX_QBRT_AUDIO_H
#define QBNEX_QBRT_AUDIO_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

void qbrt_snd_init(void);
void qbrt_snd_un_init(void);
void qbrt_snd_mainloop(void);
// Mem locks of freed _MEMSOUND handles; returns 0 when none are left.
uintptr_t qbrt_snd_take_released_lock(void);

int32_t qbrt_snd_sound(double frequency, double ticks);
int32_t qbrt_snd_beep(void);
int32_t qbrt_snd_play(const uint8_t *s, int32_t len, const uint8_t *dblock);
int32_t qbrt_snd_play_count(void);
int32_t qbrt_snd_voice(int32_t voice);
int32_t qbrt_snd_adsr(double attack, double decay, double sustain, double release, int32_t voice, int32_t passed);
int32_t qbrt_snd_wave(const uint8_t *spec, int32_t len, int32_t voice, int32_t passed);

int32_t qbrt_snd_rate(void);
int32_t qbrt_snd_open(const uint8_t *name, int32_t name_len, const uint8_t *req, int32_t req_len, int32_t passed);
void qbrt_snd_close(int32_t handle);
int32_t qbrt_snd_copy(int32_t handle);
void qbrt_snd_play_handle(int32_t handle);
void qbrt_snd_play_copy(int32_t handle, double volume, int32_t passed);
void qbrt_snd_play_file(const uint8_t *name, int32_t len, int32_t sync, double volume, int32_t passed);
void qbrt_snd_pause(int32_t handle);
int32_t qbrt_snd_playing(int32_t handle);
int32_t qbrt_snd_paused(int32_t handle);
void qbrt_snd_vol(int32_t handle, float volume);
void qbrt_snd_loop(int32_t handle);
void qbrt_snd_bal(int32_t handle, double x, double y, double z, int32_t channel, int32_t passed);
double qbrt_snd_len(int32_t handle);
double qbrt_snd_getpos(int32_t handle);
void qbrt_snd_setpos(int32_t handle, double seconds);
void qbrt_snd_limit(int32_t handle, double seconds);
void qbrt_snd_stop(int32_t handle);

int32_t qbrt_snd_open_raw(void);
void qbrt_snd_raw(float left, float right, int32_t handle, int32_t passed);
void qbrt_snd_raw_done(int32_t handle, int32_t passed);
double qbrt_snd_raw_len(int32_t handle, int32_t passed);

typedef struct qbrt_mem_sound {
    uintptr_t offset;
    uintptr_t size;
    uintptr_t element_size;
    int32_t mem_type;
    uintptr_t lock_offset; // 0: no lock yet, create one and call qbrt_snd_memsound_set_lock
    int64_t lock_id;
} qbrt_mem_sound;
int32_t qbrt_snd_memsound(int32_t handle, int32_t target_channel, qbrt_mem_sound *out);
void qbrt_snd_memsound_set_lock(int32_t handle, uintptr_t lock_offset, int64_t lock_id);

// Device-less synth engine for offline rendering (tests/diff_audio.cpp).
typedef struct qbrt_synth qbrt_synth;
qbrt_synth *qbrt_synth_new(uint32_t rate);
void qbrt_synth_free(qbrt_synth *e);
int32_t qbrt_synth_sound(qbrt_synth *e, double frequency, double ticks);
int32_t qbrt_synth_play(qbrt_synth *e, const uint8_t *s, int32_t len, const uint8_t *dblock);
int32_t qbrt_synth_voice(qbrt_synth *e, int32_t voice);
int32_t qbrt_synth_adsr(qbrt_synth *e, double a, double d, double s, double r, int32_t voice, int32_t passed);
int32_t qbrt_synth_wave(qbrt_synth *e, const uint8_t *spec, int32_t len, int32_t voice, int32_t passed);
int32_t qbrt_synth_drain(qbrt_synth *e, int32_t voice, float *out, int32_t max_frames);
int32_t qbrt_synth_queued(qbrt_synth *e, int32_t voice);

#ifdef __cplusplus
}
#endif

#endif
