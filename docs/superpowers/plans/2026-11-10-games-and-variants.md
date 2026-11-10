# Games and variants, server and data (PR 1 of 2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A game becomes a container of variants in the game crate and API, so Go is one game with three variants, with every existing match, lobby, rules version and cover untouched.

**Architecture:** `game_pages` stays the game row and gains `name` and `preview_variant`. A new `game_variants` table maps a variant slug (the slug of its rules in `game_specs`) to a game. The seed saves a game and its variants in one transaction and drops page rows that became variants of another game. The API is additive; handlers resolve lobbies and matches through `game_variants`.

**Tech Stack:** Rust, axum, sqlx 0.8 (Postgres), tests against the throwaway-database helper `modern_chess_testdb::with_db` (needs `make db-up`).

**Spec:** `docs/superpowers/specs/2026-11-10-games-and-variants-design.md`. PR 2 (web) gets its own plan after this one is built, so it is written against the API as it really came out.

## Global Constraints

- Variant slugs of existing games do not change: `go`, `go-13`, `go-19` stay the slugs of the rules in `game_specs` and of matches in `game_instances.slug`. No change to `game_instances`, `game_specs` or any engine file.
- The seed only adds and moves. It never deletes a variant or a game because a file stopped listing it. Its one deletion: a `game_pages` row whose slug is a variant of a different game **and that has no variants of its own left**.
- A game is listed if it has a page and at least one variant whose rules the engine can read. Unreadable variants are omitted and logged with `warn!`.
- The API only gains fields; nothing is removed or renamed. `spec` in `GET /catalog/{game}` stays, and is the preview variant's.
- A variant's skin is a full replacement of the game's, never merged. The server sends a variant's `skin` only when it differs from the game's, otherwise `null`.
- Code and API say "game" and "variant" explicitly, never a bare "slug" for either, except where the existing field is named `slug`.
- A page file without `variants` is one variant: slug = the page's slug, label = the rules' display name. The six single-variant page files do not change.
- Only the engine crate denies `unwrap`/`expect`/indexing, but avoid `unwrap` in non-test game code anyway. Scope clippy to changed crates (`cargo clippy -p modern-chess-game --all-targets -- -D warnings`); format only touched files with `rustfmt --edition 2021 <files>`.
- Never stage `images/` or `modern-chess.pen`; stage files by explicit list. Commits are conventional and end with `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. Do not push (the user runs pushes). Work on branch `feature/game-variants`.
- Do not touch the user's servers on 3001/3002 or kill processes by name.

## Review Focus

- Re-running the seed on a database already regrouped changes nothing (idempotent), and a cover uploaded on `go` survives it.
- Seeding on a database that still has the old `go-13` and `go-19` page rows (the real upgrade path) ends with exactly one `go` game and no leftover pages, with old lobbies and matches of `go-13` still readable.
- A variant moved out of a game that still has other variants does not delete that game's page.
- A match whose variant row is missing still answers (`game` = its own slug, `variant_label` null).
- `GET /catalog/go-13` is 404 and `POST /catalog/go-13/lobbies` still creates a lobby.
- A game whose only variant has unreadable rules is left out of `/catalog` instead of failing it.

---

## File Structure

- Create `crates/game/migrations/0002_variants.up.sql`, `0002_variants.down.sql`: columns, table, backfill.
- Create `crates/game/src/domain/variant.rs`: `Variant`. Modify `domain/page.rs` (two fields), `domain/mod.rs`.
- Create `crates/game/src/store/variant.rs`: `VariantRepo` (reads). Modify `store/page.rs` (new columns, `save_game`), `store/instance.rs` (`open_lobbies` takes slugs), `store/mod.rs`.
- Modify `crates/game/src/seed.rs`: page files with variants.
- Modify `crates/game/src/http/{mod.rs,catalog.rs,lobbies.rs,play.rs}`.
- Modify `crates/game/pages/go.json`; delete `go-13.json`, `go-19.json` (with `git rm`).
- Tests: create `crates/game/tests/variants.rs`; update `tests/{catalog,lobbies,store}.rs` for new fields and the Go regrouping.
- Docs: `docs/superpowers/plans/2026-11-09-roadmap.md`.

---

### Task 1: Data model, migration and repositories

**Files:**
- Create: `crates/game/migrations/0002_variants.up.sql`, `crates/game/migrations/0002_variants.down.sql`, `crates/game/src/domain/variant.rs`, `crates/game/src/store/variant.rs`
- Modify: `crates/game/src/domain/{mod.rs,page.rs}`, `crates/game/src/store/{mod.rs,page.rs,instance.rs}`, `crates/game/src/http/mod.rs` (add `variants: VariantRepo` to `GameState`), existing tests that build `GamePage` literals
- Test: `crates/game/tests/variants.rs` (new)

**Interfaces:**
- Produces:
  - `pub struct Variant { pub slug: Slug, pub game: Slug, pub label: String, pub sort_order: i32, pub skin: Option<Value> }` (derive `Debug, Clone, PartialEq`), with `Variant::single(game: &Slug, label: &str) -> Variant` (slug = game, sort_order 0, no skin).
  - `GamePage` gains `#[serde(default)] pub name: Option<String>` and `#[serde(default)] pub preview_variant: Option<Slug>`.
  - `PageRepo::save_game(&self, page: &GamePage, variants: &[Variant]) -> Result<(), StoreError>`; `PageRepo::upsert` now also stores `name` and `preview_variant` (it keeps its cover rule and still saves the page alone).
  - `VariantRepo::new(PgPool)`, `get(&Slug) -> Result<Option<Variant>, StoreError>`, `for_game(&Slug) -> Result<Vec<Variant>, StoreError>`, `all() -> Result<Vec<Variant>, StoreError>`; the last two ordered by `sort_order, slug`.
  - `InstanceRepo::open_lobbies(&self, slugs: &[Slug], not_before, limit)`.
  - `GameState.variants: VariantRepo` (public, like the other repos).

- [ ] **Step 1: Write the failing tests** in `crates/game/tests/variants.rs`. Add `store_*` helpers local to the file (copy the style of `tests/catalog.rs`: `ready`, `store_spec`, `get`).
  - `migration_backfills_one_variant_per_existing_page`: with a raw database, run the SQL of `0001_init.up.sql` with `sqlx::raw_sql(include_str!("../migrations/0001_init.up.sql"))`, insert a `game_specs` row (slug `chess`, name `Chess`, version 1) and two `game_pages` rows (`chess`, and `orphan` which has no rules), then run `0002_variants.up.sql` the same way. Assert `game_variants` holds `(chess, chess, 'Chess', 0)` and `(orphan, orphan, 'orphan', 0)` and `game_pages.name` is `Chess` and `orphan`.
  - `the_down_migration_removes_what_the_up_migration_added`: run up then `0002_variants.down.sql`; assert `game_variants` no longer exists and `game_pages` has no `name` column.
  - `save_game_stores_the_page_and_its_variants_together`: run `migrate`, `save_game` a page `go` with variants `go` (label `9x9`), `go-13` (label `13x13`, skin `{"board":{"x":1}}`, sort 1); assert `VariantRepo::get` / `for_game` / `all` return them in `sort_order, slug` order and `get` of an unknown slug is `None`, and `PageRepo::get` returns the page with `name` and `preview_variant` round-tripped.
  - `saving_a_game_moves_a_variant_that_belonged_to_another_game`: save pages `go-13` (single variant `go-13`) and `go` (variants `go`, `go-13`); assert `VariantRepo::get(go-13).game == go`, and that the page row `go-13` is gone (`PageRepo::get` is `None`) while `go` remains.
  - `a_variant_moved_out_of_a_game_that_keeps_others_does_not_delete_that_game`: save game `a` with variants `a`, `b`; save game `c` with variant `b`; assert page `a` still exists with variant `a`.
  - `saving_a_game_twice_changes_nothing_and_keeps_its_cover`: save, `set_cover_image(go, "covers/go/a.jpg")`, save again with `cover_image: None`; page and variants equal the first save, cover kept.
  - `open_lobbies_spans_several_variants_newest_first`: seed two lobbies in `chess` and `checkers` (built with `GameInstance::new_lobby` from stored specs and `InstanceRepo::create`), call `open_lobbies(&[chess, checkers], cutoff, 50)`; assert both, newest first, and with `&[chess]` only the chess one.

- [ ] **Step 2: Run to verify they fail.** Run: `cargo test -p modern-chess-game --test variants`. Expected: compile errors (`Variant`, `save_game`, `VariantRepo` missing).

- [ ] **Step 3: Implement.**
  - `0002_variants.up.sql`: `ALTER TABLE game_pages ADD COLUMN name TEXT, ADD COLUMN preview_variant TEXT;` then `CREATE TABLE game_variants (slug TEXT PRIMARY KEY, game_slug TEXT NOT NULL REFERENCES game_pages (slug), label TEXT NOT NULL, sort_order INTEGER NOT NULL DEFAULT 0, skin JSONB);` (no cascade: deleting a page that still has variants must fail), `CREATE INDEX idx_game_variants_game ON game_variants (game_slug, sort_order, slug);`, backfill `name` with `COALESCE((SELECT s.name FROM game_specs s WHERE s.slug = p.slug ORDER BY s.version DESC LIMIT 1), p.slug)`, then `INSERT INTO game_variants SELECT slug, slug, name, 0, NULL FROM game_pages`. Header comments say what each table means, in the style of `0001_init.up.sql`. The down file drops `game_variants`, then both columns.
  - `domain/variant.rs` as in Interfaces; re-export from `domain/mod.rs`. `GamePage` gets the two fields.
  - `PageRepo`: add `name, preview_variant` to `COLUMNS`, the insert, and the update list (both replaced on conflict, no keep rule); `page_from_row` reads them (`preview_variant` via `Slug::parse` on an `Option<String>`). Factor the insert into a private `async fn upsert_page(conn: &mut PgConnection, page: &GamePage)` so `save_game` can run it in a transaction. `save_game`, in one transaction: `upsert_page`; for each variant `INSERT ... ON CONFLICT (slug) DO UPDATE SET game_slug, label, sort_order, skin`; then `DELETE FROM game_pages p WHERE p.slug = ANY($1) AND p.slug <> $2 AND NOT EXISTS (SELECT 1 FROM game_variants v WHERE v.game_slug = p.slug)` binding the variant slugs and the game slug.
  - `VariantRepo` in `store/variant.rs` following `PageRepo`'s shape; export it from `store/mod.rs`; add the field to `GameState::new`.
  - `InstanceRepo::open_lobbies`: `slug = ANY($1)` with a `Vec<String>` bind.
  - Update `GamePage` literals in `tests/{store,lobbies,catalog}.rs` with `name: None, preview_variant: None`. The `open_lobbies` caller in `http/catalog.rs` passes `&[slug.clone()]` for now so the crate compiles.

- [ ] **Step 4: Run to verify they pass.** Run: `cargo test -p modern-chess-game`. Expected: all pass, including every test that existed before.

- [ ] **Step 5: Commit.**
```bash
cd /Users/frank/Projects/personal/modern-chess
rustfmt --edition 2021 crates/game/src/domain/variant.rs crates/game/src/domain/page.rs crates/game/src/domain/mod.rs crates/game/src/store/variant.rs crates/game/src/store/page.rs crates/game/src/store/instance.rs crates/game/src/store/mod.rs crates/game/src/http/mod.rs crates/game/tests/variants.rs crates/game/tests/store.rs crates/game/tests/lobbies.rs crates/game/tests/catalog.rs
git add crates/game/migrations/0002_variants.up.sql crates/game/migrations/0002_variants.down.sql crates/game/src crates/game/tests
git commit -m "feat(game): store games and their variants"
```
(Message body: one paragraph saying why; end with the Co-Authored-By line.)

---

### Task 2: The seed reads variants, and Go is regrouped

**Files:**
- Modify: `crates/game/src/seed.rs`, `crates/game/pages/go.json`
- Delete (via `git rm`): `crates/game/pages/go-13.json`, `crates/game/pages/go-19.json`
- Test: `crates/game/tests/variants.rs`, `crates/game/tests/catalog.rs`

**Interfaces:**
- Consumes: `PageRepo::save_game`, `Variant::single`, `SpecRepo::latest`.
- Produces: `seed::run_with` keeps its signature. `SeedReport` gains `pub variants: usize`. A page file may carry `"name"`, `"preview_variant"` and `"variants": [{ "slug", "label", "skin"? }]`.

- [ ] **Step 1: Write the failing tests** in `tests/variants.rs`, calling `seed::run_with` with inline JSON for the page files and the real `CHESS`/`GO` rules files (`include_str!("../../engine/specs/go.json")` etc.):
  - `a_page_file_without_variants_is_one_variant_named_after_its_rules`: file `{slug: chess, tagline, description}` → `VariantRepo::for_game(chess)` is one variant `chess` labelled `Chess`; `PageRepo::get(chess).name` is `Some("Chess")`.
  - `a_page_file_with_variants_saves_them_in_order_with_skins`: variants `go` (`9x9`), `go-13` (`13x13`, skin object), game `name` `"Go"`, `preview_variant` `"go"`; assert rows, `report.variants == 2`, `report.pages == 1`.
  - `seeding_a_page_with_a_preview_variant_that_is_not_listed_skips_that_page`: `preview_variant: "nope"` → `report.skipped.len() == 1`, no page stored, other files unaffected.
  - `duplicate_variant_slugs_in_one_file_skip_the_page`.
  - `the_old_go_pages_become_variants_of_go_and_vanish_as_pages`: first seed three separate single-variant page files `go`, `go-13`, `go-19` (the old layout, with the three Go rules), set a cover on `go`, create a lobby in `go-13` through `InstanceRepo`; then seed only the new `go` file with three variants. Assert pages = exactly `[go]`, variants of `go` = `go, go-13, go-19`, cover on `go` kept, the `go-13` lobby still loads by id, and `report.skipped` is empty.
  - `seeding_the_regrouped_game_twice_changes_nothing`: seed the new layout twice; `PageRepo::list` and `VariantRepo::all` equal after each.
  - In `tests/catalog.rs`, update `embedded_seed_files_publish_the_launch_games`: the listed games become `chess, checkers, ataxx, chaturanga, shogi, atomic, xiangqi, go` (and keep its per-game checks). It still fails until the HTTP task, so mark it expected-failing only through the order of work: it is run for real in Task 3.

- [ ] **Step 2: Run to verify they fail.** Run: `cargo test -p modern-chess-game --test variants`. Expected: the new seed tests FAIL (variants ignored by the seed), the rest pass.

- [ ] **Step 3: Implement** in `seed.rs`:
  - A private `PageFile { #[serde(flatten)] page: GamePage, #[serde(default)] variants: Vec<VariantFile> }` and `VariantFile { slug: Slug, label: String, #[serde(default)] skin: Option<Value> }`.
  - For each page file: parse; with no variants use `Variant::single(&page.slug, &label)` where `label` is the name of `specs.latest(&page.slug)` (this runs after the spec files, so it is there) or the slug text when there are no rules. Otherwise build `Variant`s with `game` = the page's slug and `sort_order` = position in the list.
  - Reject, via `skip(...)` with a reason: a duplicate variant slug; a `preview_variant` not among the variants; an empty `variants` array written explicitly counts as "no variants" (the default).
  - When the page has no `name`, set it to the display name of the preview variant's (else first variant's) rules from `specs.latest`; leave `None` when there are no rules.
  - Call `pages.save_game(&page, &variants)`; count `report.pages += 1`, `report.variants += variants.len()`.
  - `go.json`: `"name": "Go"`, `"preview_variant": "go"`, `"variants"` for `go` (`9x9`, no skin: the page skin is the 9x9 one), `go-13` (`13x13`, a full skin copied from `go-13.json`), `go-19` (`19x19`, full skin from `go-19.json`). Text: tagline and description rewritten to cover the three boards, `how_to_play` text from `go.json` with the sentence about "a 9 by 9 board" replaced by one that names all three sizes. `sort_order` stays 7. Do not change the stone art or board colours. Then `git rm` the other two files. Use `python3 -I` to build the JSON so it stays valid.

- [ ] **Step 4: Run to verify they pass.** Run: `cargo test -p modern-chess-game --test variants`. Expected: PASS. Then `cargo test -p modern-chess-game --lib` to check the seed's unit tests, if any, still pass.

- [ ] **Step 5: Commit.** Stage `crates/game/src/seed.rs`, `crates/game/pages/go.json`, the two removed files (already staged by `git rm`), `crates/game/tests`. Message: `feat(game): seed games with variants and regroup Go`.

---

### Task 3: The API answers in games and variants

**Files:**
- Modify: `crates/game/src/http/catalog.rs`, `crates/game/src/http/lobbies.rs`, `crates/game/src/http/play.rs`
- Test: `crates/game/tests/variants.rs`, `crates/game/tests/catalog.rs`, `crates/game/tests/lobbies.rs`

**Interfaces:**
- Consumes: `GameState.variants`, `InstanceRepo::open_lobbies(&[Slug], ..)`.
- Produces (JSON, all additive):
  - `GET /catalog` entry: `name` (the game's), `variants: [{slug, label}]` (readable ones only).
  - `GET /catalog/{game}`: `page` (with resolved `name` and `preview_variant`), `spec` (the preview variant's rules, same shape as today), `preview_variant: slug`, `variants: [{slug, label, name, seats, skin}]` (`skin` null unless it differs from `page.skin`), `open_lobbies` where each has `variant` and `variant_label`, covering every readable variant, newest first, at most 50.
  - `POST /catalog/{variant}/lobbies`: same shape as today; resolved through `game_variants`.
  - `GET /instances/{id}`, the join response and command snapshots gain `game: slug` and `variant_label: string | null`.

- [ ] **Step 1: Write the failing tests** in `tests/variants.rs` against the embedded seed (`seed::run`, like `lobbies.rs`'s `world`), plus the helper shapes from `catalog.rs`:
  - `the_front_page_lists_go_once_with_its_three_variants`: `/catalog` slugs contain `go` and neither `go-13` nor `go-19`; the `go` entry's `name` is `Go` and `variants` is `[{slug:go,label:9x9},{slug:go-13,label:13x13},{slug:go-19,label:19x19}]`.
  - `the_game_page_describes_every_variant`: `/catalog/go`: `preview_variant` is `go`; `variants[0].skin` is null, `variants[1].skin` is an object; each variant has `name` (`Go 13x13` is the rules' name, assert `variants[1].name` equals what `/catalog`-less `SpecRepo::latest(go-13).name` gives), `seats` `["BLACK","WHITE"]`; `page.name` is `Go`; `spec.name` is the preview variant's.
  - `variant_slugs_are_not_game_slugs`: `GET /catalog/go-13` is 404 with `{"error":"game not found"}`.
  - `a_lobby_can_be_opened_in_a_variant_that_has_no_page`: `POST /catalog/go-13/lobbies` with a token → 201; `GET /instances/{id}` has `slug` `go-13`, `game` `go`, `variant_label` `13x13`.
  - `the_game_page_lists_lobbies_of_all_its_variants`: open one lobby in `go` and one in `go-19`; `GET /catalog/go` `open_lobbies` has both, newest first, with `variant` and `variant_label` set; `GET /catalog/chess` only lists chess lobbies and its lobbies carry `variant` `chess`, `variant_label` `Chess`.
  - `a_single_variant_game_has_one_variant_and_no_variant_skin`: chess detail `variants` has one entry, `skin` null.
  - `a_variant_that_cannot_be_read_is_left_out_and_the_game_stays`: store a game with two variants where one has rules the engine cannot read (insert a `game_specs` row like `tests/catalog.rs` does); the list and detail show only the readable one, and `preview_variant` falls back to it when the named one is the unreadable one.
  - `a_game_with_no_readable_variant_is_left_out`: that game is absent from `/catalog` and `/catalog/{game}` is 404.
  - `a_match_whose_variant_row_is_missing_still_answers`: delete the `game_variants` row of an instance's slug in SQL; `GET /instances/{id}` is 200 with `game` equal to the instance's `slug` and `variant_label` null.
  - Update `tests/lobbies.rs` `world`: the extra `4player-chess` page is saved with `PageRepo::save_game` and `Variant::single`, so the existing four-seat tests pass unchanged. Update `tests/catalog.rs` `store_page` the same way (single variant named after the slug).

- [ ] **Step 2: Run to verify they fail.** Run: `cargo test -p modern-chess-game --test variants --test catalog --test lobbies`. Expected: FAIL on the new fields and on `go-13` listing; compile otherwise.

- [ ] **Step 3: Implement.**
  - `catalog.rs`: replace `Playable` with a `Game { page: GamePage, name: String, variants: Vec<PlayableVariant>, preview: usize }` where `PlayableVariant { variant: Variant, spec: GameSpec, seats: Vec<SeatName> }`. `async fn playable_game(state, page: GamePage, variants: Vec<Variant>) -> Result<Option<Game>, StoreError>` keeps the old per-variant check (latest spec exists and `spec.seats()` parses; otherwise `warn!` and omit that variant) and returns `None` with a `warn!` when none are left. `name` is `page.name` or the preview spec's name. `preview` is the index of `page.preview_variant` among the readable ones, else 0. `list` loads `variants.all()` once and groups by game slug; `detail` uses `variants.for_game`. `find_playable` becomes `find_game(state, game: &Slug)`. `GameDetail.page` has `name` and `preview_variant` filled with the resolved values. The order of the list stays `(sort_order, name)`, now on the game's name.
  - Lobbies in the detail: call `open_lobbies` with every readable variant's slug; `LobbySummary::new(&GameInstance, &Variant)` adds `variant`, `variant_label` (find the variant in the readable list by `instance.slug()`).
  - `lobbies.rs`: `create` takes `Path(variant)`, resolves `state.variants.get`, then loads `specs.latest` and checks `seats()` (a shared helper `find_variant(state, &Slug) -> Result<Option<PlayableVariant>, StoreError>` in `catalog.rs`), else 404 `game not found`. `InstanceSummary` gets `game: Slug` and `variant_label: Option<String>`; build it with `InstanceSummary::new(&GameInstance, Option<&Variant>)` (`game` is the variant's game, or the instance's own slug when there is none). `join` and `play.rs` (`show`, `command`) look the variant up with `state.variants.get(instance.slug())` before building the summary or snapshot. In `command`, do the lookup before the lock, with the rules read, so a second pool connection is never requested while the row is locked.

- [ ] **Step 4: Run to verify they pass.** Run: `cargo test -p modern-chess-game`. Expected: everything passes, including `embedded_seed_files_publish_the_launch_games` from Task 2. Then `cargo clippy -p modern-chess-game --all-targets -- -D warnings` and `cargo test -p modern-chess-game-api` (the app crate that seeds on start), expecting a clean run.

- [ ] **Step 5: Commit.** Format and stage the touched files by name. Message: `feat(game): serve games with their variants`.

---

### Task 4: Roadmap note and a real-upgrade check

**Files:**
- Modify: `docs/superpowers/plans/2026-11-09-roadmap.md`

- [ ] **Step 1: Update the roadmap.** In "3. Go", replace "the three catalogue pages (9x9, 13x13, 19x19)" with a sentence that Go is one game with three variants, and add a short "Games and variants" paragraph under it: the game is a container, variants are what is played, user variants will be new variants under a game, and the second half (the web) is the next pull request. Plain sentences, no em-dashes.

- [ ] **Step 2: Check the upgrade on a copy of the real database.** The user's `game` database in the dev container holds the old pages. Do not touch it. Create a scratch copy with `docker exec modern-chess-db createdb -U modern_chess -T game game_variants_check` (a copy needs no other connection on `game`; if that fails because the user's server is connected, dump and restore instead: `pg_dump | psql` into the new database). Run the migration and the seed against the copy with a short throwaway program or an ignored test that uses `DATABASE_URL` pointing at it, and assert: one `go` game, three variants, no `go-13` or `go-19` page, existing `go-13` instances unchanged. Drop the copy afterwards with `dropdb` (name the database, it is the one this step created). Record the outcome in the final summary. If a copy cannot be made without touching the user's database or servers, say so and skip: the tests in Task 2 already cover this path.

- [ ] **Step 3: Whole-crate verification and commit.** Run `cargo test -p modern-chess-game -p modern-chess-game-api` and `cargo clippy -p modern-chess-game -p modern-chess-game-api --all-targets -- -D warnings`. Commit the roadmap: `docs: note games and variants in the roadmap`.

---

## Handoff

After the final review and its fixes: give the user `! git -C /Users/frank/Projects/personal/modern-chess push -u origin feature/game-variants`, then open the PR with `gh pr create --base main` (body in the user's PR template; no AI attribution in the PR text). The PR description must say that between this PR and the web PR the old web app is partly wrong for Go, so the two merge back to back before servers are restarted for anyone. Then write the PR 2 plan.

## Self-review notes

- Spec coverage: model, listing rule, page files, data and seed, API, "nothing existing changes", missing-variant fallback, and the interim gap are each owned by a task above. The web half is PR 2, deliberately planned later.
- Types consistent across tasks: `Variant`, `Variant::single`, `PageRepo::save_game`, `VariantRepo::{get,for_game,all}`, `open_lobbies(&[Slug], ..)`, `GameState.variants`, `InstanceSummary::new`, `LobbySummary::new`.
- Two calls of mine, beyond the spec: the stale-page delete also requires the page to have no variants left (so a game can never be deleted from under its own variants), and `name` falls back at read time to the preview variant's rules name, so pages saved without a seed still show a name.
