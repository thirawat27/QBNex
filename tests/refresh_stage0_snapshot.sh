#!/usr/bin/env sh
# Regenerates the stage0 bootstrap snapshot internal/source from source/qbnex.bas.
# Only the generated C++ fragments (*.txt) and DATA (data.bin) are replaced; the
# hand-kept helper files (icon.rc, *.bat, manifest.h, manifests, icons, *.bin) stay
# unchanged because the generated versions contain machine-specific paths.
# Run tests/selfhost_fixpoint.sh first: the snapshot must come from a fixpoint compiler.
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
cd "$ROOT/.."
if [ ! -x ./qb ]; then
  echo "[FAIL] qb not found. Build QBNex first."
  exit 2
fi

rm -rf internal/temp
mkdir -p internal/temp
if ! ./qb source/qbnex.bas -q -z > temp-refresh.log 2>&1; then
  cat temp-refresh.log; rm -f temp-refresh.log
  echo "[FAIL] qb -z failed"
  exit 1
fi
rm -f temp-refresh.log

rm -f internal/source/*.txt
cp internal/temp/*.txt internal/source/
cp internal/temp/data.bin internal/source/data.bin
echo "[ok] internal/source refreshed from source/qbnex.bas"
