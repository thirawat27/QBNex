// Differential test: legacy synth/PLAY/SOUND/_VOICE/_ADSR/_WAVE (verbatim audio.cpp
// excerpt in tests/legacy/audio_synth*.inc) vs the Rust audio runtime, rendered
// offline (no device) to PCM and compared statement by statement.
//
//   g++ -O2 -std=c++17 -w internal/c/qbrt/tests/diff_audio.cpp <libqbrt.a> <native libs> -o diff_audio
//   ./diff_audio [--quick]
//
// One long session per sample rate, so PLAY's state that persists between
// statements (octave, tempo, length, MB/MF, volume, dots) and the rand()
// sequence used by the noise waveforms are exercised too.
//
// Tolerance: every 16-bit sample may differ by at most 1 LSB (libm sin/pow
// rounding); frame counts and error codes must be identical.
// Environment differences (not legacy code changes):
// - malloc/calloc/realloc are routed through a size-tracking allocator whose
//   realloc zero-fills the grown tail. Legacy PLAY reads that tail
//   uninitialised when a chord (",") is followed by a longer note; Rust
//   defines it as silence (deviation D-61).
// - The ring of raw-queue blocks is replaced by a vector that records frames.

#include <algorithm>
#include <cctype>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>

#ifndef _WIN32
// Rust's noise uses the MSVCRT/UCRT rand() LCG on every platform (D-62);
// give the legacy code the same rand() when it is not built against MSVCRT.
namespace std {
static uint32_t lg_rand_state = 1;
static int lg_rand() {
    lg_rand_state = lg_rand_state * 214013u + 2531011u;
    return (int)((lg_rand_state >> 16) & 0x7fff);
}
} // namespace std
#undef RAND_MAX
#define RAND_MAX 32767
#define rand lg_rand
static void reset_rand() { std::lg_rand_state = 1; }
#else
static void reset_rand() { srand(1); }
#endif

namespace legacy {
typedef uint8_t ma_uint8;
typedef int16_t ma_int16;
typedef uint16_t ma_uint16;
typedef int32_t ma_int32;
typedef uint32_t ma_uint32;
typedef int64_t ma_int64;
typedef uint64_t ma_uint64;
typedef int32_t int32;
typedef intptr_t ptrszint;

static const int QBNEX_SYNTH_VOICE_COUNT = 4;
static const double QBNEX_PI = 3.14159265358979323846;
#define ZERO_VARIABLE(_v_) memset(&(_v_), 0, sizeof(_v_))
#define SAMPLE_FRAME_SIZE(_type_, _channels_) (sizeof(_type_) * (_channels_)) // audio.cpp:67

struct ma_sound {
    int unused;
};
struct qbs {
    uint8_t *chr;
    int32_t len;
};

static int32 new_error = 0;
static int32 last_error = 0;
static void error(int32 n) { last_error = n; }
static ptrszint dblock = 0;
static void Sleep(uint32_t) {}

struct SampleFrameBlockNode {
    bool force;
};
struct SampleFrameBlockQueue {
    std::vector<float> frames;
    SampleFrameBlockNode node{};
    SampleFrameBlockNode *last = nullptr;
    bool PushSampleFrame(float l, float r) {
        frames.push_back(l);
        frames.push_back(r);
        last = &node;
        return true;
    }
    double GetTimeRemaining() { return 0; }
    ma_uint64 GetSampleFramesRemaining() { return frames.size() / 2; }
};

// size-tracking allocator (see header comment)
static void *lg_malloc(size_t n) {
    size_t *p = (size_t *)std::malloc(n + 16);
    if (!p)
        return nullptr;
    p[0] = n;
    return (char *)p + 16;
}
static void *lg_calloc(size_t n, size_t m) {
    void *p = lg_malloc(n * m);
    if (p)
        memset(p, 0, n * m);
    return p;
}
static void lg_free(void *p) {
    if (p)
        std::free((char *)p - 16);
}
static void *lg_realloc(void *p, size_t n) {
    if (!p)
        return lg_calloc(n, 1);
    size_t old = *(size_t *)((char *)p - 16);
    char *q = (char *)std::realloc((char *)p - 16, n + 16);
    if (!q)
        return nullptr;
    *(size_t *)q = n;
    if (n > old)
        memset(q + 16 + old, 0, n - old);
    return q + 16;
}

#define malloc(n) lg_malloc(n)
#define calloc(n, m) lg_calloc(n, m)
#define realloc(p, n) lg_realloc(p, n)
#define free(p) lg_free(p)

#include "legacy/audio_synth_types.inc"

struct AudioEngineStub {
    bool isInitialized;
    ma_uint32 sampleRate;
    SynthVoice synthVoices[QBNEX_SYNTH_VOICE_COUNT];
    int32_t currentSynthVoice;
    int32_t nextSynthVoice;
    bool synthVoicePinned;
};
static AudioEngineStub audioEngine;

#include "legacy/audio_synth.inc"

#undef malloc
#undef rand
#undef calloc
#undef realloc
#undef free
} // namespace legacy

#include "../include/qbrt_audio.h"

static uint64_t rng = 0x9E3779B97F4A7C15ull;
static uint64_t next() {
    rng ^= rng << 13;
    rng ^= rng >> 7;
    rng ^= rng << 17;
    return rng;
}
static int pick(int n) { return (int)(next() % (uint64_t)n); }

static std::vector<uint8_t> dblock_mem(70000);

static std::string num(int lo, int hi) { return std::to_string(lo + pick(hi - lo + 1)); }

static std::string random_mml() {
    static const char *notes = "ABCDEFGabcdefg";
    std::string s;
    int tokens = 1 + pick(10);
    for (int t = 0; t < tokens; ++t) {
        int k = pick(100);
        if (k < 40) {
            s += notes[pick(14)];
            int m = pick(8);
            if (m == 0) s += '#';
            if (m == 1) s += '+';
            if (m == 2) s += '-';
            if (pick(3) == 0) s += num(0, 64);
            int dots = pick(6) == 0 ? 1 + pick(2) : 0;
            s.append(dots, '.');
            if (pick(8) == 0) s += ',';
        } else if (k < 46) {
            s += "O" + num(0, 7);
        } else if (k < 50) {
            s += pick(2) ? '<' : '>';
        } else if (k < 56) {
            s += "L" + num(0, 66);
        } else if (k < 62) {
            s += "T" + num(20, 260);
        } else if (k < 67) {
            s += "P" + num(0, 65);
            if (pick(4) == 0) s += '.';
            if (pick(8) == 0) s += ',';
        } else if (k < 71) {
            s += "N" + num(0, 86);
            if (pick(5) == 0) s += '.';
        } else if (k < 79) {
            static const char *m[] = {"MN", "ML", "MS", "MB", "MF", "mb", "mf", "MX", "M"};
            s += m[pick(9)];
        } else if (k < 84) {
            s += "V" + num(0, 102);
        } else if (k < 88) {
            s += ' ';
        } else if (k < 91) {
            // "=" + VARPTR$: command letter, '=', type byte, 16-bit offset
            static const char cmds[] = "OLTVNP";
            static const uint8_t types[] = {1, 129, 2, 130, 4, 5, 133, 6, 8, 20, 148, 3, 64 + 4, 64 + 128 + 4, 64 + 57, 64 + 128 + 8};
            s += cmds[pick(6)];
            s += '=';
            s += (char)types[pick(16)];
            uint16_t off = (uint16_t)pick(65000);
            s += (char)(off & 255);
            s += (char)(off >> 8);
        } else if (k < 93) {
            static const char junk[] = "XZ;=!\t.0";
            s += junk[pick(8)];
        } else {
            s += num(0, 9);
        }
    }
    return s;
}

static std::string random_wave() {
    static const char *w[] = {"TRIANGLE", "sine", " Square ", "SAW", "sawtooth", "NOISE", "white noise", "PINK", "pink noise", "BROWN",
                              "brownian noise", "LFSR", "lfsr noise", "CUSTOM:0,1,0,-1", "custom:16384;-32768;0", "0.5, -0.5", "CUSTOM:",
                              "1,x", "BOGUS", "", "custom: 0x1p-1, .25, , 1e0", "CUSTOM:inf,-nan", "a:b,c"};
    return w[pick(23)];
}

struct Session {
    uint32_t rate;
    legacy::qbs q;
    qbrt_synth *rust;
    long statements = 0, frames = 0, exact_diff = 0, failures = 0;
};

static int legacy_call(Session &s, int kind, const std::string &text, double a, double b, double c, double d, int voice, int passed) {
    legacy::last_error = 0;
    legacy::qbs str;
    str.chr = (uint8_t *)text.data();
    str.len = (int32_t)text.size();
    switch (kind) {
    case 0: legacy::sub_play(&str); break;
    case 1: legacy::sub_sound(a, b); break;
    case 2: legacy::sub__voice(voice); break;
    case 3: legacy::sub__adsr(a, b, c, d, voice, passed); break;
    case 4: legacy::sub__wave(&str, voice, passed); break;
    }
    return legacy::last_error;
}

static int rust_call(Session &s, int kind, const std::string &text, double a, double b, double c, double d, int voice, int passed) {
    const uint8_t *t = (const uint8_t *)text.data();
    int n = (int)text.size();
    switch (kind) {
    case 0: return qbrt_synth_play(s.rust, t, n, dblock_mem.data());
    case 1: return qbrt_synth_sound(s.rust, a, b);
    case 2: return qbrt_synth_voice(s.rust, voice);
    case 3: return qbrt_synth_adsr(s.rust, a, b, c, d, voice, passed);
    case 4: return qbrt_synth_wave(s.rust, t, n, voice, passed);
    }
    return -1;
}

static void describe(int kind, const std::string &text, double a, double b, double c, double d, int voice, int passed) {
    static const char *names[] = {"PLAY", "SOUND", "_VOICE", "_ADSR", "_WAVE"};
    printf("  %s text=\"", names[kind]);
    for (unsigned char ch : text)
        printf(ch >= 32 && ch < 127 ? "%c" : "\\x%02x", ch);
    printf("\" a=%g b=%g c=%g d=%g voice=%d passed=%d\n", a, b, c, d, voice, passed);
}

static void run_session(Session &s, long count) {
    legacy::audioEngine.isInitialized = true;
    legacy::audioEngine.sampleRate = s.rate;
    for (int v = 0; v < legacy::QBNEX_SYNTH_VOICE_COUNT; ++v) {
        legacy::audioEngine.synthVoices[v] = legacy::SynthVoice();
        legacy::audioEngine.synthVoices[v].rawQueue = new legacy::SampleFrameBlockQueue();
    }
    legacy::audioEngine.currentSynthVoice = legacy::audioEngine.nextSynthVoice = 0;
    legacy::audioEngine.synthVoicePinned = false;
    s.rust = qbrt_synth_new(s.rate);
    reset_rand(); // rust starts its rand() at seed 1 as well
    // re-sync PLAY state left by a previous session (legacy keeps it in statics)
    legacy_call(s, 0, "T120L4O4MNMFV50", 0, 0, 0, 0, 0, 0);
    for (int v = 0; v < 4; ++v)
        legacy::audioEngine.synthVoices[v].rawQueue->frames.clear();
    legacy::audioEngine.nextSynthVoice = 0;

    std::vector<float> buf;
    for (long i = 0; i < count; ++i) {
        int kind;
        int k = pick(100);
        kind = k < 70 ? 0 : k < 82 ? 1 : k < 87 ? 2 : k < 93 ? 3 : 4;
        std::string text;
        double a = 0, b = 0, c = 0, d = 0;
        int voice = 0, passed = 0;
        switch (kind) {
        case 0: text = random_mml(); break;
        case 1: {
            static const double f[] = {0, 36, 37, 100, 440, 1000, 5000, 19999, 20000, 32767, 32768};
            a = pick(3) ? 37 + pick(4000) + pick(1000) / 1000.0 : f[pick(11)];
            b = pick(10) == 0 ? (pick(2) ? -1 : 70000) : pick(40) / 10.0;
            break;
        }
        case 2: voice = pick(7) - 1; break;
        case 3:
            a = pick(10) == 0 ? -0.1 : pick(100) / 400.0;
            b = pick(100) / 400.0;
            c = pick(4) == 0 ? pick(120) : pick(100) / 100.0;
            d = pick(100) / 400.0;
            voice = pick(6);
            passed = pick(2) ? 16 : 0;
            break;
        case 4:
            text = random_wave();
            voice = pick(6);
            passed = pick(2) ? 2 : 0;
            break;
        }
        int el = legacy_call(s, kind, text, a, b, c, d, voice, passed);
        int er = rust_call(s, kind, text, a, b, c, d, voice, passed);
        ++s.statements;
        bool bad = el != er;
        for (int v = 0; v < 4 && !bad; ++v) {
            std::vector<float> &lf = legacy::audioEngine.synthVoices[v].rawQueue->frames;
            long nl = (long)lf.size() / 2;
            long nr = qbrt_synth_queued(s.rust, v);
            if (nl != nr) {
                if (s.failures < 20)
                    printf("frame count voice %d: legacy %ld rust %ld\n", v, nl, nr);
                bad = true;
                break;
            }
            buf.resize(lf.size() + 2);
            qbrt_synth_drain(s.rust, v, buf.data(), (int)nr);
            for (long j = 0; j < nl * 2; ++j) {
                long x = lrintf(lf[j] * 32768.0f), y = lrintf(buf[j] * 32768.0f);
                if (x != y) {
                    if (getenv("DIFF_AUDIO_DEBUG") && s.exact_diff < 8) {
                        printf("lsb rate=%u stmt=%ld voice=%d wave=%d sample=%ld legacy=%ld rust=%ld\n", s.rate, i, v,
                               (int)legacy::audioEngine.synthVoices[v].waveform, j, x, y);
                        describe(kind, text, a, b, c, d, voice, passed);
                    }
                    ++s.exact_diff;
                    if (labs(x - y) > 1) {
                        if (s.failures < 20)
                            printf("sample %ld voice %d: legacy %ld rust %ld\n", j, v, x, y);
                        bad = true;
                        break;
                    }
                }
            }
            s.frames += nl;
            lf.clear();
        }
        if (bad) {
            if (s.failures < 20) {
                printf("MISMATCH rate=%u stmt=%ld err legacy=%d rust=%d\n", s.rate, i, el, er);
                describe(kind, text, a, b, c, d, voice, passed);
            }
            ++s.failures;
            // resynchronise both sides
            for (int v = 0; v < 4; ++v) {
                legacy::audioEngine.synthVoices[v].rawQueue->frames.clear();
                buf.resize((size_t)qbrt_synth_queued(s.rust, v) * 2 + 2);
                qbrt_synth_drain(s.rust, v, buf.data(), qbrt_synth_queued(s.rust, v));
            }
        }
    }
    qbrt_synth_free(s.rust);
    for (int v = 0; v < 4; ++v)
        delete legacy::audioEngine.synthVoices[v].rawQueue;
}

int main(int argc, char **argv) {
    const bool quick = argc > 1 && strcmp(argv[1], "--quick") == 0;
    const long n = quick ? 1500 : 20000;
    for (auto &b : dblock_mem)
        b = (uint8_t)next();
    legacy::dblock = (legacy::ptrszint)dblock_mem.data();

    const uint32_t rates[] = {8000, 22050, 44100, 48000};
    long failures = 0, total = 0, frames = 0, exact = 0;
    for (uint32_t r : rates) {
        Session s{};
        s.rate = r;
        run_session(s, r >= 44100 ? n / 4 : n);
        printf("diff_audio rate %u: %ld statements, %ld frames, %ld samples off by 1 LSB, %ld mismatches\n", r, s.statements, s.frames,
               s.exact_diff, s.failures);
        failures += s.failures;
        total += s.statements;
        frames += s.frames;
        exact += s.exact_diff;
    }
    printf("diff_audio: %ld statements, %ld frames compared, %ld mismatches\n", total, frames, failures);
    return failures ? 1 : 0;
}
