# The builder's third slice: publishing a game to the hub (3a)

Status: **a proposal**. The one open ruling (how a stranger's spec is run safely) is settled: Frank chose the sandbox, to be built first (`2026-11-27-engine-sandbox-design.md`, on `feature/engine-sandbox`). Slice 3 of
the road in `2026-11-30-builder-workbench-design.md`, built on the ten decisions of
`docs/superpowers/plans/2026-11-27-ownership-and-credit.md` (all decided) and the drafts of
`2026-11-27-builder-drafts-design.md`. Slice 3 is split: **3a** publishes a game and lets it be played
with a friend; **3b** is forks and variants (the `relation` column and the one-way move to a fork);
reporting and removal (decision 6) come after, as 3c.

## What it is

A person with an account opens a draft, presses **Publish**, and fills in what the game's page needs: a
tagline, a description, and the licence they agree to. The game appears in the catalogue under their
handle as its author, and anyone can open a lobby in it and play a friend, like any other game. A change
to it is a new revision; a published revision never changes (decision 3). The first game of each new
author is looked at by staff before it appears (decision 9); after one is approved, their games publish
at once.

## The state machine gains a step

Decision 9 needs a place for "waiting to be looked at", and the plan's table has none, so the machine
(`domain/lifecycle.rs`, the database trigger, and the table in the plan) gains **`in_review`**:

| From | Event | By | To |
|------|-------|----|----|
| `draft` | Submit | author | `in_review` |
| `in_review` | Withdraw | author | `draft` |
| `in_review` | Approve | staff | `published` |
| `in_review` | Reject (with a reason) | staff | `draft` |
| `draft` | Publish | author, only once they have an approved game | `published` |

`Publish` and `Submit` are both an author's move. Which one the server accepts is the **policy** for
that author (has a game been approved before?), kept outside the machine, because the machine knows
which moves exist and who may make them, not who has earned trust. The existing moves stay. Every move
writes a history row, with the reason on a rejection.

## Staff

There are no roles yet (they come with communities). A person is staff when their id is in
`STAFF_USER_IDS` of game-api (a comma-separated list), read at start. `Actor::Staff` is made only from
that list, never from a token claim a user could ask for. A small page `/review` lists the games
waiting, plays each in the browser, and approves or rejects with a reason. It is the only staff screen.

## What publishing writes

A published game is the same thing the catalogue already has, so the existing play code serves it
unchanged:

- `game_specs`: a new immutable version for the game's slug (the revision), the rules as they were
  when published. `active` marks the latest.
- `game_pages` and `game_variants`: the page (tagline, description, skin from the draft's look) and one
  variant, so the game is listed and has lobbies.
- `user_games`: gains `slug` (made once from the name, unique across the catalogue and the reserved
  names, never changed), `author_kind` (`user`), the tagline and description, and the licence accepted.
- `user_game_revisions`: one row per publish (game, revision number, the spec's id and hash, when, and the
  page text and look as published), so a revision is a fixed thing a fork can point at.

Publishing is one transaction: the status change, the history row and these writes together, or none.
Editing a published game starts a new draft revision of it; the live one keeps playing until the new one
is published, and running matches keep the spec they started with, as they do today.

## Credit

The page says "by <handle>" and names the licence. A published game is credited to the handle chosen at
set-up (decision 10), through the user id, so it follows the account. Deleting an account follows
decision 7 (the author chooses, defaulting to credit to "a deleted account"); that is built with the
deletion flow, not here.

## Limits and running a stranger's spec

Settled (27 Nov 2026): every move of every game runs in a WebAssembly sandbox with a fuel and memory
budget, built **before** this slice (`2026-11-27-engine-sandbox-design.md`). That removes the danger of a
hanging, huge or faulting spec: it costs one answer, not the server. What publishing adds on top:

1. **Static limits at publish**, checked through the engine (`Engine::check`): the text size (the draft
   limits already cap it at 128 KiB) and, from the engine's summary, the number of players. The engine already
   bounds a board's size. Further numbers (pieces, rules) are set from the biggest catalogue games with a
   margin, in one table in the code.
2. **A probe at publish**: play a number of random games on the spec through the sandbox and refuse one that
   exhausts its budget, faults or finds no legal move on its opening position. Cheap now that the sandbox
   exists, and it gives the reviewer a first look at how the game behaves.
3. **Review stays as decided** (decision 9): the first game of each new author. The earlier idea of also
   reviewing every revision until a sandbox existed is dropped, since the sandbox exists.
4. **Quarantine**: a published game whose calls keep stopping is taken down by the existing
   `published -> unpublished` move (staff), with the count of stopped calls per game shown on the review page.

## Web

- **Workbench:** a Publish button on an account draft, enabled when the engine reads the spec. It opens
  a short form (tagline, description, the licence, with its words in plain language) and says what
  happens next: published at once, or waiting for review. A draft in review shows that, with Withdraw;
  a rejected one shows the reason.
- **The catalogue and game page** need no new screens: a published game is a game. The page gains the
  credit line and the licence.
- **`/review`** for staff, as above.

## Order of building

1. The `in_review` machine and its trigger, `STAFF_USER_IDS`, `user_game_revisions`, the table columns.
2. Static limits, and the publish transaction into `game_specs`, pages and variants (the engine reads the rules first).
3. The probe.
4. The Publish form and the review page.
(The sandbox is already built: see above.)

## Not in this slice

Forks and variants (3b), reporting and removal, account deletion's effect on credit, a rename flow, a
licence choice per game, staff roles beyond the list, and a notification to the author when a game is
approved (the workbench shows the state when they next open it).
