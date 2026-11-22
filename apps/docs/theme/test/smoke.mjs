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
board.select("4,1");
board.make_move("4,3");
const view = JSON.parse(board.view());
assert.equal(view.turn, "BLACK");
assert.ok(view.pieces.some((p) => p.position === "4,3" && p.player === "WHITE"));

assert.throws(() => new Board("{"));
assert.throws(() => board.select("0,0"), "selecting on the wrong turn must throw");

// A piece taken in Shogi goes to the hand and can be dropped back through the same bundle.
const shogi = readFileSync(join(here, "..", "..", "..", "..", "crates", "engine", "specs", "shogi.json"), "utf8");
const game = new Board(shogi);
const play = (from, to) => {
  game.select(from);
  game.make_move(to);
};
play("6,2", "6,3");
play("2,6", "2,5");
game.select("7,1");
game.make_move("1,7"); // takes the bishop, landing in the promotion zone
game.promote("HORSE");
play("0,6", "0,5");
const before = JSON.parse(game.view());
assert.deepEqual(before.hand.SENTE, { BISHOP: 1 });
const name = before.drops.BISHOP[0];
game.drop_piece("BISHOP", name);
const after = JSON.parse(game.view());
assert.deepEqual(after.hand.SENTE, {});
assert.ok(after.pieces.some((p) => p.position === name && p.code === "BISHOP" && p.player === "SENTE"));
assert.throws(() => game.drop_piece("BISHOP", name), "the hand is empty now");

// Go needs no hand: stones come from a supply, a surrounded stone is taken, and passing ends the game.
const go = new Board(readFileSync(join(here, "..", "..", "..", "..", "crates", "engine", "specs", "go.json"), "utf8"));
const stone = (name) => go.drop_piece("STONE", name);
stone("1,0"); // BLACK
stone("0,0"); // WHITE, in the corner
stone("0,1"); // BLACK takes it
const goView = JSON.parse(go.view());
assert.deepEqual(goView.supply, ["STONE"]);
assert.ok(!goView.pieces.some((p) => p.position === "0,0"), "the corner stone was captured");
assert.equal(goView.captured.WHITE.STONE, 1);
assert.ok(goView.can_pass);
go.pass();
go.pass();
assert.equal(JSON.parse(go.view()).status.state, "won", "two passes end the game");
assert.throws(() => go.pass(), "nothing can be played after the end");

// A map board: Pong hau k'i plays along its links, and the Catan sampler's three pieces each move.
const specs = join(here, "..", "..", "..", "..", "crates", "engine", "specs");
const pong = new Board(readFileSync(join(specs, "pong_hau_ki.json"), "utf8"));
const pongView = JSON.parse(pong.view());
assert.equal(pongView.shape, "map");
assert.equal(pongView.nodes.length, 5);
assert.equal(pongView.lines.length, 7);
pong.select("bottom-left");
pong.make_move("centre");
assert.equal(JSON.parse(pong.view()).turn, "BLACK");

const sampler = new Board(readFileSync(join(here, "..", "..", "src", "boards", "catan_sampler.json"), "utf8"));
const samplerView = JSON.parse(sampler.view());
assert.equal(samplerView.nodes.length, 145, "19 tiles, 54 corners, 72 sides");
for (const [piece, from, to] of [
  ["ROBBER", "2,2", "1,1"],
  ["SETTLEMENT", "1,1|1,2|2,2", "1,2|2,2|2,3"],
  ["ROAD", "2,2|2,3", "2,2|3,3"],
]) {
  sampler.select(from);
  sampler.make_move(to);
  const moved = JSON.parse(sampler.view());
  assert.ok(moved.pieces.some((p) => p.position === to && p.code === piece), `${piece} moved to ${to}`);
  assert.equal(moved.status.state, "playing", "a game with one player goes on");
}

console.log("smoke ok");
