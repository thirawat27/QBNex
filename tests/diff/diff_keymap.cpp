// Differential test: legacy GLUT_key_ascii/GLUT_key_special (tests/legacy/glut_keys.inc,
// verbatim) vs Rust qb_runtime::keymap::{glut_ascii, glut_special}. Exhaustive over every
// unsigned-char key, every GLUT modifier combination and both directions, plus
// special key codes -16..511.
//
//   g++ -O1 -std=c++17 -w tests/diff/diff_keymap.cpp <libqb_runtime.a> <native libs> -o diff_keymap
//   ./diff_keymap [--quick] [--mutate]

#include <cstdint>
#include <cstdio>
#include <cstring>

typedef int32_t int32;
typedef uint32_t uint32;

#define VK 100000
#define QBVK_RSHIFT 303
#define QBVK_LSHIFT 304
#define QBVK_RCTRL 305
#define QBVK_LCTRL 306
#define QBVK_RALT 307
#define QBVK_LALT 308
#define QBVK_BREAK 318

namespace legacy {
#define QBNex_GLUT
#define CORE_FREEGLUT
#define GLUT_ACTIVE_SHIFT 1
#define GLUT_ACTIVE_CTRL 2
#define GLUT_ACTIVE_ALT 4
#define GLUT_KEY_F1 0x0001
#define GLUT_KEY_F2 0x0002
#define GLUT_KEY_F3 0x0003
#define GLUT_KEY_F4 0x0004
#define GLUT_KEY_F5 0x0005
#define GLUT_KEY_F6 0x0006
#define GLUT_KEY_F7 0x0007
#define GLUT_KEY_F8 0x0008
#define GLUT_KEY_F9 0x0009
#define GLUT_KEY_F10 0x000A
#define GLUT_KEY_F11 0x000B
#define GLUT_KEY_F12 0x000C
#define GLUT_KEY_LEFT 0x0064
#define GLUT_KEY_UP 0x0065
#define GLUT_KEY_RIGHT 0x0066
#define GLUT_KEY_DOWN 0x0067
#define GLUT_KEY_PAGE_UP 0x0068
#define GLUT_KEY_PAGE_DOWN 0x0069
#define GLUT_KEY_HOME 0x006A
#define GLUT_KEY_END 0x006B
#define GLUT_KEY_INSERT 0x006C
static int32 mods = 0;
static int glutGetModifiers() { return mods; }
static int32 calls, last_kind;
static uint32 last_code;
static void rec(int32 kind, uint32 x) { calls++; last_kind = kind; last_code = x; }
static void keydown_vk(uint32 x) { rec(1, x); }
static void keyup_vk(uint32 x) { rec(2, x); }
static void keydown_ascii(uint32 x) { rec(1, x); }
static void keyup_ascii(uint32 x) { rec(2, x); }
#include "legacy/glut_keys.inc"
} // namespace legacy

#include "../../crates/qb-runtime/include/qb_runtime.h"

int main(int argc, char **argv) {
    bool mutate = false;
    for (int i = 1; i < argc; i++)
        if (!strcmp(argv[i], "--mutate")) mutate = true;
    long checked = 0, mismatches = 0;
    // GLUT_key_ascii: key arrives as unsigned char (0..255), modifiers 0..7
    for (int key = 0; key <= 255; key++)
        for (int m = 0; m <= 7; m++)
            for (int down = 0; down <= 1; down++) {
                legacy::mods = m;
                legacy::calls = 0;
                legacy::GLUT_key_ascii(key, down);
                uint32 code = 0;
                int32 kind = qb_keymap_glut_ascii(key, m, down, 0, &code);
                if (mutate && key == 65) code ^= 1;
                checked++;
                if (legacy::calls != 1 || kind != legacy::last_kind || code != legacy::last_code) {
                    if (mismatches < 10)
                        printf("ascii key=%d mods=%d down=%d: legacy kind=%d code=%u calls=%d, rust kind=%d code=%u\n",
                               key, m, down, legacy::last_kind, legacy::last_code, legacy::calls, kind, code);
                    mismatches++;
                }
            }
    // GLUT_key_special: any int, modifiers do not matter on the freeglut path
    for (int key = -16; key <= 511; key++)
        for (int m = 0; m <= 7; m++)
            for (int down = 0; down <= 1; down++) {
                legacy::mods = m;
                legacy::calls = 0;
                legacy::GLUT_key_special(key, down);
                uint32 code = 0;
                int32 kind = qb_keymap_glut_special(key, down, &code);
                if (mutate && key == 5) kind = 0;
                checked++;
                int32 lk = legacy::calls ? legacy::last_kind : 0;
                if (kind != lk || (lk && code != legacy::last_code)) {
                    if (mismatches < 10)
                        printf("special key=%d down=%d: legacy kind=%d code=%u, rust kind=%d code=%u\n", key, down, lk,
                               legacy::last_code, kind, code);
                    mismatches++;
                }
            }
    printf("diff_keymap: %ld cases, %ld mismatches\n", checked, mismatches);
    return mismatches ? 1 : 0;
}
