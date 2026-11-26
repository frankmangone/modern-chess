// Shared by the three pages: data, the page shell, the live board and the dice tray.
export async function load() {
  const data = await (await fetch(new URL("data/games.json", import.meta.url))).json();
  const bySlug = Object.fromEntries(data.games.map((g) => [g.slug, g]));
  return { ...data, bySlug };
}

export const h = (tag, attrs = {}, ...kids) => {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (v === false || v == null) continue;
    if (k === "class") node.className = v;
    else if (k.startsWith("on")) node.addEventListener(k.slice(2), v);
    else if (k === "style") node.setAttribute("style", v);
    else node.setAttribute(k, v === true ? "" : v);
  }
  node.append(...kids.flat().filter((x) => x != null && x !== false));
  return node;
};

export function shell(active = "games") {
  document.getElementById("nav").innerHTML = `
    <nav class="navbar">
      <a href="home.html" class="navbar-logo"><div class="navbar-logo-mark">♟</div><span class="navbar-logo-text">BoardHub</span></a>
      <div class="navbar-links">
        <a href="home.html" class="navbar-link ${active === "games" ? "current" : ""}">Games</a>
        <a href="home.html" class="navbar-link">Community</a>
        <a href="home.html" class="navbar-link">Learn</a>
      </div>
      <div class="navbar-right"><button class="btn-login">Log in</button><button class="btn-signup">Sign up</button></div>
    </nav>`;
  document.getElementById("foot").innerHTML = `
    <footer class="footer">
      <div class="footer-logo"><div class="footer-logo-mark">♟</div><span class="footer-logo-text">BoardHub</span></div>
      <span class="footer-tagline">The hub for every board game community.</span>
    </footer>`;
}

export const tagsOf = (game) => game.tags.filter((t) => t !== "two players");
export const tagEls = (game) => game.tags.map((t) => h("span", { class: "game-tag" }, t));
export const param = (name, fallback) => new URLSearchParams(location.search).get(name) ?? fallback;

/** A new live board in `host`, playing `game` with the real engine. */
export function mountBoard(host, game) {
  host.replaceChildren();
  const board = h("modern-chess-board", {
    spec: new URL(game.spec, import.meta.url).href,
    skin: new URL(game.skin, import.meta.url).href,
  });
  board.textContent = "Playable board (needs JavaScript).";
  host.append(board);
  return board;
}

// ---------- A little markdown: paragraphs, bullets and bold ----------
const esc = (s) => s.replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]);
const inline = (s) => esc(s).replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>").replace(/\*(.+?)\*/g, "<em>$1</em>");
export function markdown(src) {
  const out = [];
  let list = null;
  for (const block of src.split(/\n\n+/)) {
    const lines = block.split("\n");
    if (lines.every((l) => l.startsWith("- "))) {
      out.push(`<ul>${lines.map((l) => `<li>${inline(l.slice(2))}</li>`).join("")}</ul>`);
    } else out.push(`<p>${inline(block.replace(/\n/g, " "))}</p>`);
  }
  return out.join("");
}
export const firstParagraph = (src) => src.split(/\n\n+/)[0].replace(/\*\*/g, "");

// ---------- Dice: the die face, the tray, and what it says ----------
const PIPS = { 1: [4], 2: [0, 8], 3: [0, 4, 8], 4: [0, 2, 6, 8], 5: [0, 2, 4, 6, 8], 6: [0, 2, 3, 5, 6, 8] };
const COLOURS = { RED: "#d6523a", BLUE: "#3f6fd1", WHITE: "#f3e5cb", BLACK: "#3a2a20" };
const nice = (p) => p[0] + p.slice(1).toLowerCase();

export function ghost() {
  return h("div", { class: "d-die ghost", "aria-hidden": "true" }, "?");
}

export function die(value, { tumble = false, spent = false, tint } = {}) {
  const el = h("div", { class: `d-die${tumble ? " tumble" : ""}${spent ? " spent" : ""}`, role: "img", "aria-label": `A die showing ${value}` });
  if (tint) el.style.setProperty("--die", `color-mix(in srgb, ${tint} 30%, #f7ecd5)`);
  for (let i = 0; i < 9; i += 1) el.append(h("i", { class: PIPS[value].includes(i) ? "on" : "" }));
  return el;
}

/** Shows what the dice of a game say, read from the engine's bags after every move. */
export function diceTray(tray, board, game) {
  if (!game.tags.includes("dice")) { tray.hidden = true; return; }
  tray.hidden = false;
  const faces = h("div", { class: "d-dice-faces" });
  const text = h("div", { class: "d-dice-text", "aria-live": "polite" });
  tray.replaceChildren(faces, text);
  let seen = { turn: null, key: "" };

  const show = (kind, list, line, note, tint, fresh) => {
    faces.replaceChildren(...list.map((v) => die(v, { tumble: fresh, tint })));
    text.replaceChildren(h("b", {}, line), h("span", {}, note));
  };
  const update = () => {
    let view;
    try { view = board.view; } catch { return; }
    if (!view) return;
    const turn = view.turn;
    if (game.slug === "snakes-and-ladders") {
      const pips = view.bags.find((b) => b.name === "TRAY")?.pieces?.PIP ?? 0;
      const key = `${turn}:${pips}`;
      if (key === seen.key) return;
      if (pips > 0 && seen.turn && turn !== seen.turn) {
        show("roll", [pips], `${nice(seen.turn)} rolled ${pips}`, `${nice(turn)} to roll`, COLOURS[seen.turn], true);
      } else if (pips === 0) {
        faces.replaceChildren(ghost()); text.replaceChildren(h("b", {}, `${nice(turn)} to roll`), h("span", {}, "Press Roll below the board"));
      }
      seen = { turn, key };
    } else {
      const bag = view.bags.find((b) => b.name === "DICE" && b.owner === turn);
      const list = Object.entries(bag?.pieces ?? {}).flatMap(([code, n]) => Array(n).fill(Number(code.slice(1))));
      const key = `${turn}:${list.join("")}`;
      if (key === seen.key) return;
      const grew = list.length > (seen.turn === turn ? (seen.len ?? 0) : 0);
      if (list.length) show("dice", list, `${nice(turn)}’s dice`, list.length > 2 ? "A double: four moves" : "Each die is one move", COLOURS[turn], grew);
      else { faces.replaceChildren(ghost()); text.replaceChildren(h("b", {}, `${nice(turn)} to roll`), h("span", {}, "Press Roll below the board")); }
      seen = { turn, key, len: list.length };
    }
  };
  new MutationObserver(update).observe(board, { childList: true, subtree: true });
  const wait = setInterval(() => { try { if (board.view) { clearInterval(wait); update(); } } catch {} }, 150);
}
