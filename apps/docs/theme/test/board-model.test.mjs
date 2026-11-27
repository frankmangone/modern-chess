import test from "node:test";
import assert from "node:assert/strict";
import { stripeAngle, stripeColour, stripeGradient, zoneColours, zoneStripeColours, gridBounds, gridRows, hasHoles, neighbourLines, groupPieces, isTurned, normalizeSkin, piecesLabel, pieceFace, pieceLine, tableSlots, zoneTint } from "../board-model.js";

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

import { hexBorders, hexCentre, hexColour, hexCorners, hexGeometry } from "../board-model.js";

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
  assert.equal(+hexGeometry(hexRhombus).aspect.toFixed(6), 1.630165);
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

test("the rhombus leans like a parallelogram, its first row on top, with pointy hexes", () => {
  const rows = (x) => hexRhombus.nodes.filter(({ at }) => at[0] === x).map(({ at }) => hexCentre(hexRhombus, at));
  const top = rows(0);
  const second = rows(1);
  assert.ok(top.every(({ top: t }) => near(t, top[0].top)), "a row is level");
  assert.ok(second[0].top > top[0].top, "the next row is lower");
  const shift = Math.min(...second.map((c) => c.left)) - Math.min(...top.map((c) => c.left));
  const { cellWidth } = hexGeometry(hexRhombus);
  assert.ok(near(shift, cellWidth / 2), "each row is half a hex to the right");
  assert.ok(hexGeometry(hexRhombus).aspect > 1, "wider than tall");
});

test("on the rhombus every neighbour is one hex away, so no hexes overlap", () => {
  const { width, height } = hexGeometry(hexRhombus);
  const centre = (at) => {
    const { left, top } = hexCentre(hexRhombus, at);
    return [left * width, top * height];
  };
  for (const [dx, dy] of HEX_NEIGHBOURS) {
    const [a, b] = [centre([5, 5]), centre([5 + dx, 5 + dy])];
    assert.ok(near(Math.hypot(a[0] - b[0], a[1] - b[1]), Math.sqrt(3)), `step ${dx},${dy}`);
  }
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

// A 3x3 board with its four corners missing: a plus.
const plus = {
  nodes: [[1, 0], [0, 1], [1, 1], [2, 1], [1, 2]].map((at) => ({ at })),
};
const full = { nodes: [0, 1, 2].flatMap((x) => [0, 1, 2].map((y) => ({ at: [x, y] }))) };

test("a board with places missing has holes, and a whole one does not", () => {
  assert.equal(hasHoles(plus), true);
  assert.equal(hasHoles(full), false);
});

test("the lines of a board with holes join only places that are on it", () => {
  const lines = neighbourLines(plus).map(([a, b]) => `${a}-${b}`).sort();
  assert.deepEqual(lines, ["0,1-1,1", "1,0-1,1", "1,1-1,2", "1,1-2,1"].sort());
});

// ----- The rhombus's borders -----

const hexSkin = normalizeSkin({ zones: { SIDE_A: { RED: "#c0392b", BLUE: "#2f5fa8" }, SIDE_B: { RED: "#c0392b", BLUE: "#2f5fa8" } } });
// Hex as the engine reports it: RED owns the columns x = 0 and x = 10, BLUE the rows y = 0 and y = 10.
const hexZones = (n) => {
  const column = (x) => Array.from({ length: n }, (_, y) => `${x},${y}`);
  const row = (y) => Array.from({ length: n }, (_, x) => `${x},${y}`);
  return {
    SIDE_A: { RED: column(0), BLUE: row(0) },
    SIDE_B: { RED: column(n - 1), BLUE: row(n - 1) },
  };
};
const hexPlayed = { ...hexRhombus, players: ["RED", "BLUE"], zones: hexZones(11) };
const bordersOf = (colour, flipped = false) => hexBorders(hexSkin, hexPlayed, flipped).filter((b) => b.colour === colour);

test("a rhombus's zones are borders: every outer side is coloured once", () => {
  const borders = hexBorders(hexSkin, hexPlayed);
  // The board's outline is 8 · 11 sides, less the side the two acute corners each count twice.
  assert.equal(borders.length, 8 * 11 - 2);
  assert.equal(bordersOf("#c0392b").length + bordersOf("#2f5fa8").length, borders.length);
  const keys = new Set(borders.map((b) => [b.from, b.to].map(([x, y]) => `${x.toFixed(3)},${y.toFixed(3)}`).sort().join("|")));
  assert.equal(keys.size, borders.length, "no side is drawn twice");
});

test("red runs along the top and bottom, blue along the left and right", () => {
  const { height } = hexGeometry(hexRhombus);
  const mid = (b) => (b.from[1] + b.to[1]) / 2;
  const red = bordersOf("#c0392b");
  assert.ok(red.every((b) => mid(b) < 0.3 || mid(b) > height - 0.3 || (mid(b) > 0.3 && mid(b) < 1.1)), "red hugs the top and bottom");
  assert.equal(red.filter((b) => mid(b) < height / 2).length, 22, "two sides for each of the 11 hexes on top");
  assert.equal(red.filter((b) => mid(b) >= height / 2).length, 22, "and on the bottom");
  assert.equal(bordersOf("#2f5fa8").length, 42, "blue has the rest: the long edges, a side less at the top corners");
});

test("a corner's shared side goes to the first player", () => {
  const first = hexBorders(hexSkin, hexPlayed)[0];
  assert.equal(first.colour, "#c0392b");
});

test("flipping the board turns its borders half way round", () => {
  const round = (v) => String(Math.round(v * 1000) / 1000 + 0);
  const key = (b) => [b.from, b.to].map(([x, y]) => `${round(x)},${round(y)}`).sort().join("|");
  const { width, height } = hexGeometry(hexRhombus);
  const direct = hexBorders(hexSkin, hexPlayed, false);
  const flipped = hexBorders(hexSkin, hexPlayed, true);
  const turned = new Set(direct.map((b) => key({ from: [width - b.from[0], height - b.from[1]], to: [width - b.to[0], height - b.to[1]] })));
  assert.equal(flipped.length, direct.length);
  assert.ok(flipped.every((b) => turned.has(key(b))));
});

test("a hex's corners are one radius from its centre", () => {
  const box = hexGeometry(hexRhombus);
  const { left, top } = hexCentre(hexRhombus, [3, 4], false, box);
  for (const [x, y] of hexCorners(hexRhombus, [3, 4], false, box)) {
    assert.ok(near(Math.hypot(x - left * box.width, y - top * box.height), 1));
  }
});

test("a rhombus's hexes are cut pointy-topped, with a ring that follows them", () => {
  const cut = rule(".mcb-hexes-pointy .mcb-hex");
  assert.ok(cut && /clip-path: polygon\(50% 0/.test(cut), "a corner on top");
  const ring = rule(".mcb-hexes-pointy .mcb-hex:focus-visible::before");
  assert.ok(ring && /clip-path/.test(ring), "its own focus ring");
  assert.ok(/pointer-events: none/.test(rule(".mcb-hex-lines") ?? ""), "the lines never take a click");
});

// ----- A board's own drawing -----

test("a skin may draw a map itself with an inline svg, and nothing else counts as art", () => {
  const svg = '<svg viewBox="0 0 12 11"></svg>';
  assert.equal(normalizeSkin({ board: { art: svg } }).board.art, svg);
  assert.equal(normalizeSkin({ board: {} }).board.art, null);
  for (const bad of [42, "<div></div>", "", null, "x".repeat(200001)]) {
    assert.equal(normalizeSkin({ board: { art: bad } }).board.art, null, String(bad).slice(0, 12));
  }
});

test("a catalogue page's board art fits its map: its viewBox is the places' bounds plus a unit all round", () => {
  const root = new URL("../../../../crates/", import.meta.url);
  const page = JSON.parse(readFileSync(new URL("game/pages/snakes-and-ladders.json", root), "utf8"));
  const spec = JSON.parse(readFileSync(new URL("engine/specs/snakes_and_ladders.json", root), "utf8"));
  const view = { nodes: spec.board.places.map((p) => ({ name: p.name, at: p.at, kind: "point" })) };
  const { width, height } = mapSize(mapGeometry(view));
  const box = /viewBox="0 0 ([\d.]+) ([\d.]+)"/.exec(page.skin.board.art);
  assert.deepEqual([+box[1], +box[2]], [width, height]);
});

// ---- a table of slots, for a game of cards ----

const table = normalizeSkin({
  table: {
    slots: [
      { label: "Prizes", bag: "DECK", pile: true },
      { label: "Prize", bag: "PRIZE" },
      { label: "RED", bag: "LAST", of: "RED", bid: "PICK" },
      { label: "BLUE", bag: "LAST", of: "BLUE", bid: "PICK" },
    ],
  },
});
const bag = (name, pieces = {}, extra = {}) => ({ name, pieces, ...extra });
const bagsOf = (...list) => ({ bags: list });
const states = (view) => tableSlots(table, view).map((slot) => slot.state);

test("a skin without a table, or with a bad one, has no table", () => {
  assert.equal(normalizeSkin({}).table, null);
  assert.equal(normalizeSkin({ table: { slots: [] } }).table, null);
  assert.equal(normalizeSkin({ table: { slots: [{ label: "x" }, 7, null] } }).table, null);
  assert.equal(normalizeSkin({ table: "yes" }).table, null);
  assert.deepEqual(tableSlots(normalizeSkin({}), bagsOf()), []);
});

test("a slot's label is its bag's name unless it says otherwise", () => {
  const skin = normalizeSkin({ table: { slots: [{ bag: "PRIZE" }, { bag: "DECK", label: "Prizes" }] } });
  assert.deepEqual(skin.table.slots.map((slot) => slot.label), ["PRIZE", "Prizes"]);
});

test("empty bags are empty slots, and a pile shows its count face down", () => {
  const view = bagsOf(bag("DECK", { C01: 1, C02: 1, C03: 1 }), bag("PRIZE"), bag("LAST", {}, { owner: "RED" }), bag("LAST", {}, { owner: "BLUE" }));
  assert.deepEqual(states(view), ["pile", "empty", "empty", "empty"]);
  assert.equal(tableSlots(table, view)[0].count, 3);
  assert.equal(tableSlots(table, bagsOf(bag("DECK"))).at(0).state, "empty");
});

test("a hidden bag is a pile with the count the game tells, or without it", () => {
  const counted = bagsOf(bag("DECK", {}, { hidden: true, count: 12 }));
  assert.deepEqual(tableSlots(table, counted)[0], { label: "Prizes", owner: "", state: "pile", count: 12 });
  const uncounted = bagsOf(bag("DECK", {}, { hidden: true }));
  assert.equal(tableSlots(table, uncounted)[0].state, "pile");
  assert.equal(tableSlots(table, bagsOf(bag("DECK", {}, { hidden: true, count: 0 })))[0].state, "empty");
});

test("the prize on offer and the bids turned over are face up", () => {
  const view = bagsOf(bag("PRIZE", { C05: 1 }), bag("LAST", { C09: 1 }, { owner: "RED" }), bag("LAST", { C02: 1 }, { owner: "BLUE" }));
  const slots = tableSlots(table, view);
  assert.deepEqual(slots.slice(1).map((slot) => [slot.state, slot.codes]), [["up", ["C05"]], ["up", ["C09"]], ["up", ["C02"]]]);
  assert.equal(slots[2].owner, "RED");
});

test("a seat that has answered while another has not has a card face down", () => {
  const view = { waiting: ["RED"], ...bagsOf(bag("LAST", {}, { owner: "RED" }), bag("LAST", {}, { owner: "BLUE" })) };
  assert.deepEqual(states(view).slice(2), ["empty", "down"]);
});

test("a bid placed shows face down for its owner and for the others, whether or not they can see it", () => {
  const mine = bagsOf(bag("PICK", { C09: 1 }, { owner: "RED" }), bag("PICK", {}, { owner: "BLUE", hidden: true, count: 1 }));
  assert.deepEqual(states(mine).slice(2), ["down", "down"]);
  const hiddenOnly = bagsOf(bag("PICK", {}, { owner: "RED", hidden: true, count: 1 }));
  assert.equal(states(hiddenOnly)[2], "down");
});

test("nothing is down once the bids are turned over, and the old bids go with the next round", () => {
  const turned = { waiting: [], ...bagsOf(bag("PICK", {}, { owner: "RED" }), bag("LAST", { C09: 1 }, { owner: "RED" })) };
  assert.equal(states(turned)[2], "up");
  const fresh = { waiting: ["RED", "BLUE"], ...bagsOf(bag("LAST", {}, { owner: "RED" }), bag("LAST", {}, { owner: "BLUE" })) };
  assert.deepEqual(states(fresh).slice(2), ["empty", "empty"]);
});

test("an answered seat does not count for a slot without a bid", () => {
  const view = { waiting: ["RED"], ...bagsOf(bag("PRIZE", { C05: 1 })) };
  assert.equal(states(view)[1], "up");
});

// ---- pieces in a line, for a game where a place holds many (the points of Backgammon) ----

test("a skin has no lines unless it asks for them, and a bad ask is no ask", () => {
  assert.equal(normalizeSkin({}).stacks, null);
  for (const bad of [null, true, "5", [], { max: 0 }, { max: -1 }, { max: 2.5 }, { max: "5" }]) {
    assert.equal(normalizeSkin({ stacks: bad }).stacks, null, JSON.stringify(bad));
  }
  assert.deepEqual(normalizeSkin({ stacks: { max: 5 } }).stacks, { max: 5 });
  assert.deepEqual(normalizeSkin({ stacks: {} }).stacks, { max: 5 });
});

test("a place draws its pieces one by one, up to the limit, and says the count on the last when there are more", () => {
  const skin = normalizeSkin({ stacks: { max: 5 } });
  assert.deepEqual(pieceLine(skin, 1), null);
  assert.deepEqual(pieceLine(skin, 2), { shown: 2, label: undefined });
  assert.deepEqual(pieceLine(skin, 5), { shown: 5, label: undefined });
  assert.deepEqual(pieceLine(skin, 6), { shown: 5, label: "6" });
  assert.deepEqual(pieceLine(skin, 15), { shown: 5, label: "15" });
});

test("without lines a place keeps its single piece and its count", () => {
  assert.equal(pieceLine(normalizeSkin({}), 12), null);
});
