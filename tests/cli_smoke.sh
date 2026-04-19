#!/usr/bin/env sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$ROOT/.." && pwd)"
QB="$REPO_ROOT/qb"
SRC_OK="$ROOT/fixtures/label_recompile_success.bas"

if [ ! -x "$QB" ]; then
  echo "[FAIL] qb not found at \"$QB\""
  echo "Build QBNex first, then run this smoke test again."
  exit 2
fi

if [ ! -f "$SRC_OK" ]; then
  echo "[FAIL] Fixture source not found at \"$SRC_OK\""
  exit 2
fi

TMPDIR="$(mktemp -d "${TMPDIR:-/tmp}/qbnex_cli_smoke_XXXXXX")"
OUT_HELP="$TMPDIR/help.txt"
OUT_VERSION="$TMPDIR/version.txt"
OUT_INVALID="$TMPDIR/invalid.txt"
OUT_BADOUT="$TMPDIR/bad_output.txt"

"$QB" --help >"$OUT_HELP" 2>&1
EC_HELP=$?

"$QB" --version >"$OUT_VERSION" 2>&1
EC_VERSION=$?

set +e
"$QB" --definitely-invalid >"$OUT_INVALID" 2>&1
EC_INVALID=$?
"$QB" "$SRC_OK" -o "$TMPDIR/missing-dir/out" >"$OUT_BADOUT" 2>&1
EC_BADOUT=$?
set -e

FAIL=0

if [ "$EC_HELP" -ne 0 ]; then
  echo "[FAIL] --help should exit successfully."
  FAIL=1
fi
grep -F "Usage: qb <file> [switches]" "$OUT_HELP" >/dev/null 2>&1 || {
  echo "[FAIL] --help output is missing usage text."
  FAIL=1
}

if [ "$EC_VERSION" -ne 0 ]; then
  echo "[FAIL] --version should exit successfully."
  FAIL=1
fi
grep -F "QBNex Compiler " "$OUT_VERSION" >/dev/null 2>&1 || {
  echo "[FAIL] --version output is missing compiler version text."
  FAIL=1
}

if [ "$EC_INVALID" -eq 0 ]; then
  echo "[FAIL] Unknown switch should fail."
  FAIL=1
fi
grep -F "Unknown switch: --definitely-invalid" "$OUT_INVALID" >/dev/null 2>&1 || {
  echo "[FAIL] Unknown switch output is missing the switch error."
  FAIL=1
}
grep -F "Run 'qb --help' for usage." "$OUT_INVALID" >/dev/null 2>&1 || {
  echo "[FAIL] Unknown switch output is missing usage guidance."
  FAIL=1
}

if [ "$EC_BADOUT" -eq 0 ]; then
  echo "[FAIL] Invalid output path should fail."
  FAIL=1
fi
grep -F "Can't create output executable - path not found:" "$OUT_BADOUT" >/dev/null 2>&1 || {
  echo "[FAIL] Invalid output path output is missing the path error."
  FAIL=1
}
if grep -F "Build complete:" "$OUT_BADOUT" >/dev/null 2>&1; then
  echo "[FAIL] Invalid output path should not report a successful build."
  FAIL=1
fi

if [ "$FAIL" -eq 0 ]; then
  echo "CLI_SMOKE_OK"
  rm -rf "$TMPDIR"
  exit 0
fi

echo "CLI_SMOKE_FAIL"
echo "Inspect outputs:"
echo "  Help: \"$OUT_HELP\""
echo "  Version: \"$OUT_VERSION\""
echo "  Invalid switch: \"$OUT_INVALID\""
echo "  Bad output path: \"$OUT_BADOUT\""
exit 1
