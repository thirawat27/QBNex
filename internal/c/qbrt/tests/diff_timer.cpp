// Differential test: legacy func_timer + x87 qbr (tests/legacy/*.inc) vs Rust qbrt::timer.
// x86/x86-64 only (the legacy qbr is x87 inline assembly).
//
//   g++ -O1 -std=c++17 -w internal/c/qbrt/tests/diff_timer.cpp <libqbrt.a> <native libs> -o diff_timer
//   ./diff_timer [--quick]

#include <cstdint>
#include <cstdio>
#include <cstring>

typedef int32_t int32;
typedef uint32_t uint32;
typedef int64_t int64;
typedef uint64_t uint64;

namespace legacy {
static int32 new_error = 0;
static int32 last_error = 0;
static void error(int32 n) { last_error = n; }
static uint32 fake_ticks = 0;
static uint32 GetTicks() { return fake_ticks; }
static uint32 clock_firsttimervalue = 0;
static uint32 qbnex_firsttimervalue = 0;
#include "legacy/qbr_x87.inc"
#include "legacy/func_timer.inc"
} // namespace legacy

#include "../include/qbrt.h"

static uint64_t rng = 0xA0761D6478BD642Full;
static uint64_t next() {
    rng ^= rng << 13;
    rng ^= rng >> 7;
    rng ^= rng << 17;
    return rng;
}

int main(int argc, char **argv) {
    legacy::fpu_reinit();
    const bool quick = argc > 1 && strcmp(argv[1], "--quick") == 0;
    const long n = quick ? 300000 : 20000000;
    const double accuracies[] = {0.001, 0.01, 0.1, 0.5, 1, 2.5, 1.0 / 18.2, 0.0001, 3600, 0, -1};
    long failures = 0;
    for (long i = 0; i < n; ++i) {
        uint64_t r = next();
        legacy::fake_ticks = (uint32)r;
        legacy::clock_firsttimervalue = (uint32)(r >> 32);
        legacy::qbnex_firsttimervalue = (uint32)(next() % 86400000u);
        int32 passed = (int32)(r >> 63);
        double acc = accuracies[(r >> 40) % 11];
        legacy::last_error = 0;
        double a = legacy::func_timer(acc, passed);
        int32 ea = legacy::last_error;
        double b = -1;
        int32 eb = qbrt_timer(legacy::fake_ticks, legacy::clock_firsttimervalue, legacy::qbnex_firsttimervalue,
                              acc, passed, &b);
        if (ea != eb || memcmp(&a, &b, 8) != 0) {
            if (failures < 20)
                printf("MISMATCH ticks=%u first=%u day=%u acc=%g passed=%d legacy=%.17g err=%d rust=%.17g err=%d\n",
                       legacy::fake_ticks, legacy::clock_firsttimervalue, legacy::qbnex_firsttimervalue, acc, passed,
                       a, ea, b, eb);
            ++failures;
        }
    }
    printf("diff_timer: %s, %ld mismatches\n", quick ? "quick" : "full", failures);
    return failures == 0 ? 0 : 1;
}
