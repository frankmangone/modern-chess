// Shared by every exploration: draws one chess position into [data-board], fills the captured
// panels, and runs a small state switcher so each design can be judged in the moments that
// matter (your turn, waiting, the resign question, the match ending). Not part of the app.

const POSITION = [
  "r.bqkb.r", // 8
  "ppp..ppp", // 7
  "..n.....", // 6
  "...np.N.", // 5
  "..B.....", // 4
  "........", // 3
  "PPPP.PPP", // 2
  "RNBQK..R", // 1
];

const GLYPH = { k: "♚", q: "♛", r: "♜", b: "♝", n: "♞", p: "♟" };
const NAME = { k: "king", q: "queen", r: "rook", b: "bishop", n: "knight", p: "pawn" };
const FILES = "abcdefgh";

const glyph = (code) => GLYPH[code.toLowerCase()] + "︎";

const STATES = {
  "your-turn": {
    label: "Your turn",
    status: "Your turn.",
    yours: true,
    select: "g5",
    moves: ["e6", "e4", "f3", "h3"],
    captures: ["f7", "h7"],
    resign: "idle",
  },
  waiting: {
    label: "Waiting",
    status: "Waiting for Guest-2103…",
    yours: false,
    resign: "idle",
  },
  "resign-asking": {
    label: "Resign prompt",
    status: "Your turn.",
    yours: true,
    select: "g5",
    moves: ["e6", "e4", "f3", "h3"],
    captures: ["f7", "h7"],
    resign: "ask",
  },
  "you-resigned": {
    label: "You resigned",
    status: "You resigned.",
    over: true,
    lost: true,
    resign: "none",
  },
  lobby: {
    label: "Lobby",
    status: "Waiting for another player to join.",
    yours: false,
    waitingRoom: true,
    resign: "lobby",
  },
  "opponent-resigned": {
    label: "They resigned",
    status: "Your opponent resigned. You won!",
    over: true,
    won: true,
    resign: "none",
  },
};

function drawBoard(root, { flipped = false } = {}) {
  root.setAttribute("role", "group");
  root.setAttribute("aria-label", "Chess board");
  root.replaceChildren();
  for (let row = 0; row < 8; row++) {
    for (let file = 0; file < 8; file++) {
      const r = flipped ? 7 - row : row;
      const f = flipped ? 7 - file : file;
      const square = FILES[f] + (8 - r);
      const cell = document.createElement("div");
      cell.className = "sq " + ((f + r) % 2 === 0 ? "light" : "dark");
      cell.dataset.sq = square;
      const code = POSITION[r][f];
      if (code !== ".") {
        const side = code === code.toUpperCase() ? "white" : "black";
        const piece = document.createElement("span");
        piece.className = "pc " + side;
        piece.textContent = glyph(code);
        piece.setAttribute("role", "img");
        piece.setAttribute("aria-label", `${side} ${NAME[code.toLowerCase()]} on ${square}`);
        cell.append(piece);
      } else {
        cell.setAttribute("aria-label", `empty square ${square}`);
      }
      root.append(cell);
    }
  }
}

function fillCaptured() {
  // White has lost the pawn from e4; black has lost the pawn from d7.
  const lost = { white: ["P"], black: ["p"] };
  for (const el of document.querySelectorAll("[data-captured]")) {
    const side = el.dataset.captured;
    el.replaceChildren(
      ...lost[side].map((code) => {
        const piece = document.createElement("span");
        piece.className = "pc " + side;
        piece.textContent = glyph(code);
        piece.setAttribute("role", "img");
        piece.setAttribute("aria-label", `${side} ${NAME[code.toLowerCase()]}`);
        return piece;
      }),
    );
  }
}

function apply(name) {
  const state = STATES[name];
  const root = document.documentElement;
  root.dataset.state = name;
  root.classList.toggle("is-yours", Boolean(state.yours));
  root.classList.toggle("is-over", Boolean(state.over));
  root.classList.toggle("is-lobby", Boolean(state.waitingRoom));
  for (const el of document.querySelectorAll("[data-status]")) el.textContent = state.status;
  for (const cell of document.querySelectorAll("[data-board] .sq")) {
    const sq = cell.dataset.sq;
    cell.classList.toggle("sel", sq === state.select);
    cell.classList.toggle("move", Boolean(state.moves?.includes(sq)));
    cell.classList.toggle("capture", Boolean(state.captures?.includes(sq)));
  }
  for (const bar of document.querySelectorAll("[data-bar]")) {
    const mine = bar.dataset.bar === "you";
    bar.classList.toggle("is-turn", !state.over && (mine ? state.yours : !state.yours));
  }
  for (const el of document.querySelectorAll("[data-resign]")) {
    el.hidden = el.dataset.resign !== state.resign;
  }
  for (const button of document.querySelectorAll("[data-poc-state]")) {
    button.setAttribute("aria-pressed", String(button.dataset.pocState === name));
  }
}

const DEFAULT_STATES = ["your-turn", "waiting", "resign-asking", "you-resigned", "opponent-resigned"];

function switcher(keys) {
  const bar = document.createElement("nav");
  bar.className = "poc-states";
  bar.setAttribute("aria-label", "Exploration controls: match state");
  bar.innerHTML =
    "<span>State</span>" +
    keys
      .map((key) => `<button type="button" data-poc-state="${key}">${STATES[key].label}</button>`)
      .join("");
  const style = document.createElement("style");
  style.textContent = `
    .poc-states{position:fixed;left:50%;bottom:12px;transform:translateX(-50%);z-index:99;display:flex;flex-wrap:nowrap;overflow-x:auto;gap:4px;align-items:center;max-width:calc(100vw - 24px);padding:6px 8px;border-radius:10px;background:#111c;color:#fff;font:600 12px/1 system-ui,sans-serif;backdrop-filter:blur(6px)}
    .poc-states span{padding:0 6px;opacity:.7}
    .poc-states button{flex:none;white-space:nowrap;min-height:32px;padding:0 10px;border:0;border-radius:6px;background:#fff2;color:#fff;font:inherit;cursor:pointer}
    .poc-states button[aria-pressed="true"]{background:#fff;color:#111}
    .poc-states button:focus-visible{outline:2px solid #fff;outline-offset:2px}`;
  document.head.append(style);
  document.body.append(bar);
  bar.addEventListener("click", (event) => {
    const target = event.target.closest("[data-poc-state]");
    if (target) apply(target.dataset.pocState);
  });
}

function startMatch({ states = DEFAULT_STATES } = {}) {
  for (const board of document.querySelectorAll("[data-board]")) drawBoard(board);
  fillCaptured();
  switcher(states);
  apply(states[0] === "lobby" ? "your-turn" : "your-turn");

  document.addEventListener("click", (event) => {
    const action = event.target.closest("[data-action]")?.dataset.action;
    if (action === "resign") {
      apply("resign-asking");
      document.querySelector("[data-action=keep]")?.focus();
    } else if (action === "keep") {
      apply("your-turn");
      document.querySelector("[data-action=resign]")?.focus();
    } else if (action === "confirm") {
      apply("you-resigned");
    }
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && document.documentElement.dataset.state === "resign-asking") {
      apply("your-turn");
      document.querySelector("[data-action=resign]")?.focus();
    }
  });
}

window.startMatch = startMatch;
window.drawBoard = drawBoard;
window.pocGlyph = glyph;
