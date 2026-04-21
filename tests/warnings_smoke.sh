#!/usr/bin/env sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$ROOT/.." && pwd)"
QB="$REPO_ROOT/qb"
SRC_REL="tests/fixtures/unused_variable_warning.bas"

if [ ! -x "$QB" ]; then
  echo "[FAIL] qb not found at \"$QB\""
  echo "Build QBNex first, then run this smoke test again."
  exit 2
fi

if [ ! -f "$REPO_ROOT/$SRC_REL" ]; then
  echo "[FAIL] Warning fixture source not found at \"$REPO_ROOT/$SRC_REL\""
  exit 2
fi

mkdir -p "$REPO_ROOT/temp"
cd "$REPO_ROOT"

TMPDIR="$(mktemp -d "./temp/qbnex_warnings_smoke_XXXXXX")"
BIN_WARN="$TMPDIR/warn"
BIN_WERROR="$TMPDIR/werror"
OUT_WARN="$TMPDIR/warn.txt"
OUT_WERROR="$TMPDIR/werror.txt"

"$QB" "$SRC_REL" -w -o "$BIN_WARN" >"$OUT_WARN" 2>&1
EC_WARN=$?

set +e
"$QB" "$SRC_REL" --warnings-as-errors -o "$BIN_WERROR" >"$OUT_WERROR" 2>&1
EC_WERROR=$?
set -e

FAIL=0

if [ "$EC_WARN" -ne 0 ]; then
  echo "[FAIL] Warning fixture with -w should compile successfully."
  FAIL=1
fi
grep -F "warning: " "$OUT_WARN" >/dev/null 2>&1 || {
  echo "[FAIL] Warning fixture with -w is missing warning output."
  FAIL=1
}
grep -F "unused variable" "$OUT_WARN" >/dev/null 2>&1 || {
  echo "[FAIL] Warning fixture with -w is missing the warning header."
  FAIL=1
}
grep -F "Build complete:" "$OUT_WARN" >/dev/null 2>&1 || {
  echo "[FAIL] Warning fixture with -w is missing successful build output."
  FAIL=1
}
if [ ! -f "$BIN_WARN" ]; then
  echo "[FAIL] Warning fixture with -w should emit an executable."
  FAIL=1
fi

if [ "$EC_WERROR" -eq 0 ]; then
  echo "[FAIL] Warning fixture with --warnings-as-errors should fail compilation."
  FAIL=1
fi
grep -F "warning promoted to blocking diagnostic" "$OUT_WERROR" >/dev/null 2>&1 || {
  echo "[FAIL] Warnings-as-errors output is missing the promotion note."
  FAIL=1
}
grep -F "unused variable" "$OUT_WERROR" >/dev/null 2>&1 || {
  echo "[FAIL] Warnings-as-errors output is missing the warning header."
  FAIL=1
}
grep -F "[x] QBNex :: Error [W" "$OUT_WERROR" >/dev/null 2>&1 || {
  echo "[FAIL] Warnings-as-errors output is missing structured diagnostic headline."
  FAIL=1
}
grep -F "  [@] " "$OUT_WERROR" >/dev/null 2>&1 || {
  echo "[FAIL] Warnings-as-errors output is missing location marker [@]."
  FAIL=1
}
grep -F "  [#] source" "$OUT_WERROR" >/dev/null 2>&1 || {
  echo "[FAIL] Warnings-as-errors output is missing source marker [#]."
  FAIL=1
}
grep -F "  [>] next" "$OUT_WERROR" >/dev/null 2>&1 || {
  echo "[FAIL] Warnings-as-errors output is missing suggestion marker [>]."
  FAIL=1
}
grep -F "  [::] flow" "$OUT_WERROR" >/dev/null 2>&1 || {
  echo "[FAIL] Warnings-as-errors output is missing flow marker [::]."
  FAIL=1
}
grep -F "  [!] cause" "$OUT_WERROR" >/dev/null 2>&1 || {
  echo "[FAIL] Warnings-as-errors output is missing cause marker [!]."
  FAIL=1
}
grep -F "  [+] example" "$OUT_WERROR" >/dev/null 2>&1 || {
  echo "[FAIL] Warnings-as-errors output is missing example marker [+]."
  FAIL=1
}
grep -F "[x] QBNex :: Build Halted" "$OUT_WERROR" >/dev/null 2>&1 || {
  echo "[FAIL] Warnings-as-errors output is missing build halt summary."
  FAIL=1
}
if grep -F "Build complete:" "$OUT_WERROR" >/dev/null 2>&1; then
  echo "[FAIL] Warnings-as-errors run should not report a successful build."
  FAIL=1
fi

if [ "$FAIL" -eq 0 ]; then
  echo "WARNINGS_SMOKE_OK"
  rm -rf "$TMPDIR"
  exit 0
fi

echo "WARNINGS_SMOKE_FAIL"
echo "Inspect outputs:"
echo "  Warn:   \"$OUT_WARN\""
echo "  Werror: \"$OUT_WERROR\""
exit 1
