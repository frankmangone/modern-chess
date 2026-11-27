# The builder's first slice: the workbench

Status: agreed with Frank on 2026-11-27, built in the same branch. Slice 1 of the road in
`docs/superpowers/plans/2026-11-27-ownership-and-credit.md` ("The builder"). The endgame is still a
visual editor (board, pieces, bags); the workbench is what it will be built on.

## The road

1. **Workbench (this slice).** Edit a spec, have the engine judge it, play it in the page. No account,
   nothing sent to a server.
2. **Drafts and sharing.** Drafts saved to an account, a private link to try one, playing it with a
   friend (which runs a user's spec on the game server: it needs the limits of ownership decision 9).
3. **Publish into the catalogue.** The ownership and credit decisions apply.
4. **A guided editor on the recipes** of `tools/spec-builder`.
5. **An agent inside the builder.** Bring-your-own-agent works from slice 1: the agent writes JSON,
   the person pastes it.

## What it is

A page at `/build`: a spec editor on the left, the game playing on the right.

- **The engine is the judge.** Typing pauses for 350 ms, then the text goes through `check_spec`
  (in `crates/session`, exported by `crates/wasm`): the same reader the game server uses. A good spec
  answers with its name and players; a bad one with a message in words, the line and column when the
  text is at fault, and the path to the key (`board.wide`, `pieces[2].moves[0]`, found with
  `serde_path_to_error`). A rule that contradicts another part of the spec has no place, and its
  message names the culprit. The editor marks the line and "Go to line N" selects it.
- **The playground.** The first time a game is good it starts, playable against yourself with the
  dice under it (the board element in demo mode). After that, changes wait for **Run**
  (Ctrl or Cmd + Enter), so typing never throws a game in progress away; the page says the board is
  still on the previous version. "What the engine sees" shows the position as the engine holds it.
- **Rules and Look.** Two tabs: the spec, and the board's skin (optional). The skin only has to be a
  JSON object; the board element is lenient about what is inside.
- **Where to start.** The first draft is March, the docs book's first game. "New draft" can also copy
  any game in the catalogue with its look, which is the quickest way to learn the format; "Open a
  file" brings a spec in; "Download" takes one out.
- **Drafts live in `localStorage`** (`hubbable.drafts`), written 400 ms after the last change. If the
  browser refuses, the page says so and points to the download.

## Safety of a hostile or broken spec

The engine runs in the person's own tab, so a spec that hangs it hurts only that person. The one
trap is a reload that starts the same game again. The page writes `hubbable.workbench-run` before it
runs a game and clears it when the board has drawn; if it is still set on the next visit, that draft
is not started and the page says why. `check_spec` itself is not guarded (a pathological spec could
stall the check); a worker would fix that and is not worth it yet.

## Not in this slice

Saving to an account, sharing, publishing, forking, any server change, a syntax-highlighting code
editor (a text box with line numbers is enough while the visual editor is the goal), and hints that
go beyond the engine's own messages.
