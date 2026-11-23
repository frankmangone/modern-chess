// <modern-chess-board spec="…" skin="…"> — a playable board driven by the real engine.
//
//   spec  URL of a game spec (JSON). Or put the spec inline as a
//         <script type="application/json"> child.
//   skin  optional URL of a skin (JSON). Skins only change looks, never rules. A skin may ask for
//         a board with lines on the crossings (`board`), a colour for each player (`colours`)
//         and round pieces (`pieceStyle: "disc"`); see board-model.js. A hex board (the view
//         says `shape: "hex"`) colours its cells in three: `squares.light`, `squares.mid` and
//         `squares.dark` (a skin without `mid` gets the colour between the other two). A zone
//         tint (`zones`) is one colour, or one per player (`{ "SIDE_A": { "RED": "#…",
//         "BLUE": "#…" } }`) so each player's cells of the zone show in their colour.
//   remote  no engine here: the page gives the board a `view` (the JSON an engine reports)
//         and hears `choose` and `cancel` CustomEvents. A `choose` carries one answer as its
//         detail ({place}, {code}, {hand} or {action}; a place is a node's name from
//         `view.nodes`); a click on any node emits `choose {place}`, and the engine decides what
//         it means. A hand piece emits `choose {hand}`, and a chosen one emits `cancel` when it is
//         clicked again. Clicks only count while `interactive` is true, and code choices (a
//         promotion) are only shown then. Set `flipped` to draw the board from the second
//         player's side. Nothing is loaded but the skin.
//         The board marks from `view.choice`: the chosen piece (`start`) as selected, and the
//         places of a `MOVE` choice, or of a `PLACE` choice once something is chosen, as
//         destinations. Places that pick a piece stay unmarked.
//         A local board also shows a button for each of `view.choice.actions` (a pass is one)
//         and the score; a page that plays remotely shows those itself.
//         Set `externalPrompts` when the page shows the code choice (a promotion) itself.
//
// All rules come from an engine (WebAssembly here, a server in remote mode); this file
// only draws what the engine reports and forwards clicks.
import init, { Board } from "./pkg/modern_chess_wasm.js";
import {
  gridBounds,
  gridRows,
  hexCentre,
  hexColour,
  hexGeometry,
  isTurned,
  mapGeometry,
  mapPlace,
  mapSize,
  normalizeSkin,
  pieceFace,
  zoneTint,
} from "./board-model.js";

const SVG_NS = "http://www.w3.org/2000/svg";
const svgEl = (tag, attributes) => {
  const node = document.createElementNS(SVG_NS, tag);
  for (const [name, value] of Object.entries(attributes)) node.setAttribute(name, String(value));
  return node;
};

const DEFAULT_BOARD = "#e8c98a";
const DEFAULT_LINES = "#5c3b1e";
// A map's areas, where no zone tints them: a pale wash over the board.
const AREA_FILL = "rgba(255, 255, 255, 0.3)";
// How wide a map's cell button is, in units: a point's, a path's (small, so it keeps clear of the
// points at its ends) and an area's.
const MAP_CELL_SIZE = { point: 0.8, path: 0.5, area: 0.9 };

let wasmReady;
const loadWasm = () => (wasmReady ??= init());

const el = (tag, className, text) => {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
};

async function fetchText(url) {
  const response = await fetch(new URL(url, document.baseURI));
  if (!response.ok) throw new Error(`could not load ${url} (${response.status})`);
  return response.text();
}

class ModernChessBoard extends HTMLElement {
  connectedCallback() {
    if (this.started) return;
    this.started = true;
    // A page may set these before this element is defined. Such values sit on the element
    // itself and hide the accessors below, so move them back onto the accessors.
    for (const name of ["view", "interactive", "flipped", "externalPrompts"]) {
      if (Object.hasOwn(this, name)) {
        const value = this[name];
        delete this[name];
        this[name] = value;
      }
    }
    this.start();
  }

  get remote() {
    return this.hasAttribute("remote");
  }

  async start() {
    this.classList.add("mcb");
    if (this.remote) return this.startRemote();
    const inline = this.querySelector('script[type="application/json"]')?.textContent;
    this.textContent = "Loading board…";
    try {
      const specUrl = this.getAttribute("spec");
      const skinUrl = this.getAttribute("skin");
      const [, specText, skinRaw] = await Promise.all([
        loadWasm(),
        specUrl ? fetchText(specUrl) : Promise.resolve(inline),
        skinUrl ? fetchText(skinUrl).then(JSON.parse).catch(() => null) : Promise.resolve(null),
      ]);
      if (!specText) throw new Error("add a spec attribute or an inline JSON spec");
      this.skin = normalizeSkin(skinRaw);
      this.specText = specText;
      this.board = new Board(specText);
      this.steps = [];
      this.message = "";
      this.ready = true;
      this.render();
    } catch (error) {
      this.replaceChildren(el("p", "mcb-error", `Could not show this board: ${error?.message ?? error}`));
    }
  }

  // Remote mode: the skin is the only thing to load; then the page's view is drawn.
  async startRemote() {
    await this.loadSkin();
    this.ready = true;
    this.render();
  }

  async loadSkin() {
    const skinUrl = this.getAttribute("skin");
    const skinRaw = skinUrl ? await fetchText(skinUrl).then(JSON.parse).catch(() => null) : null;
    this.skin = normalizeSkin(skinRaw);
  }

  // A page may learn the skin after the board is up (it is fetched with the game's page).
  static get observedAttributes() {
    return ["skin"];
  }

  async attributeChangedCallback() {
    if (!this.remote || !this.ready) return;
    await this.loadSkin();
    this.render();
  }

  get view() {
    if (this.remote) return this.remoteView ?? null;
    return JSON.parse(this.board.view());
  }

  set view(value) {
    this.remoteView = value;
    if (this.ready) this.render();
  }

  get interactive() {
    return this.remoteInteractive ?? false;
  }

  set interactive(value) {
    this.remoteInteractive = Boolean(value);
    if (this.ready) this.render();
  }

  get flipped() {
    return this.remoteFlipped ?? false;
  }

  set flipped(value) {
    this.remoteFlipped = Boolean(value);
    if (this.ready) this.render();
  }

  // True when the page shows the code choice (a promotion), so the board does not.
  get externalPrompts() {
    return this.remoteExternalPrompts ?? false;
  }

  set externalPrompts(value) {
    this.remoteExternalPrompts = Boolean(value);
    if (this.ready) this.render();
  }

  // The places to mark as destinations: a `MOVE` choice's, and a `PLACE` choice's once something is
  // chosen (a hand piece or an action). A `PLACE` choice open from the start of the turn, as in Go,
  // stays unmarked: every empty point would light up.
  destinationNames(view) {
    const { places, chosen } = view.choice;
    return places.filter((place) => place.for === "MOVE" || (place.for === "PLACE" && chosen.length > 0)).map((place) => place.name);
  }

  // Remote mode asks the page; local mode asks the engine.
  choose(answer) {
    if (this.remote) this.dispatchEvent(new CustomEvent("choose", { detail: answer, bubbles: true }));
    else this.act(["choose", JSON.stringify(answer)]);
  }

  cancel() {
    if (this.remote) this.dispatchEvent(new CustomEvent("cancel", { bubbles: true }));
    else this.act(["cancel"]);
  }

  render() {
    const view = this.view;
    if (!view) return; // remote, and the page has not given a view yet
    this.style.setProperty("--mcb-light", this.skin.squares.light);
    this.style.setProperty("--mcb-dark", this.skin.squares.dark);
    this.style.setProperty("--mcb-mid", this.skin.squares.mid);
    // Lets the stylesheet make the cells smaller on a board with more than ten points a side.
    this.style.setProperty("--mcb-n", String(this.boardSpan(view)));

    const children = [el("p", "mcb-status", this.statusText(view))];
    if (this.message) children.push(el("p", "mcb-message", this.message));
    // A page that plays remotely shows the hand itself; a local board shows it here.
    if (view.hands && !this.remote) children.push(this.renderHand(view));
    children.push(this.renderGrid(view));
    if (!this.remote) children.push(...this.renderActionsAndScore(view));
    if (view.choice.codes.length > 0 && (!this.remote || (this.interactive && !this.externalPrompts))) {
      children.push(this.renderCodes(view));
    }

    if (!this.remote) {
      const reset = el("button", "mcb-reset", "Reset");
      reset.type = "button";
      reset.addEventListener("click", () => {
        this.board.reset();
        this.steps = [];
        this.message = "";
        this.render();
      });
      children.push(reset);
    }
    this.replaceChildren(...children);
  }

  // How many cells wide or tall the board is, whichever is more.
  boardSpan(view) {
    if (view.shape === "map") {
      const geometry = mapGeometry(view);
      const { width, height } = mapSize(geometry);
      return Math.max(width, height) / geometry.unit;
    }
    const { minX, minY, maxX, maxY } = gridBounds(view);
    return Math.max(maxX - minX + 1, maxY - minY + 1);
  }

  statusText(view) {
    if (view.status.state === "won") return `${view.status.winner} wins`;
    if (view.status.state === "draw") return "Draw";
    if (view.choice.codes.length > 0) return `${view.turn}: choose a promotion`;
    return view.choice.start ? `${view.turn} to move — pick a highlighted square` : `${view.turn} to move`;
  }

  // What the cells of a board share: the pieces by position, the squares to light up and whether
  // a click counts.
  cellContext(view) {
    return {
      pieces: new Map(view.pieces.map((p) => [p.position, p])),
      dests: new Set(this.destinationNames(view)),
      playable: (!this.remote || this.interactive) && view.status.state === "playing" && view.choice.codes.length === 0,
    };
  }

  // One cell button for a node: its label, tint, highlights, piece and click.
  renderCell(node, view, context, className, emptyLabel) {
    const piece = context.pieces.get(node.name);
    const cell = el("button", className);
    cell.type = "button";
    cell.disabled = !context.playable;
    cell.setAttribute("aria-label", piece ? `${piece.player} ${piece.code} at ${node.name}` : `${emptyLabel} ${node.name}`);
    const tint = zoneTint(this.skin, view, node.name);
    if (tint) cell.style.setProperty("--mcb-tint", tint);
    if (tint) cell.classList.add("mcb-zone");
    // While a promotion waits, the moved piece is the thing to look at, not where it came from.
    if (view.choice.start === node.name && view.choice.codes.length === 0) cell.classList.add("mcb-selected");
    if (context.dests.has(node.name)) cell.classList.add("mcb-dest");
    if (piece) cell.append(this.renderPiece(piece.code, piece.player, view, true));
    cell.addEventListener("click", () => {
      // A click that answers nothing and has nothing to put back is not sent.
      const { places, chosen } = view.choice;
      if (chosen.length === 0 && !places.some((place) => place.name === node.name)) return;
      this.choose({ place: node.name });
    });
    return cell;
  }

  renderGrid(view) {
    if (view.shape === "hex") return this.renderHexes(view);
    if (view.shape === "map") return this.renderMap(view);
    const lined = this.skin.board.style === "intersections";
    const grid = el("div", lined ? "mcb-grid mcb-intersections" : "mcb-grid");
    const rows = gridRows(view, this.flipped);
    grid.style.gridTemplateColumns = `repeat(${rows[0].length}, var(--mcb-cell))`;
    if (lined) {
      grid.style.setProperty("--mcb-board", this.skin.board.background ?? DEFAULT_BOARD);
      grid.append(this.renderLines(view));
    }
    const context = this.cellContext(view);
    for (const node of rows.flat()) {
      if (node === null) {
        grid.append(el("div", "mcb-hole"));
        continue;
      }
      const [x, y] = node.at;
      grid.append(this.renderCell(node, view, context, `mcb-cell ${(x + y) % 2 === 0 ? "mcb-dark" : "mcb-light"}`, "empty square"));
    }
    return grid;
  }

  // A hex board: the same cell buttons, each placed at its hex's centre in a box as wide as a
  // square board of the same width, and cut to a hexagon. Cells are coloured in three.
  renderHexes(view) {
    const box = el("div", "mcb-hexes");
    const geometry = hexGeometry(view);
    box.style.setProperty("--mcb-hex-aspect", String(geometry.aspect));
    box.style.setProperty("--mcb-hex-w", `${geometry.cellWidth * 100}%`);
    box.style.setProperty("--mcb-hex-h", `${geometry.cellHeight * 100}%`);
    box.style.width = `calc(var(--mcb-cell) * ${geometry.bounds.maxX - geometry.bounds.minX + 1})`;
    const context = this.cellContext(view);
    const tones = ["mcb-light", "mcb-mid", "mcb-dark"];

    for (const node of view.nodes) {
      const [x, y] = node.at;
      const cell = this.renderCell(node, view, context, `mcb-cell mcb-hex ${tones[hexColour(x, y)]}`, "empty cell");
      const { left, top } = hexCentre(view, node.at, this.flipped, geometry);
      cell.style.left = `${left * 100}%`;
      cell.style.top = `${top * 100}%`;
      box.append(cell);
    }
    return box;
  }

  // A map: its shapes drawn in one SVG, and the same cell buttons as on a grid, each centred on its
  // place's `at`. A place never has coordinates read out of its name. A box as wide as a grid
  // with the same spacing, in the map's own proportions.
  renderMap(view) {
    const geometry = mapGeometry(view);
    const { unit } = geometry;
    const { width, height } = mapSize(geometry);
    const box = el("div", "mcb-map");
    box.style.setProperty("--mcb-map-aspect", String(width / height));
    box.style.width = `calc(var(--mcb-cell) * ${width / unit})`;
    box.style.background = this.skin.board.background ?? DEFAULT_BOARD;
    box.append(this.renderMapShapes(view, geometry));

    const context = this.cellContext(view);
    // Points on top, so a point's button wins where it overlaps a path's.
    for (const kind of ["area", "path", "point"]) {
      for (const node of view.nodes) {
        if ((node.kind ?? "point") !== kind) continue;
        const cell = this.renderCell(node, view, context, `mcb-cell mcb-map-cell mcb-${kind}`, `empty ${kind}`);
        const size = MAP_CELL_SIZE[kind];
        const { left, top } = mapPlace(view, node.at, this.flipped, geometry);
        cell.style.left = `${left * 100}%`;
        cell.style.top = `${top * 100}%`;
        cell.style.width = `${((size * unit) / width) * 100}%`;
        cell.style.height = `${((size * unit) / height) * 100}%`;
        cell.style.setProperty("--mcb-k", String(size));
        box.append(cell);
      }
    }
    return box;
  }

  // Under the buttons: areas as filled outlines, then the written lines, the paths and the
  // points. Drawn in the box's own map units, so line weights follow `unit`.
  renderMapShapes(view, geometry) {
    const { unit } = geometry;
    const { width, height } = mapSize(geometry);
    const colour = this.skin.board.lines ?? DEFAULT_LINES;
    const svg = svgEl("svg", {
      class: "mcb-lines",
      viewBox: `0 0 ${width} ${height}`,
      preserveAspectRatio: "none",
      "aria-hidden": "true",
    });
    const byName = new Map(view.nodes.map((node) => [node.name, node]));
    const spot = (name) => {
      const node = byName.get(name);
      if (!node) return null;
      const { left, top } = mapPlace(view, node.at, this.flipped, geometry);
      return [left * width, top * height];
    };
    const line = (a, b, weight) => {
      const [from, to] = [spot(a), spot(b)];
      if (!from || !to) return;
      svg.append(svgEl("line", {
        x1: from[0], y1: from[1], x2: to[0], y2: to[1],
        stroke: colour, "stroke-width": weight * unit, "stroke-linecap": "round",
      }));
    };

    for (const node of view.nodes) {
      if (node.kind !== "area") continue;
      const corners = (node.around ?? []).map(spot).filter(Boolean);
      if (corners.length < 3) continue;
      const tint = zoneTint(this.skin, view, node.name);
      svg.append(svgEl("polygon", {
        points: corners.map(([x, y]) => `${x},${y}`).join(" "),
        fill: tint ?? AREA_FILL,
        "fill-opacity": tint ? 0.75 : 1,
        stroke: colour,
        "stroke-opacity": 0.35,
        "stroke-width": 0.03 * unit,
        "stroke-linejoin": "round",
      }));
    }
    for (const [a, b] of view.lines ?? []) line(a, b, 0.05);
    for (const node of view.nodes) {
      if (node.kind === "path" && node.ends) line(node.ends[0], node.ends[1], 0.14);
    }
    for (const node of view.nodes) {
      if ((node.kind ?? "point") !== "point") continue;
      const [cx, cy] = spot(node.name);
      svg.append(svgEl("circle", { cx, cy, r: 0.1 * unit, fill: colour }));
    }
    return svg;
  }

  // The lines of a board whose pieces stand on the crossings. Drawn in cell units: crossing
  // (x, y) is the middle of its cell, so the squares sit on top and the pieces land on the crossings.
  renderLines(view) {
    const { board } = this.skin;
    const { minX, minY, maxX, maxY } = gridBounds(view);
    const width = maxX - minX + 1;
    const height = maxY - minY + 1;
    const colour = board.lines ?? DEFAULT_LINES;
    const col = (x) => (this.flipped ? width - 1 - x : x) + 0.5;
    const row = (y) => (this.flipped ? y : height - 1 - y) + 0.5;
    const svg = svgEl("svg", {
      class: "mcb-lines",
      viewBox: `0 0 ${width} ${height}`,
      preserveAspectRatio: "none",
      "aria-hidden": "true",
    });
    const line = (x1, y1, x2, y2, weight = 0.04) =>
      svg.append(svgEl("line", { x1, y1, x2, y2, stroke: colour, "stroke-width": weight, "stroke-linecap": "round" }));

    for (let y = 0; y < height; y++) {
      const frame = y === 0 || y === height - 1;
      line(col(0), row(y), col(width - 1), row(y), frame ? 0.07 : 0.04);
    }
    // A break stops the lines between two neighbouring rows, except on the two edge files.
    const broken = new Set(board.breaks.filter(([a, b]) => Math.abs(a - b) === 1).map(([a, b]) => Math.min(a, b)));
    for (let x = 0; x < width; x++) {
      if (x === 0 || x === width - 1) {
        line(col(x), row(0), col(x), row(height - 1), 0.07);
        continue;
      }
      let start = 0;
      for (let y = 0; y <= height - 1; y++) {
        if (y === height - 1 || broken.has(y)) {
          if (y > start) line(col(x), row(start), col(x), row(y));
          start = y + 1;
        }
      }
    }
    for (const [[x1, y1], [x2, y2]] of board.diagonals) line(col(x1), row(y1), col(x2), row(y2));
    for (const [x, y] of board.dots) {
      svg.append(svgEl("circle", { cx: col(x), cy: row(y), r: 0.09, fill: colour }));
    }
    for (const [x, y] of board.crosses) svg.append(this.renderCross(col(x), row(y), width, colour));
    for (const { text, x, y } of board.labels) {
      const label = svgEl("text", {
        x: col(x),
        y: row(y),
        "text-anchor": "middle",
        "dominant-baseline": "central",
        "font-size": 0.42,
        fill: colour,
        opacity: 0.8,
      });
      label.textContent = text;
      svg.append(label);
    }
    return svg;
  }

  // The four corner marks around a crossing; the ones that would stick out of the board are left off.
  renderCross(cx, cy, width, colour) {
    const gap = 0.1;
    const reach = 0.24;
    const left = cx - 0.5 === 0;
    const right = cx + 0.5 === width;
    let d = "";
    for (const sx of [-1, 1]) {
      if ((sx < 0 && left) || (sx > 0 && right)) continue;
      for (const sy of [-1, 1]) {
        d += `M${cx + sx * gap} ${cy + sy * reach} L${cx + sx * gap} ${cy + sy * gap} L${cx + sx * reach} ${cy + sy * gap} `;
      }
    }
    return svgEl("path", { class: "mcb-cross", d: d.trim(), fill: "none", stroke: colour, "stroke-width": 0.04 });
  }

  // `onBoard` is true for a piece standing on a square, which may be turned to face the
  // opponent; pieces in a hand or a promotion choice always stand upright.
  renderPiece(code, player, view, onBoard = false) {
    const face = pieceFace(this.skin, code, player, view.players);
    const span = el("span", `mcb-piece mcb-${face.kind}`);
    if (face.kind === "svg") {
      span.innerHTML = face.value; // skins are authored with the book, like the specs
    } else if (face.kind === "image") {
      const image = el("img", "mcb-img");
      image.src = face.value; // the skin only gets to name a picture file on this site
      image.alt = "";
      image.draggable = false;
      span.append(image);
    } else {
      span.textContent = face.value;
    }
    if (onBoard && isTurned(this.skin, player, view.players, this.flipped)) span.classList.add("mcb-turned");
    if (face.colour) {
      span.style.setProperty(face.kind === "fallback" ? "background" : "color", face.colour);
    }
    if (face.disc) {
      span.classList.add("mcb-disc");
      if (face.colour) span.style.setProperty("--mcb-ring", face.colour);
    }
    return span;
  }

  // The pieces the player to move holds. A piece the engine offers can be chosen; a chosen one is
  // taken back by clicking it again.
  renderHand(view) {
    const box = el("div", "mcb-hand");
    box.append(el("span", "mcb-hand-label", "In hand:"));
    const held = Object.entries(view.hand[view.turn] ?? {}).filter(([, count]) => count > 0);
    if (held.length === 0) box.append(el("span", "mcb-hand-empty", "nothing yet"));
    for (const [code, count] of held) {
      const chosen = view.choice.chosen.some((answer) => answer.hand === code);
      const chip = el("button", "mcb-hand-piece");
      chip.type = "button";
      chip.setAttribute("aria-label", `${code} in hand, ${count}`);
      chip.setAttribute("aria-pressed", String(chosen));
      chip.disabled = view.status.state !== "playing" || view.choice.codes.length > 0 || !(chosen || view.choice.hand.includes(code) || view.choice.chosen.length > 0);
      chip.append(this.renderPiece(code, view.turn, view), el("span", "mcb-hand-count", `×${count}`));
      chip.addEventListener("click", () => (chosen ? this.cancel() : this.choose({ hand: code })));
      box.append(chip);
    }
    return box;
  }

  // A button for each action to name now (a pass is one), and the count so far, for a game in
  // which both matter.
  renderActionsAndScore(view) {
    const parts = [];
    const scores = Object.entries(view.score ?? {});
    if (scores.length > 0) {
      const text = scores.map(([player, points]) => `${player} ${points}`).join(" · ");
      parts.push(el("p", "mcb-score", `Count if the game ended now: ${text}`));
    }
    // Actions that only narrow the choice (a plain "Move" or "Stone") are noise at the start
    // of a turn: they get a button once something is answered, or when supplies compete.
    const { actions, chosen, places } = view.choice;
    const narrowing = actions.filter((action) => !action.plays);
    const competing = narrowing.length > 1 && places.some((place) => place.for === "PLACE");
    for (const { name, plays } of actions) {
      if (!plays && chosen.length === 0 && !competing) continue;
      const button = el("button", "mcb-pass", actionLabel(name));
      button.type = "button";
      button.dataset.action = name;
      button.addEventListener("click", () => this.choose({ action: name }));
      parts.push(button);
    }
    return parts;
  }

  renderCodes(view) {
    const box = el("div", "mcb-promo");
    box.append(el("span", "mcb-promo-label", "Promote to:"));
    for (const code of view.choice.codes) {
      const button = el("button", "mcb-promo-option");
      button.type = "button";
      button.title = code;
      button.append(this.renderPiece(code, view.turn, view));
      button.addEventListener("click", () => this.choose({ code }));
      box.append(button);
    }
    return box;
  }

  // Runs one engine call, given as [method, ...arguments]. Engine refusals are shown, never
  // thrown. A fault (the engine crashed) is handled by `recover`.
  act(step) {
    const [method, ...args] = step;
    try {
      this.board[method](...args);
      this.steps.push(step);
      this.message = "";
    } catch (error) {
      if (isFault(error)) this.recover();
      else this.message = error?.message ?? String(error);
    }
    this.render();
  }

  // After a crash the engine's board object refuses every call, which would leave the page
  // frozen. A new board is built from the same rules and the calls that worked are played
  // again, so the position comes back as it was before the call that crashed.
  recover() {
    try {
      this.board = new Board(this.specText);
      for (const [method, ...args] of this.steps) this.board[method](...args);
      this.message = "The engine could not handle that move, so it was undone.";
    } catch {
      this.board = new Board(this.specText);
      this.steps = [];
      this.message = "The engine hit an error, so the board was restarted.";
    }
  }
}

// "PASS" on a button reads "Pass".
function actionLabel(name) {
  const text = name.replaceAll("_", " ").toLowerCase();
  return text.charAt(0).toUpperCase() + text.slice(1);
}

// A crash of the engine shows up as a WebAssembly trap, or as the "recursive use" error every
// later call to the crashed board raises. An engine refusal is neither.
function isFault(error) {
  return (
    error instanceof WebAssembly.RuntimeError || /recursive use of an object/.test(error?.message ?? "")
  );
}

if (!customElements.get("modern-chess-board")) {
  customElements.define("modern-chess-board", ModernChessBoard);
}
