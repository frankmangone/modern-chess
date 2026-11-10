// <modern-chess-board spec="…" skin="…"> — a playable board driven by the real engine.
//
//   spec  URL of a game spec (JSON). Or put the spec inline as a
//         <script type="application/json"> child.
//   skin  optional URL of a skin (JSON). Skins only change looks, never rules. A skin may ask for
//         a board with lines on the crossings (`board`), a colour for each player (`colours`)
//         and round pieces (`pieceStyle: "disc"`); see board-model.js.
//   remote  no engine here: the page gives the board a `view` (the JSON an engine reports)
//         and hears `select`, `move`, `promote` and `dropPiece` CustomEvents (detail {x, y},
//         {code} or {code, x, y}). Clicks only count while `interactive` is true, and promotion
//         choices are only shown then. Set `flipped` to draw the board from the second player's
//         side. Nothing is loaded but the skin.
//         In a game with a hand, set `dropCode` to the piece the player picked from it: the
//         squares in `view.drops[dropCode]` light up, and clicking one emits `dropPiece`
//         (not `drop`, which is also a native browser event).
//         A game with a supply (`view.supply`, the stones in Go) needs no hand: with exactly one
//         such piece, clicking a square listed in `view.drops` for it emits `dropPiece` at once.
//         With several, pick one through `dropCode` first.
//         A local board also shows a Pass button (when `view.can_pass`) and the score; a page that
//         plays remotely shows those itself.
//         Set `externalPrompts` when the page shows the promotion choice itself.
//
// All rules come from an engine (WebAssembly here, a server in remote mode); this file
// only draws what the engine reports and forwards clicks.
import init, { Board } from "./pkg/modern_chess_wasm.js";
import { isTurned, normalizeSkin, pieceFace, zoneTint } from "./board-model.js";

const SVG_NS = "http://www.w3.org/2000/svg";
const svgEl = (tag, attributes) => {
  const node = document.createElementNS(SVG_NS, tag);
  for (const [name, value] of Object.entries(attributes)) node.setAttribute(name, String(value));
  return node;
};

const DEFAULT_BOARD = "#e8c98a";
const DEFAULT_LINES = "#5c3b1e";

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
    for (const name of ["view", "interactive", "flipped", "dropCode", "externalPrompts"]) {
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

  // The piece picked from the hand, or null.
  get dropCode() {
    return this.remoteDropCode ?? null;
  }

  set dropCode(value) {
    this.remoteDropCode = value ?? null;
    if (this.ready) this.render();
  }

  // True when the page shows the promotion choice, so the board does not.
  get externalPrompts() {
    return this.remoteExternalPrompts ?? false;
  }

  set externalPrompts(value) {
    this.remoteExternalPrompts = Boolean(value);
    if (this.ready) this.render();
  }

  // Where the picked hand piece may go. Only a piece that was picked lights its squares.
  dropSquares(view) {
    if (this.dropCode === null) return null;
    return view.drops?.[this.dropCode] ?? [];
  }

  // The piece a click on an empty square would place: the one picked from the hand, or else the
  // game's only supply piece. Null when it is not clear which one is meant.
  placeCode(view) {
    if (this.dropCode !== null) return this.dropCode;
    return view.supply?.length === 1 ? view.supply[0] : null;
  }

  // Remote mode asks the page; local mode asks the engine. `local` is the engine call as
  // [method, ...arguments].
  emit(type, detail, local) {
    if (this.remote) this.dispatchEvent(new CustomEvent(type, { detail, bubbles: true }));
    else this.act(local);
  }

  render() {
    const view = this.view;
    if (!view) return; // remote, and the page has not given a view yet
    this.style.setProperty("--mcb-light", this.skin.squares.light);
    this.style.setProperty("--mcb-dark", this.skin.squares.dark);
    // Lets the stylesheet make the cells smaller on a board with more than ten points a side.
    this.style.setProperty("--mcb-n", String(Math.max(view.width, view.height)));

    const children = [el("p", "mcb-status", this.statusText(view))];
    if (this.message) children.push(el("p", "mcb-message", this.message));
    // A page that plays remotely shows the hand itself; a local board shows it here.
    if (view.hands && !this.remote) children.push(this.renderHand(view));
    children.push(this.renderGrid(view));
    if (!this.remote) children.push(...this.renderPassAndScore(view));
    if (view.promotion && (!this.remote || (this.interactive && !this.externalPrompts))) {
      children.push(this.renderPromotion(view));
    }

    if (!this.remote) {
      const reset = el("button", "mcb-reset", "Reset");
      reset.type = "button";
      reset.addEventListener("click", () => {
        this.board.reset();
        this.steps = [];
        this.remoteDropCode = null;
        this.message = "";
        this.render();
      });
      children.push(reset);
    }
    this.replaceChildren(...children);
  }

  statusText(view) {
    if (view.status.state === "won") return `${view.status.winner} wins`;
    if (view.status.state === "draw") return "Draw";
    if (view.promotion) return `${view.turn}: choose a promotion`;
    return view.selected ? `${view.turn} to move — pick a highlighted square` : `${view.turn} to move`;
  }

  renderGrid(view) {
    const lined = this.skin.board.style === "intersections";
    const grid = el("div", lined ? "mcb-grid mcb-intersections" : "mcb-grid");
    grid.style.gridTemplateColumns = `repeat(${view.width}, var(--mcb-cell))`;
    if (lined) {
      grid.style.setProperty("--mcb-board", this.skin.board.background ?? DEFAULT_BOARD);
      grid.append(this.renderLines(view));
    }
    const disabled = new Set(view.disabled.map(([x, y]) => `${x},${y}`));
    const pieces = new Map(view.pieces.map((p) => [`${p.x},${p.y}`, p]));
    const dests = new Set((this.dropSquares(view) ?? view.destinations).map(([x, y]) => `${x},${y}`));
    const playable =
      (!this.remote || this.interactive) &&
      view.status.state === "playing" &&
      !view.promotion;

    // Rows are drawn top first, so y = 0 (the first player's side) ends up at the bottom.
    // A flipped board is that picture turned half way round.
    for (let row = 0; row < view.height; row++) {
      const y = this.flipped ? row : view.height - 1 - row;
      for (let column = 0; column < view.width; column++) {
        const x = this.flipped ? view.width - 1 - column : column;
        const key = `${x},${y}`;
        if (disabled.has(key)) {
          grid.append(el("div", "mcb-hole"));
          continue;
        }
        const piece = pieces.get(key);
        const cell = el("button", `mcb-cell ${(x + y) % 2 === 0 ? "mcb-dark" : "mcb-light"}`);
        cell.type = "button";
        cell.disabled = !playable;
        cell.setAttribute("aria-label", piece ? `${piece.player} ${piece.code} at ${x},${y}` : `empty square ${x},${y}`);
        const tint = zoneTint(this.skin, view, x, y);
        if (tint) cell.style.setProperty("--mcb-tint", tint);
        if (tint) cell.classList.add("mcb-zone");
        if (view.selected && view.selected[0] === x && view.selected[1] === y) cell.classList.add("mcb-selected");
        if (dests.has(key)) cell.classList.add("mcb-dest");
        if (piece) cell.append(this.renderPiece(piece.code, piece.player, view, true));
        cell.addEventListener("click", () => this.onCell(x, y));
        grid.append(cell);
      }
    }
    return grid;
  }

  // The lines of a board whose pieces stand on the crossings. Drawn in cell units: crossing
  // (x, y) is the middle of its cell, so the squares sit on top and the pieces land on the crossings.
  renderLines(view) {
    const { board } = this.skin;
    const { width, height } = view;
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

  // The pieces the player to move holds. Picking one lights the squares it may be dropped on.
  renderHand(view) {
    const box = el("div", "mcb-hand");
    box.append(el("span", "mcb-hand-label", "In hand:"));
    const held = Object.entries(view.hand[view.turn] ?? {}).filter(([, count]) => count > 0);
    if (held.length === 0) box.append(el("span", "mcb-hand-empty", "nothing yet"));
    for (const [code, count] of held) {
      const chip = el("button", "mcb-hand-piece");
      chip.type = "button";
      chip.setAttribute("aria-label", `${code} in hand, ${count}`);
      chip.setAttribute("aria-pressed", String(this.dropCode === code));
      chip.disabled = view.status.state !== "playing" || Boolean(view.promotion);
      chip.append(this.renderPiece(code, view.turn, view), el("span", "mcb-hand-count", `×${count}`));
      chip.addEventListener("click", () => {
        this.dropCode = this.dropCode === code ? null : code;
      });
      box.append(chip);
    }
    return box;
  }

  // A pass button and the count so far, for a game in which both matter.
  renderPassAndScore(view) {
    const parts = [];
    const scores = Object.entries(view.score ?? {});
    if (scores.length > 0) {
      const text = scores.map(([player, points]) => `${player} ${points}`).join(" · ");
      parts.push(el("p", "mcb-score", `Count if the game ended now: ${text}`));
    }
    if (view.can_pass) {
      const pass = el("button", "mcb-pass", "Pass");
      pass.type = "button";
      pass.addEventListener("click", () => this.act(["pass"]));
      parts.push(pass);
    }
    return parts;
  }

  renderPromotion(view) {
    const box = el("div", "mcb-promo");
    box.append(el("span", "mcb-promo-label", "Promote to:"));
    for (const code of view.promotion) {
      const button = el("button", "mcb-promo-option");
      button.type = "button";
      button.title = code;
      button.append(this.renderPiece(code, view.turn, view));
      button.addEventListener("click", () => this.emit("promote", { code }, ["promote", code]));
      box.append(button);
    }
    return box;
  }

  onCell(x, y) {
    const view = this.view;
    const drops = this.dropSquares(view);
    if (drops?.some(([dx, dy]) => dx === x && dy === y)) {
      this.emit("dropPiece", { code: this.dropCode, x, y }, ["drop_piece", this.dropCode, x, y]);
      return;
    }
    if (view.destinations.some(([dx, dy]) => dx === x && dy === y)) {
      this.emit("move", { x, y }, ["make_move", x, y]);
      return;
    }
    // Nothing was picked from a hand: a legal square for the game's supply piece takes it.
    const supplied = this.dropCode === null ? this.placeCode(view) : null;
    if (supplied !== null && view.drops?.[supplied]?.some(([dx, dy]) => dx === x && dy === y)) {
      this.emit("dropPiece", { code: supplied, x, y }, ["drop_piece", supplied, x, y]);
      return;
    }
    const piece = view.pieces.find((p) => p.x === x && p.y === y);
    if (piece && piece.player === view.turn) {
      if (!this.remote) this.remoteDropCode = null; // picking up a piece cancels a hand choice
      this.emit("select", { x, y }, ["select", x, y]);
    }
  }

  // Runs one engine call, given as [method, ...arguments]. Engine refusals are shown, never
  // thrown. A fault (the engine crashed) is handled by `recover`.
  act(step) {
    const [method, ...args] = step;
    try {
      this.board[method](...args);
      this.steps.push(step);
      this.remoteDropCode = null;
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
    this.remoteDropCode = null;
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
