// <modern-chess-board spec="…" skin="…"> — a playable board driven by the real engine.
//
//   spec  URL of a game spec (JSON). Or put the spec inline as a
//         <script type="application/json"> child.
//   skin  optional URL of a skin (JSON). Skins only change looks, never rules.
//   remote  no engine here: the page gives the board a `view` (the JSON an engine reports)
//         and hears `select`, `move`, `promote` and `dropPiece` CustomEvents (detail {x, y},
//         {code} or {code, x, y}). Clicks only count while `interactive` is true, and promotion
//         choices are only shown then. Set `flipped` to draw the board from the second player's
//         side. Nothing is loaded but the skin.
//         In a game with a hand, set `dropCode` to the piece the player picked from it: the
//         squares in `view.drops[dropCode]` light up, and clicking one emits `dropPiece`
//         (not `drop`, which is also a native browser event).
//         Set `externalPrompts` when the page shows the promotion choice itself.
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

  // Where the picked hand piece may go. Only a remote board takes drops for now.
  dropSquares(view) {
    if (!this.remote || this.dropCode === null) return null;
    return view.drops?.[this.dropCode] ?? [];
  }

  // Remote mode asks the page; local mode asks the engine. `local` is the engine call as
  // [method, ...arguments], or null when this board has no such call.
  emit(type, detail, local) {
    if (this.remote) this.dispatchEvent(new CustomEvent(type, { detail, bubbles: true }));
    else if (local) this.act(local);
  }

  render() {
    const view = this.view;
    if (!view) return; // remote, and the page has not given a view yet
    this.style.setProperty("--mcb-light", this.skin.squares.light);
    this.style.setProperty("--mcb-dark", this.skin.squares.dark);

    const children = [el("p", "mcb-status", this.statusText(view))];
    if (this.message) children.push(el("p", "mcb-message", this.message));
    if (view.hands && !this.remote) {
      children.push(el("p", "mcb-note", "Drops from the hand are not supported on this board yet, so it is read-only."));
    }
    children.push(this.renderGrid(view));
    if (view.promotion && (!this.remote || (this.interactive && !this.externalPrompts))) {
      children.push(this.renderPromotion(view));
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
    const dests = new Set((this.dropSquares(view) ?? view.destinations).map(([x, y]) => `${x},${y}`));
    const playable =
      (!this.remote || this.interactive) &&
      (this.remote || !view.hands) &&
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
      button.addEventListener("click", () => this.emit("promote", { code }, ["promote", code]));
      box.append(button);
    }
    return box;
  }

  onCell(x, y) {
    const view = this.view;
    const drops = this.dropSquares(view);
    if (drops?.some(([dx, dy]) => dx === x && dy === y)) {
      this.emit("dropPiece", { code: this.dropCode, x, y }, null);
      return;
    }
    if (view.destinations.some(([dx, dy]) => dx === x && dy === y)) {
      this.emit("move", { x, y }, ["make_move", x, y]);
      return;
    }
    const piece = view.pieces.find((p) => p.x === x && p.y === y);
    if (piece && piece.player === view.turn) this.emit("select", { x, y }, ["select", x, y]);
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
