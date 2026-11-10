#!/usr/bin/env bash
# Runs the whole app on this machine and opens a temporary public address for it, so friends
# can try it. Everything stops with Ctrl-C and the address dies with it.
# Use `make share`, which builds the web app and starts the database and image store first.
set -euo pipefail
cd "$(dirname "$0")/.."

# Where the gateway (nginx) listens. Change it if the port is taken.
PORT="${SHARE_PORT:-8787}"
export SHARE_PORT="$PORT"
LOGS=target/share
mkdir -p "$LOGS"

need() {
  command -v "$1" >/dev/null || {
    echo "$1 is not installed. Install it with: brew install $1" >&2
    exit 1
  }
}
need nginx
need cloudflared
need curl
need lsof

for port in 3001 3002 "$PORT"; do
  if lsof -nP -iTCP:"$port" -sTCP:LISTEN >/dev/null 2>&1; then
    echo "Port $port is already in use. Stop make dev (or whatever uses it) first." >&2
    exit 1
  fi
done

trap 'kill 0' INT TERM

# nginx cannot read environment variables, so the port is filled into a copy of the config.
# Run from the repo root: the config's relative paths (the web build, the logs) start here.
sed "s|__SHARE_PORT__|$PORT|g" deploy/nginx.conf >"$LOGS/nginx.conf"
mkdir -p "$LOGS/nginx-tmp"
nginx -t -p "$PWD/" -c "$PWD/$LOGS/nginx.conf" >"$LOGS/nginx-check.log" 2>&1 || {
  echo "nginx does not accept deploy/nginx.conf. See $LOGS/nginx-check.log" >&2
  exit 1
}
nginx -p "$PWD/" -c "$PWD/$LOGS/nginx.conf" >"$LOGS/nginx.log" 2>&1 &
cloudflared tunnel --no-autoupdate --url "http://127.0.0.1:$PORT" >"$LOGS/tunnel.log" 2>&1 &

url=""
for _ in $(seq 1 60); do
  url=$(grep -o 'https://[a-z0-9-]*\.trycloudflare\.com' "$LOGS/tunnel.log" | head -1 || true)
  [ -n "$url" ] && break
  sleep 1
done
if [ -z "$url" ]; then
  echo "The tunnel gave no address after 60 seconds. See $LOGS/tunnel.log" >&2
  kill 0
fi

# Cover images are served from this address too, so the API has to hand out links to it.
export IMAGES_PUBLIC_URL="$url/modern-chess"
cargo run -q -p modern-chess-social-api &
cargo run -q -p modern-chess-game-api &

echo "Waiting for the apps to start (the first build can take a few minutes)..."
local_ok=""
for _ in $(seq 1 300); do
  # A foreign Host header, like the tunnel sends, and a real answer, not just a 200.
  if curl -sf -H "Host: tunnel-check.example" "http://127.0.0.1:$PORT/api/game/catalog" | grep -q '"slug"'; then
    local_ok=1
    break
  fi
  sleep 1
done
if [ -z "$local_ok" ]; then
  echo "The apps did not come up. Their output is above; the gateway's is in $LOGS/nginx-error.log" >&2
  kill 0
fi

echo
echo "Share this address: $url"
echo "It may take up to a minute before it opens from other machines. Press Ctrl-C to stop."
echo
wait
