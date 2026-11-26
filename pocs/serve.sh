#!/usr/bin/env bash
# Shows the rounds that run the real engine (d-play-first, e-quiet-cards). They need HTTP and the
# wasm build (make wasm). Only what the pages load is copied into target/poc-site and served on
# this machine alone, so nothing else in the repo is exposed.
#   pocs/serve.sh   then open http://127.0.0.1:4180/pocs/e-quiet-cards/home.html
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
site="$repo/target/poc-site"
[ -d "$repo/apps/docs/theme/pkg" ] || { echo "Build the engine first: make wasm" >&2; exit 1; }
for round in d-play-first e-quiet-cards; do node "$repo/pocs/$round/build-data.mjs"; done
rm -rf "$site"
mkdir -p "$site/pocs" "$site/apps/web/src" "$site/apps/docs" "$site/crates/engine"
cp -R "$repo/pocs/d-play-first" "$repo/pocs/e-quiet-cards" "$site/pocs/"
cp "$repo/apps/web/src/styles.css" "$site/apps/web/src/"
cp -R "$repo/apps/docs/theme" "$site/apps/docs/theme"
cp -R "$repo/crates/engine/specs" "$site/crates/engine/specs"
cp -R "$repo/apps/web/public/skins" "$site/skins"
echo "http://127.0.0.1:4180/pocs/e-quiet-cards/home.html"
exec python3 -m http.server 4180 --bind 127.0.0.1 --directory "$site"
