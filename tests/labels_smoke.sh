#!/usr/bin/env sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$ROOT/.." && pwd)"
QB="$REPO_ROOT/qb"
SRC_OK_REL="tests/fixtures/label_recompile_success.bas"
SRC_FAIL_REL="tests/fixtures/label_missing_failure.bas"
SRC_SCOPE_REL="tests/fixtures/label_scope_conflict.bas"
SRC_DATA_REL="tests/fixtures/label_ambiguous_data.bas"

if [ ! -x "$QB" ]; then
  echo "[FAIL] qb not found at \"$QB\""
  echo "Build QBNex first, then run this smoke test again."
  exit 2
fi

if [ ! -f "$REPO_ROOT/$SRC_OK_REL" ]; then
  echo "[FAIL] Success fixture source not found at \"$REPO_ROOT/$SRC_OK_REL\""
  exit 2
fi

if [ ! -f "$REPO_ROOT/$SRC_FAIL_REL" ]; then
  echo "[FAIL] Failure fixture source not found at \"$REPO_ROOT/$SRC_FAIL_REL\""
  exit 2
fi

if [ ! -f "$REPO_ROOT/$SRC_SCOPE_REL" ]; then
  echo "[FAIL] Scope-conflict fixture source not found at \"$REPO_ROOT/$SRC_SCOPE_REL\""
  exit 2
fi

if [ ! -f "$REPO_ROOT/$SRC_DATA_REL" ]; then
  echo "[FAIL] Ambiguous-data fixture source not found at \"$REPO_ROOT/$SRC_DATA_REL\""
  exit 2
fi

mkdir -p "$REPO_ROOT/temp"
cd "$REPO_ROOT"

TMPDIR="$(mktemp -d "./temp/qbnex_labels_smoke_XXXXXX")"
BIN_OK="$TMPDIR/label_recompile_success"
BIN_FAIL="$TMPDIR/label_missing_failure"
BIN_SCOPE="$TMPDIR/label_scope_conflict"
BIN_DATA="$TMPDIR/label_ambiguous_data"
BIN_STALE="$TMPDIR/label_stale_failure"
OUT_OK="$TMPDIR/label_recompile_success.txt"
OUT_FAIL="$TMPDIR/label_missing_failure.txt"
OUT_SCOPE="$TMPDIR/label_scope_conflict.txt"
OUT_DATA="$TMPDIR/label_ambiguous_data.txt"
OUT_STALE="$TMPDIR/label_stale_failure.txt"

"$QB" "$SRC_OK_REL" -o "$BIN_OK" >"$OUT_OK" 2>&1
EC_OK=$?

set +e
"$QB" "$SRC_FAIL_REL" -o "$BIN_FAIL" >"$OUT_FAIL" 2>&1
EC_FAIL=$?
"$QB" "$SRC_SCOPE_REL" -o "$BIN_SCOPE" >"$OUT_SCOPE" 2>&1
EC_SCOPE=$?
"$QB" "$SRC_DATA_REL" -o "$BIN_DATA" >"$OUT_DATA" 2>&1
EC_DATA=$?
"$QB" "$SRC_OK_REL" -o "$BIN_STALE" >/dev/null 2>&1
"$QB" "$SRC_FAIL_REL" -o "$BIN_STALE" >"$OUT_STALE" 2>&1
EC_STALE=$?
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
grep -F "Build Halted" "$OUT_FAIL" >/dev/null 2>&1 || {
  echo "[FAIL] Missing-label fixture is missing blocking diagnostic output."
  FAIL=1
}
if ! grep -F "Label 'MissingLabel' not defined" "$OUT_FAIL" >/dev/null 2>&1 && ! grep -F "Unknown statement" "$OUT_FAIL" >/dev/null 2>&1; then
  echo "[FAIL] Missing-label fixture should report a blocking label-path failure."
  FAIL=1
fi
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
grep -F "Build Halted" "$OUT_SCOPE" >/dev/null 2>&1 || {
  echo "[FAIL] Scope-conflict fixture is missing blocking diagnostic output."
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
grep -F "Build Halted" "$OUT_DATA" >/dev/null 2>&1 || {
  echo "[FAIL] Ambiguous-data fixture is missing blocking diagnostic output."
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

if [ "$EC_STALE" -eq 0 ]; then
  echo "[FAIL] Missing-label stale-output fixture should fail compilation."
  FAIL=1
fi
grep -F "Warning: Existing output was not updated because compilation failed." "$OUT_STALE" >/dev/null 2>&1 || {
  echo "[FAIL] Missing-label stale-output fixture is missing the stale executable warning."
  FAIL=1
}
if [ ! -f "$BIN_STALE" ]; then
  echo "[FAIL] Missing-label stale-output fixture should keep the previous executable."
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
echo "  Stale:   \"$OUT_STALE\""
exit 1
