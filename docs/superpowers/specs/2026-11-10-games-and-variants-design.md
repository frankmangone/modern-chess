# Games and variants

## Why

Go is on the front page three times: Go 9x9, Go 13x13 and Go 19x19, each with its own page
that repeats the same text. They are one game played on three boards. The same will happen
with every game that has variants, and later with variants that users propose.

So a game becomes a container, and its variants are the real games inside it. "Go" is one
card on the front page; opening it shows the three boards.

This is also the foundation for the roadmap's community variants: a user's variant is a new
variant under an existing game, not a new game beside it.

## What a person sees

**Front page.** One card per game. Go appears once.

**A game's page.** In the order the game's page lists them (the page already works this way):

- **How to play.** Written once for the whole game. Anything specific to a variant goes in
  that text, not in a per-variant text.
- **Try it.** A local board, playable against yourself with no server, on one variant: the
  simplest, which the game names (`preview_variant`; the first variant when it does not).
- **Play.** A variant picker (9x9, 13x13, 19x19) and a button that opens a lobby in the picked
  variant. Below it, one list of open lobbies of every variant of the game, each labelled
  with its variant, so a person joins whichever they like. A game with one variant shows no
  picker and no labels, exactly as today.

**A match.** The match page names the game and the variant ("Go · 13x13") and its back link
goes to the game's page. For a game with a single variant it shows the game's name alone.

## Goals

- Go on the front page once, with its three variants on its page.
- Text and look that are shared live once, on the game.
- Lobbies of all variants of a game in one list.
- No change to matches, lobbies, rules or covers that already exist.
- A page file for a game with a single variant does not get longer.

## Non-goals

- Moving Atomic into Chess, or any other regrouping of existing games. Only Go is regrouped.
- A different how-to-play text per variant, a cover per variant, or a variant switcher on
  "Try it".
- Any screen where users make games or variants. This design only makes room for them.
- Redirecting old `/games/go-13` links. They stop working; the app is not public yet.

## Anti-goals

- Existing matches changing meaning or becoming unreachable.
- Admins having to touch a database by hand to regroup a game: it is done by editing page
  files.

## The model

**Game** is the existing page row (`game_pages`). It owns the tagline, description, tags,
accent, cover, sections, `featured` and `sort_order`, plus:

- `name`: the game's own name ("Go"). Until now the name was taken from the rules ("Go 9x9").
  When a page file gives no name, the name of its single variant's rules is used, so the
  other games keep their names.
- `skin`: the base board skin the variants inherit.
- `preview_variant`: the slug of the variant that "Try it" uses.

**Variant** is a new table, `game_variants`:

| Column | Meaning |
|---|---|
| `slug` (primary key) | The slug of the variant's rules in `game_specs`. Unchanged for every existing game, which is why matches need no migration. Unique across all games. |
| `game_slug` | The game it belongs to (foreign key to `game_pages`). |
| `label` | What the picker shows: "9x9". For a single-variant game, the rules' display name. |
| `sort_order` | Order in the picker. |
| `skin` | A full skin for this variant, or empty to use the game's. |

A variant has no foreign key to `game_specs`, whose rows are versioned per slug. The newest
version of a variant's rules is the one used, as today.

Game slugs and variant slugs are separate namespaces that happen to coincide for single-variant
games, and for Go's 9x9 variant (`go`). The API and the code say "game" and "variant"
explicitly, never a bare "slug".

**Skins.** Go's three boards differ in only one thing: where the star points are. So a
variant may carry its own full skin, which replaces the game's (no merging). Go's variants
each carry a full board skin, and the shared stone art is repeated in each; the game's own
skin is the 9x9 one. The server sends a variant's skin only when it differs from the game's.

**Listing rule.** A game is listed if it has a page row and at least one variant whose rules
the engine can read. A variant that cannot be read is left out and logged; a game with none
left is left out and logged. (Today the same rule applies to a page.)

## Page files

`crates/game/pages/<game>.json` gains three optional fields:

```json
{
  "slug": "go",
  "name": "Go",
  "preview_variant": "go",
  "variants": [
    { "slug": "go",    "label": "9x9" },
    { "slug": "go-13", "label": "13x13", "skin": { ... } },
    { "slug": "go-19", "label": "19x19", "skin": { ... } }
  ]
}
```

A page file without `variants` means one variant whose slug is the page's slug and whose
label is its rules' display name. The six other games' files do not change. `go-13.json` and
`go-19.json` disappear; their text and skins fold into `go.json`.

## Data

Migration `0002` in the game crate:

1. Adds `name`, `preview_variant` to `game_pages`, and creates `game_variants`.
2. Backfills: every existing page becomes a game with one variant of the same slug, `name`
   from its newest rules' display name, `label` the same.

Regrouping Go is done by the seed, not by SQL that names Go. When the seed saves a game with
variants, in one transaction: it saves the page, saves each variant (a variant that already
belongs to another game moves to this one), then removes any page row whose slug is a variant
of a different game. That removes the old `go-13` and `go-19` pages and nothing else, and it
makes regrouping any game a matter of editing files. Removed pages had no covers.

Seeding stays safe to repeat. A page's cover is still kept when the file has none.

The seed only adds and moves. It never deletes a variant or a game because a file stopped
listing it (it deletes nothing today), so a variant dropped from a page file stays in the
game until someone retires it on purpose; retiring is not part of this design. The one
deletion is the stale page row described above. As a result every match keeps a variant row to
resolve through. If one is ever missing anyway, the match answers with `game` set to its own
slug and `variant_label` null, and the match page shows what it shows today.

## API (additive; nothing that exists is removed)

- `GET /catalog`: each entry gains `variants: [{slug, label}]`. `name` is the game's name.
- `GET /catalog/{game}`: the response gains
  - `variants`: `[{slug, label, name, seats, skin}]`, `skin` null when it is the game's;
  - `preview_variant`: a slug, while `spec` stays what it is now but is the preview
    variant's rules;
  - on each lobby in `open_lobbies`: `variant` (slug) and `variant_label`. The list covers
    every variant of the game, newest first, at most 50 in all.
- `POST /catalog/{variant}/lobbies`: unchanged in shape, but `{variant}` is resolved through
  `game_variants`, so a variant needs no page of its own.
- `GET /instances/{id}` and the lobby summaries gain `game` (slug) and `variant_label`.

## Web

- Types follow the API.
- Game page: "Try it" uses the preview variant's rules and skin; "Play" (the existing
  `open_lobbies` section, renamed in its heading only) gets the picker and per-lobby variant
  labels, shown only when the game has more than one variant. "Create lobby" opens a lobby in
  the picked variant. The picker starts on the preview variant.
- Match page: the page already loads the game's detail, now by the match's `game`. Title
  "Game · variant" when the game has more than one variant (from that detail), else the game's
  name; back link to `game`; the skin is the variant's, or the game's.
- Front page needs no change beyond the new catalogue (one Go).

## Order of work, two pull requests

1. **Server and data:** migration, repository, seed, page files, API, and docs.
2. **Web:** the page and match changes above.

Between the two the old web app is partly wrong for Go: its match page cannot find a 13x13 or
19x19 match's game (so it loses the skin, shows the raw slug as the title, and its back link
goes nowhere), and Go's page heading reads "Go 9x9". To avoid shipping that, the second
pull request is built straight after the first and the two are merged back to back, before
the servers are restarted for anyone. No temporary compatibility code is added for the gap.

## Left to the builder

Naming of types and files; the exact response field order; how the seed reads a page's
default name from the rules; test layout. Following the existing patterns (store per table,
`playable` check in the catalogue code, fixtures in the web tests) is expected.

## Not decided, deliberately

- Whether `spec` in `GET /catalog/{game}` is removed once the web no longer reads it.
- How a user's variant is proposed, reviewed and shown. That is the roadmap's community step.
