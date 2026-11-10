// Differential test: real x87 `long double` (MinGW g++, control word 0x037F) vs the
// software crate crates/qb-float80 (R11d).
//
// Covers fadd/fsub/fmul/fdiv/fsqrt/frndint(4 modes)/fscale/fucomi, fld/fst for f32
// and f64, fild/fistp/fisttp for 16/32/64-bit integers and the u64 casts, MinGW's
// expl/exp2l/log2l/powl/__powil/ldexpl/floorl/ceill, the legacy libqb functions
// func_exp_float/pow2/func_fix_float/qbr, __mingw_strtold and __mingw_snprintf
// (%Le/%Lf/%Lg). Inputs: random bit patterns (incl. unnormals, pseudo-NaN/inf,
// pseudo-denormals), denormals, specials, ties and near-ties.
//
//   cargo rustc -p qb-float80 --release --target x86_64-pc-windows-gnu \
//       --features diff-hooks --crate-type staticlib
//   g++ -O2 -std=c++17 -w tests/diff/diff_float80.cpp \
//       target/x86_64-pc-windows-gnu/release/libqb_float80.a -lntdll -luserenv -lws2_32 -o diff_float80
//   ./diff_float80 [--quick] [--mutate]
//
// --mutate runs the hardware reference with 53-bit precision control (0x027F)
// to show that the harness detects wrong results.

#define __USE_MINGW_ANSI_STDIO 1
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>

using namespace std; // like libqb.h: exp/pow/floor/ceil resolve to the long double overloads

typedef int64_t int64;
typedef uint64_t uint64;
typedef int32_t int32;

namespace legacy {
#include "legacy/qbr_x87.inc"

static int err;
static void error(int e) { err = e; }
// Verbatim from internal/c/libqb.cpp (func_exp_float, pow2, func_fix_float).
long double func_exp_float(long double value){
    if (value<=709.782712893){
        return exp(value);
    }
    error(6); return 0;
}
long double pow2(long double x,long double y){
    if (x<0){
        if (y!=floor(y)){error(5); return 0;}
    }
    return pow(x,y);
}
long double func_fix_float(long double value){
    if (value<0) return ceil(value); else return floor(value);
}
}

extern "C" long double __powil(long double, int);
extern "C" {
int64_t qb_f80_diff_op(uint32_t op, const void *a, const void *b, int64_t i, void *out);
int32_t qb_f80_parse(const unsigned char *s, int32_t len, void *out);
int32_t qb_f80_format(const void *a, unsigned char conv, int32_t prec, char *buf, size_t cap);
}

enum {
    ADD, SUB, MUL, DIV, SQRT, RNDINT, SCALE, CMP, FROM_F64, FROM_F32, TO_F64, TO_F32, FROM_I64, FROM_U64,
    TO_INT, TO_U64, EXPL, EXP2L, LOG2L, POWL, POWIL, LDEXPL, FLOORL, CEILL, QB_EXP, QB_POW, QB_FIX, QB_QBR,
    SOFT_F2XM1, SOFT_FYL2X, SOFT_FYL2XP1, F2XM1, FYL2X, FYL2XP1
};

static uint64_t rng = 0x243F6A8885A308D3ull;
static uint64_t next() {
    rng ^= rng << 13;
    rng ^= rng >> 7;
    rng ^= rng << 17;
    return rng;
}

static unsigned short CW = 0x037F;
static void set_cw(unsigned short cw) { __asm__ volatile("fldcw %0" : : "m"(cw)); }
static unsigned short get_cw() { unsigned short c; __asm__ volatile("fnstcw %0" : "=m"(c)); return c; }
// Hardware reference runs under CW (0x037F, or 0x027F for --mutate); the Rust
// side sets its own control word around the x87 primitives it uses.
struct Hw { unsigned short saved; Hw(unsigned short rc = 0) { saved = get_cw(); set_cw((CW & ~0x0C00) | (rc << 10)); } ~Hw() { set_cw(saved); } };

typedef struct { unsigned char b[10]; } F;
static F from_ld(long double v) { F f; memcpy(f.b, &v, 10); return f; }
static long double to_ld(F f) { long double v = 0; memcpy(&v, f.b, 10); return v; }
static F mk(bool neg, unsigned e, uint64_t m) { F f; memcpy(f.b, &m, 8); unsigned short se = (unsigned short)((neg ? 0x8000 : 0) | (e & 0x7FFF)); memcpy(f.b + 8, &se, 2); return f; }
static uint64_t mant(F f) { uint64_t m; memcpy(&m, f.b, 8); return m; }
static unsigned se(F f) { unsigned short s; memcpy(&s, f.b + 8, 2); return s; }

struct Stat { const char *name; long n, bad; };
static Stat stats[64];
static int nstats = 0;
static long failures = 0;
static Stat *stat(const char *name) {
    for (int i = 0; i < nstats; i++) if (!strcmp(stats[i].name, name)) return &stats[i];
    stats[nstats] = {name, 0, 0};
    return &stats[nstats++];
}
static void hex(char *o, F f) { sprintf(o, "%04X:%016llX", se(f), (unsigned long long)mant(f)); }
static void check_f(const char *op, F a, F b, F want, F got, long long extra = 0) {
    Stat *s = stat(op);
    s->n++;
    if (!memcmp(want.b, got.b, 10)) return;
    s->bad++;
    failures++;
    if (s->bad <= 8) {
        char ha[32], hb[32], hw[32], hg[32];
        hex(ha, a); hex(hb, b); hex(hw, want); hex(hg, got);
        printf("MISMATCH %s a=%s b=%s i=%lld legacy=%s rust=%s\n", op, ha, hb, extra, hw, hg);
    }
}
static void check_i(const char *op, F a, F b, long long want, long long got, long long extra = 0) {
    Stat *s = stat(op);
    s->n++;
    if (want == got) return;
    s->bad++;
    failures++;
    if (s->bad <= 8) {
        char ha[32], hb[32];
        hex(ha, a); hex(hb, b);
        printf("MISMATCH %s a=%s b=%s i=%lld legacy=%llx rust=%llx\n", op, ha, hb, extra, want, got);
    }
}
static void check_s(const char *op, F a, long long extra, const char *want, const char *got) {
    Stat *s = stat(op);
    s->n++;
    if (!strcmp(want, got)) return;
    s->bad++;
    failures++;
    if (s->bad <= 8) {
        char ha[32];
        hex(ha, a);
        printf("MISMATCH %s a=%s i=%lld legacy=\"%.80s\" rust=\"%.80s\"\n", op, ha, extra, want, got);
    }
}

static F rust(int op, F a, F b = F(), long long i = 0, long long *ret = nullptr) {
    F out;
    long long r = qb_f80_diff_op(op, a.b, b.b, i, out.b);
    if (ret) *ret = r;
    return out;
}

// ---- input generators ----
static F specials[] = {
    mk(0, 0, 0), mk(1, 0, 0), mk(0, 0x3FFF, 1ull << 63), mk(1, 0x3FFF, 1ull << 63),
    mk(0, 0x7FFF, 1ull << 63), mk(1, 0x7FFF, 1ull << 63),                                   // inf
    mk(0, 0x7FFF, 0xC000000000000000ull), mk(1, 0x7FFF, 0xC000000000000000ull),            // qnan, indefinite
    mk(0, 0x7FFF, 0x8000000000000001ull), mk(1, 0x7FFF, 0xA000000000000000ull),            // snan
    mk(0, 0x7FFF, 0xFFFFFFFFFFFFFFFFull), mk(0, 0x7FFF, 0), mk(1, 0x7FFF, 0x4000000000000000ull), // pseudo
    mk(0, 0x3FFF, 0x4000000000000000ull), mk(0, 1, 0x7FFFFFFFFFFFFFFFull),                  // unnormal
    mk(0, 0, 1), mk(1, 0, 0x7FFFFFFFFFFFFFFFull), mk(0, 0, 0x8000000000000000ull),          // denormal, pseudo-denormal
    mk(0, 0, 0xFFFFFFFFFFFFFFFFull), mk(0, 1, 1ull << 63), mk(0, 0x7FFE, ~0ull), mk(1, 0x7FFE, ~0ull),
    mk(0, 0x3FFE, 1ull << 63), mk(0, 0x4000, 1ull << 63), mk(0, 0x403E, 1ull << 63), mk(0, 0x403D, ~0ull),
    mk(0, 0x401D, 1ull << 63), mk(1, 0x401E, 1ull << 63), mk(0, 0x400D, 1ull << 63), mk(1, 0x400E, 0x8000000100000000ull),
};
static const int NSPECIAL = sizeof(specials) / sizeof(specials[0]);

static uint64_t rand_mant() {
    switch (next() % 6) {
        case 0: return next() | (1ull << 63);
        case 1: return (1ull << 63) | (next() >> (next() % 64)); // few leading bits
        case 2: return (1ull << 63) | (next() << (next() % 64)); // trailing zeros
        case 3: return ~0ull ^ (next() % 4 ? 0 : (next() >> (next() % 64)));
        case 4: return (1ull << 63) | (1ull << (next() % 63)) | (next() & 0xF);
        default: return (next() | (1ull << 63)) & ~((1ull << (next() % 64)) - 1);
    }
}

static F gen() {
    uint64_t k = next() % 100;
    bool neg = next() & 1;
    if (k < 8) { F f; uint64_t m = next(); memcpy(f.b, &m, 8); unsigned short s = (unsigned short)next(); memcpy(f.b + 8, &s, 2); return f; }
    if (k < 14) return specials[next() % NSPECIAL];
    if (k < 50) return mk(neg, 0x3FFF + (int)(next() % 140) - 70, rand_mant());
    if (k < 62) return mk(neg, 1 + next() % 0x7FFE, rand_mant());
    if (k < 67) return mk(neg, 0, next() >> (next() % 64)); // denormal (maybe pseudo)
    if (k < 72) return mk(neg, 1 + next() % 80, rand_mant()); // near denormal range
    if (k < 77) return mk(neg, 0x7FFE - next() % 80, rand_mant()); // near overflow
    if (k < 87) { long double v = (long double)(int64_t)(next() >> (next() % 64)); return from_ld(neg ? -v : v); }
    if (k < 95) { double d; uint64_t bits = next(); memcpy(&d, &bits, 8); return from_ld((long double)d); }
    return mk(neg, 0x3FFF + (int)(next() % 8) - 4, (1ull << 63) | ((next() & 0xFF) << 1)); // short mantissas
}

// b related to a: same exponent region / nearly equal / far apart.
static F gen_pair(F a) {
    uint64_t k = next() % 10;
    if (k < 4) return gen();
    unsigned e = se(a) & 0x7FFF;
    bool neg = next() & 1;
    if (k < 7) return mk(neg, e + (int)(next() % 140) - 70, rand_mant());
    if (k < 9) { uint64_t m = mant(a) ^ (next() >> (next() % 64 + 1)); return mk(neg, e + (int)(next() % 3) - 1, m | (1ull << 63)); }
    return mk(neg, e, mant(a));
}

// ---- hardware reference ----
static long double hw_add(long double a, long double b) { long double r; __asm__("faddp" : "=t"(r) : "0"(b), "u"(a) : "st(1)"); return r; }
static long double hw_mul(long double a, long double b) { long double r; __asm__("fmulp" : "=t"(r) : "0"(b), "u"(a) : "st(1)"); return r; }
// AT&T `fsubrp`/`fdivrp` with no operands compute st(1) = st(1) op st(0); checked at start.
static long double hw_sub(long double a, long double b) { long double r; __asm__("fsubrp" : "=t"(r) : "0"(b), "u"(a) : "st(1)"); return r; }
static long double hw_div(long double a, long double b) { long double r; __asm__("fdivrp" : "=t"(r) : "0"(b), "u"(a) : "st(1)"); return r; }
static long double hw_sqrt(long double a) { long double r; __asm__("fsqrt" : "=t"(r) : "0"(a)); return r; }
static long double hw_rnd(long double a) { long double r; __asm__("frndint" : "=t"(r) : "0"(a)); return r; }
static long double hw_scale(long double a, long double b) { long double r; __asm__("fscale" : "=t"(r) : "0"(a), "u"(b)); return r; }
static long double hw_f2xm1(long double a) { long double r; __asm__("f2xm1" : "=t"(r) : "0"(a)); return r; }
static long double hw_fyl2x(long double x, long double y) { long double r; __asm__("fyl2x" : "=t"(r) : "0"(x), "u"(y) : "st(1)"); return r; }
static long double hw_fyl2xp1(long double x, long double y) { long double r; __asm__("fyl2xp1" : "=t"(r) : "0"(x), "u"(y) : "st(1)"); return r; }
static int hw_cmp(long double a, long double b) {
    // fucomip st(1): ZF,PF,CF
    unsigned char zf, pf, cf;
    __asm__("fucomip %%st(1), %%st\n\tfstp %%st(0)\n\tsetz %0\n\tsetp %1\n\tsetc %2" : "=r"(zf), "=r"(pf), "=r"(cf) : "t"(a), "u"(b) : "st", "cc");
    if (pf) return 2;
    if (zf) return 0;
    return cf ? -1 : 1;
}
static int64_t hw_fistp(long double a, int bits) {
    if (bits == 16) { int16_t v; __asm__("fistps %0" : "=m"(v) : "t"(a) : "st"); return v; }
    if (bits == 32) { int32_t v; __asm__("fistpl %0" : "=m"(v) : "t"(a) : "st"); return v; }
    int64_t v; __asm__("fistpll %0" : "=m"(v) : "t"(a) : "st"); return v;
}

static volatile long double sink;

static void test_arith(long n) {
    for (long it = 0; it < n; it++) {
        F a = gen(), b = gen_pair(a);
        if (next() & 1) { F t = a; a = b; b = t; }
        long double x = to_ld(a), y = to_ld(b);
        { Hw h; check_f("fadd", a, b, from_ld(hw_add(x, y)), rust(ADD, a, b)); }
        { Hw h; check_f("fsub", a, b, from_ld(hw_sub(x, y)), rust(SUB, a, b)); }
        { Hw h; check_f("fmul", a, b, from_ld(hw_mul(x, y)), rust(MUL, a, b)); }
        { Hw h; check_f("fdiv", a, b, from_ld(hw_div(x, y)), rust(DIV, a, b)); }
        { Hw h; check_f("fsqrt", a, b, from_ld(hw_sqrt(x)), rust(SQRT, a)); }
        for (int rc = 0; rc < 4; rc++) { Hw h(rc); check_f("frndint", a, b, from_ld(hw_rnd(x)), rust(RNDINT, a, b, rc), rc); }
        F sb = (next() & 3) ? from_ld((long double)((int)(next() % 40000) - 20000) + (next() & 1 ? 0.5L : 0)) : b;
        { Hw h; check_f("fscale", a, sb, from_ld(hw_scale(x, to_ld(sb))), rust(SCALE, a, sb)); }
        long long r;
        { Hw h; int c = hw_cmp(x, y); rust(CMP, a, b, 0, &r); check_i("fucomi", a, b, c, r); }
    }
}

static void test_conv(long n) {
    for (long it = 0; it < n; it++) {
        F a = gen(), z = F();
        long long r;
        {
            Hw h;
            volatile double d = (double)to_ld(a);
            uint64_t bits; double dd = d; memcpy(&bits, &dd, 8);
            rust(TO_F64, a, z, 0, &r);
            check_i("fst_f64", a, z, (long long)bits, r);
            volatile float f = (float)to_ld(a);
            uint32_t fb; float ff = f; memcpy(&fb, &ff, 4);
            rust(TO_F32, a, z, 0, &r);
            check_i("fst_f32", a, z, (long long)fb, (uint32_t)r);
        }
        {
            Hw h;
            uint64_t bits = next();
            if (next() % 4 == 0) bits = (bits & 0x800FFFFFFFFFFFFFull) | ((uint64_t)(next() % 3 == 0 ? 0x7FF : 0) << 52);
            double d; memcpy(&d, &bits, 8);
            volatile double vd = d;
            check_f("fld_f64", a, z, from_ld((long double)vd), rust(FROM_F64, a, z, (long long)bits));
            uint32_t fb = (uint32_t)next();
            if (next() % 4 == 0) fb = (fb & 0x807FFFFF) | ((next() % 3 == 0 ? 0xFFu : 0u) << 23);
            float f; memcpy(&f, &fb, 4);
            volatile float vf = f;
            check_f("fld_f32", a, z, from_ld((long double)vf), rust(FROM_F32, a, z, (long long)fb));
            int64_t iv = (int64_t)(next() >> (next() % 64)); if (next() & 1) iv = -iv;
            volatile int64_t vi = iv;
            check_f("fild_i64", a, z, from_ld((long double)vi), rust(FROM_I64, a, z, iv));
            uint64_t uv = next() >> (next() % 64);
            volatile uint64_t vu = uv;
            check_f("fild_u64", a, z, from_ld((long double)vu), rust(FROM_U64, a, z, (long long)uv));
        }
        // Integers: values around the integer ranges.
        F c = a;
        if (next() & 1) c = mk(next() & 1, 0x3FFF + 13 + next() % 52, rand_mant());
        long double x = to_ld(c);
        for (int bits : {16, 32, 64}) {
            for (int rc = 0; rc < 4; rc++) {
                Hw h(rc);
                int64_t want = hw_fistp(x, bits);
                rust(TO_INT, c, z, bits + 1000 * rc, &r);
                check_i("fistp", c, z, want, r, bits + 1000 * rc);
            }
            Hw h;
            int64_t want = bits == 16 ? (int16_t)x : bits == 32 ? (int64_t)(int32_t)x : (int64_t)x;
            rust(TO_INT, c, z, bits + 3000, &r);
            check_i("cast_int", c, z, want, r, bits);
        }
        { Hw h; uint64_t want = (uint64_t)x; rust(TO_U64, c, z, 0, &r); check_i("cast_u64", c, z, (long long)want, r); }
        { Hw h; long long want = legacy::qbr(x); rust(QB_QBR, c, z, 0, &r); check_i("qbr", c, z, want, r); }
    }
}

static F gen_math() {
    uint64_t k = next() % 10;
    bool neg = next() & 1;
    if (k < 3) return gen();
    if (k < 6) return mk(neg, 0x3FFF + (int)(next() % 16) - 6, rand_mant()); // |x| ~ 0.01 .. 1000
    if (k < 8) return mk(neg, 0x3FFF + (int)(next() % 80) - 66, rand_mant());
    long double v = (long double)((int)(next() % 2000) - 1000) / (long double)(1 + next() % 16);
    return from_ld(v);
}

static void test_libm(long n) {
    for (long it = 0; it < n; it++) {
        F a = gen_math(), b = gen_math(), z = F();
        long double x = to_ld(a), y = to_ld(b);
        { Hw h; check_f("expl", a, z, from_ld(expl(x)), rust(EXPL, a)); }
        { Hw h; check_f("exp2l", a, z, from_ld(exp2l(x)), rust(EXP2L, a)); }
        { Hw h; long double v = (next() & 1) ? fabsl(x) : x; F av = from_ld(v); check_f("log2l", av, z, from_ld(log2l(v)), rust(LOG2L, av)); }
        // powl: random, integer exponents, halves, near 1.
        F py = b;
        switch (next() % 5) {
            case 0: py = from_ld((long double)((int)(next() % 200) - 100)); break;
            case 1: py = from_ld((long double)((int)(next() % 200) - 100) + 0.5L); break;
            case 2: py = from_ld(0.5L); break;
            default: break;
        }
        F px = a;
        if (next() % 4 == 0) px = mk(next() & 1, 0x3FFF, (1ull << 63) | (next() >> (20 + next() % 44)));
        { Hw h; check_f("powl", px, py, from_ld(powl(to_ld(px), to_ld(py))), rust(POWL, px, py)); }
        int iy = (int)(next() % 4 ? (int64_t)(next() % 400) - 200 : (int32_t)next());
        { Hw h; check_f("__powil", a, z, from_ld(__powil(x, iy)), rust(POWIL, a, z, iy), iy); }
        int ie = (int)(next() % 4 ? (int64_t)(next() % 40000) - 20000 : (int32_t)next());
        { Hw h; check_f("ldexpl", a, z, from_ld(ldexpl(x, ie)), rust(LDEXPL, a, z, ie), ie); }
        { Hw h; check_f("floorl", a, z, from_ld(floorl(x)), rust(FLOORL, a)); }
        { Hw h; check_f("ceill", a, z, from_ld(ceill(x)), rust(CEILL, a)); }
        long long r;
        {
            Hw h;
            legacy::err = 0;
            long double v = legacy::func_exp_float(x);
            F got = rust(QB_EXP, a, z, 0, &r);
            check_i("func_exp_float.err", a, z, legacy::err, r);
            check_f("func_exp_float", a, z, from_ld(v), got);
        }
        {
            Hw h;
            legacy::err = 0;
            long double v = legacy::pow2(to_ld(px), to_ld(py));
            F got = rust(QB_POW, px, py, 0, &r);
            check_i("pow2.err", px, py, legacy::err, r);
            check_f("pow2", px, py, from_ld(v), got);
        }
        { Hw h; check_f("func_fix_float", a, z, from_ld(legacy::func_fix_float(x)), rust(QB_FIX, a)); }
        // The primitives as used (the hardware path in Rust must equal hardware).
        F fr = mk(next() & 1, 0x3FFE - next() % 70, rand_mant());
        { Hw h; check_f("f2xm1", fr, z, from_ld(hw_f2xm1(to_ld(fr))), rust(F2XM1, fr)); }
        F one = from_ld(1.0L);
        { Hw h; F p = from_ld(fabsl(x)); check_f("fyl2x", p, one, from_ld(hw_fyl2x(to_ld(p), 1.0L)), rust(FYL2X, p, one)); }
        F sm = mk(next() & 1, 0x3FFD - next() % 70, rand_mant());
        { Hw h; check_f("fyl2xp1", sm, one, from_ld(hw_fyl2xp1(to_ld(sm), 1.0L)), rust(FYL2XP1, sm, one)); }
    }
}

// Software primitives vs hardware: not part of the pass/fail count (the x87
// unit is not correctly rounded); reported for deviation D-11d.
static void report_soft(long n) {
    long same[3] = {0, 0, 0}, far[3] = {0, 0, 0};
    auto ulps = [](F a, F b) {
        long double x = to_ld(a), y = to_ld(b);
        if (x == y) return 0.0L;
        return fabsl(x - y) / fabsl(ldexpl(1.0L, ilogbl(x) - 63));
    };
    F one = from_ld(1.0L), z = F();
    for (long it = 0; it < n; it++) {
        F a = mk(next() & 1, 0x3FFE - next() % 70, rand_mant());
        F b = mk(0, 0x3FFF + (int)(next() % 80) - 40, rand_mant());
        F c = mk(next() & 1, 0x3FFD - next() % 70, rand_mant());
        F h1, h2, h3;
        { Hw h; h1 = from_ld(hw_f2xm1(to_ld(a))); h2 = from_ld(hw_fyl2x(to_ld(b), 1.0L)); h3 = from_ld(hw_fyl2xp1(to_ld(c), 1.0L)); }
        F s1 = rust(SOFT_F2XM1, a), s2 = rust(SOFT_FYL2X, b, one), s3 = rust(SOFT_FYL2XP1, c, one);
        F hs[3] = {h1, h2, h3}, ss[3] = {s1, s2, s3};
        for (int k = 0; k < 3; k++) {
            if (!memcmp(hs[k].b, ss[k].b, 10)) same[k]++;
            else if (ulps(hs[k], ss[k]) > 1.0L) far[k]++;
        }
        (void)z;
    }
    const char *nm[3] = {"f2xm1", "fyl2x", "fyl2xp1"};
    for (int k = 0; k < 3; k++)
        printf("  soft %-8s identical %ld/%ld (%.2f%%), >1 ulp: %ld\n", nm[k], same[k], n, 100.0 * same[k] / n, far[k]);
    long bad = far[0] + far[1] + far[2];
    if (bad) { printf("FAIL: software primitive more than 1 ulp from hardware\n"); failures += bad; }
}

static std::string ld_fmt(const char *conv, int prec, long double v) {
    char fmt[16], buf[6000];
    snprintf(fmt, sizeof fmt, "%%.*L%s", conv);
    __mingw_snprintf(buf, sizeof buf, fmt, prec, v);
    return buf;
}

static void test_decimal(long n) {
    static char buf[6000];
    for (long it = 0; it < n; it++) {
        F a = (next() % 4) ? gen_math() : gen();
        long double x = to_ld(a);
        int prec = next() % 4 ? (int)(next() % 25) : (int)(next() % 70);
        const char *convs[3] = {"e", "f", "g"};
        for (int k = 0; k < 3; k++) {
            int p = prec;
            // %Lf of huge values prints thousands of digits; keep a few.
            if (k == 1 && (se(a) & 0x7FFF) > 0x3FFF + 400 && next() % 8) continue;
            if (k == 1 && (se(a) & 0x7FFF) < 0x3FFF - 400 && next() % 8) continue;
            std::string want = ld_fmt(convs[k], p, x);
            qb_f80_format(a.b, (unsigned char)convs[k][0], p, buf, sizeof buf);
            static const char *names[3] = {"printf %Le", "printf %Lf", "printf %Lg"};
            check_s(names[k], a, p, want.c_str(), buf);
        }
        // strtold on: printf output, random digit strings, hex, specials, junk.
        std::string s;
        switch (next() % 6) {
            case 0: s = ld_fmt("e", (int)(next() % 30), x); break;
            case 1: s = ld_fmt("g", (int)(1 + next() % 25), x); break;
            case 2: {
                int nd = 1 + next() % 40;
                if (next() & 1) s += '-';
                for (int d = 0; d < nd; d++) s += (char)('0' + next() % 10);
                if (next() & 1) { s.insert(s.size() - next() % nd, "."); }
                if (next() & 1) { s += (next() & 1) ? 'e' : 'E'; s += std::to_string((int)(next() % 9900) - 4950); }
                break;
            }
            case 3: { // exact halfway cases between neighbours
                F h = mk(next() & 1, 1 + next() % 0x7FFD, rand_mant());
                F h2 = mk(se(h) >> 15, se(h) & 0x7FFF, mant(h) + 1);
                long double mid = (to_ld(h) + to_ld(h2)); // not exact in general: print with many digits instead
                s = ld_fmt("e", 40 + next() % 30, mid / 2);
                break;
            }
            case 4: {
                const char *lit[] = {"inf", "-INF", "Infinity", "nan", "-nan", "nan(123)", "nan(0x10)", "nan(zz)", "nan(1g)", "nan(0)", "nan(10", "-nan(1)", "nAn(ab_c)", "nan(123456789abcdef01)", "-x", "-", "-0x", "infinit", "0x1.8p3", "0X.Fp-16440", "0x", " +12abc",
                                     "1e", "1e+", ".5", "-.e5", "  \t42", "0x1p-16446", "0x1.fffffffffffffffffffp16383", "1e4932", "1.2e-4951",
                                     "0.000000000000000000000000000000000000001e-4910", "18973149535723176508575932662800702e4897"};
                s = lit[next() % (sizeof lit / sizeof lit[0])];
                break;
            }
            default: {
                char hb[64];
                __mingw_snprintf(hb, sizeof hb, "%La", x);
                s = hb;
                break;
            }
        }
        char *end = nullptr;
        long double want = __mingw_strtold(s.c_str(), &end);
        F got;
        int used = qb_f80_parse((const unsigned char *)s.c_str(), (int)s.size(), got.b);
        F w = from_ld(want);
        if (std::isnan(want)) w = from_ld(want); // NaN payload compared as is
        check_f("strtold", a, F(), w, got, (long long)it);
        check_i("strtold.end", a, F(), (long long)(end - s.c_str()), used);
        if (memcmp(w.b, got.b, 10) && stat("strtold")->bad <= 8) printf("    text: \"%s\"\n", s.c_str());
    }
}

int main(int argc, char **argv) {
    bool quick = false;
    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "--quick")) quick = true;
        if (!strcmp(argv[i], "--mutate")) CW = 0x027F;
    }
    set_cw(0x037F);
    {   // Operand order of the AT&T reverse forms.
        Hw h;
        if (hw_sub(1.0L, 2.0L) != -1.0L || hw_div(1.0L, 4.0L) != 0.25L) {
            if (CW == 0x037F) { printf("harness: fsubrp/fdivrp operand order unexpected\n"); return 2; }
        }
    }
    long scale = quick ? 1 : 20;
    test_arith(150000 * scale);
    test_conv(100000 * scale);
    test_libm(25000 * scale);
    test_decimal(5000 * scale);
    long checks = 0;
    for (int i = 0; i < nstats; i++) {
        printf("  %-20s %10ld checks %8ld mismatches\n", stats[i].name, stats[i].n, stats[i].bad);
        checks += stats[i].n;
    }
    report_soft(20000 * scale);
    printf("diff_float80: %ld checks, %ld mismatches%s\n", checks, failures, CW != 0x037F ? " (mutation run)" : "");
    return failures ? 1 : 0;
}
