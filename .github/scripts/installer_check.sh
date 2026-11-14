#!/usr/bin/env bash
# The one-file installer on a real system: install into a folder that already
# holds a file of the user's and an old copy of qb, compile and run a program
# with the installed compiler, install again over it, then uninstall.
# usage: installer_check.sh <qbnex-setup> <bundle-dir> <scratch-dir>
set -euo pipefail
setup="$1"; bundle="$2"; work="$3"
rm -rf "$work"; mkdir -p "$work"
pkg="$work/qbnex-setup-test"
"$setup" --pack "$bundle" --out "$pkg"
dir="$work/QBNex Install"
mkdir -p "$dir"
echo mine > "$dir/notes.txt"
echo "old qb" > "$dir/qb"

run_hello() {
  printf 'PRINT "hello from"; 1 + 1\n' > "$work/hello.bas"
  rm -f "$work/hello"
  "$dir/qb" "$work/hello.bas" -o "$work/hello" > "$work/build.log" 2>&1 || { cat "$work/build.log"; exit 1; }
  out=$(QBNEX_HEADLESS=1 QBNEX_NO_ERROR_DIALOG=1 "$work/hello")
  [ "$out" = "hello from 2 " ] || { echo "unexpected output: [$out]"; exit 1; }
}

"$pkg" --yes --no-path --no-menu --dir "$dir"
[ "$(cat "$dir/notes.txt")" = mine ] || { echo "install touched the user's file"; exit 1; }
"$dir/qb" --version
run_hello
echo "first install: ok"

"$pkg" --yes --no-path --no-menu --dir "$dir"
run_hello
echo "install over the old one: ok"

"$dir/qbnex-setup" --uninstall --yes --dir "$dir"
[ ! -e "$dir/qb" ] || { echo "uninstall left qb behind"; exit 1; }
[ "$(cat "$dir/notes.txt")" = mine ] || { echo "uninstall removed the user's file"; exit 1; }
echo "uninstall: ok"
