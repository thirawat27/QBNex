#!/bin/sh
# Builds and runs the legacy-vs-Rust differential tests against the libqb_runtime.a
# built by setup_build (crates/qb-runtime/os/<os>/libqb_runtime.a).
# Usage: sh tests/diff/run_diff.sh [--quick]
set -eu
cd "$(dirname "$0")"
mode="${1:-}"
case "$(uname -s)" in
  Darwin) os=macos; libs="-framework AppKit -framework CoreGraphics -framework QuartzCore -framework Foundation -framework IOKit -framework CoreFoundation -lobjc" ;;
  *) os=linux; libs="-lpthread -ldl -lm -ludev" ;;
esac
lib="../../crates/qb-runtime/os/$os/libqb_runtime.a"
out="${TMPDIR:-/tmp}/qb_diff_$$"
mkdir -p "$out"
cxx="${CXX:-c++}"

# host_stub.cpp: the host symbols libqb_runtime.a imports from libqb.cpp (diff_qbs defines its own).
for t in diff_msbin diff_str diff_mem diff_keymap diff_keybuf diff_layout diff_systime diff_net diff_screen diff_printer; do
  $cxx -O2 -std=c++17 -w "$t.cpp" host_stub.cpp "$lib" $libs -o "$out/$t"
  "$out/$t" $mode
done
$cxx -O2 -std=c++17 -w diff_qbs.cpp "$lib" $libs -o "$out/diff_qbs"
"$out/diff_qbs" $mode

# The legacy references for rounding, VAL and TIMER need x87 80-bit long double.
case "$(uname -m)" in
  x86_64|i?86)
    $cxx -O1 -std=c++17 -w diff_round.cpp host_stub.cpp "$lib" $libs -o "$out/diff_round"
    "$out/diff_round" $mode
    for t in diff_val diff_timer diff_gfx diff_audio; do
      $cxx -O1 -std=c++17 -w "$t.cpp" host_stub.cpp "$lib" $libs -o "$out/$t"
      "$out/$t" $mode
    done
    ;;
  *) echo "diff_round/diff_val/diff_timer skipped: legacy reference needs x87 long double ($(uname -m))" ;;
esac
# diff_files / diff_sys compare against the legacy Win32 code paths: Windows only (run_diff.cmd).

# R10: _DEFLATE/_INFLATE vs the system zlib they linked before (reference only; needs zlib headers).
if printf '#include <zlib.h>\nint main(){return 0;}\n' | $cxx -x c++ - -lz -o "$out/zcheck" 2>/dev/null; then
  $cxx -O2 -std=c++17 -w diff_zlib.cpp host_stub.cpp "$lib" $libs -lz -o "$out/diff_zlib"
  "$out/diff_zlib" $mode
else
  echo "diff_zlib skipped: zlib development files not installed"
fi

# R7: FreeType, stb_image and dr_pcx were deleted from the tree; the references are
# taken from git history (the last commit that still had them).
rev=4aa6f1f037277585c381fda9c956e1ddcc411033
leg="$out/legacy"
mkdir -p "$leg"
have_legacy=1
for f in image/stb_image.h image/dr_pcx.h font/ttf/src/freetypeamalgam.c font/ttf/src/freetypeamalgam.h; do
  git show "$rev:internal/c/parts/video/$f" > "$leg/$(basename "$f")" 2>/dev/null || have_legacy=0
done
if [ "$have_legacy" = 1 ]; then
  $cxx -O2 -std=c++17 -w -I"$leg" diff_image.cpp host_stub.cpp "$lib" $libs -o "$out/diff_image"
  "$out/diff_image" data/images $mode
  case "$(uname -m)" in
    x86_64|i?86)
      # the font baseline uses the legacy x87 qbr
      $cxx -O1 -c -w -x c++ "$leg/freetypeamalgam.c" -o "$out/freetype.o"
      $cxx -O2 -std=c++17 -w -I"$leg" diff_font.cpp "$out/freetype.o" host_stub.cpp "$lib" $libs -o "$out/diff_font"
      fonts="data/fonts/NotoSans-Subset.ttf data/fonts/NotoSansSymbols2-Subset.ttf data/fonts/NotoSansThai-Regular.ttf"
      for f in /usr/share/fonts/truetype/dejavu/DejaVuSans.ttf /usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf \
               /System/Library/Fonts/Supplemental/Arial.ttf; do
        [ -f "$f" ] && fonts="$fonts $f"
      done
      # shellcheck disable=SC2086
      "$out/diff_font" $mode $fonts
      ;;
    *) echo "diff_font skipped: legacy baseline needs x87 long double ($(uname -m))" ;;
  esac
else
  echo "diff_image/diff_font skipped: legacy sources not in git history (shallow clone?)"
fi
rm -rf "$out"
