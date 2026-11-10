# Games and variants, web (PR 2 of 2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The web app shows Go as one game: a variant picker on the game page, labelled lobbies from every variant, and a match page that names the game and variant, links back to the game and uses the variant's skin.

**Architecture:** Types follow the API that PR #50 shipped. The game page reads `detail.variants`; the match page loads the game by the match's `game` and finds its own variant in that detail. No new routes, no new API calls.

**Tech Stack:** Solid, @solidjs/router, vitest with @solidjs/testing-library (`pnpm test`, `pnpm typecheck` in `apps/web`).

**Spec:** `docs/superpowers/specs/2026-11-10-games-and-variants-design.md` (sections "What a person sees" and "Web"). Server half: `2026-11-10-games-and-variants.md`.

## Global Constraints

- The API answers (from #50): `GET /catalog` entries gain `variants: [{slug,label}]`; `GET /catalog/{game}` gains `variants: [{slug,label,name,seats,skin}]` (`skin` null when it is the game's), `preview_variant`, `page.name` and `page.preview_variant` (resolved), each `open_lobbies` entry has `variant` and `variant_label`; `GET /instances/{id}` has `slug` (the variant), `game` (the game's slug) and `variant_label` (null if unknown). `spec` is still the preview variant's rules.
- A game with one variant shows no picker and no variant labels; Create lobby then behaves as today.
- The picker starts on the preview variant. "Create lobby" opens a lobby in the picked variant (`POST /catalog/{variant}/lobbies`, the variant slug, never the game slug).
- Match title: "Game · variant" when the game has more than one variant, else the game's name alone. Back link goes to `/games/{game}`. Skin: the variant's when it has one, else the game's.
- Never use `spec.name` for a game's name (it is the rules' name, "Go 9x9"): use `page.name`.
- No em-dashes in text. Functional style, no `any`, named exports, `pnpm` only. Stage files by explicit list; never stage `images/` or `modern-chess.pen`. Commits are conventional and end with `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. Do not push.

## Review Focus

- A game page for a game whose preview variant is not the first still starts the picker on the preview.
- A lobby whose variant is missing from the detail does not break the list (it still renders, without a Join seat).
- A match whose game cannot be loaded still renders (name falls back to the slug, plain skin) as today.
- A single-variant match shows the game name only, never "Chess · Chess".

---

### Task 1: Game page: variants, picker, labelled lobbies

**Files:**
- Modify: `apps/web/src/api/types.ts`, `apps/web/src/api/game.ts` (rename the `createLobby` argument to `variant`), `apps/web/src/test/fixtures.ts`, `apps/web/src/pages/game.tsx`, `apps/web/src/components/{section-renderer,lobby-list}.tsx`, `apps/web/src/styles.css`
- Test: `apps/web/src/pages/game.test.tsx`, `apps/web/src/components/section-renderer.test.tsx`

**Interfaces:**
- Produces:
  - Types: `VariantLabel {slug,label}`, `VariantDetail {slug,label,name,seats,skin}`; `CatalogEntry.variants`; `GamePage.name: string`, `GamePage.preview_variant: string`; `GameDetail.preview_variant`, `GameDetail.variants`; `LobbySummary.variant`, `LobbySummary.variant_label`; `InstanceState.game`, `InstanceState.variant_label: string | null`.
  - `LobbyActions.create: (variant: string) => void`.
  - `LobbyList` props: `lobbies`, `variants: VariantDetail[]`, `preview: string`, `actions` (the `seats` prop goes away; a lobby's seats come from its variant).
  - fixtures: `detail()` gains `name`, `preview_variant`, `variants` (one variant, chess); `snapshot()` gains `game: "chess"`, `variant_label: "Chess"`; a `GO_VARIANTS` constant (`go` 9x9 with null skin, `go-13` 13x13 with a skin, `go-19` 19x19) and a `goDetail()` helper built on `detail()` with `page.name` "Go", `preview_variant` "go", lobbies carrying `variant`/`variant_label`.

- [ ] **Step 1: Write the failing tests.**
  - `game.test.tsx`: `the_hero_names_the_game_not_its_preview_rules` (goDetail: heading "Go", no "Go 9x9"); `a_single_variant_game_shows_no_picker_and_no_labels` (chess detail: no `radiogroup`, no element with `data-testid="lobby-variant"`, Create lobby posts to `/catalog/chess/lobbies`); `the_picker_lists_every_variant_and_starts_on_the_preview` (goDetail with `preview_variant: "go-13"`: three radios named 9x9, 13x13, 19x19, 13x13 `aria-checked="true"`); `create_lobby_opens_it_in_the_picked_variant` (click 19x19, Create lobby, expect `POST /api/game/catalog/go-19/lobbies`); `lobbies_of_every_variant_are_listed_with_their_label` (two lobbies, variants `go` and `go-19`, each row shows its label via `data-testid="lobby-variant"`); `joining_uses_a_seat_of_the_lobbys_own_variant` (a lobby of `go-13` with BLACK taken: Join posts `{seat:"WHITE"}`); `a_lobby_of_an_unknown_variant_still_renders_without_a_join_button`.
  - `section-renderer.test.tsx`: `the_preview_uses_the_preview_variants_own_skin_when_it_has_one` and keep the existing skin test (game's skin when the preview variant's is null). Update the existing tests' `LobbyActions` stub only if its type changed.
- [ ] **Step 2: Run to verify they fail.** Run: `cd apps/web && pnpm test`. Expected: type errors and the new tests failing; the other 135 still pass once types compile.
- [ ] **Step 3: Implement.**
  - Types and fixtures as in Interfaces.
  - `game.tsx`: `Hero` uses `page().name` for heading, breadcrumb and image alt. `lobbies.create` becomes `(variant) => void enter(... gameApi.createLobby(variant, token))`.
  - `section-renderer.tsx`: `preview` passes `skin = variants.find(v => v.slug === preview_variant)?.skin ?? page.skin`; `open_lobbies` passes `variants` and `preview`.
  - `lobby-list.tsx`: heading "Play" (was "Join a game"). When `variants.length > 1`, show a `role="radiogroup"` (aria-label "Variant") of buttons `role="radio"` with `aria-checked`, held in a local signal that starts on `preview`; Create lobby calls `actions.create(selected())`. With one variant, no picker and Create calls `actions.create(variants[0].slug)`. Each lobby row shows `<span data-testid="lobby-variant">` with `variant_label` only when `variants.length > 1`. `freeSeat` uses the lobby's variant seats (`[]` when unknown, so no Join button).
  - `styles.css`: small `.variant-picker`, `.variant-option` (selected state) and `.lobby-variant` rules in the style of the existing lobby rules.
- [ ] **Step 4: Run to verify they pass.** Run: `cd apps/web && pnpm test && pnpm typecheck`. Expected: all pass.
- [ ] **Step 5: Commit.** `feat(web): choose a variant on the game page` (stage the files above by name).

---

### Task 2: Match page: game and variant

**Files:**
- Modify: `apps/web/src/pages/play.tsx`, `apps/web/src/components/play-nav.tsx`
- Test: `apps/web/src/pages/play.test.tsx`

**Interfaces:**
- Consumes: `InstanceState.game`, `InstanceState.variant_label`, `GameDetail.variants`, `GamePage.name` (Task 1).
- Produces: `PlayNav` props `{ game: string; name: string; title: string; status: string }` (`game` is the game's slug for the back link, `name` the back link text, `title` the heading text before " — Match").

- [ ] **Step 1: Write the failing tests** in `play.test.tsx`, using `goDetail()` and a match `snapshot({ slug: "go-13", game: "go", variant_label: "13x13", ... })` with the game stub `GET /api/game/catalog/go`:
  - `the_match_page_loads_the_game_by_the_matchs_game_slug`: calls include `GET /api/game/catalog/go` and none for `catalog/go-13`.
  - `a_variant_match_is_titled_game_dot_variant_and_links_back_to_the_game`: heading contains "Go · 13x13", the back link's `href` ends `/games/go` and reads "Go".
  - `a_single_variant_match_is_titled_with_the_game_name_only`: the chess snapshot's heading is "Chess — Match", never "Chess · Chess".
  - `the_match_uses_the_variants_skin_and_falls_back_to_the_games`: for a `go-13` match the board receives the variant's skin; for a `go` match (null variant skin) the game's. Assert through what the board element is given (see how existing tests in this file check the skin; if none do, check the `skin` attribute of `modern-chess-board`/the rendered stone art that differs between the two fixtures).
  - `a_match_whose_game_cannot_be_loaded_still_renders`: game stub 500; the page still shows the board; title falls back to the slug.
- [ ] **Step 2: Run to verify they fail.** Run: `cd apps/web && pnpm test -- play`. Expected: the new tests fail.
- [ ] **Step 3: Implement.** In `play.tsx`: the game resource is keyed on `state()?.game`; `name = game()?.page.name ?? state()?.slug`; `title = variants.length > 1 ? `${name} · ${variantLabel}` : name` where `variantLabel` is the matching variant's label in the detail, else `state().variant_label`, else the slug; `skin = () => variant?.skin ?? game()?.page.skin ?? null` (variant found by `state().slug` in `game().variants`), used for `CapturedPanel`, `RemoteBoard` and `PlayIsland`. `PlayNav` links to `/games/${game}`.
- [ ] **Step 4: Run to verify they pass.** Run: `cd apps/web && pnpm test && pnpm typecheck && pnpm build`. Expected: all pass, build succeeds.
- [ ] **Step 5: Commit.** `feat(web): name the game and variant on the match page`.

---

### Task 3: Roadmap note

- [ ] Change the roadmap's "Games and variants" paragraph (`docs/superpowers/plans/2026-11-09-roadmap.md`) from "the web half is the next pull request" to say it is built. Commit `docs: games and variants are built end to end`.

## Handoff

After the final review: `! git -C /Users/frank/Projects/personal/modern-chess push -u origin feature/game-variants-web`, then open the PR (no AI attribution in the body). The user restarts the servers after merging.
