#!/bin/sh
# QBNex setup on macOS: the same QIR pipeline as Linux.
cd "$(dirname "$0")/.." || exit 1
exec sh scripts/setup_lnx.sh "$@"
