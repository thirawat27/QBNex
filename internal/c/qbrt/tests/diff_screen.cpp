// Differential test: legacy sub__screenprint / sub__screenclick / func__screenimage
// (tests/legacy/screenprint.inc, screenimage.inc, verbatim) vs Rust qbrt::screen.
//
// The legacy code is compiled twice, once as the Windows build and once as the
// macOS build, against recording mocks of SendInput/VkKeyScan, CoreGraphics and
// GDI. The event sequences each side would send are compared.
//
//   g++ -O1 -std=c++17 -w internal/c/qbrt/tests/diff_screen.cpp <libqbrt.a> <native libs> -o diff_screen
//   ./diff_screen [--quick] [--mutate]

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <vector>

#include "../include/qbrt.h"

typedef int8_t int8;
typedef uint8_t uint8;
typedef uint16_t uint16;
typedef int32_t int32;
typedef uint32_t uint32;
typedef int64_t int64;
typedef uint64_t uint64;

struct qbs {
    uint8 *chr;
    int32 len;
};

static int16_t scan_table[256];  // fake VkKeyScanA
static std::vector<int32> events;  // recorded by the mocks

namespace win {
typedef uint32 DWORD;
typedef uint16 WORD;
typedef int32 LONG;
typedef uintptr_t ULONG_PTR;
struct MOUSEINPUT {
    LONG dx, dy;
    DWORD mouseData, dwFlags, time;
    ULONG_PTR dwExtraInfo;
};
struct KEYBDINPUT {
    WORD wVk, wScan;
    DWORD dwFlags, time;
    ULONG_PTR dwExtraInfo;
};
struct INPUT {
    DWORD type;
    union {
        MOUSEINPUT mi;
        KEYBDINPUT ki;
    };
};
enum { INPUT_MOUSE = 0, INPUT_KEYBOARD = 1, KEYEVENTF_KEYUP = 2 };
enum {
    MOUSEEVENTF_MOVE = 1, MOUSEEVENTF_LEFTDOWN = 2, MOUSEEVENTF_LEFTUP = 4, MOUSEEVENTF_RIGHTDOWN = 8,
    MOUSEEVENTF_RIGHTUP = 16, MOUSEEVENTF_MIDDLEDOWN = 32, MOUSEEVENTF_MIDDLEUP = 64, MOUSEEVENTF_ABSOLUTE = 0x8000
};
enum {
    VK_BACK = 8, VK_TAB = 9, VK_RETURN = 13, VK_SHIFT = 16, VK_CONTROL = 17, VK_PRIOR = 0x21, VK_NEXT = 0x22,
    VK_END = 0x23, VK_HOME = 0x24, VK_LEFT = 0x25, VK_UP = 0x26, VK_RIGHT = 0x27, VK_DOWN = 0x28, VK_INSERT = 0x2D,
    VK_DELETE = 0x2E
};
typedef void *HWND;
struct RECT {
    LONG left, top, right, bottom;
};
static int32 screen_w = 1920, screen_h = 1080;
static HWND GetDesktopWindow() { return (HWND)1; }
static int GetWindowRect(HWND, RECT *r) {
    r->left = 0;
    r->top = 0;
    r->right = screen_w;
    r->bottom = screen_h;
    return 1;
}
static void ZeroMemory(void *p, size_t n) { memset(p, 0, n); }
static uint32 SendInput(uint32 n, INPUT *in, int) {
    if (in->type == INPUT_MOUSE) {
        events.push_back((int32)in->mi.dwFlags);
        events.push_back(in->mi.dx);
        events.push_back(in->mi.dy);
    } else {
        events.push_back(in->ki.wVk | ((in->ki.dwFlags & KEYEVENTF_KEYUP) ? 1 << 16 : 0));
    }
    return n;
}
static int16_t VkKeyScan(int c) { return scan_table[(uint8)c]; }
#define QBNex_WINDOWS
#include "legacy/screenprint.inc"
#undef QBNex_WINDOWS
}  // namespace win

namespace mac {
typedef void *CGEventSourceRef;
typedef uint16 CGKeyCode;
struct Event {
    int32 code, down;
    uint64 flags;
};
typedef Event *CGEventRef;
struct CGPoint {
    double x, y;
};
enum { kCGEventSourceStateCombinedSessionState = 0, kCGAnnotatedSessionEventTap = 2, kCGHIDEventTap = 0 };
enum { kCGEventLeftMouseDown = 1, kCGEventLeftMouseUp = 2, kCGMouseButtonLeft = 0 };
static const uint64 kCGEventFlagMaskShift = 0x20000, kCGEventFlagMaskCommand = 0x100000;
static CGEventSourceRef CGEventSourceCreate(int) { return (void *)1; }
static CGEventRef CGEventCreateKeyboardEvent(CGEventSourceRef, CGKeyCode k, bool down) {
    return new Event{k, down ? 1 : 0, 0};
}
static void CGEventSetFlags(CGEventRef e, uint64 f) { e->flags = f; }
static void CGEventPost(int, CGEventRef e) {
    events.push_back(e->code);
    events.push_back(e->down);
    events.push_back((int32)e->flags);
}
static void CFRelease(void *) {}
static CGPoint CGPointMake(double x, double y) { return CGPoint{x, y}; }
static CGEventRef CGEventCreateMouseEvent(void *, int type, CGPoint p, int) {
    return new Event{(int32)p.x, type, (uint64)p.y};
}
#define QBNex_MACOSX
#include "legacy/screenprint.inc"
#undef QBNex_MACOSX
}  // namespace mac

// _SCREENIMAGE: GDI mocks record the BitBlt source rectangle.
#define SCREENIMAGE_NS(NS, W, H)                                                                                     \
    namespace NS {                                                                                                   \
    typedef void *HWND;                                                                                              \
    typedef void *HDC;                                                                                               \
    typedef void *HBITMAP;                                                                                           \
    struct RECT {                                                                                                    \
        int32 left, top, right, bottom;                                                                              \
    };                                                                                                               \
    struct BITMAPFILEHEADER {};                                                                                      \
    struct BITMAPINFOHEADER {                                                                                        \
        uint32 biSize;                                                                                               \
        int32 biWidth, biHeight;                                                                                     \
        uint16 biPlanes, biBitCount;                                                                                 \
        uint32 biCompression, biSizeImage;                                                                           \
        int32 biXPelsPerMeter, biYPelsPerMeter;                                                                      \
        uint32 biClrUsed, biClrImportant;                                                                            \
    };                                                                                                               \
    struct BITMAPINFO {};                                                                                            \
    enum { BI_RGB = 0, DIB_RGB_COLORS = 0, SRCCOPY = 0xCC0020 };                                                     \
    static HWND GetDesktopWindow() { return (HWND)1; }                                                               \
    static int GetWindowRect(HWND, RECT *r) {                                                                        \
        r->left = 0;                                                                                                 \
        r->top = 0;                                                                                                  \
        r->right = W;                                                                                                \
        r->bottom = H;                                                                                               \
        return 1;                                                                                                    \
    }                                                                                                                \
    static HDC GetDC(void *) { return (HDC)1; }                                                                      \
    static int ReleaseDC(void *, HDC) { return 1; }                                                                  \
    static HDC CreateCompatibleDC(HDC) { return (HDC)2; }                                                            \
    static HBITMAP CreateCompatibleBitmap(HDC, int, int) { return (HBITMAP)3; }                                      \
    static void *SelectObject(HDC, void *) { return nullptr; }                                                       \
    static int DeleteObject(void *) { return 1; }                                                                    \
    static int BitBlt(HDC, int, int, int w, int h, HDC, int x, int y, int) {                                         \
        events.push_back(x);                                                                                         \
        events.push_back(y);                                                                                         \
        events.push_back(w);                                                                                         \
        events.push_back(h);                                                                                         \
        return 1;                                                                                                    \
    }                                                                                                                \
    static int GetDIBits(HDC, HBITMAP, int, int, void *, BITMAPINFO *, int) { return 0; }                            \
    struct page_t {                                                                                                  \
        void *offset;                                                                                                \
    };                                                                                                               \
    static page_t page0, *write_page = &page0;                                                                       \
    static int32 func__dest() { return 0; }                                                                          \
    static void sub__dest(int32) {}                                                                                  \
    static int32 func__newimage(int32 w, int32 h, int32, int32) { return -2; }                                       \
    static void sub__setalpha(int32, int32, int32, int32, int32) {}                                                  \
    static int32 func_screenwidth() { return W; }                                                                    \
    static int32 func_screenheight() { return H; }                                                                   \
    static const int sw = W, sh = H;                                                                                 \
    }

#define DEPENDENCY_SCREENIMAGE
SCREENIMAGE_NS(img1920, 1920, 1080)
namespace img1920 {
#define QBNex_WINDOWS
#include "legacy/screenimage.inc"
#undef QBNex_WINDOWS
}
SCREENIMAGE_NS(img800, 800, 600)
namespace img800 {
#define QBNex_WINDOWS
#include "legacy/screenimage.inc"
#undef QBNex_WINDOWS
}

static uint64_t rng = 0xD1B54A32D192ED03ull;
static uint64_t next() {
    rng ^= rng << 13;
    rng ^= rng >> 7;
    rng ^= rng << 17;
    return rng;
}

static bool mutate = false;

static std::vector<uint8> random_text() {
    static const uint8 interesting[] = {0, 1, 3, 8, 9, 13, 15, 26, 27, 31, 32, 'a', 'A', '!', '~', 126, 127, 128, 200, 255,
                                        71, 72, 73, 75, 77, 79, 80, 81, 82, 83, 59};
    std::vector<uint8> t(next() % 12);
    for (auto &c : t) c = (next() & 1) ? interesting[next() % sizeof(interesting)] : (uint8)next();
    return t;
}

static bool same(const std::vector<int32> &a, const int32 *b, int32 n) {
    return (int32)a.size() == n && (n == 0 || !memcmp(a.data(), b, n * sizeof(int32)));
}

int main(int argc, char **argv) {
    bool quick = false;
    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "--quick")) quick = true;
        if (!strcmp(argv[i], "--mutate")) mutate = true;
    }
    const long n = quick ? 20000 : 500000;
    long failures = 0;
    std::vector<int32> out(4096);

    for (long k = 0; k < n; ++k) {
        // new random keyboard layout every 1000 strings (incl. -1 = no key)
        if (k % 1000 == 0)
            for (auto &s : scan_table) s = (next() % 10 == 0) ? (int16_t)-1 : (int16_t)(next() & 0x07FF);
        std::vector<uint8> t = random_text();
        qbs q{t.data(), (int32)t.size()};

        events.clear();
        win::sub__screenprint(&q);
        int32 rn = qbrt_test_win_screenprint(t.data(), (int32)t.size(), scan_table, out.data(), (int32)out.size());
        if (mutate && rn > 2) out[1] ^= 1 << 16;
        if (!same(events, out.data(), rn)) {
            if (failures < 10) printf("WIN SCREENPRINT MISMATCH len=%d legacy=%zu rust=%d events\n", q.len, events.size(), rn);
            ++failures;
        }

        events.clear();
        mac::sub__screenprint(&q);
        rn = qbrt_test_mac_screenprint(t.data(), (int32)t.size(), out.data(), (int32)out.size() / 3);
        if (!same(events, out.data(), rn * 3)) {
            if (failures < 10) printf("MAC SCREENPRINT MISMATCH len=%d legacy=%zu rust=%d events\n", q.len, events.size() / 3, rn);
            ++failures;
        }
    }

    for (long k = 0; k < n; ++k) {
        int32 x = (int32)(next() % 5000) - 1000, y = (int32)(next() % 5000) - 1000;
        int32 button = (int32)(next() % 6) - 1, passed = (int32)(next() & 1);
        win::screen_w = 1 + (int32)(next() % 4000);
        win::screen_h = 1 + (int32)(next() % 3000);
        events.clear();
        win::sub__screenclick(x, y, button, passed);
        int32 rn = qbrt_test_win_screenclick(x, y, button, passed, win::screen_w, win::screen_h, out.data(), 64);
        if (mutate && rn) out[1] += 1;
        if (!same(events, out.data(), rn * 3)) {
            if (failures < 10) printf("SCREENCLICK MISMATCH x=%d y=%d button=%d passed=%d\n", x, y, button, passed);
            ++failures;
        }
    }

    for (long k = 0; k < n; ++k) {
        int32 x1 = (int32)(next() % 3000) - 500, y1 = (int32)(next() % 3000) - 500;
        int32 x2 = (int32)(next() % 3000) - 500, y2 = (int32)(next() % 3000) - 500;
        int32 passed = (int32)(next() & 1);
        bool big = next() & 1;
        events.clear();
        if (big)
            img1920::func__screenimage(x1, y1, x2, y2, passed);
        else
            img800::func__screenimage(x1, y1, x2, y2, passed);
        int32 r[4];
        qbrt_test_screen_region(big ? 1920 : 800, big ? 1080 : 600, x1, y1, x2, y2, passed, r);
        if (mutate) r[2] -= 1;
        if (!same(events, r, 4)) {
            if (failures < 10)
                printf("SCREENIMAGE MISMATCH %d,%d-%d,%d passed=%d: legacy %d,%d %dx%d rust %d,%d %dx%d\n", x1, y1, x2, y2,
                       passed, events[0], events[1], events[2], events[3], r[0], r[1], r[2], r[3]);
            ++failures;
        }
    }

    printf("diff_screen: %ld strings (Windows + macOS), %ld clicks, %ld capture regions: %ld mismatches\n", n, n, n,
           failures);
    return failures ? 1 : 0;
}
