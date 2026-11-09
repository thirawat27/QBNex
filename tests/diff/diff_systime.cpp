// Differential test: legacy start-of-day setup from main() (tests/legacy/time_init.inc,
// verbatim) vs Rust qb_runtime::systime (qb_day_ms), plus a live check that
// qb_time_init and qb_get_ticks agree with the C library clock.
//
// time(), localtime() and GetTicks() are replaced by scripted mocks so every
// path of the legacy block runs: normal second tick-over, time() failing at
// once, localtime() failing (NULL).
//
//   g++ -O1 -std=c++17 -w tests/diff/diff_systime.cpp <libqb_runtime.a> <native libs> -o diff_systime
//   ./diff_systime [--quick] [--mutate]

#include <cstdint>
#include <cstdio>
#include <cstring>
#include <ctime>
#include <vector>

#include "../../crates/qb-runtime/include/qb_runtime.h"

typedef int32_t int32;
typedef uint32_t uint32;
typedef int64_t int64;

namespace legacy {
typedef int64 time_t;
static std::vector<time_t> script;  // successive time() results
static size_t pos = 0;
static int calls = 0;
static void time(time_t *t) {
    ++calls;
    *t = pos < script.size() ? script[pos++] : script.back();
}
struct tm {
    int tm_sec, tm_min, tm_hour;
};
static tm fake_tm;
static bool localtime_fails = false;
static tm *localtime(time_t *) { return localtime_fails ? nullptr : &fake_tm; }
static int64 fake_ticks = 0;
static int64 GetTicks() { return fake_ticks; }
static uint32 clock_firsttimervalue, qbnex_firsttimervalue;

static void start_of_day() {
    tm *qbnex_tm;
    time_t qbnex_tm_val;
    time_t qbnex_tm_val_old;
#include "legacy/time_init.inc"
}
}  // namespace legacy

static uint64_t rng = 0x2545F4914F6CDD1Dull;
static uint64_t next() {
    rng ^= rng << 13;
    rng ^= rng >> 7;
    rng ^= rng << 17;
    return rng;
}

int main(int argc, char **argv) {
    bool quick = false, mutate = false;
    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "--quick")) quick = true;
        if (!strcmp(argv[i], "--mutate")) mutate = true;
    }
    const long n = quick ? 100000 : 2000000;
    long failures = 0;
    for (long k = 0; k < n; ++k) {
        int32 h = (int32)(next() % 24), m = (int32)(next() % 60), s = (int32)(next() % 61);
        bool clock_fails = next() % 50 == 0;
        legacy::localtime_fails = next() % 50 == 0;
        legacy::fake_tm = {s, m, h};
        legacy::fake_ticks = (int64)next();
        legacy::script.clear();
        if (clock_fails) {
            legacy::script.push_back(-1);
        } else {
            int64 t0 = (int64)(next() % 4000000000ull);
            int reps = (int)(next() % 5);
            for (int r = 0; r <= reps; r++) legacy::script.push_back(t0);
            legacy::script.push_back(t0 + 1);
        }
        legacy::pos = 0;
        legacy::calls = 0;
        legacy::start_of_day();

        // Rust: wait_for_second_change reads the clock once when it fails, and
        // otherwise until the value changes (checked in its unit tests); here the
        // day value and the tick snapshot are compared.
        bool valid = !legacy::localtime_fails;
        uint32 day = qb_day_ms((uint32)h, (uint32)m, (uint32)s, valid);
        if (mutate && valid) day += 1000;
        size_t expected_calls = clock_fails ? 1 : legacy::script.size();
        if (day != legacy::qbnex_firsttimervalue || legacy::clock_firsttimervalue != (uint32)legacy::fake_ticks ||
            (size_t)legacy::calls != expected_calls) {
            if (failures < 10)
                printf("MISMATCH %02d:%02d:%02d valid=%d: legacy=%u rust=%u\n", h, m, s, valid,
                       legacy::qbnex_firsttimervalue, day);
            ++failures;
        }
    }

    // Live: the Rust start-up values agree with the C library's local time.
    uint32 clock_first, day_first;
    qb_time_init(&clock_first, &day_first);
    std::time_t now = std::time(nullptr);
    std::tm *lt = std::localtime(&now);
    uint32 c_day = (uint32)((lt->tm_hour * 3600 + lt->tm_min * 60 + lt->tm_sec) * 1000);
    int64 diff = (int64)c_day - (int64)day_first;
    bool live_ok = diff >= 0 && diff <= 2000;  // the C read happens just after
    int64 t1 = qb_get_ticks();
    bool ticks_ok = t1 >= (int64)clock_first && t1 - (int64)clock_first < 1000;
    if (!live_ok || !ticks_ok) {
        printf("LIVE MISMATCH: rust day=%u C day=%u ticks %lld vs first %u\n", day_first, c_day, (long long)t1, clock_first);
        ++failures;
    }
    printf("diff_systime: %ld start-up scripts + live clock check: %ld mismatches\n", n, failures);
    return failures ? 1 : 0;
}
