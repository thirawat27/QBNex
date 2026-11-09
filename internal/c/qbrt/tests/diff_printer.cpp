// Differential test: legacy sub__printimage (tests/legacy/printer.inc, a verbatim
// copy of internal/c/libqb/printer.cpp) vs the Rust page flow in qbrt::printer.
//
// GDI and the image engine are replaced by recorders; every combination of
// failing GDI calls, page sizes and "scaled image could not be created" is run
// through both, and the sequences of printer calls are compared.
//
//   g++ -O1 -std=c++17 -w internal/c/qbrt/tests/diff_printer.cpp <libqbrt.a> <native libs> -o diff_printer
//   ./diff_printer [--quick] [--mutate]

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <vector>

#include "../include/qbrt.h"

typedef int32_t int32;
typedef uint32_t uint32;

namespace legacy {
static std::vector<int32> calls;
static uint32 fail = 0;
static int32 page_w = 3, page_h = 2;
static bool image_ok = true;
static int32 rc(int32 code) {
    calls.push_back(code);
    return (fail & (1u << code)) ? 0 : 1;
}

typedef char *LPSTR;
typedef uint32 DWORD;
typedef void *HDC;
struct DOCINFO {
    int cbSize;
    const char *lpszDocName;
};
struct BITMAPFILEHEADER {};
struct BITMAPINFOHEADER {
    uint32 biSize;
    int32 biWidth, biHeight;
    uint16_t biPlanes, biBitCount;
    uint32 biCompression, biSizeImage;
    int32 biXPelsPerMeter, biYPelsPerMeter;
    uint32 biClrUsed, biClrImportant;
};
struct BITMAPINFO {};
enum { HORZRES = 8, VERTRES = 10, BI_RGB = 0, DIB_RGB_COLORS = 0 };
#define TEXT(s) s
static void ZeroMemory(void *p, size_t n) { memset(p, 0, n); }
static int GetDefaultPrinter(LPSTR name, DWORD *len) {
    calls.push_back(1);
    strcpy(name, "printer");
    return 1;
}
static HDC CreateDC(const char *, const char *, void *, void *) { return rc(2) ? (HDC)1 : nullptr; }
static int StartDoc(HDC, DOCINFO *) { return rc(3); }
static int StartPage(HDC) { return rc(4); }
static int GetDeviceCaps(HDC, int what) {
    calls.push_back(5);
    return what == HORZRES ? page_w : page_h;
}
static int SetDIBitsToDevice(HDC, int, int, uint32, uint32, int, int, uint32, uint32, const void *, BITMAPINFO *, int) {
    calls.push_back(6);
    return 1;
}
static int EndPage(HDC) { return rc(7); }
static int EndDoc(HDC) { return rc(8); }
static int DeleteDC(HDC) {
    calls.push_back(9);
    return 1;
}

// image engine
struct img_struct {
    int32 valid;
    uint32 *offset32;
};
static img_struct img[8];
static int32 page[1] = {1};
static int32 nextimg = 8;
static void validatepage(int32) {}
static void error(int32) {}
static uint32 pixels[4096];
static int32 func__newimage(int32 w, int32 h, int32, int32) {
    img[2].valid = 1;
    img[2].offset32 = pixels;
    return image_ok ? -2 : -1;
}
static void sub__dontblend(int32, int32) {}
static void sub__putimage(...) {}
static void sub__freeimage(int32, int32) {}

#define QBNex_WINDOWS
#define DEPENDENCY_PRINTER
#include "legacy/printer.inc"
#undef QBNex_WINDOWS
}  // namespace legacy

int main(int argc, char **argv) {
    bool quick = false, mutate = false;
    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "--quick")) quick = true;
        if (!strcmp(argv[i], "--mutate")) mutate = true;
    }
    (void)quick;
    long cases = 0, failures = 0;
    legacy::img[1].valid = 1;
    for (uint32 fail = 0; fail < (1u << 10); fail += 2) {  // bit 0 unused
        for (int32 h : {0, 1, 2, 7}) {
            for (int image_ok = 0; image_ok <= 1; image_ok++) {
                legacy::calls.clear();
                legacy::fail = fail;
                legacy::page_w = 5;
                legacy::page_h = h;
                legacy::image_ok = image_ok;
                legacy::sub__printimage(0);
                int32 out[256];
                int32 n = qbrt_test_printer_trace(fail, 5, h, image_ok, out, 256);
                if (mutate && n > 3) out[n - 1] = 0;
                ++cases;
                if ((int32)legacy::calls.size() != n || memcmp(legacy::calls.data(), out, n * sizeof(int32))) {
                    if (failures < 10) {
                        printf("MISMATCH fail=%#x h=%d image_ok=%d\n  legacy:", fail, h, image_ok);
                        for (int32 c : legacy::calls) printf(" %d", c);
                        printf("\n  rust:  ");
                        for (int32 i = 0; i < n; i++) printf(" %d", out[i]);
                        printf("\n");
                    }
                    ++failures;
                }
            }
        }
    }
    printf("diff_printer: %ld call sequences: %ld mismatches\n", cases, failures);
    return failures ? 1 : 0;
}
