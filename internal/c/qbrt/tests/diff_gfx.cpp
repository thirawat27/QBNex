// Differential test: legacy software graphics (tests/legacy/gfx_*.inc, verbatim
// from libqb.cpp) vs Rust qbrt::gfx. Every operation runs on twin surfaces and
// the results must be identical: return values, error codes, every img_struct
// field and every pixel/palette byte.
// x86/x86-64 only (the legacy rounding is x87 inline assembly).
//
//   g++ -O2 -std=c++17 -w internal/c/qbrt/tests/diff_gfx.cpp <libqbrt.a> <native libs> -o diff_gfx
//   ./diff_gfx [--quick] [--mutate]
//
// --mutate perturbs the parameters passed to the Rust side only; the run must
// then report mismatches for every primitive (proves the harness can fail).

#include <windows.h>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <vector>
#include <string>
#include <memory>
#include <algorithm>
#include <cstdarg>

typedef int8_t int8;
typedef uint8_t uint8;
typedef int16_t int16;
typedef uint16_t uint16;
typedef int32_t int32;
typedef uint32_t uint32;
typedef int64_t int64;
typedef uint64_t uint64;
#define QBNex_WINDOWS

// Verbatim img_struct from internal/c/common.h.
struct img_struct{
    void *lock_offset;
    int64 lock_id;
    uint8 valid;//0,1 0=invalid
    uint8 text;//if set, surface is a text surface
    uint8 console;//dummy surface to absorb unimplemented console functionality
    uint16 width,height;
    uint8 bytes_per_pixel;//1,2,4
    uint8 bits_per_pixel;//1,2,4,8,16(text),32
    uint32 mask;//1,3,0xF,0xFF,0xFFFF,0xFFFFFFFF
    uint16 compatible_mode;//0,1,2,7,8,9,10,11,12,13,32,256
    uint32 color,background_color,draw_color;
    uint32 font;//8,14,16,?
    int16 top_row,bottom_row;//VIEW PRINT settings, unique (as in QB) to each "page"
    int16 cursor_x,cursor_y;//unique (as in QB) to each "page"
    uint8 cursor_show, cursor_firstvalue, cursor_lastvalue;
    union{
        uint8 *offset;
        uint32 *offset32;
    };
    uint32 flags;
    uint32 *pal;
    int32 transparent_color;//-1 means no color is transparent
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
// Must match internal/c/qbrt/src/gfx/mod.rs (ImgStruct).
static_assert(offsetof(img_struct, width) == 20, "layout");
static_assert(offsetof(img_struct, offset) == 64, "layout");
static_assert(offsetof(img_struct, pal) == 80, "layout");
static_assert(offsetof(img_struct, view_x1) == 96, "layout");
static_assert(offsetof(img_struct, clipping_or_scaling) == 128, "layout");
static_assert(offsetof(img_struct, draw_ta) == 168, "layout");
static_assert(offsetof(img_struct, apm_p2) == 184, "layout");
static_assert(sizeof(img_struct) == 192, "layout");
#define IMG_FREEPAL 1 //free palette data before freeing image (common.h)
#define IMG_SCREEN 2 //img is linked to other screen pages
#define IMG_FREEMEM 4 //if set, it means memory must be freed

namespace legacy {
struct qbs { uint8 *chr; int32 len; }; // the only fields the legacy code reads
static img_struct *write_page = NULL;
static img_struct *read_page = NULL;
static int32 new_error = 0;
static int32 last_error = 0;
static void error(int32 n) { if (!last_error) last_error = n; }
static uint32 palette_256[256];
static uint32 palette_64[64];
static int32 qbg_text_only = 0;
static uint8 *qbg_active_page_offset;
static int32 qbg_width, qbg_view_x1, qbg_view_y1, qbg_view_x2, qbg_view_y2;
// libqb.cpp line 131 (verbatim)
uint32 rotateLeft(uint32 word,uint32 shift){
return (word << shift) | (word >> (32 - shift));
}
#include "legacy/qbr_x87.inc"
#include "legacy/gfx_data.inc"
#include "legacy/gfx_blend.inc"
#include "legacy/gfx_restorepalette_pset.inc"
#include "legacy/gfx_lineclip.inc"
#include "legacy/gfx_palette_color.inc"
#include "legacy/gfx_primitives.inc"
#include "legacy/gfx_window_view.inc"
#include "legacy/gfx_pmap.inc"
// --- registry (newimg/freeimg/grow/imgframe/imgnew/imgrevert) and image ops ---
typedef intptr_t ptrszint;
struct mem_lock { int dummy; };
static void free_mem_lock(mem_lock *) {}
static void qb_init_text_utf8_cells(int32) {}
static void qb_release_text_utf8_cells(int32) {}
static void refresh_selected_pages() {}
static void sub__freeimage(int32, int32) {}
static std::vector<int32> page(1, 0);
static uint32 write_page_index = 0, read_page_index = 0;
inline int32 resolve_page_or_image_index(int32 handle, int32 &resolved_index) { resolved_index = -handle; return -1; }
using std::swap;
#include "legacy/gfx_registry.inc"
#include "legacy/gfx_imgops.inc"
} // namespace legacy

#include "../include/qbrt.h"

// ---------------------------------------------------------------------------
static uint64_t rng = 0x9E3779B97F4A7C15ull;
static uint64_t next() {
    rng ^= rng << 13;
    rng ^= rng >> 7;
    rng ^= rng << 17;
    return rng;
}
static int32 rint_(int32 lo, int32 hi) { return lo + (int32)(next() % (uint64)(hi - lo + 1)); }
static double runit() { return (double)(next() >> 11) / 9007199254740992.0; }
static float rcoord(int32 size) {
    // mostly inside, sometimes far outside, often fractional (x.5 to hit ties)
    switch (next() % 6) {
    case 0: return (float)rint_(-size, 2 * size);
    case 1: return (float)rint_(0, size) + 0.5f;
    case 2: return (float)(runit() * 3 * size - size);
    case 3: return (float)rint_(-100000, 100000);
    default: return (float)(runit() * size);
    }
}

static bool quick = false, mutate = false;
static long failures = 0, checks = 0;
static std::vector<std::string> mutated_ops;

struct Twin {
    img_struct L, R; // legacy, rust
    std::vector<uint8> lpix, rpix;
    std::vector<uint32> lpal, rpal;
};

// Property setup equivalent to legacy imgframe() (newimg/registry parts left out).
static void setup(img_struct &im, int w, int h, int bpp) {
    memset(&im, 0, sizeof(im));
    im.valid = 1;
    im.width = w; im.height = h;
    im.bytes_per_pixel = 1; im.font = 16; im.color = 15; im.compatible_mode = bpp; im.print_mode = 3;
    im.draw_ta = 0.0; im.draw_scale = 1.0;
    switch (bpp) {
    case 1: im.bits_per_pixel = 2; im.font = 8; im.color = 3; break;
    case 2: im.bits_per_pixel = 1; im.font = 8; im.color = 1; break;
    case 7: case 8: im.bits_per_pixel = 4; im.font = 8; break;
    case 9: im.bits_per_pixel = 4; im.font = 14; break;
    case 10: im.bits_per_pixel = 2; im.font = 14; im.color = 3; break;
    case 11: im.bits_per_pixel = 1; im.color = 1; break;
    case 12: im.bits_per_pixel = 4; break;
    case 13: im.bits_per_pixel = 8; im.font = 8; break;
    case 256: im.bits_per_pixel = 8; break;
    case 32: im.bits_per_pixel = 32; im.bytes_per_pixel = 4; im.color = 0xFFFFFFFF; im.background_color = 0xFF000000; break;
    }
    im.draw_color = im.color;
    im.transparent_color = bpp == 32 ? 0 : -1;
    if (im.bits_per_pixel < 32) im.mask = (1 << im.bits_per_pixel) - 1; else im.mask = 0xFFFFFFFF;
    im.cursor_x = 1; im.cursor_y = 1; im.top_row = 1;
    im.bottom_row = im.height / im.font; im.bottom_row--; if (im.bottom_row <= 0) im.bottom_row = 1;
    im.x = ((double)im.width) / 2.0; im.y = ((double)im.height) / 2.0;
    im.view_x2 = im.width - 1; im.view_y2 = im.height - 1;
    im.scaling_x = 1; im.scaling_y = 1;
    im.window_x2 = im.view_x2; im.window_y2 = im.view_y2;
}

static void make_twin(Twin &t, int w, int h, int bpp) {
    setup(t.L, w, h, bpp);
    size_t n = (size_t)w * h * t.L.bytes_per_pixel;
    t.lpix.assign(n + 16, 0);
    t.lpal.assign(256, 0);
    for (size_t i = 0; i < n; i++) t.lpix[i] = (next() % 4 == 0) ? (uint8)next() : 0; // some noise
    t.L.offset = t.lpix.data();
    t.L.pal = bpp == 32 ? NULL : t.lpal.data();
    if (bpp != 32) legacy::restorepalette(&t.L);
    t.R = t.L;
    t.rpix = t.lpix;
    t.rpal = t.lpal;
    t.R.offset = t.rpix.data();
    t.R.pal = bpp == 32 ? NULL : t.rpal.data();
}

#define F(f) if (memcmp(&a.f, &b.f, sizeof(a.f))) { snprintf(buf, sizeof buf, "field " #f); return buf; }
static const char *diff_state(const Twin &t) {
    static char buf[128];
    const img_struct &a = t.L, &b = t.R;
    F(valid) F(text) F(width) F(height) F(bytes_per_pixel) F(bits_per_pixel) F(mask) F(compatible_mode)
    F(color) F(background_color) F(draw_color) F(font) F(top_row) F(bottom_row) F(cursor_x) F(cursor_y)
    F(flags) F(transparent_color) F(alpha_disabled) F(print_mode)
    F(view_x1) F(view_y1) F(view_x2) F(view_y2) F(view_offset_x) F(view_offset_y) F(x) F(y)
    F(clipping_or_scaling) F(scaling_x) F(scaling_y) F(scaling_offset_x) F(scaling_offset_y)
    F(window_x1) F(window_y1) F(window_x2) F(window_y2) F(draw_ta) F(draw_scale)
    if (t.lpix != t.rpix) {
        for (size_t i = 0; i < t.lpix.size(); i++)
            if (t.lpix[i] != t.rpix[i]) { snprintf(buf, sizeof buf, "pixel byte %zu (legacy %u rust %u)", i, t.lpix[i], t.rpix[i]); return buf; }
    }
    if (t.lpal != t.rpal) {
        for (size_t i = 0; i < t.lpal.size(); i++)
            if (t.lpal[i] != t.rpal[i]) { snprintf(buf, sizeof buf, "palette %zu (legacy %08x rust %08x)", i, t.lpal[i], t.rpal[i]); return buf; }
    }
    return NULL;
}

static void report(const char *op, const Twin &t, const char *why, const std::string &args) {
    ++failures;
    if (failures <= 25)
        printf("MISMATCH %s mode=%d %dx%d: %s  [%s]\n", op, t.L.compatible_mode, t.L.width, t.L.height, why, args.c_str());
}

static void check(const char *op, Twin &t, int32 lerr, int32 rerr, const std::string &args) {
    ++checks;
    if (lerr != rerr) {
        char b[64];
        snprintf(b, sizeof b, "error legacy=%d rust=%d", lerr, rerr);
        report(op, t, b, args);
        return;
    }
    if (const char *why = diff_state(t)) report(op, t, why, args);
}

static uint32 rcolor(const Twin &t) {
    if (t.L.bytes_per_pixel == 4) {
        static const uint32 alphas[] = {0, 127, 128, 255, 255, 1, 254};
        uint32 a = (next() % 3) ? alphas[next() % 7] : (uint32)(next() & 255);
        return (a << 24) | (uint32)(next() & 0xFFFFFF);
    }
    return (next() % 8) ? (uint32)(next() % (t.L.mask + 1)) : (uint32)next();
}

static std::string fmt(const char *f, ...) {
    char b[512];
    va_list ap;
    va_start(ap, f);
    vsnprintf(b, sizeof b, f, ap);
    va_end(ap);
    return b;
}

static float mf(float v) { return mutate ? v + 1.0f : v; }

static void op_line(Twin &t) {
    float x1 = rcoord(t.L.width), y1 = rcoord(t.L.height), x2 = rcoord(t.L.width), y2 = rcoord(t.L.height);
    uint32 col = rcolor(t), style = (next() % 3) ? 0xFFFF : (uint32)next();
    int32 bf = rint_(0, 2), passed = (int32)(next() & 31);
    if (next() % 4) passed |= 1;
    legacy::last_error = 0;
    legacy::write_page = &t.L;
    legacy::sub_line(x1, y1, x2, y2, col, bf, style, passed);
    int32 r = qbrt_gfx_line(&t.R, mf(x1), y1, x2, y2, col, bf, style, passed);
    check("LINE", t, legacy::last_error, r, fmt("%.9g,%.9g,%.9g,%.9g col=%08x bf=%d style=%x passed=%d", x1, y1, x2, y2, col, bf, style, passed));
}

static void op_circle(Twin &t) {
    double x = rcoord(t.L.width), y = rcoord(t.L.height);
    double r = (next() % 5) ? runit() * t.L.width : runit() * 3000;
    double pi2 = 6.2831853071795865;
    double start = runit() * 2.4 * pi2 - 1.2 * pi2, end = runit() * 2.4 * pi2 - 1.2 * pi2;
    if (next() % 2) start = (next() % 2) ? 0 : -start;
    double aspect = (next() % 3) ? runit() * 6 - 3 : (double)rint_(-2, 3);
    uint32 col = rcolor(t);
    int32 passed = (int32)(next() & 31);
    legacy::last_error = 0;
    legacy::write_page = &t.L;
    legacy::sub_circle(x, y, r, col, start, end, aspect, passed);
    int32 e = qbrt_gfx_circle(&t.R, x, y, mutate ? r + 1 : r, col, start, end, aspect, passed);
    check("CIRCLE", t, legacy::last_error, e, fmt("%.17g,%.17g r=%.17g col=%08x start=%.17g end=%.17g aspect=%.17g passed=%d", x, y, r, col, start, end, aspect, passed));
}

static void op_paint(Twin &t) {
    float x = rcoord(t.L.width), y = rcoord(t.L.height);
    if (next() % 3) { x = (float)rint_(0, t.L.width - 1); y = (float)rint_(0, t.L.height - 1); }
    uint32 fill = rcolor(t), border = rcolor(t);
    int32 passed = (int32)(next() & 7);
    if (next() % 20 == 0) passed |= 8;
    legacy::last_error = 0;
    legacy::write_page = &t.L;
    legacy::sub_paint(x, y, fill, border, (legacy::qbs *)NULL, passed);
    int32 e = qbrt_gfx_paint(&t.R, mf(x), y, fill, border, passed);
    check("PAINT", t, legacy::last_error, e, fmt("%.9g,%.9g fill=%08x border=%08x passed=%d", x, y, fill, border, passed));
}

static void op_paint_tile(Twin &t) {
    float x = (float)rint_(0, t.L.width - 1), y = (float)rint_(0, t.L.height - 1);
    if (next() % 5 == 0) { x = rcoord(t.L.width); y = rcoord(t.L.height); }
    uint8 tile[80];
    // mode 2 (8 columns) is limited to 64 rows: longer strings overrun the legacy
    // static tile[8][64] (undefined behaviour, see paint.rs)
    int32 len = (next() % 10 == 0) ? 0 : rint_(1, (next() % 8) ? 16 : (t.L.compatible_mode == 2 ? 64 : 80));
    for (int i = 0; i < len; i++) tile[i] = (uint8)next();
    uint32 border = (next() % 3) ? 0 : rcolor(t);
    int32 passed = (int32)(next() & 7);
    if (next() % 4) passed |= 2;
    legacy::qbs s = {tile, len};
    legacy::last_error = 0;
    legacy::write_page = &t.L;
    legacy::sub_paint(x, y, &s, border, (legacy::qbs *)NULL, passed);
    int32 e = qbrt_gfx_paint_tile(&t.R, mf(x), y, tile, len, border, passed);
    check("PAINT tile", t, legacy::last_error, e, fmt("%.9g,%.9g len=%d border=%08x passed=%d", x, y, len, border, passed));
}

static void op_pset(Twin &t) {
    float x = rcoord(t.L.width), y = rcoord(t.L.height);
    uint32 col = rcolor(t);
    int32 passed = (int32)(next() & 3);
    bool pre = next() % 3 == 0;
    legacy::last_error = 0;
    legacy::write_page = &t.L;
    int32 e;
    if (pre) { legacy::sub_preset(x, y, col, passed); e = qbrt_gfx_preset_stmt(&t.R, mf(x), y, col, passed); }
    else { legacy::sub_pset(x, y, col, passed); e = qbrt_gfx_pset_stmt(&t.R, mf(x), y, col, passed); }
    check(pre ? "PRESET" : "PSET", t, legacy::last_error, e, fmt("%.9g,%.9g col=%08x passed=%d", x, y, col, passed));
}

static void op_point(Twin &t) {
    float x = rcoord(t.L.width), y = rcoord(t.L.height);
    int32 passed = (next() % 4) ? 1 : 0;
    if (!passed) x = (float)rint_(-1, 4) + ((next() % 4) ? 0.0f : 0.5f);
    legacy::last_error = 0;
    legacy::write_page = legacy::read_page = &t.L;
    double a = legacy::func_point(x, y, passed);
    int32 err = 0;
    double b = qbrt_gfx_point(&t.R, &t.R, mf(x), y, passed, &err);
    if (memcmp(&a, &b, 8)) { ++checks; report("POINT", t, "value", fmt("%.9g,%.9g passed=%d legacy=%.17g rust=%.17g", x, y, passed, a, b)); return; }
    check("POINT", t, legacy::last_error, err, fmt("%.9g,%.9g passed=%d", x, y, passed));
}

static void op_window(Twin &t) {
    float x1 = rcoord(t.L.width), y1 = rcoord(t.L.height), x2 = rcoord(t.L.width), y2 = rcoord(t.L.height);
    if (next() % 8 == 0) { x1 = 0; y1 = 0; x2 = t.L.width - 1; y2 = t.L.height - 1; }
    if (next() % 10 == 0) x2 = x1;
    int32 passed = (int32)(next() % 4);
    legacy::last_error = 0;
    legacy::write_page = &t.L;
    legacy::qbg_sub_window(x1, y1, x2, y2, passed);
    int32 e = qbrt_gfx_window(&t.R, mf(x1), y1, x2, y2, passed);
    check("WINDOW", t, legacy::last_error, e, fmt("%.9g,%.9g,%.9g,%.9g passed=%d", x1, y1, x2, y2, passed));
}

static void op_view(Twin &t) {
    int32 x1 = rint_(-2, t.L.width + 1), y1 = rint_(-2, t.L.height + 1), x2 = rint_(-2, t.L.width + 1), y2 = rint_(-2, t.L.height + 1);
    int32 fill = (int32)rcolor(t), border = (int32)rcolor(t);
    int32 passed = (int32)(next() % 16);
    if (next() % 6 == 0) passed = 0;
    legacy::last_error = 0;
    legacy::write_page = &t.L;
    legacy::qbg_sub_view(x1, y1, x2, y2, fill, border, passed);
    int32 e = qbrt_gfx_view(&t.R, mutate ? x1 + 1 : x1, y1, x2, y2, fill, border, passed);
    check("VIEW", t, legacy::last_error, e, fmt("%d,%d,%d,%d fill=%08x border=%08x passed=%d", x1, y1, x2, y2, fill, border, passed));
}

static void op_pmap(Twin &t) {
    float v = rcoord(t.L.width);
    int32 option = rint_(-1, 4);
    legacy::last_error = 0;
    legacy::write_page = &t.L;
    float a = legacy::func_pmap(v, option);
    int32 err = 0;
    float b = qbrt_gfx_pmap(&t.R, mf(v), option, &err);
    ++checks;
    if (memcmp(&a, &b, 4) || err != legacy::last_error)
        report("PMAP", t, "value/error", fmt("%.9g option=%d legacy=%.9g/%d rust=%.9g/%d", v, option, a, legacy::last_error, b, err));
}

static void op_palette(Twin &t) {
    uint32 attr = (next() % 4) ? (uint32)rint_(0, 20) : (uint32)next();
    uint32 col = (next() % 2) ? (uint32)rint_(0, 70) : (uint32)(next() & 0x3F3F3F) | ((next() % 5 == 0) ? 0x40 : 0);
    int32 passed = (next() % 10) ? 1 : 0;
    legacy::last_error = 0;
    legacy::write_page = &t.L;
    legacy::qbg_palette(attr, col, passed);
    int32 e = qbrt_gfx_palette(&t.R, attr, mutate ? col ^ 1 : col, passed);
    check("PALETTE", t, legacy::last_error, e, fmt("attr=%u col=%08x passed=%d", attr, col, passed));
}

static void op_color(Twin &t) {
    uint32 c1 = (next() % 4) ? (uint32)rint_(0, 300) : (uint32)next();
    uint32 c2 = (next() % 4) ? (uint32)rint_(0, 70) : (uint32)next();
    int32 passed = (int32)(next() % 8);
    legacy::last_error = 0;
    legacy::write_page = &t.L;
    legacy::qbg_sub_color(c1, c2, 0, passed);
    int32 e = qbrt_gfx_color(&t.R, mutate ? c1 ^ 1 : c1, c2, passed);
    check("COLOR", t, legacy::last_error, e, fmt("c1=%u c2=%u passed=%d", c1, c2, passed));
}

static void op_defaults(Twin &t) {
    legacy::write_page = &t.L;
    legacy::defaultcolors();
    qbrt_gfx_defaultcolors(&t.R);
    if (mutate) t.R.color ^= 1;
    check("defaultcolors", t, 0, 0, "");
    legacy::restorepalette(&t.L);
    qbrt_gfx_restorepalette(&t.R);
    check("restorepalette", t, 0, 0, "");
}

// --- image ops: the legacy functions run on legacy::img[write_page_index] ----
static img_struct *legacy_registry_base;

static void op_imgops(Twin &t) {
    legacy::img = &t.L;
    legacy::write_page_index = 0;
    legacy::write_page = &t.L;
    int k = (int)(next() % 7);
    int32 r = rint_(-20, 280), g = rint_(-20, 280), b = rint_(-20, 280), a = rint_(-20, 280);
    uint32 col = (next() % 2) ? rcolor(t) : (uint32)rint_(0, 300);
    int32 n = rint_(-2, 258);
    legacy::last_error = 0;
    std::string args = fmt("k=%d r=%d g=%d b=%d a=%d col=%08x n=%d", k, r, g, b, a, col, n);
    uint32 lv = 0, rv = 0;
    int32 re = 0;
    switch (k) {
    case 0: lv = legacy::func__rgb(r, g, b, 0, 0); rv = qbrt_gfx_rgb(&t.R, mutate ? r + 9 : r, g, b); break;
    case 1: lv = legacy::func__rgba(r, g, b, a, 0, 0); rv = qbrt_gfx_rgba(&t.R, mutate ? r + 9 : r, g, b, a); break;
    case 2: {
        int which = rint_(0, 3);
        args += fmt(" which=%d", which);
        lv = which == 0 ? legacy::func__red(col, 0, 0) : which == 1 ? legacy::func__green(col, 0, 0)
           : which == 2 ? legacy::func__blue(col, 0, 0) : legacy::func__alpha(col, 0, 0);
        rv = (uint32)qbrt_gfx_component(&t.R, mutate ? col ^ 0x10101 : col, which, &re);
        break;
    }
    case 3: {
        int32 passed = (int32)(next() % 8) & ~2;
        uint32 c2 = (next() % 4) ? rcolor(t) : col;
        int32 alpha = (next() % 10) ? rint_(0, 255) : rint_(-5, 300);
        args += fmt(" passed=%d c2=%08x alpha=%d", passed, c2, alpha);
        legacy::sub__setalpha(alpha, col, c2, 0, passed);
        re = qbrt_gfx_setalpha(&t.R, mutate ? alpha ^ 1 : alpha, col, c2, passed);
        break;
    }
    case 4: {
        int32 passed = (int32)(next() % 4);
        args += fmt(" passed=%d", passed);
        legacy::sub__clearcolor(col, 0, passed);
        re = qbrt_gfx_clearcolor(&t.R, mutate ? col ^ 1 : col, passed);
        break;
    }
    case 5: lv = legacy::func__palettecolor(n, 0, 0); rv = qbrt_gfx_palettecolor_get(&t.R, mutate ? n ^ 1 : n, &re); break;
    case 6:
        legacy::sub__palettecolor(n, col, 0, 0);
        re = qbrt_gfx_palettecolor_set(&t.R, n, mutate ? col ^ 1 : col);
        break;
    }
    legacy::img = legacy_registry_base;
    ++checks;
    if (lv != rv) { report("image op", t, "value", args + fmt(" legacy=%08x rust=%08x", lv, rv)); return; }
    check("image op", t, legacy::last_error, re, args);
}

// --- registry: legacy newimg/freeimg/imgframe/imgnew/imgrevert vs Rust ---------
static std::vector<std::vector<uint8>> external_buffers;

static const char *diff_slot(int i) {
    static char buf[160];
    const img_struct &a = legacy::img[i], &b = img[i];
    F(valid) F(text) F(width) F(height) F(bytes_per_pixel) F(bits_per_pixel) F(mask) F(compatible_mode)
    F(color) F(background_color) F(draw_color) F(font) F(top_row) F(bottom_row) F(cursor_x) F(cursor_y)
    F(cursor_show) F(cursor_firstvalue) F(cursor_lastvalue)
    F(flags) F(transparent_color) F(alpha_disabled) F(print_mode)
    F(view_x1) F(view_y1) F(view_x2) F(view_y2) F(view_offset_x) F(view_offset_y) F(x) F(y)
    F(clipping_or_scaling) F(scaling_x) F(scaling_y) F(scaling_offset_x) F(scaling_offset_y)
    F(window_x1) F(window_y1) F(window_x2) F(window_y2) F(draw_ta) F(draw_scale)
    if ((a.offset == NULL) != (b.offset == NULL)) return "offset null-ness";
    if ((a.pal == NULL) != (b.pal == NULL)) return "pal null-ness";
    if (!(a.flags & 4) && a.offset != b.offset) return "external offset";
    if (a.offset && a.valid && (a.flags & 4)) {
        size_t n = (size_t)a.width * a.height * a.bytes_per_pixel;
        if (memcmp(a.offset, b.offset, n)) return "pixels";
    }
    if (a.pal && b.pal && memcmp(a.pal, b.pal, 1024)) return "palette";
    return NULL;
}

static void check_registry(const std::string &args, int32 lr, int32 rr, int32 le, int32 re) {
    ++checks;
    const char *why = NULL;
    char b[200];
    if (lr != rr) { snprintf(b, sizeof b, "result legacy=%d rust=%d", lr, rr); why = b; }
    else if (le != re) { snprintf(b, sizeof b, "error legacy=%d rust=%d", le, re); why = b; }
    else if (legacy::nimg != nimg || legacy::nextimg != nextimg) {
        snprintf(b, sizeof b, "nimg/nextimg %d/%d vs %d/%d", legacy::nimg, legacy::nextimg, nimg, nextimg);
        why = b;
    } else {
        for (int i = 0; i < legacy::nextimg && !why; i++) {
            if (const char *d = diff_slot(i)) { snprintf(b, sizeof b, "slot %d: %s", i, d); why = b; }
        }
    }
    if (why) {
        ++failures;
        if (failures <= 25) printf("MISMATCH registry: %s [%s]\n", why, args.c_str());
    }
}

static void registry_test(long steps) {
    // start-up as in libqb.cpp: indexes 0 and 1 reserved
    memset(legacy::img, 0, IMG_BUFFERSIZE * sizeof(img_struct));
    int32 x = legacy::newimg(); legacy::img[x].valid = 0;
    x = legacy::newimg(); legacy::img[x].valid = 0;
    qbrt_img_init();
    int32 e = 0;
    x = qbrt_img_new(&e); img[x].valid = 0;
    x = qbrt_img_new(&e); img[x].valid = 0;
    static const int modes[] = {0, 1, 2, 7, 8, 9, 10, 11, 12, 13, 256, 32, 3, 33, -1};
    std::vector<int32> live;
    for (long st = 0; st < steps; st++) {
        int op = (int)(next() % 10);
        int32 w = (next() % 8) ? rint_(1, 64) : rint_(-2, 300), h = (next() % 8) ? rint_(1, 48) : rint_(-2, 200);
        int bpp = modes[next() % (sizeof modes / sizeof modes[0])];
        legacy::last_error = 0;
        int32 re = 0, lr = 0, rr = 0;
        std::string args;
        if (op <= 3) {
            args = fmt("imgnew %d,%d,%d", w, h, bpp);
            lr = legacy::imgnew(w, h, bpp);
            rr = qbrt_imgnew(mutate ? w + 1 : w, h, bpp, &re);
            if (lr) live.push_back(lr);
        } else if (op == 4) {
            external_buffers.emplace_back((size_t)std::max(w, 1) * std::max(h, 1) * 4 + 4, (uint8)0);
            uint8 *o = external_buffers.back().data();
            args = fmt("imgframe %d,%d,%d", w, h, bpp);
            lr = legacy::imgframe(o, w, h, bpp);
            rr = qbrt_imgframe(o, w, h, bpp, &re);
            if (lr) live.push_back(lr);
        } else if (op <= 7 && !live.empty()) {
            size_t k = next() % live.size();
            int32 i = live[k];
            live.erase(live.begin() + k);
            args = fmt("free %d", i);
            // legacy sub__freeimage path: release owned resources, then freeimg
            legacy::qb_release_img_owned_resources(i);
            lr = legacy::freeimg(i);
            qbrt_img_release_owned(i);
            if (i < nimg && img[i].valid) { qbrt_img_free(mutate ? i ^ 1 : i); rr = 1; }
        } else if (op == 8 && !live.empty()) {
            int32 i = live[next() % live.size()];
            args = fmt("imgrevert %d", i);
            if (legacy::img[i].compatible_mode != 0 && legacy::img[i].offset) {
                legacy::imgrevert(i);
                qbrt_imgrevert(i);
            }
        } else {
            int32 req = legacy::nimg < 40000 ? legacy::nimg + rint_(-10, 3000) : rint_(0, legacy::nimg);
            args = fmt("grow %d", req);
            lr = legacy::grow_img_registry(req) ? 0 : 502;
            rr = qbrt_img_grow(req);
        }
        legacy_registry_base = legacy::img;
        check_registry(args, lr, rr, legacy::last_error, re);
        if (failures > 25) return;
    }
}

static void check_tables() {
    // Built-in data: legacy start-up code (libqb.cpp main init) vs Rust statics.
    memcpy(&legacy::palette_256, &legacy::file_qbnex_pal[0], legacy::file_qbnex_pal_len);
    for (int i = 0; i < 256; i++) legacy::palette_256[i] |= 0xFF000000;
    memcpy(&legacy::palette_64, &legacy::file_qbnexega_pal[0], legacy::file_qbnexega_pal_len);
    for (int i = 0; i < 64; i++) legacy::palette_64[i] |= 0xFF000000;
    bool ok = memcmp(legacy::palette_256, palette_256, 1024) == 0 && memcmp(legacy::palette_64, palette_64, 256) == 0 &&
              memcmp(legacy::file_charset8_raw, charset8x8, 16384) == 0 && memcmp(legacy::file_chrset16_raw, charset8x16, 32768) == 0;
    const uint32 m10[2][9] = {{0, 0, 0, 0x808080, 0x808080, 0x808080, 0xFFFFFF, 0xFFFFFF, 0xFFFFFF},
                              {0, 0x808080, 0xFFFFFF, 0, 0x808080, 0xFFFFFF, 0, 0x808080, 0xFFFFFF}};
    ok = ok && memcmp(m10, pal_mode10, sizeof m10) == 0;
    ++checks;
    if (!ok) { ++failures; printf("MISMATCH built-in palettes/fonts\n"); }
    legacy::init_blend();
    const uint8 *cb, *ab;
    qbrt_gfx_blend_tables(&cb, &ab);
    ++checks;
    if (memcmp(cb, legacy::cblend, 16777216) || memcmp(ab, legacy::ablend, 65536)) {
        ++failures;
        printf("MISMATCH blend tables\n");
    }
}

int main(int argc, char **argv) {
    legacy::fpu_reinit();
    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "--quick")) quick = true;
        if (!strcmp(argv[i], "--mutate")) mutate = true;
    }
    check_tables();
    legacy_registry_base = legacy::img;
    registry_test(quick ? 3000 : 40000);
    static const int modes[] = {1, 2, 7, 8, 9, 10, 11, 12, 13, 256, 32, 32, 13, 12};
    const long rounds = quick ? 300 : 6000;
    const int ops_per_round = 60;
    long per_op[16] = {0};
    for (long round = 0; round < rounds; round++) {
        Twin t;
        int bpp = modes[next() % (sizeof modes / sizeof modes[0])];
        int w = (next() % 4) ? rint_(1, 120) : rint_(100, 700);
        int h = (next() % 4) ? rint_(1, 100) : rint_(100, 500);
        make_twin(t, w, h, bpp);
        if (bpp == 32 && next() % 4 == 0) { t.L.alpha_disabled = t.R.alpha_disabled = 1; }
        for (int k = 0; k < ops_per_round; k++) {
            int op = (int)(next() % 14);
            per_op[op]++;
            switch (op) {
            case 0: case 1: op_line(t); break;
            case 2: op_circle(t); break;
            case 3: op_paint(t); break;
            case 4: op_paint_tile(t); break;
            case 5: op_pset(t); break;
            case 6: op_point(t); break;
            case 7: op_window(t); break;
            case 8: op_view(t); break;
            case 9: op_pmap(t); break;
            case 10: op_palette(t); op_color(t); break;
            case 11: op_defaults(t); break;
            case 12: case 13: op_imgops(t); break;
            }
            if (diff_state(t)) { // resynchronise after a reported mismatch
                t.R = t.L; t.rpix = t.lpix; t.rpal = t.lpal;
                t.R.offset = t.rpix.data(); t.R.pal = t.L.pal ? t.rpal.data() : NULL;
            }
        }
    }
    printf("diff_gfx: %s%s, %ld checks, %ld mismatches\n", quick ? "quick" : "full", mutate ? " (mutated)" : "", checks, failures);
    return failures == 0 ? 0 : 1;
}
