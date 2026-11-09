// Differential test: zlib 1.3.1 (MinGW libz.a, what _DEFLATE/_INFLATE linked before R10)
// vs Rust qb_runtime::compress (qb_deflate_bound / qb_deflate / qb_inflate).
//
//   g++ -O2 -std=c++17 -w tests/diff/diff_zlib.cpp <libqb_runtime.a> <native libs> -l:libz.a -o diff_zlib
//   ./diff_zlib [--quick]
//
// Checked exactly:
//   * compressBound == qb_deflate_bound for every length tried;
//   * uncompress vs qb_inflate on zlib's and on Rust's compressed data, at many output
//     capacities (0, 1, short, exact, larger) and on truncated input: result code,
//     output length and output bytes;
//   * Rust's compressed bytes are a stream zlib's uncompress reads back exactly.
//   * corrupt streams (bit flips, garbage): result code, length and partial output.
// Reported, not failed: how often the compressed bytes are identical (deviation D-R10-1).

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <vector>
#include <zlib.h>

#include "../../crates/qb-runtime/include/qb_runtime.h"

static uint64_t rng = 0x9E3779B97F4A7C15ull;
static uint64_t next() {
    rng ^= rng << 13;
    rng ^= rng >> 7;
    rng ^= rng << 17;
    return rng;
}

static bool verbose = false;
static int verbose_lines = 0;
static long failures = 0, checks = 0, corrupt_cases = 0, corrupt_code_diff = 0, corrupt_bytes_diff = 0;

static void fail(const char *what, size_t n, long long a, long long b) {
    if (failures++ < 25) printf("MISMATCH %s (input %zu bytes): zlib=%lld rust=%lld\n", what, n, a, b);
}

static std::vector<unsigned char> make_input(size_t n) {
    std::vector<unsigned char> v(n);
    int mode = next() % 5;
    for (size_t i = 0; i < n; i++) {
        uint64_t r = next();
        switch (mode) {
        case 0: v[i] = (unsigned char)r; break;                              // random
        case 1: v[i] = "QBNex BASIC "[i % 12]; break;                       // periodic
        case 2: v[i] = (r % 8) ? v[i ? i - 1 : 0] : (unsigned char)r; break; // runs
        case 3: v[i] = (unsigned char)('A' + r % 4); break;                  // small alphabet
        default: v[i] = (i % 97 < 60) ? "PRINT X\r\n"[i % 9] : (unsigned char)r; break;
        }
    }
    return v;
}

// One uncompress-vs-qb_inflate comparison. corrupt: input may be damaged.
static void compare_inflate(const unsigned char *src, size_t len, uint32_t cap, size_t n, bool corrupt) {
    std::vector<unsigned char> a(cap + 1, 0xAA), b(cap + 1, 0xAA);
    uLongf alen = cap;
    uint32_t blen = cap;
    int ra = uncompress(a.data(), &alen, src, (uLong)len);
    int rb = qb_inflate(src, (uint32_t)len, b.data(), &blen);
    checks++;
    bool same_code = ra == rb && alen == blen;
    bool same_bytes = same_code && memcmp(a.data(), b.data(), alen) == 0;
    if (corrupt) {
        corrupt_cases++;
        if (!same_code) corrupt_code_diff++;
        else if (!same_bytes) corrupt_bytes_diff++;
        if (!same_code) fail("inflate corrupt code/length", n, ra, rb);
        else if (!same_bytes) fail("inflate corrupt partial output", n, 0, 1);
        if (verbose && !same_code && verbose_lines++ < 40) {
            printf("corrupt: zlib=%d/%lu rust=%d/%u cap=%u in=%zu:", ra, (unsigned long)alen, rb, blen, cap, len);
            for (size_t q = 0; q < len && q < 24; q++) printf(" %02x", src[q]);
            printf("\n");
        }
        return;
    }
    if (ra != rb) fail("inflate code", n, ra, rb);
    else if (alen != blen) fail("inflate length", n, alen, blen);
    else if (memcmp(a.data(), b.data(), alen) != 0) fail("inflate bytes", n, 0, 1);
}

int main(int argc, char **argv) {
    bool quick = false;
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--quick") == 0) quick = true;
        if (strcmp(argv[i], "--verbose") == 0) verbose = true;
    }
    int rounds = quick ? 300 : 3000;
    long identical = 0, deflated = 0;

    for (uint32_t n = 0; n < 70000; n += (n < 300 ? 1 : 997)) {
        if (compressBound(n) != qb_deflate_bound(n)) fail("compressBound", n, compressBound(n), qb_deflate_bound(n));
        checks++;
    }

    for (int round = 0; round < rounds; round++) {
        size_t n;
        switch (next() % 4) {
        case 0: n = next() % 16; break;
        case 1: n = next() % 1000; break;
        case 2: n = next() % 70000; break;
        default: n = next() % 400000; break;
        }
        std::vector<unsigned char> in = make_input(n);

        // zlib compress
        uLongf zlen = compressBound(n);
        std::vector<unsigned char> z(zlen);
        compress(z.data(), &zlen, in.data(), n);
        // Rust compress
        uint32_t rlen = qb_deflate_bound(n);
        std::vector<unsigned char> r(rlen);
        int rr = qb_deflate(in.data(), n, r.data(), &rlen);
        if (rr != 0) fail("qb_deflate code", n, 0, rr);
        deflated++;
        if (rlen == zlen && memcmp(r.data(), z.data(), zlen) == 0) identical++;

        // zlib must read Rust's stream back exactly
        {
            std::vector<unsigned char> out(n + 1);
            uLongf olen = n;
            int ro = uncompress(out.data(), &olen, r.data(), rlen);
            checks++;
            if (ro != Z_OK || olen != n || memcmp(out.data(), in.data(), n) != 0) fail("zlib reads rust stream", n, ro, olen);
        }

        uint32_t caps[] = {0, 1, (uint32_t)(n / 2), n ? (uint32_t)n - 1 : 0, (uint32_t)n, (uint32_t)n + 1,
                           (uint32_t)(next() % (n + 50))};
        for (uint32_t cap : caps) {
            compare_inflate(z.data(), zlen, cap, n, false);
            compare_inflate(r.data(), rlen, cap, n, false);
        }
        // truncated input
        for (int t = 0; t < 3; t++) {
            size_t cut = zlen ? next() % zlen : 0;
            compare_inflate(z.data(), cut, (uint32_t)n + 10, n, false);
            compare_inflate(z.data(), cut, (uint32_t)(n / 3), n, false);
        }
        // corrupt input (bit flips)
        for (int t = 0; t < 3 && zlen > 2; t++) {
            std::vector<unsigned char> c(z.begin(), z.begin() + zlen);
            c[2 + next() % (zlen - 2)] ^= (unsigned char)(1u << (next() % 8));
            compare_inflate(c.data(), zlen, (uint32_t)n + 10, n, true);
        }
    }
    // garbage
    for (int g = 0; g < rounds; g++) {
        size_t len = next() % 64;
        std::vector<unsigned char> junk(len);
        for (auto &c : junk) c = (unsigned char)next();
        if (len >= 2 && (next() & 1)) { junk[0] = 0x78; junk[1] = 0x9c; }
        compare_inflate(junk.data(), len, (uint32_t)(next() % 100), len, true);
    }

    printf("diff_zlib: %ld checks, %ld mismatches; compressed bytes identical to zlib in %ld/%ld inputs (D-R10-1); "
           "corrupt streams: %ld, result differs %ld, partial output differs %ld\n",
           checks, failures, identical, deflated, corrupt_cases, corrupt_code_diff, corrupt_bytes_diff);
    return failures ? 1 : 0;
}
