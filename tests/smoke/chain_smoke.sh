#!/usr/bin/env sh
# CHAIN runs another program and passes it the COMMON variables by position.
set -eu
export QBNEX_NO_ERROR_DIALOG=1

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$ROOT/../.." && pwd)"
QB="$REPO_ROOT/qb"
FIX="$ROOT/../fixtures"
[ -x "$QB" ] || { echo "[FAIL] qb not found at $QB"; exit 2; }

TMPDIR_SMOKE="$(mktemp -d "${TMPDIR:-/tmp}/qbnex_chain.XXXXXX")"
"$QB" "$FIX/chain_common_main.bas" -o "$TMPDIR_SMOKE/chain_common_main" > "$TMPDIR_SMOKE/main.log" 2>&1 \
  || { cat "$TMPDIR_SMOKE/main.log"; echo "[FAIL] chain_common_main.bas does not compile"; exit 1; }
"$QB" "$FIX/chain_common_child.bas" -o "$TMPDIR_SMOKE/chain_common_child" > "$TMPDIR_SMOKE/child.log" 2>&1 \
  || { cat "$TMPDIR_SMOKE/child.log"; echo "[FAIL] chain_common_child.bas does not compile"; exit 1; }

out="$(cd "$TMPDIR_SMOKE" && ./chain_common_main < /dev/null 2>&1 | tr -d '\r')"
if ! printf '%s\n' "$out" | grep -qx ' 7  2.5 text abcd 3 -4 '; then
  printf '%s\n' "$out"
  echo "[FAIL] the chained program did not get the COMMON values"
  exit 1
fi
if printf '%s\n' "$out" | grep -q 'not reached'; then
  echo "[FAIL] the program went on after CHAIN"
  exit 1
fi
echo "CHAIN_SMOKE_OK"
rm -rf "$TMPDIR_SMOKE"
