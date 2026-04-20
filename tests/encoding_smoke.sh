#!/usr/bin/env bash

ROOT="$(cd -- "$(dirname -- "$0")" >/dev/null 2>&1 && pwd)"
REPO_ROOT="$(cd -- "$ROOT/.." >/dev/null 2>&1 && pwd)"
QB="$REPO_ROOT/qb"

if [ ! -x "$QB" ]; then
  echo "[FAIL] qb not found at \"$QB\""
  echo "Build QBNex first, then run this smoke test again."
  exit 2
fi

TMPDIR="$(mktemp -d "${TMPDIR:-/tmp}/qbnex_encoding_smoke_XXXXXX")"
if [ ! -d "$TMPDIR" ]; then
  echo "[FAIL] Could not create temp directory"
  exit 2
fi

SRC_UTF16="$TMPDIR/utf16_source.bas"
SRC_INVALID="$TMPDIR/invalid_utf8_source.bas"
SRC_INVALID_MID="$TMPDIR/invalid_utf8_mid_source.bas"
SRC_BOM="$TMPDIR/utf8_bom_source.bas"
SRC_EMPTY="$TMPDIR/empty_source.bas"
SRC_SPACE_DIR="$TMPDIR/source with spaces"
SRC_SPACE="$SRC_SPACE_DIR/hello world.bas"
SRC_QUOTE_DIR="$TMPDIR/source's apostrophe"
SRC_QUOTE="$SRC_QUOTE_DIR/hello's world.bas"
BIN_UTF16="$TMPDIR/utf16_output"
BIN_INVALID="$TMPDIR/invalid_utf8_output"
BIN_INVALID_MID="$TMPDIR/invalid_utf8_mid_output"
BIN_BOM="$TMPDIR/utf8_bom_output"
BIN_EMPTY="$TMPDIR/empty_output"
BIN_SPACE="$TMPDIR/output with spaces/hello world"
BIN_QUOTE="$TMPDIR/output's apostrophe/hello's world"
BIN_STALE="$TMPDIR/stale_output"
OUT_UTF16="$TMPDIR/utf16_output.txt"
OUT_INVALID="$TMPDIR/invalid_utf8_output.txt"
OUT_INVALID_MID="$TMPDIR/invalid_utf8_mid_output.txt"
OUT_BOM="$TMPDIR/utf8_bom_output.txt"
OUT_EMPTY="$TMPDIR/empty_output.txt"
OUT_SPACE="$TMPDIR/space_path_output.txt"
OUT_QUOTE="$TMPDIR/apostrophe_path_output.txt"
OUT_STALE="$TMPDIR/stale_output.txt"

printf '\377\376P\000R\000I\000N\000T\000 \000"\000h\000i\000"\000\r\000\n\000' > "$SRC_UTF16"
printf 'PRINT "\377"\r\n' > "$SRC_INVALID"
printf 'PRINT "ok"\r\nPRINT "\377"\r\n' > "$SRC_INVALID_MID"
printf '\357\273\277PRINT "hi"\r\n' > "$SRC_BOM"
: > "$SRC_EMPTY"
mkdir -p "$SRC_SPACE_DIR" "$TMPDIR/output with spaces"
printf 'PRINT "hi"\r\n' > "$SRC_SPACE"
mkdir -p "$SRC_QUOTE_DIR" "$TMPDIR/output's apostrophe"
printf 'PRINT "hi"\r\n' > "$SRC_QUOTE"

set +e
"$QB" "$SRC_UTF16" -o "$BIN_UTF16" > "$OUT_UTF16" 2>&1
EC_UTF16=$?

"$QB" "$SRC_INVALID" -o "$BIN_INVALID" > "$OUT_INVALID" 2>&1
EC_INVALID=$?

"$QB" "$SRC_INVALID_MID" -o "$BIN_INVALID_MID" > "$OUT_INVALID_MID" 2>&1
EC_INVALID_MID=$?

"$QB" "$SRC_BOM" -o "$BIN_BOM" > "$OUT_BOM" 2>&1
EC_BOM=$?

"$QB" "$SRC_EMPTY" -o "$BIN_EMPTY" > "$OUT_EMPTY" 2>&1
EC_EMPTY=$?

"$QB" "$SRC_SPACE" -o "$BIN_SPACE" > "$OUT_SPACE" 2>&1
EC_SPACE=$?

"$QB" "$SRC_QUOTE" -o "$BIN_QUOTE" > "$OUT_QUOTE" 2>&1
EC_QUOTE=$?

"$QB" "$SRC_BOM" -o "$BIN_STALE" >/dev/null 2>&1
"$QB" "$SRC_INVALID" -o "$BIN_STALE" > "$OUT_STALE" 2>&1
EC_STALE=$?
set -e

FAIL=0

if [ "$EC_UTF16" -eq 0 ]; then
  echo "[FAIL] UTF-16 source should fail compilation."
  FAIL=1
fi
grep -F "UTF-16 LE encoding detected" "$OUT_UTF16" >/dev/null 2>&1 || {
  echo "[FAIL] UTF-16 diagnostics are missing the encoding error."
  FAIL=1
}
grep -F "Build complete:" "$OUT_UTF16" >/dev/null 2>&1 && {
  echo "[FAIL] UTF-16 diagnostics should not report a successful build."
  FAIL=1
}
if [ -f "$BIN_UTF16" ]; then
  echo "[FAIL] UTF-16 compilation should not emit an executable."
  FAIL=1
fi

if [ "$EC_INVALID" -eq 0 ]; then
  echo "[FAIL] Invalid UTF-8 source should fail compilation."
  FAIL=1
fi
grep -F "Invalid UTF-8 byte sequence detected in source file" "$OUT_INVALID" >/dev/null 2>&1 || {
  echo "[FAIL] Invalid UTF-8 diagnostics are missing the fatal error."
  FAIL=1
}
grep -F "Build complete:" "$OUT_INVALID" >/dev/null 2>&1 && {
  echo "[FAIL] Invalid UTF-8 diagnostics should not report a successful build."
  FAIL=1
}
if [ -f "$BIN_INVALID" ]; then
  echo "[FAIL] Invalid UTF-8 compilation should not emit an executable."
  FAIL=1
fi

if [ "$EC_INVALID_MID" -eq 0 ]; then
  echo "[FAIL] Mid-file invalid UTF-8 source should fail compilation."
  FAIL=1
fi
grep -F "Invalid UTF-8 byte sequence detected in source file" "$OUT_INVALID_MID" >/dev/null 2>&1 || {
  echo "[FAIL] Mid-file invalid UTF-8 diagnostics are missing the fatal error."
  FAIL=1
}
grep -F "Build complete:" "$OUT_INVALID_MID" >/dev/null 2>&1 && {
  echo "[FAIL] Mid-file invalid UTF-8 diagnostics should not report a successful build."
  FAIL=1
}
if [ -f "$BIN_INVALID_MID" ]; then
  echo "[FAIL] Mid-file invalid UTF-8 compilation should not emit an executable."
  FAIL=1
fi

if [ "$EC_BOM" -ne 0 ]; then
  echo "[FAIL] UTF-8 BOM source should compile successfully."
  FAIL=1
fi
grep -F "Build complete:" "$OUT_BOM" >/dev/null 2>&1 || {
  echo "[FAIL] UTF-8 BOM fixture is missing successful build output."
  FAIL=1
}
if grep -F "encoding detected" "$OUT_BOM" >/dev/null 2>&1; then
  echo "[FAIL] UTF-8 BOM fixture should not report an encoding failure."
  FAIL=1
fi
if [ ! -f "$BIN_BOM" ]; then
  echo "[FAIL] UTF-8 BOM compilation should emit an executable."
  FAIL=1
fi

if [ "$EC_EMPTY" -ne 0 ]; then
  echo "[FAIL] Empty source should compile successfully."
  FAIL=1
fi
grep -F "Build complete:" "$OUT_EMPTY" >/dev/null 2>&1 || {
  echo "[FAIL] Empty-source fixture is missing successful build output."
  FAIL=1
}
if [ ! -f "$BIN_EMPTY" ]; then
  echo "[FAIL] Empty-source compilation should emit an executable."
  FAIL=1
fi

if [ "$EC_SPACE" -ne 0 ]; then
  echo "[FAIL] Source and output paths with spaces should compile successfully."
  FAIL=1
fi
grep -F "Build complete:" "$OUT_SPACE" >/dev/null 2>&1 || {
  echo "[FAIL] Spaced-path fixture is missing successful build output."
  FAIL=1
}
if [ ! -f "$BIN_SPACE" ]; then
  echo "[FAIL] Spaced-path compilation should emit an executable."
  FAIL=1
fi

if [ "$EC_QUOTE" -ne 0 ]; then
  echo "[FAIL] Source and output paths with apostrophes should compile successfully."
  FAIL=1
fi
grep -F "Build complete:" "$OUT_QUOTE" >/dev/null 2>&1 || {
  echo "[FAIL] Apostrophe-path fixture is missing successful build output."
  FAIL=1
}
if [ ! -f "$BIN_QUOTE" ]; then
  echo "[FAIL] Apostrophe-path compilation should emit an executable."
  FAIL=1
fi

if [ "$EC_STALE" -eq 0 ]; then
  echo "[FAIL] Recompiling invalid UTF-8 over an existing output should fail."
  FAIL=1
fi
grep -F "Warning: Existing output was not updated because compilation failed." "$OUT_STALE" >/dev/null 2>&1 || {
  echo "[FAIL] Stale-output run is missing the stale executable warning."
  FAIL=1
}
if [ ! -f "$BIN_STALE" ]; then
  echo "[FAIL] Stale-output run should keep the existing executable in place."
  FAIL=1
fi

if [ "$FAIL" -eq 0 ]; then
  echo "ENCODING_SMOKE_OK"
  rm -rf "$TMPDIR"
  exit 0
fi

echo "ENCODING_SMOKE_FAIL"
echo "Inspect outputs:"
echo "  UTF-16: \"$OUT_UTF16\""
echo "  Invalid UTF-8: \"$OUT_INVALID\""
echo "  Mid invalid UTF-8: \"$OUT_INVALID_MID\""
echo "  UTF-8 BOM: \"$OUT_BOM\""
echo "  Empty: \"$OUT_EMPTY\""
echo "  Spaced path: \"$OUT_SPACE\""
echo "  Apostrophe path: \"$OUT_QUOTE\""
echo "  Stale output: \"$OUT_STALE\""
exit 1
