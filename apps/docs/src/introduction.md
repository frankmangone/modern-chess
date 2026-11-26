# Introduction

In Modern Chess, a game is not code. It is **one JSON file** that describes the board, the
players, the pieces and how they move. The engine reads that file and runs the game for you:
legal moves, turns, captures, check, promotion, dice, and the end of the game.

That means you can invent a game, or recreate one you love, without touching the engine. The
games on the site are made this way: chess, Shogi, Go, Checkers, Backgammon, Kalah and the rest are
all files, and the engine knows none of them by name.

## How this guide works

The guide is a tutorial built from real games. Each chapter builds one game and introduces one or
two new ideas, so you never learn more than you can try right away.

Most chapters follow the same shape:

1. **The rule**, in plain words.
2. **The spec**, the JSON that expresses it, a piece at a time.
3. **What the engine does** with it.
4. **Play it here**, a board in the page that runs the real engine on that spec.
5. **Try it**, a command that lets you play it in the terminal (when the game has a grid).
6. **Now change it**, a few small exercises.

All the example games live in the repository, and every one of them runs on the engine. Nothing
in this guide is pseudocode.

## The chapters

| | Chapter | New ideas |
|-|---------|-----------|
| **Pieces** | [1. Your first game](01-first-game.md) | board, players, turns, actions, pieces, tests and effects |
| | [2. Pieces that move](02-pieces-that-move.md) | capturing, leaping, sliding, check |
| | [3. Promotion and zones](03-promotion-and-zones.md) | named parts of the board, `if`, promotion |
| | [4. Conditions and memory](04-conditions-and-state.md) | regions, filters, flags, castling, en passant |
| **Endings** | [5. Winning and drawing](05-winning-and-drawing.md) | every way a game ends, counters, scores |
| | [6. Zones and blocking](06-zones-and-blocking.md) | blocked leaps, `until`, `threat_only` |
| **Beyond moving** | [7. The hand](07-the-hand.md) | bags, drops, optional promotion |
| | [8. Effects](08-side-effects.md) | explosions, cloning, converting |
| | [9. Forced captures and jumping](09-forced-captures.md) | `must`, stages |
| | [10. Placing stones](10-placing-stones.md) | placing, liberties, ko, counting |
| **Other boards** | [11. Maps](11-maps.md) | places and links, tiles |
| | [12. Bags and places that hold many](12-bags-and-many-pieces.md) | counts, sowing, Kalah |
| **Your own** | [13. Designing your own game](13-design-your-own.md) | a way of working, a lookup table, what goes wrong |

Read them in order the first time: each one uses what the one before taught. Later, chapter 13 is
the one to keep open. Two reference pages sit after the chapters: the
[spec reference](reference.md), which lists every field, and the
[known limitations](known-limitations.md).

## What you need

- A text editor.
- To read the guide, only a browser: the boards in each chapter run in the page.
- To play your own games in the terminal, the repository checked out and [Rust](https://rustup.rs)
  installed. (Games with a grid only. A map board is played in the web app.)

You do not need to know Rust, and you do not need to be a programmer. If you can read a list in
square brackets, you can read a spec.

Ready? [Let's build a game.](01-first-game.md)
