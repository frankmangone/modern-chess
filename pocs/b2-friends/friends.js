// B2 · Friends. A hand-drawn piece set where every piece has a face, plus the colourway picker.
// The board script draws plain glyphs; dress() swaps each one for a character.

(function () {
  const INK = "#4a2412";
  const eyes = (y, s) => `
    <g class="eyes"><ellipse cx="${32 - s}" cy="${y}" rx="2.3" ry="3"/><ellipse cx="${32 + s}" cy="${y}" rx="2.3" ry="3"/></g>
    <g class="xeyes"><path d="M${32 - s - 3} ${y - 3}l6 6M${32 - s + 3} ${y - 3}l-6 6M${32 + s - 3} ${y - 3}l6 6M${32 + s + 3} ${y - 3}l-6 6"/></g>`;
  const mouth = (y, s) => `<path class="mouth" d="M${32 - s * 0.45} ${y + 5}q${s * 0.45} 3.6 ${s * 0.9} 0"/>`;
  const blush = (y, s) => `<ellipse class="blush" cx="${32 - s - 5}" cy="${y + 4.5}" rx="3" ry="1.9"/><ellipse class="blush" cx="${32 + s + 5}" cy="${y + 4.5}" rx="3" ry="1.9"/>`;
  const face = (y, s = 7) => `<g class="face">${eyes(y, s)}${blush(y, s)}${mouth(y, s)}</g>`;
  const base = `<rect class="b" x="14" y="52" width="36" height="9" rx="4.5"/>`;

  const SYMBOLS = {
    pawn: `<circle class="b" cx="32" cy="19" r="11"/><path class="b" d="M21 54c0-12 4-18 7-23h8c3 5 7 11 7 23z"/>${base}${face(19, 4.4)}`,
    rook: `<path class="b" d="M18 7h8v7h4V7h4v7h4V7h8v21H18z"/><path class="b" d="M21 28h22l3 24H18z"/>${base}${face(39)}`,
    knight: `<path class="b" d="M16 54c-1-12 5-18 12-25-4-1-9-4-10-9l9-9 3-7 5 6c11 2 18 12 16 26l-1 18z"/>${base}
      <g class="face"><g class="eyes"><ellipse cx="36" cy="22" rx="2.3" ry="3"/></g>
      <g class="xeyes"><path d="M33 19l6 6M39 19l-6 6"/></g>
      <ellipse class="blush" cx="42" cy="29" rx="3" ry="1.9"/><path class="mouth" d="M26 31q3 3 7 0"/><circle cx="23" cy="25" r="1.2" style="fill:${INK}"/></g>`,
    bishop: `<path class="b" d="M32 4c8 8 14 18 8 28 3 6 5 14 5 21H19c0-7 2-15 5-21C18 22 24 12 32 4z"/>${base}<path d="M29 12l6 7" style="fill:none;stroke:#8b4a2b;stroke-width:3;stroke-linecap:round"/>${face(40)}`,
    queen: `<path class="b" d="M13 14l9 16 6-18h8l6 18 9-16-3 26H16z"/><circle class="b" cx="13" cy="12" r="3.6"/><circle class="b" cx="28" cy="9" r="3.6"/><circle class="b" cx="36" cy="9" r="3.6"/><circle class="b" cx="51" cy="12" r="3.6"/><path class="b" d="M17 38h30l3 15H14z"/>${base}${face(44, 8)}`,
    king: `<path class="b" d="M18 38c-2-12 4-19 14-19s16 7 14 19z"/><path class="b" d="M29 2h6v5h5v6h-5v5h-6v-5h-5V7h5z"/><path class="b" d="M18 38h28l4 15H14z"/>${base}${face(32, 6.5)}`,
    stone: `<circle class="b" cx="32" cy="32" r="24"/>${face(30, 8)}`,
    checker: `<circle class="b" cx="32" cy="32" r="24"/><circle cx="32" cy="32" r="16" style="fill:none;stroke:#8b4a2b;stroke-width:3"/>${face(30, 7)}`,
  };
  const BY_GLYPH = { "♚": "king", "♛": "queen", "♜": "rook", "♝": "bishop", "♞": "knight", "♟": "pawn" };

  const sprite = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  sprite.setAttribute("width", "0");
  sprite.setAttribute("height", "0");
  sprite.setAttribute("aria-hidden", "true");
  sprite.style.position = "absolute";
  sprite.innerHTML = Object.entries(SYMBOLS)
    .map(([name, body]) => `<symbol id="pc-${name}" viewBox="0 0 64 64">${body}</symbol>`)
    .join("");
  document.body.prepend(sprite);

  let count = 0;
  function pieceSvg(name) {
    count += 1;
    const delay = ((count * 1.7) % 6).toFixed(1);
    return `<svg class="pc-svg" viewBox="0 0 64 64" aria-hidden="true" style="--d:-${delay}s"><use href="#pc-${name}"/></svg>`;
  }

  window.Friends = {
    piece: pieceSvg,
    dress(root = document) {
      for (const node of root.querySelectorAll(".pc:not(.dressed)")) {
        const name = BY_GLYPH[node.textContent[0]];
        if (!name) continue;
        node.classList.add("dressed");
        node.innerHTML = pieceSvg(name);
      }
      for (const host of root.querySelectorAll("[data-piece]")) host.innerHTML = pieceSvg(host.dataset.piece);
    },
  };

  // Colourways: the same drawing in four moods.
  const WAYS = { peach: "#ffe1c9", mint: "#d6f0e0", lilac: "#e5dcf7", butter: "#fff1b8" };
  const saved = (() => { try { return localStorage.getItem("poc-b2-way"); } catch { return null; } })();
  const set = (way) => {
    document.documentElement.dataset.theme = way;
    try { localStorage.setItem("poc-b2-way", way); } catch {}
    for (const b of document.querySelectorAll("[data-way]")) b.setAttribute("aria-pressed", String(b.dataset.way === way));
  };
  const picker = document.createElement("div");
  picker.className = "poc-ways";
  picker.setAttribute("role", "group");
  picker.setAttribute("aria-label", "Exploration controls: colourway");
  picker.innerHTML = Object.entries(WAYS).map(([w, c]) => `<button type="button" data-way="${w}" aria-label="${w} colourway" title="${w}" style="background:${c}"></button>`).join("");
  const style = document.createElement("style");
  style.textContent = `.poc-ways{position:fixed;left:12px;bottom:12px;z-index:99;display:flex;gap:6px;padding:6px;border-radius:12px;background:#111c;backdrop-filter:blur(6px)}
  .poc-ways button{width:32px;height:32px;border-radius:50%;border:3px solid #fff0;cursor:pointer}
  .poc-ways button[aria-pressed="true"]{border-color:#fff}
  .poc-ways button:focus-visible{outline:2px solid #fff;outline-offset:2px}
  @media (max-width:700px){.poc-ways{bottom:64px}}`;
  document.head.append(style);
  document.body.append(picker);
  picker.addEventListener("click", (e) => { const b = e.target.closest("[data-way]"); if (b) set(b.dataset.way); });
  set(saved && WAYS[saved] ? saved : "peach");
})();
