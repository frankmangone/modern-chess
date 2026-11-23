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

// Every answer is one JSON object: a place, a code, a held piece or an action.
const at = (board, name) => board.choose(JSON.stringify({ place: name }));
const answer = (board, kind, value) => board.choose(JSON.stringify({ [kind]: value }));

const board = new Board(chess);
at(board, "4,1");
at(board, "4,3");
const view = JSON.parse(board.view());
assert.equal(view.turn, "BLACK");
assert.ok(view.pieces.some((p) => p.position === "4,3" && p.player === "WHITE"));

assert.throws(() => new Board("{"));
assert.deepEqual(view.choice.chosen, [], "a played turn leaves nothing chosen");
assert.throws(() => at(board, "nowhere"), "a place the board does not have must throw");
assert.throws(() => answer(board, "code", "QUEEN"), "a code nobody asked for must throw");
at(board, "4,6");
assert.equal(JSON.parse(board.view()).choice.start, "4,6", "choosing a piece marks where it stands");
board.cancel();
assert.equal(JSON.parse(board.view()).choice.start, null, "cancel puts the piece back");

// A piece taken in Shogi goes to the hand and can be dropped back through the same bundle.
const shogi = readFileSync(join(here, "..", "..", "..", "..", "crates", "engine", "specs", "shogi.json"), "utf8");
const game = new Board(shogi);
const play = (from, to) => {
  at(game, from);
  at(game, to);
};
play("6,2", "6,3");
play("2,6", "2,5");
at(game, "7,1");
at(game, "1,7"); // takes the bishop, landing in the promotion zone
answer(game, "code", "HORSE");
play("0,6", "0,5");
const before = JSON.parse(game.view());
const handOf = (view, player) => view.bags.find((bag) => bag.name === "HAND" && bag.owner === player)?.pieces;
assert.deepEqual(handOf(before, "SENTE"), { BISHOP: 1 });
assert.ok(before.choice.held.includes("BISHOP"), "the bishop in hand can be chosen");
answer(game, "held", "BISHOP");
const name = JSON.parse(game.view()).choice.places.find((place) => place.for === "PLACE").name;
at(game, name);
const after = JSON.parse(game.view());
assert.deepEqual(handOf(after, "SENTE"), {});
assert.ok(after.pieces.some((p) => p.position === name && p.code === "BISHOP" && p.player === "SENTE"));
assert.throws(() => answer(game, "held", "BISHOP"), "the hand is empty now");

// Go needs no hand: stones come from an action that places them, a surrounded stone is taken, and passing ends the game.
const go = new Board(readFileSync(join(here, "..", "..", "..", "..", "crates", "engine", "specs", "go.json"), "utf8"));
const stone = (name) => at(go, name);
stone("1,0"); // BLACK
stone("0,0"); // WHITE, in the corner
stone("0,1"); // BLACK takes it
const goView = JSON.parse(go.view());
assert.ok(goView.choice.actions.some((a) => a.name === "STONE" && !a.plays), "the stone is an action that only narrows");
assert.ok(!goView.pieces.some((p) => p.position === "0,0"), "the corner stone was captured");
assert.equal(goView.captured.WHITE.STONE, 1);
assert.ok(goView.choice.actions.some((a) => a.name === "PASS" && a.plays), "a pass plays at once");
answer(go, "action", "PASS");
answer(go, "action", "PASS");
assert.equal(JSON.parse(go.view()).status.state, "won", "two passes end the game");
assert.throws(() => answer(go, "action", "PASS"), "nothing can be played after the end");

// A map board: Pong hau k'i plays along its links, and the Catan sampler's three pieces each move.
const specs = join(here, "..", "..", "..", "..", "crates", "engine", "specs");
const pong = new Board(readFileSync(join(specs, "pong_hau_ki.json"), "utf8"));
const pongView = JSON.parse(pong.view());
assert.equal(pongView.shape, "map");
assert.equal(pongView.nodes.length, 5);
assert.equal(pongView.lines.length, 7);
at(pong, "bottom-left");
at(pong, "centre");
assert.equal(JSON.parse(pong.view()).turn, "BLACK");

const sampler = new Board(readFileSync(join(here, "..", "..", "src", "boards", "catan_sampler.json"), "utf8"));
const samplerView = JSON.parse(sampler.view());
assert.equal(samplerView.nodes.length, 145, "19 tiles, 54 corners, 72 sides");
for (const [piece, from, to] of [
  ["ROBBER", "2,2", "1,1"],
  ["SETTLEMENT", "1,1|1,2|2,2", "1,2|2,2|2,3"],
  ["ROAD", "2,2|2,3", "2,2|3,3"],
]) {
  at(sampler, from);
  at(sampler, to);
  const moved = JSON.parse(sampler.view());
  assert.ok(moved.pieces.some((p) => p.position === to && p.code === piece), `${piece} moved to ${to}`);
  assert.equal(moved.status.state, "playing", "a game with one player goes on");
}

console.log("smoke ok");
