#!/bin/sh
# QBNex setup (QIR pipeline). Linux and macOS.
#
#   1. builds the QIR runner qb-exec with the Rust toolchain only, and packs it with
#      brotli behind the launcher qb-stub (qb-exec.pack, and qb-exec-lite.pack: the
#      runner without the GPU feature, for console programs without 3D); qb puts the
#      packed runner in front of every program it builds
#   2. turns the bootstrap compiler bootstrap/qbnex.qir into ./qb-stage0
#      (runner + QIR payload; QBNEX_QIR_PACK=native adds a native image
#       with `qb-exec build`)
#   3. stage0 compiles compiler/qbnex.bas into ./qb through QIR
#
# Variables: QBNEX_QB_EXEC (prebuilt runner), QBNEX_RUST_TARGET, QBNEX_BOOTSTRAP, QBNEX_QIR_PACK (ir|native),
# QBNEX_STAGE1_ARGS, QBNEX_SKIP_SELFHOST=1, QBNEX_KEEP_STAGE0=1, QBNEX_GPU=0 (runner without the GPU feature),
# QBNEX_PACK=0 (no packed runners: every program carries a copy of qb-exec).

set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$ROOT"

echo "QBNex Setup (QIR pipeline)"

case "$(uname -s)" in
  Darwin) OSDIR=macos ;;
  *) OSDIR=linux ;;
esac

BOOTSTRAP="${QBNEX_BOOTSTRAP:-bootstrap/qbnex.qir}"
PACK="${QBNEX_QIR_PACK:-ir}"

RUNNER="crates/engine/exec/bin/$OSDIR/qb-exec"
mkdir -p "crates/engine/exec/bin/$OSDIR"
# packed runners of an earlier build would not match the runner built now
rm -f "crates/engine/exec/bin/$OSDIR/qb-exec.pack" "crates/engine/exec/bin/$OSDIR/qb-exec-lite.pack"
if [ -n "${QBNEX_QB_EXEC:-}" ]; then
  cp "$QBNEX_QB_EXEC" "$RUNNER"
else
  if ! command -v cargo >/dev/null 2>&1; then
    echo "Error: cargo was not found. Install Rust from https://rustup.rs and run this script again."
    exit 1
  fi
  TARGET="${QBNEX_RUST_TARGET:-$(rustc -vV | sed -n 's/^host: //p')}"
  echo "Building the QIR runner 'qb-exec' for $TARGET"
  if command -v rustup >/dev/null 2>&1; then rustup target add "$TARGET" >/dev/null 2>&1 || true; fi
  # The runner draws 3D on the GPU (wgpu: Vulkan or Metal, loaded at run time); without a
  # usable adapter it falls back to software. QBNEX_GPU=0 builds it without.
  if [ "${QBNEX_GPU:-1}" = "0" ]; then
    cargo build --release --locked -p qb-exec --target "$TARGET" --target-dir target
  else
    cargo build --release --locked -p qb-exec --features gpu --target "$TARGET" --target-dir target
  fi
  cp "target/$TARGET/release/qb-exec" "$RUNNER"
  if [ "${QBNEX_PACK:-1}" != "0" ]; then
    echo "Packing the runner behind the launcher 'qb-stub'"
    cargo build --release --locked -p qb-stub --target "$TARGET" --target-dir target
    PACKER="target/$TARGET/release/qb-pack"
    STUB="target/$TARGET/release/qb-stub"
    "$PACKER" "$RUNNER" --stub "$STUB" -o "crates/engine/exec/bin/$OSDIR/qb-exec.pack"
    # with QBNEX_GPU=0 the runner already has no GPU feature
    if [ "${QBNEX_GPU:-1}" != "0" ]; then
      echo "Building the runner without the GPU feature for console programs"
      cargo build --release --locked -p qb-exec --target "$TARGET" --target-dir target
      "$PACKER" "target/$TARGET/release/qb-exec" --stub "$STUB" -o "crates/engine/exec/bin/$OSDIR/qb-exec-lite.pack"
    fi
  fi
fi

if [ ! -f "$BOOTSTRAP" ]; then
  echo "Error: $BOOTSTRAP is missing."
  echo "It is produced from compiler/qbnex.bas by a QIR-capable compiler (qb --ir); see CONTRIBUTING.md (bootstrap)."
  exit 1
fi

# runner + QIR + u64 LE length + "QBNEXIR0"
pack_ir() {
  cat "$1" "$2" > "$3"
  n=$(wc -c < "$2" | tr -d ' ')
  i=0
  while [ $i -lt 8 ]; do
    b=$(( (n >> (8 * i)) & 255 ))
    printf "\\$(printf '%03o' "$b")" >> "$3"
    i=$((i + 1))
  done
  printf 'QBNEXIR0' >> "$3"
  chmod +x "$3"
}

echo "Building './qb-stage0' from $BOOTSTRAP ($PACK)"
rm -f qb-stage0
if [ "$PACK" = "ir" ]; then
  pack_ir "$RUNNER" "$BOOTSTRAP" qb-stage0
else
  "./$RUNNER" build "$BOOTSTRAP" -o qb-stage0
fi
[ -x qb-stage0 ] || { echo "Error: could not build qb-stage0."; exit 1; }

if [ "${QBNEX_SKIP_SELFHOST:-}" = "1" ]; then
  echo "qb-stage0 is ready (self-host skipped)."
  exit 0
fi

echo "Self-hosting 'QBNex' through QIR"
rm -f qb
# shellcheck disable=SC2086
./qb-stage0 compiler/qbnex.bas -o qb ${QBNEX_STAGE1_ARGS:-}
[ -x qb ] || { echo "Final compiler build failed."; exit 1; }

echo "Smoke test"
SMOKE="$(mktemp -d)"
echo 'PRINT "QBNex QIR OK"' > "$SMOKE/smoke.bas"
./qb "$SMOKE/smoke.bas" -o "$SMOKE/smoke"
"$SMOKE/smoke" | grep -q "QBNex QIR OK" || { echo "Error: the smoke test failed."; exit 1; }
rm -rf "$SMOKE"

[ "${QBNEX_KEEP_STAGE0:-}" = "1" ] || rm -f qb-stage0
# Product-name front end: ./qbnex runs the compiler like ./qb, plus the
# developer tools (qbnex fmt / lsp / dap; see scripts/qbnex-launcher.sh).
rm -f ./qbnex
if cp scripts/qbnex-launcher.sh ./qbnex 2>/dev/null; then chmod +x ./qbnex; else ln -s qb ./qbnex 2>/dev/null || cp ./qb ./qbnex; fi
echo "QBNex CLI compiler is ready"
echo "  ./qb yourfile.bas"
echo "  ./qbnex yourfile.bas"
