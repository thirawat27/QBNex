// Differential test: legacy _LOADIMAGE decoders (dr_pcx + stb_image v2.27 and the 8bpp quantizer/palette
// remapper from internal/c/parts/video/image/src.c, copied in tests/legacy/image_codec.inc) vs Rust
// qbrt::image.
//
// dr_pcx.h and stb_image.h are no longer in the tree; run_diff extracts them from git history into a
// temporary directory that is passed with -I.
//
//   g++ -O2 -std=c++17 -w -I<legacy headers> internal/c/qbrt/tests/diff_image.cpp <libqbrt.a> <libs> -o diff_image
//   ./diff_image <corpus dir> [--quick]      compare every corpus file, fuzzed variants and the quantizer
//   ./diff_image <corpus dir> --dump         print "name w h fnv1a(BGRA)" from the legacy decoder
//
// Exactness: every lossless format must match bit for bit (size, success/failure and every BGRA byte).
// JPEG is lossy and the two IDCT/upsampling implementations differ, so it is held to the documented
// tolerance (deviation D-R7-2): same size, mean absolute error below 1 and at most 1% of samples off by
// more than 12. (Chroma upsampling differs most on noisy content, where single samples can be far off.)

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <string>
#include <vector>

typedef uint8_t uint8;
typedef int32_t int32;

#include "legacy/image_codec.inc"

#include "../include/qbrt.h"

static const int JPEG_TOLERANCE = 12;       // per-sample difference that counts as an outlier
static const double JPEG_MEAN_TOLERANCE = 1.0; // mean absolute difference per sample
static const size_t JPEG_OUTLIER_PERCENT = 1;  // outliers allowed, in percent of samples

static uint64_t rng = 0x9E3779B97F4A7C15ull;
static uint64_t next() {
    rng ^= rng << 13;
    rng ^= rng >> 7;
    rng ^= rng << 17;
    return rng;
}

static uint64_t fnv1a(const uint8_t *p, size_t n) {
    uint64_t h = 0xcbf29ce484222325ull;
    for (size_t i = 0; i < n; ++i) {
        h ^= p[i];
        h *= 0x100000001b3ull;
    }
    return h;
}

// Same steps as the legacy func__loadimage: dr_pcx first, then stb_image, then RGBA -> BGRA.
static uint8_t *legacy_decode(std::vector<uint8_t> &content, int32_t *w, int32_t *h) {
    int32_t result = 0;
    uint8_t *data = content.empty() ? (uint8_t *)"" : content.data();
    uint8_t *pixels = image_decode_drpcx(data, (int32_t)content.size(), &result, w, h);
    if (!(result & 1))
        pixels = image_decode_stbi(data, (int32_t)content.size(), &result, w, h);
    if (!(result & 1))
        return nullptr;
    for (int64_t i = 0; i < (int64_t)*w * *h; ++i) {
        uint8_t t = pixels[i * 4];
        pixels[i * 4] = pixels[i * 4 + 2];
        pixels[i * 4 + 2] = t;
    }
    return pixels;
}

static std::vector<uint8_t> read_file(const std::filesystem::path &p) {
    std::ifstream f(p, std::ios::binary);
    return std::vector<uint8_t>((std::istreambuf_iterator<char>(f)), std::istreambuf_iterator<char>());
}

// dr_pcx's width * height * components wraps in 32 bits for these headers.
static bool pcx_size_overflows(const std::vector<uint8_t> &c) {
    if (c.size() < 128 || c[0] != 10 || c[2] != 1) return false;
    if (c[3] != 1 && c[3] != 2 && c[3] != 4 && c[3] != 8) return false;
    auto f = [&](int o) { return (uint32_t)(c[o] | c[o + 1] << 8); };
    uint32_t l = f(4), t = f(6), r = f(8), b = f(10);
    uint64_t w = (l > r ? l - r : r - l) + 1, h = (t > b ? t - b : b - t) + 1;
    uint64_t comps = (c[3] == 8 && c[65] == 4) ? 4 : 3;
    return w * h * comps > 0xFFFFFFFFull;
}

// Pixel count a corrupted header asks for (0 = unknown). Fuzz inputs above HUGE_PIXELS are skipped
// both decoders then allocate gigabytes, which only tests the machine's memory.
static const uint64_t HUGE_PIXELS = 16u << 20;
static uint64_t declared_pixels(const std::vector<uint8_t> &c) {
    auto le16 = [&](size_t o) { return o + 2 <= c.size() ? (uint64_t)(c[o] | c[o + 1] << 8) : 0; };
    auto le32 = [&](size_t o) { return o + 4 <= c.size() ? (uint64_t)(uint32_t)(c[o] | c[o + 1] << 8 | c[o + 2] << 16 | (uint32_t)c[o + 3] << 24) : 0; };
    auto be32 = [&](size_t o) { return o + 4 <= c.size() ? (uint64_t)((uint32_t)c[o] << 24 | c[o + 1] << 16 | c[o + 2] << 8 | c[o + 3]) : 0; };
    if (c.size() >= 128 && c[0] == 10) {
        uint64_t l = le16(4), t = le16(6), r = le16(8), b = le16(10);
        return ((l > r ? l - r : r - l) + 1) * ((t > b ? t - b : b - t) + 1);
    }
    if (c.size() > 24 && c[0] == 0x89 && c[1] == 'P') return be32(16) * be32(20);
    if (c.size() > 26 && c[0] == 'B' && c[1] == 'M') {
        int32_t h = (int32_t)le32(22);
        return le32(14) == 12 ? le16(18) * le16(20) : le32(18) * (uint64_t)(h < 0 ? -(int64_t)h : h);
    }
    if (c.size() > 10 && c[0] == 'G' && c[1] == 'I') return le16(6) * le16(8);
    if (c.size() > 2 && c[0] == 0xFF && c[1] == 0xD8) {
        for (size_t i = 2; i + 9 < c.size(); ++i)
            if (c[i] == 0xFF && c[i + 1] >= 0xC0 && c[i + 1] <= 0xCF && c[i + 1] != 0xC4 && c[i + 1] != 0xC8 && c[i + 1] != 0xCC)
                return (uint64_t)(c[i + 5] << 8 | c[i + 6]) * (uint64_t)(c[i + 7] << 8 | c[i + 8]);
        return 0;
    }
    if (c.size() > 18) return le16(12) * le16(14); // TGA (and harmless for anything else)
    return 0;
}

static bool is_jpeg(const std::vector<uint8_t> &c) { return c.size() > 2 && c[0] == 0xFF && c[1] == 0xD8; }

struct Outcome {
    bool ok;
    std::string why;
};

static Outcome compare(std::vector<uint8_t> &content, bool lossy_allowed) {
    int32_t lw = -1, lh = -1, rw = -1, rh = -1;
    std::vector<uint8_t> copy = content; // legacy decoders must not see Rust's buffer and vice versa
    uint8_t *a = legacy_decode(copy, &lw, &lh);
    uint8_t *b = qbrt_image_decode(content.empty() ? nullptr : content.data(), (int32_t)content.size(), &rw, &rh);
    Outcome o{true, ""};
    char buf[256];
    if (!a || !b) {
        if (a || b) {
            snprintf(buf, sizeof buf, "legacy %s, rust %s", a ? "decoded" : "failed", b ? "decoded" : "failed");
            o = {false, buf};
        }
    } else if (lw != rw || lh != rh) {
        snprintf(buf, sizeof buf, "size legacy %dx%d rust %dx%d", lw, lh, rw, rh);
        o = {false, buf};
    } else {
        size_t n = (size_t)lw * lh * 4;
        if (lossy_allowed && is_jpeg(content)) {
            int maxd = 0;
            size_t over = 0;
            double sum = 0;
            for (size_t i = 0; i < n; ++i) {
                int d = abs((int)a[i] - (int)b[i]);
                if (d > maxd) maxd = d;
                over += d > JPEG_TOLERANCE;
                sum += d;
            }
            snprintf(buf, sizeof buf, "jpeg max diff %d, mean %.3f, %.3f%% of samples above %d", maxd, sum / n,
                     100.0 * over / n, JPEG_TOLERANCE);
            if (getenv("DIFF_IMAGE_VERBOSE")) printf("  %s\n", buf);
            if (sum / n >= JPEG_MEAN_TOLERANCE || over * 100 > n * JPEG_OUTLIER_PERCENT) o = {false, buf};
        } else if (memcmp(a, b, n) != 0) {
            size_t i = 0;
            while (a[i] == b[i]) ++i;
            snprintf(buf, sizeof buf, "pixel %zu (x=%zu y=%zu ch=%zu) legacy %d rust %d", i / 4, (i / 4) % lw,
                     (i / 4) / lw, i % 4, a[i], b[i]);
            o = {false, buf};
        }
    }
    free(a);
    free(b);
    return o;
}

static long check_quantizer(long iterations) {
    long failures = 0;
    for (long it = 0; it < iterations; ++it) {
        int32_t w = 1 + next() % 70, h = 1 + next() % 50;
        std::vector<uint8_t> img((size_t)w * h * 4);
        int style = next() % 3;
        for (auto &c : img) c = style == 0 ? (uint8_t)next() : style == 1 ? (uint8_t)(next() % 8 * 36) : (uint8_t)(next() % 2 * 255);
        uint32_t pal_a[256], pal_b[256];
        std::vector<uint8_t> copy = img;
        uint8_t *qa = image_convert_8bpp(copy.data(), w, h, pal_a);
        uint8_t *qb = qbrt_image_to_8bpp(img.data(), w, h, pal_b);
        bool bad = memcmp(qa, qb, (size_t)w * h) != 0 || memcmp(pal_a, pal_b, sizeof pal_a) != 0;
        // remap the quantized image onto a random destination palette (sometimes with duplicates)
        uint32_t dst[256];
        for (auto &d : dst) d = next() % 4 == 0 ? 0xFF000000u : (uint32_t)next() | 0xFF000000u;
        if (!bad) {
            qbrt_image_remap_palette(qb, w, h, pal_b, dst);
            image_remap_palette(qa, w, h, pal_a, dst);
            bad = memcmp(qa, qb, (size_t)w * h) != 0;
        }
        if (bad && failures++ < 10) printf("QUANTIZER MISMATCH iteration %ld (%dx%d style %d)\n", it, w, h, style);
        free(qa);
        free(qb);
    }
    return failures;
}

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: diff_image <corpus dir> [--quick|--dump]\n");
        return 2;
    }
    const bool quick = argc > 2 && strcmp(argv[2], "--quick") == 0;
    const bool dump = argc > 2 && strcmp(argv[2], "--dump") == 0;
    std::vector<std::filesystem::path> files;
    for (auto &e : std::filesystem::directory_iterator(argv[1]))
        if (e.is_regular_file() && e.path().extension() != ".py" && e.path().extension() != ".md") files.push_back(e.path());
    std::sort(files.begin(), files.end());

    if (dump) {
        for (auto &p : files) {
            auto c = read_file(p);
            int32_t w = 0, h = 0;
            uint8_t *px = legacy_decode(c, &w, &h);
            if (px)
                printf("(\"%s\", %d, %d, 0x%016llx),\n", p.filename().string().c_str(), w, h,
                       (unsigned long long)fnv1a(px, (size_t)w * h * 4));
            else
                printf("(\"%s\", FAIL),\n", p.filename().string().c_str());
            free(px);
        }
        return 0;
    }

    long failures = 0, jpegs = 0;
    for (auto &p : files) {
        auto c = read_file(p);
        jpegs += is_jpeg(c);
        Outcome o = compare(c, true);
        if (!o.ok) {
            ++failures;
            printf("MISMATCH %s: %s\n", p.filename().string().c_str(), o.why.c_str());
        }
    }
    printf("diff_image corpus: %zu files (%ld JPEG within tolerance %d), %ld mismatches\n", files.size(), jpegs,
           JPEG_TOLERANCE, failures);

    // Fuzzing: corrupt corpus files. Rust must never crash. PCX is our own port of dr_pcx and must
    // match exactly even on garbage; for the other formats the decoders are different libraries, so
    // corrupt input may be accepted by one and rejected by the other: those are only counted.
    long fuzz_n = quick ? 3000 : 60000, pcx_fail = 0, other_diff = 0, overflow_skipped = 0, huge_skipped = 0;
    for (long i = 0; i < fuzz_n; ++i) {
        auto c = read_file(files[next() % files.size()]);
        if (c.empty()) continue;
        int kind = next() % 3;
        if (kind == 0) c.resize(next() % c.size());
        int flips = 1 + next() % 4;
        for (int f = 0; f < flips && !c.empty(); ++f) {
            size_t pos = next() % 4 == 0 ? next() % std::min<size_t>(c.size(), 140) : next() % c.size();
            c[pos] ^= (uint8_t)(1 + next() % 255);
        }
        if (const char *trace = getenv("DIFF_IMAGE_TRACE")) { // keep the input in case a decoder crashes
            std::ofstream t(trace, std::ios::binary | std::ios::trunc);
            t.write((const char *)c.data(), (std::streamsize)c.size());
        }
        bool pcx = !c.empty() && c[0] == 10;
        if (pcx && pcx_size_overflows(c)) {
            // dr_pcx would compute a wrapped 32-bit size and overflow the heap (D-R7-1): Rust must fail.
            int32_t w, h;
            uint8_t *b = qbrt_image_decode(c.data(), (int32_t)c.size(), &w, &h);
            if (b && pcx_fail++ < 10) printf("PCX FUZZ %ld: Rust decoded an image whose size overflows\n", i);
            free(b);
            ++overflow_skipped;
            continue;
        }
        if (declared_pixels(c) > HUGE_PIXELS) {
            ++huge_skipped;
            continue;
        }
        Outcome o = compare(c, true);
        if (!o.ok) {
            if (pcx) {
                if (pcx_fail++ < 10) printf("PCX FUZZ MISMATCH %ld: %s\n", i, o.why.c_str());
                if (const char *trace = getenv("DIFF_IMAGE_TRACE")) {
                    std::ofstream t(std::string(trace) + "." + std::to_string(i) + ".pcx", std::ios::binary);
                    t.write((const char *)c.data(), (std::streamsize)c.size());
                }
            } else
                ++other_diff;
        }
    }
    printf("diff_image fuzz: %ld corrupted inputs, %ld PCX mismatches, %ld non-PCX disagreements (informational), "
           "%ld PCX size overflows checked on Rust only, %ld huge headers skipped\n",
           fuzz_n, pcx_fail, other_diff, overflow_skipped, huge_skipped);
    failures += pcx_fail;

    long qf = check_quantizer(quick ? 300 : 5000);
    printf("diff_image quantizer: %ld mismatches\n", qf);
    failures += qf;
    printf("diff_image: %s\n", failures ? "FAILED" : "OK");
    return failures ? 1 : 0;
}
