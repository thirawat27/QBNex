#!/bin/sh
# R14: inputs that crashed the compiler or stopped it with an internal error
# must give a clean diagnostic (exit 1, "[!!]", no "INTERNAL COMPILER ERROR");
# the valid ones must compile. Front end only (-z): no C++ compiler needed.
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
for f in deep_parens deep_blocks deep_for ifdef_unclosed endif_alone ifdef_too_deep; do
  "$qb" "$fix/$f.bas" -z -m < /dev/null > "$tmp/$f.txt" 2>&1
  ec=$?
  if [ "$ec" -ne 1 ]; then
    echo "[FAIL] $f: exit code $ec, expected 1"; failed=1
  elif grep -q "INTERNAL COMPILER ERROR" "$tmp/$f.txt"; then
    echo "[FAIL] $f: internal compiler error"; failed=1
  elif ! grep -qF "[!!]" "$tmp/$f.txt"; then
    echo "[FAIL] $f: no diagnostic"; failed=1
  fi
done
for f in ifdef_valid deep_parens_ok; do
  "$qb" "$fix/$f.bas" -z -m < /dev/null > "$tmp/$f.txt" 2>&1
  ec=$?
  if [ "$ec" -ne 0 ]; then
    echo "[FAIL] $f: exit code $ec, expected 0"; cat "$tmp/$f.txt"; failed=1
  fi
done
rm -rf "$tmp"
[ "$failed" -eq 0 ] || exit 1
echo "[PASS] compiler robustness smoke"
