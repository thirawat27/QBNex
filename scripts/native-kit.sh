#!/usr/bin/env sh
# Builds the QBNex native kit on Linux or macOS: libqb_native.a
# (crate qb-native), rust-lld from the active Rust toolchain and kit.txt with
# the linker command (ld.lld / ld64.lld flavor). Result: $1 (default
# target/release/native). Use it with
#   QBNEX_NATIVE_KIT=<dir>   or copy <dir> to native/ next to qb-exec.
# Needs: Linux - the system library development files (libc6-dev) and the
# libraries the runtime links (libasound, libudev); macOS - the Xcode command
# line tools (SDK).
set -eu
out="${1:-target/release/native}"
: "${CARGO_BUILD_JOBS:=1}"
export CARGO_BUILD_JOBS
# GPU 3D in native programs as in the runner; QBNEX_GPU=0 leaves it out
features="--features qb-exec/gpu"
[ "${QBNEX_GPU:-1}" = "0" ] && features=""
# shellcheck disable=SC2086
cargo build --release -p qb-exec $features
# Build the runtime library and capture the system libraries it needs.
# --color never: with CARGO_TERM_COLOR=always (CI) the note ends in an ANSI reset
# code that would stick to the last library (-lc becomes "-lc\033[0m").
libs=$(cargo rustc --color never --release -p qb-native $features -- --print native-static-libs 2>&1 \
    | sed -n 's/.*native-static-libs: //p' | tail -n 1)
[ -n "$libs" ] || { echo "native-kit: rustc printed no native-static-libs" >&2; exit 1; }
sysroot=$(rustc --print sysroot)
host=$(rustc -vV | sed -n 's/^host: //p')
lld="$sysroot/lib/rustlib/$host/bin/rust-lld"
[ -x "$lld" ] || { echo "native-kit: rust-lld not found: $lld" >&2; exit 1; }
mkdir -p "$out"
exec target/release/qb-exec native-kit --staticlib target/release/libqb_native.a --lld "$lld" --libs "$libs" -o "$out"
