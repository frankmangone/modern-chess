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

// A piece taken in Shogi goes to the hand and can be dropped back through the same bundle.
const shogi = readFileSync(join(here, "..", "..", "..", "..", "crates", "engine", "specs", "shogi.json"), "utf8");
const game = new Board(shogi);
const play = (from, to) => {
  game.select(...from);
  game.make_move(...to);
};
play([6, 2], [6, 3]);
play([2, 6], [2, 5]);
game.select(7, 1);
game.make_move(1, 7); // takes the bishop, landing in the promotion zone
game.promote("HORSE");
play([0, 6], [0, 5]);
const before = JSON.parse(game.view());
assert.deepEqual(before.hand.SENTE, { BISHOP: 1 });
const [x, y] = before.drops.BISHOP[0];
game.drop_piece("BISHOP", x, y);
const after = JSON.parse(game.view());
assert.deepEqual(after.hand.SENTE, {});
assert.ok(after.pieces.some((p) => p.x === x && p.y === y && p.code === "BISHOP" && p.player === "SENTE"));
assert.throws(() => game.drop_piece("BISHOP", x, y), "the hand is empty now");

// Go needs no hand: stones come from a supply, a surrounded stone is taken, and passing ends the game.
const go = new Board(readFileSync(join(here, "..", "..", "..", "..", "crates", "engine", "specs", "go.json"), "utf8"));
const stone = (x, y) => go.drop_piece("STONE", x, y);
stone(1, 0); // BLACK
stone(0, 0); // WHITE, in the corner
stone(0, 1); // BLACK takes it
const goView = JSON.parse(go.view());
assert.deepEqual(goView.supply, ["STONE"]);
assert.ok(!goView.pieces.some((p) => p.x === 0 && p.y === 0), "the corner stone was captured");
assert.equal(goView.captured.WHITE.STONE, 1);
assert.ok(goView.can_pass);
go.pass();
go.pass();
assert.equal(JSON.parse(go.view()).status.state, "won", "two passes end the game");
assert.throws(() => go.pass(), "nothing can be played after the end");

console.log("smoke ok");
