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

// --- String engine (was qbs_* in libqb.cpp, qbs_cleanup in qbx.cpp) ------------
// struct qbs is defined in internal/c/common.h; generated code uses its fields.
// The engine also exports the globals qbs_tmp_list_nexti, qbs_cmem_sp and
// qbs_cmem_descriptor_space (uint32; declared where they are used, not here, to
// keep their existing C++ declarations valid). It needs from the host:
//   uint8 cmem[]; uint32 cmem_sp; ptrszint dblock;
//   void qbrt_host_error(int32_t); void qbrt_host_field_free(struct qbs*);
struct qbs;
struct qbs *qbrt_qbs_new_descriptor(void);
void qbrt_qbs_free_descriptor(struct qbs *s);
void qbrt_qbs_free(struct qbs *s);
void qbrt_qbs_cmem_concat_list(void);
void qbrt_qbs_concat_list(void);
void qbrt_qbs_tmp_concat_list(void);
void qbrt_qbs_concat(uint32_t bytesrequired);
void qbrt_qbs_concat_cmem(uint32_t bytesrequired);
struct qbs *qbrt_qbs_new_cmem(int32_t size, uint8_t tmp);
struct qbs *qbrt_qbs_new_txt(const char *txt);
struct qbs *qbrt_qbs_new_txt_len(const char *txt, int32_t len);
struct qbs *qbrt_qbs_new_fixed(uint8_t *offset, uint32_t size, uint8_t tmp);
struct qbs *qbrt_qbs_new(int32_t size, uint8_t tmp);
void qbrt_qbs_maketmp(struct qbs *s);
struct qbs *qbrt_qbs_set(struct qbs *dest, struct qbs *src);
void qbrt_qbs_cleanup(uint32_t base); // slow path; callers test qbs_tmp_list_nexti==base first
struct qbs *qbrt_qbs_add(struct qbs *a, struct qbs *b);
struct qbs *qbrt_qbs_ucase(struct qbs *s);
struct qbs *qbrt_qbs_lcase(struct qbs *s);
struct qbs *qbrt_func_chr(int32_t value);
struct qbs *qbrt_func_varptr_helper(uint8_t type, uint16_t offset);
struct qbs *qbrt_qbs_left(struct qbs *s, int32_t l);
struct qbs *qbrt_qbs_right(struct qbs *s, int32_t l);
struct qbs *qbrt_func_space(int32_t spaces);
struct qbs *qbrt_func_string(int32_t characters, int32_t asciivalue);
void qbrt_set_qbs_size(intptr_t *target_qbs, int32_t newlength);
int32_t qbrt_func_instr(int32_t start, struct qbs *str, struct qbs *substr, int32_t passed);
int32_t qbrt_func__instrrev(int32_t start, struct qbs *str, struct qbs *substr, int32_t passed);
// MID$ statement without the new_error / nothingstring checks (done by the shim).
void qbrt_sub_mid(struct qbs *dest, int32_t start, int32_t l, struct qbs *src, int32_t passed);
struct qbs *qbrt_func_mid(struct qbs *str, int32_t start, int32_t l, int32_t passed);
struct qbs *qbrt_qbs_ltrim(struct qbs *s);
struct qbs *qbrt_qbs_rtrim(struct qbs *s);
struct qbs *qbrt_qbs__trim(struct qbs *s);
int32_t qbrt_func__str_nc_compare(struct qbs *a, struct qbs *b);
int32_t qbrt_func__str_compare(struct qbs *a, struct qbs *b);
int32_t qbrt_qbs_equal(struct qbs *a, struct qbs *b);
int32_t qbrt_qbs_notequal(struct qbs *a, struct qbs *b);
int32_t qbrt_qbs_greaterthan(struct qbs *a, struct qbs *b);
int32_t qbrt_qbs_lessthan(struct qbs *a, struct qbs *b);
int32_t qbrt_qbs_lessorequal(struct qbs *a, struct qbs *b);
int32_t qbrt_qbs_greaterorequal(struct qbs *a, struct qbs *b);
int32_t qbrt_qbs_asc(struct qbs *s, uint32_t i);
int32_t qbrt_qbs_asc1(struct qbs *s);
int32_t qbrt_qbs_len(struct qbs *s);
// Introspection (tests / differential harness only).
typedef struct QbrtQbsDebugState {
    const intptr_t *list;
    uint32_t list_nexti, list_lasti;
    const intptr_t *cmem_list;
    uint32_t cmem_list_nexti, cmem_list_lasti;
    const intptr_t *tmp_list;
    uint32_t tmp_list_lasti;
    const uint8_t *data;
    uint32_t data_size, sp;
    struct qbs *const *freed;
    uint32_t freed_count, blocks, malloc_next;
} QbrtQbsDebugState;
void qbrt_qbs_debug_state(QbrtQbsDebugState *out);
int64_t qbrt_qbs_debug_descriptor_index(const struct qbs *s);
void qbrt_qbs_reset(void);

#ifdef __cplusplus
}
#endif

#endif // QBNEX_QBRT_H
