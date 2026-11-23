// B4 · Study. Three small jobs: the colour-scheme picker, the state of each cover print (loading,
// loaded, missing), and the board tool in the builder. Classic script so it works from disk.

(function () {
  // ---- Schemes: the same page in three woods. ----
  const WAYS = { walnut: ["Walnut", "#3a2315", "#f3e5cb"], chocolate: ["Chocolate", "#140800", "#ffb627"], boxwood: ["Boxwood", "#e6d3b2", "#2b160b"] };
  const read = () => { try { return localStorage.getItem("poc-b4-way"); } catch { return null; } };
  const set = (way) => {
    if (way === "walnut") delete document.documentElement.dataset.theme;
    else document.documentElement.dataset.theme = way;
    try { localStorage.setItem("poc-b4-way", way); } catch {}
    for (const b of document.querySelectorAll("[data-way]")) b.setAttribute("aria-pressed", String(b.dataset.way === way));
  };
  const picker = document.createElement("div");
  picker.className = "poc-ways";
  picker.setAttribute("role", "group");
  picker.setAttribute("aria-label", "Exploration controls: colour scheme");
  picker.innerHTML = "<span>Scheme</span>" + Object.entries(WAYS)
    .map(([w, [name, a, b]]) => `<button type="button" data-way="${w}"><i style="background:linear-gradient(135deg,${a} 50%,${b} 50%)"></i>${name}</button>`)
    .join("");
  const style = document.createElement("style");
  style.textContent = `.poc-ways{position:fixed;left:12px;bottom:12px;z-index:99;display:flex;gap:4px;align-items:center;padding:6px 8px;border-radius:10px;background:#111c;color:#fff;font:600 12px/1 system-ui,sans-serif;backdrop-filter:blur(6px)}
  .poc-ways span{padding:0 6px;opacity:.7}
  .poc-ways button{display:inline-flex;align-items:center;gap:6px;min-height:32px;padding:0 10px;border:0;border-radius:6px;background:#fff2;color:#fff;font:inherit}
  .poc-ways i{width:14px;height:14px;border-radius:50%;border:1.5px solid #fff}
  .poc-ways button[aria-pressed="true"]{background:#fff;color:#111}
  .poc-ways button[aria-pressed="true"] i{border-color:#111}
  .poc-ways button:focus-visible{outline:2px solid #fff;outline-offset:2px}
  @media (max-width:1100px){.poc-states ~ .poc-ways,.poc-ways.raise{bottom:64px}}`;
  document.head.append(style);
  document.body.append(picker);
  picker.addEventListener("click", (e) => { const b = e.target.closest("[data-way]"); if (b) set(b.dataset.way); });
  const saved = read();
  set(saved && WAYS[saved] ? saved : "walnut");
  if (document.querySelector("[data-board]")) picker.classList.add("raise");

  // ---- Cover prints: fade in once loaded, fall back to a piece when the file is missing. ----
  for (const img of document.querySelectorAll(".print img")) {
    const print = img.closest(".print");
    const ok = () => print.classList.add("loaded");
    const bad = () => print.classList.add("no-photo");
    img.addEventListener("load", ok);
    img.addEventListener("error", bad);
    if (img.complete) (img.naturalWidth ? ok : bad)();
  }

  // ---- The board tool. ----
  const canvas = document.querySelector("[data-canvas]");
  if (!canvas) return;
  const summary = document.querySelector("[data-summary]");
  const FILES = "abcdefghi";
  let size = 7;
  let brush = "square";
  let cells = [];
  let focus = 0;

  // A first shape that shows what the tool is for: a 7×7 cross with a zone across the far row.
  function starter(n) {
    return Array.from({ length: n * n }, (_, i) => {
      const r = Math.floor(i / n), c = i % n;
      const corner = n >= 7 && (r < 2 || r > n - 3) && (c < 2 || c > n - 3);
      if (corner) return "hole";
      return r === 0 ? "zone" : "square";
    });
  }

  function name(i) {
    const r = Math.floor(i / size), c = i % size;
    return FILES[c] + (size - r);
  }

  function describe() {
    const squares = cells.filter((s) => s !== "hole").length;
    const zone = cells.filter((s) => s === "zone").length;
    const holes = cells.length - squares;
    const parts = [`${size}×${size} board`, `${squares} ${squares === 1 ? "square" : "squares"}`];
    if (holes) parts.push(`${holes} removed`);
    if (zone) parts.push(`${zone} in a zone`);
    summary.textContent = parts.join(", ") + ".";
  }

  function paint(i) {
    const node = canvas.children[i];
    const state = cells[i];
    const r = Math.floor(i / size), c = i % size;
    node.className = "cell" + ((r + c) % 2 ? " dark" : "") + (state === "hole" ? " hole" : "") + (state === "zone" ? " zone" : "");
    node.setAttribute("aria-label", `${name(i)}: ${state === "hole" ? "removed" : state === "zone" ? "square in a zone" : "square"}`);
  }

  function build(n, shape) {
    size = n;
    cells = shape || starter(n);
    focus = Math.min(focus, n * n - 1);
    canvas.style.setProperty("--n", n);
    canvas.replaceChildren(...cells.map((_, i) => {
      const b = document.createElement("button");
      b.type = "button";
      b.tabIndex = i === focus ? 0 : -1;
      b.dataset.i = i;
      return b;
    }));
    cells.forEach((_, i) => paint(i));
    describe();
    for (const b of document.querySelectorAll("[data-size]")) b.setAttribute("aria-pressed", String(Number(b.dataset.size) === n));
  }

  canvas.addEventListener("click", (e) => {
    const b = e.target.closest(".cell");
    if (!b) return;
    const i = Number(b.dataset.i);
    // Using the same brush twice puts the square back, so a slip is one click to undo.
    cells[i] = cells[i] === brush && brush !== "square" ? "square" : brush;
    paint(i);
    describe();
  });

  canvas.addEventListener("keydown", (e) => {
    const step = { ArrowRight: 1, ArrowLeft: -1, ArrowDown: size, ArrowUp: -size }[e.key];
    if (step === undefined && e.key !== "Home" && e.key !== "End") return;
    e.preventDefault();
    let next = e.key === "Home" ? 0 : e.key === "End" ? cells.length - 1 : focus + step;
    if (next < 0 || next >= cells.length) return;
    if (Math.abs(step) === 1 && Math.floor(next / size) !== Math.floor(focus / size)) return;
    canvas.children[focus].tabIndex = -1;
    focus = next;
    canvas.children[focus].tabIndex = 0;
    canvas.children[focus].focus();
  });

  document.addEventListener("click", (e) => {
    const s = e.target.closest("[data-size]");
    if (s) build(Number(s.dataset.size));
    const b = e.target.closest("[data-brush]");
    if (b) {
      brush = b.dataset.brush;
      for (const x of document.querySelectorAll("[data-brush]")) x.setAttribute("aria-pressed", String(x === b));
    }
    if (e.target.closest("[data-clear]")) build(size, Array(size * size).fill("square"));
  });

  build(7);
})();
