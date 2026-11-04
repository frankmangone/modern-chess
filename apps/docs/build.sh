#!/usr/bin/env bash
# Builds the WebAssembly bundle, then the book.
#   Prerequisites:
#     rustup target add wasm32-unknown-unknown
#     brew install wasm-pack mdbook
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd)"

need() {
  command -v "$1" >/dev/null 2>&1 || { echo "missing '$1' — install it with: $2" >&2; exit 1; }
}
need wasm-pack "brew install wasm-pack"
need mdbook "brew install mdbook"
rustup target list --installed 2>/dev/null | grep -q '^wasm32-unknown-unknown$' \
  || { echo "missing the wasm target — run: rustup target add wasm32-unknown-unknown" >&2; exit 1; }

wasm-pack build "$repo/crates/wasm" --target web --release \
  --out-dir "$here/theme/pkg" --out-name modern_chess_wasm

mdbook build "$here"
