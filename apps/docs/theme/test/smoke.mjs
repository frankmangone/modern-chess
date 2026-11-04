// Plays a real move through the compiled WebAssembly bundle.
// Run `apps/docs/build.sh` (or the wasm-pack step in it) first.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const pkg = join(here, "..", "pkg");
const { initSync, Board } = await import(join(pkg, "modern_chess_wasm.js"));

initSync({ module: readFileSync(join(pkg, "modern_chess_wasm_bg.wasm")) });

const chess = readFileSync(join(here, "..", "..", "..", "..", "crates", "engine", "specs", "chess.json"), "utf8");

const board = new Board(chess);
board.select(4, 1);
board.make_move(4, 3);
const view = JSON.parse(board.view());
assert.equal(view.turn, "BLACK");
assert.ok(view.pieces.some((p) => p.x === 4 && p.y === 3 && p.player === "WHITE"));

assert.throws(() => new Board("{"));
assert.throws(() => board.select(0, 0), "selecting on the wrong turn must throw");

console.log("smoke ok");
