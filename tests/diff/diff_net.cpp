// Differential test: legacy connection_new / func__openclient / func__openhost /
// func__connectionaddress (tests/legacy/net_glue.inc, verbatim) vs Rust
// qb_net_parse_info and qb_net_format_address.
//
// The legacy code runs on a minimal qbs/list runtime defined here; the socket
// calls (tcp_client_open, tcp_host_open) are replaced by recorders, so the test
// compares what each side would open (host text, port) and which error it
// raises. Info strings with more than ten parts are not generated: legacy
// wrote past its 10-entry part array there (undefined behaviour).
//
//   g++ -O1 -std=c++17 -w tests/diff/diff_net.cpp <libqb_runtime.a> <native libs> -o diff_net
//   ./diff_net [--quick] [--mutate]

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>

#include "../../crates/qb-runtime/include/qb_runtime.h"

typedef int8_t int8;
typedef uint8_t uint8;
typedef int16_t int16;
typedef int32_t int32;
typedef uint32_t uint32;
typedef int64_t int64;
typedef uint64_t uint64;
typedef intptr_t ptrszint;

#define DEPENDENCY_SOCKETS 1

namespace legacy {

// --- minimal runtime the legacy code needs -----------------------------------
struct qbs {
    uint8 *chr;
    int32 len;
    uint8 tmp, fixed, readonly, in_cmem;
};
static qbs *qbs_new(int32 size, uint8 tmp) {
    qbs *s = new qbs();
    s->chr = (uint8 *)calloc(size + 1, 1);
    s->len = size;
    s->tmp = tmp;
    return s;
}
static qbs *qbs_new_txt(const char *t) {
    qbs *s = qbs_new((int32)strlen(t), 1);
    memcpy(s->chr, t, s->len);
    return s;
}
static qbs *qbs_set(qbs *d, qbs *s) {
    uint8 *c = (uint8 *)calloc(s->len + 1, 1);
    memcpy(c, s->chr, s->len);
    d->chr = c;
    d->len = s->len;
    return d;
}
static qbs *qbs_add(qbs *a, qbs *b) {
    qbs *s = qbs_new(a->len + b->len, 1);
    memcpy(s->chr, a->chr, a->len);
    memcpy(s->chr + a->len, b->chr, b->len);
    return s;
}
static qbs *qbs_ucase(qbs *str) {
    qbs *s = qbs_new(str->len, 1);
    for (int32 i = 0; i < str->len; i++) {
        uint8 c = str->chr[i];
        s->chr[i] = (c >= 'a' && c <= 'z') ? (c & 223) : c;
    }
    return s;
}
static int32 qbs_equal(qbs *a, qbs *b) {
    return (a->len == b->len && !memcmp(a->chr, b->chr, a->len)) ? -1 : 0;
}
static qbs *qbs_ltrim(qbs *str) {
    int32 i = 0;
    while (i < str->len && str->chr[i] == 32) i++;
    qbs *s = qbs_new(str->len - i, 1);
    memcpy(s->chr, str->chr + i, s->len);
    return s;
}
static qbs *qbs_str(int32 v) {
    uint8 buf[QB_STR_MAX];
    int32 n = qb_str_i64(v, buf);
    qbs *s = qbs_new(n, 1);
    memcpy(s->chr, buf, n);
    return s;
}
static qbs *qbs_str(uint8 v) {
    uint8 buf[QB_STR_MAX];
    int32 n = qb_str_u64(v, buf);
    qbs *s = qbs_new(n, 1);
    memcpy(s->chr, buf, n);
    return s;
}
// INSTR and MID$ with the legacy semantics (libqb.cpp func_instr/func_mid).
static int32 func_instr(int32 start, qbs *str, qbs *substr, int32 passed) {
    if (!passed) start = 1;
    if (!str->len) return 0;
    if (start < 1) {
        start = 1;
        if (!substr->len) return 0;
    }
    if (start > str->len) return 0;
    if (!substr->len) return start;
    if ((start + substr->len - 1) > str->len) return 0;
    for (int32 p = start; p + substr->len - 1 <= str->len; p++)
        if (!memcmp(str->chr + p - 1, substr->chr, substr->len)) return p;
    return 0;
}
static qbs *func_mid(qbs *str, int32 start, int32 l, int32 passed) {
    if (passed) {
        if (start < 1) {
            l = l - 1 + start;
            start = 1;
        }
        if ((l >= 1) && (start <= str->len)) {
            if ((start + l) > str->len) l = str->len - start + 1;
        } else {
            l = 0;
            start = 1;
        }
    } else {
        if (start < 1) start = 1;
        l = str->len - start + 1;
        if (l < 1) {
            l = 0;
            start = 1;
        }
    }
    qbs *s = qbs_new(l, 1);
    if (l) memcpy(s->chr, str->chr + start - 1, l);
    return s;
}

static int32 new_error = 0;
static int32 first_error = 0;
static void error(int32 n) {
    if (!new_error) {
        new_error = n;
        first_error = n;
    }
}
static int32 vwatch = 0;

// VAL and the double->long rounding were ported in R2 (diff_val, diff_round).
static long double func_val(qbs *s) {
    unsigned char r[10];
    int32 code = qb_val(s->chr, s->len, r);
    if (code) {
        error(code);
        return 0;
    }
    return (long double)qb_f80_to_f64(r);
}
static int32 qbr_double_to_long(double d) { return qb_fistp_i32(d); }

struct list {
    int32 size;
    std::vector<uint8 *> items;
};
static list *list_new(int32 size) {
    list *l = new list();
    l->size = size;
    l->items.push_back(nullptr);
    return l;
}
static int32 list_add(list *l) {
    l->items.push_back((uint8 *)calloc(l->size, 1));
    return (int32)l->items.size() - 1;
}
static void *list_get(list *l, ptrszint i) {
    if (i < 1 || i >= (ptrszint)l->items.size()) return nullptr;
    return l->items[i];
}

struct special_handle_struct {
    int8 type;
    ptrszint index;
};
struct stream_struct {
    int32 type;
    ptrszint index;
    uint8 *in;
    ptrszint in_size, in_limit;
};
static list *special_handles, *stream_handles, *connection_handles;

// socket recorders
static int32 opened = 0;  // 0 none, 1 client, 2 host
static std::string opened_host;
static int64 opened_port = 0;
void *tcp_client_open(uint8 *host, int64 port);
void *tcp_host_open(int64 port);
void *tcp_connection_open(void *host) { return nullptr; }

static std::string whatismyip_text;
static qbs *WHATISMYIP() { return qbs_new_txt(whatismyip_text.c_str()); }

#include "legacy/net_glue.inc"

void *tcp_client_open(uint8 *host, int64 port) {
    opened = 1;
    opened_host = (char *)host;  // legacy passed a NUL-terminated copy
    opened_port = port;
    return new tcp_connection();
}
void *tcp_host_open(int64 port) {
    opened = 2;
    opened_host.clear();
    opened_port = port;
    return new tcp_connection();
}

}  // namespace legacy

static uint64_t rng = 0x9E3779B97F4A7C15ull;
static uint64_t next() {
    rng ^= rng << 13;
    rng ^= rng >> 7;
    rng ^= rng << 17;
    return rng;
}
template <size_t N>
static const char *pick(const char *const (&a)[N]) { return a[next() % N]; }

static bool mutate = false;

static std::string random_info(int32 &method) {
    static const char *const kinds[] = {"TCP/IP", "tcp/ip", "Tcp/Ip", "UDP", "QBNEXIDE", "qbnexide", "", "TCP/IP ", "TCP"};
    static const char *const ports[] = {"80", "0", "65535", "70000", "-1", " 12.5", "13.5", "&H10", "&H11111111111111111",
                                        "1E20", "abc", "", "1D3", "&O777", "&B101", "  7319  ", "-0", "2147483648", "4.5E2"};
    static const char *const hosts[] = {"localhost", "127.0.0.1", "example.com", "", "a b", "x", "::1"};
    method = (int32)(next() & 1);
    int32 parts = 1 + (int32)(next() % 5);
    if (next() % 16 == 0) parts = 1 + (int32)(next() % 10);
    std::string s = pick(kinds);
    if (parts >= 2) s += std::string(":") + pick(ports);
    if (parts >= 3) {
        s += std::string(":") + pick(hosts);
        if (next() % 8 == 0) s += std::string(1, '\0') + "tail";  // embedded NUL
    }
    for (int32 i = 4; i <= parts; i++) s += ":" + std::to_string(next() % 100);
    if (next() % 10 == 0 && !s.empty()) s[next() % s.size()] = (char)(next() & 255);  // noise byte
    return s;
}

static long check_parse(long n) {
    using namespace legacy;
    long failures = 0;
    for (long k = 0; k < n; ++k) {
        int32 method;
        std::string info = random_info(method);
        // skip inputs with more than 10 parts (legacy undefined behaviour)
        int32 colons = 0;
        for (char c : info) colons += c == ':';
        if (colons >= 10) continue;
        int32 vw = (next() & 3) == 0 ? -1 : 0;

        // legacy
        new_error = first_error = 0;
        vwatch = vw;
        opened = 0;
        opened_host.clear();
        legacy::qbs *q = qbs_new((int32)info.size(), 1);
        memcpy(q->chr, info.data(), info.size());
        int32 lr = method == 0 ? func__openclient(q) : func__openhost(q);
        int32 l_err = first_error;

        // rust + the C++ shim logic of connection_new
        int32 port = 0, off = -1, hlen = 0, verr = 0, r_err = 0, r_opened = 0;
        std::string r_host;
        int32 r = qb_net_parse_info((const uint8_t *)info.data(), (int32)info.size(), method, vw == -1, &port, &off,
                                      &hlen, &verr);
        if (mutate && port == 80) port = 81;
        if (verr) r_err = verr;
        if (r) {
            if (!r_err) r_err = 5;
        } else {
            r_opened = method == 0 ? 1 : 2;
            if (method == 0) {
                std::string h(info.data() + off, hlen);
                r_host = h.substr(0, h.find('\0'));
            }
        }
        bool same = l_err == r_err && opened == r_opened;
        if (same && opened) same = opened_host == r_host && opened_port == port;
        if (same && !opened) same = lr == 0;
        if (!same) {
            if (failures < 20) {
                std::string shown;
                for (unsigned char c : info) shown += (c >= 32 && c < 127) ? (char)c : '?';
                printf("PARSE MISMATCH method=%d vwatch=%d info=\"%s\": legacy err=%d opened=%d host=\"%s\" port=%lld | "
                       "rust err=%d opened=%d host=\"%s\" port=%d\n",
                       method, vw, shown.c_str(), l_err, opened, opened_host.c_str(), (long long)opened_port, r_err,
                       r_opened, r_host.c_str(), port);
            }
            ++failures;
        }
    }
    return failures;
}

static long check_address(long n) {
    using namespace legacy;
    long failures = 0;
    for (long k = 0; k < n; ++k) {
        int32 type = 1 + (int32)(next() % 3);
        int32 port = (int32)next();
        if (next() & 1) port = (int32)(next() % 70000);
        uint8 ip4[4] = {(uint8)next(), (uint8)next(), (uint8)next(), (uint8)next()};
        std::string host;
        int32 hl = (int32)(next() % 12);
        for (int32 i = 0; i < hl; i++) host += (char)(33 + next() % 94);
        whatismyip_text = (next() & 1) ? "" : "203.0.113.9";

        // build the legacy handle structures
        int32 h = list_add(special_handles);
        special_handle_struct *sh = (special_handle_struct *)list_get(special_handles, h);
        connection_struct *cs = (connection_struct *)calloc(1, sizeof(connection_struct));
        tcp_connection *tcp = new tcp_connection();
        cs->protocol = 1;
        cs->type = type;
        cs->connection = tcp;
        tcp->port = port;
        memcpy(tcp->ip4, ip4, 4);
        tcp->hostname = (uint8 *)strdup(host.c_str());
        if (type == 2) {
            cs->port = port;
            sh->type = 2;
            sh->index = (ptrszint)cs;
        } else {
            stream_struct *ss = (stream_struct *)calloc(1, sizeof(stream_struct));
            ss->type = 1;
            ss->index = (ptrszint)cs;
            sh->type = 1;
            sh->index = (ptrszint)ss;
        }
        new_error = first_error = 0;
        legacy::qbs *l = func__connectionaddress(-1 - h);
        std::string ls((char *)l->chr, l->len);

        uint8 out[256];
        int32 rn = qb_net_format_address(type, port, ip4, (const uint8_t *)host.data(), (int32)host.size(),
                                           (const uint8_t *)whatismyip_text.data(), (int32)whatismyip_text.size(), out,
                                           sizeof(out));
        std::string rs((char *)out, rn);
        if (mutate && type == 3) rs += "!";
        if (ls != rs || first_error) {
            if (failures < 20)
                printf("ADDRESS MISMATCH type=%d port=%d: legacy=\"%s\" err=%d rust=\"%s\"\n", type, port, ls.c_str(),
                       first_error, rs.c_str());
            ++failures;
        }
    }
    return failures;
}

int main(int argc, char **argv) {
    bool quick = false;
    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "--quick")) quick = true;
        if (!strcmp(argv[i], "--mutate")) mutate = true;
    }
    legacy::special_handles = legacy::list_new(sizeof(legacy::special_handle_struct));
    legacy::stream_handles = legacy::list_new(sizeof(legacy::stream_struct));
    legacy::connection_handles = legacy::list_new(sizeof(legacy::connection_struct));
    const long n = quick ? 20000 : 400000;
    long f1 = check_parse(n);
    long f2 = check_address(quick ? 5000 : 100000);
    printf("diff_net: %ld info strings, %ld address texts: %ld + %ld mismatches\n", n, quick ? 5000L : 100000L, f1, f2);
    return (f1 || f2) ? 1 : 0;
}

#include "host_stubs.h"
