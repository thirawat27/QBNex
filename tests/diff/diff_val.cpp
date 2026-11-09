// Differential test: legacy func_val (tests/legacy/func_val.inc) vs Rust qb_runtime::val.
// Built with MinGW on Windows (the legacy code calls __mingw_sscanf) or glibc elsewhere.
//
//   g++ -O1 -std=c++17 -w tests/diff/diff_val.cpp <libqb_runtime.a> <native libs> -o diff_val
//   ./diff_val [--quick]
//
// Inputs with a negative exponent are only generated in the form "d.ddd E-x" (one integer
// digit), where legacy is correct: for other forms legacy has defect L-05, which the Rust
// port fixes on purpose (covered by unit tests in val.rs instead).

#include <cstdint>
#include <cstdio>
#include <cstring>
#include <string>

typedef int8_t int8;
typedef uint8_t uint8;
typedef int32_t int32;
typedef int64_t int64;

namespace legacy {
struct qbs {
    uint8 *chr;
    int32 len;
};
static int32 last_error = 0;
static void error(int32 n) { last_error = n; }
#if defined(__MINGW32__)
#define QBNex_MINGW
#define QBNex_WINDOWS
#endif
#include "legacy/func_val.inc"
} // namespace legacy

#include "../../crates/qb-runtime/include/qb_runtime.h"

static uint64_t rng = 0x9E3779B97F4A7C15ull;
static uint64_t next() {
    rng ^= rng << 13;
    rng ^= rng >> 7;
    rng ^= rng << 17;
    return rng;
}
static int pick(int n) { return (int)(next() % (uint64_t)n); }

static std::string digits(int n, bool leading_nonzero) {
    std::string s;
    for (int i = 0; i < n; ++i) s += (char)('0' + ((i == 0 && leading_nonzero) ? 1 + pick(9) : pick(10)));
    return s;
}

static std::string make_input() {
    std::string s;
    if (pick(8) == 0) s += "  ";
    switch (pick(10)) {
    case 0: { // hex / octal / binary
        const char *pre[] = {"&H", "&h", "&O", "&o", "&B", "&b", "&X"};
        s += pre[pick(7)];
        const char *alpha = "0123456789abcdefABCDEFG";
        int n = pick(26);
        for (int i = 0; i < n; ++i) s += alpha[pick(23)];
        return s;
    }
    case 1: { // random junk from the scanner's alphabet
        const char *alpha = "0123456789.+-eEdD &xa\t";
        int n = pick(14);
        for (int i = 0; i < n; ++i) s += alpha[pick(22)];
        return s;
    }
    case 2: { // integers, sometimes long
        if (pick(2)) s += pick(2) ? "-" : "+";
        s += digits(1 + pick(pick(4) == 0 ? 40 : 19), pick(3) != 0);
        return s;
    }
    case 3: { // one integer digit, fraction, negative exponent (legacy-correct form)
        if (pick(2)) s += "-";
        s += digits(1, true);
        s += ".";
        s += digits(pick(25), false);
        s += pick(2) ? "E-" : "d-";
        s += std::to_string(pick(pick(5) == 0 ? 5000 : 330));
        return s;
    }
    default: { // general decimals with positive or no exponent
        if (pick(3) == 0) s += "-";
        s += digits(pick(12), false);
        s += ".";
        s += digits(pick(pick(6) == 0 ? 60 : 22), false);
        if (pick(2)) {
            s += pick(2) ? "E" : "e";
            if (pick(2)) s += "+";
            s += std::to_string(pick(pick(5) == 0 ? 5000 : 330));
        }
        if (pick(5) == 0) s += "x9";
        return s;
    }
    }
}

// True when an exponent marker is followed later by '-': the forms affected by L-05.
static bool may_have_negative_exponent(const std::string &s) {
    size_t e = s.find_first_of("eEdD");
    return e != std::string::npos && s.find('-', e) != std::string::npos;
}

// Exactly the case-3 shape -?[1-9]\.[0-9]*[Ed]-[0-9]+ (one integer digit: legacy is correct).
static bool one_digit_negative_exponent(const std::string &s) {
    size_t i = (!s.empty() && s[0] == '-') ? 1 : 0;
    if (i + 1 >= s.size() || s[i] < '1' || s[i] > '9' || s[i + 1] != '.') return false;
    i += 2;
    while (i < s.size() && s[i] >= '0' && s[i] <= '9') ++i;
    if (i + 1 >= s.size() || (s[i] != 'E' && s[i] != 'd') || s[i + 1] != '-') return false;
    i += 2;
    if (i >= s.size()) return false;
    while (i < s.size() && s[i] >= '0' && s[i] <= '9') ++i;
    return i == s.size();
}

int main(int argc, char **argv) {
    const bool quick = argc > 1 && strcmp(argv[1], "--quick") == 0;
    const long n = quick ? 200000 : 3000000;
    long failures = 0;
    const char *fixed[] = {"", "-", "0", "-0", "1", "0.1", ".5", "1E4932", "1.18973149535723176502e4932",
                           "1.2e4932", "3.64519953188247460252840593361941982e-4951", "2e-4951",
                           "&H8000000000000000", "&HFFFFFFFFFFFFFFFF1", "&O1777777777777777777777",
                           "&O2000000000000000000000", "9223372036854775807", "9223372036854775808",
                           "18446744073709551617", "1 2 3", "1e18", "1e0", "12.5.3"};
    const long nfixed = sizeof(fixed) / sizeof(fixed[0]);
    for (long i = 0; i < n + nfixed; ++i) {
        std::string in = i < nfixed ? std::string(fixed[i]) : make_input();
        // Case 3 inputs keep their negative exponent; legacy is correct there.
        while (i >= nfixed && may_have_negative_exponent(in) && !one_digit_negative_exponent(in)) {
            in = make_input();
        }
        legacy::qbs q{(uint8 *)in.data(), (int32)in.size()};
        legacy::last_error = 0;
        long double a = legacy::func_val(&q);
        int32 ea = legacy::last_error;
        unsigned char ra[10], rb[10];
        memcpy(ra, &a, 10);
        if (ea) memset(ra, 0, 10); // runtime discards the value after error()
        int32 eb = qb_val((const unsigned char *)in.data(), (int32)in.size(), rb);
        if (ea != eb || memcmp(ra, rb, 10) != 0) {
            if (failures < 25) {
                printf("MISMATCH \"%s\" legacy=", in.c_str());
                for (int k = 9; k >= 0; --k) printf("%02X", ra[k]);
                printf(" err=%d rust=", ea);
                for (int k = 9; k >= 0; --k) printf("%02X", rb[k]);
                printf(" err=%d\n", eb);
            }
            ++failures;
        }
    }
    printf("diff_val: %s, %ld mismatches\n", quick ? "quick" : "full", failures);
    return failures == 0 ? 0 : 1;
}
