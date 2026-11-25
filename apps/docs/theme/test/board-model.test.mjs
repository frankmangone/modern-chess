import test from "node:test";
import assert from "node:assert/strict";
import { stripeAngle, stripeColour, stripeGradient, zoneColours, zoneStripeColours, gridBounds, gridRows, groupPieces, isTurned, normalizeSkin, piecesLabel, pieceFace, zoneTint } from "../board-model.js";

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
  const view = { zones: { END: { WHITE: ["0,3", "1,3"], BLACK: ["0,0"] } } };
  assert.equal(zoneTint(skin, view, "1,3"), "#ffe9a8");
  assert.equal(zoneTint(skin, view, "0,0"), "#ffe9a8");
  assert.equal(zoneTint(skin, view, "1,1"), null);
});

test("zoneTint ignores zones the skin does not colour", () => {
  const skin = normalizeSkin({});
  const view = { zones: { END: { WHITE: ["0,3"] } } };
  assert.equal(zoneTint(skin, view, "0,3"), null);
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
// A grid's nodes, named as the engine names them: "x,y", drawn at [x, y]. `holes` are left out.
const grid = (width, height, holes = []) => {
  const nodes = [];
  for (let x = 0; x < width; x++) {
    for (let y = 0; y < height; y++) {
      if (!holes.includes(`${x},${y}`)) nodes.push({ name: `${x},${y}`, at: [x, y] });
    }
  }
  return nodes;
};
// A hex board of side `n` on an (2n-1)-wide grid: a hexagon drops the corners with |x − y| ≥ n.
const hexView = (layout, side) => {
  const size = layout === "rhombus" ? side : 2 * side - 1;
  const nodes = grid(size, size).filter(({ at: [x, y] }) => layout === "rhombus" || Math.abs(x - y) < side);
  return { shape: "hex", layout, nodes };
};
const glinski = hexView("hexagon", 6);
const hexRhombus = hexView("rhombus", 11);
const near = (a, b) => Math.abs(a - b) < 1e-9;

test("grid rows put the first row at the bottom and leave holes empty", () => {
  const view = { nodes: grid(3, 2, ["1,1"]) };
  assert.deepEqual(gridRows(view, false).map((r) => r.map((n) => n?.name ?? null)),
    [["0,1", null, "2,1"], ["0,0", "1,0", "2,0"]]);
});

test("a flipped grid is turned half way round", () => {
  const view = { nodes: grid(3, 2, ["1,1"]) };
  assert.deepEqual(gridRows(view, true).map((r) => r.map((n) => n?.name ?? null)),
    [["2,0", "1,0", "0,0"], ["2,1", null, "0,1"]]);
});

test("bounds come from the nodes", () => {
  const view = { nodes: grid(4, 2).filter((n) => n.at[0] > 0) };
  assert.deepEqual(gridBounds(view), { minX: 1, minY: 0, maxX: 3, maxY: 1 });
});

test("hex geometry from nodes matches the layout", () => {
  assert.equal(+hexGeometry(glinski).aspect.toFixed(6), 0.892269);
  assert.equal(+hexGeometry(hexRhombus).aspect.toFixed(6), 1.679564);
});

test("hex colours never repeat between neighbours", () => {
  for (const [dx, dy] of HEX_NEIGHBOURS) assert.notEqual(hexColour(5, 5), hexColour(5 + dx, 5 + dy));
});

test("the hexagon is upright and centred", () => {
  const bottom = hexCentre(glinski, [5, 0]);
  const top = hexCentre(glinski, [5, 10]);
  assert.ok(near(bottom.left, 0.5) && near(top.left, 0.5));
  const tops = glinski.nodes.map(({ at }) => hexCentre(glinski, at).top);
  assert.ok(near(bottom.top, Math.max(...tops)));
  assert.ok(near(top.top, Math.min(...tops)));
});

test("the rhombus lies wide", () => {
  const a = hexCentre(hexRhombus, [10, 0]);
  const b = hexCentre(hexRhombus, [0, 10]);
  assert.ok(near(a.top, b.top) && near(a.top, 0.5));
  const lefts = hexRhombus.nodes.map(({ at }) => hexCentre(hexRhombus, at).left);
  assert.ok(near(Math.min(a.left, b.left), Math.min(...lefts)));
  assert.ok(near(Math.max(a.left, b.left), Math.max(...lefts)));
  assert.ok(hexGeometry(hexRhombus).aspect > 1, "wider than tall");
});

test("flipping turns the board round", () => {
  for (const view of [glinski, hexRhombus]) {
    for (const [x, y] of [[5, 5], [2, 3], [7, 6]]) {
      const flipped = hexCentre(view, [x, y], true);
      const turned = hexCentre(view, [10 - x, 10 - y], false);
      assert.ok(near(flipped.left, turned.left) && near(flipped.top, turned.top));
    }
  }
});

test("neighbouring hexes sit one cell apart and do not overlap", () => {
  const { cellWidth, aspect } = hexGeometry(glinski);
  const a = hexCentre(glinski, [5, 5]);
  const b = hexCentre(glinski, [5, 6]);
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
    zones: { SIDE_A: { RED: ["0,3"], BLUE: ["3,0", "0,3"] }, GOAL: { RED: ["1,1"] } },
  };
  assert.equal(zoneTint(skin, view, "0,3"), "#ff0000", "a cell in both players' zones takes the first player's tint");
  assert.equal(zoneTint(skin, view, "3,0"), "#0000ff");
  assert.equal(zoneTint(skin, view, "1,1"), "#00ff00", "a one-colour zone still tints");
  assert.equal(zoneTint(skin, view, "2,2"), null);
});

test("zone stripes keep each player's colour, and a shared cell gets both", () => {
  const skin = normalizeSkin({ zones: { SIDE_A: { RED: "#ff0000", BLUE: "#0000ff" }, GOAL: "#f3d98b" } });
  const view = {
    players: ["RED", "BLUE"],
    zones: { SIDE_A: { RED: ["0,3"], BLUE: ["3,0", "0,3"] }, GOAL: { RED: ["1,1"] } },
  };
  assert.deepEqual(zoneStripeColours(skin, view, "3,0"), [stripeColour("#0000ff")]);
  assert.deepEqual(zoneStripeColours(skin, view, "0,3"), [stripeColour("#ff0000"), stripeColour("#0000ff")]);
  assert.deepEqual(zoneStripeColours(skin, view, "2,2"), []);
  assert.equal(stripeColour("#ff8000"), "rgba(166, 83, 0, 0.8)");
  assert.equal(stripeColour("teal"), "teal");
  assert.deepEqual(zoneColours(skin, view, "0,3"), ["#ff0000", "#0000ff"]);
});

test("each zone colour's stripes run their own way, over a wash of every colour", () => {
  const one = stripeGradient(["#ff0000"]);
  assert.equal(one.match(/repeating-linear-gradient/g).length, 1);
  assert.match(one, /repeating-linear-gradient\(-45deg, rgba\(166, 0, 0, 0\.8\)/);
  assert.match(one, /color-mix\(in srgb, #ff0000 30%, transparent\)/);

  const shared = stripeGradient(["#ff0000", "#0000ff"]);
  assert.match(shared, /repeating-linear-gradient\(-45deg, rgba\(166, 0, 0/);
  assert.match(shared, /repeating-linear-gradient\(45deg, rgba\(0, 0, 166/);
  assert.match(shared, /#0000ff 30%/);
  assert.notEqual(stripeAngle(0), stripeAngle(1));
});

test("a corner shared across zones takes the first player's tint", () => {
  const skin = normalizeSkin({ zones: { SIDE_A: { RED: "#ff0000", BLUE: "#0000ff" }, SIDE_B: { RED: "#ff0000", BLUE: "#0000ff" } } });
  // Hex's corner (10, 0): BLUE's SIDE_A (y = 0) and RED's SIDE_B (x = 10).
  const view = {
    players: ["RED", "BLUE"],
    zones: { SIDE_A: { RED: ["0,0"], BLUE: ["10,0"] }, SIDE_B: { RED: ["10,0"], BLUE: ["0,10"] } },
  };
  assert.equal(zoneTint(skin, view, "10,0"), "#ff0000");
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
  const view = { shape: "hex", layout: "rhombus", nodes: grid(99, 99) };
  const geometry = hexGeometry(view);
  const started = performance.now();
  for (const { at } of view.nodes) hexCentre(view, at, true, geometry);
  assert.ok(performance.now() - started < 500, "under half a second for 9,801 cells");
});

// ----- Maps -----

import { mapGeometry, mapPlace, mapSize } from "../board-model.js";

// Three points a unit apart along the bottom and one above the middle one, a path between the
// first two and an area over the lot, each drawn at the `at` the engine sends.
const mapView = {
  shape: "map",
  nodes: [
    { name: "a", at: [0, 0], kind: "point" },
    { name: "b", at: [2, 0], kind: "point" },
    { name: "c", at: [4, 0], kind: "point" },
    { name: "d", at: [2, 1.5], kind: "point" },
    { name: "ab", at: [1, 0], kind: "path", ends: ["a", "b"] },
    { name: "t", at: [2, 0.5], kind: "area", around: ["a", "c", "d"] },
  ],
};

test("a map's bounds cover every place and its unit is the shortest gap between two points", () => {
  assert.deepEqual(mapGeometry(mapView), { minX: 0, minY: 0, maxX: 4, maxY: 1.5, unit: 1.5 });
});

test("a map's unit is not dragged down by paths and areas sitting between points", () => {
  const near = { shape: "map", nodes: [...mapView.nodes, { name: "ad", at: [2, 0.75], kind: "path", ends: ["b", "d"] }] };
  assert.equal(mapGeometry(near).unit, 1.5);
});

test("a map with fewer than two points has unit 1 and places without dividing by zero", () => {
  const single = { shape: "map", nodes: [{ name: "a", at: [3, 7], kind: "point" }] };
  const geometry = mapGeometry(single);
  assert.deepEqual(geometry, { minX: 3, minY: 7, maxX: 3, maxY: 7, unit: 1 });
  assert.deepEqual(mapSize(geometry), { width: 2, height: 2 });
  assert.deepEqual(mapPlace(single, [3, 7], false, geometry), { left: 0.5, top: 0.5 });
  assert.deepEqual(mapPlace(single, [3, 7], true, geometry), { left: 0.5, top: 0.5 });
  assert.equal(mapGeometry({ shape: "map", nodes: [{ name: "a", at: [0, 0], kind: "path", ends: [] }] }).unit, 1);
});

test("two points on the same spot do not make the unit zero", () => {
  const twins = { shape: "map", nodes: [{ name: "a", at: [0, 0] }, { name: "b", at: [0, 0] }, { name: "c", at: [3, 0] }] };
  assert.equal(mapGeometry(twins).unit, 3);
});

test("a map's box is its bounds with one unit of padding all round", () => {
  assert.deepEqual(mapSize(mapGeometry(mapView)), { width: 4 + 3, height: 1.5 + 3 });
});

test("a corner point sits one unit in from the box's corner, y up", () => {
  const geometry = mapGeometry(mapView);
  const { width, height } = mapSize(geometry);
  const bottomLeft = mapPlace(mapView, [0, 0], false, geometry);
  assert.ok(Math.abs(bottomLeft.left - 1.5 / width) < 1e-12);
  assert.ok(Math.abs(bottomLeft.top - (1 - 1.5 / height)) < 1e-12);
  const topRight = mapPlace(mapView, [4, 1.5], false, geometry);
  assert.ok(Math.abs(topRight.left - (1 - 1.5 / width)) < 1e-12);
  assert.ok(Math.abs(topRight.top - 1.5 / height) < 1e-12);
});

test("a flipped map turns half way round about its bounds' centre", () => {
  const geometry = mapGeometry(mapView);
  for (const { at } of mapView.nodes) {
    const flipped = mapPlace(mapView, at, true, geometry);
    const opposite = mapPlace(mapView, [4 - at[0], 1.5 - at[1]], false, geometry);
    assert.ok(Math.abs(flipped.left - opposite.left) < 1e-12 && Math.abs(flipped.top - opposite.top) < 1e-12);
  }
  const bottomLeft = mapPlace(mapView, [0, 0], true, geometry);
  const { width, height } = mapSize(geometry);
  assert.ok(Math.abs(bottomLeft.left - (1 - 1.5 / width)) < 1e-12);
  assert.ok(Math.abs(bottomLeft.top - 1.5 / height) < 1e-12);
});

test("a path's and an area's piece sit at their own `at`", () => {
  const geometry = mapGeometry(mapView);
  const { width, height } = mapSize(geometry);
  for (const name of ["ab", "t"]) {
    const node = mapView.nodes.find((n) => n.name === name);
    const { left, top } = mapPlace(mapView, node.at, false, geometry);
    assert.ok(Math.abs(left * width - (node.at[0] + 1.5)) < 1e-12);
    assert.ok(Math.abs(top * height - (1.5 - node.at[1] + 1.5)) < 1e-12);
  }
});

test("placing a map's places without a given geometry still works", () => {
  const place = mapPlace(mapView, [2, 0.75], false);
  assert.ok(place.left > 0 && place.left < 1 && place.top > 0 && place.top < 1);
});

test("placing every place of a big map stays fast", () => {
  const nodes = [];
  for (let x = 0; x < 100; x++) for (let y = 0; y < 100; y++) nodes.push({ name: `${x},${y}`, at: [x * 1.25, y * 1.25], kind: "point" });
  const view = { shape: "map", nodes };
  const started = performance.now();
  const geometry = mapGeometry(view);
  for (const { at } of nodes) mapPlace(view, at, true, geometry);
  assert.equal(geometry.unit, 1.25);
  assert.ok(performance.now() - started < 500, "under half a second for 10,000 places");
});

test("several counted entries on one place are all kept, in order", () => {
  const pieces = [
    { position: "C1", code: "WARRIOR", player: "WHITE", count: 3 },
    { position: "C2", code: "SCOUT", player: "BLACK" },
    { position: "C1", code: "WARRIOR", player: "BLACK", count: 2 },
  ];
  const grouped = groupPieces(pieces);
  assert.deepEqual(grouped.get("C1"), [pieces[0], pieces[2]]);
  assert.deepEqual(grouped.get("C2"), [pieces[1]]);
});

test("the label of a place lists every entry; nobody's piece is neutral", () => {
  assert.equal(piecesLabel([{ position: "A", code: "ROOK", player: "WHITE" }], "A"), "WHITE ROOK at A");
  assert.equal(
    piecesLabel(
      [
        { position: "C1", code: "WARRIOR", player: "WHITE", count: 3 },
        { position: "C1", code: "WARRIOR", player: "BLACK", count: 2 },
        { position: "C1", code: "STONE", player: "" },
      ],
      "C1",
    ),
    "WHITE WARRIOR ×3, BLACK WARRIOR ×2, neutral STONE at C1",
  );
});
