// Differential test: legacy qbs_str (tests/legacy/qbs_str.inc) vs Rust qbrt::numfmt (STR$).
//
//   g++ -O1 -std=c++17 -w internal/c/qbrt/tests/diff_str.cpp <libqbrt.a> <native libs> -o diff_str
//   ./diff_str [--quick]
//
// Skipped on purpose (intentional deviations, covered by unit tests in numfmt.rs):
// - legacy outputs containing characters that are not part of a number (L-07, broken
//   printf format for more than 19 decimals);
// - non-finite SINGLE values (D-05, legacy read a stale buffer).

#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <string>

typedef int8_t int8;
typedef uint8_t uint8;
typedef int16_t int16;
typedef uint16_t uint16;
typedef int32_t int32;
typedef uint32_t uint32;
typedef int64_t int64;
typedef uint64_t uint64;

namespace legacy {
struct qbs {
    uint8 *chr;
    int32 len;
};
static uint8 storage[4][256];
static int slot = 0;
// Legacy allocates a temp string; the oracle only needs a buffer that outlives the call.
static qbs *qbs_new(int32 size, uint8 tmp) {
    (void)tmp;
    (void)size;
    static qbs q[4];
    slot = (slot + 1) & 3;
    q[slot].chr = storage[slot];
    q[slot].len = 0;
    return &q[slot];
}
#if defined(__MINGW32__)
#define QBNex_WINDOWS
#endif
#include "legacy/qbs_str.inc"
} // namespace legacy

#include "../include/qbrt.h"

static uint64_t rng = 0xD1B54A32D192ED03ull;
static uint64_t next() {
    rng ^= rng << 13;
    rng ^= rng >> 7;
    rng ^= rng << 17;
    return rng;
}

static long failures = 0, skipped = 0;

static bool numeric_text(const std::string &s) {
    for (char c : s)
        if (!strchr(" -+.0123456789ED", c)) return false;
    return true;
}

static void compare(const char *what, legacy::qbs *a, const unsigned char *b, int32 nb, double shown) {
    std::string la((const char *)a->chr, (size_t)a->len), rb((const char *)b, (size_t)nb);
    if (!numeric_text(la)) { ++skipped; return; }
    if (la != rb) {
        if (failures < 25) printf("MISMATCH %s(%.17g) legacy=[%s] rust=[%s]\n", what, shown, la.c_str(), rb.c_str());
        ++failures;
    }
}

static void check_double(double d) {
    unsigned char out[QBRT_STR_MAX];
    compare("STR$#", legacy::qbs_str(d), out, qbrt_str_f64(d, out), d);
}

static void check_float(float f) {
    if (!std::isfinite(f)) { ++skipped; return; }
    unsigned char out[QBRT_STR_MAX];
    compare("STR$!", legacy::qbs_str(f), out, qbrt_str_f32(f, out), f);
}

static void check_ints(uint64_t r) {
    unsigned char out[QBRT_STR_MAX];
    compare("STR$&&", legacy::qbs_str((int64)r), out, qbrt_str_i64((int64)r, out), 0);
    compare("STR$~&&", legacy::qbs_str((uint64)r), out, qbrt_str_u64(r, out), 0);
    compare("STR$&", legacy::qbs_str((int32)r), out, qbrt_str_i64((int32)r, out), 0);
    compare("STR$~&", legacy::qbs_str((uint32)r), out, qbrt_str_u64((uint32)r, out), 0);
    compare("STR$%", legacy::qbs_str((int16)r), out, qbrt_str_i64((int16)r, out), 0);
    compare("STR$~%", legacy::qbs_str((uint16)r), out, qbrt_str_u64((uint16)r, out), 0);
    compare("STR$%%", legacy::qbs_str((int8)r), out, qbrt_str_i64((int8)r, out), 0);
    compare("STR$~%%", legacy::qbs_str((uint8)r), out, qbrt_str_u64((uint8)r, out), 0);
}

int main(int argc, char **argv) {
    const bool quick = argc > 1 && strcmp(argv[1], "--quick") == 0;
    const long n = quick ? 200000 : 5000000;
    const double fixed[] = {0, -0.0, 1, -1, 0.1, 0.5, 2.5, 42, 1e15, 1e16, 1e-16, 1e-17, 123456789012345.6,
                            9.999999999999999e15, 9999999999999999.0, 0.000123, 1.25e-5, 1e300, 5e-324,
                            1.7976931348623157e308, INFINITY, -INFINITY, NAN, 1234567, 12345678, 1.5e-8,
                            0.30000000000000004, 1.0 / 3.0, 2.0 / 3.0, 99999.995, 0.125, 0.375};
    for (double d : fixed) {
        check_double(d);
        check_float((float)d);
    }
    for (long i = 0; i < n; ++i) {
        uint64_t r = next();
        double d;
        switch (r & 3) {
        case 0: memcpy(&d, &r, 8); break;                                   // any bit pattern
        case 1: d = ldexp((double)(next() >> 11), (int)(r >> 8) % 120 - 110); break; // around 1e-17 .. 1e16
        case 2: d = (double)(int64_t)(next() % 2000000001ull) / pow(10.0, (double)((r >> 8) % 19)); break; // short decimals
        default: d = (double)(int64_t)(next() >> (r % 50)); break;          // integers of many sizes
        }
        if (r & 0x100) d = -d;
        check_double(d);
        float f;
        uint32_t fb = (uint32_t)(r >> 32);
        if ((r >> 9) & 1) memcpy(&f, &fb, 4); else f = (float)d;
        check_float(f);
        check_ints(next());
    }
    printf("diff_str: %s, %ld mismatches, %ld skipped (L-07/D-05)\n", quick ? "quick" : "full", failures, skipped);
    return failures == 0 ? 0 : 1;
}
