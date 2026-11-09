#!/usr/bin/env sh
# Self-host fixpoint check (legacy defect L-00).
# qb (gen1) builds qb-gen2 from source/qbnex.bas, qb-gen2 builds qb-gen3.
# gen2 and gen3 must emit byte-identical C++ for source/qbnex.bas, and both must pass
# the conformance corpus. Pass --no-conformance to skip the corpus runs.
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$ROOT/.." && pwd)"
cd "$REPO_ROOT"
WORK="temp/fixpoint"

if [ ! -x ./qb ]; then
  echo "[FAIL] qb not found. Build QBNex first."
  exit 2
fi
rm -rf "$WORK"
mkdir -p "$WORK"

echo "[fixpoint] gen1 -> qb-gen2"
if ! ./qb source/qbnex.bas -q -o qb-gen2 > "$WORK/gen2.log" 2>&1; then
  cat "$WORK/gen2.log"; echo "[FAIL] gen1 could not build gen2"; exit 1
fi
echo "[fixpoint] gen2 -> qb-gen3"
if ! ./qb-gen2 source/qbnex.bas -q -o qb-gen3 > "$WORK/gen3.log" 2>&1; then
  cat "$WORK/gen3.log"; echo "[FAIL] gen2 could not build gen3"; exit 1
fi

for g in gen2 gen3; do
  if ! "./qb-$g" source/qbnex.bas -q -z > "$WORK/$g-z.log" 2>&1; then
    cat "$WORK/$g-z.log"; echo "[FAIL] $g -z failed"; exit 1
  fi
  cp -R internal/temp "$WORK/$g-c"
done
if ! git diff --no-index --quiet "$WORK/gen2-c" "$WORK/gen3-c"; then
  git diff --no-index --stat "$WORK/gen2-c" "$WORK/gen3-c" || true
  echo "[FAIL] gen2 and gen3 emit different C++ for source/qbnex.bas"
  exit 1
fi
echo "[ok] gen2 and gen3 emit identical C++"

if [ "${1:-}" != "--no-conformance" ]; then
  for g in gen2 gen3; do
    echo "[fixpoint] conformance with qb-$g"
    cargo run --release -q -p qbnex-conformance -- --qb "qb-$g" --work "$WORK/conf-$g" --timeout 60
  done
fi

rm -f qb-gen2 qb-gen3
echo "[PASS] self-host fixpoint"
