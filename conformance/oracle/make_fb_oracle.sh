#!/usr/bin/env bash
# Adds the framebuffer capture hook (fb_capture_legacy.inc) to a CLONE of the
# legacy tree so it can bless framebuffer golden files (*.fb).
#
#   git clone --branch main <repo> .oracle-fb      # unmodified legacy commit 545b859
#   conformance/oracle/make_fb_oracle.sh .oracle-fb
#   (cd .oracle-fb && cmake --preset windows-mingw && cmake --build --preset windows-mingw-selfhost)
#   cargo run --release -p qbnex-conformance -- --qb .oracle-fb/qb.exe --bless --filter graphics/
#
# Only the hook is added; drawing code stays legacy. Never run this on D:\QBNex
# or on the shared .oracle checkout.
set -euo pipefail
dir="${1:?usage: make_fb_oracle.sh <legacy clone>}"
lib="$dir/internal/c/libqb.cpp"
here="$(cd "$(dirname "$0")" && pwd)"
if grep -q "qbnex_fb_capture" "$lib"; then
    echo "already patched: $lib"
    exit 0
fi
cp "$here/fb_capture_legacy.inc" "$dir/internal/c/fb_capture_legacy.inc"
# end(): capture the final frame; sub__display(): capture every _DISPLAY.
sed -i \
    -e 's/^void end(){/#include "fb_capture_legacy.inc"\nvoid end(){ qbnex_fb_capture(1);/' \
    -e 's/^\( *\)void sub__display(){/\1void sub__display(){ qbnex_fb_capture(0);/' \
    "$lib"
grep -c "qbnex_fb_capture(" "$lib" | grep -qx 2 || { echo "patch did not apply cleanly" >&2; exit 1; }
echo "patched $lib"
