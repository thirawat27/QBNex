#!/bin/sh
# Builds and runs the legacy-vs-Rust differential tests against the libqbrt.a
# built by setup_build (internal/c/qbrt/os/<os>/libqbrt.a).
# Usage: sh internal/c/qbrt/tests/run_diff.sh [--quick]
set -eu
cd "$(dirname "$0")"
mode="${1:-}"
case "$(uname -s)" in
  Darwin) os=osx; libs="" ;;
  *) os=lnx; libs="-lpthread -ldl -lm" ;;
esac
lib="../os/$os/libqbrt.a"
out="${TMPDIR:-/tmp}/qbrt_diff_$$"
mkdir -p "$out"
cxx="${CXX:-c++}"

for t in diff_msbin diff_str diff_qbs; do
  $cxx -O2 -std=c++17 -w "$t.cpp" "$lib" $libs -o "$out/$t"
  "$out/$t" $mode
done

# The legacy references for rounding, VAL and TIMER need x87 80-bit long double.
case "$(uname -m)" in
  x86_64|i?86)
    $cxx -O1 -std=c++17 -w diff_round.cpp "$lib" $libs -o "$out/diff_round"
    "$out/diff_round" $mode
    for t in diff_val diff_timer; do
      $cxx -O1 -std=c++17 -w "$t.cpp" "$lib" $libs -o "$out/$t"
      "$out/$t" $mode
    done
    ;;
  *) echo "diff_round/diff_val/diff_timer skipped: legacy reference needs x87 long double ($(uname -m))" ;;
esac
rm -rf "$out"
