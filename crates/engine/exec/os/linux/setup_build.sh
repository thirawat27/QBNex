#!/bin/sh
# Builds the QIR runner (qb-exec) used by `qb` and copies it to
# crates/engine/exec/bin/linux/qb-exec. Requires cargo on PATH.
set -e
cd "$(dirname "$0")"
cargo build --release --locked --manifest-path ../../Cargo.toml --target-dir ../../../../../target
mkdir -p ../../bin/linux
cp ../../../../../target/release/qb-exec ../../bin/linux/qb-exec
