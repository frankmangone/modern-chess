# Playable App v1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (inline) to
> implement this plan task by task. Steps use checkbox (`- [ ]`) syntax.
> **This plan is reviewed in slices: stop after every task** and wait for the author's review.
> Do not start the next task on your own.
>
> Amended 2026-11-07: the two backend modules run as separate processes (`game-api`,
> `social-api`). Task 0 is done (PR #8).

**Goal:** A front page of games, a page per game, a lobby, and a page to play, backed by two
Rust apps (`game-api`, `social-api`).

**Architecture:** Two separate processes in one repository, each with its own port and its own
database. `crates/game` and `crates/social` hold each side's domain structs, store and router;
`crates/kernel` holds only identifiers and the token contract. social-api signs guest tokens;
game-api verifies them offline with the public key. The browser renders snapshots produced by
one native `Session` shared with the WebAssembly board. The web app is Solid.

**Tech Stack:** Rust 2021, axum 0.8, sqlx 0.8 (Postgres, runtime queries), tokio, serde, an
Ed25519 JWT library, the existing `modern-chess-engine`; Solid + Vite + TypeScript strict;
Postgres 16 via Docker.

**Spec:** `docs/superpowers/specs/2026-11-07-playable-app-design.md`

## Global Constraints

- Rules live only in the engine. No legality logic in the server's routes or in the web app.
- `game` and `social` never depend on each other (crates or apps), share no tables or database,
  and never call each other at runtime. Only a signed token crosses between them.
- The private signing key exists only in social-api's environment (`SOCIAL_SIGNING_KEY_PEM`).
  game-api is configured with the public key only (`AUTH_PUBLIC_KEY_PEM`).
- Package manager for the web app is `pnpm` (remove `package-lock.json`). TypeScript strict, no
  `any`, functional components, named exports, early returns.
- Secrets and connection strings come from environment variables; commit only `.env.example`.
  Tokens and keys are never logged.
- API errors never carry SQL text, engine debug output or stack traces. Inputs are validated at
  the API boundary.
- Database: snake_case, no `SELECT *`, indexes on foreign-key and filter columns, one
  transaction for multi-table writes. Migrations are SQL files run by the app's migrator, never
  by hand.
- Commits are conventional (`feat:`, `fix:`, `chore:`, `refactor:`, `test:`) and end with the
  Co-Authored-By line. Nothing is pushed to `main`; each task becomes a pull request.
- Tests first: watch each fail before making it pass.

## Review Focus

1. **Two people take the last seat at the same moment.** Exactly one wins; the other gets a
   clear "seat taken", never two players in one seat (Tasks 2 and 4).
2. **A move arrives late or twice** (double click, slow network): it carries the revision it was
   based on; a stale one is rejected and does not apply twice (Task 5).
3. **A bad token:** missing, malformed, expired, tampered, or signed with a different key gives
   401, never 500 and never a guess (Task 4).
4. **A stored spec the engine can no longer parse:** the catalogue still loads, that game is
   left out and logged (Task 3).
5. **Abandoned lobbies pile up:** lobbies older than 24 hours are not listed (Task 4). Cleanup
   of old rows is a known limitation, not in v1.
6. **Refreshing the browser mid-game** resumes from the server's state (Task 8, manual).
7. **One API is down:** with social-api stopped, game-api still serves the catalogue and play
   for holders of valid tokens; with game-api stopped, the web app shows a readable message
   (Tasks 4 and 6).

## File Structure

| Path | Responsibility |
|---|---|
| `apps/game-api/` | game server binary: config, pool, migrations, mounts `crates/game`. Port 3001. |
| `apps/social-api/` | social server binary, same shape, mounts `crates/social`. Port 3002. |
| `crates/kernel/` | `UserId`, `Handle`, `TokenClaims`, `TokenVerifier`, `CurrentUser` extractor. |
| `crates/game/src/domain/` | Pure structs and lobby rules: no database, no HTTP. |
| `crates/game/src/store/`, `src/http/` | sqlx repositories; axum router (handlers only translate). |
| `crates/game/migrations/`, `pages/` | SQL for the `game` database; one JSON file per game page. |
| `crates/social/` | Same layout for users, plus `TokenIssuer`. |
| `crates/session/` | Native `Session` and `View`, extracted from `crates/wasm` (Task 5). |
| `apps/web/src/` | `api/` clients and types, `pages/`, `components/`. |

---

### Task 0: Carry the prototypes onto the branch (done)

Delivered in PR #8: `4500ae8`. Baseline compiles; engine 230 and wasm 27 tests pass.

---

### Task 1: Rename, crates and domain structs (no database, no HTTP)

**Files:** rename `apps/server` to `apps/game-api` (package `modern-chess-game-api`; update
`Makefile`, `Dockerfile`, workspace members); create `crates/kernel`, `crates/game`,
`crates/social`; create `apps/game-api/tests/boundaries.rs`.

**Interfaces:**
- `kernel::UserId(Uuid)` (serde as a plain string, `Display`, `FromStr`),
  `kernel::Handle::parse(&str) -> Result<Handle, DomainError>`,
  `kernel::TokenClaims { sub: UserId, handle: Handle, exp: i64 }`.
- The structs in "Data model" in the spec, with these signatures:
  - `game::domain::Slug::parse(&str) -> Result<Slug, DomainError>`
  - `GameSpec::seats(&self) -> Result<Vec<SeatName>, DomainError>`
  - `InstanceStatus::can_become(self, next: InstanceStatus) -> bool`
  - `GameInstance::new_lobby(spec: &GameSpec, now) -> Result<GameInstance, DomainError>`
  - `GameInstance::join(&mut self, user: UserId, handle: Handle, seat: SeatName, now) -> Result<(), LobbyError>`
  - `GameInstance::is_full(&self) -> bool`
  - `LobbyError::{SeatTaken, UnknownSeat, AlreadySeated, NotInLobby}`
  - `social::domain::User::new_guest(handle: Handle, now) -> User`

- [ ] **Step 1: Rename** as its own commit, `refactor: renames the server to game-api`. Run
  `cargo test --workspace` (engine 230, wasm 27). Expected: unchanged.
- [ ] **Step 2: Failing tests.** `kernel`: `user_id_serialises_as_a_plain_string`,
  `handle_rejects_too_short_too_long_and_symbols`, `handle_accepts_letters_digits_underscore_dash`,
  `token_claims_roundtrip_through_json`. `game`: `slug_rejects_uppercase_spaces_and_empty`,
  `slug_accepts_lowercase_digits_and_dashes`, `spec_reports_two_seats_for_chess`,
  `spec_reports_four_seats_for_4player_chess`, `new_lobby_is_empty_with_capacity_from_the_spec`,
  `join_takes_the_requested_seat_and_keeps_the_handle`, `join_rejects_a_taken_seat`,
  `join_rejects_a_second_seat_for_the_same_user`, `join_rejects_an_unknown_seat`,
  `last_join_makes_the_instance_active_and_sets_started_at`, `join_rejects_when_not_in_lobby`,
  `finished_is_terminal`, `game_page_defaults_missing_optional_fields`,
  `game_page_keeps_section_order`. `social`: `guest_user_gets_the_given_handle_and_a_fresh_id`.
  `boundaries.rs`: `game_and_social_do_not_depend_on_each_other` (reads `cargo metadata`; fails if
  either crate or app has the other anywhere in its dependency tree).
- [ ] **Step 3:** Run `cargo test -p modern-chess-kernel -p modern-chess-game -p modern-chess-social`.
  Expected: fails to compile (types missing).
- [ ] **Step 4:** Implement the types in `domain/` modules. Constructors validate; state changes
  go through methods, not public fields where an invariant exists. `GameSpec::seats` parses
  `rules` with the engine's `parse_game_spec_str`.
- [ ] **Step 5:** Same command. Expected: all pass. Then `cargo test --workspace`.
- [ ] **Step 6:** Commit `feat: adds the game and social domain models`.

**Review stop:** the author reads the structs. This is the data model; later tasks only
persist and expose it.

---

### Task 2: Databases and repositories

**Files:** `crates/{game,social}/migrations/*.sql`, `crates/{game,social}/src/store/`,
`Makefile` (`db-up`, `db-down`), `.env.example` per app, replace
`apps/game-api/migrations/` and `infrastructure/db/models/`.

**Interfaces:**
- Consumes the Task 1 structs.
- `game::store::{SpecRepo, PageRepo, InstanceRepo, CommandRepo}` and
  `social::store::UserRepo`, each built from a `PgPool`, returning domain structs, never rows.
- `game::migrate(&PgPool)` and `social::migrate(&PgPool)`; each app runs its own on startup
  against its own `DATABASE_URL`.
- `InstanceRepo::join(id, user, handle, seat) -> Result<GameInstance, StoreError>` locks the row
  (`FOR UPDATE`) inside one transaction.

- [ ] **Step 1:** `make db-up` starts one Postgres 16 container and creates two databases,
  `game` and `social`. Each app's `.env.example` lists its own `DATABASE_URL`. Verify with `psql`.
- [ ] **Step 2: Failing tests** (each runs against a throwaway database):
  `game_migrates_its_own_database`, `social_migrates_its_own_database`,
  `spec_upsert_is_idempotent_for_identical_rules`, `changed_rules_create_a_new_version`,
  `page_upsert_replaces_fields_without_a_new_spec_version`,
  `instance_roundtrips_through_the_database`, `player_roundtrips_with_its_handle_copy`,
  `concurrent_joins_for_one_seat_have_one_winner`,
  `commands_get_consecutive_sequence_numbers`, `guest_user_roundtrips`,
  `handles_are_unique_and_a_clash_is_reported_not_swallowed`.
- [ ] **Step 3:** Run them. Expected: fail (nothing exists).
- [ ] **Step 4:** Write the migrations from the structs (snake_case, indexed foreign keys and
  filter columns) and the repositories.
- [ ] **Step 5:** Run them. Expected: pass. Then `cargo test --workspace`.
- [ ] **Step 6:** Commit `feat: adds the game and social databases and repositories`.

**Review stop:** the author reads the SQL next to the structs.

---

### Task 3: Catalogue API (read-only) and the four pages

**Files:** `crates/game/src/http/catalog.rs`, `crates/game/pages/{chess,checkers,ataxx,chaturanga}.json`,
`apps/game-api/src/` composing the router and seeding on startup.

**Interfaces:**
- Consumes `SpecRepo`, `PageRepo`. Produces `game::router(GameState) -> Router`.
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
- [ ] **Step 4:** Run, then `cargo test --workspace`. Then run game-api and `curl` both endpoints.
- [ ] **Step 5:** Commit `feat: adds the read-only game catalogue`.

**Review stop:** the author calls the endpoints and edits a page file to see it change.

---

### Task 4: Guest tokens and lobbies

**Files:** `crates/kernel/src/token.rs`, `crates/social/src/{token.rs,http/}`, `apps/social-api/`,
`crates/game/src/http/lobbies.rs`, `Makefile` (`dev-keys`, `dev`).

**Interfaces:**
- `kernel::TokenVerifier::from_public_pem(&str) -> Result<TokenVerifier, TokenError>` and
  `verify(&self, token: &str, now) -> Result<TokenClaims, TokenError>`.
- `kernel::CurrentUser { id: UserId, handle: Handle }`: an axum extractor reading
  `Authorization: Bearer`, using a `TokenVerifier` found in the app state. Any failure is 401.
- `social::TokenIssuer::from_private_pem(&str)` and `issue(&self, user: &User, now) -> String`
  (7-day expiry). Lives in `crates/social`, so game-api cannot link signing code.
- `POST /api/social/guests` returns `201 { user, token }` (generated handle `Guest-NNNN`,
  retried on a clash); `GET /api/social/me`.
- `POST /api/game/catalog/{slug}/lobbies` returns `201 { id }` and seats the caller;
  `POST /api/game/instances/{id}/join { seat }`; `GET /api/game/instances/{id}` (lobby form).
- `GET /api/game/catalog/{slug}` now fills `open_lobbies` (status Lobby, newer than 24 hours).
- `make dev-keys` writes a development key pair into git-ignored env files; `make dev` starts
  both apps.

- [ ] **Step 1: Failing tests:** `guest_token_verifies_with_the_public_key`,
  `token_signed_with_a_different_key_is_401`, `expired_token_is_401`, `tampered_token_is_401`,
  `garbage_or_missing_token_is_401_not_500`, `me_returns_the_callers_user`,
  `game_api_config_has_no_private_key`, `creating_a_lobby_requires_a_valid_token`,
  `creator_is_seated_in_the_first_free_seat_with_the_handle_from_the_token`,
  `second_player_joining_a_two_seat_game_makes_it_active`, `four_player_lobby_stays_open_until_four`,
  `cannot_join_twice`, `joining_a_taken_seat_is_409`, `concurrent_joins_over_http_seat_one_player`,
  `open_lobbies_hide_those_older_than_24_hours`,
  `game_api_serves_the_catalogue_and_lobbies_with_social_api_stopped` (Review Focus 7).
- [ ] **Step 2:** Run. Expected: fail. **Step 3:** Implement. **Step 4:** Run, then the
  workspace tests and `game_and_social_do_not_depend_on_each_other` (it must still pass).
  Run both apps with `make dev`, create two guests with `curl` and fill a lobby.
- [ ] **Step 5:** Commit `feat: adds guest tokens and lobbies`.

**Review stop:** the author creates two guests with `curl` and fills a lobby.

---

### Task 5: Playing through game-api

**Files:** create `crates/session` (move `Session`, `View`, tests out of `crates/wasm`; the wasm
crate keeps only the `Board` shim); `crates/game/src/http/play.rs`; remove the old
`/legal-moves`, `/move`, `/drop` routes.

**Interfaces:**
- `session::Session::resume(spec_json: &str, state: &Value, revision: u64) -> Result<Session, String>`
  and `Session::state(&self) -> Value`, added to the existing `new/select/make_move/promote/view`.
- `GET /api/game/instances/{id}?since=N` returns `{ changed: false }` when the revision is N,
  otherwise `{ revision, view, players, status, outcome, you: <seat or null> }`.
- `POST /api/game/instances/{id}/commands` takes `{ revision, kind }` where kind is
  `select`, `move` or `promote`. Returns the new snapshot. 401 without a valid token, 403 when
  it is not your seat's turn, 409 on a stale revision, 422 for an illegal move (logged as rejected).

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
  work). **Step 5:** Commit `feat: plays games through the game api`.

**Review stop:** the author plays a short game with two `curl` sessions.

---

### Task 6: Web foundation and the front page

**Files:** `apps/web/` (`pnpm`), `vite.config.ts` (dev proxy), `src/api/{game.ts,social.ts,types.ts}`,
`src/pages/home.tsx`, `src/components/`.

**Interfaces:** `gameApi.getCatalog()`, `gameApi.getGame(slug)`, `socialApi.createGuest()`; types
mirror the Rust responses. The dev server proxies `/api/game` to port 3001 and `/api/social` to
3002. The token is kept in `localStorage` behind one small module.

- [ ] **Step 1:** Remove `package-lock.json`; `pnpm install`; enable `strict`.
- [ ] **Step 2: Failing tests (Vitest):** `catalog_card_uses_the_games_accent_and_tagline`,
  `home_renders_one_card_per_catalog_entry`, `home_shows_a_readable_message_when_game_api_is_down`,
  `api_client_throws_a_typed_error_for_non_2xx`.
- [ ] **Step 3:** Replace the hard-coded mockup content in `HomePage` with the catalogue. Keep the
  existing visual design. **Step 4:** Tests, `pnpm build`, then open it in a browser against the
  running apps. **Step 5:** Commit `feat: shows the game catalogue on the front page`.

**Review stop:** the author looks at the page.

---

### Task 7: Game page and lobby

**Files:** `src/pages/game.tsx`, `src/components/{lobby-list,section-renderer}.tsx`, the identity bootstrap.

**Interfaces:** the page renders `GamePage.sections` in order through one `section-renderer`:
`HowToPlay` (markdown), `Preview` (the board in demo mode), `OpenLobbies`. "Create lobby" and
"Join" ensure a guest exists first (`socialApi.createGuest` once, remembered), then navigate to
`/play/{id}`.

- [ ] **Step 1: Failing tests:** `sections_render_in_the_order_the_page_defines`,
  `an_unknown_section_type_is_skipped_not_fatal`, `create_lobby_makes_a_guest_first_when_none_exists`,
  `create_lobby_reuses_an_existing_guest`, `an_expired_token_is_replaced_by_a_new_guest`,
  `join_navigates_to_the_play_page`.
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
