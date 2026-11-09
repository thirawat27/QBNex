// qbrt.h — C ABI of the Rust runtime (internal/c/qbrt).
//
// Every function declared here is implemented in Rust and replaces code that
// used to live in libqb.cpp. Keep this file in sync with the #[no_mangle]
// exports in internal/c/qbrt/src and bump QBRT_ABI_VERSION on breaking changes.
#ifndef QBNEX_QBRT_H
#define QBNEX_QBRT_H

#include <stdint.h>

#define QBRT_ABI_VERSION 1u

#ifdef __cplusplus
extern "C" {
#endif

uint32_t qbrt_abi_version(void);

// --- MBF <-> IEEE (was internal/c/msbin.cpp) -------------------------------
// Return 0 on success, 1 on overflow. Pointers may be unaligned.
int32_t _fmsbintoieee(float *src4, float *dest4);
int32_t _fieeetomsbin(float *src4, float *dest4);
int32_t _dmsbintoieee(double *src8, double *dest8);
int32_t _dieeetomsbin(double *src8, double *dest8);

// --- QBasic rounding (was qbr* in libqb.cpp, x87 fistp) ---------------------
// *_f80 take a pointer to the 10 bytes of an x87 long double (may be unaligned).
// *_f64 are used where long double is not the x87 80-bit format.
int64_t qbrt_qbr_f80(const unsigned char *f80);
int64_t qbrt_fistp_i64_f80(const unsigned char *f80);
int64_t qbrt_qbr_f64(double v);
int64_t qbrt_fistp_i64_f64(double v);
int32_t qbrt_fistp_i32(double v);
void qbrt_fpu_reinit(void);

// --- RND / RANDOMIZE (was func_rnd / sub_randomize / rnd_seed) --------------
float qbrt_rnd(float n, int32_t passed);
void qbrt_randomize(double seed, int32_t use_using);
void qbrt_randomize_prompt(int16_t seed);
void qbrt_rnd_reset(void);

// --- VAL (was func_val) -------------------------------------------------------
// Writes the 10-byte x87 extended result to out_f80; returns 0 or a QBasic error code.
int32_t qbrt_val(const unsigned char *chr, int32_t len, unsigned char *out_f80);
double qbrt_f80_to_f64(const unsigned char *f80);

// --- STR$ (was the qbs_str overloads) -----------------------------------------
// Write the text (no terminator) into out[QBRT_STR_MAX] and return its length.
#define QBRT_STR_MAX 64
int32_t qbrt_str_i64(int64_t v, unsigned char *out);
int32_t qbrt_str_u64(uint64_t v, unsigned char *out);
int32_t qbrt_str_f32(float v, unsigned char *out);
int32_t qbrt_str_f64(double v, unsigned char *out);

// --- TIMER (was the arithmetic of func_timer) ---------------------------------
// Writes the value to *out and returns 0, or returns a QBasic error code.
int32_t qbrt_timer(uint32_t ticks, uint32_t clock_first, uint32_t day_first,
                   double accuracy, int32_t passed, double *out);

// ============================================================================
// R4: files, file system, environment, SHELL, _MEM
// Functions return 0 or a QBasic error code that the C++ shim raises.
// ============================================================================

// --- _MEM (was new_mem_lock / free_mem_lock / func__mem* / sub__mem*) --------
// Same layout as mem_block / mem_lock in common.h (generated code uses them).
typedef struct qbrt_mem_block {
    intptr_t offset, size;
    int64_t lock_id;
    intptr_t lock_offset, type, elementsize;
    int32_t image, sound;
} qbrt_mem_block;
typedef struct qbrt_mem_lock {
    uint64_t id;
    int32_t type;
    void *offset;
} qbrt_mem_lock;
// Defined in Rust; generated code reads and writes them directly.
extern uint64_t mem_lock_id;
extern qbrt_mem_lock *mem_lock_tmp;
extern qbrt_mem_lock *mem_lock_base;
qbrt_mem_lock *qbrt_new_mem_lock(void); // also sets mem_lock_tmp
void qbrt_free_mem_lock(qbrt_mem_lock *lock);
qbrt_mem_lock *qbrt_mem_lock_invalid(void);
int32_t qbrt_memfree(qbrt_mem_block *blk);
int32_t qbrt_mem_at_offset(intptr_t offset, intptr_t size, int32_t new_error, qbrt_mem_block *out);
int32_t qbrt_memnew(intptr_t bytes, int32_t new_error, qbrt_mem_block *out);
int32_t qbrt_memexists(const qbrt_mem_block *blk);
void *qbrt_memget(const qbrt_mem_block *blk, intptr_t off, intptr_t bytes, int32_t *err);
int32_t qbrt_memfill(const qbrt_mem_block *blk, intptr_t doff, intptr_t dbytes, const uint8_t *src, intptr_t sbytes);
void qbrt_memfill_nochecks(intptr_t doff, intptr_t dbytes, const uint8_t *src, intptr_t sbytes);
int32_t qbrt_memcopy(const qbrt_mem_block *sblk, intptr_t soff, intptr_t bytes, const qbrt_mem_block *dblk,
                     intptr_t doff);
void qbrt_mem(intptr_t offset, intptr_t size, int32_t type, intptr_t elementsize, qbrt_mem_lock *lock,
              qbrt_mem_block *out);

#ifdef __cplusplus
}
#endif

#endif // QBNEX_QBRT_H
