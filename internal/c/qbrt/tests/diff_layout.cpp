// Differential test: legacy prepare_environment_2d (tests/legacy/prepare_environment_2d.inc,
// verbatim from libqb/gui.cpp) vs Rust qbrt::raster::layout (qbrt_layout), on random and
// edge-case SCREEN/window sizes and every full-screen/stretch mode.
//
//   g++ -O1 -std=c++17 -w internal/c/qbrt/tests/diff_layout.cpp <libqbrt.a> <native libs> -o diff_layout
//   ./diff_layout [--quick] [--mutate]

#include <cstdint>
#include <cstdio>
#include <cstring>

typedef int32_t int32;

namespace legacy {
static int32 full_screen, fullscreen_smooth, resize_auto;
static int32 environment_2d__screen_width, environment_2d__screen_height;
static int32 environment__window_width, environment__window_height;
static int32 environment_2d__screen_x1, environment_2d__screen_y1, environment_2d__screen_x2, environment_2d__screen_y2;
static int32 environment_2d__screen_scaled_width, environment_2d__screen_scaled_height;
static float environment_2d__screen_x_scale, environment_2d__screen_y_scale;
static int32 environment_2d__screen_smooth, environment_2d__letterbox;
#include "legacy/prepare_environment_2d.inc"
} // namespace legacy

#include "../include/qbrt.h"

static uint64_t rng = 0xD1B54A32D192ED03ull;
static uint32_t rnd() {
    rng ^= rng << 13;
    rng ^= rng >> 7;
    rng ^= rng << 17;
    return (uint32_t)rng;
}

static int32 dim() {
    static const int32 common[] = {1, 2, 40, 80, 200, 320, 350, 400, 480, 600, 640, 720, 768, 800, 1024, 1080, 1280, 1440, 1920, 2160, 3840};
    if (rnd() % 3) return common[rnd() % (sizeof(common) / sizeof(common[0]))];
    return 1 + rnd() % 5000;
}

int main(int argc, char **argv) {
    long iters = 5000000;
    bool mutate = false;
    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "--quick")) iters = 200000;
        if (!strcmp(argv[i], "--mutate")) mutate = true;
    }
    long mismatches = 0;
    for (long it = 0; it < iters; it++) {
        using namespace legacy;
        int32 sw = dim(), sh = dim(), ww = dim(), wh = dim();
        if (rnd() % 4 == 0) { ww = sw; wh = sh; }
        if (rnd() % 8 == 0) { ww = sw * 2; wh = sh * 2; }
        full_screen = rnd() % 3;
        fullscreen_smooth = rnd() % 2;
        resize_auto = rnd() % 3;
        environment_2d__screen_width = sw; environment_2d__screen_height = sh;
        environment__window_width = ww; environment__window_height = wh;
        // stale values from a previous call must not matter in the "sizes match" branch either
        prepare_environment_2d();
        int32 out[8];
        float sc[2];
        qbrt_layout(sw, sh, ww, wh, full_screen, fullscreen_smooth, resize_auto, out, sc);
        if (mutate && full_screen == 2 && environment_2d__letterbox == 1) out[0]++;
        int32 exp[8] = {environment_2d__screen_x1, environment_2d__screen_y1, environment_2d__screen_x2, environment_2d__screen_y2,
                        environment_2d__screen_scaled_width, environment_2d__screen_scaled_height, environment_2d__screen_smooth,
                        environment_2d__letterbox};
        bool bad = memcmp(out, exp, sizeof(out)) != 0 || sc[0] != environment_2d__screen_x_scale || sc[1] != environment_2d__screen_y_scale;
        if (bad) {
            if (mismatches < 10)
                printf("screen %dx%d window %dx%d fs=%d smooth=%d auto=%d: legacy %d,%d,%d,%d %dx%d s%d lb%d %g/%g rust %d,%d,%d,%d %dx%d s%d lb%d %g/%g\n",
                       sw, sh, ww, wh, full_screen, fullscreen_smooth, resize_auto, exp[0], exp[1], exp[2], exp[3], exp[4], exp[5], exp[6], exp[7],
                       environment_2d__screen_x_scale, environment_2d__screen_y_scale, out[0], out[1], out[2], out[3], out[4], out[5], out[6], out[7], sc[0], sc[1]);
            mismatches++;
        }
    }
    printf("diff_layout: %ld cases, %ld mismatches\n", iters, mismatches);
    return mismatches ? 1 : 0;
}
