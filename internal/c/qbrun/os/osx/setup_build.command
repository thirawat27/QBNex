#!/bin/sh
# R11: builds the QIR runner (qbrun) used by `qb --ir` and copies it to
# internal/c/qbrun/bin/osx/qbrun. Requires cargo on PATH.
set -e
cd "$(dirname "$0")"
cargo build --release --locked --manifest-path ../../Cargo.toml --target-dir ../../../../../target
mkdir -p ../../bin/osx
cp ../../../../../target/release/qbrun ../../bin/osx/qbrun
