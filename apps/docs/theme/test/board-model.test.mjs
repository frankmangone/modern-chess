import test from "node:test";
import assert from "node:assert/strict";
import { normalizeSkin, pieceFace, cellRect, zoneTint } from "../board-model.js";

const players = ["WHITE", "BLACK"];

test("a per-player glyph wins", () => {
  const skin = normalizeSkin({ pieces: { ROOK: { WHITE: "♖", BLACK: "♜" } } });
  assert.deepEqual(pieceFace(skin, "ROOK", "BLACK", players), { kind: "glyph", value: "♜" });
});

test("an svg entry is returned as svg", () => {
  const skin = normalizeSkin({ pieces: { ROOK: { WHITE: { svg: "<svg/>" }, BLACK: { svg: "<svg2/>" } } } });
  assert.deepEqual(pieceFace(skin, "ROOK", "WHITE", players), { kind: "svg", value: "<svg/>" });
});

test("a single entry for both players is tinted by player", () => {
  const skin = normalizeSkin({ pieces: { ROOK: "♜" } });
  const white = pieceFace(skin, "ROOK", "WHITE", players);
  const black = pieceFace(skin, "ROOK", "BLACK", players);
  assert.equal(white.kind, "glyph");
  assert.equal(white.value, "♜");
  assert.ok(white.colour && black.colour && white.colour !== black.colour);
});

test("a missing piece falls back to its first letter, coloured by owner", () => {
  const skin = normalizeSkin({});
  const white = pieceFace(skin, "MARCHER", "WHITE", players);
  const black = pieceFace(skin, "MARCHER", "BLACK", players);
  assert.equal(white.kind, "fallback");
  assert.equal(white.value, "M");
  assert.notEqual(white.colour, black.colour);
});

test("a per-player entry missing one player falls back for that player", () => {
  const skin = normalizeSkin({ pieces: { ROOK: { WHITE: "♖" } } });
  assert.equal(pieceFace(skin, "ROOK", "BLACK", players).kind, "fallback");
});

test("a malformed skin never throws and yields defaults", () => {
  for (const bad of [null, undefined, "x", 5, [], { pieces: 5 }, { pieces: { ROOK: 7 } }, { squares: "red" }, { zones: [] }]) {
    const skin = normalizeSkin(bad);
    assert.equal(typeof skin.squares.light, "string");
    assert.equal(typeof skin.squares.dark, "string");
    assert.equal(pieceFace(skin, "ROOK", "WHITE", players).kind, "fallback");
  }
});

test("y = 0 is drawn on the bottom row", () => {
  const view = { width: 3, height: 5 };
  assert.deepEqual(cellRect(view, 0, 0), { col: 0, row: 4 });
  assert.deepEqual(cellRect(view, 2, 4), { col: 2, row: 0 });
});

test("zoneTint colours squares inside a named zone and nothing else", () => {
  const skin = normalizeSkin({ zones: { END: "#ffe9a8", OTHER: "#000000" } });
  const view = { zones: { END: { WHITE: [[0, 3], [1, 3]], BLACK: [[0, 0]] } } };
  assert.equal(zoneTint(skin, view, 1, 3), "#ffe9a8");
  assert.equal(zoneTint(skin, view, 0, 0), "#ffe9a8");
  assert.equal(zoneTint(skin, view, 1, 1), null);
});

test("zoneTint ignores zones the skin does not colour", () => {
  const skin = normalizeSkin({});
  const view = { zones: { END: { WHITE: [[0, 3]] } } };
  assert.equal(zoneTint(skin, view, 0, 3), null);
});
