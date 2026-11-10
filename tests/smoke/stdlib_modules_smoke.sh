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
