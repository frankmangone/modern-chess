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
need node "brew install node"
rustup target list --installed 2>/dev/null | grep -q '^wasm32-unknown-unknown$' \
  || { echo "missing the wasm target — run: rustup target add wasm32-unknown-unknown" >&2; exit 1; }

wasm-pack build "$repo/crates/wasm" --target web --release \
  --out-dir "$here/theme/pkg" --out-name modern_chess_wasm

# Boards play the same specs the chapters print: copy them next to the skins.
mkdir -p "$here/src/boards"
cp "$here/examples/march.json" "$here/examples/chaturanga_pieces.json" "$here/src/boards/"
cp "$repo/crates/engine/specs/checkers.json" "$repo/crates/engine/specs/ataxx.json" "$here/src/boards/"
# Small prepared positions, generated from the real specs (see boards.derived.json).
node "$here/scripts/derive-boards.mjs"

mdbook build "$here"

# The board element is an ES module plus the WebAssembly bundle; mdBook only copies
# the loader and the stylesheet, so put the rest beside them.
cp "$here/theme/modern-chess-board.js" "$here/theme/board-model.js" "$here/book/theme/"
cp -R "$here/theme/pkg" "$here/book/theme/pkg"
