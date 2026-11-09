// Differential test: legacy string engine (tests/legacy/qbs_engine.inc, a verbatim
// copy of libqb.cpp/qbx.cpp) vs the Rust engine qb_runtime::qbs.
//
// Both engines run the same random BASIC-like statement stream in lockstep:
// assignments of nested string expressions (literals, variables, tmp strings,
// + LEFT$ RIGHT$ MID$ LTRIM$ RTRIM$ _TRIM$ UCASE$ LCASE$ CHR$ SPACE$ STRING$),
// comparisons, INSTR/_INSTRREV/ASC/LEN, the MID$ statement, fixed-length and
// DBLOCK (cmem) strings, FREE, scope cleanup, mass allocation (list and
// descriptor-pool growth) and long concatenation loops (arena growth).
// After every statement the complete engine state is compared: every tracking
// list entry, the descriptor-pool position of every descriptor (incl. the free
// list), arena/DBLOCK offsets, stack pointers, DBLOCK bytes and every variable.
//
//   g++ -O2 -std=c++17 -w tests/diff/diff_qbs.cpp <libqb_runtime.a> <native libs> -o diff_qbs
//   ./diff_qbs [--quick]

#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <memory>
#include <new>
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

struct qbs_field;
// internal/c/common.h
struct qbs{
    uint8 *chr;//a 32 bit pointer to the string's data
    int32 len;//must be signed for comparisons against signed int32s
    uint8 in_cmem;//set to 1 if in the conventional memory DBLOCK
    uint16 *cmem_descriptor;
    uint16 cmem_descriptor_offset;
    uint32 listi;//the index in the list of strings that references it
    uint8 tmp;//set to 1 if the string can be deleted immediately after being processed
    uint32 tmplisti;//the index in the list of strings that references it
    uint8 fixed;//fixed length string
    uint8 readonly;//set to 1 if string is read only
    qbs_field *field;
};

static const size_t CMEM_SIZE = 1114099;

namespace legacy {
static int32 last_error = 0;
static void error(int32 n) { last_error = n; }
int32 new_error = 0;
qbs *nothingstring = NULL;
uint8 cmem[CMEM_SIZE];
uint32 cmem_sp = 65536;
ptrszint dblock;
uint32 qbs_cmem_sp = 256;
uint32 qbs_tmp_list_nexti = 1;
uint32 qbs_cmem_descriptor_space = 256;
void field_free(qbs *str) {}
#include "legacy/qbs_engine.inc"
} // namespace legacy

#include "../../crates/qb-runtime/include/qb_runtime.h"

// Host side of the Rust engine (libqb.cpp/qbx.cpp provide these in a real build).
static int32 rust_last_error = 0;
extern "C" {
uint8 cmem[CMEM_SIZE];
uint32 cmem_sp = 65536;
ptrszint dblock;
void qb_host_error(int32 n) { rust_last_error = n; }
void qb_host_field_free(qbs *s) {}
extern uint32 qbs_tmp_list_nexti;
extern uint32 qbs_cmem_sp;
}
static int32 rust_new_error = 0;
static qbs *rust_nothingstring = NULL;

// ------------------------------------------------------------------------------
struct Api {
    const char *name;
    qbs *(*new_)(int32, uint8);
    qbs *(*new_cmem)(int32, uint8);
    qbs *(*new_txt_len)(const char *, int32);
    qbs *(*new_txt)(const char *);
    qbs *(*new_fixed)(uint8 *, uint32, uint8);
    qbs *(*set)(qbs *, qbs *);
    void (*free_)(qbs *);
    void (*cleanup)(uint32);
    void (*maketmp)(qbs *);
    qbs *(*add)(qbs *, qbs *);
    qbs *(*left)(qbs *, int32);
    qbs *(*right)(qbs *, int32);
    qbs *(*mid)(qbs *, int32, int32, int32);
    qbs *(*ltrim)(qbs *);
    qbs *(*rtrim)(qbs *);
    qbs *(*trim)(qbs *);
    qbs *(*ucase)(qbs *);
    qbs *(*lcase)(qbs *);
    qbs *(*chr)(int32);
    qbs *(*space)(int32);
    qbs *(*string)(int32, int32);
    qbs *(*varptr)(uint8, uint16);
    int32 (*instr)(int32, qbs *, qbs *, int32);
    int32 (*instrrev)(int32, qbs *, qbs *, int32);
    void (*sub_mid)(qbs *, int32, int32, qbs *, int32);
    int32 (*cmp[8])(qbs *, qbs *);
    int32 (*asc)(qbs *, uint32);
    int32 (*asc1)(qbs *);
    int32 (*len)(qbs *);
    void (*set_size)(ptrszint *, int32);
    uint32 *tmp_nexti;
    uint32 *qbs_cmem_sp;
    uint32 *cmem_sp;
    ptrszint *dblock;
    uint8 *cmem;
    int32 *last_error;
    qbs **nothing;
};

static void legacy_cleanup(uint32 base) { legacy::qbs_cleanup(base, 0); }
static int32 legacy_asc(qbs *s, uint32 i) { return legacy::qbs_asc(s, i); }
static int32 legacy_asc1(qbs *s) { return legacy::qbs_asc(s); }
static void rust_sub_mid(qbs *d, int32 s, int32 l, qbs *src, int32 p) {
    // libqb.cpp shim: the new_error/nothingstring checks stay in C++
    if (rust_new_error) return;
    if (d == rust_nothingstring) return;
    qb_sub_mid(d, s, l, src, p);
}
static void rust_cleanup(uint32 base) {
    // qbx.cpp shim: inline fast path, Rust slow path
    if (qbs_tmp_list_nexti == base) return;
    qb_qbs_cleanup(base);
}

static Api L = {"legacy",
                legacy::qbs_new, legacy::qbs_new_cmem, legacy::qbs_new_txt_len, legacy::qbs_new_txt,
                legacy::qbs_new_fixed, legacy::qbs_set, legacy::qbs_free, legacy_cleanup, legacy::qbs_maketmp,
                legacy::qbs_add, legacy::qbs_left, legacy::qbs_right, legacy::func_mid, legacy::qbs_ltrim,
                legacy::qbs_rtrim, legacy::qbs__trim, legacy::qbs_ucase, legacy::qbs_lcase, legacy::func_chr,
                legacy::func_space, legacy::func_string, legacy::func_varptr_helper, legacy::func_instr,
                legacy::func__instrrev, legacy::sub_mid,
                {legacy::qbs_equal, legacy::qbs_notequal, legacy::qbs_lessthan, legacy::qbs_greaterthan,
                 legacy::qbs_lessorequal, legacy::qbs_greaterorequal, legacy::func__str_compare,
                 legacy::func__str_nc_compare},
                legacy_asc, legacy_asc1, legacy::qbs_len, legacy::set_qbs_size,
                &legacy::qbs_tmp_list_nexti, &legacy::qbs_cmem_sp, &legacy::cmem_sp, &legacy::dblock, legacy::cmem,
                &legacy::last_error, &legacy::nothingstring};

static Api R = {"rust",
                qb_qbs_new, qb_qbs_new_cmem, qb_qbs_new_txt_len, qb_qbs_new_txt, qb_qbs_new_fixed,
                qb_qbs_set, qb_qbs_free, rust_cleanup, qb_qbs_maketmp, qb_qbs_add, qb_qbs_left,
                qb_qbs_right, qb_func_mid, qb_qbs_ltrim, qb_qbs_rtrim, qb_qbs__trim, qb_qbs_ucase,
                qb_qbs_lcase, qb_func_chr, qb_func_space, qb_func_string, qb_func_varptr_helper,
                qb_func_instr, qb_func__instrrev, rust_sub_mid,
                {qb_qbs_equal, qb_qbs_notequal, qb_qbs_lessthan, qb_qbs_greaterthan, qb_qbs_lessorequal,
                 qb_qbs_greaterorequal, qb_func__str_compare, qb_func__str_nc_compare},
                qb_qbs_asc, qb_qbs_asc1, qb_qbs_len, qb_set_qbs_size,
                &qbs_tmp_list_nexti, &qbs_cmem_sp, &cmem_sp, &dblock, cmem,
                &rust_last_error, &rust_nothingstring};

// ------------------------------------------------------------------------------
struct Rng {
    uint64_t s;
    uint32_t next() {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        return (uint32_t)(s >> 11);
    }
    int32 range(int32 lo, int32 hi) { return lo + (int32)(next() % (uint32_t)(hi - lo + 1)); }
    bool chance(int pct) { return (int)(next() % 100) < pct; }
};

static const char *literals[] = {"", " ", "a", "hello", "  padded  ", "HeLLo WoRLD", "abcabcabc", "x y z",
                                 "    ", "The quick brown fox jumps over the lazy dog", "\x01\xff\x80z", "lo",
                                 "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"};
static const int NLIT = sizeof(literals) / sizeof(literals[0]);

enum VarKind { VAR, CMEM, FIXED, FIXED_DBLOCK };
struct Var {
    qbs *s;
    VarKind kind;
};
struct World {
    Api *api;
    std::vector<Var> vars;
    uint8 *fixed_pool;
    size_t fixed_next;
    size_t dblock_fixed_next;
};

static uint8 legacy_fixed_pool[1 << 20];
static uint8 rust_fixed_pool[1 << 20];
static long mismatches = 0;
static long statements = 0;

static void report(const char *what, long long a, long long b) {
    if (mismatches < 30)
        printf("MISMATCH stmt=%ld %s legacy=%lld rust=%lld\n", statements, what, a, b);
    mismatches++;
}

// ------------------------------------------------------------------------------
// Expression generator: consumes randomness identically for both engines.
static qbs *gen_expr(World &w, Rng &r, int depth) {
    Api &a = *w.api;
    int k = r.range(0, depth > 3 ? 4 : 19);
    switch (k) {
    case 0:
    case 1: {
        int i = r.range(0, NLIT - 1);
        return r.chance(50) ? a.new_txt(literals[i]) : a.new_txt_len(literals[i], (int32)strlen(literals[i]));
    }
    case 2:
    case 3:
        if (!w.vars.empty()) return w.vars[r.next() % w.vars.size()].s;
        return a.new_txt("novars");
    case 4: {
        int n = r.chance(95) ? r.range(0, 40) : r.range(0, 5000);
        qbs *t = a.new_(n, 1);
        uint32 seed = r.next();
        for (int i = 0; i < n; i++) t->chr[i] = (uint8)(32 + (seed + i * 7) % 90);
        return t;
    }
    case 5:
    case 6: {
        qbs *x = gen_expr(w, r, depth + 1);
        return a.add(x, gen_expr(w, r, depth + 1));
    }
    case 7: {
        int32 l = r.range(-3, 30);
        return a.left(gen_expr(w, r, depth + 1), l);
    }
    case 8: {
        int32 l = r.range(-3, 30);
        return a.right(gen_expr(w, r, depth + 1), l);
    }
    case 9: {
        int32 s = r.range(-5, 30), l = r.range(-5, 30), p = r.range(0, 1);
        return a.mid(gen_expr(w, r, depth + 1), s, l, p);
    }
    case 10: return a.ltrim(gen_expr(w, r, depth + 1));
    case 11: return a.rtrim(gen_expr(w, r, depth + 1));
    case 12: return a.trim(gen_expr(w, r, depth + 1));
    case 13: return a.ucase(gen_expr(w, r, depth + 1));
    case 14: return a.lcase(gen_expr(w, r, depth + 1));
    case 15: return a.chr(r.chance(97) ? r.range(0, 255) : r.range(-300, 600));
    case 16: return a.space(r.range(-3, 40));
    case 17: return a.string(r.range(-3, 40), r.range(-1000, 1000));
    case 18: return a.varptr((uint8)r.next(), (uint16)r.next());
    default: {
        // a tmp copy of a variable made with qbs_maketmp (function return values)
        qbs *v = a.new_(0, 0);
        a.set(v, gen_expr(w, r, depth + 1));
        a.maketmp(v);
        return v;
    }
    }
}

static void add_var(World &w, Rng &r, VarKind kind) {
    Api &a = *w.api;
    qbs *s;
    switch (kind) {
    case VAR: s = a.new_(0, 0); break;
    case CMEM: s = a.new_cmem(0, 0); break;
    case FIXED: {
        uint32 n = (uint32)r.range(1, 50);
        s = a.new_fixed(w.fixed_pool + w.fixed_next, n, 0);
        memset(s->chr, ' ', n);
        w.fixed_next += n;
        if (w.fixed_next > sizeof(legacy_fixed_pool) - 64) w.fixed_next = 0;
        break;
    }
    default: {
        uint32 n = (uint32)r.range(1, 40);
        uint8 *base = (uint8 *)(*a.dblock) + 60000;
        s = a.new_fixed(base + w.dblock_fixed_next, n, 0);
        memset(s->chr, ' ', n);
        w.dblock_fixed_next = (w.dblock_fixed_next + n) % 2000;
        break;
    }
    }
    w.vars.push_back({s, kind});
}

// One random statement. Returns a value that must match between the engines.
static long long statement(World &w, Rng &r, int max_vars) {
    Api &a = *w.api;
    *a.last_error = 0;
    uint32 base = *a.tmp_nexti;
    long long result = 0;
    int k = r.range(0, 99);
    if (w.vars.empty() || (k < 6 && (int)w.vars.size() < max_vars)) {
        int kk = r.range(0, 99);
        int ncmem = 0;
        for (const Var &x : w.vars) ncmem += x.kind == CMEM;
        VarKind kind = kk < 70 ? VAR : kk < 85 ? CMEM : kk < 97 ? FIXED : FIXED_DBLOCK;
        if (kind == CMEM && ncmem >= 8) kind = VAR; // DBLOCK holds 64K
        // Each DBLOCK descriptor takes 4 bytes below cmem_sp and qbs_free never returns
        // them (generated code restores cmem_sp when a SUB exits); stay within DBLOCK.
        if ((kind == CMEM || kind == FIXED_DBLOCK) && *a.cmem_sp < 64000) kind = VAR;
        add_var(w, r, kind);
        return 1;
    }
    Var &v = w.vars[r.next() % w.vars.size()];
    if (k < 9) {
        size_t i = r.next() % w.vars.size();
        a.free_(w.vars[i].s);
        w.vars.erase(w.vars.begin() + i);
        return 2;
    }
    if (k < 60) {
        qbs *x = gen_expr(w, r, 0);
        if (v.kind != CMEM || x->len <= 2000) a.set(v.s, x);
    } else if (k < 70) {
        int op = r.range(0, 7);
        qbs *x = gen_expr(w, r, 1);
        qbs *y = gen_expr(w, r, 1);
        result = a.cmp[op](x, y);
    } else if (k < 76) {
        qbs *x = gen_expr(w, r, 1);
        qbs *y = r.chance(60) ? a.left(a.right(x, r.range(0, 8)), r.range(0, 4)) : gen_expr(w, r, 1);
        int32 s = r.range(-3, 30), p = r.range(0, 1);
        result = r.chance(50) ? a.instr(s, x, y, p) : a.instrrev(s, x, y, p);
    } else if (k < 80) {
        qbs *x = gen_expr(w, r, 1);
        int c = r.range(0, 2);
        result = c == 0 ? a.asc(x, (uint32)r.range(-2, 20)) : c == 1 ? a.asc1(x) : a.len(x);
    } else if (k < 88) {
        int32 s = r.range(-5, 30), l = r.range(-5, 30), p = r.range(0, 1);
        qbs *src = r.chance(15) ? v.s : gen_expr(w, r, 1);
        a.sub_mid(v.s, s, l, src, p);
    } else if (k < 91) {
        if (v.kind != FIXED && v.kind != FIXED_DBLOCK) {
            ptrszint slot = (ptrszint)v.s;
            a.set_size(&slot, r.range(0, 60));
        }
    } else if (k < 96) {
        // PRINT expr: evaluate, look at it, let cleanup free it
        qbs *x = gen_expr(w, r, 0);
        result = x->len;
        for (int i = 0; i < x->len; i++) result = result * 31 + x->chr[i];
    } else {
        // self concatenation: a$ = a$ + expr
        qbs *x = a.add(v.s, gen_expr(w, r, 1));
        if (v.kind != CMEM || x->len <= 2000) a.set(v.s, x);
    }
    a.cleanup(base);
    return result;
}

// ------------------------------------------------------------------------------
static int64 legacy_index(qbs *p) {
    if (!p) return -2;
    if ((ptrszint)p == -1) return -1;
    for (size_t b = 0; b < legacy::qbs_malloc_blocks.size(); b++) {
        qbs *base = legacy::qbs_malloc_blocks[b].get();
        if (p >= base && p < base + 65536) return (int64)b * 65536 + (p - base);
    }
    return -3;
}
static int64 rust_index(qbs *p) {
    if (!p) return -2;
    if ((ptrszint)p == -1) return -1;
    int64 i = qb_qbs_debug_descriptor_index(p);
    return i < 0 ? -3 : i;
}

static void compare_list(const char *what, const ptrszint *a, const ptrszint *b, uint32 n) {
    for (uint32 i = 0; i < n; i++) {
        int64 x = legacy_index((qbs *)a[i]), y = rust_index((qbs *)b[i]);
        if (x != y) {
            char buf[64];
            snprintf(buf, sizeof buf, "%s[%u]", what, i);
            report(buf, x, y);
            return;
        }
    }
}

static long long offset_of_chr(const Var &v, Api &a, uint8 *data, uint8 *fixed_pool) {
    if (v.s->in_cmem || v.kind == FIXED_DBLOCK) return v.s->chr - (uint8 *)(*a.dblock);
    if (v.kind == FIXED) return v.s->chr - fixed_pool;
    return v.s->chr - data;
}

static void compare_state(World &wl, World &wr, bool full = true) {
    QbQbsDebugState st;
    qb_qbs_debug_state(&st);
    if (legacy::qbs_tmp_list_nexti != qbs_tmp_list_nexti) report("tmp_nexti", legacy::qbs_tmp_list_nexti, qbs_tmp_list_nexti);
    if (legacy::qbs_tmp_list_lasti != st.tmp_list_lasti) report("tmp_lasti", legacy::qbs_tmp_list_lasti, st.tmp_list_lasti);
    if (legacy::qbs_list_nexti != st.list_nexti) report("list_nexti", legacy::qbs_list_nexti, st.list_nexti);
    if (legacy::qbs_list_lasti != st.list_lasti) report("list_lasti", legacy::qbs_list_lasti, st.list_lasti);
    if (legacy::qbs_cmem_list_nexti != st.cmem_list_nexti) report("cmem_nexti", legacy::qbs_cmem_list_nexti, st.cmem_list_nexti);
    if (legacy::qbs_cmem_list_lasti != st.cmem_list_lasti) report("cmem_lasti", legacy::qbs_cmem_list_lasti, st.cmem_list_lasti);
    if (legacy::qbs_sp != st.sp) report("qbs_sp", legacy::qbs_sp, st.sp);
    if (legacy::qbs_data_size != st.data_size) report("data_size", legacy::qbs_data_size, st.data_size);
    if (legacy::qbs_cmem_sp != qbs_cmem_sp) report("qbs_cmem_sp", legacy::qbs_cmem_sp, qbs_cmem_sp);
    if (legacy::cmem_sp != cmem_sp) report("cmem_sp", legacy::cmem_sp, cmem_sp);
    if (legacy::qbs_malloc_next != st.malloc_next) report("malloc_next", legacy::qbs_malloc_next, st.malloc_next);
    if (legacy::qbs_malloc_blocks.size() != st.blocks) report("blocks", legacy::qbs_malloc_blocks.size(), st.blocks);
    if (legacy::qbs_malloc_freed.size() != st.freed_count) {
        report("freed_count", legacy::qbs_malloc_freed.size(), st.freed_count);
    } else {
        for (uint32 i = full ? 0 : (st.freed_count > 16 ? st.freed_count - 16 : 0); i < st.freed_count; i++)
            if (legacy_index(legacy::qbs_malloc_freed[i]) != rust_index(st.freed[i])) {
                report("freed[]", legacy_index(legacy::qbs_malloc_freed[i]), rust_index(st.freed[i]));
                break;
            }
    }
    if (full && legacy::qbs_tmp_list_nexti == qbs_tmp_list_nexti)
        compare_list("tmp_list", legacy::qbs_tmp_list.data(), st.tmp_list, qbs_tmp_list_nexti);
    if (full && legacy::qbs_list_nexti == st.list_nexti) compare_list("list", legacy::qbs_list.data(), st.list, st.list_nexti);
    if (full && legacy::qbs_cmem_list_nexti == st.cmem_list_nexti)
        compare_list("cmem_list", legacy::qbs_cmem_list.data(), st.cmem_list, st.cmem_list_nexti);
    if (full && memcmp((uint8 *)legacy::dblock, (uint8 *)dblock, 65536) != 0) {
        for (int i = 0; i < 65536; i++)
            if (((uint8 *)legacy::dblock)[i] != ((uint8 *)dblock)[i]) {
                report("dblock byte", i, ((uint8 *)dblock)[i]);
                break;
            }
    }
    if (wl.vars.size() != wr.vars.size()) {
        report("vars", wl.vars.size(), wr.vars.size());
        return;
    }
    for (size_t i = 0; i < wl.vars.size(); i++) {
        const Var &x = wl.vars[i], &y = wr.vars[i];
        qbs *p = x.s, *q = y.s;
        if (legacy_index(p) != rust_index(q)) report("var descriptor", legacy_index(p), rust_index(q));
        if (p->len != q->len) { report("var len", p->len, q->len); continue; }
        if (p->len > 0 && memcmp(p->chr, q->chr, p->len) != 0) report("var text", (long long)i, 0);
        long long ox = offset_of_chr(x, L, (uint8 *)legacy::qbs_data, legacy_fixed_pool);
        long long oy = offset_of_chr(y, R, (uint8 *)st.data, rust_fixed_pool);
        if (ox != oy) report("var chr offset", ox, oy);
        if (p->in_cmem != q->in_cmem || p->tmp != q->tmp || p->fixed != q->fixed || p->readonly != q->readonly)
            report("var flags", p->in_cmem * 1000 + p->tmp * 100 + p->fixed * 10 + p->readonly,
                   q->in_cmem * 1000 + q->tmp * 100 + q->fixed * 10 + q->readonly);
        if (p->listi != q->listi) report("var listi", p->listi, q->listi);
        if (p->tmplisti != q->tmplisti) report("var tmplisti", p->tmplisti, q->tmplisti);
        if (p->cmem_descriptor_offset != q->cmem_descriptor_offset)
            report("var cmem_descriptor_offset", p->cmem_descriptor_offset, q->cmem_descriptor_offset);
        long long dx = p->cmem_descriptor ? (uint8 *)p->cmem_descriptor - (uint8 *)legacy::dblock : -1;
        long long dy = q->cmem_descriptor ? (uint8 *)q->cmem_descriptor - (uint8 *)dblock : -1;
        if (dx != dy) report("var cmem_descriptor", dx, dy);
    }
}

// One statement on both engines with the same randomness.
static void step(World &wl, World &wr, Rng &rng, int max_vars, bool check) {
    statements++;
    Rng a = rng, b = rng;
    long long x = statement(wl, a, max_vars);
    long long y = statement(wr, b, max_vars);
    rng = a;
    if (x != y) report("statement result", x, y);
    if (*L.last_error != *R.last_error) report("error", *L.last_error, *R.last_error);
    if (check) compare_state(wl, wr, statements % 256 == 0);
}

int main(int argc, char **argv) {
    setvbuf(stdout, NULL, _IONBF, 0);
    const bool quick = argc > 1 && strcmp(argv[1], "--quick") == 0;
    const bool trace = getenv("DIFF_QBS_TRACE") != NULL;
    if (sizeof(qbs) != sizeof(void *) * 3 + 32 && sizeof(void *) == 8) {
        printf("unexpected sizeof(qbs)=%zu\n", sizeof(qbs));
        return 1;
    }
    legacy::dblock = (ptrszint)&legacy::cmem + 1280;
    dblock = (ptrszint)&cmem + 1280;
    World wl{&L, {}, legacy_fixed_pool, 0, 0}, wr{&R, {}, rust_fixed_pool, 0, 0};
    Rng rng{0x9E3779B97F4A7C15ull};

    // 1. random statements with a small set of variables
    const long n1 = quick ? 200000 : 3000000;
    for (long i = 0; i < n1 && mismatches < 30; i++) {
        if (trace && i % 100000 == 0) printf("phase 1: %ld\n", i);
        step(wl, wr, rng, 48, true);
    }
    printf("diff_qbs: phase 1 (%ld random statements) mismatches=%ld\n", n1, mismatches);

    // 2. mass allocation: tracking-list expansion and several descriptor blocks
    const int n2 = quick ? 140000 : 300000;
    for (int i = 0; i < n2; i++) {
        Rng a = rng, b = rng;
        add_var(wl, a, VAR);
        add_var(wr, b, VAR);
        rng = a;
        if (i % 3 == 0) step(wl, wr, rng, 1 << 30, false);
    }
    compare_state(wl, wr);
    for (int i = 0; i < 20000 && mismatches < 30; i++) step(wl, wr, rng, 1 << 30, i % 500 == 0);
    while (wl.vars.size() > 40) {
        size_t i = rng.next() % wl.vars.size();
        L.free_(wl.vars[i].s);
        R.free_(wr.vars[i].s);
        wl.vars.erase(wl.vars.begin() + i);
        wr.vars.erase(wr.vars.begin() + i);
    }
    compare_state(wl, wr);
    // many tmp strings in one scope: tmp list expansion
    {
        uint32 bl = *L.tmp_nexti, br = *R.tmp_nexti;
        for (int i = 0; i < 150000; i++) {
            L.new_(i % 7, 1);
            R.new_(i % 7, 1);
        }
        compare_state(wl, wr);
        L.cleanup(bl);
        R.cleanup(br);
        compare_state(wl, wr);
    }
    printf("diff_qbs: phase 2 (mass allocation) mismatches=%ld\n", mismatches);

    // 3. concatenation loops: arena compaction and growth to several MB
    for (int round = 0; round < (quick ? 2 : 6); round++) {
        int vi = (int)(rng.next() % wl.vars.size());
        for (int i = 0; i < 4000 && mismatches < 30; i++) {
            statements++;
            for (World *w : {&wl, &wr}) {
                Api &a = *w->api;
                uint32 base = *a.tmp_nexti;
                qbs *v = w->vars[vi].s;
                if (w->vars[vi].kind == FIXED || w->vars[vi].kind == FIXED_DBLOCK) continue;
                if (v->in_cmem && v->len > 2000) continue; // DBLOCK is 64K
                a.set(v, a.add(v, a.string(1 + (i * 37 + round) % 700, 65 + i % 26)));
                a.cleanup(base);
            }
            if (i % 50 == 0) compare_state(wl, wr);
            step(wl, wr, rng, 64, false);
        }
        compare_state(wl, wr);
    }
    printf("diff_qbs: phase 3 (concat loops, arena %u bytes) mismatches=%ld\n", legacy::qbs_data_size, mismatches);

    // 4. long random tail with state checks
    const long n4 = quick ? 100000 : 2000000;
    for (long i = 0; i < n4 && mismatches < 30; i++) step(wl, wr, rng, 200, true);
    printf("diff_qbs: %ld statements, %ld mismatches\n", statements, mismatches);
    return mismatches ? 1 : 0;
}
