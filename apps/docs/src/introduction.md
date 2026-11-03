# Introduction

In Modern Chess, a game is not code. It is **one JSON file** that describes the board, the
players, the pieces and how they move. The engine reads that file and runs the game for you:
legal moves, turns, captures, check, promotion, and the end of the game.

That means you can invent a game, or recreate one you love, without touching the engine.

## How this guide works

The guide is a tutorial built from real games. Each chapter builds one game and introduces one or
two new ideas, so you never learn more than you can try right away.

Every chapter follows the same shape:

1. **The rule**, in plain words.
2. **The spec**, the JSON that expresses it.
3. **What the engine does** with it.
4. **Try it**, a command that lets you play what you just wrote.
5. **Now change it**, a small exercise.

All the example games live in the repository, and every one of them runs on the engine. Nothing
in this guide is pseudocode.

## What you need

- A text editor.
- The repository checked out, and [Rust](https://rustup.rs) installed, to play your games in the
  terminal.

You do not need to know Rust, and you do not need to be a programmer. If you can read a list in
square brackets, you can read a spec.

Ready? [Let's build a game.](01-first-game.md)
