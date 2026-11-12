#!/usr/bin/env sh
set -eu
# Runtime errors go to stderr (exit code 1) instead of a modal dialog.
export QBNEX_NO_ERROR_DIALOG=1

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$ROOT/../.." && pwd)"
QB="$REPO_ROOT/qb"
SRC_OK_REL="tests/fixtures/label_recompile_success.bas"
SRC_CONSOLE_REL="tests/fixtures/cli_console_output.bas"
BIN_XMODE="$REPO_ROOT/cli_console_output"
BIN_ZMODE="$REPO_ROOT/label_recompile_success"

if [ ! -x "$QB" ]; then
  echo "[FAIL] qb not found at \"$QB\""
  echo "Build QBNex first, then run this smoke test again."
  exit 2
fi

if [ ! -f "$REPO_ROOT/$SRC_OK_REL" ]; then
  echo "[FAIL] Fixture source not found at \"$REPO_ROOT/$SRC_OK_REL\""
  exit 2
fi

if [ ! -f "$REPO_ROOT/$SRC_CONSOLE_REL" ]; then
  echo "[FAIL] Console fixture source not found at \"$REPO_ROOT/$SRC_CONSOLE_REL\""
  exit 2
fi

mkdir -p "$REPO_ROOT/temp"
cd "$REPO_ROOT"

SMOKE_TMP="$(mktemp -d "./temp/qbnex_cli_smoke_XXXXXX")"
OUT_HELP="$SMOKE_TMP/help.txt"
OUT_VERSION="$SMOKE_TMP/version.txt"
OUT_INVALID="$SMOKE_TMP/invalid.txt"
OUT_BADOUT="$SMOKE_TMP/bad_output.txt"
OUT_QUIET="$SMOKE_TMP/quiet.txt"
OUT_SETTINGS="$SMOKE_TMP/settings.txt"
OUT_ZMODE="$SMOKE_TMP/zmode.txt"
OUT_XMODE="$SMOKE_TMP/xmode.txt"
OUT_EXTERNAL="$SMOKE_TMP/external_cwd.txt"
OUT_CACHE_A="$SMOKE_TMP/cache_a.txt"
OUT_CACHE_B="$SMOKE_TMP/cache_b.txt"
OUT_CACHE_A_STRINGS="$SMOKE_TMP/cache_a_strings.txt"
OUT_CACHE_B_STRINGS="$SMOKE_TMP/cache_b_strings.txt"
BIN_EXTERNAL="$SMOKE_TMP/external_cwd_output"
EXTERNAL_CWD="$SMOKE_TMP/external-workdir"
SRC_OK_FROM_EXTERNAL="../../../$SRC_OK_REL"
BIN_EXTERNAL_FROM_EXTERNAL="../external_cwd_output"
SRC_CACHE_A="$SMOKE_TMP/cache_a.bas"
SRC_CACHE_B="$SMOKE_TMP/cache_b.bas"
BIN_CACHE_A="$SMOKE_TMP/cache_a_output"
BIN_CACHE_B="$SMOKE_TMP/cache_b_output"

rm -f "$BIN_XMODE" "$BIN_ZMODE"
mkdir -p "$EXTERNAL_CWD"

"$QB" --help >"$OUT_HELP" 2>&1
EC_HELP=$?

"$QB" --version >"$OUT_VERSION" 2>&1
EC_VERSION=$?

set +e
"$QB" --definitely-invalid >"$OUT_INVALID" 2>&1
EC_INVALID=$?
"$QB" "$SRC_OK_REL" -o "$SMOKE_TMP/missing-dir/out" >"$OUT_BADOUT" 2>&1
EC_BADOUT=$?
"$QB" "$SRC_OK_REL" -q -o "$SMOKE_TMP/quiet" >"$OUT_QUIET" 2>&1
EC_QUIET=$?
"$QB" -s >"$OUT_SETTINGS" 2>&1
EC_SETTINGS=$?
"$QB" "$SRC_OK_REL" -z >"$OUT_ZMODE" 2>&1
EC_ZMODE=$?
"$QB" "$SRC_CONSOLE_REL" -x >"$OUT_XMODE" 2>&1
EC_XMODE=$?

rm -f "$BIN_EXTERNAL"
(
  cd "$EXTERNAL_CWD" &&
  "$QB" "$SRC_OK_FROM_EXTERNAL" -o "$BIN_EXTERNAL_FROM_EXTERNAL"
) >"$OUT_EXTERNAL" 2>&1
EC_EXTERNAL=$?

printf '%s\n' 'PRINT "CACHE_A_UNIQUE"' >"$SRC_CACHE_A"
printf '%s\n' 'PRINT "CACHE_B_UNIQUE"' >"$SRC_CACHE_B"
"$QB" "$SRC_CACHE_A" -q -o "$BIN_CACHE_A" >"$OUT_CACHE_A" 2>&1
EC_CACHE_A=$?
"$QB" "$SRC_CACHE_B" -q -o "$BIN_CACHE_B" >"$OUT_CACHE_B" 2>&1
EC_CACHE_B=$?

# each binary must print its own program's text (the QIR payload stores strings
# hex-encoded, so the output is checked rather than the bytes of the file)
: >"$OUT_CACHE_A_STRINGS"
: >"$OUT_CACHE_B_STRINGS"
[ -f "$BIN_CACHE_A" ] && "$BIN_CACHE_A" </dev/null >"$OUT_CACHE_A_STRINGS" 2>&1
[ -f "$BIN_CACHE_B" ] && "$BIN_CACHE_B" </dev/null >"$OUT_CACHE_B_STRINGS" 2>&1
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

if [ "$EC_QUIET" -ne 0 ]; then
  echo "[FAIL] Quiet compile should exit successfully."
  FAIL=1
fi
if grep -F "Compiling program..." "$OUT_QUIET" >/dev/null 2>&1; then
  echo "[FAIL] Quiet compile should suppress progress banner."
  FAIL=1
fi
if [ ! -f "$SMOKE_TMP/quiet" ]; then
  echo "[FAIL] Quiet compile should still emit an executable."
  FAIL=1
fi

if [ "$EC_SETTINGS" -ne 0 ]; then
  echo "[FAIL] -s should exit successfully."
  FAIL=1
fi
grep -F "debuginfo" "$OUT_SETTINGS" >/dev/null 2>&1 || {
  echo "[FAIL] -s output is missing debuginfo setting."
  FAIL=1
}
grep -F "exewithsource" "$OUT_SETTINGS" >/dev/null 2>&1 || {
  echo "[FAIL] -s output is missing exewithsource setting."
  FAIL=1
}

if [ "$EC_ZMODE" -ne 0 ]; then
  echo "[FAIL] -z should exit successfully."
  FAIL=1
fi
if grep -F "Compiling program..." "$OUT_ZMODE" >/dev/null 2>&1; then
  echo "[FAIL] -z should not invoke native executable build output."
  FAIL=1
fi

if [ "$EC_XMODE" -ne 0 ]; then
  echo "[FAIL] -x should exit successfully."
  FAIL=1
fi
grep -F "Build complete:" "$OUT_XMODE" >/dev/null 2>&1 || {
  echo "[FAIL] -x output is missing successful build output."
  FAIL=1
}
if [ ! -f "$BIN_XMODE" ]; then
  echo "[FAIL] -x should emit the default output executable."
  FAIL=1
fi

if [ "$EC_EXTERNAL" -ne 0 ]; then
  echo "[FAIL] Running qb from outside the repo root should still compile successfully."
  FAIL=1
fi
grep -F "Build complete:" "$OUT_EXTERNAL" >/dev/null 2>&1 || {
  echo "[FAIL] External-cwd compile is missing successful build output."
  FAIL=1
}
if [ ! -f "$BIN_EXTERNAL" ]; then
  echo "[FAIL] External-cwd compile should emit the requested executable."
  FAIL=1
fi


if [ "$EC_CACHE_A" -ne 0 ]; then
  echo "[FAIL] Cache regression compile A should exit successfully."
  FAIL=1
fi
if [ "$EC_CACHE_B" -ne 0 ]; then
  echo "[FAIL] Cache regression compile B should exit successfully."
  FAIL=1
fi
if [ ! -f "$BIN_CACHE_A" ]; then
  echo "[FAIL] Cache regression compile A should emit an executable."
  FAIL=1
fi
if [ ! -f "$BIN_CACHE_B" ]; then
  echo "[FAIL] Cache regression compile B should emit an executable."
  FAIL=1
fi
grep -F "CACHE_A_UNIQUE" "$OUT_CACHE_A_STRINGS" >/dev/null 2>&1 || {
  echo "[FAIL] Cache regression binary A is missing its unique string."
  FAIL=1
}
if grep -F "CACHE_B_UNIQUE" "$OUT_CACHE_A_STRINGS" >/dev/null 2>&1; then
  echo "[FAIL] Cache regression binary A contains binary B's string."
  FAIL=1
fi
grep -F "CACHE_B_UNIQUE" "$OUT_CACHE_B_STRINGS" >/dev/null 2>&1 || {
  echo "[FAIL] Cache regression binary B is missing its unique string."
  FAIL=1
}
if grep -F "CACHE_A_UNIQUE" "$OUT_CACHE_B_STRINGS" >/dev/null 2>&1; then
  echo "[FAIL] Cache regression binary B contains binary A's string."
  FAIL=1
fi

if [ "$FAIL" -eq 0 ]; then
  echo "CLI_SMOKE_OK"
  rm -f "$BIN_XMODE" "$BIN_ZMODE"
  rm -rf "$SMOKE_TMP"
  exit 0
fi

echo "CLI_SMOKE_FAIL"
echo "Inspect outputs:"
echo "  Help: \"$OUT_HELP\""
echo "  Version: \"$OUT_VERSION\""
echo "  Invalid switch: \"$OUT_INVALID\""
echo "  Bad output path: \"$OUT_BADOUT\""
echo "  Quiet: \"$OUT_QUIET\""
echo "  Settings: \"$OUT_SETTINGS\""
echo "  Z mode: \"$OUT_ZMODE\""
echo "  X mode: \"$OUT_XMODE\""
echo "  External cwd: \"$OUT_EXTERNAL\""
echo "  Cache A: \"$OUT_CACHE_A\""
echo "  Cache B: \"$OUT_CACHE_B\""
exit 1
