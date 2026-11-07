# Playable app v1: design

Date: 2026-11-07. Status: draft for review.

## Intent

Turn the engine and the docs board into a first playable product: a front page listing
games, a page per game, a lobby where players meet, and a page to play. People will be able
to author their own games later, and a community layer (profiles, friends, chat) will sit
beside the games. The backend is therefore built as a **modular monolith** so the two halves
can grow without tangling, while v1 stays one process and one database.

## Goals

1. A visitor opens the front page, sees the games on offer, opens one, creates or joins a
   lobby, and plays a full game against another person in a second browser.
2. Two backend modules, `game` and `social`, in Rust, mounted by one server binary. Neither
   can import the other. They share only IDs.
3. The data model is real Rust structs in each module, with the database derived from them.
4. Each game's page is **tunable** without code changes: its text, look and sections are data.
5. Rules stay in the engine. The server runs the engine; the browser never decides legality.
6. Built in small steps, each one reviewable on its own.

## Non-goals (v1)

- Authoring games in the app. v1 ships a handful of built-in games. The model is shaped so
  authoring can be added without a rewrite (see "Later").
- Community features: profiles, friends, chat, ratings, leaderboards.
- Real accounts. v1 identity is a **guest**: one tap, a generated handle, a session token.
- Real-time sockets, spectators, clocks, rematch, undo, AI opponents.
- Drops from a hand in the UI. The built-in games chosen need none. The engine and server
  already support drops, so this is a later UI step.
- Admin screens. A game's page is edited as data (a file that is loaded into the database).

## Anti-goals

- `game` importing `social` (or the reverse), directly or through a shared "utils" dumping
  ground. The only shared crate holds IDs and the identity contract.
- Foreign keys across the two modules' tables. They reference each other by ID only.
- The server and the board disagreeing about a position. One piece of code builds the
  snapshot both use.
- Trusting a caller-supplied user id. Identity always comes from a verified session token.
- Internal errors (SQL text, engine debug output) in API responses.

## Shape

```
crates/kernel   UserId, and the identity contract (a trait). Nothing else.
crates/game     domain structs, Postgres store, HTTP router. Depends on engine, kernel.
crates/social   domain structs, Postgres store, HTTP router. Depends on kernel.
crates/session  native Session/View extracted from crates/wasm; used by server and wasm shim.
apps/server     composes the modules: one pool, two routers, wires identity into game.
apps/web        Solid app: front page, game page, play page.
```

Postgres has two schemas, `game` and `social`, each with its own migration history. There are
no foreign keys between them. A `user_id` in `game` is a UUID the identity contract vouches
for.

URLs: `/api/social/*` and `/api/game/*`, so a future split into two services is a routing
change, not a rewrite.

## Data model

All types live in a module's `domain` folder as plain structs, with serde. Invariants are
enforced by constructors and methods, and unit-tested without a database.

**kernel**: `UserId(Uuid)`.

**social**
- `Handle`: validated display name (3 to 24 characters: letters, digits, `_`, `-`).
- `User { id: UserId, handle: Handle, created_at }`. In v1 every user is a guest.
- `Session { token_hash, user: UserId, created_at, expires_at }`. Only a hash of the token is
  stored. `Session::is_valid_at(now)`.

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
- `Player { seat, user: UserId, joined_at }`.
- `GameInstance { id, spec_id, slug, spec_version, status, capacity, players, state, revision,
  outcome, created_at, started_at, finished_at }`. Methods: `join`, `is_full`, `start`. It
  becomes Active by itself when the last seat is taken.
- `Command { instance_id, seq, actor: UserId, kind, accepted, error, created_at }` with
  `CommandKind { Select{x,y}, Move{x,y}, Promote{code}, Drop{code,x,y} }`: an append-only log
  of what players asked for.

## API (v1)

| Module | Call | Does |
|---|---|---|
| social | `POST /api/social/guests` | Creates a guest, returns `{ user, token }`. |
| social | `GET /api/social/me` | The caller's user. |
| game | `GET /api/game/catalog` | Cards for every game that has a page. |
| game | `GET /api/game/catalog/{slug}` | The page, the spec summary, open lobbies. |
| game | `POST /api/game/catalog/{slug}/lobbies` | Creates a lobby and seats the caller. |
| game | `POST /api/game/instances/{id}/join` | Takes a seat. |
| game | `GET /api/game/instances/{id}?since=N` | The caller's view, or "unchanged" if revision is N. |
| game | `POST /api/game/instances/{id}/commands` | Select, move or promote, with the revision it was based on. |

Live updates use polling (`since=revision`, every second or two). It is the simpler thing, it
fits a first version, and sockets can replace it behind the same `GET`.

## v1 catalogue

Chess, Checkers, Ataxx and Chaturanga. Each already has a skin from the docs, none needs
drops, and together they cover 2-player, promotion, forced captures and a non-chess board.

## Decisions already made

- Modular monolith with two modules (the author's).
- Rust for both APIs; the data model as real structs first (the author's).
- A tunable per-game front page (the author's).
- Authoring deferred, with a small built-in catalogue first (the author's, conditional on my
  recommendation).

## My calls (flagged so they can be overruled)

- Build on the existing server and web prototypes: carry them onto this branch and reshape
  them, rather than starting clean.
- Crates, not folders, for modules, so the compiler enforces the boundary.
- Separate Postgres schemas, and one migration history per schema, run with the schema as
  the `search_path`. To be proven in the schema task before anything depends on it.
- Rewrite the uncommitted `0001` migration. Nothing is deployed, so there is no data to
  migrate. (Assumption: nothing in the author's local database is worth keeping.)
- Keep the existing names `GameInstance` and `GameSpec`; routes move under `/api/game/` and
  `/api/social/`.
- Guest identity now, accounts later, behind the same session token.
- Polling, not sockets.
- Extract `Session`/`View` from `crates/wasm` into a native crate so the server and the
  board produce identical snapshots.

## Later

Authoring adds an `author` and a `visibility` to `GameSpec`, an upload endpoint that runs the
engine's string parser (`parse_game_spec_str`) before storing, and a `GamePage` per game that
the author edits. None of that changes a table that v1 creates, apart from added columns.

## Left to the builder

Component structure and styling of the web pages (the existing mockups are the visual
reference), the exact error-body shape (it must not leak internals), and test tooling for the
web app.
