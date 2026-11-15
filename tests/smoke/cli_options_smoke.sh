#!/usr/bin/env sh
set -eu
# Every command and option of `qb --help` does what its help line says, and each
# one prints something of its own: a switch that is accepted but changes nothing
# is caught here. (-m: output to a file is uncoloured anyway, so only its exit
# code is checked; the colours themselves need a terminal.)
export QBNEX_NO_ERROR_DIALOG=1
unset NO_COLOR || true

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$ROOT/../.." && pwd)"
QB="$REPO_ROOT/qb"

if [ ! -x "$QB" ]; then
  echo "[FAIL] qb not found at \"$QB\""
  echo "Build QBNex first, then run this smoke test again."
  exit 2
fi

mkdir -p "$REPO_ROOT/temp"
T="$(mktemp -d "$REPO_ROOT/temp/qbnex_cli_options_XXXXXX")"
printf 'PRINT "CLEAN_OK"\n' > "$T/clean.bas"
printf 'DIM unusedVar AS INTEGER\nPRINT "WARN_OK"\n' > "$T/warn.bas"
printf 'PRINT "before"\nx = (1 + 2\nPRINT "after the error"\n' > "$T/err.bas"
printf 'total = 5\nPRINT total\n' > "$T/undecl.bas"

FAIL=0

# run name expected-exit args... -> $T/name.txt; expected exit 1 means "not 0"
run() {
  name="$1"; want="$2"; shift 2
  set +e
  "$QB" "$@" > "$T/$name.txt" 2>&1
  ec=$?
  set -e
  if [ "$want" = 0 ] && [ "$ec" != 0 ]; then
    echo "[FAIL] qb $* exited with $ec, expected 0"; FAIL=1
  fi
  if [ "$want" != 0 ] && [ "$ec" = 0 ]; then
    echo "[FAIL] qb $* exited with 0, expected a failure"; FAIL=1
  fi
}
has() {
  grep -F -q -- "$2" "$T/$1.txt" || { echo "[FAIL] $1: missing \"$2\""; FAIL=1; }
}
hasnot() {
  if grep -F -q -- "$2" "$T/$1.txt"; then echo "[FAIL] $1: should not contain \"$2\""; FAIL=1; fi
}
exists() {
  [ -f "$1" ] || { echo "[FAIL] expected file \"$1\""; FAIL=1; }
}
missing() {
  if [ -f "$1" ]; then echo "[FAIL] unexpected file \"$1\""; FAIL=1; fi
}

# ---- commands: four different screens ----
run help 0 -h
has help "Usage: qb <file> [switches]"
run version 0 --version
has version "QBNex Compiler "
hasnot version "Usage:"
hasnot version "Repository:"
run info 0 -i
has info "Settings:"
has info "Repository:"
hasnot info "Usage:"
run examples 0 -g
has examples "Examples:"
has examples "qb -s:exewithsource=true"
hasnot examples "Usage:"

# ---- building: <file>, -c, -o, -x, -p, -z build as usual ----
run clean 0 "$T/clean.bas" -o "$T/clean"
has clean "Build complete:"
exists "$T/clean"
for s in c x p z; do
  run "clean_$s" 0 "$T/clean.bas" "-$s" -o "$T/clean_$s"
  has "clean_$s" "Build complete:"
  exists "$T/clean_$s"
done
"$T/clean" < /dev/null > "$T/clean_run.txt" 2>&1 || true
has clean_run "CLEAN_OK"

# ---- -q: no banner or progress ----
run quiet 0 "$T/clean.bas" -q -o "$T/quiet"
hasnot quiet "Build complete"
hasnot quiet "QBNex Compiler"
exists "$T/quiet"

# ---- -m ----
run mono 0 "$T/clean.bas" -m -o "$T/mono"
exists "$T/mono"

# ---- warnings: counted, -w shows them, -Werror blocks ----
run warn 0 "$T/warn.bas" -o "$T/warn"
has warn "warning(s) not shown; add -w"
hasnot warn "warning: unused variable"
run warn_w 0 "$T/warn.bas" -w -o "$T/warn_w"
has warn_w "warning: unused variable"
has warn_w "warning(s) shown above"
run clean_w 0 "$T/clean.bas" -w -o "$T/clean_w"
has clean_w "No warnings."
run werror 1 "$T/warn.bas" -Werror -o "$T/werror"
has werror "warning promoted to blocking diagnostic"
missing "$T/werror"
run werror_long 1 "$T/warn.bas" --warnings-as-errors -k -o "$T/werror"
hasnot werror_long "[!] cause"

# ---- diagnostics: default, -d (lines around), -k (compact) ----
run err 1 "$T/err.bas" -o "$T/err"
has err "[!] cause"
hasnot err "after the error"
run err_d 1 "$T/err.bas" -d -o "$T/err"
has err_d "[!] cause"
has err_d "after the error"
run err_k 1 "$T/err.bas" --compact-errors -o "$T/err"
has err_k "Missing )"
hasnot err_k "[!] cause"

# ---- -e: OPTION _EXPLICIT ----
run undecl 0 "$T/undecl.bas" -o "$T/undecl"
run undecl_e 1 "$T/undecl.bas" -e -o "$T/undecl_e"
has undecl_e "'total' is not defined"
missing "$T/undecl_e"

# ---- --emit-ir and -o name.qir: QIR text, no executable ----
run emit 0 "$T/clean.bas" --emit-ir -o "$T/emit.exe"
exists "$T/emit.qir"
missing "$T/emit.exe"
run qir 0 "$T/clean.bas" -o "$T/named.qir"
exists "$T/named.qir"

# ---- -s: list, read one, write one (debuginfo has no effect), reject others ----
run settings 0 -s
has settings "Settings file:"
has settings "ignorewarnings"
run settings_one 0 -s:exewithsource
has settings_one "exewithsource = "
run settings_set 0 -s:debuginfo=false
has settings_set "debuginfo = false"
hasnot settings_set "Usage:"
run settings_bad 1 -s:no-such-setting
has settings_bad "Invalid settings switch"

if [ "$FAIL" = 0 ]; then
  echo "CLI_OPTIONS_SMOKE_OK"
  rm -rf "$T"
  exit 0
fi
echo "CLI_OPTIONS_SMOKE_FAIL"
echo "Outputs are in \"$T\""
exit 1
