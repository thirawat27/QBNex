#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$ROOT/.." && pwd)"
QB="$REPO_ROOT/qb"
SRC_QBNEX_REL="tests/fixtures/stdlib_import_success.bas"
SRC_URL_REL="tests/fixtures/url_import_success.bas"

if [ ! -x "$QB" ]; then
  echo "[FAIL] qb not found at \"$QB\""
  echo "Build QBNex first, then run this smoke test again."
  exit 2
fi

if [ ! -f "$REPO_ROOT/$SRC_QBNEX_REL" ]; then
  echo "[FAIL] qbnex stdlib fixture not found at \"$REPO_ROOT/$SRC_QBNEX_REL\""
  exit 2
fi

if [ ! -f "$REPO_ROOT/$SRC_URL_REL" ]; then
  echo "[FAIL] url stdlib fixture not found at \"$REPO_ROOT/$SRC_URL_REL\""
  exit 2
fi

mkdir -p "$REPO_ROOT/temp"
cd "$REPO_ROOT"

TMPDIR="$(mktemp -d "./temp/qbnex_stdlib_smoke_XXXXXX")"

BIN_QBNEX="$TMPDIR/stdlib_qbnex"
BIN_URL="$TMPDIR/stdlib_url"
OUT_QBNEX="$TMPDIR/stdlib_qbnex.txt"
OUT_URL="$TMPDIR/stdlib_url.txt"
RUN_QBNEX="$TMPDIR/stdlib_qbnex_run.txt"
RUN_URL="$TMPDIR/stdlib_url_run.txt"

"$QB" "$SRC_QBNEX_REL" -o "$BIN_QBNEX" > "$OUT_QBNEX" 2>&1
EC_QBNEX=$?

"$QB" "$SRC_URL_REL" -o "$BIN_URL" > "$OUT_URL" 2>&1
EC_URL=$?

if [ -x "$BIN_QBNEX" ]; then
  "$BIN_QBNEX" > "$RUN_QBNEX" 2>&1
  EC_RUN_QBNEX=$?
else
  EC_RUN_QBNEX=999
fi

if [ -x "$BIN_URL" ]; then
  "$BIN_URL" > "$RUN_URL" 2>&1
  EC_RUN_URL=$?
else
  EC_RUN_URL=999
fi

FAIL=0

if [ "$EC_QBNEX" -ne 0 ]; then
  echo "[FAIL] qbnex stdlib import should compile successfully."
  FAIL=1
fi
grep -F "Build complete:" "$OUT_QBNEX" >/dev/null || {
  echo "[FAIL] qbnex stdlib compile output is missing success text."
  FAIL=1
}
if [ ! -x "$BIN_QBNEX" ]; then
  echo "[FAIL] qbnex stdlib import should emit an executable."
  FAIL=1
fi
if [ "$EC_RUN_QBNEX" -ne 0 ]; then
  echo "[FAIL] qbnex stdlib executable should run successfully."
  FAIL=1
fi
grep -F "1.0.0" "$RUN_QBNEX" >/dev/null || {
  echo "[FAIL] qbnex stdlib runtime output is missing version text."
  FAIL=1
}
grep -F "QBNex" "$RUN_QBNEX" >/dev/null || {
  echo "[FAIL] qbnex stdlib runtime output is missing JSON text."
  FAIL=1
}

if [ "$EC_URL" -ne 0 ]; then
  echo "[FAIL] url stdlib import should compile successfully."
  FAIL=1
fi
grep -F "Build complete:" "$OUT_URL" >/dev/null || {
  echo "[FAIL] url stdlib compile output is missing success text."
  FAIL=1
}
if [ ! -x "$BIN_URL" ]; then
  echo "[FAIL] url stdlib import should emit an executable."
  FAIL=1
fi
if [ "$EC_RUN_URL" -ne 0 ]; then
  echo "[FAIL] url stdlib executable should run successfully."
  FAIL=1
fi
grep -F "example.com" "$RUN_URL" >/dev/null || {
  echo "[FAIL] url stdlib runtime output is missing hostname."
  FAIL=1
}
grep -F "file" "$RUN_URL" >/dev/null || {
  echo "[FAIL] url stdlib runtime output is missing basename."
  FAIL=1
}

if [ "$FAIL" -eq 0 ]; then
  echo "STDLIB_SMOKE_OK"
  rm -rf "$TMPDIR"
  exit 0
fi

echo "STDLIB_SMOKE_FAIL"
echo "Inspect outputs:"
echo "  QBNex compile: \"$OUT_QBNEX\""
echo "  QBNex run:     \"$RUN_QBNEX\""
echo "  URL compile:   \"$OUT_URL\""
echo "  URL run:       \"$RUN_URL\""
exit 1
