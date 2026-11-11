#!/usr/bin/env sh
# Self-host fixpoint check (legacy defect L-00), QIR pipeline.
# qb (gen1) builds qb-gen2 from compiler/qbnex.bas, qb-gen2 builds qb-gen3.
# The QIR that gen2 and gen3 emit for compiler/qbnex.bas must be byte-identical, and
# identical to bootstrap/qbnex.qir. Pass --conformance to run the corpus with gen3 too.
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$ROOT/.." && pwd)"
cd "$REPO_ROOT"
WORK="target/fixpoint"

if [ ! -x ./qb ]; then
  echo "[FAIL] qb not found. Run scripts/setup_lnx.sh first."
  exit 2
fi
rm -rf "$WORK"
mkdir -p "$WORK"

echo "[fixpoint] gen1 -> qb-gen2"
./qb compiler/qbnex.bas -q -o qb-gen2 > "$WORK/gen2.log" 2>&1 || { cat "$WORK/gen2.log"; echo "[FAIL] gen1 could not build gen2"; exit 1; }
echo "[fixpoint] gen2 -> qb-gen3"
./qb-gen2 compiler/qbnex.bas -q -o qb-gen3 > "$WORK/gen3.log" 2>&1 || { cat "$WORK/gen3.log"; echo "[FAIL] gen2 could not build gen3"; exit 1; }
cp target/qb-temp/program.qir "$WORK/gen2.qir"
./qb-gen3 compiler/qbnex.bas -q -o "$WORK/qb-gen4" > "$WORK/gen4.log" 2>&1 || { cat "$WORK/gen4.log"; echo "[FAIL] gen3 could not build gen4"; exit 1; }
cp target/qb-temp/program.qir "$WORK/gen3.qir"

cmp -s "$WORK/gen2.qir" "$WORK/gen3.qir" || { echo "[FAIL] gen2 and gen3 emit different QIR for compiler/qbnex.bas"; exit 1; }
echo "[ok] gen2 and gen3 emit identical QIR"
cmp -s "$WORK/gen3.qir" bootstrap/qbnex.qir || { echo "[FAIL] the compiler's QIR differs from bootstrap/qbnex.qir (refresh it)"; exit 1; }
echo "[ok] bootstrap/qbnex.qir is up to date"

if [ "${1:-}" = "--conformance" ]; then
  echo "[fixpoint] conformance with qb-gen3"
  cargo run --release -q -p qbnex-conformance -- --qb ./qb-gen3 --work "$WORK/conf" --timeout 60
fi
rm -f qb-gen2 qb-gen3
echo "[PASS] self-host fixpoint"
