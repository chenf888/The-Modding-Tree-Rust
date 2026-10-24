#!/usr/bin/env bash
# Build the Rust engine to WebAssembly and place it next to index.html.
set -euo pipefail
cd "$(dirname "$0")"

# Ensure the wasm target is present. `rustup target add` is idempotent on a
# healthy install; on installs with a corrupted component manifest it fails
# while the std files are actually fine, so the error is ignored and cargo
# gets the final say.
rustup target add wasm32-unknown-unknown 2>/dev/null || true
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/tmt_engine.wasm engine.wasm
echo "engine.wasm updated ($(stat -c %s engine.wasm) bytes)"
