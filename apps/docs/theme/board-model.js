// Pure helpers for the board: skins, fallback pieces and geometry.
// Nothing here knows the rules of any game.

const DEFAULT_SQUARES = { light: "#f0d9b5", dark: "#b58863" };
const PLAYER_COLOURS = ["#d94f4f", "#3f6fd9", "#3fa86b", "#d9a13f"];
const NEUTRAL_COLOUR = "#777777";

const isObject = (v) => typeof v === "object" && v !== null && !Array.isArray(v);

// An image address a skin may use: a path on this site, in plain characters, to a picture file.
// Nothing that names another site, runs code or climbs out of the folder gets through.
const IMAGE_PATH = /^\/(?!\/)[A-Za-z0-9_\-./]+\.(svg|png|webp|jpe?g)$/;
const isImagePath = (value) => typeof value === "string" && IMAGE_PATH.test(value) && !value.includes("..");

// A colour a skin may use: a hex value, a colour name or rgb()/hsl() with plain numbers. Nothing
// that loads a file, names another style rule or runs code gets through.
const HEX_COLOUR = /^#[0-9a-fA-F]{3,8}$/;
const NAMED_COLOUR = /^[a-zA-Z]{3,20}$/;
const FUNCTION_COLOUR = /^(rgb|rgba|hsl|hsla)\(\s*[0-9.,%\s/-]+\)$/;
const isColour = (value) =>
  typeof value === "string" && (HEX_COLOUR.test(value) || NAMED_COLOUR.test(value) || FUNCTION_COLOUR.test(value));

// Board drawings are numbers in intersection units: whole numbers for crossings, halves for the
// space between two. A skin cannot ask for a large drawing.
const MAX_ITEMS = 200;
const isCoordinate = (value) => Number.isFinite(value) && Math.abs(value) <= 100;
const isPoint = (value) => Array.isArray(value) && value.length === 2 && value.every(isCoordinate);

function listOf(raw, accept) {
  return Array.isArray(raw) ? raw.filter(accept).slice(0, MAX_ITEMS) : [];
}

const BOARD_STYLES = ["squares", "intersections"];

function normalizeBoard(raw) {
  const board = {
    style: "squares",
    background: null,
    lines: null,
    breaks: [],
    diagonals: [],
    dots: [],
    crosses: [],
    labels: [],
  };
  if (!isObject(raw)) return board;
  if (BOARD_STYLES.includes(raw.style)) board.style = raw.style;
  if (isColour(raw.background)) board.background = raw.background;
  if (isColour(raw.lines)) board.lines = raw.lines;
  // Rows where the lines between two neighbouring rows stop, but for the two edge files (a river).
  board.breaks = listOf(raw.breaks, isPoint);
  board.diagonals = listOf(raw.diagonals, (d) => Array.isArray(d) && d.length === 2 && d.every(isPoint));
  board.dots = listOf(raw.dots, isPoint);
  board.crosses = listOf(raw.crosses, isPoint);
  board.labels = listOf(
    raw.labels,
    (l) => isObject(l) && typeof l.text === "string" && l.text.length > 0 && l.text.length <= 12 && isCoordinate(l.x) && isCoordinate(l.y),
  ).map(({ text, x, y }) => ({ text, x, y }));
  return board;
}

// A piece face: a glyph string, { svg: "<svg…>" } or { image: "/path/to/file.svg" }. Anything
// else is dropped.
function normalizeFace(raw) {
  if (typeof raw === "string" && raw.length > 0) return { kind: "glyph", value: raw };
  if (isObject(raw) && typeof raw.svg === "string" && raw.svg.length > 0) {
    return { kind: "svg", value: raw.svg };
  }
  if (isObject(raw) && isImagePath(raw.image)) return { kind: "image", value: raw.image };
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
  const skin = {
    pieces: {},
    squares: { ...DEFAULT_SQUARES },
    zones: {},
    facing: false,
    board: normalizeBoard(undefined),
    colours: {},
    pieceStyle: "plain",
  };
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
  skin.facing = raw.facing === true;
  skin.board = normalizeBoard(raw.board);
  if (isObject(raw.colours)) {
    for (const [player, colour] of Object.entries(raw.colours)) {
      if (isColour(colour)) skin.colours[player] = colour;
    }
  }
  if (raw.pieceStyle === "disc") skin.pieceStyle = "disc";
  if (isObject(raw.zones)) {
    for (const [name, colour] of Object.entries(raw.zones)) {
      if (typeof colour === "string") skin.zones[name] = colour;
    }
  }
  return skin;
}

function playerColour(skin, player, players) {
  if (skin.colours[player]) return skin.colours[player];
  const index = players.indexOf(player);
  return index >= 0 ? PLAYER_COLOURS[index % PLAYER_COLOURS.length] : NEUTRAL_COLOUR;
}

/**
 * What to draw for a piece: { kind: "glyph" | "svg" | "image" | "fallback", value, colour?, disc? }.
 * A per-player entry wins; a single entry is tinted by owner; otherwise the piece code's first
 * letter, coloured by owner. With the disc style, text and drawings sit on a round tile.
 */
export function pieceFace(skin, code, player, players) {
  const entry = skin.pieces[code];
  let face;
  if (entry?.perPlayer?.[player]) {
    face = { ...entry.perPlayer[player] };
    // A colour chosen for the player is kept; otherwise the face is drawn as it is written.
    if (skin.colours[player] && face.kind !== "image") face.colour = skin.colours[player];
  } else if (entry?.shared) {
    // A picture is drawn as it is. Only text and drawings take the owner's colour.
    face = { ...entry.shared };
    if (face.kind !== "image") face.colour = playerColour(skin, player, players);
  } else {
    face = {
      kind: "fallback",
      value: (code[0] ?? "?").toUpperCase(),
      colour: playerColour(skin, player, players),
    };
  }
  if (skin.pieceStyle === "disc" && face.kind !== "image") face.disc = true;
  return face;
}

/**
 * Whether a piece is drawn turned half way round. With the skin's `facing` option on, the player
 * at the top of the board has pieces that point down at the opponent, as on a real Shogi board.
 * The player at the top is the second one, or the first one when the board is flipped.
 */
export function isTurned(skin, player, players, flipped) {
  if (!skin.facing || players.length !== 2) return false;
  return player === players[flipped ? 0 : 1];
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
