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
// Image operations (handles resolved by the C++ shims). which: 0 red 1 green 2 blue 3 alpha.
uint32_t qbrt_gfx_rgb(const struct img_struct *im, int32_t r, int32_t g, int32_t b);
uint32_t qbrt_gfx_rgba(const struct img_struct *im, int32_t r, int32_t g, int32_t b, int32_t a);
uint32_t qbrt_gfx_matchcol(const struct img_struct *im, int32_t r, int32_t g, int32_t b);
int32_t qbrt_gfx_component(const struct img_struct *im, uint32_t col, int32_t which, int32_t *err);
int32_t qbrt_gfx_setalpha(struct img_struct *im, int32_t a, uint32_t c, uint32_t c2, int32_t passed);
int32_t qbrt_gfx_clearcolor(struct img_struct *im, uint32_t c, int32_t passed);
uint32_t qbrt_gfx_palettecolor_get(const struct img_struct *im, int32_t n, int32_t *err);
int32_t qbrt_gfx_palettecolor_set(struct img_struct *im, int32_t n, uint32_t c);
// Image registry. `img`/`nimg`/`nextimg` are owned by Rust; `img` moves when
// the registry grows (compare nimg before/after a call and refresh cached
// page pointers). Index 0 is never returned by a successful allocation once
// libqb has reserved slots 0 and 1 at start-up.
extern struct img_struct *img;
extern int32_t nimg;
extern int32_t nextimg;
void qbrt_img_init(void);
uint32_t qbrt_img_new(int32_t *err);
void qbrt_img_free(uint32_t i);
int32_t qbrt_img_grow(int32_t required);
void qbrt_img_alloc_pixels(int32_t i, intptr_t bytes);
void qbrt_img_alloc_palette(int32_t i);
void qbrt_img_release_pixels(int32_t i);
void qbrt_img_release_palette(int32_t i);
void qbrt_img_set_external_offset(int32_t i, uint8_t *offset);
void qbrt_img_set_external_palette(int32_t i, uint32_t *pal);
void qbrt_img_release_owned(int32_t i);
int32_t qbrt_imgframe(uint8_t *o, int32_t x, int32_t y, int32_t bpp, int32_t *err);
int32_t qbrt_imgnew(int32_t x, int32_t y, int32_t bpp, int32_t *err);
void qbrt_imgrevert(int32_t i);
// DRAW: cmem is the conventional memory block (VARPTR$ arguments point into it).
int32_t qbrt_gfx_draw(struct img_struct *im, const uint8_t *s, int32_t len, const uint8_t *cmem);
double qbrt_gfx_draw_f80(const uint8_t *p); // diagnostics: x87 long double -> double load
// _PUTIMAGE on resolved, distinct surfaces (passed: legacy bits). With hw_out
// (8 int32: sx1,sy1,sx2,sy2,dx1,dy1,dx2,dy2) only the coordinates are resolved.
typedef struct qbrt_put_coords { double dx1, dy1, dx2, dy2, sx1, sy1, sx2, sy2; } qbrt_put_coords;
int32_t qbrt_gfx_putimage(struct img_struct *s, struct img_struct *d, const qbrt_put_coords *c, int32_t passed,
                          int32_t *hw_out);
uint64_t qbrt_gfx_putimage_oob_count(void); // diagnostics (deviation D-06)
// Graphics GET/PUT on read_page/write_page; data/len = the array memory.
int32_t qbrt_gfx_get(struct img_struct *im, float x1, float y1, float x2, float y2, uint8_t *data, int32_t len,
                     uint32_t mask, int32_t passed);
int32_t qbrt_gfx_put(struct img_struct *im, float x1, float y1, const uint8_t *data, int32_t len, int32_t option,
                     uint32_t mask, int32_t passed);
// PCOPY on resolved pages; built-in font glyph at the text cursor of a graphics page.
int32_t qbrt_gfx_pcopy(const struct img_struct *s, const struct img_struct *d);
/* SCREEN decisions (gfx/screen.rs): mode check (mode >= 0), graphics mode
   size + font override (out[3] = w, h, font or 0; returns 0 if not a graphics
   mode), SCREEN 0 layout (out[5] = cols, rows, font, cmem bytes per page,
   cmem pages); the override pointers are reset to -1 where legacy did. */
int32_t qbrt_gfx_screen_mode_valid(int32_t mode);
int32_t qbrt_gfx_screen_geometry(int32_t mode, int32_t prev_w, int32_t prev_h, int32_t *out);
void qbrt_gfx_screen0_layout(int32_t prev_w, int32_t prev_h, int32_t *width, int32_t *height, int32_t *font, int32_t *out);
void qbrt_gfx_printchr(const struct img_struct *im, int32_t character);
// Software _MAPTRIANGLE on resolved surfaces: s = sx1,sy1,..,sy3; d = dx1,dy1,..,dy3.
int32_t qbrt_gfx_maptriangle(const struct img_struct *src, const struct img_struct *dst, const float *s, const float *d,
                             int32_t seamless);
// Headless framebuffer capture (QBNEX_FB_CAPTURE); kind 0 = _DISPLAY, 1 = end.
void qbrt_fb_capture(const struct img_struct *display_page, int32_t kind);

#ifdef __cplusplus
}
#endif

#endif // QBNEX_QBRT_H
