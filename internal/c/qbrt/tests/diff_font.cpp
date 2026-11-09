// Differential test: legacy FreeType 2.4.9 wrapper (tests/legacy/font_ttf.inc + freetypeamalgam.c from git
// history) vs Rust qbrt::font.
//
//   g++ -O1 -c -w <legacy>/freetypeamalgam.c -o ft.o
//   g++ -O2 -std=c++17 -w -I<legacy> internal/c/qbrt/tests/diff_font.cpp ft.o <libqbrt.a> <libs> -o diff_font
//   ./diff_font [--quick] [--verbose] font.ttf...
//
// What must match exactly (deviation D-R7-6 lists the rest):
//   - load success, handle numbers, _FONTWIDTH of monospace fonts;
//   - the rendered height (= requested pixel height) of every string.
// What is held to a tolerance, because hinting and coverage are not FreeType's:
//   - rendered width and pre-increment: at least WIDTH_EXACT_PERCENT of renders exact; the rest may place a
//     glyph one pixel off (pre-increment +-1, width +-2 per glyph: a shifted bitmap also changes the padding);
//   - anti-aliased pixels (over renders of equal size): mean absolute difference below PIXEL_MEAN_TOLERANCE.
// The harness fails if a font misses a threshold, so a regression in metrics or rasterisation shows up.

#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <string>
#include <vector>

typedef int32_t int32;
typedef uint32_t uint32;
typedef uint8_t uint8;
typedef uint16_t uint16;
typedef int64_t int64;
typedef uint64_t uint64;

#include "freetypeamalgam.h"

#include "legacy/qbr_x87.inc"
static uint16 codepage437_to_unicode16[256];
#include "legacy/font_ttf.inc"

#include "../include/qbrt.h"

static const double WIDTH_EXACT_PERCENT = 94.0;
static const double WIDTH_FAR_PERCENT = 0.5;    // renders outside the +-1 pixel per glyph band
static const double PIXEL_MEAN_TOLERANCE = 16.0; // 0..255 scale, over all pixels of equal-size renders

static uint64_t rng = 0xD1B54A32D192ED03ull;
static uint64_t next() {
    rng ^= rng << 13;
    rng ^= rng >> 7;
    rng ^= rng << 17;
    return rng;
}

struct Stats {
    long renders = 0, width_exact = 0, width_off_by_more = 0, height_bad = 0, fail_bad = 0, compared_px = 0;
    double abs_sum = 0;
    long px_diff = 0;
};

static bool verbose = false;
static int cur_size = 0;

static void compare_render(int32 hl, int32 hr, const std::vector<uint32> &cps, int32 opt, Stats &st, const char *what) {
    uint8 *da = nullptr, *db = nullptr;
    int32 xa = -1, ya = -1, pa = -1, qa = -1, xb = -2, yb = -2, pb = -2, qb = -2;
    int32 ra = FontRenderTextUTF32(hl, (uint32 *)cps.data(), (int32)cps.size(), opt, &da, &xa, &ya, &pa, &qa);
    int32 rb = qbrt_font_render_utf32(hr, cps.data(), (int32)cps.size(), opt, &db, &xb, &yb, &pb, &qb);
    st.renders++;
    if (ra != rb) st.fail_bad++;
    if (ya != yb || qa != qb) st.height_bad++;
    if (xa == xb && pa == pb)
        st.width_exact++;
    else {
        if (abs(xa - xb) > 2 * (int)cps.size() || abs(pa - pb) > 1) st.width_off_by_more++;
        if (verbose)
            printf("  size %d %s cps=%zu first=U+%04X opt=%d: legacy w=%d pre=%d, rust w=%d pre=%d\n", cur_size, what, cps.size(), cps[0],
                   opt, xa, pa, xb, pb);
    }
    if (da && db && xa == xb && ya == yb) {
        for (long i = 0; i < (long)xa * ya; ++i) {
            int d = abs((int)da[i] - (int)db[i]);
            st.abs_sum += d;
            st.px_diff += d != 0;
        }
        st.compared_px += (long)xa * ya;
    }
    free(da);
    free(db);
}

static std::vector<uint8_t> read_file(const char *p) {
    std::ifstream f(p, std::ios::binary);
    return std::vector<uint8_t>((std::istreambuf_iterator<char>(f)), std::istreambuf_iterator<char>());
}

static uint64_t fnv1a(const uint8 *p, size_t n) {
    uint64_t h = 0xcbf29ce484222325ull;
    for (size_t i = 0; i < n; ++i) h = (h ^ p[i]) * 0x100000001b3ull;
    return h;
}

// --dump font size cp...: what the legacy wrapper reports, for Rust unit tests.
static int dump(int argc, char **argv) {
    auto content = read_file(argv[2]);
    int32 size = atoi(argv[3]);
    for (int32 options : {0, 16}) {
        int32 h = FontLoad(content.data(), (int32)content.size(), size, -1, options);
        if (!h) { printf("load failed\n"); return 1; }
        FT_Size_Metrics &m = fonts[h].handle->size->metrics;
        printf("size %d options %d: handle %d ascender %ld height %ld baseline %d fontwidth %d\n", size, options, h,
               (long)m.ascender, (long)m.height, fonts[h].baseline, FontWidth(h));
        for (int i = 4; i < argc; ++i) {
            uint32 cp = (uint32)strtoul(argv[i], nullptr, 0);
            for (int32 opt : {0, 1}) {
                uint8 *d = nullptr;
                int32 x, y, pre, post;
                FontRenderTextUTF32(h, &cp, 1, opt, &d, &x, &y, &pre, &post);
                printf("  (0x%X, %d, %d, %d, %d, 0x%016llx),\n", cp, opt, x, y, pre, (unsigned long long)fnv1a(d, (size_t)x * y));
                free(d);
            }
        }
        FontFree(h);
    }
    return 0;
}

int main(int argc, char **argv) {
    for (int i = 0; i < 256; ++i) codepage437_to_unicode16[i] = (uint16)i;
    if (argc > 3 && !strcmp(argv[1], "--dump")) return dump(argc, argv);
    bool quick = false;
    std::vector<const char *> files;
    for (int i = 1; i < argc; ++i) {
        if (!strcmp(argv[i], "--quick"))
            quick = true;
        else if (!strcmp(argv[i], "--verbose"))
            verbose = true;
        else
            files.push_back(argv[i]);
    }
    if (files.empty()) {
        fprintf(stderr, "usage: diff_font [--quick] [--verbose] font.ttf...\n");
        return 2;
    }
    std::vector<uint32> pool;
    for (uint32 c = 32; c < 127; ++c) pool.push_back(c);
    for (uint32 c = 160; c < 256; ++c) pool.push_back(c);
    for (uint32 c = 0x2500; c < 0x2600; c += 3) pool.push_back(c); // box drawing, blocks, shapes
    for (uint32 c = 0x0E01; c < 0x0E5C; ++c) pool.push_back(c); // Thai
    const int sizes_full[] = {6, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 20, 22, 24, 28, 32, 36, 40, 48, 64, 100};
    const int sizes_quick[] = {8, 12, 16, 24, 40};
    long failures = 0;

    for (const char *path : files) {
        long idx = 0; // "font.ttc:N" selects face N of a collection
        std::string p = path;
        size_t colon = p.rfind(':');
        if (colon != std::string::npos && colon > 1) {
            idx = atol(p.c_str() + colon + 1);
            p = p.substr(0, colon);
        }
        auto content = read_file(p.c_str());
        if (content.empty()) {
            printf("SKIP %s (cannot read)\n", path);
            continue;
        }
        Stats st;
        long load_bad = 0, fontwidth_bad = 0;
        const int *sizes = quick ? sizes_quick : sizes_full;
        int nsizes = quick ? 5 : 21;
        for (int si = 0; si < nsizes; ++si) {
            cur_size = sizes[si];
            for (int32 options : {32, 16 | 32}) {
                int32 hl = FontLoad(content.data(), (int32)content.size(), sizes[si], idx ? idx : -1, options);
                int32 hr = qbrt_font_load(content.data(), (int32)content.size(), sizes[si], idx ? idx : -1, options);
                if (hl != hr) {
                    load_bad++;
                    printf("  LOAD MISMATCH size %d options %d: legacy %d rust %d\n", sizes[si], options, hl, hr);
                }
                if (!hl || !hr) continue;
                if (FontWidth(hl) != qbrt_font_width(hr)) {
                    fontwidth_bad++;
                    if (verbose) printf("  FONTWIDTH size %d: legacy %d rust %d\n", sizes[si], FontWidth(hl), qbrt_font_width(hr));
                }
                for (int32 opt : {0, 1}) {
                    for (uint32 c : pool) compare_render(hl, hr, {c}, opt, st, "glyph");
                    for (int k = 0; k < (quick ? 20 : 150); ++k) {
                        std::vector<uint32> s(2 + next() % 6);
                        for (auto &c : s) c = pool[next() % 95]; // ASCII strings
                        compare_render(hl, hr, s, opt, st, "string");
                    }
                }
                // empty and negative counts
                uint8 *d = nullptr;
                int32 x, y, p, q, x2, y2, p2, q2;
                int32 r1 = FontRenderTextUTF32(hl, nullptr, 0, 0, &d, &x, &y, &p, &q);
                int32 r2 = qbrt_font_render_utf32(hr, nullptr, 0, 0, &d, &x2, &y2, &p2, &q2);
                if (r1 != r2 || x != x2 || y != y2) st.height_bad++;
                FontFree(hl);
                qbrt_font_free(hr);
            }
        }
        double exact = 100.0 * st.width_exact / (st.renders ? st.renders : 1);
        double mean = st.compared_px ? st.abs_sum / st.compared_px : 0;
        double far = 100.0 * st.width_off_by_more / (st.renders ? st.renders : 1);
        bool bad = load_bad || fontwidth_bad || st.height_bad || st.fail_bad || far > WIDTH_FAR_PERCENT ||
                   exact < WIDTH_EXACT_PERCENT || mean >= PIXEL_MEAN_TOLERANCE;
        printf("%s %s: %ld renders, width/pre exact %.2f%%, %ld off by more than 1/glyph, height/fail mismatches %ld/%ld, "
               "load %ld, fontwidth %ld, pixels differing %.2f%% mean abs %.2f\n",
               bad ? "FAIL" : "ok  ", path, st.renders, exact, st.width_off_by_more, st.height_bad, st.fail_bad, load_bad,
               fontwidth_bad, st.compared_px ? 100.0 * st.px_diff / st.compared_px : 0.0, mean);
        failures += bad;
    }
    // garbage must fail on both sides
    std::vector<uint8_t> junk(500);
    for (auto &b : junk) b = (uint8_t)next();
    if ((FontLoad(junk.data(), 500, 16, -1, 0) != 0) != (qbrt_font_load(junk.data(), 500, 16, -1, 0) != 0)) {
        printf("MISMATCH: garbage font load\n");
        failures++;
    }
    printf("diff_font: %s\n", failures ? "FAILED" : "OK");
    return failures ? 1 : 0;
}
