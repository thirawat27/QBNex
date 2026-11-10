#!/bin/sh
# QBNex setup without a C/C++ toolchain (cutover to the QIR pipeline, D10/D15).
# Linux and macOS.
#
#   1. builds the QIR runner qb-exec with the Rust toolchain only
#   2. turns the bootstrap compiler bootstrap/qbnex.qir into ./qb-stage0
#      (`qb-exec build`: runner + program + native image, linked by qb-link;
#       QBNEX_QIR_PACK=ir packs the QIR alone instead)
#   3. stage0 compiles compiler/qbnex.bas into ./qb through QIR
#
# DISABLED until the unified QIR pipeline is in rust-port: run with
#   QBNEX_QIR_CUTOVER=1 scripts/setup_qir.sh   or   scripts/setup_qir.sh --enable
# scripts/setup_lnx.sh / setup_osx.command stay the default until then.
#
# Variables: QBNEX_QB_EXEC (prebuilt runner), QBNEX_RUST_TARGET, QBNEX_BOOTSTRAP, QBNEX_QIR_PACK (native|ir),
# QBNEX_STAGE1_ARGS, QBNEX_SKIP_SELFHOST=1, QBNEX_KEEP_STAGE0=1.

set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$ROOT"

if [ "${QBNEX_QIR_CUTOVER:-}" != "1" ] && [ "${1:-}" != "--enable" ]; then
  echo "The QIR setup is disabled until the unified QIR pipeline lands in rust-port."
  echo "Use scripts/setup_lnx.sh or scripts/setup_osx.command, or enable this one with QBNEX_QIR_CUTOVER=1 / --enable."
  exit 3
fi

echo "QBNex Setup (QIR pipeline, no C/C++ toolchain)"

case "$(uname -s)" in
  Darwin) OSDIR=macos ;;
  *) OSDIR=linux ;;
esac

BOOTSTRAP="${QBNEX_BOOTSTRAP:-bootstrap/qbnex.qir}"
PACK="${QBNEX_QIR_PACK:-native}"

RUNNER="crates/qb-exec/bin/$OSDIR/qb-exec"
mkdir -p "crates/qb-exec/bin/$OSDIR"
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
  cargo build --release --locked -p qb-exec --target "$TARGET" --target-dir target
  cp "target/$TARGET/release/qb-exec" "$RUNNER"
fi

if [ ! -f "$BOOTSTRAP" ]; then
  echo "Error: $BOOTSTRAP is missing."
  echo "It is produced from compiler/qbnex.bas by a QIR-capable compiler (qb --ir); see docs/rust-port/cutover-plan.md."
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
echo "QBNex CLI compiler is ready (QIR pipeline):"
echo "  qb yourfile.bas"
echo "  qbnex yourfile.bas"
