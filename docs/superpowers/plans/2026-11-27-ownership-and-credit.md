# Ownership, credit and the road to the builder: decisions to settle

The social part of the product (accounts, communities, user-made games) and the builder meet at
one question: who owns a game, and who gets credit for it. This plan is the list of decisions to
settle **before** any code for user-made games, in the order the work will need them. Each one
has the options, a recommendation and what it changes.

Status: nothing here is decided. Frank rules on each; the answers go back into this file.

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

**Recommend A.** It tells the truth (nobody owns the game, we wrote the spec) and gives variants
somewhere honest to point. Needs: an `author_kind` (`user`, `traditional`, `platform`) and a free
`source` note on a game.

### 2. What does a fork keep?

A user forks a game (a variant of Go, or of another user's game).

- **A. The parent is permanent and shown.** A fork always lists "Based on X by Y", and the chain
  is kept. Forking is allowed by default.
- B. Same, plus the author can switch forking off for their game.
- C. Forking only with the author's approval.

**Recommend B.** Credit is automatic and cannot be removed, and the author can say "do not build
on this". Games under a licence that allows it (decision 4) are the default case. Needs:
`parent_game_id` and `parent_revision` on a game; never editable by the forker.

### 3. Is a published game immutable?

- **A. Publishing makes a revision, and a revision never changes.** Edits make a new revision.
  Running matches keep theirs (as today). Lobbies and new matches use the latest unless the
  author unpublishes.
- B. Authors may edit a published revision in place, with a warning.

**Recommend A.** It already matches how the game server works, and it gives a fork a fixed
thing to point at ("based on revision 3"). Open follow-up: whether old revisions stay playable
for a rematch, or only for matches in progress.

### 4. Licence

- **A. The platform sets one licence for every published game**, stated at publish time (for
  instance CC BY-SA, so credit and share-alike hold on forks).
- B. The author picks from a short list (CC BY, CC BY-SA, CC0, all rights reserved).
- C. No licence stated.

**Recommend A for the first version.** One rule is easier to explain, enforce and put in the
terms. B can come later by adding a column. This one needs a decision from Frank as the
product owner and probably a read by someone who knows licensing: games as rules are a grey
area (a game's rules are generally not copyrightable, its text, art and spec file may be).
Check before writing terms; do not assume.

### 5. Credit for what is not the spec

A game has a spec, page text, a skin (colours, later images) and possibly a cover.

- **A. One game, one author, plus a list of contributors** who may be named (for instance a
  skin by someone else).
- B. One author per part, each credited on its own.
- C. One author, no contributors.

**Recommend C at first, A when skins from images land** (roadmap step 5.1). A credit model with
parts before there are parts is guesswork.

### 6. Moderation and removal

- Who can unpublish a game: the author, and platform staff.
- What happens to forks of an unpublished or removed game: they keep playing (matches and the
  published fork stay) but show "based on a game that was removed" with the name kept, unless
  the removal is for legal or safety reasons, in which case the fork is also reviewed.
- Reporting: a report button on a game, going to a queue only staff see.
- Roles beyond the author (moderators of a community) arrive with communities, not before.

**Recommend** the above as written. It needs: `status` on a game (`draft`, `published`,
`unpublished`, `removed`) and a report table. No automatic takedown.

### 7. What happens to credit when an author deletes their account

The accounts plan keeps the user row so finished matches still resolve, and renames the handle
to `deleted-<hex>`.

- **A. Their games stay published, credited to "a deleted account".** Forks still show lineage.
- B. Their games are unpublished with the account.
- C. The author chooses at deletion.

**Recommend C**, defaulting to A. Deleting an account should not break other people's forks,
but the person must be able to take their work down.

### 8. Who counts as the author of a game made with an agent

The product claims "anyone, or an agent, writes a game". An agent-written game published by a
user is that user's game; the page may carry "made with Claude" as a free tag, not as an author.

**Recommend** that rule. Authorship is the account that publishes.

### 9. Limits on what a user game can do

Not about credit, but the same gate. Roadmap step 5.3 already lists them: board size, number of
pieces, a time budget per move and a review step. The decision to settle here is only whether
first publication of a game needs staff review, or only reports do.

**Recommend** review of the first game of each new author, then none, plus the limits.

### 10. Names and handles

Authors need a name to be credited under. Handles are generated today (`Guest-1234`).

- **Recommend** letting a person choose a handle and a display name when they first publish, with
  a uniqueness check and a reserved list. This is where the accounts plan's "choosing a handle
  is a later, separate change" lands, and deleting must then clear it everywhere.

## What each decision changes in the data

New columns or tables, to be designed when decisions are made (not before):

- a game: `author_kind`, `author_user_id`, `source`, `parent_game_id`, `parent_revision`,
  `licence`, `status`, `forkable`;
- a report table;
- a handle on `users`, with the uniqueness rule and a reserved list.

## What this plan does not decide

- The shape of the builder (its screens, what it edits, whether an agent writes specs inside
  it). That is a design of its own, after these.
- Communities (roles, rules, discovery).
- Payments, premium features, any revenue model.
- The product name (see Naming in `PRODUCT.md`).

## Needs from Frank

Rulings on decisions 1 to 4 and 7 first; the rest can follow the builder design. Decision 4
(licence) should be checked with someone who can speak to it before it is written into terms.
