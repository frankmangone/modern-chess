# Sharing a throwaway copy with friends

`make share` runs the whole app on your machine and opens a temporary public address for it
through a free Cloudflare quick tunnel. Friends open the address in a browser. Press Ctrl-C and
the address stops working. Nothing is deployed anywhere.

## One-time setup

```
brew install caddy cloudflared
```

You also need the usual dev setup: Docker running, `pnpm install` done in `apps/web`, and the
covers uploaded once with `make covers-seed`.

## Run it

1. Stop `make dev` if it is running. Both want ports 3001 and 3002. Port 8787 must be free too.
2. `make share`
3. Wait for `Share this address: https://....trycloudflare.com` and send that link.

The address is new on every run. The first run compiles the apps and can take a few minutes.

## What runs

| Part | Where |
|---|---|
| Caddy (`deploy/Caddyfile`) | `127.0.0.1:8787`, the only thing the tunnel can reach. Use `SHARE_PORT=9000 make share` if that port is taken |
| Web app | built into `apps/web/dist`, served by Caddy |
| game-api, social-api | ports 3001 and 3002, behind `/api/game` and `/api/social` |
| Cover images | `/modern-chess/covers/*` only, forwarded to MinIO |
| Postgres, MinIO console | not reachable from outside |

game-api is started with `IMAGES_PUBLIC_URL` set to the tunnel address, so the covers load for
everyone. Logs of the proxy and the tunnel are in `target/share/`.

## Things to know

- Players are guests: each friend gets one on the first visit, kept in that browser. Clearing
  site data, or an expired token, means a new guest and a lost seat in any running match.
- There is no rate limiting. Do not post the address publicly.
- The signing keys are the development ones. Do not reuse them for anything real.
- Your machine has to stay on and awake while friends play.
