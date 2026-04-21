#!/usr/bin/env bash

ROOT="$(cd -- "$(dirname -- "$0")" >/dev/null 2>&1 && pwd)"
REPO_ROOT="$(cd -- "$ROOT/.." >/dev/null 2>&1 && pwd)"
QB="$REPO_ROOT/qb"
SRC_REL="tests/fixtures/diagnostics_compile_error.bas"
SRC_INCLUDE_REL="tests/fixtures/diagnostics_include_chain_main.bas"

if [ ! -x "$QB" ]; then
  echo "[FAIL] qb not found at \"$QB\""
  echo "Build QBNex first, then run this smoke test again."
  exit 2
fi

if [ ! -f "$REPO_ROOT/$SRC_REL" ]; then
  echo "[FAIL] Fixture source not found at \"$REPO_ROOT/$SRC_REL\""
  exit 2
fi

if [ ! -f "$REPO_ROOT/$SRC_INCLUDE_REL" ]; then
  echo "[FAIL] Include-chain fixture source not found at \"$REPO_ROOT/$SRC_INCLUDE_REL\""
  exit 2
fi

mkdir -p "$REPO_ROOT/temp"
pushd "$REPO_ROOT" >/dev/null

TMPDIR="$(mktemp -d "./temp/qbnex_diag_smoke_XXXXXX")"
if [ ! -d "$TMPDIR" ]; then
  echo "[FAIL] Could not create temp directory"
  popd >/dev/null
  exit 2
fi

OUT_DEFAULT="$TMPDIR/default_diagnostics.txt"
OUT_COMPACT="$TMPDIR/compact_diagnostics.txt"
OUT_INCLUDE="$TMPDIR/include_chain_diagnostics.txt"

set +e
"$QB" "$SRC_REL" > "$OUT_DEFAULT" 2>&1
EC_DEFAULT=$?

"$QB" "$SRC_REL" --compact-errors > "$OUT_COMPACT" 2>&1
EC_COMPACT=$?

"$QB" "$SRC_INCLUDE_REL" > "$OUT_INCLUDE" 2>&1
EC_INCLUDE=$?
set -e

FAIL=0

if [ "$EC_DEFAULT" -eq 0 ]; then
  echo "[FAIL] Default diagnostics run should fail for invalid source."
  FAIL=1
fi

grep -F "[!] cause" "$OUT_DEFAULT" >/dev/null 2>&1 || {
  echo "[FAIL] Default diagnostics output is missing [!] cause."
  FAIL=1
}
grep -F "[+] example" "$OUT_DEFAULT" >/dev/null 2>&1 || {
  echo "[FAIL] Default diagnostics output is missing [+] example."
  FAIL=1
}
grep -F "[::] flow" "$OUT_DEFAULT" >/dev/null 2>&1 || {
  echo "[FAIL] Default diagnostics output is missing [::] flow."
  FAIL=1
}

if [ "$EC_COMPACT" -eq 0 ]; then
  echo "[FAIL] Compact diagnostics run should fail for invalid source."
  FAIL=1
fi

grep -F "[!] cause" "$OUT_COMPACT" >/dev/null 2>&1 && {
  echo "[FAIL] Compact diagnostics output should not include [!] cause."
  FAIL=1
}
grep -F "[+] example" "$OUT_COMPACT" >/dev/null 2>&1 && {
  echo "[FAIL] Compact diagnostics output should not include [+] example."
  FAIL=1
}

if [ "$EC_INCLUDE" -eq 0 ]; then
  echo "[FAIL] Include-chain diagnostics run should fail for missing nested include."
  FAIL=1
fi
grep -F "diagnostics_include_chain_missing.bas" "$OUT_INCLUDE" >/dev/null 2>&1 || {
  echo "[FAIL] Include-chain diagnostics are missing the nested include filename."
  FAIL=1
}
grep -F "diagnostics_include_chain_mid.bas" "$OUT_INCLUDE" >/dev/null 2>&1 || {
  echo "[FAIL] Include-chain diagnostics are missing the parent include filename."
  FAIL=1
}
grep -F "Triggered while compiling an included module." "$OUT_INCLUDE" >/dev/null 2>&1 || {
  echo "[FAIL] Include-chain diagnostics are missing the location note."
  FAIL=1
}
COUNT_INCLUDE="$(grep -F "File diagnostics_include_chain_missing.bas not found" "$OUT_INCLUDE" | wc -l | tr -d ' ')"
if [ "$COUNT_INCLUDE" -ne 1 ]; then
  echo "[FAIL] Include-chain diagnostics should render the blocking error once."
  FAIL=1
fi

if [ "$FAIL" -eq 0 ]; then
  echo "DIAGNOSTICS_SMOKE_OK"
  echo "Source fixture: \"$SRC_REL\""
  echo "Output samples:"
  echo "  Default: \"$OUT_DEFAULT\""
  echo "  Compact: \"$OUT_COMPACT\""
  echo "  Include: \"$OUT_INCLUDE\""
  rm -rf "$TMPDIR"
  popd >/dev/null
  exit 0
fi

echo "DIAGNOSTICS_SMOKE_FAIL"
echo "Source fixture: \"$SRC_REL\""
echo "Inspect outputs:"
echo "  Default: \"$OUT_DEFAULT\""
echo "  Compact: \"$OUT_COMPACT\""
echo "  Include: \"$OUT_INCLUDE\""
popd >/dev/null
exit 1
