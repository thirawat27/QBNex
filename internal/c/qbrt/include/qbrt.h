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

// --- _LOADIMAGE codecs (was dr_pcx + stb_image in parts/video/image/src.c) -------
// Decodes PCX (dr_pcx rules) or BMP/PNG/JPEG/GIF/TGA/PNM (stb_image rules).
// Returns w*h*4 bytes of BGRA allocated with malloc (release with free) or NULL.
uint8_t *qbrt_image_decode(const uint8_t *data, int32_t len, int32_t *w, int32_t *h);
// 32bpp BGRA -> 8bpp with ordered dithering; fills palette_out[256] (BGRA).
// Returns w*h bytes allocated with malloc, or NULL.
uint8_t *qbrt_image_to_8bpp(const uint8_t *bgra, int32_t w, int32_t h, uint32_t *palette_out);
// Rewrites the indices of an 8bpp image from src_pal to the nearest colours of dst_pal.
void qbrt_image_remap_palette(uint8_t *pixels, int32_t w, int32_t h, const uint32_t *src_pal,
                              const uint32_t *dst_pal);

// --- _LOADFONT fonts (was the FreeType wrapper parts/video/font/ttf/src.c) -------
// Same contracts as the legacy FontLoad / FontRenderTextUTF32 / FontWidth / FontFree.
// qbrt_font_load returns a handle >= 1 or 0; options 16 = monospace.
// qbrt_font_render_utf32 returns 1/0; options 1 = monochrome; *out_data is malloc'ed
// (NULL for an empty string) and released by the caller with free.
int32_t qbrt_font_load(const uint8_t *content, int32_t bytes, int32_t pixel_height, int32_t which_font,
                       int32_t options);
int32_t qbrt_font_render_utf32(int32_t font, const uint32_t *codepoint, int32_t codepoints, int32_t options,
                               uint8_t **out_data, int32_t *out_x, int32_t *out_y, int32_t *out_x_pre_increment,
                               int32_t *out_x_post_increment);
int32_t qbrt_font_width(int32_t font);
void qbrt_font_free(int32_t font);

#ifdef __cplusplus
}
#endif

#endif // QBNEX_QBRT_H
