// Differential test: legacy _MEM code (tests/legacy/mem.inc) vs Rust qb_runtime::mem.
// Random operation sequences run on both implementations with mirrored blocks;
// error codes, block fields, lock ids and memory contents must match.
// Legacy _MEMFREE of a _MEMNEW block double-frees (L-09), so the harness only
// frees unsecured (_MEM(offset,size)) blocks and stale copies on the legacy side.
//
//   g++ -O1 -std=c++17 -w tests/diff/diff_mem.cpp <libqb_runtime.a> <native libs> -o diff_mem
//   ./diff_mem [--quick]

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <memory>
#include <new>
#include <unordered_map>
#include <vector>

typedef int8_t int8;
typedef int16_t int16;
typedef int32_t int32;
typedef int64_t int64;
typedef uint8_t uint8;
typedef uint64_t uint64;
typedef intptr_t ptrszint;

namespace legacy {
struct mem_block {
    ptrszint offset;
    ptrszint size;
    int64 lock_id;
    ptrszint lock_offset;
    ptrszint type;
    ptrszint elementsize;
    int32 image;
    int32 sound;
};
struct mem_lock {
    uint64 id;
    int32 type;
    void *offset;
};
static int32 new_error = 0;
static int32 last_error = 0;
static void error(int32 n) { last_error = n; }
#include "legacy/mem.inc"
} // namespace legacy

#include "../../crates/qb-runtime/include/qb_runtime.h"

static uint64_t rng = 0x9E3779B97F4A7C15ull;
static uint64_t next() {
    rng ^= rng << 13;
    rng ^= rng >> 7;
    rng ^= rng << 17;
    return rng;
}
static long rnd(long lo, long hi) { return lo + (long)(next() % (uint64_t)(hi - lo + 1)); }

struct Pair {
    legacy::mem_block a;
    qb_mem_block b;
};

static long failures = 0;
static void fail(const char *what, long i, long x, long y) {
    if (failures < 20) printf("MISMATCH %s at op %ld: legacy=%ld rust=%ld\n", what, i, x, y);
    ++failures;
}

// Compare fields that do not depend on addresses.
static void cmp_block(const char *what, long i, const legacy::mem_block &a, const qb_mem_block &b) {
    if (a.size != b.size) fail(what, i, (long)a.size, (long)b.size);
    if (a.type != b.type) fail(what, i, (long)a.type, (long)b.type);
    if (a.elementsize != b.elementsize) fail(what, i, (long)a.elementsize, (long)b.elementsize);
    if (a.image != b.image) fail(what, i, a.image, b.image);
    if (a.lock_id != b.lock_id) fail(what, i, (long)a.lock_id, (long)b.lock_id);
    if ((a.offset <= 1 || b.offset <= 1) && a.offset != b.offset) fail(what, i, (long)a.offset, (long)b.offset);
}

static void cmp_bytes(const char *what, long i, const Pair &p) {
    if (p.a.size > 1 && p.a.offset > 1 && memcmp((void *)p.a.offset, (void *)p.b.offset, p.a.size) != 0)
        fail(what, i, 0, 1);
}

int main(int argc, char **argv) {
    const bool quick = argc > 1 && strcmp(argv[1], "--quick") == 0;
    const long n = quick ? 200000 : 5000000;
    std::vector<Pair> blocks;
    for (long i = 0; i < n; ++i) {
        legacy::last_error = 0;
        int op = (int)rnd(0, 9);
        if (blocks.size() < 4) op = 0;
        if (blocks.size() > 64) blocks.erase(blocks.begin(), blocks.begin() + 32);
        Pair &p = blocks.empty() ? *(Pair *)nullptr : blocks[next() % blocks.size()];
        Pair &q = blocks.empty() ? *(Pair *)nullptr : blocks[next() % blocks.size()];
        switch (op) {
        case 0: { // _MEMNEW
            long sz = rnd(-2, 64);
            int ne = rnd(0, 20) == 0;
            legacy::new_error = ne;
            Pair r;
            r.a = legacy::func__memnew(sz);
            int32 eb = qb_memnew(sz, ne, &r.b);
            legacy::new_error = 0;
            if (legacy::last_error != eb) fail("memnew err", i, legacy::last_error, eb);
            cmp_block("memnew", i, r.a, r.b);
            if (r.a.size > 0) {
                cmp_bytes("memnew bytes", i, r);
                blocks.push_back(r);
            }
            break;
        }
        case 1: { // _MEMFILL with checks, random pattern
            uint8 pat[16];
            for (int k = 0; k < 16; ++k) pat[k] = (uint8)next();
            long doff = rnd(-3, p.a.size + 3), dbytes = rnd(-2, p.a.size + 3), sb = rnd(-1, 9);
            if (rnd(0, 3) == 0) sb = 1;
            legacy::sub__memfill(&p.a, p.a.offset + doff, dbytes, (ptrszint)pat, sb);
            int32 eb = qb_memfill(&p.b, p.b.offset + doff, dbytes, pat, sb);
            if (legacy::last_error != eb) fail("memfill err", i, legacy::last_error, eb);
            cmp_bytes("memfill bytes", i, p);
            break;
        }
        case 2: { // _MEMCOPY
            long soff = rnd(-2, p.a.size + 2), doff = rnd(-2, q.a.size + 2), bytes = rnd(-2, 70);
            legacy::sub__memcopy(&p.a, p.a.offset + soff, bytes, &q.a, q.a.offset + doff);
            int32 eb = qb_memcopy(&p.b, p.b.offset + soff, bytes, &q.b, q.b.offset + doff);
            if (legacy::last_error != eb) fail("memcopy err", i, legacy::last_error, eb);
            cmp_bytes("memcopy dst", i, q);
            break;
        }
        case 3: { // _MEMGET
            long off = rnd(-2, p.a.size + 2), bytes = rnd(0, 12);
            void *ra = legacy::func__memget(&p.a, p.a.offset + off, bytes);
            int32 eb = 0;
            void *rb = qb_memget(&p.b, p.b.offset + off, bytes, &eb);
            if (legacy::last_error != eb) fail("memget err", i, legacy::last_error, eb);
            if (eb == 0) {
                if ((ptrszint)ra - p.a.offset != (ptrszint)rb - p.b.offset) fail("memget addr", i, 0, 1);
            } else if (memcmp(ra, rb, bytes) != 0) {
                fail("memget fail buffer", i, 0, 1);
            }
            break;
        }
        case 4: { // _MEMEXISTS (live and stale copies)
            legacy::mem_block a = p.a;
            qb_mem_block b = p.b;
            if (rnd(0, 1)) {
                a.lock_id += 1;
                b.lock_id += 1;
            }
            int32 ra = legacy::func__memexists(&a), rb = qb_memexists(&b);
            if (ra != rb) fail("memexists", i, ra, rb);
            break;
        }
        case 5: { // stale copy: _MEMFREE / _MEMFILL / _MEMCOPY / _MEMGET report a dead lock
            legacy::mem_block a = p.a;
            qb_mem_block b = p.b;
            a.lock_id -= 1;
            b.lock_id -= 1;
            legacy::sub__memfree(&a);
            int32 eb = qb_memfree(&b);
            if (legacy::last_error != eb) fail("memfree stale", i, legacy::last_error, eb);
            if (a.lock_id != b.lock_id) fail("memfree stale id", i, (long)a.lock_id, (long)b.lock_id);
            legacy::last_error = 0;
            uint8 v = 7;
            legacy::sub__memcopy(&a, a.offset, 1, &q.a, q.a.offset);
            eb = qb_memcopy(&b, b.offset, 1, &q.b, q.b.offset);
            if (legacy::last_error != eb) fail("memcopy stale", i, legacy::last_error, eb);
            legacy::last_error = 0;
            legacy::sub__memfill(&a, a.offset, 1, (ptrszint)&v, 1);
            eb = qb_memfill(&b, b.offset, 1, &v, 1);
            if (legacy::last_error != eb) fail("memfill stale", i, legacy::last_error, eb);
            break;
        }
        case 6: { // _MEM(offset, size) + _MEMFREE (unsecured locks are freed on both sides)
            long off = (long)rnd(1, 1000000), sz = rnd(-2, 50);
            int ne = rnd(0, 10) == 0;
            legacy::new_error = ne;
            legacy::mem_block a = legacy::func__mem_at_offset(off, sz);
            legacy::new_error = 0;
            qb_mem_block b;
            int32 eb = qb_mem_at_offset(off, sz, ne, &b);
            if (legacy::last_error != eb) fail("mem_at_offset err", i, legacy::last_error, eb);
            cmp_block("mem_at_offset", i, a, b);
            if (a.offset != b.offset) fail("mem_at_offset offset", i, (long)a.offset, (long)b.offset);
            legacy::last_error = 0;
            legacy::sub__memfree(&a);
            eb = qb_memfree(&b);
            if (legacy::last_error != eb) fail("memfree", i, legacy::last_error, eb);
            if (a.lock_id != b.lock_id) fail("memfree id", i, (long)a.lock_id, (long)b.lock_id);
            legacy::last_error = 0;
            legacy::sub__memfree(&a); // second free: 307
            eb = qb_memfree(&b);
            if (legacy::last_error != eb) fail("memfree twice", i, legacy::last_error, eb);
            break;
        }
        case 7: { // scope locks: new/free sequences keep ids in step
            legacy::new_mem_lock();
            legacy::mem_lock *la = legacy::mem_lock_tmp;
            qb_mem_lock *lb = qb_new_mem_lock();
            if (la->id != lb->id || mem_lock_tmp != lb) fail("new_mem_lock", i, (long)la->id, (long)lb->id);
            la->type = lb->type = 3;
            legacy::mem_block a = legacy::func__mem(1234, 8, 1 + 128, 8, la);
            qb_mem_block b;
            qb_mem(1234, 8, 1 + 128, 8, lb, &b);
            cmp_block("func__mem", i, a, b);
            legacy::free_mem_lock(la);
            qb_free_mem_lock(lb);
            if (la->id != lb->id) fail("free_mem_lock", i, (long)la->id, (long)lb->id);
            break;
        }
        case 8: { // _MEMFILL without checks inside a block
            uint8 pat[8];
            for (int k = 0; k < 8; ++k) pat[k] = (uint8)next();
            long doff = rnd(0, p.a.size), dbytes = rnd(0, p.a.size - doff), sb = rnd(1, 8);
            legacy::sub__memfill_nochecks(p.a.offset + doff, dbytes, (ptrszint)pat, sb);
            qb_memfill_nochecks(p.b.offset + doff, dbytes, pat, sb);
            cmp_bytes("memfill_nochecks", i, p);
            break;
        }
        default: { // generated-code style: bump mem_lock_id directly on both sides
            legacy::mem_lock_id++;
            mem_lock_id++;
            if (legacy::mem_lock_id != mem_lock_id) fail("mem_lock_id", i, (long)legacy::mem_lock_id, (long)mem_lock_id);
            break;
        }
        }
    }
    printf("diff_mem: %s, %ld operations, %ld mismatches\n", quick ? "quick" : "full", n, failures);
    return failures == 0 ? 0 : 1;
}
