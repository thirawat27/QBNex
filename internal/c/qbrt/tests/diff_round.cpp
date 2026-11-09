// Differential test: legacy x87 rounding (tests/legacy/qbr_x87.inc) vs Rust qbrt::round.
// x86/x86-64 only (the legacy reference is x87 inline assembly).
//
//   g++ -O1 -std=c++17 -w internal/c/qbrt/tests/diff_round.cpp <libqbrt.a> <native libs> -o diff_round
//   ./diff_round [--quick]

#include <cstdint>
#include <cstdio>
#include <cstring>
#include <cmath>

typedef int64_t int64;
typedef uint64_t uint64;
typedef int32_t int32;

namespace legacy {
#include "legacy/qbr_x87.inc"
}

#include "../include/qbrt.h"

static uint64_t rng = 0x243F6A8885A308D3ull;
static uint64_t next() {
    rng ^= rng << 13;
    rng ^= rng >> 7;
    rng ^= rng << 17;
    return rng;
}

static long failures = 0;
static void report(const char *what, const unsigned char *bytes, int n, long long a, long long b) {
    if (failures++ < 20) {
        printf("MISMATCH %s input=", what);
        for (int i = n - 1; i >= 0; --i) printf("%02X", bytes[i]);
        printf(" legacy=%lld rust=%lld\n", a, b);
    }
}

static void check_f80(long double f) {
    unsigned char b[10];
    memcpy(b, &f, 10);
    long long a = legacy::qbr(f), r = qbrt_qbr_f80(b);
    if (a != r) report("qbr", b, 10, a, r);
    a = (long long)legacy::qbr_longdouble_to_uint64(f);
    r = qbrt_fistp_i64_f80(b);
    if (a != r) report("qbr_longdouble_to_uint64", b, 10, a, r);
}

static void check_f64(double d) {
    long long a = legacy::qbr_double_to_long(d), r = qbrt_fistp_i32(d);
    if (a != r) report("qbr_double_to_long", (const unsigned char *)&d, 8, a, r);
    float f = (float)d;
    a = legacy::qbr_float_to_long(f);
    r = qbrt_fistp_i32((double)f);
    if (a != r) report("qbr_float_to_long", (const unsigned char *)&f, 4, a, r);
}

int main(int argc, char **argv) {
    static_assert(sizeof(long double) >= 10, "x87 long double required");
    legacy::fpu_reinit();
    const bool quick = argc > 1 && strcmp(argv[1], "--quick") == 0;
    const long n = quick ? 300000 : 30000000;

    for (long i = 0; i < n; ++i) {
        // 80-bit values: exponents clustered around the int64/int32 range, plus arbitrary ones.
        uint64_t mant = next();
        uint64_t r = next();
        unsigned exp;
        switch (r & 3) {
        case 0: exp = 16383 + (unsigned)((r >> 8) % 70) - 3; break;   // ~0.1 .. 2^66
        case 1: exp = 16383 - (unsigned)((r >> 8) % 70); break;       // tiny .. 1
        case 2: exp = (unsigned)((r >> 8) & 0x7FFF); break;           // anything, incl. NaN/inf/denormal
        default: exp = 16383 + 62 + (unsigned)((r >> 8) % 3); mant |= 1ull << 63; break; // around 2^63
        }
        if ((r >> 32) % 13 == 0) mant &= ~0ull << ((r >> 40) % 64); // many trailing zeros: exact halves
        if ((r >> 48) % 17 != 0 && exp != 0) mant |= 1ull << 63;     // mostly normals, some unnormals
        unsigned short se = (unsigned short)(exp | (((r >> 20) & 1) << 15));
        long double f = 0;
        memcpy(&f, &mant, 8);
        memcpy((char *)&f + 8, &se, 2);
        check_f80(f);

        // doubles around the int32 range, half-way values included
        double d = (double)(int64_t)(next() % 8589934592ull) - 4294967296.0;
        if (r & 4) d += 0.5;
        if (r & 8) d = ldexp((double)(next() >> 11), (int)((r >> 24) % 80) - 70);
        check_f64(d);
        check_f64(-d);
    }
    const double specials[] = {NAN, INFINITY, -INFINITY, 0.0, -0.0, 2147483647.5, -2147483648.5,
                               2147483648.0, -2147483649.0, 0.5, 1.5, 2.5, -2.5};
    for (double d : specials) {
        check_f64(d);
        check_f80((long double)d);
    }
    printf("diff_round: %s, %ld mismatches\n", quick ? "quick" : "full", failures);
    return failures == 0 ? 0 : 1;
}
