// Text console (phase R12a): crates/qb-runtime/src/console. Included by qb_runtime.h.
#ifndef QB_CONSOLE_H
#define QB_CONSOLE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

struct img_struct;

// Services libqb.cpp provides to the console (registered once at start-up).
typedef struct qb_console_host {
    struct img_struct *(*write_page)(void);
    struct img_struct *(*read_page)(void);
    int32_t (*image_index)(const struct img_struct *im);
    struct img_struct *(*image)(int32_t handle);
    int32_t (*new_error)(void);
    void (*error)(int32_t n);
    int32_t (*font_width)(uint32_t f);
    int32_t (*font_height)(uint32_t f);
    int32_t (*font_flags)(uint32_t f);
    void (*printchr_custom)(uint32_t character);
    int32_t (*chrwidth_custom)(uint32_t character);
    int32_t (*printwidth)(const uint8_t *text, int32_t len);
    void (*lprint_print_page)(void);
    void (*key_redraw)(void);
    int32_t (*take_width8050switch)(void);
    void (*width_80x50)(void);
    int32_t (*page_count)(void);
    struct img_struct *(*page_image)(int32_t i);
    int32_t (*dest)(void);
    void (*set_dest)(int32_t h);
    int32_t (*source)(void);
    void (*set_source)(int32_t h);
    int32_t (*copyimage)(int32_t h);
    void (*freeimage)(int32_t h);
    void (*pcopy)(int32_t src, int32_t dst);
    int32_t (*autodisplay_begin)(void);
    void (*autodisplay_end)(int32_t previous);
    int32_t (*stop_program)(void);
    uint8_t *(*bios)(void);
    void (*events)(void);
    void (*sleep)(uint32_t ms);
} qb_console_host;

void qb_console_set_host(const qb_console_host *host);

// PRINT, cursor, CLS, VIEW PRINT, LOCATE, SCREEN(), COLOR, WIDTH (console)
void qb_console_print(const uint8_t *s, int32_t len, int32_t finish_on_new_line);
void qb_console_newline(void);
void qb_console_tab(void);
void qb_console_makefit(const uint8_t *s, int32_t len);
void qb_console_printchr(uint32_t character, int32_t cell_span);
int32_t qb_console_chrwidth(uint32_t character);
void qb_console_view_print(int32_t top, int32_t bottom, int32_t passed);
void qb_console_cls(int32_t method, uint32_t use_color, int32_t passed);
void qb_console_locate(int32_t row, int32_t column, int32_t cursor, int32_t start, int32_t stop, int32_t passed);
int32_t qb_console_csrlin(void);
int32_t qb_console_pos(void);
uint32_t qb_console_screen(int32_t y, int32_t x, int32_t returncol, int32_t passed);
void qb_console_color(uint32_t col1, uint32_t col2, int32_t passed);
int32_t qb_console_width(int32_t v1, int32_t v2, int32_t v3, int32_t v4);

// stdout / stdin and the Windows console
void qb_console_write_utf8(const uint8_t *text, int32_t len, int32_t finish_on_new_line);
void qb_console_cursor(int32_t visible, int32_t size, int32_t passed);
int32_t qb_console_input_event(void);
int32_t qb_console_cinp(int32_t toggle, int32_t passed);

// keyboard: INPUT / LINE INPUT, INKEY$ buffer (bios = cmem+0x400), INPUT$
void qb_console_input(void **offsets, int32_t *types, int32_t n, int32_t newline);
int32_t qb_console_inkey(uint8_t *out2);
void qb_inkey_push(uint8_t *bios, uint8_t b1, uint8_t b2);
void qb_inkey_clear(uint8_t *bios);
void qb_console_flush_input(void);
void qb_console_input_n(int32_t n, uint8_t *out);

// UTF-8 side table of SCREEN 0 surfaces and code page 437
void qb_text_cells_init(int32_t image, int32_t cells);
void qb_text_cells_release(int32_t image);
void qb_text_cells_copy(int32_t src, int32_t dst);
int32_t qb_text_cells_present(int32_t image);
int32_t qb_text_cells_has_content(int32_t image);
int32_t qb_text_cells_span(int32_t image, int32_t cell);
int32_t qb_text_cell_utf8(int32_t image, int32_t cell, uint8_t *out, int32_t cap);
int32_t qb_utf8_decode(const uint8_t *text, int32_t len, uint32_t *codepoint);
int32_t qb_utf8_is_zero_width(uint32_t codepoint);
int32_t qb_utf8_is_emoji_like(uint32_t codepoint);
uint32_t qb_unicode_to_cp437(uint32_t x);
extern uint16_t codepage437_to_unicode16[256];

// state shared with libqb.cpp
extern int32_t lprint, lpos, width_lprint, lprint_buffered;
extern int32_t no_control_characters, no_control_characters2;
extern int32_t consolekey, consolemousex, consolemousey, consolebutton;

#ifdef __cplusplus
}
#endif

#endif // QB_CONSOLE_H
