// <modern-chess-board spec="…" skin="…"> — a playable board driven by the real engine.
//
//   spec  URL of a game spec (JSON). Or put the spec inline as a
//         <script type="application/json"> child.
//   skin  optional URL of a skin (JSON). Skins only change looks, never rules.
//   remote  no engine here: the page gives the board a `view` (the JSON an engine reports)
//         and hears `select`, `move` and `promote` CustomEvents (detail {x, y} or {code}).
//         Clicks only count while `interactive` is true. Nothing is loaded but the skin.
//
// All rules come from an engine (WebAssembly here, a server in remote mode); this file
// only draws what the engine reports and forwards clicks.
import init, { Board } from "./pkg/modern_chess_wasm.js";
import { normalizeSkin, pieceFace, zoneTint } from "./board-model.js";

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
    for (const name of ["view", "interactive"]) {
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
      this.board = new Board(specText);
      this.message = "";
      this.render();
    } catch (error) {
      this.replaceChildren(el("p", "mcb-error", `Could not show this board: ${error?.message ?? error}`));
    }
  }

  // Remote mode: the skin is the only thing to load; then the page's view is drawn.
  async startRemote() {
    const skinUrl = this.getAttribute("skin");
    const skinRaw = skinUrl ? await fetchText(skinUrl).then(JSON.parse).catch(() => null) : null;
    this.skin = normalizeSkin(skinRaw);
    this.ready = true;
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

  // Remote mode asks the page; local mode asks the engine.
  emit(type, detail, local) {
    if (this.remote) this.dispatchEvent(new CustomEvent(type, { detail, bubbles: true }));
    else this.act(local);
  }

  render() {
    const view = this.view;
    if (!view) return; // remote, and the page has not given a view yet
    this.style.setProperty("--mcb-light", this.skin.squares.light);
    this.style.setProperty("--mcb-dark", this.skin.squares.dark);

    const children = [el("p", "mcb-status", this.statusText(view))];
    if (this.message) children.push(el("p", "mcb-message", this.message));
    if (view.hands) {
      children.push(el("p", "mcb-note", "Drops from the hand are not supported on this board yet, so it is read-only."));
    }
    children.push(this.renderGrid(view));
    if (view.promotion) children.push(this.renderPromotion(view));

    if (!this.remote) {
      const reset = el("button", "mcb-reset", "Reset");
      reset.type = "button";
      reset.addEventListener("click", () => {
        this.board.reset();
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
    const grid = el("div", "mcb-grid");
    grid.style.gridTemplateColumns = `repeat(${view.width}, var(--mcb-cell))`;
    const disabled = new Set(view.disabled.map(([x, y]) => `${x},${y}`));
    const pieces = new Map(view.pieces.map((p) => [`${p.x},${p.y}`, p]));
    const dests = new Set(view.destinations.map(([x, y]) => `${x},${y}`));
    const playable =
      (!this.remote || this.interactive) && !view.hands && view.status.state === "playing" && !view.promotion;

    // Rows are drawn top first, so y = 0 (the first player's side) ends up at the bottom.
    for (let row = 0; row < view.height; row++) {
      const y = view.height - 1 - row;
      for (let x = 0; x < view.width; x++) {
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
        if (piece) cell.append(this.renderPiece(piece.code, piece.player, view));
        cell.addEventListener("click", () => this.onCell(x, y));
        grid.append(cell);
      }
    }
    return grid;
  }

  renderPiece(code, player, view) {
    const face = pieceFace(this.skin, code, player, view.players);
    const span = el("span", `mcb-piece mcb-${face.kind}`);
    if (face.kind === "svg") {
      span.innerHTML = face.value; // skins are authored with the book, like the specs
    } else {
      span.textContent = face.value;
    }
    if (face.colour) {
      span.style.setProperty(face.kind === "fallback" ? "background" : "color", face.colour);
    }
    return span;
  }

  renderPromotion(view) {
    const box = el("div", "mcb-promo");
    box.append(el("span", "mcb-promo-label", "Promote to:"));
    for (const code of view.promotion) {
      const button = el("button", "mcb-promo-option");
      button.type = "button";
      button.title = code;
      button.append(this.renderPiece(code, view.turn, view));
      button.addEventListener("click", () => this.emit("promote", { code }, () => this.board.promote(code)));
      box.append(button);
    }
    return box;
  }

  onCell(x, y) {
    const view = this.view;
    if (view.destinations.some(([dx, dy]) => dx === x && dy === y)) {
      this.emit("move", { x, y }, () => this.board.make_move(x, y));
      return;
    }
    const piece = view.pieces.find((p) => p.x === x && p.y === y);
    if (piece && piece.player === view.turn) this.emit("select", { x, y }, () => this.board.select(x, y));
  }

  // Runs one engine call. Engine refusals are shown, never thrown.
  act(call) {
    try {
      call();
      this.message = "";
    } catch (error) {
      this.message = error?.message ?? String(error);
    }
    this.render();
  }
}

if (!customElements.get("modern-chess-board")) {
  customElements.define("modern-chess-board", ModernChessBoard);
}
