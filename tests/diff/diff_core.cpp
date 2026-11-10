// Differential test for R12b (core runtime): verbatim legacy code in tests/diff/legacy/core_*.inc
// vs the Rust modules numparse, error, alloc, arrays, mathfn, events and waits of qb_runtime.
//
//   g++ -O2 -std=c++17 -w tests/diff/diff_core.cpp <libqb_runtime.a> <native libs> -o diff_core
//   ./diff_core [--quick] [--mutate]
//
// --mutate perturbs the legacy side slightly (one constant per area) to prove each area can fail.

#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <memory>
#include <mutex>
#include <new>
#include <string>
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

#define ISFLOAT 536870912
#define ISUNSIGNED 268435456
#ifndef MB_OK
#define MB_OK 0
#define MB_YESNO 4
#define MB_SYSTEMMODAL 0x1000
#define IDOK 1
#define IDNO 7
#endif

static bool g_mutate = false;
static long g_fail = 0;
static long g_checks = 0;
#define CHECK(cond, ...)                                                                                               \
    do {                                                                                                               \
        ++g_checks;                                                                                                    \
        if (!(cond)) {                                                                                                 \
            if (g_fail < 25) {                                                                                         \
                printf(__VA_ARGS__);                                                                                   \
                printf("\n");                                                                                          \
            }                                                                                                          \
            ++g_fail;                                                                                                  \
        }                                                                                                              \
    } while (0)

static uint64_t rng = 0x9E3779B97F4A7C15ull;
static uint64_t next() {
    rng ^= rng << 13;
    rng ^= rng >> 7;
    rng ^= rng << 17;
    return rng;
}
static int64 rnd(int64 n) { return n <= 0 ? 0 : (int64)(next() % (uint64)n); }

// ------------------------------------------------------------- legacy: numbers + READ
namespace legacy_num {
static int32 last_error = 0;
static uint32 new_error = 0;
static void error(int32 n) { last_error = n; }
#include "legacy/qbr_x87.inc"
#include "qbs_stub.h"
int64 build_int64(uint32 val2, uint32 val1) {
    int64 val = static_cast<int64>(val2);
    val <<= 32;
    val |= val1;
    return val;
}
uint64 build_uint64(uint32 val2, uint32 val1) {
    uint64 val = static_cast<uint64>(val2);
    val <<= 32;
    val |= val1;
    return val;
}
static int32 fake_input_result = 0;
int32 n_inputnumberfromfile(int32 fileno) { return fake_input_result; }
#include "legacy/core_numbers.inc"
} // namespace legacy_num

// ------------------------------------------------------------- legacy: errors
namespace legacy_err {
struct LegacyExit {
    int code;
};
[[noreturn]] static void exit(int c) { throw LegacyExit{c}; }
#include "qbs_stub.h"
static uint32 new_error = 0, error_err = 0, error_occurred = 0, error_goto_line = 0, error_handling = 0, qbevent = 0;
static double error_erl = 0, last_line = 0;
static uint32 ercl = 0, inclercl = 0;
static char *includedfilename = NULL;
static uint8 close_program = 0;
static int ended = 0, qbmain = 0;
static std::string box_msg, box_title;
static int box_type = -1, box_answer = 0;
static int MessageBox2(int ignore, const char *message, const char *title, int type) {
    box_msg = message;
    box_title = title;
    box_type = type;
    return box_answer;
}
static void end() { ended = 1; }
static void QBMAIN(void *) { qbmain = 1; }
static std::string command0;
qbs *func_command(int32 index, int32 passed) { return qbs_new_txt_len(command0.data(), (int32)command0.size()); }
qbs *func_mid(qbs *str, int32 start, int32 l, int32 passed) {
    int32 from = start - 1;
    if (from > str->len) from = str->len;
    return qbs_new_txt_len((char *)str->chr + from, str->len - from);
}
#include "legacy/core_error.inc"
} // namespace legacy_err

// ------------------------------------------------------------- legacy: allocators
namespace legacy_alloc {
static int32 last_error = 0;
static void error(int32 n) {
    if (!last_error) last_error = n;
}
uint8 cmem[1114099];
uint8 *cmem_static_pointer = &cmem[0] + 1280 + 65536;
uint8 *cmem_dynamic_base = &cmem[0] + 655360;
uint8 *mem_static;
uint8 *mem_static_pointer;
uint8 *mem_static_limit;
#include "legacy/core_alloc.inc"
void init_mem_static(uint32 size) { allocate_mem_static_block(size); }
} // namespace legacy_alloc

// ------------------------------------------------------------- legacy: arrays, math, radix
namespace legacy_arr {
static int32 last_error = 0;
static void error(int32 n) { last_error = n; }
#include "qbs_stub.h"
#include "legacy/core_arrays_math.inc"
} // namespace legacy_arr

// ------------------------------------------------------------- legacy: waits
namespace legacy_wait {
static int32 last_error = 0;
static uint32 new_error = 0;
static void error(int32 n) { last_error = n; }
static std::vector<int64> times;
static size_t at = 0;
static std::vector<int64> trace;
static int64 evnts = 0, break_after = -1, stop_after = -1;
static int32 sleep_break = 0;
static int32 stop_program = 0;
static int64 GetTicks() {
    int64 t = times[at < times.size() ? at : times.size() - 1];
    ++at;
    return t;
}
static void Sleep(int64 ms) { trace.push_back(ms); }
static void evnt(int32) {
    ++evnts;
    trace.push_back(-1);
    if (break_after >= 0 && evnts >= break_after) sleep_break = 1;
    if (stop_after >= 0 && evnts >= stop_after) stop_program = 1;
}
#include "legacy/core_waits.inc"
} // namespace legacy_wait

// ------------------------------------------------------------- legacy: events
struct ontimer_struct {
    uint8 allocated;
    uint32 id;
    int64 pass;
    uint8 active;
    uint8 state;
    double seconds;
    double last_time;
};
struct onkey_struct {
    uint32 id;
    int64 pass;
    uint8 active;
    uint8 state;
    uint32 keycode;
    uint32 keycode_alternate;
    uint8 key_scancode;
    uint8 key_flags;
    void *text;
};
struct onstrig_struct {
    uint32 id;
    int64 pass;
    uint8 active;
    uint8 state;
};
namespace legacy_ev {
static int32 last_error = 0;
static uint32 new_error = 0;
static void error(int32 n) { last_error = n; }
static uint32 qbevent = 0;
static int32 device_last = 1;
static int32 func__devices() { return 0; }
static uint8 stop_program = 0;
static int32 exit_ok = 0;
static double now_ms = 0;
static int64 GetTicks() { return (int64)now_ms; }
static void Sleep(int32) { stop_program = 1; }
#include "legacy/core_events.inc"
// Timer part of events() in qbx.cpp (around the generated switch).
static uint32 take(int32 i, int64 *pass) {
    std::lock_guard<std::mutex> lock(ontimer_mutex);
    if (ontimer[i].allocated && ontimer[i].id && ontimer[i].active == 1 && ontimer[i].state == 1) {
        ontimer[i].state = 2;
        *pass = ontimer[i].pass;
        return ontimer[i].id;
    }
    return 0;
}
static void done(int32 i) {
    std::lock_guard<std::mutex> lock(ontimer_mutex);
    if ((i < ontimer_nextfree) && ontimer[i].allocated) ontimer[i].state = 0;
}
static void tick(double now_seconds) {
    now_ms = now_seconds * 1000.0;
    stop_program = 0;
    TIMERTHREAD();
}
} // namespace legacy_ev

// ------------------------------------------------------------- Rust side
#include "../../crates/qb-runtime/include/qb_runtime.h"

extern uint32 new_error, error_err, error_occurred, error_goto_line, error_handling, ercl, inclercl, qbevent;
extern double error_erl, last_line;
extern char *includedfilename;
extern uint8 close_program;
extern uint8 *mem_static, *mem_static_pointer, *mem_static_limit;
extern uint32 mem_static_size;
extern onkey_struct *onkey;
extern onstrig_struct *onstrig;

extern "C" {
uint8 cmem[1114099];
uint8 *cmem_static_pointer = &cmem[0] + 1280 + 65536;
uint8 *cmem_dynamic_base = &cmem[0] + 655360;
uint32 cmem_sp = 65536;
ptrszint dblock;
void qb_host_error(int32 n) { qb_error(n); }
void qb_host_field_free(void *) {}
static std::string r_box_msg, r_box_title;
static int r_box_type = -1, r_box_answer = 0, r_ended = 0;
int32 qb_host_message_box(const char *m, const char *t, int32 type) {
    r_box_msg = m;
    r_box_title = t;
    r_box_type = type;
    return r_box_answer;
}
void qb_host_end() { r_ended = 1; }
void qb_host_evnt() {}
void qb_host_devices_init() {}
void qb_test_cmem_dynamic_reset();
int32 qb_test_critical_error(int32 code, const char **msg, const char **title);
int32 qb_test_error_dialog(uint32 code, uint32 ercl, uint32 inclercl, const char *file, const uint8 *command0,
                           int32 len, uint8 *title, uint8 *body, int32 cap);
void qb_test_timer_reset();
int32 qb_test_timer_tick(double now);
int32 qb_test_timer_get(int32 i, int64 *out4, int64 *pass, double *secs2);
int32 qb_test_wait(int32 kind, double a, double b, const int64 *times, int32 n, int64 break_after, int64 stop_after,
                   double *prev, int64 *trace, int32 cap, int32 *err, int32 *ticks_used);
}

static bool same_ld(long double a, long double b) { return memcmp(&a, &b, 10) == 0; }

// ------------------------------------------------------------- numbers + READ
static const char *TOKENS[] = {"0", "1", "7", "9", "5", "12", "255", "32767", "32768", "65535", "2147483647",
                               "2147483648", "9223372036854775807", "9223372036854775808", "18446744073709551615",
                               "18446744073709551616", ".", "-", "+", "E", "D", "e", "d", "E+", "E-", "&H", "&h",
                               "&O", "&B", "FFFF", "7FFF", "8000", "ABC", "abc", "G", "!", "#", "%", "&", "$", "`",
                               "~", "\"", ",", " ", "x", "00000", "17976931348623157", "308", "309", "324", "325",
                               "1.5", "2.5", "0.5", "-0.4", "999999999999999999999", "3.402823466E+38",
                               "1.7976931348623158E308", "0.001", "&HFFFFFFFFFFFFFFFF", "&O1777777777777777777777",
                               "&B1111111111111111111111111111111111111111111111111111111111111111", "1E-400",
                               "4.9E-324", "2.2250738585072014E-308"};
static const int NTOK = sizeof(TOKENS) / sizeof(TOKENS[0]);

static std::string random_data() {
    std::string s;
    int n = 1 + (int)rnd(14);
    for (int i = 0; i < n; ++i) {
        if (rnd(4) == 0) {
            s += (char)(rnd(3) == 0 ? rnd(256) : "0123456789.,-+EeDd&HhOoBb!#%$`~\" "[rnd(35)]);
        } else {
            s += TOKENS[rnd(NTOK)];
        }
    }
    if (rnd(2)) {
        std::string digits(1 + rnd(300), '0');
        for (auto &c : digits) c = (char)('0' + rnd(10));
        s += digits;
    }
    return s;
}

static int32 random_typ() {
    static const int32 bits[] = {1, 2, 3, 7, 8, 15, 16, 31, 32, 63, 64};
    switch (rnd(6)) {
    case 0: return 32 | ISFLOAT;
    case 1: return 64 | ISFLOAT;
    case 2: return 256 | ISFLOAT;
    default: {
        int32 t = bits[rnd(11)];
        if (t == 64) t = 32;
        if (rnd(2)) t |= ISUNSIGNED;
        return t;
    }
    }
}

static bool round_ub(const std::string &s, ptrszint off) {
    // Legacy n_roundincrement reads n_digit[n_exp+1] with a negative index for
    // numbers below 0.1 (undefined); those reads are skipped (D-R12b-1).
    ptrszint o = off;
    if (legacy_num::n_inputnumberfromdata((uint8 *)s.data(), &o, (ptrszint)s.size()) != 0) return false;
    return legacy_num::n_digits && legacy_num::n_digits > legacy_num::n_exp + 1 && legacy_num::n_exp + 1 < 0;
}

static long skipped_ub = 0;
static long sscanf_ulp = 0;
static bool one_ulp(double a, double b) {
    uint64 x, y;
    memcpy(&x, &a, 8);
    memcpy(&y, &b, 8);
    return x != y && (x - y == 1 || y - x == 1);
}

static void test_numbers(long cases) {
    for (long c = 0; c < cases; ++c) {
        std::string s = random_data();
        ptrszint lo = 0, ro = 0;
        uint8 *d = (uint8 *)s.data();
        ptrszint size = (ptrszint)s.size();
        for (int r = 0; r < 10; ++r) {
            int kind = (int)rnd(5);
            legacy_num::last_error = 0;
            if ((kind == 1 || kind == 2) && round_ub(s, lo)) {
                ++skipped_ub;
                break;
            }
            if (kind == 0) {
                int32 typ = random_typ();
                long double a = legacy_num::func_read_float(d, &lo, size, typ);
                if (g_mutate && legacy_num::last_error == 6) legacy_num::last_error = 5;
                qb_num_value v{};
                int32 e = qb_read_float(d, size, &ro, typ, &v);
                long double b = e ? 0 : (v.is_int ? (long double)v.i : (long double)v.f);
                if (!e && e == legacy_num::last_error && lo == ro && !v.is_int && one_ulp((double)a, v.f)) {
                    ++sscanf_ulp; // D-R12b-2: MinGW sscanf("%lf") is off by one ulp, Rust rounds correctly
                    b = a;
                }
                CHECK(e == legacy_num::last_error && lo == ro && same_ld(a, b),
                      "READ float typ=%d data=[%s] legacy=%Lg err=%d off=%d rust=%Lg err=%d off=%d", typ, s.c_str(), a,
                      legacy_num::last_error, (int)lo, b, e, (int)ro);
            } else if (kind == 1) {
                int64 a = legacy_num::func_read_int64(d, &lo, size), b = 0;
                int32 e = qb_read_int64(d, size, &ro, &b);
                if (e) b = 0;
                CHECK(e == legacy_num::last_error && lo == ro && a == b, "READ int64 data=[%s] %lld/%lld", s.c_str(),
                      (long long)a, (long long)b);
            } else if (kind == 2) {
                uint64 a = legacy_num::func_read_uint64(d, &lo, size), b = 0;
                int32 e = qb_read_uint64(d, size, &ro, &b);
                if (e) b = 0;
                CHECK(e == legacy_num::last_error && lo == ro && a == b, "READ uint64 data=[%s] %llu/%llu", s.c_str(),
                      (unsigned long long)a, (unsigned long long)b);
            } else if (kind == 3) {
                legacy_num::qbs *ls = legacy_num::qbs_new(0, 0);
                legacy_num::sub_read_string(d, &lo, size, ls);
                const uint8 *out;
                int32 len;
                int32 e = qb_read_string(d, size, &ro, &out, &len);
                CHECK(e == legacy_num::last_error && lo == ro && len == ls->len && memcmp(out, ls->chr, len) == 0,
                      "READ string data=[%s] err %d/%d off %d/%d", s.c_str(), legacy_num::last_error, e, (int)lo,
                      (int)ro);
            } else {
                // INPUT # conversions on the repr of the next item (parsed by legacy).
                ptrszint o = lo;
                if (legacy_num::n_inputnumberfromdata(d, &o, size) != 0) continue;
                qb_num_repr n{};
                n.digits = legacy_num::n_digits;
                memcpy(n.digit, legacy_num::n_digit, sizeof n.digit);
                n.exp = legacy_num::n_exp;
                n.neg = legacy_num::n_neg;
                n.hex = legacy_num::n_hex;
                int32 typ = random_typ();
                legacy_num::fake_input_result = 0;
                long double a = legacy_num::func_file_input_float(0, typ);
                qb_num_value v{};
                int32 e = qb_num_to_float(&n, typ, &v);
                long double b = e ? 0 : (v.is_int ? (long double)v.i : (long double)v.f);
                CHECK(e == legacy_num::last_error && same_ld(a, b), "INPUT# float typ=%d data=[%s] %Lg/%Lg", typ,
                      s.c_str(), a, b);
                bool ub = legacy_num::n_digits && legacy_num::n_digits > legacy_num::n_exp + 1 &&
                          legacy_num::n_exp + 1 < 0;
                if (!ub) {
                    int32 ok = legacy_num::n_int64();
                    int64 i64 = 0;
                    int32 ok2 = qb_num_to_int64(&n, &i64);
                    CHECK(ok == ok2 && (!ok || i64 == legacy_num::n_int64_value), "n_int64 data=[%s]", s.c_str());
                    ok = legacy_num::n_uint64();
                    uint64 u64 = 0;
                    ok2 = qb_num_to_uint64(&n, &u64);
                    CHECK(ok == ok2 && (!ok || u64 == legacy_num::n_uint64_value), "n_uint64 data=[%s]", s.c_str());
                } else {
                    ++skipped_ub;
                }
            }
            if (lo >= size) break;
        }
    }
}

// ------------------------------------------------------------- errors
static void test_errors(long cases) {
    for (int32 code = -20; code <= 1100; ++code) {
        const char *a = legacy_err::human_error(code);
        const char *b = qb_human_error(code);
        CHECK(strcmp(a, b) == 0 && !(g_mutate && code == 61), "human_error(%d) %s/%s", code, a, b);
        // critical errors
        legacy_err::box_type = -1;
        legacy_err::new_error = 0;
        int lexit = -1;
        try {
            legacy_err::error(code);
        } catch (legacy_err::LegacyExit &e) {
            lexit = e.code;
        }
        const char *m = nullptr, *t = nullptr;
        int crit = qb_test_critical_error(code, &m, &t);
        CHECK((lexit == 0) == (crit == 1), "critical %d", code);
        if (crit && lexit == 0)
            CHECK(legacy_err::box_msg == m && legacy_err::box_title == t &&
                      legacy_err::box_type == (MB_OK | MB_SYSTEMMODAL),
                  "critical box %d", code);
    }
    for (long c = 0; c < cases; ++c) {
        // error() latching
        uint32 start = rnd(3) ? 0 : (uint32)rnd(100);
        int32 code = (int32)rnd(80) - 10;
        if (code == 11) code = 12;
        legacy_err::new_error = start;
        legacy_err::qbevent = 0;
        new_error = start;
        qbevent = 0;
        legacy_err::error(code);
        qb_error(code);
        CHECK(legacy_err::new_error == new_error && legacy_err::qbevent == qbevent, "error(%d) from %u", code, start);
        // fix_error
        static const char *cmds[] = {"C:\\dir\\prog.exe", "prog", "/usr/bin/x y", "a/b\\c.exe", "", "trail\\"};
        static const char *files[] = {"inc.bi", "", "dir\\lib.bm", nullptr};
        uint32 err = (uint32)(1 + rnd(330));
        bool crit = err >= 300 && err <= 315;
        legacy_err::command0 = cmds[rnd(6)];
        if (rnd(4) == 0) legacy_err::command0 += std::string("\0zz", 3);
        const char *file = files[rnd(4)];
        uint32 el = (uint32)rnd(5000), il = rnd(2) ? 0 : (uint32)rnd(300);
        uint32 gl = rnd(2) ? (uint32)rnd(5) : 0, hd = rnd(4) == 0;
        double ll = (double)rnd(1000);
        int answer = (int)"\x00\x01\x06\x07"[rnd(4)];
        if (crit) {
            legacy_err::new_error = err;
            legacy_err::ercl = el;
            legacy_err::inclercl = il;
            legacy_err::includedfilename = (char *)file;
            legacy_err::error_goto_line = gl;
            legacy_err::error_handling = hd;
            int lexit = -1;
            try {
                legacy_err::fix_error();
            } catch (legacy_err::LegacyExit &e) {
                lexit = e.code;
            }
            uint8 title[512], body[512];
            int rc = qb_test_error_dialog(err, el, il, file, (const uint8 *)legacy_err::command0.data(),
                                          (int32)legacy_err::command0.size(), title, body, 512);
            CHECK(lexit == 0 && rc == 1 && legacy_err::box_title == (char *)title && legacy_err::box_msg == (char *)body &&
                      legacy_err::box_type == MB_OK,
                  "fix_error critical %u: [%s|%s] vs [%s|%s]", err, legacy_err::box_title.c_str(),
                  legacy_err::box_msg.c_str(), title, body);
            continue;
        }
        legacy_err::new_error = err;
        legacy_err::error_err = 0;
        legacy_err::error_erl = 0;
        legacy_err::error_occurred = 0;
        legacy_err::error_goto_line = gl;
        legacy_err::error_handling = hd;
        legacy_err::last_line = ll;
        legacy_err::ercl = el;
        legacy_err::inclercl = il;
        legacy_err::includedfilename = (char *)file;
        legacy_err::close_program = 0;
        legacy_err::ended = legacy_err::qbmain = 0;
        legacy_err::box_type = -1;
        legacy_err::box_answer = answer;
        legacy_err::fix_error();
        new_error = err;
        error_err = 0;
        error_erl = 0;
        error_occurred = 0;
        error_goto_line = gl;
        error_handling = hd;
        last_line = ll;
        ercl = el;
        inclercl = il;
        includedfilename = (char *)file;
        close_program = 0;
        r_ended = 0;
        r_box_type = -1;
        r_box_answer = answer;
        int32 enter = qb_fix_error((const uint8 *)legacy_err::command0.data(), (int32)legacy_err::command0.size());
        CHECK(enter == legacy_err::qbmain && new_error == legacy_err::new_error && error_err == legacy_err::error_err &&
                  error_erl == legacy_err::error_erl && error_occurred == legacy_err::error_occurred &&
                  close_program == legacy_err::close_program && r_ended == legacy_err::ended &&
                  r_box_type == legacy_err::box_type &&
                  (r_box_type < 0 || (r_box_msg == legacy_err::box_msg && r_box_title == legacy_err::box_title)),
              "fix_error %u goto=%u handling=%u: legacy [%s|%s] qbmain=%d rust [%s|%s] %d", err, gl, hd,
              legacy_err::box_title.c_str(), legacy_err::box_msg.c_str(), legacy_err::qbmain, r_box_title.c_str(),
              r_box_msg.c_str(), enter);
    }
    new_error = 0;
}

// ------------------------------------------------------------- allocators
static void test_alloc(long ops) {
    // mem_static
    legacy_alloc::init_mem_static(4096);
    qb_mem_static_new_block(4096);
    std::vector<std::pair<uint8 *, uint8 *>> marks;
    for (long i = 0; i < ops; ++i) {
        // Old blocks are never freed (by design), so start a fresh 4 KB block once
        // the current one passes 512 KB to keep the harness's memory bounded.
        if (mem_static_size > (1u << 19)) {
            legacy_alloc::init_mem_static(4096);
            qb_mem_static_new_block(4096);
            marks.clear();
        }
        if (rnd(10) == 0 && !marks.empty()) {
            auto m = marks[rnd((int64)marks.size())];
            legacy_alloc::mem_static_restore(m.first);
            qb_mem_static_restore(m.second);
        } else {
            uint32 size = rnd(20) == 0 ? (uint32)rnd(40000) : (uint32)rnd(600);
            uint8 *a = legacy_alloc::mem_static_malloc(size);
            int32 e = 0;
            uint8 *b = qb_mem_static_malloc(size, &e);
            if (g_mutate && size == 9) a += 8;
            CHECK(e == 0 && a - legacy_alloc::mem_static == b - mem_static &&
                      legacy_alloc::mem_static_size == mem_static_size &&
                      legacy_alloc::mem_static_pointer - legacy_alloc::mem_static == mem_static_pointer - mem_static &&
                      legacy_alloc::mem_static_limit - legacy_alloc::mem_static == mem_static_limit - mem_static,
                  "mem_static_malloc(%u) op %ld", size, i);
            if (marks.size() < 64) marks.push_back({a, b});
            else marks[rnd(64)] = {a, b};
        }
    }
    // cmem_dynamic: several independent runs, each until the first (critical) error
    for (int run = 0; run < 40; ++run) {
        legacy_alloc::cmem_dynamic_link_first = NULL;
        legacy_alloc::cmem_dynamic_next_link = 0;
        legacy_alloc::cmem_dynamic_free_link = 0;
        legacy_alloc::cmem_dynamic_base = &legacy_alloc::cmem[0] + 655360;
        legacy_alloc::last_error = 0;
        qb_test_cmem_dynamic_reset();
        cmem_dynamic_base = &cmem[0] + 655360;
        uint32 st = 1280 + 65536 + (uint32)rnd(500000);
        legacy_alloc::cmem_static_pointer = &legacy_alloc::cmem[0] + st;
        cmem_static_pointer = &cmem[0] + st;
        std::vector<ptrszint> live;
        for (long i = 0; i < ops / 40; ++i) {
            if (rnd(3) == 0 && !live.empty()) {
                size_t k = rnd((int64)live.size());
                ptrszint o = live[k];
                live.erase(live.begin() + k);
                legacy_alloc::cmem_dynamic_free(&legacy_alloc::cmem[0] + o);
                qb_cmem_dynamic_free(&cmem[0] + o);
                continue;
            }
            if (rnd(30) == 0) {
                ptrszint o = (ptrszint)rnd(655361);
                legacy_alloc::cmem_dynamic_free(&legacy_alloc::cmem[0] + o);
                qb_cmem_dynamic_free(&cmem[0] + o);
                continue;
            }
            uint32 size = rnd(50) == 0 ? (uint32)rnd(70000) : (uint32)rnd(3000);
            uint8 *a = legacy_alloc::cmem_dynamic_malloc(size);
            int32 e = 0;
            uint8 *b = qb_cmem_dynamic_malloc(size, &e);
            if (legacy_alloc::last_error || e) {
                CHECK(legacy_alloc::last_error == e, "cmem_dynamic_malloc(%u) error %d/%d", size,
                      legacy_alloc::last_error, e);
                break;
            }
            ptrszint oa = a - legacy_alloc::cmem, ob = b - cmem;
            if (g_mutate && size == 33) oa += 16;
            CHECK(oa == ob && legacy_alloc::cmem_dynamic_base - legacy_alloc::cmem == cmem_dynamic_base - cmem,
                  "cmem_dynamic_malloc(%u) %lld/%lld", size, (long long)oa, (long long)ob);
            if (size) live.push_back(oa);
        }
    }
}

// ------------------------------------------------------------- arrays, math, radix
static double random_double() {
    switch (rnd(5)) {
    case 0: {
        uint64 b = next();
        double d;
        memcpy(&d, &b, 8);
        return d;
    }
    case 1: return (double)(rnd(2000) - 1000) / 7.0;
    case 2: return (double)(rnd(200) - 20);
    case 3: return ldexp((double)(rnd(1000000) + 1), (int)rnd(80) - 40);
    default: return 88.02969 + ((double)rnd(100) - 50) * 1e-6;
    }
}

static void test_arrays_math(long cases) {
    for (long c = 0; c < cases; ++c) {
        // LBOUND/UBOUND
        ptrszint desc[64];
        for (auto &x : desc) x = (ptrszint)next();
        int32 nidx = 1 + (int32)rnd(8), idx = (int32)rnd(12) - 2;
        if (rnd(4)) desc[2] |= 1;
        for (int u = 0; u < 2; ++u) {
            legacy_arr::last_error = 0;
            ptrszint a = u ? legacy_arr::func_ubound(desc, idx, nidx) : legacy_arr::func_lbound(desc, idx, nidx);
            ptrszint b = 0;
            int32 e = qb_array_bound(desc, idx, nidx, u, &b);
            if (e) b = 0;
            if (g_mutate && u && idx == 1) a++;
            CHECK(a == b && e == legacy_arr::last_error, "bound u=%d idx=%d n=%d", u, idx, nidx);
        }
        // bit arrays
        uint8 la[80], ra[80];
        for (int k = 0; k < 80; ++k) la[k] = ra[k] = (uint8)next();
        for (int k = 0; k < 8; ++k) {
            uint32 bs = 1 + (uint32)rnd(63);
            ptrszint i = (ptrszint)rnd((64 * 8 - 64) / bs);
            int64 v = (int64)next();
            if (rnd(2)) {
                legacy_arr::setbits(bs, la, i, v);
                qb_setbits(bs, ra, i, v);
                CHECK(memcmp(la, ra, 80) == 0, "setbits %u %d", bs, (int)i);
            }
            int64 g1 = legacy_arr::getbits(bs, la, i), g2 = qb_getbits(bs, ra, i);
            uint64 u1 = legacy_arr::getubits(bs, la, i), u2 = qb_getubits(bs, ra, i);
            CHECK(g1 == g2 && u1 == u2, "getbits %u %d", bs, (int)i);
        }
        // math
        double x = random_double(), r;
        legacy_arr::last_error = 0;
        double a = legacy_arr::func_log(x);
        int32 e = qb_log(x, &r);
        CHECK(e == legacy_arr::last_error && memcmp(&a, &r, 8) == 0, "log(%.17g)", x);
        legacy_arr::last_error = 0;
        a = legacy_arr::func_exp_single(x);
        e = qb_exp_single(x, &r);
        CHECK(e == legacy_arr::last_error && memcmp(&a, &r, 8) == 0, "exp(%.17g)", x);
        legacy_arr::last_error = 0;
        a = legacy_arr::func_sqr(x);
        e = qb_sqr(x, &r);
        CHECK(e == legacy_arr::last_error && memcmp(&a, &r, 8) == 0, "sqr(%.17g)", x);
        a = legacy_arr::func_fix_double(x);
        r = qb_fix_double(x);
        CHECK(memcmp(&a, &r, 8) == 0, "fix(%.17g) %016llx/%016llx from %016llx", x, *(unsigned long long *)&a, *(unsigned long long *)&r, *(unsigned long long *)&x);
        // HEX$/OCT$/_BIN$
        int64 v;
        switch (rnd(4)) {
        case 0: v = (int64)next(); break;
        case 1: v = (int64)rnd(70000) - 35000; break;
        case 2: v = -(int64)rnd(1ll << (rnd(62) + 1)); break;
        default: v = (int64)rnd(1ll << rnd(63));
        }
        static const int32 widths[] = {4, 8, 16, 32, 64, 1, 3};
        int32 w = widths[rnd(7)];
        uint8 buf[64];
        int32 hw = w > 16 ? 16 : w; // HEX$ callers pass 4, 8 or 16 (wider reads before the legacy buffer)
        legacy_arr::qbs *h = legacy_arr::func_hex(v, hw);
        int32 n = qb_radix_string(v, hw, 16, buf);
        CHECK(n == h->len && memcmp(buf, h->chr, n) == 0 && !(g_mutate && v == -1 && w == 4), "hex %lld %d", (long long)v, hw);
        h = legacy_arr::func_oct(v, w);
        n = qb_radix_string(v, w, 8, buf);
        CHECK(n == h->len && memcmp(buf, h->chr, n) == 0, "oct %lld %d", (long long)v, w);
        h = legacy_arr::func__bin(v, w);
        n = qb_radix_string(v, w, 2, buf);
        CHECK(n == h->len && memcmp(buf, h->chr, n) == 0, "bin %lld %d", (long long)v, w);
    }
}

// ------------------------------------------------------------- waits
static void test_waits(long cases) {
    double rust_limit_prev = 0;
    for (long c = 0; c < cases; ++c) {
        int kind = (int)rnd(3);
        std::vector<int64> times;
        int64 t = (int64)rnd(1000000);
        int n = 1 + (int)rnd(60);
        for (int i = 0; i < n; ++i) {
            times.push_back(t);
            if (rnd(25) == 0) t -= rnd(500);
            else t += rnd(rnd(2) ? 4 : 16);
        }
        times.push_back(t + 3000000000ll); // every wait ends (_DELAY allows up to 2147483.647 s)
        double a = 0, b = 0;
        int64 brk = -1, stop = -1;
        if (kind == 0) {
            static const double s[] = {-1, 0, 0.001, 0.009, 0.01, 0.0105, 0.05, 2147483.647, 2147483.648};
            a = rnd(2) ? s[rnd(9)] : (double)rnd(200) / 1000.0;
        } else if (kind == 1) {
            static const double f[] = {0, -5, 0.0166, 0.0167, 1, 30, 50, 60, 100, 1000, 3.3};
            a = f[rnd(11)];
        } else {
            a = (double)times[0] + (double)(rnd(40) - 20);
            b = rnd(4) == 0 ? (double)(-rnd(2)) : (double)(rnd(150) * 1000) / 1000.0;
            if (b <= 0 || rnd(3) == 0) brk = 1 + rnd(6);
            if (rnd(6) == 0) stop = 1 + rnd(6);
        }
        legacy_wait::times = times;
        legacy_wait::at = 0;
        legacy_wait::trace.clear();
        legacy_wait::evnts = 0;
        legacy_wait::break_after = brk;
        legacy_wait::stop_after = stop;
        legacy_wait::sleep_break = 0;
        legacy_wait::stop_program = 0;
        legacy_wait::last_error = 0;
        if (kind == 0) legacy_wait::sub__delay(a);
        else if (kind == 1) legacy_wait::sub__limit(a);
        else legacy_wait::sub_sleep_tail(a, b);
        int64 trace[4096];
        int32 err = 0, used = 0;
        int32 len = qb_test_wait(kind, a, b, times.data(), (int32)times.size(), brk, stop, &rust_limit_prev, trace,
                                 4096, &err, &used);
        if (g_mutate && kind == 0 && !legacy_wait::trace.empty()) legacy_wait::trace.back() ^= 1;
        bool same = len == (int32)legacy_wait::trace.size() && err == legacy_wait::last_error &&
                    used == (int32)legacy_wait::at;
        for (int32 i = 0; same && i < len; ++i) same = trace[i] == legacy_wait::trace[i];
        CHECK(same, "wait kind=%d a=%.6g b=%.6g: legacy %d entries err %d ticks %d, rust %d entries err %d ticks %d",
              kind, a, b, (int)legacy_wait::trace.size(), legacy_wait::last_error, (int)legacy_wait::at, len, err,
              used);
    }
}

// ------------------------------------------------------------- events
static void compare_timers(long op, const char *what) {
    int32 n = legacy_ev::ontimer_nextfree;
    CHECK(n == qb_timer_count(), "%s op %ld: timer count %d/%d", what, op, n, qb_timer_count());
    for (int32 i = 0; i < n && i < qb_timer_count(); ++i) {
        int64 o[4], pass;
        double s[2];
        qb_test_timer_get(i, o, &pass, s);
        auto &l = legacy_ev::ontimer[i];
        bool same = o[0] == (l.allocated != 0) && o[1] == l.id && o[2] == l.active && o[3] == l.state;
        if (l.allocated && l.id) same = same && pass == l.pass && s[0] == l.seconds && s[1] == l.last_time;
        CHECK(same, "%s op %ld: timer %d legacy a%d id%u act%d st%d last%.3f, rust a%d id%d act%d st%d last%.3f", what,
              op, i, l.allocated, l.id, l.active, l.state, l.last_time, (int)o[0], (int)o[1], (int)o[2], (int)o[3],
              s[1]);
    }
}

static void test_events(long ops) {
    legacy_ev::ontimer[0].allocated = 1;
    qb_test_timer_reset();
    double now = 1000;
    for (long op = 0; op < ops; ++op) {
        legacy_ev::last_error = 0;
        legacy_ev::qbevent = 0;
        new_error = 0;
        qbevent = 0;
        int32 cnt = legacy_ev::ontimer_nextfree;
        int32 i = (int32)rnd(cnt + 2) - (rnd(10) == 0 ? 1 : 0);
        const char *what = "";
        switch (rnd(8)) {
        case 0: {
            what = "freetimer";
            int32 a = legacy_ev::func__freetimer(), b = qb_freetimer_new();
            CHECK(a == b, "freetimer %d/%d", a, b);
            break;
        }
        case 1: {
            what = "setup";
            double sec = rnd(5) == 0 ? 0 : (double)rnd(3000) / 1000.0;
            uint32 id = (uint32)rnd(4);
            int64 pass = (int64)rnd(100);
            legacy_ev::ontimer_setup(i, sec, id, pass);
            qb_ontimer_setup(i, sec, id, pass);
            break;
        }
        case 2:
        case 3: {
            what = "timer";
            int32 option = 1 + (int32)rnd(4), passed = rnd(4) != 0;
            if (g_mutate && option == 3) option = rnd(2) ? 3 : 1;
            legacy_ev::sub_timer(i, option, passed);
            if (g_mutate && option == 1 && rnd(2)) option = 3;
            qb_timer_statement(i, option, passed);
            break;
        }
        case 4:
        case 5: {
            what = "tick";
            now += (double)rnd(1500) / 1000.0;
            legacy_ev::tick(now);
            int32 f = qb_test_timer_tick((double)(int64)(now * 1000.0) * 0.001); // GetTicks() is whole ms
            CHECK((legacy_ev::qbevent != 0) == (f != 0), "tick %.3f fired %u/%d", now, legacy_ev::qbevent, f);
            legacy_ev::qbevent = 0;
            break;
        }
        default: {
            what = "dispatch";
            for (int32 k = 0; k < legacy_ev::ontimer_nextfree; ++k) {
                int64 p1 = 0, p2 = 0;
                uint32 a = legacy_ev::take(k, &p1), b = qb_timer_take(k, &p2);
                CHECK(a == b && (!a || p1 == p2), "take %d: %u/%u", k, a, b);
                if (a && rnd(3)) {
                    legacy_ev::done(k);
                    qb_timer_done(k);
                }
            }
        }
        }
        CHECK(legacy_ev::last_error == (int32)new_error && legacy_ev::qbevent == (new_error ? 0 : qbevent),
              "%s op %ld: error %d/%u", what, op, legacy_ev::last_error, new_error);
        compare_timers(op, what);
    }
    // ON KEY / ON STRIG
    for (long op = 0; op < ops; ++op) {
        legacy_ev::last_error = 0;
        legacy_ev::qbevent = 0;
        new_error = 0;
        qbevent = 0;
        switch (rnd(6)) {
        case 0: {
            int32 i = (int32)rnd(34) - 1;
            uint32 id = (uint32)rnd(5);
            int64 pass = (int64)rnd(50);
            legacy_ev::onkey_setup(i, id, pass);
            qb_onkey_setup(i, id, pass);
            break;
        }
        case 1: {
            int32 i = (int32)rnd(34) - 1, option = 1 + (int32)rnd(3);
            legacy_ev::sub_key(i, option);
            qb_key_statement(i, option);
            break;
        }
        case 2: {
            int32 i = rnd(3) ? (int32)rnd(20) : (int32)rnd(70000) - 10, ctl = (int32)rnd(300) - 2, cp = rnd(2);
            uint32 id = (uint32)rnd(5);
            legacy_ev::onstrig_setup(i, ctl, cp, id, 7);
            qb_onstrig_setup(i, ctl, cp, id, 7);
            break;
        }
        case 3: {
            int32 i = rnd(3) ? (int32)rnd(20) : (int32)rnd(70000) - 10, ctl = (int32)rnd(300) - 2;
            int32 option = 1 + (int32)rnd(3), passed = (int32)rnd(4);
            legacy_ev::sub_strig(i, ctl, option, passed);
            qb_strig_statement(i, ctl, option, passed);
            break;
        }
        default: { // the keyboard/gamepad code queues events
            int32 k = (int32)rnd(32);
            uint8 st = (uint8)rnd(4);
            legacy_ev::onkey[k].state = st;
            onkey[k].state = st;
            int32 s = (int32)rnd(1024);
            legacy_ev::onstrig[s].state = st;
            onstrig[s].state = st;
        }
        }
        CHECK(legacy_ev::last_error == (int32)new_error && legacy_ev::qbevent == (new_error ? 0 : qbevent),
              "key/strig op %ld: error %d/%u qbevent %u/%u", op, legacy_ev::last_error, new_error, legacy_ev::qbevent,
              qbevent);
        bool same = true;
        for (int k = 0; k < 32; ++k) {
            auto &a = legacy_ev::onkey[k];
            auto &b = onkey[k];
            same = same && a.id == b.id && a.pass == b.pass && a.active == b.active && a.state == b.state;
        }
        int lim = op % 64 == 0 ? 65536 : 1024;
        for (int k = 0; k < lim; ++k) {
            auto &a = legacy_ev::onstrig[k];
            auto &b = onstrig[k];
            same = same && a.id == b.id && a.pass == b.pass && a.active == b.active && a.state == b.state;
        }
        CHECK(same, "key/strig op %ld: arrays differ", op);
    }
    new_error = 0;
    qbevent = 0;
}

int main(int argc, char **argv) {
    bool quick = false;
    const char *only = nullptr;
    for (int i = 1; i < argc; ++i) {
        if (!strcmp(argv[i], "--only") && i + 1 < argc) only = argv[++i];
        if (!strcmp(argv[i], "--quick")) quick = true;
        if (!strcmp(argv[i], "--mutate")) g_mutate = true;
    }
    setvbuf(stdout, NULL, _IONBF, 0);
    legacy_num::fpu_reinit();
    long k = quick ? 1 : 10;
    struct {
        const char *name;
        void (*fn)(long);
        long n;
    } parts[] = {{"numbers", test_numbers, 60000 * k}, {"errors", test_errors, 20000 * k},
                 {"alloc", test_alloc, 40000 * k},     {"arrays_math", test_arrays_math, 60000 * k},
                 {"waits", test_waits, 40000 * k},     {"events", test_events, 8000 * k}};
    for (auto &p : parts) {
        if (only && strcmp(only, p.name)) continue;
        long before = g_fail, checks = g_checks;
        p.fn(p.n);
        printf("diff_core %-12s %9ld checks, %ld mismatches\n", p.name, g_checks - checks, g_fail - before);
    }
    printf("diff_core: %s%s, %ld checks, %ld mismatches (%ld reads skipped: undefined legacy rounding read; %ld decimal READs where MinGW sscanf is one ulp off, D-R12b-2)\n",
           quick ? "quick" : "full", g_mutate ? " MUTATED" : "", g_checks, g_fail, skipped_ub, sscanf_ulp);
    return g_fail == 0 ? 0 : 1;
}
