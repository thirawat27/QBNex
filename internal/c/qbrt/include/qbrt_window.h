// qbrt_window.h — C ABI of the Rust window/input layer (R8, internal/c/qbrt/src/{window,keybuf,keymap,raster}.rs).
// Replaces freeglut, GLEW and the OpenGL presenter in libqb/gui.cpp.
#ifndef QBNEX_QBRT_WINDOW_H
#define QBNEX_QBRT_WINDOW_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// --- keyboard buffers (keybuf.rs) ---------------------------------------------
void qbrt_input_lock(void);   // re-entrant; guards keydown()/keyup()/mouse queue
void qbrt_input_unlock(void);
void qbrt_keyhit_push(int64_t v);
int32_t qbrt_keyhit_pop(void);  // 0 when empty
void qbrt_keyhit_clear(void);
int32_t qbrt_keyheld(uint32_t x);  // includes the numpad multimapping
void qbrt_keyheld_add(uint32_t x, uint32_t bind);
void qbrt_keyheld_remove(uint32_t x);
int32_t qbrt_keyheld_bound_to(uint32_t x, uint32_t *out);
void qbrt_shift_state(uint8_t *out3);  // cmem 0x417, 0x418, 0x496

// --- key translation (keymap.rs); used by tests/diff_keymap.cpp -------------
// Return 0 (no call), 1 keydown, 2 keyup; *code receives the key code.
int32_t qbrt_keymap_glut_special(int32_t key, int32_t down, uint32_t *code);
int32_t qbrt_keymap_glut_ascii(int32_t key, int32_t mods, int32_t down, int32_t macos, uint32_t *code);

// --- window (window.rs) ---------------------------------------------------------
typedef struct qbrt_window_callbacks {
    void (*key)(int32_t action, uint32_t code);  // 1 down, 2 up, 3 unicode down, 4 unicode up
    void (*mouse_button)(int32_t glut_button, int32_t down, int32_t x, int32_t y);
    void (*mouse_move)(int32_t x, int32_t y);
    void (*mouse_wheel)(int32_t direction, int32_t x, int32_t y);
    void (*mouse_relative)(int32_t dx, int32_t dy);
    void (*resize)(int32_t w, int32_t h);
    void (*close)(void);
    void (*focus)(int32_t focused);
    void (*file_drop)(const char *utf8_path);
    void (*created)(void *native_handle);
} qbrt_window_callbacks;

#define QBRT_CURSOR_HIDDEN 0
#define QBRT_CURSOR_DEFAULT 1
#define QBRT_CURSOR_LINK 2
#define QBRT_CURSOR_TEXT 3
#define QBRT_CURSOR_CROSSHAIR 4
#define QBRT_CURSOR_VERTICAL 5
#define QBRT_CURSOR_HORIZONTAL 6
#define QBRT_CURSOR_TOPLEFT_BOTTOMRIGHT 7
#define QBRT_CURSOR_TOPRIGHT_BOTTOMLEFT 8
#define QBRT_CURSOR_WAIT 9
#define QBRT_CURSOR_HELP 10
#define QBRT_CURSOR_MOVE 11

// Runs on the main thread; does not return in windowed mode.
int32_t qbrt_window_run(const qbrt_window_callbacks *cb, int32_t w, int32_t h, const char *title,
                        int32_t resizable, int32_t start_hidden);
int32_t qbrt_window_headless(void);
int32_t qbrt_window_exists(void);
void qbrt_window_create(void);
void qbrt_window_present(const uint32_t *bgra, int32_t w, int32_t h, int32_t full_screen,
                         int32_t fullscreen_smooth, int32_t resize_auto);
void qbrt_window_set_title(const char *title);
void qbrt_window_set_size(int32_t w, int32_t h);
void qbrt_window_set_fullscreen(int32_t on);
void qbrt_window_set_position(int32_t x, int32_t y);
void qbrt_window_show(int32_t visible);
void qbrt_window_minimize(void);
int32_t qbrt_window_minimized(void);
void qbrt_window_set_cursor(int32_t style);
void qbrt_window_warp_mouse(int32_t x, int32_t y);
void qbrt_window_set_icon(const uint32_t *bgra, int32_t w, int32_t h);
void qbrt_window_size(int32_t *w, int32_t *h);
void qbrt_window_position(int32_t *x, int32_t *y);
void qbrt_desktop_size(int32_t *w, int32_t *h);
// out: x1,y1,x2,y2,scaled_w,scaled_h,smooth,letterbox; scale: x,y
void qbrt_layout(int32_t screen_w, int32_t screen_h, int32_t window_w, int32_t window_h,
                 int32_t full_screen, int32_t fullscreen_smooth, int32_t resize_auto,
                 int32_t *out8, float *scale2);

// --- "hardware" images in software (raster.rs) ----------------------------------
void qbrt_hw_put(uint32_t *dst, int32_t dw, int32_t dh, const uint32_t *src, int32_t sw, int32_t sh,
                 const int32_t *d4, const int32_t *s4, int32_t use_alpha, int32_t smooth);
void qbrt_hw_tri2d(uint32_t *dst, int32_t dw, int32_t dh, const uint32_t *src, int32_t sw, int32_t sh,
                   const float *d6, const float *s6, int32_t use_alpha, int32_t smooth);
void qbrt_hw_tri3d(uint32_t *dst, int32_t dw, int32_t dh, float *depth, const uint32_t *src, int32_t sw,
                   int32_t sh, const float *d9, const float *s6, int32_t use_alpha, int32_t smooth,
                   int32_t cull, int32_t depth_mode);

#ifdef __cplusplus
}
#endif

#endif // QBNEX_QBRT_WINDOW_H
