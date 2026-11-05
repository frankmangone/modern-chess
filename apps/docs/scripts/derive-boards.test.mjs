import test from "node:test";
import assert from "node:assert/strict";
import { derive } from "./derive-boards.mjs";

const base = {
  name: "BASE",
  board: { dimensions: [8, 8] },
  conditions: [{ code: "END", type: "POSITION", check: { A: [[0, 7]], B: [[0, 0]] } }],
  players: [
    { name: "A", direction: [[1, 0], [0, 1]], starting_positions: [{ piece: "P", positions: [[0, 1]] }] },
    { name: "B", direction: [[-1, 0], [0, -1]], starting_positions: [{ piece: "P", positions: [[0, 6]] }] },
  ],
  pieces: [{ code: "P", name: "p", moves: [] }],
};
const recipe = {
  out: "small", base: "base", name: "SMALL",
  board: { dimensions: [3, 3] },
  zones: { END: { A: [[0, 2]], B: [[0, 0]] } },
  players: { A: [["P", [[1, 0]]]], B: [["P", [[1, 2]]]] },
};

test("replaces name, board, zones and each player's layout", () => {
  const spec = derive(recipe, base);
  assert.equal(spec.name, "SMALL");
  assert.deepEqual(spec.board, { dimensions: [3, 3] });
  assert.deepEqual(spec.conditions[0].check, { A: [[0, 2]], B: [[0, 0]] });
  assert.deepEqual(spec.players[0].starting_positions, [{ piece: "P", positions: [[1, 0]] }]);
  assert.deepEqual(spec.players[1].starting_positions, [{ piece: "P", positions: [[1, 2]] }]);
});

test("keeps everything it was not told to change", () => {
  const spec = derive(recipe, base);
  assert.deepEqual(spec.pieces, base.pieces);
  assert.deepEqual(spec.players[1].direction, [[-1, 0], [0, -1]]);
});

test("does not modify the base spec", () => {
  const before = JSON.stringify(base);
  derive(recipe, base);
  assert.equal(JSON.stringify(base), before);
});

test("a recipe without a board or zones leaves them alone", () => {
  const spec = derive({ out: "x", base: "base", players: recipe.players }, base);
  assert.deepEqual(spec.board, base.board);
  assert.deepEqual(spec.conditions, base.conditions);
});

test("fails loudly when a zone is not in the base spec", () => {
  assert.throws(() => derive({ ...recipe, zones: { NOPE: {} } }, base), /no condition NOPE/);
});

test("fails loudly when a player has no layout", () => {
  assert.throws(() => derive({ ...recipe, players: { A: recipe.players.A } }, base), /no layout for player B/);
});

test("fails loudly when the recipe names a player the base does not have", () => {
  assert.throws(() => derive({ ...recipe, players: { ...recipe.players, C: [] } }, base), /unknown player C/);
});
