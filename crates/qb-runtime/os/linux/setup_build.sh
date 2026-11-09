#!/bin/sh
# Builds the Rust runtime (qb-runtime) as a static library and copies it here as libqb_runtime.a.
# Requires cargo on PATH. End users get the prebuilt libqb_runtime.a in release bundles.
set -e
cd "$(dirname "$0")"
cargo build --release --locked --manifest-path ../../Cargo.toml --target-dir ../../../../target
cp ../../../../target/release/libqb_runtime.a libqb_runtime.a
echo "$(date) $$" > qb_runtime.stamp
