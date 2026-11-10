// Differential test: legacy KILL/NAME/CHDIR/MKDIR/RMDIR/FILES/_FILEEXISTS/
// _DIREXISTS/_CWD$/ENVIRON/COMMAND$/SHELL (tests/legacy/sys.inc) vs Rust
// qb_runtime::fsops / qb_runtime::env / qb_runtime::shell. Windows only (legacy Win32 paths).
//
// File-system operations run on random names in two temp folders, one per
// implementation; error codes and the resulting folder listings must match.
// Environment and COMMAND$ compare against the same process state. SHELL
// compares exit codes and side effects of hidden, waited commands.
//
//   g++ -O1 -std=c++17 -w tests/diff/diff_sys.cpp <libqb_runtime.a> <native libs> -o diff_sys
//   ./diff_sys [--quick]

#define NOMINMAX
#include <windows.h>
#include <shellapi.h>
#include <direct.h>
#include <io.h>
#include <cerrno>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <algorithm>
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
#define QBNex_WINDOWS
#include "qbs_stub.h"
static int32 new_error = 0;
static void error(int32 n) {
    if (!new_error) new_error = n;
}
static char *fixdir(qbs *filename) {
    for (int32 i = 0; i < filename->len; i++)
        if (filename->chr[i] == 47) filename->chr[i] = 92;
    return (char *)filename->chr;
}
static int32 full_screen = 0, full_screen_set = 0;
static int32 console = 1, console_active = 0;
static int32 func_pos(int32) { return 1; }
static void makefit(qbs *) {}
void sub_shell4(qbs *, int32);
#include "legacy/sys.inc"
#undef envp
} // namespace legacy

#include "../../crates/qb-runtime/include/qb_runtime.h"

static uint64_t rng = 0xDEADBEEFCAFEF00Dull;
static uint64_t next() {
    rng ^= rng << 13;
    rng ^= rng >> 7;
    rng ^= rng << 17;
    return rng;
}
static long rnd(long lo, long hi) { return lo + (long)(next() % (uint64_t)(hi - lo + 1)); }

static long failures = 0, op_index = 0;
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
static legacy::qbs *lq(const std::string &s) { return legacy::qbs_new_txt_len(s.data(), (int32)s.size()); }
static std::string sq(legacy::qbs *q) { return std::string((char *)q->chr, q->len); }

// Sorted recursive listing (names + D/F) of a folder.
static void listing(const std::string &dir, const std::string &rel, std::vector<std::string> &out) {
    WIN32_FIND_DATAA fd;
    HANDLE h = FindFirstFileA((dir + "\\*").c_str(), &fd);
    if (h == INVALID_HANDLE_VALUE) return;
    do {
        std::string n = fd.cFileName;
        if (n == "." || n == "..") continue;
        bool d = fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY;
        out.push_back(rel + n + (d ? "/" : ""));
        if (d) listing(dir + "\\" + n, rel + n + "/", out);
    } while (FindNextFileA(h, &fd));
    FindClose(h);
    std::sort(out.begin(), out.end());
}
static std::string list_str(const std::string &dir) {
    std::vector<std::string> v;
    listing(dir, "", v);
    std::string s;
    for (auto &x : v) s += x + " ";
    return s;
}

static const char *names[] = {"a.txt", "b.txt", "d1", "d1/c.txt", "d1/d2", "*.txt", "d1/*", "nope/x.txt", "d?", ""};
static const int NNAMES = 10;

static void make_file(const std::string &p) {
    FILE *f = fopen(p.c_str(), "wb");
    if (f) {
        fputs("x", f);
        fclose(f);
    }
}

int main(int argc, char **argv) {
    const bool quick = argc > 1 && strcmp(argv[1], "--quick") == 0;
    const long n = quick ? 3000 : 60000;
    char tmp[MAX_PATH];
    GetTempPathA(MAX_PATH, tmp);
    std::string base = std::string(tmp) + "qb_diff_sys_" + std::to_string(GetCurrentProcessId());
    std::string dirA = base + "_legacy", dirB = base + "_rust";
    CreateDirectoryA(dirA.c_str(), NULL);
    CreateDirectoryA(dirB.c_str(), NULL);

    // --- file system ---------------------------------------------------------
    for (op_index = 0; op_index < n; ++op_index) {
        legacy::new_error = 0;
        std::string x = names[next() % NNAMES], y = names[next() % NNAMES];
        std::string a = dirA + "/" + x, b = dirB + "/" + x, a2 = dirA + "/" + y, b2 = dirB + "/" + y;
        int op = (int)rnd(0, 7);
        int32 e = 0;
        switch (op) {
        case 0: // create a file in both trees (setup, not compared)
            make_file(a);
            make_file(b);
            break;
        case 1:
            legacy::sub_mkdir(lq(a));
            e = qb_mkdir((const uint8_t *)b.data(), (int32)b.size());
            if (legacy::new_error != e) fail("mkdir", legacy::new_error, e);
            break;
        case 2:
            legacy::sub_rmdir(lq(a));
            e = qb_rmdir((const uint8_t *)b.data(), (int32)b.size());
            if (legacy::new_error != e) fail("rmdir", legacy::new_error, e);
            break;
        case 3:
            legacy::sub_kill(lq(a));
            e = qb_kill((const uint8_t *)b.data(), (int32)b.size());
            if (legacy::new_error != e) fail("kill", legacy::new_error, e);
            break;
        case 4:
            legacy::sub_name(lq(a), lq(a2));
            e = qb_name((const uint8_t *)b.data(), (int32)b.size(), (const uint8_t *)b2.data(), (int32)b2.size());
            if (legacy::new_error != e) fail("name", legacy::new_error, e);
            break;
        case 5: {
            int32 ra = legacy::func__fileexists(lq(a)), rb = qb_exists((const uint8_t *)b.data(), (int32)b.size(), 0);
            if (ra != rb) fail("_fileexists", ra, rb);
            ra = legacy::func__direxists(lq(a)), rb = qb_exists((const uint8_t *)b.data(), (int32)b.size(), 1);
            if (ra != rb) fail("_direxists", ra, rb);
            break;
        }
        case 6: { // FILES (same pattern relative to each tree; directory line differs by folder name)
            std::string pa = a, pb = b;
            int32 passed = (int32)rnd(0, 7) != 0;
            legacy::printed_lines.clear();
            legacy::sub_files(lq(pa), passed);
            const uint8_t *out;
            int32 olen = 0;
            e = qb_files((const uint8_t *)pb.data(), (int32)pb.size(), passed, &out, &olen);
            if (legacy::new_error != e) fail("files err", legacy::new_error, e);
            std::string ra, rb((const char *)out, olen);
            for (size_t k = 0; k < legacy::printed_lines.size(); k++) ra += (k ? "\n" : "") + legacy::printed_lines[k];
            // free space may change between the two calls; compare up to the last line
            ra = ra.substr(0, ra.rfind('\n') == std::string::npos ? ra.size() : ra.rfind('\n'));
            rb = rb.substr(0, rb.rfind('\n') == std::string::npos ? rb.size() : rb.rfind('\n'));
            if (passed) cmp_str("files", ra, rb);
            break;
        }
        default: { // CHDIR + _CWD$ (shared process state: legacy then Rust)
            char keep[MAX_PATH];
            GetCurrentDirectoryA(MAX_PATH, keep);
            legacy::sub_chdir(lq(a));
            int32 ea = legacy::new_error;
            std::string ca = sq(legacy::func__cwd());
            SetCurrentDirectoryA(keep);
            e = qb_chdir((const uint8_t *)b.data(), (int32)b.size());
            const uint8_t *out;
            int32 olen = 0;
            qb_cwd(&out, &olen);
            std::string cb((const char *)out, olen);
            SetCurrentDirectoryA(keep);
            if (ea != e) fail("chdir", ea, e);
            if (ea == 0) {
                // same relative location in each tree
                cmp_str("_cwd$", ca.substr(dirA.size()), cb.substr(dirB.size()));
            }
            break;
        }
        }
        std::string la = list_str(dirA), lb = list_str(dirB);
        if (la != lb) {
            cmp_str("tree", la, lb);
            break;
        }
        if (failures >= 25) break;
    }

    // --- environment ---------------------------------------------------------------
    const char *envs[] = {"QB_E1=one", "QB_E2 two words", "QB_E1=", "QB_E3=a=b", "QB_E2 ", "NOSEPARATOR",
                          "QB_E4= lead", "QB_E5==x", " =y", "QB_E6=x y"};
    for (long k = 0; k < (quick ? 200 : 2000); ++k, ++op_index) {
        std::string s = envs[next() % 10];
        legacy::new_error = 0;
        int32 e;
        if (rnd(0, 1)) {
            legacy::sub_environ(lq(s));
            e = legacy::new_error;
        } else {
            e = qb_environ_set((const uint8_t *)s.data(), (int32)s.size());
            if (e) legacy::error(e);
        }
        legacy::new_error = 0;
        legacy::sub_environ(lq(s)); // same call again on the legacy side: same result
        if (legacy::new_error != e) fail("environ", legacy::new_error, e);
        for (const char *q : {"QB_E1", "QB_E2", "QB_E3", "QB_E5", "QB_E6", "PATH", "NOSUCH"}) {
            const uint8_t *out;
            int32 olen = 0;
            qb_environ_get((const uint8_t *)q, (int32)strlen(q), &out, &olen);
            cmp_str("environ$(name)", sq(legacy::func_environ(legacy::qbs_new_txt(q))), std::string((const char *)out, olen));
        }
        int32 ca = legacy::func__environcount(), cb = qb_environ_count(_environ);
        if (ca != cb) fail("_environcount", ca, cb);
        for (int32 i = -1; i <= ca + 1; i++) {
            legacy::new_error = 0;
            std::string ra = sq(legacy::func_environ(i));
            const uint8_t *out;
            int32 olen = 0;
            int32 eb = qb_environ_index(_environ, i, &out, &olen);
            if (legacy::new_error != eb) fail("environ$(n) err", legacy::new_error, eb);
            cmp_str("environ$(n)", ra, std::string((const char *)out, olen));
        }
    }

    // --- COMMAND$ ---------------------------------------------------------------------
    {
        static char a0[] = "prog.exe", a1[] = "first", a2[] = "two words";
        static char *argv3[] = {a0, a1, a2, NULL};
        legacy::func_command_array = argv3;
        legacy::func_command_count = 3;
        for (int32 i = -2; i <= 4; i++) {
            const uint8_t *out;
            int32 olen = 0;
            qb_command_arg(argv3, 3, i, &out, &olen);
            cmp_str("command$(i)", sq(legacy::func_command(i, 1)), std::string((const char *)out, olen));
        }
        legacy::func_command_array = NULL;
        const uint8_t *out;
        int32 olen = 0;
        qb_command_arg(NULL, 3, 0, &out, &olen);
        cmp_str("command$(null)", sq(legacy::func_command(0, 1)), std::string((const char *)out, olen));
    }

    // --- SHELL (hidden, waited) + console system() ---------------------------------------
    const char *cmds[] = {"exit 3", "cmd /c exit 9", "dir >nul", "echo hi>nul", "  exit 4", "\"cmd\" /c exit 2",
                          "nosuchprogram_qb_runtime", "exit 0", "set QBTESTX=1", "type nul>nul"};
    for (long k = 0; k < (quick ? 10 : 40); ++k, ++op_index) {
        std::string c = cmds[k % 10];
        legacy::new_error = 0;
        int64 ra = legacy::func__shellhide(lq(c));
        int64 rb = qb_shell_run((const uint8_t *)c.data(), (int32)c.size(), 0, 1, 0);
        if (ra != rb) fail(("_shellhide " + c).c_str(), (long)ra, (long)rb);
        legacy::console_active = 1; // SHELL with an active console goes through system()
        ra = legacy::func_shell(lq(c));
        legacy::console_active = 0;
        rb = qb_shell_run((const uint8_t *)c.data(), (int32)c.size(), 1, 1, 1);
        if (ra != rb) fail(("shell console " + c).c_str(), (long)ra, (long)rb);
    }
    // _DONTWAIT _HIDE: side effect appears later on both sides
    {
        std::string fa = dirA + "\\dw.txt", fb = dirB + "\\dw.txt";
        std::string ca = "cmd /c echo x>\"" + fa + "\"", cb = "cmd /c echo x>\"" + fb + "\"";
        legacy::sub_shell4(lq(ca), 2);
        qb_shell_run((const uint8_t *)cb.data(), (int32)cb.size(), 0, 0, 0);
        for (int t = 0; t < 100 && (GetFileAttributesA(fa.c_str()) == INVALID_FILE_ATTRIBUTES ||
                                    GetFileAttributesA(fb.c_str()) == INVALID_FILE_ATTRIBUTES);
             t++)
            Sleep(50);
        Sleep(100);
        cmp_str("dontwait side effect", list_str(dirA), list_str(dirB));
    }

    // clean up both trees
    std::string rm = "cmd /c rd /s /q \"" + dirA + "\" & rd /s /q \"" + dirB + "\"";
    system(rm.c_str());
    printf("diff_sys: %s, %ld operations, %ld mismatches\n", quick ? "quick" : "full", op_index, failures);
    return failures == 0 ? 0 : 1;
}

#include "host_stubs.h"
