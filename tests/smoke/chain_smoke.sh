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
# COMMON arrays: static INTEGER, TYPE and STRING * 3 arrays, then dynamic STRING and DOUBLE arrays with their bounds
for want in ' 10  20  30  5  6  abcxyz' '-1  1  one||three' ' 2  1  2  1.5 '; do
  if ! printf '%s\n' "$out" | grep -qxF -- "$want"; then
    printf '%s\n' "$out"
    echo "[FAIL] the chained program did not get the COMMON arrays"
    exit 1
  fi
done
if printf '%s\n' "$out" | grep -q 'not reached'; then
  echo "[FAIL] the program went on after CHAIN"
  exit 1
fi

# CHAIN inside a SUB: the module-level COMMON variables and arrays (not SHARED) still go, and
# the chained program replaces the first one (exec), so its exit code (SYSTEM 3) is what we see
"$QB" "$FIX/chain_sub_main.bas" -o "$TMPDIR_SMOKE/chain_sub_main" > "$TMPDIR_SMOKE/submain.log" 2>&1 \
  || { cat "$TMPDIR_SMOKE/submain.log"; echo "[FAIL] chain_sub_main.bas does not compile"; exit 1; }
"$QB" "$FIX/chain_sub_child.bas" -o "$TMPDIR_SMOKE/chain_sub_child" > "$TMPDIR_SMOKE/subchild.log" 2>&1 \
  || { cat "$TMPDIR_SMOKE/subchild.log"; echo "[FAIL] chain_sub_child.bas does not compile"; exit 1; }
code=0
(cd "$TMPDIR_SMOKE" && ./chain_sub_main < /dev/null > sub.txt 2>&1) || code=$?
sub="$(tr -d '\r' < "$TMPDIR_SMOKE/sub.txt")"
for want in '-7  2.5 text 123456789012  1.25  200 -100 ' ' 10 -20  30 ' '-1  1  one||three' ' 2  1  2  1.5 -3 '; do
  if ! printf '%s\n' "$sub" | grep -qxF -- "$want"; then
    printf '%s\n' "$sub"
    echo "[FAIL] CHAIN inside a SUB did not pass the COMMON values"
    exit 1
  fi
done
if printf '%s\n' "$sub" | grep -q 'not reached'; then
  echo "[FAIL] the program went on after CHAIN"
  exit 1
fi
if [ "$code" -ne 3 ]; then
  echo "[FAIL] exit code $code after CHAIN, expected 3 from the chained program"
  exit 1
fi
echo "CHAIN_SMOKE_OK"
rm -rf "$TMPDIR_SMOKE"
