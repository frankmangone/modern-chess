import test from "node:test";
import assert from "node:assert/strict";
import { isTurned, normalizeSkin, pieceFace, zoneTint } from "../board-model.js";

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

test("an image entry is returned as an image", () => {
  const skin = normalizeSkin({ pieces: { ROOK: { WHITE: { image: "/skins/chess/wR.svg" }, BLACK: { image: "/skins/chess/bR.svg" } } } });
  assert.deepEqual(pieceFace(skin, "ROOK", "BLACK", players), { kind: "image", value: "/skins/chess/bR.svg" });
});

test("an image shared by both players is not tinted", () => {
  const skin = normalizeSkin({ pieces: { ROOK: { image: "/skins/chess/R.svg" } } });
  const face = pieceFace(skin, "ROOK", "WHITE", players);
  assert.equal(face.kind, "image");
  assert.equal(face.colour, undefined);
});

test("an image address that could leave this site or run code is dropped", () => {
  for (const image of [
    "javascript:alert(1)",
    "//evil.example/x.svg",
    "https://evil.example/x.svg",
    "data:image/svg+xml,<svg onload=alert(1)>",
    "/skins/../../secret.svg",
    "/skins/a b.svg",
    "/skins/x.html",
    "skins/relative.svg",
    "",
  ]) {
    const skin = normalizeSkin({ pieces: { ROOK: { image } } });
    assert.equal(pieceFace(skin, "ROOK", "WHITE", players).kind, "fallback", image);
  }
});

test("pieces face away only for the player at the top of the board", () => {
  const skin = normalizeSkin({ facing: true });
  assert.equal(isTurned(skin, "WHITE", players, false), false, "first player is at the bottom");
  assert.equal(isTurned(skin, "BLACK", players, false), true, "second player is at the top");
  assert.equal(isTurned(skin, "WHITE", players, true), true, "a flipped board puts the first player on top");
  assert.equal(isTurned(skin, "BLACK", players, true), false);
});

test("pieces stay upright without the facing option, or with more than two players", () => {
  assert.equal(isTurned(normalizeSkin({}), "BLACK", players, false), false);
  assert.equal(isTurned(normalizeSkin({ facing: "yes" }), "BLACK", players, false), false, "only true counts");
  assert.equal(isTurned(normalizeSkin({ facing: true }), "C", ["A", "B", "C", "D"], false), false);
});

test("a skin without a board option keeps the square board", () => {
  const { board } = normalizeSkin({});
  assert.equal(board.style, "squares");
  assert.deepEqual(board.breaks, []);
  assert.deepEqual(board.diagonals, []);
  assert.deepEqual(board.dots, []);
  assert.deepEqual(board.crosses, []);
  assert.deepEqual(board.labels, []);
});

test("a board with lines on the crossings is read in full", () => {
  const { board } = normalizeSkin({
    board: {
      style: "intersections",
      background: "#e8c98a",
      lines: "rgb(92, 59, 30)",
      breaks: [[4, 5]],
      diagonals: [[[3, 0], [5, 2]]],
      dots: [[3, 3]],
      crosses: [[1, 2]],
      labels: [{ text: "RIVER", x: 2, y: 4.5 }],
    },
  });
  assert.equal(board.style, "intersections");
  assert.equal(board.background, "#e8c98a");
  assert.equal(board.lines, "rgb(92, 59, 30)");
  assert.deepEqual(board.breaks, [[4, 5]]);
  assert.deepEqual(board.diagonals, [[[3, 0], [5, 2]]]);
  assert.deepEqual(board.dots, [[3, 3]]);
  assert.deepEqual(board.crosses, [[1, 2]]);
  assert.deepEqual(board.labels, [{ text: "RIVER", x: 2, y: 4.5 }]);
});

test("a board colour that could load something or run code is dropped", () => {
  const { board } = normalizeSkin({
    board: { style: "intersections", background: "url(https://example.com/a.png)", lines: "red; background: url(x)" },
  });
  assert.equal(board.background, null);
  assert.equal(board.lines, null);
});

test("an unknown board style, and bad points, are ignored", () => {
  const { board } = normalizeSkin({
    board: { style: "hexagons", dots: [[1], "x", [2, NaN], [4, 4]], breaks: [[1, 2, 3], [3, 4]], labels: [{ text: 5, x: 1, y: 1 }, { text: "ok", x: 1, y: 1 }] },
  });
  assert.equal(board.style, "squares");
  assert.deepEqual(board.dots, [[4, 4]]);
  assert.deepEqual(board.breaks, [[3, 4]]);
  assert.deepEqual(board.labels, [{ text: "ok", x: 1, y: 1 }]);
});

test("a colour for a player replaces the default tint of text and drawings", () => {
  const skin = normalizeSkin({ pieces: { SOLDIER: "兵" }, colours: { RED: "#c0392b", BLACK: "javascript:alert(1)" } });
  assert.equal(pieceFace(skin, "SOLDIER", "RED", ["RED", "BLACK"]).colour, "#c0392b");
  assert.notEqual(pieceFace(skin, "SOLDIER", "BLACK", ["RED", "BLACK"]).colour, "javascript:alert(1)");
});

test("the disc style marks text and drawings but not pictures", () => {
  const skin = normalizeSkin({ pieceStyle: "disc", pieces: { A: "兵", B: { image: "/skins/x/b.svg" } } });
  assert.equal(pieceFace(skin, "A", "RED", ["RED", "BLACK"]).disc, true);
  assert.equal(pieceFace(skin, "B", "RED", ["RED", "BLACK"]).disc, undefined);
  assert.equal(pieceFace(normalizeSkin({ pieces: { A: "兵" } }), "A", "RED", ["RED", "BLACK"]).disc, undefined);
});
