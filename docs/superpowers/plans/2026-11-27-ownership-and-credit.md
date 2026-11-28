# Ownership, credit and the road to the builder: decisions to settle

The social part of the product (accounts, communities, user-made games) and the builder meet at
one question: who owns a game, and who gets credit for it. This plan is the list of decisions to
settle **before** any code for user-made games, in the order the work will need them. Each one
has the options, a recommendation and what it changes.

Status: Frank rules on each decision; the answers go back into this file. **All ten are decided**
(27 Nov 2026). Decision 2 was amended (forks and variants are different things); the rest are the
recommendations as written, except decision 10, which is the recommendation adapted to the set-up that
was built since (see it). Decision 4 still needs a read by someone who knows licensing before it is
written into terms. State machines (below) are the way the decided parts are built.

## Order of work

1. **Login** (`2026-11-10-accounts.md`, slice 3: web sign-in). Real accounts, so a game has an
   author to point at. Settled direction; no decision below blocks it.
2. **Ownership and credit** (this plan), as a thin data model: an author on every game, a parent
   on every variant, a licence. No new screens beyond showing the credit.
3. **The builder.** A product, not a direction: draft, playtest in the browser against the wasm
   engine, publish. Its core (edit a spec, validate it, play it in "Try it") needs no account,
   but saving and publishing need step 2.
4. **Communities.** Groups, discussion and discovery around games, once there are user-made
   games to gather around.

The builder is in the middle on purpose: it needs an author and a lineage to publish into, and
communities need games to exist.

## What exists today (so the decisions start from the real shape)

- `users.id` owns matches and lobbies; a provider's id lives in `identities` and nowhere else
  (accounts plan). Games and communities are to be owned by `users.id` too.
- A game's rules are stored in the game server with a revision number and a hash. A match keeps
  the revision it started with, so a changed game does not change a match in progress. This is
  the base for "published versions" below.
- A game page can have a `parent` (a variant listed under its parent). Today that is editorial
  (Do-guti under Pong hau k'i), with no author and no source on the row.
- The catalogue is committed JSON, seeded at server start. User-made games would be database
  rows with no file behind them.

## Decisions

### 1. Who is the author of a traditional game?

Pong hau k'i, Go and Shogi belong to nobody. The catalogue we write (including spec-builder
games like Do-guti) is our rendering of a traditional game.

- **A. A `traditional` author kind**, shown as "Traditional", with an optional source note
  ("rules after Das Gupta, 1926, via Ludii"). The spec is credited to the platform.
- B. Credit every catalogue game to the platform account.
- C. No author on catalogue games; only user games carry one.

**Decided: A (27 Nov 2026).** It tells the truth (nobody owns the game, we wrote the spec) and gives variants
somewhere honest to point. Needs: an `author_kind` (`user`, `traditional`, `platform`) and a free
`source` note on a game.

### 2. What does a fork keep, and how is it different from a variant?

**Decided (27 Nov 2026), amended from the first recommendation.** A game built on another is one
of two things, not one:

- **A variant** is listed under the original game. It is found on the original's page and has
  no standing of its own in the catalogue.
- **A fork** is a different game. It has its own page and its own catalogue entry, and says
  "Based on X by Y".

Both keep the lineage: the parent and the revision it was made from are permanent, shown, and
never editable by the one who builds on it. The author of a game can switch building on it off
(`forkable`, which is the default case for a licence that allows it, decision 4).

A variant can become a fork when it has grown into a separate game. That move goes one way only:
a fork never goes back to being listed under someone else's game, since that would put it on
their page without their say. See "State machines".

Open details, to settle when the builder gets there: who may promote (the variant's author, as
proposed), and what the original's author can do about a variant listed under their game (hide
the listing; the variant then stands as a fork if forking is on, and is not published otherwise).

Options considered for the lineage rule: (A) parent permanent and shown, forking free; (B) same
plus the author can switch forking off, **chosen**; (C) forking only with the author's approval.
Needs: `parent_game_id`, `parent_revision` and `relation` on a game.

### 3. Is a published game immutable?

- **A. Publishing makes a revision, and a revision never changes.** Edits make a new revision.
  Running matches keep theirs (as today). Lobbies and new matches use the latest unless the
  author unpublishes.
- B. Authors may edit a published revision in place, with a warning.

**Decided: A (27 Nov 2026).** It already matches how the game server works, and it gives a fork
a fixed thing to point at ("based on revision 3"). A draft is the editable working copy; a
revision is made by publishing it and has no update path. Open follow-up: whether old revisions
stay playable for a rematch, or only for matches in progress.

### 4. Licence

- **A. The platform sets one licence for every published game**, stated at publish time (for
  instance CC BY-SA, so credit and share-alike hold on forks).
- B. The author picks from a short list (CC BY, CC BY-SA, CC0, all rights reserved).
- C. No licence stated.

**Decided: A for the first version (27 Nov 2026).** One rule is easier to explain, enforce and put in the
terms. B can come later by adding a column. The licence needs a read by someone who knows
licensing before it is written into terms: games as rules are a grey area (a game's rules are
generally not copyrightable, its text, art and spec file may be). Check before writing terms; do not
assume. CC BY-SA is the working example, not a ruling on the licence itself.

### 5. Credit for what is not the spec

A game has a spec, page text, a skin (colours, later images) and possibly a cover.

- **A. One game, one author, plus a list of contributors** who may be named (for instance a
  skin by someone else).
- B. One author per part, each credited on its own.
- C. One author, no contributors.

**Decided: C at first, A when skins from images land (27 Nov 2026)** (roadmap step 5.1). A credit model with
parts before there are parts is guesswork.

### 6. Moderation and removal

- Who can unpublish a game: the author, and platform staff.
- What happens to forks of an unpublished or removed game: they keep playing (matches and the
  published fork stay) but show "based on a game that was removed" with the name kept, unless
  the removal is for legal or safety reasons, in which case the fork is also reviewed.
- Reporting: a report button on a game, going to a queue only staff see.
- Roles beyond the author (moderators of a community) arrive with communities, not before.

**Decided: the above as written (27 Nov 2026).** It needs: `status` on a game (`draft`,
`published`, `unpublished`, `removed`, a state machine below) and a report table. No automatic
takedown.

With variants and forks as different things, the same rule reads: when the original of a **fork**
goes away, the fork stays and says "based on a game that was removed". A **variant** has lost the
page it was listed under, so it keeps its own page, reached directly, with the same label, until
its author promotes it to a fork or unpublishes it.

### 7. What happens to credit when an author deletes their account

The accounts plan keeps the user row so finished matches still resolve, and renames the handle
to `deleted-<hex>`.

- **A. Their games stay published, credited to "a deleted account".** Forks still show lineage.
- B. Their games are unpublished with the account.
- C. The author chooses at deletion.

**Decided: C, defaulting to A (27 Nov 2026).** Deleting an account should not break other people's forks,
but the person must be able to take their work down.

### 8. Who counts as the author of a game made with an agent

The product claims "anyone, or an agent, writes a game". An agent-written game published by a
user is that user's game; the page may carry "made with Claude" as a free tag, not as an author.

**Decided: that rule (27 Nov 2026).** Authorship is the account that publishes.

### 9. Limits on what a user game can do

Not about credit, but the same gate. Roadmap step 5.3 already lists them: board size, number of
pieces, a time budget per move and a review step. The decision to settle here is only whether
first publication of a game needs staff review, or only reports do.

**Decided: review of the first game of each new author, then none, plus the limits (27 Nov
2026).** This is the default for now and is to be re-evaluated later, once there are real
publications to judge it by.

### 10. Names and handles

Authors need a name to be credited under. Handles are generated today (`Guest-1234`).

- **Decided (27 Nov 2026), adapted to what was built.** The handle a person chooses at set-up (unique,
  with a reserved list, `PUT /me/handle`) is the name a game is credited under; there is no separate
  display name. The original recommendation (choose it at first publish) is overtaken: the set-up
  asks for it before anyone can publish. Renaming after the set-up stays refused until a rename flow
  is designed: credit lines and links would have to follow it, and deleting must clear it everywhere.

## What each decision changes in the data

New columns or tables, to be designed when decisions are made (not before):

- a game: `author_kind`, `author_user_id`, `source`, `parent_game_id`, `parent_revision`,
  `relation` (`original`, `variant`, `fork`), `licence`, `status`, `forkable`;
- a status history (who, when, from, to, reason), written with every status change;
- a report table;
- a handle on `users`, with the uniqueness rule and a reserved list (chosen at set-up today; see
  decision 10 for renaming).

## State machines

The rules above describe things that change over time, so they are built as explicit state
machines, not as columns and `if`s in handlers. This is a working rule from here on (27 Nov 2026).

**Form.**

- One enum per machine and one pure function, `transition(state, event, actor) -> Result<state,
  Refusal>`, which is the only place that knows the legal moves. No framework. The model is
  `InstanceStatus::can_become` in `crates/game/src/domain/status.rs`.
- The server owns the transition. The web app mirrors the table only to show or hide buttons.
- The database refuses an illegal state too (a `CHECK`, or a trigger on the status column), so a
  bug cannot write one.
- The transition table is test data: every pair is tested, allowed or refused. The documentation
  diagram is generated from the same table, so it cannot drift from the code.
- On the web, a reducer over a discriminated union, not XState.

**Game status** (`status`). The actor decides who may move it.

| From | To | Who |
|------|----|-----|
| `draft` | `in_review` | author (a new author's first game waits for review, decision 9) |
| `in_review` | `draft` | author (withdraws it), or staff (rejects it, with a reason) |
| `in_review` | `published` | staff (approves it; a revision is made, decision 3) |
| `draft` | `published` | author, once one of their games has been approved (a revision is made, decision 3) |
| `published` | `unpublished` | author, or staff |
| `unpublished` | `published` | author, unless staff removed it |
| `published`, `unpublished` | `removed` | staff |
| `removed` | `unpublished` | staff only (a reinstatement) |

A draft is private and is deleted, not removed. Every move writes a history row. A game in review
cannot be edited, so what staff read is what they approve. Which of `draft -> in_review` and
`draft -> published` an author gets is the policy (has one of their games been approved?), kept
outside the machine. Staff are the ids in `STAFF_USER_IDS` and nothing else; staff acting on their
own game act as its author, so nobody approves their own.

**Relation** (`relation`). Set when the game is made; `variant` to `fork` is the only change after
that, and it is never undone. `original` has no moves.

**Revision.** Not a machine: an immutable record made by publishing, with no update path. "Latest"
is derived, not stored. It is listed here so nobody adds an update.

**Existing implicit machines**, to convert when next touched, not in a sweep:

- Account set-up (`setup_pending`, a boolean in `crates/social` and the web app), when decision 10
  (renaming after set-up) lands.
- The workbench (checking, valid, stale, running, and the crash guard, spread over several
  signals), when slice 2 adds saved, unsaved and shared to it.
- Match status is already in this form (`InstanceStatus`). The rules engine is a machine too
  (state and action to state) and stays as it is; its history is below.

## What this plan does not decide

- The shape of the builder (its screens, what it edits, whether an agent writes specs inside
  it). That is a design of its own, after these.
- Communities (roles, rules, discovery).
- Payments, premium features, any revenue model.
- The product name (see Naming in `PRODUCT.md`).
- A history of moves kept by the engine for each game (a record of a match, replay, review).
  Frank raised it on 27 Nov 2026; it is its own piece of work, separate from this plan.

## Needs from Frank

Nothing is waiting for a ruling. Two things are Frank's to do outside the code: have someone who knows
licensing read decision 4 before it goes into terms, and design a rename flow when renaming handles
is wanted. Builder slice 2 (drafts) is built; slice 3a (publishing) can start.
