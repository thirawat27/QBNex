#!/bin/sh
# Inputs that crashed the compiler or stopped it with an internal error
# must give a clean diagnostic (exit 1, the expected message, a line number,
# no "INTERNAL COMPILER ERROR"); the valid ones must compile. Their
# executables go to the temp folder (-z no longer stops before the build).
set -u
root=$(cd "$(dirname "$0")/../.." && pwd)
qb="$root/qbnex"
[ -x "$qb" ] || qb="$root/qb"
fix="$root/tests/fixtures/robustness"
if [ ! -x "$qb" ]; then
  echo "[FAIL] compiler not found at $root/qbnex"
  exit 2
fi
tmp=$(mktemp -d)
failed=0
bad() {
  "$qb" "$fix/$1.bas" -z -m -o "$tmp/$1" < /dev/null > "$tmp/$1.txt" 2>&1
  ec=$?
  if [ "$ec" -ne 1 ]; then
    echo "[FAIL] $1: exit code $ec, expected 1"; failed=1; return
  fi
  grep -q "INTERNAL COMPILER ERROR" "$tmp/$1.txt" && { echo "[FAIL] $1: internal compiler error"; failed=1; }
  grep -qF "$2" "$tmp/$1.txt" || { echo "[FAIL] $1: message \"$2\" missing"; failed=1; }
  grep -qF "$3" "$tmp/$1.txt" || { echo "[FAIL] $1: location \"$3\" missing"; failed=1; }
}
good() {
  "$qb" "$fix/$1.bas" -z -m -o "$tmp/$1" < /dev/null > "$tmp/$1.txt" 2>&1
  ec=$?
  if [ "$ec" -ne 0 ]; then
    echo "[FAIL] $1: exit code $ec, expected 0"; cat "$tmp/$1.txt"; failed=1
  fi
}
bad deep_parens 'Expression is nested too deeply' 'deep_parens.bas(2)'
bad deep_blocks 'Too many nested blocks' 'deep_blocks.bas(1002)'
bad deep_for 'Too many nested blocks' 'deep_for.bas(1002)'
bad ifdef_unclosed '$IF without $END IF' 'ifdef_unclosed.bas(2)'
bad endif_alone '$END IF without $IF' 'endif_alone.bas(2)'
bad ifdef_too_deep 'Too many nested $IF blocks' 'ifdef_too_deep.bas(257)'
good ifdef_valid
good deep_parens_ok
good long_expr
rm -rf "$tmp"
[ "$failed" -eq 0 ] || exit 1
echo "[PASS] compiler robustness smoke"
