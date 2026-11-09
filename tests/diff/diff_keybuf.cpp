// Differential test: legacy keyheld()/keyheld_add/remove/unbind, update_shift_state()
// and the _KEYHIT ring (tests/legacy/keyheld.inc + keyhit.inc, verbatim) vs Rust
// qb_runtime::keybuf, driven by long random operation sequences.
//
//   g++ -O1 -std=c++17 -w tests/diff/diff_keybuf.cpp <libqb_runtime.a> <native libs> -o diff_keybuf
//   ./diff_keybuf [--quick] [--mutate]

#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <vector>

typedef int32_t int32;
typedef uint32_t uint32;
typedef int64_t int64;
typedef uint8_t uint8;

namespace legacy {
#define QBK 200000
#define VK 100000
#define QBK_SCROLL_LOCK_MODE 11
#define QBK_INSERT_MODE 12
enum {
    QBVK_PAUSE = 19, QBVK_KP0 = 256, QBVK_KP1, QBVK_KP2, QBVK_KP3, QBVK_KP4, QBVK_KP5, QBVK_KP6, QBVK_KP7, QBVK_KP8,
    QBVK_KP9, QBVK_KP_PERIOD = 266, QBVK_KP_DIVIDE = 267, QBVK_KP_MULTIPLY = 268, QBVK_KP_MINUS = 269,
    QBVK_KP_PLUS = 270, QBVK_KP_ENTER = 271, QBVK_NUMLOCK = 300, QBVK_CAPSLOCK = 301, QBVK_SCROLLOCK = 302,
    QBVK_RSHIFT = 303, QBVK_LSHIFT = 304, QBVK_RCTRL = 305, QBVK_LCTRL = 306, QBVK_RALT = 307, QBVK_LALT = 308,
    QBVK_SYSREQ = 317
};
static uint32 bindkey = 0;
static uint8 cmem[0x500];
static uint32 unbound_key;
static int32 unbound_calls;
static void keyup(uint32 x) { unbound_key = x; unbound_calls++; }
#include "legacy/keyheld.inc"
#include "legacy/keyhit.inc"
} // namespace legacy

#include "../../crates/qb-runtime/include/qb_runtime.h"

static uint64_t rng = 0x9E3779B97F4A7C15ull;
static uint32 rnd() {
    rng ^= rng << 13;
    rng ^= rng >> 7;
    rng ^= rng << 17;
    return (uint32)rng;
}

static uint32 pick_key() {
    switch (rnd() % 6) {
    case 0: return rnd() % 300;                       // ascii and friends
    case 1: return VK + 250 + rnd() % 75;             // numpad, locks, modifiers
    case 2: return QBK + rnd() % 14;                  // numpad-without-numlock, lock modes
    case 3: return (0x47 + rnd() % 13) << 8;          // BIOS extended codes 0x4700..0x5300
    case 4: return 1000000 + (rnd() % 64) * 7919;     // large values (bounded set keeps the held list small)
    default: return 13 + (rnd() % 2) * 34;            // enter / '/'
    }
}

int main(int argc, char **argv) {
    long iters = 5000000;
    bool mutate = false;
    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "--quick")) iters = 300000;
        if (!strcmp(argv[i], "--mutate")) mutate = true;
    }
    long mismatches = 0, checked = 0;
    for (long it = 0; it < iters; it++) {
        uint32 op = rnd() % 100;
        uint32 x = pick_key();
        if (op < 30) {
            uint32 bind = (rnd() % 4 == 0) ? (250 + rnd() % 75) : 0;
            legacy::bindkey = bind;
            legacy::keyheld_add(x);
            qb_keyheld_add(x, bind);
        } else if (op < 55) {
            legacy::keyheld_remove(x);
            qb_keyheld_remove(x);
        } else if (op < 60) {
            uint32 b = 250 + rnd() % 75;
            legacy::unbound_calls = 0;
            legacy::keyheld_unbind(b);
            uint32 held = 0;
            int32 found = qb_keyheld_bound_to(b, &held);
            checked++;
            if (found != legacy::unbound_calls || (found && held != legacy::unbound_key)) {
                if (mismatches < 10) printf("unbind %u: legacy %d/%u rust %d/%u\n", b, legacy::unbound_calls, legacy::unbound_key, found, held);
                mismatches++;
            }
        } else if (op < 80) {
            checked++;
            int32 l = legacy::keyheld(x);
            int32 r = qb_keyheld(x);
            if (mutate && x == 13) r ^= 1;
            if (l != r) {
                if (mismatches < 10) printf("keyheld(%u): legacy %d rust %d\n", x, l, r);
                mismatches++;
            }
        } else if (op < 85) {
            checked++;
            legacy::update_shift_state();
            uint8 b[3];
            qb_shift_state(b);
            if (b[0] != legacy::cmem[0x417] || b[1] != legacy::cmem[0x418] || b[2] != legacy::cmem[0x496]) {
                if (mismatches < 10) printf("shift state: legacy %02x %02x %02x rust %02x %02x %02x\n", legacy::cmem[0x417], legacy::cmem[0x418], legacy::cmem[0x496], b[0], b[1], b[2]);
                mismatches++;
            }
        } else if (op < 95) {
            // bursts make the 8191-message ring overflow
            int n = (rnd() % 64 == 0) ? 9000 : 1 + rnd() % 4;
            for (int k = 0; k < n; k++) {
                uint32 v = pick_key();
                int64 numpad = (rnd() & 1) ? 4294967296ll : 0;
                legacy::legacy_keyhit_push(v, numpad);
                qb_keyhit_push((int64)(v | numpad));
            }
        } else {
            int n = 1 + rnd() % 8;
            for (int k = 0; k < n; k++) {
                checked++;
                int32 l = legacy::func__keyhit();
                int32 r = qb_keyhit_pop();
                if (l != r) {
                    if (mismatches < 10) printf("keyhit: legacy %d rust %d\n", l, r);
                    mismatches++;
                }
            }
        }
    }
    printf("diff_keybuf: %ld iterations, %ld checks, %ld mismatches\n", iters, checked, mismatches);
    return mismatches ? 1 : 0;
}
