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

// ----- Hex boards -----

import { hexCentre, hexColour, hexGeometry } from "../board-model.js";

const HEX_NEIGHBOURS = [[0, 1], [0, -1], [1, 1], [1, 0], [-1, 0], [-1, -1]];
const glinski = { width: 11, height: 11, shape: "hex", layout: "hexagon" };
const hexRhombus = { width: 11, height: 11, shape: "hex", layout: "rhombus" };
const near = (a, b) => Math.abs(a - b) < 1e-9;
const onHexagon = ([x, y]) => Math.abs(x - y) <= 5;
const cells = (view) => {
  const out = [];
  for (let x = 0; x < view.width; x++) for (let y = 0; y < view.height; y++) {
    if (view.layout === "rhombus" || onHexagon([x, y])) out.push([x, y]);
  }
  return out;
};

test("hex colours never repeat between neighbours", () => {
  for (const [dx, dy] of HEX_NEIGHBOURS) assert.notEqual(hexColour(5, 5), hexColour(5 + dx, 5 + dy));
});

test("the hexagon is upright and centred", () => {
  const bottom = hexCentre(glinski, 5, 0);
  const top = hexCentre(glinski, 5, 10);
  assert.ok(near(bottom.left, 0.5) && near(top.left, 0.5));
  const tops = cells(glinski).map(([x, y]) => hexCentre(glinski, x, y).top);
  assert.ok(near(bottom.top, Math.max(...tops)));
  assert.ok(near(top.top, Math.min(...tops)));
});

test("the rhombus lies wide", () => {
  const a = hexCentre(hexRhombus, 10, 0);
  const b = hexCentre(hexRhombus, 0, 10);
  assert.ok(near(a.top, b.top) && near(a.top, 0.5));
  const lefts = cells(hexRhombus).map(([x, y]) => hexCentre(hexRhombus, x, y).left);
  assert.ok(near(Math.min(a.left, b.left), Math.min(...lefts)));
  assert.ok(near(Math.max(a.left, b.left), Math.max(...lefts)));
  assert.ok(hexGeometry(hexRhombus).aspect > 1, "wider than tall");
});

test("flipping turns the board round", () => {
  for (const view of [glinski, hexRhombus]) {
    for (const [x, y] of [[5, 5], [2, 3], [7, 6]]) {
      const flipped = hexCentre(view, x, y, true);
      const turned = hexCentre(view, view.width - 1 - x, view.height - 1 - y, false);
      assert.ok(near(flipped.left, turned.left) && near(flipped.top, turned.top));
    }
  }
});

test("neighbouring hexes sit one cell apart and do not overlap", () => {
  const { cellWidth, aspect } = hexGeometry(glinski);
  const a = hexCentre(glinski, 5, 5);
  const b = hexCentre(glinski, 5, 6);
  // Straight up a file: one hex height apart, measured in box widths.
  const distance = Math.hypot(a.left - b.left, (a.top - b.top) / aspect);
  assert.ok(near(distance, cellWidth * (Math.sqrt(3) / 2)));
});

test("a mid colour is derived when a skin gives two", () => {
  const skin = normalizeSkin({ squares: { light: "#ffffff", dark: "#000000" } });
  assert.equal(skin.squares.mid, "#808080");
  assert.equal(normalizeSkin({ squares: { light: "#fff", dark: "#000", mid: "#123456" } }).squares.mid, "#123456");
});

test("per-player zone tints", () => {
  const skin = normalizeSkin({ zones: { SIDE_A: { RED: "#ff0000", BLUE: "#0000ff" }, GOAL: "#00ff00" } });
  const view = {
    players: ["RED", "BLUE"],
    zones: { SIDE_A: { RED: [[0, 3]], BLUE: [[3, 0], [0, 3]] }, GOAL: { RED: [[1, 1]] } },
  };
  assert.equal(zoneTint(skin, view, 0, 3), "#ff0000", "a cell in both players' zones takes the first player's tint");
  assert.equal(zoneTint(skin, view, 3, 0), "#0000ff");
  assert.equal(zoneTint(skin, view, 1, 1), "#00ff00", "a one-colour zone still tints");
  assert.equal(zoneTint(skin, view, 2, 2), null);
});

test("a corner shared across zones takes the first player's tint", () => {
  const skin = normalizeSkin({ zones: { SIDE_A: { RED: "#ff0000", BLUE: "#0000ff" }, SIDE_B: { RED: "#ff0000", BLUE: "#0000ff" } } });
  // Hex's corner (10, 0): BLUE's SIDE_A (y = 0) and RED's SIDE_B (x = 10).
  const view = {
    players: ["RED", "BLUE"],
    zones: { SIDE_A: { RED: [[0, 0]], BLUE: [[10, 0]] }, SIDE_B: { RED: [[10, 0]], BLUE: [[0, 10]] } },
  };
  assert.equal(zoneTint(skin, view, 10, 0), "#ff0000");
});

// ----- Review fixes -----

import { readFileSync } from "node:fs";
const css = readFileSync(new URL("../modern-chess-board.css", import.meta.url), "utf8");
const rule = (selector) => {
  const at = css.indexOf(`${selector} {`);
  return at < 0 ? null : css.slice(at, css.indexOf("}", at));
};

test("cell tones set only the background colour, so a zone tint survives on every tone", () => {
  for (const tone of [".mcb-light", ".mcb-mid", ".mcb-dark"]) {
    const body = rule(tone);
    assert.ok(body, `${tone} has a rule`);
    assert.ok(!/background\s*:/.test(body), `${tone} must not use the background shorthand`);
  }
});

test("a focused hex shows a ring the hex outline does not cut away", () => {
  const ring = rule(".mcb-hex:focus-visible::before");
  assert.ok(ring && /clip-path/.test(ring), "a hex-shaped ring");
  assert.ok(!/brightness/.test(rule(".mcb-hex:focus-visible") ?? ""), "not only a darker shade");
});

test("placing every cell of a large hex board stays fast", () => {
  const view = { width: 99, height: 99, shape: "hex", layout: "rhombus" };
  const geometry = hexGeometry(view);
  const started = performance.now();
  for (let x = 0; x < 99; x++) for (let y = 0; y < 99; y++) hexCentre(view, x, y, false, geometry);
  assert.ok(performance.now() - started < 500, "under half a second for 9,801 cells");
});
