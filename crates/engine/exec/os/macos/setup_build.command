#!/bin/sh
# Builds the QIR runner (qb-exec) used by `qb --ir` and copies it to
# crates/engine/exec/bin/macos/qb-exec. Requires cargo on PATH.
set -e
cd "$(dirname "$0")"
cargo build --release --locked --manifest-path ../../Cargo.toml --target-dir ../../../../../target
mkdir -p ../../bin/macos
cp ../../../../../target/release/qb-exec ../../bin/macos/qb-exec
