#!/usr/bin/env sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$ROOT/.." && pwd)"
QB="$REPO_ROOT/qb"
SRC_OK="$ROOT/fixtures/label_recompile_success.bas"
SRC_FAIL="$ROOT/fixtures/label_missing_failure.bas"
SRC_SCOPE="$ROOT/fixtures/label_scope_conflict.bas"
SRC_DATA="$ROOT/fixtures/label_ambiguous_data.bas"

if [ ! -x "$QB" ]; then
  echo "[FAIL] qb not found at \"$QB\""
  echo "Build QBNex first, then run this smoke test again."
  exit 2
fi

if [ ! -f "$SRC_OK" ]; then
  echo "[FAIL] Success fixture source not found at \"$SRC_OK\""
  exit 2
fi

if [ ! -f "$SRC_FAIL" ]; then
  echo "[FAIL] Failure fixture source not found at \"$SRC_FAIL\""
  exit 2
fi

if [ ! -f "$SRC_SCOPE" ]; then
  echo "[FAIL] Scope-conflict fixture source not found at \"$SRC_SCOPE\""
  exit 2
fi

if [ ! -f "$SRC_DATA" ]; then
  echo "[FAIL] Ambiguous-data fixture source not found at \"$SRC_DATA\""
  exit 2
fi

TMPDIR="$(mktemp -d "${TMPDIR:-/tmp}/qbnex_labels_smoke_XXXXXX")"
BIN_OK="$TMPDIR/label_recompile_success"
BIN_FAIL="$TMPDIR/label_missing_failure"
BIN_SCOPE="$TMPDIR/label_scope_conflict"
BIN_DATA="$TMPDIR/label_ambiguous_data"
OUT_OK="$TMPDIR/label_recompile_success.txt"
OUT_FAIL="$TMPDIR/label_missing_failure.txt"
OUT_SCOPE="$TMPDIR/label_scope_conflict.txt"
OUT_DATA="$TMPDIR/label_ambiguous_data.txt"

"$QB" "$SRC_OK" -o "$BIN_OK" >"$OUT_OK" 2>&1
EC_OK=$?

set +e
"$QB" "$SRC_FAIL" -o "$BIN_FAIL" >"$OUT_FAIL" 2>&1
EC_FAIL=$?
"$QB" "$SRC_SCOPE" -o "$BIN_SCOPE" >"$OUT_SCOPE" 2>&1
EC_SCOPE=$?
"$QB" "$SRC_DATA" -o "$BIN_DATA" >"$OUT_DATA" 2>&1
EC_DATA=$?
set -e

FAIL=0

if [ "$EC_OK" -ne 0 ]; then
  echo "[FAIL] Label recompile fixture should compile successfully."
  FAIL=1
fi
grep -F "Build complete:" "$OUT_OK" >/dev/null 2>&1 || {
  echo "[FAIL] Label recompile fixture is missing successful build output."
  FAIL=1
}
if grep -F "not defined" "$OUT_OK" >/dev/null 2>&1; then
  echo "[FAIL] Label recompile fixture should not report undefined labels."
  FAIL=1
fi
if [ ! -f "$BIN_OK" ]; then
  echo "[FAIL] Label recompile fixture should emit an executable."
  FAIL=1
fi

if [ "$EC_FAIL" -eq 0 ]; then
  echo "[FAIL] Missing-label fixture should fail compilation."
  FAIL=1
fi
grep -F "Label 'MissingLabel' not defined" "$OUT_FAIL" >/dev/null 2>&1 || {
  echo "[FAIL] Missing-label fixture is missing the unresolved label detail."
  FAIL=1
}
if grep -F "Build complete:" "$OUT_FAIL" >/dev/null 2>&1; then
  echo "[FAIL] Missing-label fixture should not report a successful build."
  FAIL=1
fi
if [ -f "$BIN_FAIL" ]; then
  echo "[FAIL] Missing-label fixture should not emit an executable."
  FAIL=1
fi

if [ "$EC_SCOPE" -eq 0 ]; then
  echo "[FAIL] Scope-conflict fixture should fail compilation."
  FAIL=1
fi
grep -F "Common label within a SUB/FUNCTION" "$OUT_SCOPE" >/dev/null 2>&1 || {
  echo "[FAIL] Scope-conflict fixture is missing the scope-conflict detail."
  FAIL=1
}
if grep -F "Build complete:" "$OUT_SCOPE" >/dev/null 2>&1; then
  echo "[FAIL] Scope-conflict fixture should not report a successful build."
  FAIL=1
fi
if [ -f "$BIN_SCOPE" ]; then
  echo "[FAIL] Scope-conflict fixture should not emit an executable."
  FAIL=1
fi

if [ "$EC_DATA" -eq 0 ]; then
  echo "[FAIL] Ambiguous-data fixture should fail compilation."
  FAIL=1
fi
grep -F "Ambiguous DATA label" "$OUT_DATA" >/dev/null 2>&1 || {
  echo "[FAIL] Ambiguous-data fixture is missing the DATA-label ambiguity detail."
  FAIL=1
}
if grep -F "Build complete:" "$OUT_DATA" >/dev/null 2>&1; then
  echo "[FAIL] Ambiguous-data fixture should not report a successful build."
  FAIL=1
fi
if [ -f "$BIN_DATA" ]; then
  echo "[FAIL] Ambiguous-data fixture should not emit an executable."
  FAIL=1
fi

if [ "$FAIL" -eq 0 ]; then
  echo "LABELS_SMOKE_OK"
  rm -rf "$TMPDIR"
  exit 0
fi

echo "LABELS_SMOKE_FAIL"
echo "Inspect outputs:"
echo "  Success: \"$OUT_OK\""
echo "  Failure: \"$OUT_FAIL\""
echo "  Scope:   \"$OUT_SCOPE\""
echo "  Data:    \"$OUT_DATA\""
exit 1
