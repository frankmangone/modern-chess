#!/usr/bin/env bash
# Compiles the engine (crates/wasm) to WebAssembly in theme/pkg/, git-ignored. The docs book
# and the web app (`pnpm dev` and `pnpm build` in apps/web) both load the board from there.
#   Prerequisites:
#     rustup target add wasm32-unknown-unknown
#     brew install wasm-pack
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd)"

command -v wasm-pack >/dev/null 2>&1 \
  || { echo "missing 'wasm-pack' — install it with: brew install wasm-pack" >&2; exit 1; }
rustup target list --installed 2>/dev/null | grep -q '^wasm32-unknown-unknown$' \
  || { echo "missing the wasm target — run: rustup target add wasm32-unknown-unknown" >&2; exit 1; }

wasm-pack build "$repo/crates/wasm" --target web --release \
  --out-dir "$here/theme/pkg" --out-name modern_chess_wasm
