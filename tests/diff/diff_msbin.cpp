// Differential test: legacy internal/c/msbin.cpp vs Rust qb_runtime::mbf.
//
// Build (from repo root, after building libqb_runtime.a):
//   g++ -O2 -std=c++17 tests/diff/diff_msbin.cpp <libqb_runtime.a> <native libs> -o diff_msbin
// The legacy functions are renamed with macros so both copies link into one binary.
// Single precision is checked exhaustively (all 2^32 inputs, both directions);
// double precision uses a deterministic xorshift sample plus edge cases.

#include <cstdint>
#include <cstdio>
#include <cstring>

#define _fmsbintoieee legacy_fmsbintoieee
#define _fieeetomsbin legacy_fieeetomsbin
#define _dmsbintoieee legacy_dmsbintoieee
#define _dieeetomsbin legacy_dieeetomsbin
#include "legacy/msbin.inc"
#undef _fmsbintoieee
#undef _fieeetomsbin
#undef _dmsbintoieee
#undef _dieeetomsbin

#include "../../crates/qb-runtime/include/qb_runtime.h"

static uint64_t rng_state = 0x9E3779B97F4A7C15ull;
static uint64_t next_u64() {
    uint64_t x = rng_state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    return rng_state = x;
}

static long failures = 0;

template <typename T, typename Fn>
static void check(const char *name, Fn legacy, Fn rust, const T &input) {
    // Pre-fill both destinations identically so "untouched on overflow" is compared too.
    T a, b;
    memset(&a, 0xA5, sizeof a);
    memset(&b, 0xA5, sizeof b);
    T in1 = input, in2 = input;
    int ra = legacy(&in1, &a);
    int rb = rust(&in2, &b);
    if (ra != rb || memcmp(&a, &b, sizeof a) != 0 || memcmp(&in1, &in2, sizeof in1) != 0) {
        if (failures < 20) {
            unsigned char ib[sizeof(T)];
            memcpy(ib, &input, sizeof ib);
            printf("MISMATCH %s input=", name);
            for (size_t i = 0; i < sizeof ib; ++i) printf("%02X", ib[sizeof ib - 1 - i]);
            printf(" legacy_rc=%d rust_rc=%d\n", ra, rb);
        }
        ++failures;
    }
}

int main(int argc, char **argv) {
    if (qb_abi_version() != QB_ABI_VERSION) {
        printf("ABI version mismatch\n");
        return 2;
    }
    const bool quick = argc > 1 && strcmp(argv[1], "--quick") == 0;
    const uint64_t single_step = quick ? 65521u : 1u;

    for (uint64_t bits = 0; bits <= 0xFFFFFFFFull; bits += single_step) {
        uint32_t b32 = static_cast<uint32_t>(bits);
        float f;
        memcpy(&f, &b32, 4);
        check<float>("fieeetomsbin", legacy_fieeetomsbin, _fieeetomsbin, f);
        check<float>("fmsbintoieee", legacy_fmsbintoieee, _fmsbintoieee, f);
    }

    const uint64_t edge[] = {
        0, 0x8000000000000000ull, 0x3FF0000000000000ull, 0xBFF0000000000000ull,
        0x7FF0000000000000ull, 0xFFF0000000000000ull, 0x7FF8000000000000ull,
        0x0000000000000001ull, 0x47EFFFFFFFFFFFFFull, 0x37F0000000000000ull,
        0x37E0000000000000ull, 0x7FEFFFFFFFFFFFFFull, 0xFF00000000000000ull,
        0x0100000000000000ull, 0xFFFFFFFFFFFFFFFFull,
    };
    for (uint64_t e : edge) {
        double d;
        memcpy(&d, &e, 8);
        check<double>("dieeetomsbin", legacy_dieeetomsbin, _dieeetomsbin, d);
        check<double>("dmsbintoieee", legacy_dmsbintoieee, _dmsbintoieee, d);
    }
    const long samples = quick ? 200000 : 50000000;
    for (long i = 0; i < samples; ++i) {
        uint64_t r = next_u64();
        double d;
        memcpy(&d, &r, 8);
        check<double>("dieeetomsbin", legacy_dieeetomsbin, _dieeetomsbin, d);
        check<double>("dmsbintoieee", legacy_dmsbintoieee, _dmsbintoieee, d);
    }

    printf("diff_msbin: %s, %ld mismatches\n", quick ? "quick" : "exhaustive", failures);
    return failures == 0 ? 0 : 1;
}

#include "host_stubs.h"
