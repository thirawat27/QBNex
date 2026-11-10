#!/bin/sh
# QBNex setup on macOS: the same QIR pipeline as Linux (no C/C++ toolchain).
cd "$(dirname "$0")/.." || exit 1
exec sh scripts/setup_lnx.sh "$@"
