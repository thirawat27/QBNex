// Differential test: legacy GFS layer + file statements (tests/legacy/files.inc)
// vs Rust qbrt::gfs / qbrt::fileio (Windows only: legacy uses GFS_WINDOWS).
// The same random operation sequence runs in two temp directories, one per
// implementation. Error codes, returned values, strings, the INPUT # number
// representation and the bytes of every file must match after each operation.
// FIELD is covered by unit and conformance tests (it needs live qbs strings).
//
//   g++ -O1 -std=c++17 -w internal/c/qbrt/tests/diff_files.cpp <libqbrt.a> <native libs> -o diff_files
//   ./diff_files [--quick]

#define NOMINMAX
#include <windows.h>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <string>
#include <vector>

typedef int8_t int8;
typedef int16_t int16;
typedef int32_t int32;
typedef int64_t int64;
typedef uint8_t uint8;
typedef uint16_t uint16;
typedef uint32_t uint32;
typedef uint64_t uint64;
typedef intptr_t ptrszint;

namespace legacy {
using namespace std;
#define GFS_WINDOWS
#define QBNex_WINDOWS

struct qbs_field {
    int32 fileno;
    int64 fileid;
    int64 size;
    int64 offset;
};
struct qbs {
    uint8 *chr;
    int32 len;
    uint8 fixed;
    qbs_field *field;
};
static qbs *qbs_new(int32 len, uint8 tmp) {
    qbs *s = new qbs();
    s->chr = (uint8 *)calloc(len + 1, 1);
    s->len = len;
    return s;
}
static qbs *qbs_new_txt_len(const char *t, int32 len) {
    qbs *s = qbs_new(len, 1);
    memcpy(s->chr, t, len);
    return s;
}
static qbs *qbs_new_txt(const char *t) { return qbs_new_txt_len(t, (int32)strlen(t)); }
static qbs *qbs_set(qbs *d, qbs *s) {
    if (d->fixed) {
        int32 n = s->len < d->len ? s->len : d->len;
        memcpy(d->chr, s->chr, n);
        memset(d->chr + n, 32, d->len - n);
        return d;
    }
    uint8 *c = (uint8 *)calloc(s->len + 1, 1);
    memcpy(c, s->chr, s->len);
    d->chr = c;
    d->len = s->len;
    return d;
}
static void qbs_free(qbs *s) {}
static qbs *qbs_add(qbs *a, qbs *b) {
    qbs *s = qbs_new(a->len + b->len, 1);
    memcpy(s->chr, a->chr, a->len);
    memcpy(s->chr + a->len, b->chr, b->len);
    return s;
}
static qbs *qbs_ucase(qbs *a) {
    qbs *s = qbs_new_txt_len((char *)a->chr, a->len);
    for (int i = 0; i < s->len; i++)
        if (s->chr[i] >= 'a' && s->chr[i] <= 'z') s->chr[i] -= 32;
    return s;
}
static qbs *qbs_left(qbs *a, int32 n) {
    if (n < 0) n = 0;
    if (n > a->len) n = a->len;
    return qbs_new_txt_len((char *)a->chr, n);
}
static qbs *qbs_str(int64 v) {
    char b[32];
    snprintf(b, 32, "% lld", (long long)v);
    return qbs_new_txt(b);
}
static qbs *func_space(int32 n) {
    qbs *s = qbs_new(n < 0 ? 0 : n, 1);
    memset(s->chr, 32, s->len);
    return s;
}
static int32 func_instr(int32 start, qbs *a, qbs *b, int32 passed) {
    for (int32 i = 0; i + b->len <= a->len; i++)
        if (memcmp(a->chr + i, b->chr, b->len) == 0) return i + 1;
    return 0;
}
static int64 qbr(double v) { return (int64)llround(v); }
static qbs *printed = nullptr;
static void qbs_print(qbs *s, int32 nl) { printed = s; }
static qbs *qbs_inkey() { return qbs_new(0, 1); }
static void evnt(int32) {}
static int32 stop_program = 0;
static int32 vwatch = 0;
static void showvalue(int32) {}

struct byte_element_struct {
    uint64 offset;
    int32 length;
};
static void *byte_element(uint64 offset, int32 length, byte_element_struct *info) {
    if (length < 0) length = 0;
    info->length = length;
    info->offset = offset;
    return info;
}
struct special_handle_struct {
    int32 type;
    ptrszint index;
};
struct stream_struct {
    int32 type, in_size, eof;
    uint8 *in;
};
struct list {
    int32 indexes;
};
static list special_handles_v = {0};
static list *special_handles = &special_handles_v;
static void *list_get(list *, int32) { return NULL; }
static void stream_update(stream_struct *) {}
static void stream_out(stream_struct *, void *, int32) {}
static void connection_close(int32) {}

static int32 new_error = 0;
static void error(int32 n) {
    if (!new_error) new_error = n;
}
static char *fixdir(qbs *filename) {
    for (int32 i = 0; i < filename->len; i++)
        if (filename->chr[i] == 47) filename->chr[i] = 92;
    return (char *)filename->chr;
}

uint16 n_digits;
uint8 n_digit[256];
int64 n_exp;
uint8 n_neg;
uint8 n_hex;

// forward declarations libqb.cpp had at its top
struct gfs_file_struct;
int32 gfs_eof_passed(int32 i);
int32 gfs_eof_reached(int32 i);
int64 gfs_getpos(int32 i);
int32 gfs_fileno_valid(int32 f);
int32 gfs_fileno_freefile();
void gfs_fileno_use(int32 f, int32 i);
int32 gfs_open(qbs *filename, int32 access, int32 restrictions, int32 how);
int32 gfs_close(int32 i);
int64 gfs_lof(int32 i);
int32 gfs_setpos(int32 i, int64 position);
int32 gfs_write(int32 i, int64 position, uint8 *data, int64 size);
int32 gfs_read(int32 i, int64 position, uint8 *data, int64 size);
int64 gfs_read_bytes();
int32 gfs_free(int32 i);
int32 gfs_validhandle(int32 i);
void sub_get2(int32 i, int64 offset, qbs *str, int32 passed);
int64 func_seek(int32 i);
int64 func_lof(int32 i);
int32 func_eof(int32 i);
void field_update(int32 fileno);
void field_free(qbs *str);
#include "legacy/files.inc"
} // namespace legacy

#include "../include/qbrt.h"

static uint64_t rng = 0x2545F4914F6CDD1Dull;
static uint64_t next() {
    rng ^= rng << 13;
    rng ^= rng >> 7;
    rng ^= rng << 17;
    return rng;
}
static long rnd(long lo, long hi) { return lo + (long)(next() % (uint64_t)(hi - lo + 1)); }

static long failures = 0;
static long op_index = 0;
static void fail(const char *what, long x, long y) {
    if (failures < 25) printf("MISMATCH %s at op %ld: legacy=%ld rust=%ld\n", what, op_index, x, y);
    ++failures;
}
static void cmp_str(const char *what, const std::string &a, const std::string &b) {
    if (a != b) {
        if (failures < 25) printf("MISMATCH %s at op %ld: legacy=[%s] rust=[%s]\n", what, op_index, a.c_str(), b.c_str());
        ++failures;
    }
}

static std::string dirA, dirB;
static const char *names[] = {"f1.txt", "f2.dat", "sub/f3.bin", "missing/x.txt", "SCRN:", "COM1:9600,N,8,1"};

static std::string slurp(const std::string &p) {
    // Shared read through Win32: files may be open, share-locked or LOCKed.
    HANDLE h = CreateFileA(p.c_str(), GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE, NULL, OPEN_EXISTING, 0, NULL);
    if (h == INVALID_HANDLE_VALUE) return "<unreadable " + std::to_string(GetLastError()) + ">";
    std::string s;
    char buf[4096];
    DWORD got = 0;
    while (ReadFile(h, buf, sizeof buf, &got, NULL) && got) s.append(buf, got);
    if (GetLastError() == ERROR_LOCK_VIOLATION) s += "<locked>";
    CloseHandle(h);
    return s;
}

static std::string path(const std::string &dir, int k) {
    if (k >= 4) return names[k];
    std::string p = dir + "/" + names[k];
    return p;
}

// Random printable-ish data biased towards INPUT # syntax.
static std::string random_text() {
    static const char *atoms[] = {"12", "-3.5", "1E3", "&H1F", "&O17", "&B101", " ", ",", "\"q,x\"", "\r\n", "\n",
                                  "\r", "abc", "\t", "\x1a", ".5", "+", "D-2", "0000", "99999999999999999999"};
    std::string s;
    int n = (int)rnd(0, 6);
    for (int i = 0; i < n; i++) s += atoms[next() % 20];
    return s;
}

static legacy::qbs *lq(const std::string &s) { return legacy::qbs_new_txt_len(s.data(), (int32)s.size()); }

int main(int argc, char **argv) {
    const bool quick = argc > 1 && strcmp(argv[1], "--quick") == 0;
    const long n = quick ? 20000 : 400000;
    char tmp[MAX_PATH];
    GetTempPathA(MAX_PATH, tmp);
    std::string base = std::string(tmp) + "qbrt_diff_files_" + std::to_string(GetCurrentProcessId());
    dirA = base + "_legacy";
    dirB = base + "_rust";
    for (auto &d : {dirA, dirB}) {
        CreateDirectoryA(d.c_str(), NULL);
        CreateDirectoryA((d + "/sub").c_str(), NULL);
    }
    for (int i = 0; i < 32; i++) legacy::sub_file_print_spaces[i] = 32;

    for (op_index = 0; op_index < n; ++op_index) {
        legacy::new_error = 0;
        int32 fno = (int32)rnd(-1, 5);
        int op = (int)rnd(0, 17);
        // PRINT # / INPUT # string to a TCP/IP handle return early in the C++ shim
        if (fno < 0 && (op == 3 || op == 4 || op == 5)) fno = 1;
        if (getenv("QBRT_TRACE")) {
            printf("op %ld: %d f%d\n", op_index, op, fno);
            fflush(stdout);
        }
        switch (op) {
        case 0:
        case 1: { // OPEN
            int k = (int)rnd(0, 4); // no COM ports: opening a real one may block
            if (rnd(0, 20)) k = (int)rnd(0, 2);
            int32 type = (int32)rnd(0, 5), access = (int32)rnd(0, 3), sharing = (int32)rnd(0, 4);
            if (rnd(0, 3)) access = 0;
            if (rnd(0, 3)) sharing = 0;
            int32 passed = (int32)rnd(0, 1);
            int64 rl = rnd(-2, 20);
            std::string a = path(dirA, k), b = path(dirB, k);
            if (rnd(0, 10) == 0) {
                const char *ts[] = {"R", "b", "I", "o", "A", "x", ""};
                const char *t = ts[next() % 7];
                legacy::sub_open_gwbasic(legacy::qbs_new_txt(t), fno, lq(a), rl, passed);
                int32 e = qbrt_file_open_gwbasic((const uint8_t *)t, (int32)strlen(t), fno, (const uint8_t *)b.data(),
                                                 (int32)b.size(), rl, passed);
                if (legacy::new_error != e) fail("open_gwbasic", legacy::new_error, e);
            } else {
                if (getenv("QBRT_TRACE")) printf("  open k=%d type=%d acc=%d sh=%d rl=%d p=%d\n", k, type, access, sharing, (int)rl, passed), fflush(stdout);
                legacy::sub_open(lq(a), type, access, sharing, fno, rl, passed);
                if (getenv("QBRT_TRACE")) printf("  legacy done %d\n", legacy::new_error), fflush(stdout);
                int32 e = qbrt_file_open((const uint8_t *)b.data(), (int32)b.size(), type, access, sharing, fno, rl, passed);
                if (legacy::new_error != e) fail("open", legacy::new_error, e);
            }
            break;
        }
        case 2: { // CLOSE
            if (rnd(0, 15) == 0) {
                legacy::sub_close(0, 0);
                qbrt_file_close_all();
            } else if (fno > 0) {
                legacy::sub_close(fno, 1);
                qbrt_file_close(fno);
            }
            break;
        }
        case 3:
        case 4: { // PRINT #
            std::string s = random_text();
            int32 es = (int32)rnd(0, 1), tab = (int32)rnd(0, 1), nl = (int32)rnd(0, 1);
            legacy::printed = nullptr;
            legacy::sub_file_print(fno, lq(s), es, tab, nl);
            int32 e = qbrt_file_print(fno, (const uint8_t *)s.data(), (int32)s.size(), es, tab, nl);
            if (e == -1) e = legacy::printed ? 0 : 999;
            if (legacy::new_error != e) fail("print", legacy::new_error, e);
            break;
        }
        case 5: { // INPUT # string
            legacy::qbs *d = lq("old");
            legacy::sub_file_input_string(fno, d);
            const uint8_t *out;
            int32 olen = 0, assign = 0;
            int32 e = qbrt_file_input_string(fno, &out, &olen, &assign);
            std::string rb = assign ? std::string((const char *)out, olen) : "old";
            if (legacy::new_error != e) fail("input$ err", legacy::new_error, e);
            cmp_str("input$", std::string((char *)d->chr, d->len), rb);
            break;
        }
        case 6: { // INPUT # number
            int32 ra = legacy::n_inputnumberfromfile(fno);
            int32 ea = legacy::new_error;
            qbrt_num_repr nr;
            int32 rb = -1;
            int32 eb = qbrt_file_input_number(fno, &nr, &rb);
            if (ea != eb) fail("input# err", ea, eb);
            if (ra != rb) fail("input# result", ra, rb);
            if (ra == 0 || ra == 1) {
                if (legacy::n_digits != nr.digits) fail("n_digits", legacy::n_digits, nr.digits);
                if (ra == 0 && legacy::n_exp != nr.exp) fail("n_exp", (long)legacy::n_exp, (long)nr.exp);
                if (legacy::n_neg != nr.neg) fail("n_neg", legacy::n_neg, nr.neg);
                if (legacy::n_hex != nr.hex) fail("n_hex", legacy::n_hex, nr.hex);
                if (memcmp(legacy::n_digit, nr.digit, legacy::n_digits)) fail("n_digit", 0, 1);
            }
            break;
        }
        case 7: { // LINE INPUT #
            legacy::qbs *d = lq("old");
            legacy::sub_file_line_input_string(fno, d);
            const uint8_t *out;
            int32 olen = 0, assign = 0;
            int32 e = qbrt_file_line_input(fno, &out, &olen, &assign);
            std::string rb = assign ? std::string((const char *)out, olen) : "old";
            if (legacy::new_error != e) fail("line input err", legacy::new_error, e);
            cmp_str("line input", std::string((char *)d->chr, d->len), rb);
            break;
        }
        case 8: { // INPUT$(n, #f)
            int32 cnt = (int32)rnd(-1, 8);
            legacy::qbs *a = legacy::func_input(cnt, fno, 1);
            int32 ea = legacy::new_error;
            std::vector<uint8_t> buf(cnt < 0 ? 0 : cnt);
            int32 len = 0;
            int32 eb = qbrt_file_input_n(cnt, fno, buf.data(), (int32)buf.size(), &len);
            if (ea != eb) fail("input$(n) err", ea, eb);
            if (ea == 0 || a->len == len)
                cmp_str("input$(n)", std::string((char *)a->chr, a->len), std::string((char *)buf.data(), len));
            else
                fail("input$(n) len", a->len, len);
            break;
        }
        case 9: { // GET / PUT bytes
            int32 len = (int32)rnd(0, 12), passed = (int32)rnd(0, 1);
            int64 off = rnd(-1, 6);
            if (rnd(0, 1)) {
                std::vector<uint8_t> a(len, 0xEE), b(len, 0xEE);
                legacy::byte_element_struct be;
                legacy::sub_get(fno, off, legacy::byte_element((uint64)a.data(), len, &be), passed);
                int32 e = qbrt_file_get(fno, off, b.data(), len, passed);
                if (legacy::new_error != e) fail("get err", legacy::new_error, e);
                if (a != b) fail("get data", 0, 1);
            } else {
                std::string s = random_text().substr(0, len);
                legacy::byte_element_struct be;
                legacy::sub_put(fno, off, legacy::byte_element((uint64)s.data(), (int32)s.size(), &be), passed);
                int32 e = qbrt_file_put(fno, off, (const uint8_t *)s.data(), (int32)s.size(), passed);
                if (legacy::new_error != e) fail("put err", legacy::new_error, e);
            }
            break;
        }
        case 10: { // GET / PUT variable-length strings
            int32 passed = (int32)rnd(0, 1);
            int64 off = rnd(-1, 6);
            if (rnd(0, 1)) {
                std::string init(rnd(0, 6), 'z');
                legacy::qbs *d = lq(init);
                std::string bb = init;
                legacy::sub_get2(fno, off, d, passed);
                const uint8_t *out;
                int32 olen = 0, assign = 0;
                int32 e = qbrt_file_get_string(fno, off, (uint8_t *)&bb[0], (int32)bb.size(), passed, &out, &olen, &assign);
                if (assign) bb = std::string((const char *)out, olen);
                if (legacy::new_error != e) fail("get2 err", legacy::new_error, e);
                cmp_str("get2", std::string((char *)d->chr, d->len), bb);
            } else {
                std::string s = random_text();
                legacy::byte_element_struct be;
                legacy::sub_put2(fno, off, legacy::byte_element((uint64)s.data(), (int32)s.size(), &be), passed);
                int32 e = qbrt_file_put_string(fno, off, (const uint8_t *)s.data(), (int32)s.size(), passed);
                if (legacy::new_error != e) fail("put2 err", legacy::new_error, e);
            }
            break;
        }
        case 11: { // LOF / EOF / LOC / SEEK()
            int64 a = legacy::func_lof(fno), b = -7;
            int32 e = qbrt_file_lof(fno, &b);
            if (legacy::new_error != e || a != b) fail("lof", (long)a, (long)b);
            legacy::new_error = 0;
            int32 ea = legacy::func_eof(fno), eb = -7;
            e = qbrt_file_eof(fno, &eb);
            if (legacy::new_error != e || ea != eb) fail("eof", ea, eb);
            legacy::new_error = 0;
            if (fno > 0 && legacy::gfs_fileno_valid(fno) == 1) { // legacy SEEK()/LOC() skip new_error
                a = legacy::func_loc(fno);
                e = qbrt_file_loc(fno, &b);
                if (legacy::new_error != e || a != b) fail("loc", (long)a, (long)b);
                legacy::new_error = 0;
                a = legacy::func_seek(fno);
                e = qbrt_file_seek_get(fno, &b);
                if (legacy::new_error != e || a != b) fail("seek()", (long)a, (long)b);
            }
            break;
        }
        case 12: { // SEEK #
            int64 pos = rnd(-1, 30);
            legacy::sub_seek(fno, pos);
            int32 e = qbrt_file_seek_set(fno, pos);
            if (legacy::new_error != e) fail("seek", legacy::new_error, e);
            break;
        }
        case 13: { // LOCK / UNLOCK
            int64 s = rnd(-1, 5), en = rnd(-1, 8);
            int32 passed = (int32)rnd(0, 3), lk = (int32)rnd(0, 1);
            if (lk) legacy::sub_lock(fno, s, en, passed);
            else legacy::sub_unlock(fno, s, en, passed);
            int32 e = qbrt_file_lock(fno, s, en, passed, lk);
            if (legacy::new_error != e) fail("lock", legacy::new_error, e);
            break;
        }
        case 14: { // FREEFILE
            int32 a = legacy::func_freefile(), b = qbrt_file_freefile();
            if (a != b) fail("freefile", a, b);
            break;
        }
        case 15: { // generic_put / generic_get
            int32 off = (int32)rnd(-1, 10), len = (int32)rnd(0, 6);
            if (rnd(0, 1)) {
                std::string s = random_text().substr(0, len);
                legacy::generic_put(fno, off, (uint8 *)s.data(), (int32)s.size());
                int32 e = qbrt_generic_put(fno, off, (const uint8_t *)s.data(), (int32)s.size());
                if (legacy::new_error != e) fail("generic_put", legacy::new_error, e);
            } else {
                std::vector<uint8_t> a(len, 0xEE), b(len, 0xEE);
                legacy::generic_get(fno, off, a.data(), len);
                int32 rd = -1;
                int32 e = qbrt_generic_get(fno, off, b.data(), len, &rd);
                if (legacy::new_error != e) fail("generic_get err", legacy::new_error, e);
                if (legacy::generic_get_bytes_read != rd) fail("generic_get n", legacy::generic_get_bytes_read, rd);
                if (a != b) fail("generic_get data", 0, 1);
            }
            break;
        }
        default: { // LSET / RSET on plain strings
            std::string d(rnd(0, 6), 'x'), s = random_text();
            legacy::qbs *ld = lq(d);
            legacy::qbs *ls = lq(s);
            std::string rd = d;
            if (op == 16) {
                legacy::sub_lset(ld, ls);
                qbrt_lset((uint8_t *)&rd[0], (int32)rd.size(), (const uint8_t *)s.data(), (int32)s.size());
            } else {
                legacy::sub_rset(ld, ls);
                qbrt_rset((uint8_t *)&rd[0], (int32)rd.size(), (const uint8_t *)s.data(), (int32)s.size());
            }
            cmp_str("lset/rset", std::string((char *)ld->chr, ld->len), rd);
            break;
        }
        }
        // file contents after every operation
        for (int k = 0; k < 3; k++) {
            std::string a = slurp(path(dirA, k)), b = slurp(path(dirB, k));
            if (a != b) {
                fail("file bytes", (long)a.size(), (long)b.size());
                break;
            }
        }
        if (failures >= 25) break;
    }
    legacy::sub_close(0, 0);
    qbrt_file_close_all();
    for (auto &d : {dirA, dirB}) {
        for (int k = 0; k < 3; k++) DeleteFileA(path(d, k).c_str());
        RemoveDirectoryA((d + "/sub").c_str());
        RemoveDirectoryA(d.c_str());
    }
    printf("diff_files: %s, %ld operations, %ld mismatches\n", quick ? "quick" : "full", op_index, failures);
    return failures == 0 ? 0 : 1;
}
