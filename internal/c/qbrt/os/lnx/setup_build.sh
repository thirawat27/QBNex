#!/bin/sh
# Builds the Rust runtime (qbrt) as a static library and copies it here as libqbrt.a.
# Requires cargo on PATH. End users get the prebuilt libqbrt.a in release bundles.
set -e
cd "$(dirname "$0")"
cargo build --release --locked --manifest-path ../../Cargo.toml --target-dir ../../../../../target
cp ../../../../../target/release/libqbrt.a libqbrt.a
echo "$(date) $$" > libqbrt.stamp
