// Pure helpers for the board: skins, fallback pieces and geometry.
// Nothing here knows the rules of any game.

const DEFAULT_SQUARES = { light: "#f0d9b5", dark: "#b58863" };
const PLAYER_COLOURS = ["#d94f4f", "#3f6fd9", "#3fa86b", "#d9a13f"];
const NEUTRAL_COLOUR = "#777777";

const isObject = (v) => typeof v === "object" && v !== null && !Array.isArray(v);

// A piece face: a glyph string, or { svg: "<svg…>" }. Anything else is dropped.
function normalizeFace(raw) {
  if (typeof raw === "string" && raw.length > 0) return { kind: "glyph", value: raw };
  if (isObject(raw) && typeof raw.svg === "string" && raw.svg.length > 0) {
    return { kind: "svg", value: raw.svg };
  }
  return null;
}

function normalizePieceEntry(raw) {
  const shared = normalizeFace(raw);
  if (shared) return { shared };
  if (!isObject(raw)) return null;
  const perPlayer = {};
  for (const [player, face] of Object.entries(raw)) {
    const normalized = normalizeFace(face);
    if (normalized) perPlayer[player] = normalized;
  }
  return Object.keys(perPlayer).length > 0 ? { perPlayer } : null;
}

/** Turns anything into a usable skin. Never throws. */
export function normalizeSkin(raw) {
  const skin = { pieces: {}, squares: { ...DEFAULT_SQUARES }, zones: {} };
  if (!isObject(raw)) return skin;

  if (isObject(raw.pieces)) {
    for (const [code, entry] of Object.entries(raw.pieces)) {
      const normalized = normalizePieceEntry(entry);
      if (normalized) skin.pieces[code] = normalized;
    }
  }
  if (isObject(raw.squares)) {
    if (typeof raw.squares.light === "string") skin.squares.light = raw.squares.light;
    if (typeof raw.squares.dark === "string") skin.squares.dark = raw.squares.dark;
  }
  if (isObject(raw.zones)) {
    for (const [name, colour] of Object.entries(raw.zones)) {
      if (typeof colour === "string") skin.zones[name] = colour;
    }
  }
  return skin;
}

function playerColour(player, players) {
  const index = players.indexOf(player);
  return index >= 0 ? PLAYER_COLOURS[index % PLAYER_COLOURS.length] : NEUTRAL_COLOUR;
}

/**
 * What to draw for a piece: { kind: "glyph" | "svg" | "fallback", value, colour? }.
 * A per-player entry wins; a single entry is tinted by owner; otherwise the
 * piece code's first letter, coloured by owner.
 */
export function pieceFace(skin, code, player, players) {
  const entry = skin.pieces[code];
  if (entry?.perPlayer?.[player]) {
    return { ...entry.perPlayer[player] };
  }
  if (entry?.shared) {
    return { ...entry.shared, colour: playerColour(player, players) };
  }
  return {
    kind: "fallback",
    value: (code[0] ?? "?").toUpperCase(),
    colour: playerColour(player, players),
  };
}

/** Grid cell for a square. `y = 0` is the bottom row. */
export function cellRect(view, x, y) {
  return { col: x, row: view.height - 1 - y };
}

/** The tint colour for a square, from the first skin-coloured zone that contains it. */
export function zoneTint(skin, view, x, y) {
  for (const [name, colour] of Object.entries(skin.zones)) {
    const perPlayer = view.zones?.[name];
    if (!perPlayer) continue;
    for (const squares of Object.values(perPlayer)) {
      if (squares.some(([sx, sy]) => sx === x && sy === y)) return colour;
    }
  }
  return null;
}
