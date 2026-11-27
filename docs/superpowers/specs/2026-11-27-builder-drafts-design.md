# The builder's second slice: drafts saved to an account, and a private link

Status: agreed with Frank on 2026-11-27 (scope: "Yes, start slice 2"). Slice 2 of the road in
`2026-11-30-builder-workbench-design.md`. It builds on the decisions in
`docs/superpowers/plans/2026-11-27-ownership-and-credit.md`: games are rows with a `status` that
moves through an explicit state machine (decisions 3 and 6, "State machines").

## What it is

- A signed-in person's drafts are saved to their account, so they follow them between browsers.
- A draft can be **shared by a private link**. Anyone with the link can open the game and play it
  in their own browser, with no account. The link can be turned off, or replaced by a new one.
- Signed out, the workbench works as before: drafts stay in the browser.

## What it is not

Publishing, review of a first game (decision 9), forks and variants, playing a user's game with a
friend. Those run or list a stranger's spec on the server. Here the server **stores and hands back
text and never runs it**: the engine runs in the viewer's own tab, as in slice 1.

## Server (`crates/game`)

A game made by a person is a row in `user_games`, not a file in the catalogue.

- **Columns:** `id`, `author_id` (an opaque user id, no foreign key: the social module is another
  database), `name`, `rules` and `skin` (both **text**: a draft is work in progress and may not be
  valid JSON), `status`, `version`, `share_token`, `created_at`, `updated_at`.
- **Status** is `DRAFT`, `PUBLISHED`, `UNPUBLISHED` or `REMOVED`, as in the plan. Only `DRAFT` is
  reachable in this slice. The pure function `transition(status, event, actor)` in the domain holds
  the legal moves; a database trigger refuses any other pair, so a bug cannot write one. Every status
  change, and the creation, writes a row to `user_game_status_history` (who, when, from, to).
- **A published game is never edited** (decision 3): an update to a row that is not a draft is
  refused. Nothing publishes yet.
- **Limits**, checked in the domain and again by the database: name 1 to 80 characters, rules at
  most 128 KiB, look at most 32 KiB, at most 30 drafts per person.
- **Changing a draft** carries the `version` the client last saw. A different one means the draft was
  changed somewhere else; the answer is 409 and nothing is overwritten.
- **The private link** is 128 random bits, base64url, in `share_token` (unique). It is stored as is,
  so the author can see the link again; the cost is that a copy of the database holds the live links.
  Turning sharing off clears it. Only drafts can be shared, and the link works only while the game is
  a draft.

| Call | Who | Answers |
|------|-----|---------|
| `GET /drafts` | signed in | the person's drafts, newest first |
| `POST /drafts` | signed in | 201, the draft |
| `GET /drafts/{id}` | its author | the draft; **404 for anyone else**, so ids tell nothing |
| `PUT /drafts/{id}` | its author | the draft; 409 on a stale `version` |
| `DELETE /drafts/{id}` | its author | 204 |
| `PUT /drafts/{id}/share` | its author | `{ token }`, made once, then the same |
| `DELETE /drafts/{id}/share` | its author | 204 |
| `GET /shared/{token}` | anyone | `{ name, rules, skin }` and nothing about the author; `no-store` |

## Web (`apps/web`)

- **The workbench** gains an account side. A draft is either on this browser only or on the account.
  Signed in, a new draft goes to the account; drafts made before signing in stay local until the
  person chooses "Save to my account". Signed out, nothing changes.
- **The sync is an explicit machine** (`builder/sync.ts`): a reducer over `local`, `synced`, `dirty`,
  `saving`, `conflict`, `failed`, tested for every state and event, and the page's status line reads
  from it. Edits write the browser copy at once (as in slice 1) and the account 800 ms after the last
  change. A `failed` save keeps the work in the browser and retries; a `conflict` says the draft
  changed elsewhere and offers "Load that version" or "Keep mine".
- **Sharing:** "Share a link" on an account draft shows the link with a copy button, "Stop sharing"
  and "Make a new link". Nothing is shared by default.
- **The shared page** (`/build/shared/:token`): the game's name, the board playing, the rules under
  "See the rules" (read only) and "Make my own copy", which adds it to the person's drafts. A
  stranger's spec runs in the viewer's tab, so the page takes the slice 1 guard: if a run did not
  finish drawing last time, it does not start on its own.
- The other state of the workbench (checking, valid, stale, running) is left as it is; converting it
  is its own change.

## Not in this slice

Publishing and its review, forks, variants, the author's name on a shared game, a rate limit on the
link lookup, and searching drafts. Playing a shared game with a friend waits for publishing, which
needs the limits of decision 9.
