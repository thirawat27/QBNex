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

// --- Software graphics (R5, was the drawing code of libqb.cpp) ----------------
// Surfaces are `struct img_struct` (common.h); Rust mirrors its layout exactly.
// Statement entry points act on the page passed in (the C++ shims pass
// write_page/read_page) and return a QBasic error code (0 = none) that the
// shim raises with error().
struct img_struct;
// Built-in data (were filled from embedded arrays at start-up).
extern const uint32_t palette_256[256];
extern const uint32_t palette_64[64];
extern const uint32_t pal_mode10[2][9];
// The fonts are immutable Rust statics; they are declared non-const because
// legacy text code walks them with plain uint8* cursors. Never write to them.
extern uint8_t charset8x8[256][8][8];
extern uint8_t charset8x16[256][16][8];
// Blending tables (cblend: 16M, ablend: 64K), built on first use.
void qbrt_gfx_blend_tables(const uint8_t **cblend, const uint8_t **ablend);
// Pixel writers (no range check beyond the VIEW for pset_and_clip).
void qbrt_gfx_pset(const struct img_struct *im, int32_t x, int32_t y, uint32_t col);
uint32_t qbrt_gfx_point_raw(const struct img_struct *im, int32_t x, int32_t y);
void qbrt_gfx_fast_boxfill(const struct img_struct *im, int32_t x1, int32_t y1, int32_t x2, int32_t y2, uint32_t col);
void qbrt_gfx_boxfill(const struct img_struct *im, float x1, float y1, float x2, float y2, uint32_t col);
void qbrt_gfx_fast_line(const struct img_struct *im, int32_t x1, int32_t y1, int32_t x2, int32_t y2, uint32_t col);
// Palettes and colors.
void qbrt_gfx_restorepalette(const struct img_struct *im);
int32_t qbrt_gfx_palette(struct img_struct *im, uint32_t attribute, uint32_t col, int32_t passed);
int32_t qbrt_gfx_color(struct img_struct *im, uint32_t col1, uint32_t col2, int32_t passed);
void qbrt_gfx_defaultcolors(struct img_struct *im);
// Statements and functions.
int32_t qbrt_gfx_line(struct img_struct *im, float x1, float y1, float x2, float y2, uint32_t col, int32_t bf,
                      uint32_t style, int32_t passed);
int32_t qbrt_gfx_circle(struct img_struct *im, double x, double y, double r, uint32_t col, double start, double end,
                        double aspect, int32_t passed);
int32_t qbrt_gfx_paint(struct img_struct *im, float x, float y, uint32_t fillcol, uint32_t bordercol, int32_t passed);
int32_t qbrt_gfx_paint_tile(struct img_struct *im, float x, float y, const uint8_t *tile, int32_t len,
                            uint32_t bordercol, int32_t passed);
int32_t qbrt_gfx_pset_stmt(struct img_struct *im, float x, float y, uint32_t col, int32_t passed);
int32_t qbrt_gfx_preset_stmt(struct img_struct *im, float x, float y, uint32_t col, int32_t passed);
double qbrt_gfx_point(const struct img_struct *write_page, const struct img_struct *read_page, float x, float y,
                      int32_t passed, int32_t *err);
int32_t qbrt_gfx_window(struct img_struct *im, float x1, float y1, float x2, float y2, int32_t passed);
int32_t qbrt_gfx_view(struct img_struct *im, int32_t x1, int32_t y1, int32_t x2, int32_t y2, int32_t fillcolor,
                      int32_t bordercolor, int32_t passed);
float qbrt_gfx_pmap(const struct img_struct *im, float val, int32_t option, int32_t *err);
// Headless framebuffer capture (QBNEX_FB_CAPTURE); kind 0 = _DISPLAY, 1 = end.
void qbrt_fb_capture(const struct img_struct *display_page, int32_t kind);

#ifdef __cplusplus
}
#endif

#endif // QBNEX_QBRT_H
