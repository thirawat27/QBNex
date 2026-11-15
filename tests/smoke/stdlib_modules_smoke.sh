#!/usr/bin/env sh
# Compiles and runs one program per stdlib module, each importing only that
# module ('$IMPORT:'<key>'), so a module that does not stand on its own fails.
set -eu
export QBNEX_NO_ERROR_DIALOG=1

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$ROOT/../.." && pwd)"
QB="$REPO_ROOT/qb"
STDLIB="$REPO_ROOT/compiler/stdlib"
[ -x "$QB" ] || { echo "[FAIL] qb not found at $QB"; exit 2; }

TMPDIR_SMOKE="$(mktemp -d "${TMPDIR:-/tmp}/qbnex_stdlib_modules.XXXXXX")"
fail=0
count=0

# 'qbnex' (qbnex_stdlib.bas) imports its modules: it must not hold a copy of
# one of their procedures again (the copies drifted apart)
procs() { grep -hiE '^[[:space:]]*(SUB|FUNCTION)[[:space:]]' "$@" | sed -E 's/^[[:space:]]*[A-Za-z]+[[:space:]]+([A-Za-z0-9_.]*).*/\1/' | tr 'a-z' 'A-Z' | sort -u; }
procs "$STDLIB"/*/*.bas > "$TMPDIR_SMOKE/module_procs.txt"
dups="$(procs "$STDLIB/qbnex_stdlib.bas" | comm -12 - "$TMPDIR_SMOKE/module_procs.txt")"
if [ -n "$dups" ]; then
  echo "[FAIL] qbnex_stdlib.bas holds copies of module procedures:" $dups
  fail=1
fi
for f in $(cd "$STDLIB" && find . -name '*.bas' | sort); do
  key="$(printf '%s' "${f#./}" | sed 's/\.bas$//; s#/#.#g')"
  [ "$key" = "qbnex_stdlib" ] && continue   # body of '$IMPORT:'qbnex' (stdlib.bas)
  count=$((count + 1))
  src="$TMPDIR_SMOKE/m$count.bas"
  printf '%s\n' '$CONSOLE:ONLY' "'\$IMPORT:'$key'" 'PRINT "ok"' 'SYSTEM' > "$src"
  if ! "$QB" "$src" -o "$TMPDIR_SMOKE/m$count" > "$TMPDIR_SMOKE/m$count.log" 2>&1 || [ ! -x "$TMPDIR_SMOKE/m$count" ]; then
    echo "[FAIL] import of $key does not compile (log: $TMPDIR_SMOKE/m$count.log)"
    fail=1
    continue
  fi
  if ! "$TMPDIR_SMOKE/m$count" < /dev/null 2>&1 | grep -qx 'ok'; then
    echo "[FAIL] program importing $key did not print ok"
    fail=1
  fi
done

if [ "$fail" = 0 ]; then
  echo "[PASS] $count stdlib modules import on their own"
  rm -rf "$TMPDIR_SMOKE"
  exit 0
fi
exit 1
