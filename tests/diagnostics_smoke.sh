#!/usr/bin/env bash

ROOT="$(cd -- "$(dirname -- "$0")" >/dev/null 2>&1 && pwd)"
REPO_ROOT="$(cd -- "$ROOT/.." >/dev/null 2>&1 && pwd)"
QB="$REPO_ROOT/qb"
SRC="$ROOT/fixtures/diagnostics_compile_error.bas"
SRC_INCLUDE="$ROOT/fixtures/diagnostics_include_chain_main.bas"

if [ ! -x "$QB" ]; then
  echo "[FAIL] qb not found at \"$QB\""
  echo "Build QBNex first, then run this smoke test again."
  exit 2
fi

if [ ! -f "$SRC" ]; then
  echo "[FAIL] Fixture source not found at \"$SRC\""
  exit 2
fi

if [ ! -f "$SRC_INCLUDE" ]; then
  echo "[FAIL] Include-chain fixture source not found at \"$SRC_INCLUDE\""
  exit 2
fi

TMPDIR="$(mktemp -d "${TMPDIR:-/tmp}/qbnex_diag_smoke_XXXXXX")"
if [ ! -d "$TMPDIR" ]; then
  echo "[FAIL] Could not create temp directory"
  exit 2
fi

OUT_DEFAULT="$TMPDIR/default_diagnostics.txt"
OUT_COMPACT="$TMPDIR/compact_diagnostics.txt"
OUT_INCLUDE="$TMPDIR/include_chain_diagnostics.txt"

set +e
"$QB" "$SRC" > "$OUT_DEFAULT" 2>&1
EC_DEFAULT=$?

"$QB" "$SRC" --compact-errors > "$OUT_COMPACT" 2>&1
EC_COMPACT=$?

"$QB" "$SRC_INCLUDE" > "$OUT_INCLUDE" 2>&1
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
  echo "Source fixture: \"$SRC\""
  echo "Output samples:"
  echo "  Default: \"$OUT_DEFAULT\""
  echo "  Compact: \"$OUT_COMPACT\""
  echo "  Include: \"$OUT_INCLUDE\""
  rm -rf "$TMPDIR"
  exit 0
fi

echo "DIAGNOSTICS_SMOKE_FAIL"
echo "Source fixture: \"$SRC\""
echo "Inspect outputs:"
echo "  Default: \"$OUT_DEFAULT\""
echo "  Compact: \"$OUT_COMPACT\""
echo "  Include: \"$OUT_INCLUDE\""
exit 1
