# Playable app v1: design

Date: 2026-11-07. Amended the same day: the two backend modules run as **separate processes**.
Status: draft for review.

## Intent

Turn the engine and the docs board into a first playable product: a front page listing
games, a page per game, a lobby where players meet, and a page to play. People will be able
to author their own games later, and a community layer (profiles, friends, chat) will sit
beside the games. The backend is therefore two separate apps, **game-api** and
**social-api**, in one repository. They share a Postgres server but no tables, and they never
call each other.

## Goals

1. A visitor opens the front page, sees the games on offer, opens one, creates or joins a
   lobby, and plays a full game against another person in a second browser.
2. Two backend apps in Rust, each its own process, port and database. Neither can import the
   other. They share only a small vocabulary (IDs and the token contract).
3. The data model is real Rust structs in each module, with the database derived from them.
4. Each game's page is **tunable** without code changes: its text, look and sections are data.
5. Rules stay in the engine. The server runs the engine; the browser never decides legality.
6. Built in small steps, each one reviewable on its own.

## Non-goals (v1)

- Authoring games in the app. v1 ships a handful of built-in games. The model is shaped so
  authoring can be added without a rewrite (see "Later").
- Community features: profiles, friends, chat, ratings, leaderboards.
- Real accounts. v1 identity is a **guest**: one tap, a generated handle, a signed token.
- Revoking a token before it expires.
- Real-time sockets, spectators, clocks, rematch, undo, AI opponents.
- Drops from a hand in the UI. The built-in games chosen need none. The engine and server
  already support drops, so this is a later UI step.
- Admin screens. A game's page is edited as data (a file that is loaded into the database).

## Anti-goals

- `game` and `social` depending on each other, directly or through a shared "utils" dumping
  ground. The only shared crate holds identifiers and the token contract.
- game-api calling social-api at runtime. The only thing that crosses is a signed token.
- The private signing key anywhere but social-api's environment.
- The server and the board disagreeing about a position. One piece of code builds the
  snapshot both use.
- Trusting a caller-supplied user id. Identity always comes from a verified token.
- Internal errors (SQL text, engine debug output) or tokens in API responses and logs.

## Shape

```
apps/game-api     game server binary (today's apps/server, renamed). Port 3001.
apps/social-api   social server binary. Port 3002.
crates/kernel     UserId, Handle, TokenClaims, TokenVerifier, the CurrentUser extractor.
crates/game       domain structs, Postgres store, HTTP router. Depends on engine, kernel.
crates/social     domain structs, Postgres store, token issuing, router. Depends on kernel.
crates/session    native Session/View extracted from crates/wasm; used by game-api and wasm.
apps/web          Solid app: front page, game page, play page. Talks to both APIs.
```

Each API owns **its own database**, `game` and `social`, on one local Postgres server, each
with its own `DATABASE_URL`. Splitting onto separate servers later is a configuration change.
There are no references between the two databases. A `user_id` in `game` is a UUID that a
verified token vouches for.

URLs: `/api/social/*` goes to social-api and `/api/game/*` to game-api. In development the
web app's dev server proxies by prefix; in production a reverse proxy does the same.

**Identity.** social-api creates a guest and signs a token (EdDSA, Ed25519) carrying
`{ sub: UserId, handle, exp }`. game-api holds only the **public** key and verifies tokens
offline. A token lives 7 days. Keys come from environment variables
(`SOCIAL_SIGNING_KEY_PEM` for social-api only, `AUTH_PUBLIC_KEY_PEM` for both), and a make
target generates a development pair.

## Data model

All types live in a module's `domain` folder as plain structs, with serde. Invariants are
enforced by constructors and methods, and unit-tested without a database.

**kernel**
- `UserId(Uuid)`.
- `Handle`: validated display name (3 to 24 characters: letters, digits, `_`, `-`).
- `TokenClaims { sub: UserId, handle: Handle, exp }`.

**social**
- `User { id: UserId, handle: Handle, created_at }`. In v1 every user is a guest. Handles are
  unique.

**game**
- `Slug`: validated (lowercase letters, digits, `-`, 1 to 64).
- `SeatName(String)`: the engine's player id, e.g. `WHITE`.
- `GameSpec { id, slug, version, name, rules: Value, rules_hash, active, created_at }`.
  Immutable once stored: changed rules mean a new version. `seats()` reads the rules through
  the engine's own parser, so seat count is never duplicated.
- `GamePage { slug, tagline, description, tags, accent, cover_image, skin, sections, featured,
  sort_order }`: the tunable front page of one game. `sections` is an ordered list of
  `PageSection` (`HowToPlay { markdown }`, `Preview`, `OpenLobbies`), so a game chooses what
  its page shows and in what order. `skin` is the board skin file the docs already use.
  **A game appears on the front page if and only if it has a `GamePage`.**
- `InstanceStatus { Lobby, Active, Finished }`, with the legal transitions encoded.
- `Outcome { Won { seat }, Draw }`.
- `Player { seat, user: UserId, handle: Handle, joined_at }`. The handle is a **copy taken
  from the token when the player joins**, because game-api cannot look it up.
- `GameInstance { id, spec_id, slug, spec_version, status, capacity, players, state, revision,
  outcome, created_at, started_at, finished_at }`. Methods: `join`, `is_full`, `start`. It
  becomes Active by itself when the last seat is taken.
- `Command { instance_id, seq, actor: UserId, kind, accepted, error, created_at }` with
  `CommandKind { Select{x,y}, Move{x,y}, Promote{code}, Drop{code,x,y} }`: an append-only log
  of what players asked for.

## API (v1)

| App | Call | Does |
|---|---|---|
| social-api | `POST /api/social/guests` | Creates a guest, returns `{ user, token }`. |
| social-api | `GET /api/social/me` | The caller's user. |
| game-api | `GET /api/game/catalog` | Cards for every game that has a page. |
| game-api | `GET /api/game/catalog/{slug}` | The page, the spec summary, open lobbies. |
| game-api | `POST /api/game/catalog/{slug}/lobbies` | Creates a lobby and seats the caller. |
| game-api | `POST /api/game/instances/{id}/join` | Takes a seat. |
| game-api | `GET /api/game/instances/{id}?since=N` | The caller's view, or "unchanged" if revision is N. |
| game-api | `POST /api/game/instances/{id}/commands` | Select, move or promote, with the revision it was based on. |

Live updates use polling (`since=revision`, every second or two). It is the simpler thing, it
fits a first version, and sockets can replace it behind the same `GET`.

## v1 catalogue

Chess, Checkers, Ataxx and Chaturanga. Each already has a skin from the docs, none needs
drops, and together they cover 2-player, promotion, forced captures and a non-chess board.

## Decisions already made

- Two backend apps, separate processes, from day one (the author's).
- Rust for both APIs; the data model as real structs first (the author's).
- A tunable per-game front page (the author's).
- Authoring deferred, with a small built-in catalogue first (the author's, conditional on my
  recommendation).

## My calls (flagged so they can be overruled)

- Build on the existing server and web prototypes: carry them onto this branch and reshape
  them, rather than starting clean. The server is renamed `game-api`.
- Signed stateless tokens (Ed25519), 7 days, no revocation. This replaces the session table
  and in-process resolver of the first draft. The price is that a token cannot be cancelled
  early; for guests that is acceptable.
- **Changed from the first draft:** one database per app instead of two schemas in one
  database. With separate processes it is the plainer boundary and needs no migration
  tricks.
- The handle travels in the token and is copied onto the seat at join time.
- Crates under both apps (`crates/game`, `crates/social`), so the code is testable without
  the binaries and a boundary test can check the dependency graph.
- Keep the existing names `GameInstance` and `GameSpec`; routes live under `/api/game/` and
  `/api/social/`.
- Polling, not sockets.
- Extract `Session`/`View` from `crates/wasm` into a native crate so game-api and the board
  produce identical snapshots.
- Rewrite the uncommitted `0001` migration. Nothing is deployed and the author confirmed
  nothing in their local database is worth keeping.

## Later

Authoring adds an `author` and a `visibility` to `GameSpec`, an upload endpoint that runs the
engine's string parser (`parse_game_spec_str`) before storing, and a `GamePage` per game that
the author edits. Real accounts add credentials to social-api and issue the same kind of token.
If token revocation is ever needed, social-api publishes a short deny list that game-api
refreshes. None of that changes a table that v1 creates, apart from added columns.

## Left to the builder

Component structure and styling of the web pages (the existing mockups are the visual
reference), the exact error-body shape (it must not leak internals), the JWT library, and test
tooling for the web app.
