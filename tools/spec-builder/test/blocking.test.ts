import test from "node:test";
import assert from "node:assert/strict";
import { blockingGame, type BlockingGraph } from "../src/recipes/blocking.ts";

const square: BlockingGraph = {
  name: "SQUARE",
  points: [["a", 0, 0], ["b", 1, 0], ["c", 2, 0], ["d", 0, 1], ["e", 1, 1]],
  lines: [["a", "b"], ["b", "c"], ["a", "d"], ["d", "e"], ["b", "e"]],
  start: { WHITE: ["a", "c"], BLACK: ["d", "e"] },
};

test("a game with stones on its points slides and has no placing", () => {
  const spec = blockingGame(square);
  assert.deepEqual(spec.actions.map((each) => each.name), ["MOVE"]);
  assert.deepEqual(spec.players.map((player) => player.starting_positions.length), [1, 1]);
  assert.deepEqual(spec.endChecks.map((check) => check.then.outcome), ["DRAW", "LOSE"]);
});

test("a game that begins empty places first, and slides only with two stones down", () => {
  const spec = blockingGame({ ...square, start: "place" });
  assert.deepEqual(spec.actions.map((each) => each.name), ["PLACE", "MOVE"]);
  assert.deepEqual(spec.players.map((player) => player.starting_positions), [[], []]);
  assert.equal(JSON.stringify(spec.actions[1]?.do[0]), '{"test":"COUNT","in":{"region":"BOARD","where":{"owner":"ALLY"}},"min":2}');
});

test("a graph that cannot be played is refused when it is written", () => {
  assert.throws(() => blockingGame({ ...square, points: [...square.points, ["a", 3, 3]] }), /two points have the same name/);
  assert.throws(() => blockingGame({ ...square, lines: [["a", "z"]] }), /a line joins z, which is not a point/);
  assert.throws(() => blockingGame({ ...square, start: { WHITE: ["a", "z"], BLACK: ["d", "e"] } }), /a stone starts on z/);
  assert.throws(() => blockingGame({ ...square, start: { WHITE: ["a", "c"], BLACK: ["c", "e"] } }), /two stones start on the same point/);
  assert.throws(() => blockingGame({ ...square, points: [["a", 0, 0], ["b", 1, 0], ["c", 2, 0], ["d", 0, 1]], lines: [], start: { WHITE: ["a", "b"], BLACK: ["c", "d"] } }), /at least five points/);
});
