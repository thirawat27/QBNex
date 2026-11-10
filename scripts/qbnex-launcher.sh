#!/bin/sh
# QBNex launcher for Linux and macOS, installed as ./qbnex by setup_lnx.sh /
# setup_osx.command (the same front end as qbnex.cmd on Windows).
#
#   qbnex fmt [options] [files]   formatter        (qb-fmt)
#   qbnex lsp                     language server  (qb-lsp, stdio)
#   qbnex dap                     debug adapter    (qb-dap, stdio)
#   qbnex <anything else>         the compiler (./qb next to this script)
#
# Tools are looked up next to this script, then in this checkout's cargo
# output (target/release, target/debug), then on PATH.

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

case "${1:-}" in
  fmt) tool=qb-fmt ;;
  lsp) tool=qb-lsp ;;
  dap) tool=qb-dap ;;
  *) exec "$here/qb" "$@" ;;
esac
shift

for dir in "$here" "$here/target/release" "$here/target/debug"; do
  if [ -x "$dir/$tool" ]; then
    exec "$dir/$tool" "$@"
  fi
done
if command -v "$tool" >/dev/null 2>&1; then
  exec "$tool" "$@"
fi
echo "qbnex: $tool was not found. Build it with: cargo build --release -p $tool" >&2
exit 127
