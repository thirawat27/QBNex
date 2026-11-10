// Differential test (R12a): the legacy text console from libqb.cpp (verbatim in
// legacy/console_*.inc: printchr, newline, tab, makefit, qbs_print, VIEW PRINT,
// CLS, LOCATE, CSRLIN, POS, SCREEN(), the UTF-8 cell side table, INKEY$ ring,
// INPUT / LINE INPUT) vs Rust qb_runtime::console through its C ABI. Both sides
// get identical surfaces (text, 8-bit, 32-bit, custom font) and the same random
// operations; surfaces, cursor state, side tables, errors, return values and
// INPUT variables are compared after every step.
//
//   g++ -O2 -std=c++17 -w tests/diff/diff_console.cpp <libqb_runtime.a> <native libs> -o diff_console
//   ./diff_console [--quick] [--ops <n> <inputs>]   (QBDIFF_TRACE=1 traces every step)
//
// The $CONSOLE (stdout / Windows console) paths are not covered here; the
// conformance corpus runs them.

#include <windows.h>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>

typedef int8_t int8;
typedef uint8_t uint8;
typedef int16_t int16;
typedef uint16_t uint16;
typedef int32_t int32;
typedef uint32_t uint32;
typedef int64_t int64;
typedef uint64_t uint64;
typedef intptr_t ptrszint;
typedef uintptr_t uptrszint;
#define QBNex_WINDOWS
// type flags (common.h)
#define ISSTRING 1073741824
#define ISFLOAT 536870912
#define ISUNSIGNED 268435456
#define ISOFFSETINBITS 16777216

// Verbatim img_struct from internal/c/common.h.
struct img_struct{
    void *lock_offset;
    int64 lock_id;
    uint8 valid;
    uint8 text;
    uint8 console;
    uint16 width,height;
    uint8 bytes_per_pixel;
    uint8 bits_per_pixel;
    uint32 mask;
    uint16 compatible_mode;
    uint32 color,background_color,draw_color;
    uint32 font;
    int16 top_row,bottom_row;
    int16 cursor_x,cursor_y;
    uint8 cursor_show, cursor_firstvalue, cursor_lastvalue;
    union{
        uint8 *offset;
        uint32 *offset32;
    };
    uint32 flags;
    uint32 *pal;
    int32 transparent_color;
    uint8 alpha_disabled;
    uint8 holding_cursor;
    uint8 print_mode;
    uint8 apm_p1;
    int32 view_x1,view_y1,view_x2,view_y2;
    int32 view_offset_x,view_offset_y;
    float x,y;
    uint8 clipping_or_scaling;
    float scaling_x,scaling_y,scaling_offset_x,scaling_offset_y;
    float window_x1,window_y1,window_x2,window_y2;
    double draw_ta;
    double draw_scale;
    uint8 apm_p2;
};
static_assert(sizeof(img_struct) == 192, "layout");
#define IMG_SCREEN 2

struct qbs_field;
struct qbs{
    uint8 *chr;
    int32 len;
    uint8 in_cmem;
    uint16 *cmem_descriptor;
    uint16 cmem_descriptor_offset;
    uint32 listi;
    uint8 tmp;
    uint32 tmplisti;
    uint8 fixed;
    uint8 readonly;
    qbs_field *field;
};

#include "../../crates/qb-runtime/include/qb_runtime.h"

// --- Rust string engine host (both sides use it for INPUT strings) ---
static const size_t CMEM_SIZE = 1114099;
static int32 rust_err = 0;
extern "C" {
uint8 cmem[CMEM_SIZE];
uint32 cmem_sp = 65536;
ptrszint dblock;
void qb_host_error(int32 n) { if (!rust_err) rust_err = n; }
void qb_host_field_free(qbs *s) {}
extern uint32 qbs_tmp_list_nexti;
}

static uint64_t rng = 0x2545F4914F6CDD1Dull;
static uint32 rnd() { rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17; return (uint32)rng; }
static bool trace = getenv("QBDIFF_TRACE") != NULL;

// shared deterministic stand-ins
static uint8 charset8x8_data[256 * 64], charset8x16_data[256 * 128];
static int32 fake_render(uint32 ch, int32 options, uint8 **out, int32 *w, int32 *h) {
    *w = 3 + (ch % 4); *h = 12;
    uint8 *d = (uint8 *)malloc(*w * *h);
    for (int32 i = 0; i < *w * *h; i++) {
        uint32 v = (ch * 31 + i * 7) % 5;
        d[i] = v == 0 ? 255 : (options ? 0 : (uint8)(v * 50));
    }
    *out = d;
    return 1;
}
static int32 fake_printwidth(int32 len) { return len * 5; }

#define MAXIMG 64
static uint8 *keyscript[2]; static int32 keyscript_len[2], keyscript_pos[2];

namespace legacy {
img_struct *img; int32 nimg = MAXIMG;
img_struct *write_page, *read_page;
int32 write_page_index, read_page_index;
int32 new_error = 0, last_error = 0;
static void error(int32 n) { if (!last_error) last_error = n; }
uint8 cmem[0x500 + 0x100];
std::vector<int32> fontwidth(33, 8), fontheight(33, 16), fontflags(33, 0), font(33, 0);
int32 lpos = 1, width_lprint = 80, lprint = 0, lprint_image = 0, lprint_buffered = 0, lprint_locked = 0;
double lprint_last = 0;
int32 key_display_redraw = 0, key_updates = 0;
static void key_update() { key_updates++; }
int32 width8050switch = 0;
int32 pages = 1; std::vector<int32> page(1, 0);
int32 autodisplay = 1; uint8 stop_program = 0;
qbs *singlespace;
static void sub__printimage(int32 i) {}
static void qbsub_width(int32, int32, int32, int32, int32, int32) {}
static double func_timer(double, int32) { return 0; }
static int32 func__newimage(int32, int32, int32, int32) { return 0; }
static void sub__font(int32, int32, int32) {}
static void qbg_sub_color(uint32, uint32, uint32, int32) {}
static int64 GetTicks() { return 0; }
static void evnt(uint32) {}
static void Sleep(uint32 ms) {
    if (ms == 10 && cmem[0x41a] == cmem[0x41c] && keyscript_pos[0] < keyscript_len[0]) {
        uint8 k = keyscript[0][keyscript_pos[0]++];
        cmem[0x400 + cmem[0x41c]] = k; cmem[0x400 + cmem[0x41c] + 1] = 0; cmem[0x41c] += 2;
        if (cmem[0x41c] == 62) cmem[0x41c] = 30;
    }
}
static void qb_write_utf8(const char *, int32, int32) {}
static int32 qb_file_input_n(int32, int32, uint8 *, int32, int32 *) { return 0; }
static int32 port60h_events = 0;
static void qb_keyhit_clear() {}
static int32 FontRenderTextASCII(int32, uint8 *cp, int32, int32 options, uint8 **out, int32 *w, int32 *h, int32 *pre, int32 *post) {
    *pre = *post = 0; return fake_render(*cp, options, out, w, h);
}
static int32 FontRenderTextUTF32(int32, uint32 *cp, int32, int32 options, uint8 **out, int32 *w, int32 *h, int32 *pre, int32 *post) {
    *pre = *post = 0; return fake_render(*cp, options, out, w, h);
}
static int32 func__printwidth(qbs *t, int32, int32) { return fake_printwidth(t->len); }
static uint8 (*charset8x8)[8][8] = (uint8 (*)[8][8])charset8x8_data;
static uint8 (*charset8x16)[16][8] = (uint8 (*)[16][8])charset8x16_data;
static void pset(int32 x, int32 y, uint32 c) { qb_gfx_pset(write_page, x, y, c); }
static uint32 point(int32 x, int32 y) { return qb_gfx_point_raw(read_page, x, y); }
static void fast_boxfill(int32 x1, int32 y1, int32 x2, int32 y2, uint32 c) { qb_gfx_fast_boxfill(write_page, x1, y1, x2, y2, c); }
static void qb32_boxfill(float x1, float y1, float x2, float y2, uint32 c) { qb_gfx_boxfill(write_page, x1, y1, x2, y2, c); }
#include "legacy/qbr_x87.inc"
// strings: Rust engine (as libqb.cpp does)
static qbs *qbs_new(int32 s, uint8 t) { return qb_qbs_new(s, t); }
static qbs *qbs_new_txt(const char *t) { return qb_qbs_new_txt(t); }
static qbs *qbs_set(qbs *d, qbs *s) { return qb_qbs_set(d, s); }
static qbs *qbs_add(qbs *a, qbs *b) { return qb_qbs_add(a, b); }
static void qbs_free(qbs *s) { qb_qbs_free(s); }
template <typename T> static T qbs_cleanup(uint32 base, T v) { if (qbs_tmp_list_nexti > base) qb_qbs_cleanup(base); return v; }
// images for INPUT (handles are -index)
static int32 func__dest() { return -write_page_index; }
static int32 func__source() { return -read_page_index; }
static void sub__dest(int32 h) { write_page_index = -h; write_page = &img[-h]; }
static void sub__source(int32 h) { read_page_index = -h; read_page = &img[-h]; }
static int32 alloc_copy(img_struct *base, int32 h);
static int32 func__copyimage(int32 h, int32, int32) { return alloc_copy(img, h); }
static void sub__freeimage(int32 h, int32) { img[-h].valid = 0; }
static void sub_pcopy(int32 s, int32 d) {
    img_struct *a = &img[-s], *b = &img[-d];
    memcpy(b->offset, a->offset, (size_t)a->width * a->height * a->bytes_per_pixel);
}
static qbs *qbs_inkey();
static void qbs_print(qbs *, int32);
static void qbg_sub_locate(int32, int32, int32, int32, int32, int32);
static void sub_cls(int32, uint32, int32);
static void qbg_sub_view_print(int32, int32, int32);
static int32 func_pos(int32);
void *qbs_input_variableoffsets[257];
int32 qbs_input_variabletypes[257];
#define IMG_BUFFERSIZE 4096
#include "legacy/console_cells.inc"
#include "legacy/console_print.inc"
#include "legacy/console_cls_locate.inc"
#include "legacy/console_input.inc"
#include "legacy/console_misc.inc"
} // namespace legacy

// --- Rust side: own surfaces, host table over them ---
static img_struct rimg[MAXIMG];
static int32 r_dest = 1, r_source = 1, r_autodisplay = 1, r_key_updates = 0;
static uint8 r_bios[0x200];

int32 legacy::alloc_copy(img_struct *base, int32 h) {
    for (int32 i = 8; i < MAXIMG; i++) {
        if (!base[i].valid) {
            img_struct *s = &base[-h];
            size_t n = (size_t)s->width * s->height * s->bytes_per_pixel;
            base[i] = *s;
            base[i].offset = (uint8 *)malloc(n);
            memcpy(base[i].offset, s->offset, n);
            base[i].valid = 1;
            return -i;
        }
    }
    return -1;
}

namespace host {
static img_struct *write_page() { return &rimg[r_dest]; }
static img_struct *read_page() { return &rimg[r_source]; }
static int32 image_index(const img_struct *im) { return (int32)(im - rimg); }
static img_struct *image(int32 h) { return &rimg[-h]; }
static int32 new_error() { return 0; }
static void error(int32 n) { if (!rust_err) rust_err = n; }
static int32 font_width(uint32 f) { return legacy::fontwidth[f]; }
static int32 font_height(uint32 f) { return legacy::fontheight[f]; }
static int32 font_flags(uint32 f) { return legacy::fontflags[f]; }
// custom glyphs: the legacy renderer drawing on the Rust surface
static void printchr_custom(uint32 c) {
    img_struct *w = legacy::write_page; legacy::write_page = &rimg[r_dest];
    legacy::printchr(c, 1);
    legacy::write_page = w;
}
static int32 chrwidth_custom(uint32 c) {
    img_struct *w = legacy::write_page; legacy::write_page = &rimg[r_dest];
    int32 r = legacy::chrwidth(c);
    legacy::write_page = w;
    return r;
}
static int32 printwidth(const uint8 *, int32 len) { return fake_printwidth(len); }
static void lprint_print_page() {}
static void key_redraw() { r_key_updates++; }
static int32 take_width8050switch() { return 0; }
static void width_80x50() {}
static int32 page_count() { return 1; }
static img_struct *page_image(int32) { return NULL; }
static int32 dest() { return -r_dest; }
static void set_dest(int32 h) { r_dest = -h; }
static int32 source() { return -r_source; }
static void set_source(int32 h) { r_source = -h; }
static int32 copyimage(int32 h) { return legacy::alloc_copy(rimg, h); }
static void freeimage(int32 h) { rimg[-h].valid = 0; }
static void pcopy(int32 s, int32 d) {
    memcpy(rimg[-d].offset, rimg[-s].offset, (size_t)rimg[-s].width * rimg[-s].height * rimg[-s].bytes_per_pixel);
}
static int32 autodisplay_begin() { int32 b = r_autodisplay; r_autodisplay = 1; return b; }
static void autodisplay_end(int32) {}
static int32 stop_program() { return 0; }
static uint8 *bios() { return r_bios; }
static void events() {}
static void sleep(uint32 ms) {
    if (ms == 10 && r_bios[0x1a] == r_bios[0x1c] && keyscript_pos[1] < keyscript_len[1])
        qb_inkey_push(r_bios, keyscript[1][keyscript_pos[1]++], 0);
}
}
static const qb_console_host host_table = {
    host::write_page, host::read_page, host::image_index, host::image, host::new_error, host::error,
    host::font_width, host::font_height, host::font_flags, host::printchr_custom, host::chrwidth_custom,
    host::printwidth, host::lprint_print_page, host::key_redraw, host::take_width8050switch, host::width_80x50,
    host::page_count, host::page_image, host::dest, host::set_dest, host::source, host::set_source,
    host::copyimage, host::freeimage, host::pcopy, host::autodisplay_begin, host::autodisplay_end,
    host::stop_program, host::bios, host::events, host::sleep};

// --- surfaces ---
static void make(int32 i, int32 kind) {
    img_struct s; memset(&s, 0, sizeof(s));
    s.valid = 1; s.cursor_x = 1; s.cursor_y = 1; s.top_row = 1; s.draw_scale = 1; s.print_mode = 3;
    s.transparent_color = -1;
    switch (kind) {
    case 1: s.text = 1; s.width = 80; s.height = 25; s.bytes_per_pixel = 2; s.font = 16; s.mask = 15; s.color = 7; break;
    case 2: s.text = 1; s.width = 24; s.height = 6; s.bytes_per_pixel = 2; s.font = 8; s.mask = 15; s.color = 7; break;
    case 3: s.width = 64; s.height = 48; s.bytes_per_pixel = 1; s.compatible_mode = 13; s.font = 8; s.mask = 255; s.color = 15; break;
    case 4: s.width = 64; s.height = 32; s.bytes_per_pixel = 4; s.compatible_mode = 32; s.font = 16; s.mask = 0xFFFFFFFF; s.color = 0xFFFFFFFF; s.alpha_disabled = 1; break;
    default: s.width = 60; s.height = 36; s.bytes_per_pixel = 1; s.compatible_mode = 13; s.font = 32; s.mask = 255; s.color = 15; break;
    }
    int32 fh = s.text ? 1 : legacy::fontheight[s.font];
    s.bottom_row = s.height / fh;
    s.view_x2 = s.width - 1; s.view_y2 = s.height - 1;
    s.window_x2 = s.width - 1; s.window_y2 = s.height - 1;
    size_t n = (size_t)s.width * s.height * s.bytes_per_pixel;
    for (int side = 0; side < 2; side++) {
        img_struct *d = side ? &rimg[i] : &legacy::img[i];
        *d = s;
        d->offset = (uint8 *)malloc(n);
        if (s.text) for (size_t k = 0; k < n; k += 2) { d->offset[k] = 32; d->offset[k + 1] = 7; }
        else memset(d->offset, 0, n);
    }
    if (s.text) { legacy::qb_init_text_utf8_cells(i); qb_text_cells_init(i, s.width * s.height); }
}

static long mismatches = 0;
static long op_no = 0;
static const char *op_name = "";
static void fail(const char *what) {
    mismatches++;
    if (mismatches <= 25) { printf("mismatch #%ld op %ld (%s): %s\n", mismatches, op_no, op_name, what); fflush(stdout); }
}
static bool same_img(int32 i) {
    img_struct a = legacy::img[i], b = rimg[i];
    size_t n = (size_t)a.width * a.height * a.bytes_per_pixel;
    if (memcmp(a.offset, b.offset, n)) return false;
    a.offset = b.offset = NULL;
    return !memcmp(&a, &b, sizeof(a));
}
static void compare_all(int32 nsurf) {
    for (int32 i = 1; i <= nsurf; i++) {
        if (!same_img(i)) { char m[64]; sprintf(m, "surface %d differs", i); fail(m); return; }
        if (legacy::img[i].text) {
            int32 cells = legacy::img[i].width * legacy::img[i].height;
            auto &lc = legacy::qb_text_utf8_cells[i];
            for (int32 c = 0; c < cells; c++) {
                uint8 buf[256];
                int32 n = qb_text_cell_utf8(i, c, buf, 256);
                if (n != (int32)lc[c].size() || memcmp(buf, lc[c].data(), n)) { fail("utf8 cell content"); return; }
                if (qb_text_cells_span(i, c) != legacy::qb_text_cell_span(i, c)) { fail("utf8 cell span"); return; }
            }
        }
    }
    if (legacy::last_error != rust_err) { char m[64]; sprintf(m, "error %d vs %d", legacy::last_error, rust_err); fail(m); }
    legacy::last_error = rust_err = 0;
    if (legacy::key_updates != r_key_updates) fail("KEY bar redraws");
}

static std::string rand_text() {
    static const char *utf[] = {"\xC3\xA9", "\xE0\xB8\x81", "\xE0\xB9\x89", "\xE4\xB8\x80", "\xF0\x9F\x98\x80", "\xCC\x81", "\xE2\x96\x91", "\xEF\xBC\xA1"};
    static const uint8 ctl[] = {7, 9, 10, 11, 12, 13, 28, 29, 30, 31};
    std::string s;
    int32 n = rnd() % 14;
    for (int32 i = 0; i < n; i++) {
        uint32 r = rnd() % 100;
        if (r < 66) s += (char)(32 + rnd() % 95);
        else if (r < 74) s += (char)ctl[rnd() % sizeof(ctl)];
        else if (r < 82) s += (char)(0x80 + rnd() % 128);
        else s += utf[rnd() % 8];
    }
    return s;
}

static void step(int32 nsurf) {
    uint32 op = rnd() % 100;
    int32 a = (int32)(rnd() % 30) - 2, b = (int32)(rnd() % 30) - 2, c = (int32)(rnd() % 3), d = (int32)(rnd() % 34) - 1, e = (int32)(rnd() % 34) - 1;
    int32 passed = rnd() % 32;
    if (op < 6) {
        op_name = "select";
        int32 i = 1 + rnd() % nsurf, j = rnd() % 4 ? i : 1 + rnd() % nsurf;
        legacy::write_page = &legacy::img[i]; legacy::write_page_index = i; r_dest = i;
        legacy::read_page = &legacy::img[j]; legacy::read_page_index = j; r_source = j;
    } else if (op < 40) {
        op_name = "print";
        std::string t = rand_text();
        int32 nl = rnd() % 3 == 0;
        qbs *s = qb_qbs_new_txt_len(t.data(), (int32)t.size());
        legacy::qbs_print(s, nl);
        qb_console_print((const uint8 *)t.data(), (int32)t.size(), nl);
    } else if (op < 48) {
        op_name = "tab";
        legacy::tab(); qb_console_tab();
    } else if (op < 52) {
        op_name = "makefit";
        std::string t = rand_text();
        qbs *s = qb_qbs_new_txt_len(t.data(), (int32)t.size());
        legacy::makefit(s); qb_console_makefit((const uint8 *)t.data(), (int32)t.size());
    } else if (op < 56) {
        op_name = "view print";
        int32 p = rnd() % 4 != 0;
        legacy::qbg_sub_view_print(a, b, p); qb_console_view_print(a, b, p);
    } else if (op < 60) {
        op_name = "cls";
        int32 m = (int32)(rnd() % 4) - (rnd() % 8 == 0);
        uint32 col = rnd() % 4 ? rnd() % 16 : rnd();
        int32 p = rnd() % 4;
        legacy::sub_cls(m, col, p); qb_console_cls(m, col, p);
    } else if (op < 72) {
        op_name = "locate";
        legacy::qbg_sub_locate(a, b, c, d, e, passed); qb_console_locate(a, b, c, d, e, passed);
    } else if (op < 78) {
        op_name = "csrlin/pos";
        if (legacy::func_csrlin() != qb_console_csrlin()) fail("CSRLIN");
        if (legacy::func_pos(0) != qb_console_pos()) fail("POS");
    } else if (op < 86) {
        op_name = "screen()";
        int32 p = rnd() % 2, col = rnd() % 2;
        uint32 l = legacy::func_screen(a, b, col, p), r = qb_console_screen(a, b, col, p);
        if (l != r) fail("SCREEN()");
    } else if (op < 92) {
        op_name = "color";
        img_struct *w = &rimg[r_dest];
        uint32 fg = w->bytes_per_pixel == 4 ? rnd() : rnd() % 32, bg = w->bytes_per_pixel == 4 ? rnd() : rnd() % 16;
        legacy::write_page->color = w->color = fg;
        legacy::write_page->background_color = w->background_color = bg;
        legacy::write_page->print_mode = w->print_mode = 1 + rnd() % 3;
    } else if (op < 94) {
        op_name = "controlchr";
        int32 v = rnd() % 2;
        legacy::no_control_characters2 = v; no_control_characters2 = v;
    } else {
        op_name = "inkey";
        if (rnd() % 2) {
            uint8 k1 = rnd() % 3 ? 32 + rnd() % 90 : 0, k2 = rnd() % 4 ? rnd() % 256 : 0;
            // legacy keydown wrote the ring inline
            int32 i = legacy::cmem[0x41a], i2 = legacy::cmem[0x41c], i3 = i2 + 2;
            if (i3 == 62) i3 = 30;
            if (i != i3) { legacy::cmem[0x400 + i2] = k1; legacy::cmem[0x400 + i2 + 1] = k2; legacy::cmem[0x41c] = i3; }
            qb_inkey_push(r_bios, k1, k2);
        } else {
            qbs *l = legacy::qbs_inkey();
            uint8 out[2]; int32 n = qb_console_inkey(out);
            if (n != l->len || memcmp(out, l->chr, n)) fail("INKEY$");
        }
        if (memcmp(&legacy::cmem[0x41a], &r_bios[0x1a], 4) || memcmp(&legacy::cmem[0x41e], &r_bios[0x1e], 32)) fail("INKEY$ ring");
    }
    if (qbs_tmp_list_nexti > 1) qb_qbs_cleanup(1);
}

// --- INPUT ---
struct VarType { int32 type; int32 bytes; };
static const VarType vtypes[] = {
    {8, 1}, {16, 2}, {32, 4}, {8 | 268435456, 1}, {16 | 268435456, 2}, {32 | 268435456, 4},
    {64, 8}, {64 | 268435456, 8}, {32 | 536870912, 4}, {64 | 536870912, 8}, {256 | 536870912, 10},
    {1073741824, 0}, {1073741824 | 1, -4}};

static void input_case(int32 surf) {
    op_name = "INPUT";
    int32 n = 1 + rnd() % 3;
    bool line = rnd() % 6 == 0;
    if (line) n = 1;
    int32 types[2][257]; void *offs[2][257];
    static uint8 store[2][4][32];
    qbs *strs[2][4];
    static uint8 fixed[2][4][4];
    int32 kinds[4];
    for (int32 v = 1; v <= n; v++) {
        int32 k = line ? 11 : rnd() % 13;
        kinds[v] = k;
        for (int s = 0; s < 2; s++) {
            types[s][v] = vtypes[k].type | (line ? 512 : 0);
            memset(store[s][v], 0xAB, 32);
            if (k == 11) { strs[s][v] = qb_qbs_new(0, 0); offs[s][v] = strs[s][v]; }
            else if (k == 12) { strs[s][v] = qb_qbs_new_fixed(fixed[s][v], 4, 0); offs[s][v] = strs[s][v]; types[s][v] = 1073741824; }
            else offs[s][v] = store[s][v];
        }
    }
    static const char keys[] = "0123456789000-+.,,eEdD&HhOoBb\" abcxyz\x08\x08\x08\x09";
    std::string script;
    int32 len = rnd() % 16;
    // legacy reads an uninitialised "max" for &H/&O/&B entries of _INTEGER64 variables
    // (D-R12a-1): no '&' in entries for them
    bool has64 = false;
    for (int32 v = 1; v <= n; v++) has64 |= kinds[v] == 6 || kinds[v] == 7;
    static const char *tokens[] = {"0.0000000001", "0.00000000000000000000000000000000000000000000012", "&HFF", "&H1FF",
        "&HFFFF", "&HFFFFFFFF", "&O377", "&O777", "&B11111111", "&B111111111", "1E38", "3.402823E+38", "3.402824E38",
        "1.401298E-45", "1.401297E-45", "-32768", "32767", "32768", "65535", "-2147483648", "4294967295",
        "1.5D300", "1.7976931D308", "1.7976932D308", "4.940656D-324", "9223372036854775807", "-9223372036854775808",
        "18446744073709551615", "12345678", "123456789", "1234567890123456", "12345678901234567", ".5", "-.25E-3", "0.",
        "00", "1E", "1E-", "2E+", "1e5", "7d2", ",", "\"a,b\"", "\"x\"y"};
    for (int32 i = 0; i < len; i++) {
        if (rnd() % 3 == 0) {
            const char *t = tokens[rnd() % (sizeof(tokens) / sizeof(tokens[0]))];
            for (; *t; t++) script += (has64 && *t == '&') ? '1' : *t;
            continue;
        }
        char ch = keys[rnd() % (sizeof(keys) - 1)];
        if (has64 && ch == '&') ch = '1';
        script += ch;
    }
    script += '\r';
    keyscript[0] = (uint8 *)script.data(); keyscript[1] = (uint8 *)script.data();
    keyscript_len[0] = keyscript_len[1] = (int32)script.size();
    keyscript_pos[0] = keyscript_pos[1] = 0;
    legacy::write_page = legacy::read_page = &legacy::img[surf]; legacy::write_page_index = legacy::read_page_index = surf;
    r_dest = r_source = surf;
    memcpy(legacy::qbs_input_variabletypes, types[0], sizeof(types[0]));
    memcpy(legacy::qbs_input_variableoffsets, offs[0], sizeof(offs[0]));
    int32 nl = rnd() % 4 != 0;
    legacy::qbs_input(n, nl);
    qb_console_input(offs[1], types[1], n, nl);
    for (int32 v = 1; v <= n; v++) {
        int32 k = kinds[v];
        if (k >= 11) {
            qbs *a = strs[0][v], *b = strs[1][v];
            if (a->len != b->len || memcmp(a->chr, b->chr, a->len)) fail("INPUT string variable");
        } else if (memcmp(store[0][v], store[1][v], 32)) {
            char m[400]; int o = sprintf(m, "INPUT variable %d/%d type %d, keys \"%s\":", v, n, vtypes[k].type, script.c_str());
            for (int q = 0; q < 12; q++) o += sprintf(m + o, " %02x/%02x", store[0][v][q], store[1][v][q]);
            fail(m);
        }
    }
    if (legacy::qbs_input_variabletypes[1] != types[1][1]) fail("LINE INPUT type marker");
    for (int s = 0; s < 2; s++)
        for (int32 v = 1; v <= n; v++)
            if (kinds[v] >= 11) qb_qbs_free(strs[s][v]);
    for (int32 i = 8; i < MAXIMG; i++) { legacy::img[i].valid = 0; rimg[i].valid = 0; }
}

static bool load(const char *path, uint8 *dst, size_t n) {
    FILE *f = fopen(path, "rb");
    if (!f) return false;
    bool ok = fread(dst, 1, n, f) == n;
    fclose(f);
    return ok;
}

int main(int argc, char **argv) {
    long iters = 400000, inputs = 20000;
    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "--quick")) { iters = 40000; inputs = 2000; }
        if (!strcmp(argv[i], "--ops") && i + 2 < argc) { iters = atol(argv[i + 1]); inputs = atol(argv[i + 2]); }
    }
    if (!load("../../crates/qb-runtime/src/gfx/data/charset8.bin", charset8x8_data, sizeof(charset8x8_data)) ||
        !load("../../crates/qb-runtime/src/gfx/data/charset16.bin", charset8x16_data, sizeof(charset8x16_data))) {
        printf("diff_console: charset data not found (run from tests/diff)\n");
        return 1;
    }
    dblock = (ptrszint)&cmem + 1280;
    qb_qbs_reset();
    qb_console_set_host(&host_table);
    legacy::img = (img_struct *)calloc(MAXIMG, sizeof(img_struct));
    legacy::fontwidth[8] = legacy::fontwidth[14] = legacy::fontwidth[16] = 8;
    legacy::fontheight[8] = 8; legacy::fontheight[14] = 14; legacy::fontheight[16] = 16;
    legacy::fontwidth[32] = 0; legacy::fontheight[32] = 12;
    legacy::singlespace = qb_qbs_new(1, 0); legacy::singlespace->chr[0] = 32;
    legacy::cmem[0x41a] = 30; legacy::cmem[0x41c] = 30;
    qb_inkey_clear(r_bios);
    const int32 nsurf = 5;
    for (int32 i = 1; i <= nsurf; i++) make(i, i);
    legacy::write_page = legacy::read_page = &legacy::img[1]; legacy::write_page_index = legacy::read_page_index = 1;
    for (op_no = 0; op_no < iters && mismatches < 25; op_no++) {
        if (trace) { fprintf(stderr, "op %ld\n", op_no); fflush(stderr); }
        step(nsurf);
        compare_all(nsurf);
    }
    long checked = op_no;
    for (long k = 0; k < inputs && mismatches < 25; k++, op_no++) {
        input_case(rnd() % 3 ? 1 : 3);
        compare_all(nsurf);
    }
    printf("diff_console: %ld operations, %ld INPUT entries, %ld mismatches\n", checked, op_no - checked, mismatches);
    return mismatches ? 1 : 0;
}
