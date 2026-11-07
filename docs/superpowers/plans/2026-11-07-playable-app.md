# Playable App v1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (inline) to
> implement this plan task by task. Steps use checkbox (`- [ ]`) syntax.
> **This plan is reviewed in slices: stop after every task** and wait for the author's review.
> Do not start the next task on your own.

**Goal:** A front page of games, a page per game, a lobby, and a page to play, backed by two
Rust modules (`game`, `social`) in one server.

**Architecture:** Modular monolith. `crates/game` and `crates/social` own their domain structs,
Postgres schema and HTTP router; `crates/kernel` holds only `UserId` and the identity
contract; `apps/server` composes them. The browser renders snapshots produced by one native
`Session` shared with the WebAssembly board. The web app is Solid.

**Tech Stack:** Rust 2021, axum 0.8, sqlx 0.8 (Postgres, runtime queries), tokio, serde,
the existing `modern-chess-engine`; Solid + Vite + TypeScript strict; Postgres 16 via Docker.

**Spec:** `docs/superpowers/specs/2026-11-07-playable-app-design.md`

## Global Constraints

- Rules live only in the engine. No legality logic in the server's routes or in the web app.
- `game` and `social` never depend on each other, and no foreign keys cross their schemas.
- Package manager for the web app is `pnpm` (remove `package-lock.json`). TypeScript strict, no
  `any`, functional components, named exports, early returns.
- Secrets and connection strings come from environment variables; commit only `.env.example`.
- API errors never carry SQL text, engine debug output or stack traces. Inputs are validated at
  the API boundary.
- Database: snake_case, no `SELECT *`, indexes on foreign-key and filter columns, one
  transaction for multi-table writes. Migrations are SQL files run by the migrator, never by hand.
- Commits are conventional (`feat:`, `fix:`, `chore:`, `refactor:`, `test:`) and end with the
  Co-Authored-By line. Nothing is pushed to `main`; each task becomes a pull request.
- Tests first: watch each fail before making it pass.

## Review Focus

1. **Two people take the last seat at the same moment.** Exactly one wins; the other gets a
   clear "seat taken", never two players in one seat (Tasks 2 and 4).
2. **A move arrives late or twice** (double click, slow network): it carries the revision it was
   based on; a stale one is rejected and does not apply twice (Task 5).
3. **Missing, malformed or expired token** gives 401, never 500 and never a guess (Task 4).
4. **A stored spec the engine can no longer parse:** the catalogue still loads, that game is
   left out and logged (Task 3).
5. **Abandoned lobbies pile up:** lobbies older than 24 hours are not listed (Task 4). Cleanup
   of old rows is a known limitation, not in v1.
6. **Refreshing the browser mid-game** resumes from the server's state (Task 8, manual).

## File Structure

| Path | Responsibility |
|---|---|
| `crates/kernel/` | `UserId`; `IdentityResolver` trait and the request extractor (Task 4). |
| `crates/game/src/domain/` | Pure structs and rules of a lobby: no database, no HTTP. |
| `crates/game/src/store/` | sqlx repositories mapping rows to domain structs. |
| `crates/game/src/http/` | axum router; handlers only translate. |
| `crates/game/migrations/` | SQL for the `game` schema. |
| `crates/game/pages/` | One JSON file per game: the tunable page, loaded into the database. |
| `crates/social/` | Same layout, for users and sessions. |
| `crates/session/` | Native `Session` and `View`, extracted from `crates/wasm` (Task 5). |
| `apps/server/` | Config, pool, migrator runs, composes routers. Replaces `features/` and `infrastructure/`. |
| `apps/web/src/` | `api/` client and types, `pages/`, `components/`. |

---

### Task 0: Carry the prototypes onto the branch

**Files:** `apps/server/**`, `apps/web/**`, `Makefile`, `Dockerfile`, `.dockerignore`.
Left out on purpose: `images/` and `modern-chess.pen`.

**Interfaces:** Produces a compiling baseline that Tasks 1 to 8 reshape.

- [ ] **Step 1:** Check with the author that `stash@{0}` (the tracked edits on
  `feature/spec-docs`) may be read, and that `images/` and the `.pen` file stay untracked.
- [ ] **Step 2:** Restore only `apps/server` and `apps/web` tracked edits from the stash with a
  path-limited checkout. Leave the stash itself in place. Let `cargo` regenerate `Cargo.lock`.
- [ ] **Step 3:** `cargo check -p modern-chess-server`. Expected: errors where the server still
  matches the removed `GamePhase::Jumping` and the old `Moving { position }`. Adapt them to
  `Moving { position, locked, capture_only }` as was done in `apps/cli` and `crates/wasm`.
- [ ] **Step 4:** `cargo test --workspace`. Expected: all pass (engine 230, wasm 27).
- [ ] **Step 5:** Commit `chore: carries the server and web prototypes onto the branch`.

**Review stop:** the author confirms the baseline is the work they meant.

---

### Task 1: Crates and domain structs (no database, no HTTP)

**Files:** Create `crates/kernel`, `crates/game`, `crates/social`; modify root `Cargo.toml`
members; create `apps/server/tests/boundaries.rs`.

**Interfaces:**
- Produces `kernel::UserId(Uuid)` (serde as a plain string, `Display`, `FromStr`).
- Produces the structs listed under "Data model" in the spec, with these signatures:
  - `game::domain::Slug::parse(&str) -> Result<Slug, DomainError>`
  - `GameSpec::seats(&self) -> Result<Vec<SeatName>, DomainError>`
  - `InstanceStatus::can_become(self, next: InstanceStatus) -> bool`
  - `GameInstance::new_lobby(spec: &GameSpec, now) -> Result<GameInstance, DomainError>`
  - `GameInstance::join(&mut self, user: UserId, seat: SeatName, now) -> Result<(), LobbyError>`
  - `GameInstance::is_full(&self) -> bool`
  - `LobbyError::{SeatTaken, UnknownSeat, AlreadySeated, NotInLobby}`
  - `social::domain::Handle::parse(&str) -> Result<Handle, DomainError>`
  - `Session::is_valid_at(&self, now) -> bool`

- [ ] **Step 1: Failing tests.** `kernel`: `user_id_serialises_as_a_plain_string`. `game`:
  `slug_rejects_uppercase_spaces_and_empty`, `slug_accepts_lowercase_digits_and_dashes`,
  `spec_reports_two_seats_for_chess`, `spec_reports_four_seats_for_4player_chess`,
  `new_lobby_is_empty_with_capacity_from_the_spec`, `join_takes_the_requested_seat`,
  `join_rejects_a_taken_seat`, `join_rejects_a_second_seat_for_the_same_user`,
  `join_rejects_an_unknown_seat`, `last_join_makes_the_instance_active_and_sets_started_at`,
  `join_rejects_when_not_in_lobby`, `finished_is_terminal`,
  `game_page_defaults_missing_optional_fields`, `game_page_keeps_section_order`.
  `social`: `handle_rejects_too_short_too_long_and_symbols`,
  `session_is_invalid_at_and_after_expiry`.
  `boundaries.rs`: `game_and_social_do_not_depend_on_each_other` (reads `cargo metadata`;
  fails if either crate's dependency tree contains the other).
- [ ] **Step 2:** Run `cargo test -p modern-chess-kernel -p modern-chess-game -p modern-chess-social`.
  Expected: fails to compile (types missing).
- [ ] **Step 3:** Implement the types in `domain/` modules. Constructors validate; state changes
  go through methods, not public fields where an invariant exists. `GameSpec::seats` parses
  `rules` with the engine's `parse_game_spec_str`.
- [ ] **Step 4:** Same command. Expected: all pass. Then `cargo test --workspace`.
- [ ] **Step 5:** Commit `feat: adds the game and social domain models`.

**Review stop:** the author reads the structs. This is the data model; later tasks only
persist and expose it.

---

### Task 2: Schema and repositories

**Files:** `crates/{game,social}/migrations/*.sql`, `crates/{game,social}/src/store/`,
`Makefile` (`db-up`, `db-down`), `.env.example`, replace `apps/server/migrations/` and
`infrastructure/db/models/`.

**Interfaces:**
- Consumes the Task 1 structs.
- Produces `game::store::{SpecRepo, PageRepo, InstanceRepo, CommandRepo}` and
  `social::store::{UserRepo, SessionRepo}`, each constructed from a `PgPool`, returning domain
  structs, never rows.
- Produces `game::migrate(&PgPool)` and `social::migrate(&PgPool)`.
- `InstanceRepo::join(id, user, seat) -> Result<GameInstance, StoreError>` locks the row
  (`FOR UPDATE`) inside one transaction.

- [ ] **Step 1:** `make db-up` starts Postgres 16 in Docker; `.env.example` lists
  `DATABASE_URL`. Verify with `psql`.
- [ ] **Step 2: Failing tests** (each runs against a throwaway database):
  `both_modules_migrate_into_one_database_with_separate_histories` (proves the schema-as-
  `search_path` approach; if it fails, record a ruling and fall back to one migrator with
  per-module version ranges), `game_tables_live_in_game_and_user_tables_in_social`,
  `no_foreign_key_crosses_the_two_schemas`, `spec_upsert_is_idempotent_for_identical_rules`,
  `changed_rules_create_a_new_version`, `page_upsert_replaces_fields_without_a_new_spec_version`,
  `instance_roundtrips_through_the_database`, `concurrent_joins_for_one_seat_have_one_winner`,
  `commands_get_consecutive_sequence_numbers`, `guest_user_and_session_roundtrip`,
  `expired_sessions_are_not_found`.
- [ ] **Step 3:** Run them. Expected: fail (nothing exists).
- [ ] **Step 4:** Write the migrations from the structs (snake_case, indexed foreign keys and
  filter columns, no cross-schema references) and the repositories.
- [ ] **Step 5:** Run them. Expected: pass. Then `cargo test --workspace`.
- [ ] **Step 6:** Commit `feat: adds the game and social schemas and repositories`.

**Review stop:** the author reads the SQL next to the structs.

---

### Task 3: Catalogue API (read-only) and the four pages

**Files:** `crates/game/src/http/catalog.rs`, `crates/game/pages/{chess,checkers,ataxx,chaturanga}.json`,
`apps/server/src/main.rs` and a new `lib.rs` composing routers, seeding on startup.

**Interfaces:**
- Consumes `SpecRepo`, `PageRepo`.
- Produces `game::router(GameState) -> Router`.
- `GET /api/game/catalog` returns `[{ slug, name, tagline, tags, accent, cover_image, featured }]`
  ordered by `sort_order`. `GET /api/game/catalog/{slug}` returns the page, the spec summary
  and the open lobbies (empty until Task 4). Unknown slug returns 404 with a plain body.
- Seeding: engine specs from `crates/engine/specs/` are stored as `GameSpec`s; each file in
  `pages/` is stored as a `GamePage`. Copy each game's skin from `apps/docs/src/boards/`.

- [ ] **Step 1: Failing tests** (axum `oneshot` against a test database):
  `catalog_lists_only_games_that_have_a_page`, `catalog_orders_by_sort_order_then_name`,
  `catalog_detail_returns_page_sections_in_order`, `unknown_slug_is_404_with_a_plain_body`,
  `a_spec_that_no_longer_parses_is_skipped_and_the_rest_still_load` (Review Focus 4),
  `reseeding_changed_page_text_updates_it_without_a_new_spec_version`.
- [ ] **Step 2:** Run. Expected: fail.
- [ ] **Step 3:** Implement the router and seeding. Error bodies are `{ "error": "<short message>" }`.
- [ ] **Step 4:** Run, then `cargo test --workspace`. Then run the server and `curl` both endpoints.
- [ ] **Step 5:** Commit `feat: adds the read-only game catalogue`.

**Review stop:** the author calls the endpoints and edits a page file to see it change.

---

### Task 4: Guest identity and lobbies

**Files:** `crates/social/src/http/`, `crates/kernel/src/identity.rs`,
`crates/game/src/http/lobbies.rs`, server wiring.

**Interfaces:**
- Produces `kernel::IdentityResolver` (`async fn resolve(&self, token: &str) -> Option<UserId>`)
  and an axum extractor `kernel::CurrentUser(UserId)` that reads `Authorization: Bearer`.
- `social` implements the resolver; the server hands it to `game`'s state. `game` only sees the trait.
- `POST /api/social/guests` returns `201 { user, token }`; `GET /api/social/me`.
- `POST /api/game/catalog/{slug}/lobbies` returns `201 { id }` and seats the caller;
  `POST /api/game/instances/{id}/join { seat }`; `GET /api/game/instances/{id}` (lobby form).
- `GET /api/game/catalog/{slug}` now fills `open_lobbies` (status Lobby, newer than 24 hours).

- [ ] **Step 1: Failing tests:** `guest_creation_returns_a_token_that_resolves_to_the_user`,
  `me_without_a_token_is_401`, `a_garbage_or_expired_token_is_401_not_500`,
  `creating_a_lobby_requires_a_valid_token`, `creator_is_seated_in_the_first_free_seat`,
  `second_player_joining_a_two_seat_game_makes_it_active`, `four_player_lobby_stays_open_until_four`,
  `cannot_join_twice`, `joining_a_taken_seat_is_409`, `concurrent_joins_over_http_seat_one_player`,
  `open_lobbies_hide_those_older_than_24_hours`, `the_token_is_never_stored_only_its_hash`.
- [ ] **Step 2:** Run. Expected: fail. **Step 3:** Implement. **Step 4:** Run, then the workspace
  tests and `game_and_social_do_not_depend_on_each_other` (it must still pass).
- [ ] **Step 5:** Commit `feat: adds guest identity and lobbies`.

**Review stop:** the author creates two guests with `curl` and fills a lobby.

---

### Task 5: Playing through the server

**Files:** create `crates/session` (move `Session`, `View`, tests out of `crates/wasm`; the wasm
crate keeps only the `Board` shim); `crates/game/src/http/play.rs`; remove the old
`/legal-moves`, `/move`, `/drop` routes.

**Interfaces:**
- `session::Session::resume(spec_json: &str, state: &Value, revision: u64) -> Result<Session, String>`
  and `Session::state(&self) -> Value`, added to the existing `new/select/make_move/promote/view`.
- `GET /api/game/instances/{id}?since=N` returns `{ changed: false }` when the revision is N,
  otherwise `{ revision, view, players, status, outcome, you: <seat or null> }`.
- `POST /api/game/instances/{id}/commands` takes `{ revision, kind }` where kind is
  `select`, `move` or `promote`. Returns the new snapshot. 401 without a token, 403 when it is
  not your seat's turn, 409 on a stale revision, 422 for an illegal move (logged as rejected).

- [ ] **Step 1: Failing tests:** `session_resume_reproduces_the_same_view`,
  `wasm_and_server_views_are_identical_for_the_same_position`,
  `only_the_seat_to_move_may_select`, `move_advances_the_turn_and_bumps_the_revision`,
  `a_stale_revision_is_409_and_changes_nothing`, `the_same_command_sent_twice_applies_once`,
  `an_illegal_destination_is_422_and_logged_as_rejected`, `promotion_asks_then_applies`,
  `a_win_marks_the_instance_finished_with_the_outcome`, `a_draw_marks_it_finished_as_draw`,
  `a_finished_instance_refuses_commands`, `a_lobby_refuses_commands`.
- [ ] **Step 2:** Run. Expected: fail. **Step 3:** Extract the crate first (existing wasm tests
  must still pass untouched), then add `resume`/`state`, then the endpoints. Each command runs
  in one transaction that locks the instance row, appends to the command log and bumps revision.
- [ ] **Step 4:** Run the workspace tests and `./build.sh` in `apps/docs` (the board must still
  work). **Step 5:** Commit `feat: plays games through the server`.

**Review stop:** the author plays a short game with two `curl` sessions.

---

### Task 6: Web foundation and the front page

**Files:** `apps/web/` (`pnpm`), `src/api/{client.ts,types.ts}`, `src/pages/home.tsx`,
`src/components/`.

**Interfaces:** `api.getCatalog()`, `api.getGame(slug)`, `api.createGuest()`; types mirror the
Rust responses. Token kept in `localStorage`, behind one small module.

- [ ] **Step 1:** Remove `package-lock.json`; `pnpm install`; enable `strict`.
- [ ] **Step 2: Failing tests (Vitest):** `catalog_card_uses_the_games_accent_and_tagline`,
  `home_renders_one_card_per_catalog_entry`, `home_shows_a_readable_message_when_the_api_is_down`,
  `api_client_throws_a_typed_error_for_non_2xx`.
- [ ] **Step 3:** Replace the hard-coded mockup content in `HomePage` with the catalogue. Keep the
  existing visual design. **Step 4:** Tests, `pnpm build`, then open it in a browser against the
  running server. **Step 5:** Commit `feat: shows the game catalogue on the front page`.

**Review stop:** the author looks at the page.

---

### Task 7: Game page and lobby

**Files:** `src/pages/game.tsx`, `src/components/{lobby-list,section-renderer}.tsx`, the identity bootstrap.

**Interfaces:** the page renders `GamePage.sections` in order through one `section-renderer`:
`HowToPlay` (markdown), `Preview` (the board in demo mode), `OpenLobbies`. "Create lobby" and
"Join" ensure a guest exists first (`createGuest` once, remembered), then navigate to `/play/{id}`.

- [ ] **Step 1: Failing tests:** `sections_render_in_the_order_the_page_defines`,
  `an_unknown_section_type_is_skipped_not_fatal`, `create_lobby_makes_a_guest_first_when_none_exists`,
  `create_lobby_reuses_an_existing_guest`, `join_navigates_to_the_play_page`.
- [ ] **Step 2:** Run, fail. **Step 3:** Implement. **Step 4:** Tests, build, then in a browser:
  open `/games/chess`, create a lobby, see it in a second browser's list. **Step 5:** Commit
  `feat: adds the game page and lobbies`.

**Review stop:** the author edits a page file and sees the page change after a reseed.

---

### Task 8: The play page

**Files:** `src/pages/play.tsx`, `apps/docs/theme/modern-chess-board.js` (new `remote` mode,
shared with the docs), `src/api/poll.ts`.

**Interfaces:** `<modern-chess-board remote>` accepts a `view` property and emits `select`,
`move` and `promote` events; it keeps no engine of its own in this mode. The page polls
`?since=revision` every 1.5 s, sends commands with the revision, and shows seats, whose turn it
is, and the result.

- [ ] **Step 1: Failing tests:** `remote_mode_renders_a_given_view_without_loading_wasm`,
  `remote_mode_emits_select_and_move_events`, `poll_stops_when_the_game_is_finished`,
  `a_409_refetches_instead_of_showing_an_error`, `waiting_room_shows_empty_seats_until_full`.
- [ ] **Step 2:** Run, fail. **Step 3:** Implement. **Step 4:** Tests and build, then drive two
  browsers (two guests) through a full game, then refresh mid-game and confirm it resumes
  (Review Focus 6). **Step 5:** Commit `feat: adds the play page`.

**Review stop:** the author plays a game against themselves in two windows.
