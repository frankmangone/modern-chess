// Isometric drawings for the Drafting table exploration: boards, a rules sheet and an engine,
// built as SVG so they stay sharp and can move. Motion is skipped for people who ask for less.

const NS = "http://www.w3.org/2000/svg";
const COS = Math.cos(Math.PI / 6);
const calm = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

function el(name, attrs = {}, parent) {
  const node = document.createElementNS(NS, name);
  for (const [key, value] of Object.entries(attrs)) node.setAttribute(key, value);
  if (parent) parent.append(node);
  return node;
}

/** Maps grid coordinates (i along one edge, j along the other, z up) to the screen. */
function projector(ox, oy, u) {
  return (i, j, z = 0) => [ox + (i - j) * u * COS, oy + (i + j) * u * 0.5 - z];
}
const pts = (list) => list.map((p) => p.join(",")).join(" ");

/** A flat slab with two visible sides. Returns the projector so the top can be drawn on. */
function slab(parent, ox, oy, w, d, u, thick, fills) {
  const p = projector(ox, oy, u);
  el("polygon", { points: pts([p(0, d), p(w, d), [p(w, d)[0], p(w, d)[1] + thick], [p(0, d)[0], p(0, d)[1] + thick]]), fill: fills.left }, parent);
  el("polygon", { points: pts([p(w, 0), p(w, d), [p(w, d)[0], p(w, d)[1] + thick], [p(w, 0)[0], p(w, 0)[1] + thick]]), fill: fills.right }, parent);
  el("polygon", { points: pts([p(0, 0), p(w, 0), p(w, d), p(0, d)]), fill: fills.top, stroke: "#14233b", "stroke-width": 1.5, "stroke-linejoin": "round" }, parent);
  return p;
}

const SLAB = { top: "#ffffff", left: "#b9c7dc", right: "#9db0cb" };
const TINT = "#cfdaea";
const INK = "#14233b";

function stone(parent, p, i, j, black) {
  const [x, y] = p(i, j);
  el("ellipse", { cx: x, cy: y - 2, rx: 6.5, ry: 4.2, fill: black ? INK : "#fff", stroke: INK, "stroke-width": 1.4 }, parent);
}

function pawn(parent, p, i, j, fill) {
  const [x, y] = p(i, j);
  el("ellipse", { cx: x, cy: y, rx: 6, ry: 3.2, fill, stroke: INK, "stroke-width": 1.3 }, parent);
  el("rect", { x: x - 2.2, y: y - 9, width: 4.4, height: 9, fill, stroke: INK, "stroke-width": 1.3 }, parent);
  el("circle", { cx: x, cy: y - 12, r: 4.4, fill, stroke: INK, "stroke-width": 1.3 }, parent);
}

function cells(parent, p, n) {
  for (let i = 0; i < n; i++) {
    for (let j = 0; j < n; j++) {
      if ((i + j) % 2 === 0) continue;
      el("polygon", { points: pts([p(i, j), p(i + 1, j), p(i + 1, j + 1), p(i, j + 1)]), fill: TINT }, parent);
    }
  }
}

function lines(parent, p, n, inset = 0.5) {
  for (let k = 0; k < n; k++) {
    el("line", { x1: p(k + inset, inset)[0], y1: p(k + inset, inset)[1], x2: p(k + inset, n - inset)[0], y2: p(k + inset, n - inset)[1], stroke: INK, "stroke-width": 1 }, parent);
    el("line", { x1: p(inset, k + inset)[0], y1: p(inset, k + inset)[1], x2: p(n - inset, k + inset)[0], y2: p(n - inset, k + inset)[1], stroke: INK, "stroke-width": 1 }, parent);
  }
}

function chessBoard(parent, ox, oy, u, thick = 8, withPieces = true) {
  const g = el("g", {}, parent);
  const p = slab(g, ox, oy, 8, 8, u, thick, SLAB);
  cells(g, p, 8);
  el("polygon", { points: pts([p(0, 0), p(8, 0), p(8, 8), p(0, 8)]), fill: "none", stroke: INK, "stroke-width": 1.5 }, g);
  if (withPieces) {
    for (const [i, j, f] of [[1.5, 1.5, "#fff"], [3.5, 2.5, "#fff"], [5.5, 5.5, INK], [2.5, 6.5, INK], [6.5, 1.5, "#fff"], [4.5, 4.5, INK]]) pawn(g, p, i, j, f);
  }
  return g;
}

function goBoard(parent, ox, oy, u, thick = 8, withStones = true) {
  const g = el("g", {}, parent);
  const p = slab(g, ox, oy, 9, 9, u, thick, SLAB);
  lines(g, p, 9);
  if (withStones) {
    for (const [i, j, b] of [[2.5, 2.5, true], [3.5, 3.5, false], [5.5, 2.5, true], [4.5, 5.5, false], [6.5, 6.5, true], [2.5, 6.5, false], [5.5, 4.5, true]]) stone(g, p, i, j, b);
  }
  return g;
}

function shogiBoard(parent, ox, oy, u, thick = 8, withPieces = true) {
  const g = el("g", {}, parent);
  const p = slab(g, ox, oy, 9, 9, u, thick, SLAB);
  for (let k = 0; k <= 9; k++) {
    el("line", { x1: p(k, 0)[0], y1: p(k, 0)[1], x2: p(k, 9)[0], y2: p(k, 9)[1], stroke: INK, "stroke-width": 1 }, g);
    el("line", { x1: p(0, k)[0], y1: p(0, k)[1], x2: p(9, k)[0], y2: p(9, k)[1], stroke: INK, "stroke-width": 1 }, g);
  }
  if (withPieces) {
    for (const [i, j, fill] of [[1.5, 2.5, INK], [3.5, 1.5, INK], [4.5, 4.5, "#2f5bea"], [6.5, 6.5, "#fff"], [2.5, 7.5, "#fff"], [7.5, 3.5, "#2f5bea"]]) {
      const c = p(i, j);
      el("polygon", { points: pts([[c[0], c[1] - 9], [c[0] + 7, c[1] - 2], [c[0] + 5, c[1] + 4], [c[0] - 5, c[1] + 4], [c[0] - 7, c[1] - 2]]), fill, stroke: INK, "stroke-width": 1.3, "stroke-linejoin": "round" }, g);
    }
  }
  return g;
}

/** The mini boards on the game cards. */
function thumbnail(host, kind) {
  const svg = el("svg", { viewBox: "0 0 190 120", role: "img", "aria-hidden": "true", class: "thumb" }, host);
  const draw = { chess: chessBoard, go: goBoard, shogi: shogiBoard }[kind];
  if (draw) return draw(svg, 95, 8, kind === "chess" ? 9.4 : 8.4, 7, true);
  // Xiangqi and checkers share the chess grid in this sketch.
  return chessBoard(svg, 95, 8, 9.4, 7, true);
}

/** The front-page picture: a rules sheet feeds one engine, which sets up every board. */
function hero(host) {
  const svg = el("svg", { viewBox: "0 0 840 560", role: "img", class: "hero-art", "aria-labelledby": "hero-title hero-desc" }, host);
  el("title", { id: "hero-title" }, svg).textContent = "A rules sheet feeds one engine, which sets up a chess, a Go and a shogi board";
  el("desc", { id: "hero-desc" }, svg).textContent = "Drawing of one rules sheet on the left, one engine block in the middle and three boards on the right.";

  const wires = el("g", { fill: "none", "stroke-linecap": "round" }, svg);
  const wire = (d, delay) => {
    el("path", { d, stroke: "#9db0cb", "stroke-width": 3 }, wires);
    el("path", { d, stroke: "#14233b", "stroke-width": 1.6, "stroke-dasharray": "2 9", class: calm ? "" : "flow" }, wires);
    if (!calm) {
      const dot = el("circle", { r: 5.5, fill: "#f2b227", stroke: INK, "stroke-width": 1.5 }, wires);
      const move = el("animateMotion", { dur: "3.4s", begin: delay + "s", repeatCount: "indefinite", path: d }, dot);
      move.setAttribute("keyPoints", "0;1");
      move.setAttribute("keyTimes", "0;1");
    }
  };

  // Rules sheet, top left.
  const sheet = el("g", {}, svg);
  const sp = slab(sheet, 150, 70, 5, 3.6, 26, 5, SLAB);
  const rows = [3.7, 3.1, 4.1, 2.5, 3.4];
  rows.forEach((len, k) => {
    const j = 0.6 + k * 0.62;
    el("line", { x1: sp(0.5, j)[0], y1: sp(0.5, j)[1], x2: sp(0.5 + len * 0.95, j)[0], y2: sp(0.5 + len * 0.95, j)[1], stroke: k === 1 ? "#2f5bea" : INK, "stroke-width": 3, "stroke-linecap": "round", class: calm ? "" : "write", style: `animation-delay:${k * 0.35}s` }, sheet);
  });
  const sheetLabel = el("text", { x: 76, y: 208, class: "tag" }, svg);
  sheetLabel.textContent = "Rules sheet";

  // Engine, centre.
  const engine = el("g", {}, svg);
  const ep = projector(300, 255, 30);
  const h = 72;
  el("polygon", { points: pts([ep(0, 3), ep(3, 3), [ep(3, 3)[0], ep(3, 3)[1] - h], [ep(0, 3)[0], ep(0, 3)[1] - h]]), fill: "#2f5bea" }, engine);
  el("polygon", { points: pts([ep(3, 0), ep(3, 3), [ep(3, 3)[0], ep(3, 3)[1] - h], [ep(3, 0)[0], ep(3, 0)[1] - h]]), fill: "#2347b8" }, engine);
  el("polygon", { points: pts([ep(0, 0, h), ep(3, 0, h), ep(3, 3, h), ep(0, 3, h)]), fill: "#6b8df5", stroke: INK, "stroke-width": 1.5, "stroke-linejoin": "round" }, engine);
  el("polygon", { points: pts([ep(0.8, 0.8, h), ep(2.2, 0.8, h), ep(2.2, 2.2, h), ep(0.8, 2.2, h)]), fill: "#f2b227", stroke: INK, "stroke-width": 1.5, class: calm ? "" : "pulse" }, engine);
  for (const k of [0.5, 1.5, 2.5]) {
    el("line", { x1: ep(0, k, 22)[0], y1: ep(0, k, 22)[1], x2: ep(0, k, 50)[0], y2: ep(0, k, 50)[1], stroke: "#fff", "stroke-width": 3, "stroke-linecap": "round", opacity: 0.8 }, engine);
  }
  const engineLabel = el("text", { x: 224, y: 392, class: "tag" }, svg);
  engineLabel.textContent = "One engine";

  // Boards, right.
  const chess = chessBoard(svg, 640, 28, 13, 8);
  el("text", { x: 700, y: 20, class: "tag" }, svg).textContent = "Chess";
  const go = goBoard(svg, 650, 200, 12.6, 8);
  el("text", { x: 722, y: 192, class: "tag" }, svg).textContent = "Go";
  const shogi = shogiBoard(svg, 604, 372, 12.6, 8);
  el("text", { x: 676, y: 364, class: "tag" }, svg).textContent = "Shogi";
  [chess, go, shogi].forEach((g, k) => { if (!calm) { g.classList.add("bob"); g.style.animationDelay = k * 0.5 + "s"; } });

  wire("M 235 150 C 250 175 262 190 275 205", 0);
  wire("M 360 235 C 440 200 500 120 575 108", 0.9);
  wire("M 390 285 C 460 280 520 280 600 292", 1.5);
  wire("M 360 330 C 440 380 500 420 575 440", 2.1);
}

window.IsoArt = { hero, thumbnail, chessBoard, goBoard, shogiBoard };
